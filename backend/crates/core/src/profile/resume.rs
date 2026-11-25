//! Uploaded resumes: checking the file type, pulling out the text, and
//! keeping the original file in local file storage.

use std::{io::Read, sync::Arc};

use chrono::{DateTime, Utc};
use object_store::{ObjectStore, ObjectStoreExt, PutPayload, local::LocalFileSystem, path::Path};
use quick_xml::events::Event;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    ProfileError, invalid, parse,
    resume_content::{self, CONTENT_VERSION, ResumeContent},
};

pub const MAX_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pdf,
    Docx,
}

impl Kind {
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Docx => "docx",
        }
    }
}

/// By the file's own bytes, not its name: PDFs start with `%PDF`, DOCX files
/// are zips (`PK`) holding `word/document.xml`.
pub fn detect(bytes: &[u8]) -> Option<Kind> {
    if bytes.starts_with(b"%PDF") {
        return Some(Kind::Pdf);
    }
    if bytes.starts_with(b"PK\x03\x04") {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
        if zip.by_name("word/document.xml").is_ok() {
            return Some(Kind::Docx);
        }
    }
    None
}

/// What a resume file holds: its text, and the addresses its links point
/// to. A link's address often isn't in the text at all: the resume shows
/// "GitHub" or a user name, and the address is only in the link.
#[derive(Debug, Clone, Default)]
pub struct Extracted {
    /// One line per paragraph or PDF text line.
    pub text: String,
    /// Link targets in reading order (page, then top to bottom, then left to
    /// right), without duplicates.
    pub links: Vec<String>,
}

/// The resume's text and links. CPU-bound, so call it from `spawn_blocking`.
pub fn extract(kind: Kind, bytes: &[u8]) -> Result<Extracted, ProfileError> {
    let (text, links) = match kind {
        Kind::Pdf => {
            // pdf-extract panics on some malformed PDFs; treat that as unreadable.
            let text = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(bytes))
                .ok()
                .and_then(Result::ok)
                .ok_or_else(|| {
                    invalid("Couldn't read that PDF. Try exporting it again, or upload a DOCX.")
                })?;
            let links = std::panic::catch_unwind(|| pdf_links(bytes)).unwrap_or_default();
            (text, links)
        }
        Kind::Docx => {
            let text = docx_text(bytes).ok_or_else(|| {
                invalid("Couldn't read that DOCX. Try saving it again, or upload a PDF.")
            })?;
            (text, docx_links(bytes).unwrap_or_default())
        }
    };
    let text = text.replace('\r', "").replace('\u{a0}', " ");
    if text.split_whitespace().count() < 20 {
        return Err(invalid(
            "That file has almost no text. If it's a scanned image, upload a PDF exported from Word or Google Docs instead.",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let links = links
        .into_iter()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty() && l.len() <= 500 && seen.insert(l.to_lowercase()))
        .collect();
    Ok(Extracted { text, links })
}

/// The `/URI` of every link annotation, in reading order.
fn pdf_links(bytes: &[u8]) -> Vec<String> {
    use pdf_extract::{Dictionary, Document, Object};
    let Ok(doc) = Document::load_mem(bytes) else {
        return Vec::new();
    };
    let deref = |o: &Object| -> Option<Object> {
        match o {
            Object::Reference(id) => doc.get_object(*id).ok().cloned(),
            o => Some(o.clone()),
        }
    };
    let uri = |a: &Dictionary| -> Option<String> {
        if a.get(b"Subtype").ok()?.as_name().ok()? != b"Link" {
            return None;
        }
        let action = deref(a.get(b"A").ok()?)?;
        let uri = deref(action.as_dict().ok()?.get(b"URI").ok()?)?;
        Some(String::from_utf8_lossy(uri.as_str().ok()?).into_owned())
    };
    let mut found: Vec<(u32, f32, f32, String)> = Vec::new();
    for (page_no, page_id) in doc.get_pages() {
        let Some(annots) = doc
            .get_dictionary(page_id)
            .ok()
            .and_then(|p| p.get(b"Annots").ok())
            .and_then(deref)
        else {
            continue;
        };
        let Ok(annots) = annots.as_array() else {
            continue;
        };
        for a in annots {
            let Some(a) = deref(a) else { continue };
            let Ok(a) = a.as_dict() else { continue };
            let Some(url) = uri(a) else { continue };
            // /Rect is [x1 y1 x2 y2], with y growing upwards.
            let rect: Vec<f32> = a
                .get(b"Rect")
                .ok()
                .and_then(deref)
                .and_then(|r| r.as_array().ok().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|n| n.as_float().ok())
                .collect();
            let (x, top) = match rect.as_slice() {
                [x1, y1, _, y2] => (*x1, y1.max(*y2)),
                _ => (0.0, 0.0),
            };
            found.push((page_no, top, x, url));
        }
    }
    // Links whose tops are within 3 points are on the same line.
    found.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| {
            if (a.1 - b.1).abs() < 3.0 {
                a.2.total_cmp(&b.2)
            } else {
                b.1.total_cmp(&a.1)
            }
        })
    });
    found.into_iter().map(|f| f.3).collect()
}

/// External hyperlink targets (`word/_rels/document.xml.rels`), in the order
/// the document uses them.
fn docx_links(bytes: &[u8]) -> Option<Vec<String>> {
    fn attr(e: &quick_xml::events::BytesStart, name: &str) -> Option<String> {
        e.attributes()
            .flatten()
            .find(|a| a.key.local_name().as_ref() == name)
            .map(|a| a.value.into_owned())
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let mut rels = String::new();
    zip.by_name("word/_rels/document.xml.rels")
        .ok()?
        .read_to_string(&mut rels)
        .ok()?;
    let mut targets = std::collections::HashMap::new();
    let mut reader = quick_xml::Reader::from_str(&rels);
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) | Event::Empty(e) => {
                if attr(&e, "Type").is_some_and(|t| t.ends_with("/hyperlink"))
                    && let (Some(id), Some(target)) = (attr(&e, "Id"), attr(&e, "Target"))
                {
                    targets.insert(id, target.replace("&amp;", "&"));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .ok()?
        .read_to_string(&mut xml)
        .ok()?;
    let mut out = Vec::new();
    let mut reader = quick_xml::Reader::from_str(&xml);
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == "hyperlink" => {
                if let Some(t) = attr(&e, "id").and_then(|id| targets.get(&id)) {
                    out.push(t.clone());
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Some(out)
}

/// Paragraph text from `word/document.xml`. List paragraphs (Word's own
/// bullets, which aren't characters in the text) get a "• " prefix.
fn docx_text(bytes: &[u8]) -> Option<String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .ok()?
        .read_to_string(&mut xml)
        .ok()?;

    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut out = String::new();
    let mut para = String::new();
    let mut in_text = false;
    let mut list_item = false;
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                "t" => in_text = true,
                "tab" => para.push('\t'),
                "br" | "cr" => para.push('\n'),
                "numPr" => list_item = true,
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                "t" => in_text = false,
                "p" => {
                    let line = para.trim();
                    if !line.is_empty() {
                        if list_item {
                            out.push_str("• ");
                        }
                        out.push_str(line);
                    }
                    out.push('\n');
                    para.clear();
                    list_item = false;
                }
                _ => {}
            },
            Event::Text(t) if in_text => para.push_str(&t.xml10_content()),
            Event::GeneralRef(r) if in_text => {
                if r.is_char_ref() {
                    if let Ok(Some(c)) = r.resolve_char_ref() {
                        para.push(c);
                    }
                } else {
                    para.push_str(match r.xml10_content().as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => "",
                    });
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Some(out)
}

/// Local file storage (`FILES_DIR`), S3-compatible later.
#[derive(Clone)]
pub struct Files(Arc<dyn ObjectStore>);

impl Files {
    pub fn local(dir: &str) -> anyhow::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Self(Arc::new(LocalFileSystem::new_with_prefix(dir)?)))
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ResumeRow {
    pub id: Uuid,
    /// Shown in the list; starts as the file name without its extension.
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    /// The job title this version is aimed at; empty when not set.
    pub target_title: String,
    /// The resume used by default (autofill, tailoring). One per user.
    pub is_primary: bool,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const ROW_COLUMNS: &str = "id, name, filename, content_type, size_bytes, target_title, is_primary, uploaded_at, updated_at";

/// Text-only characters, at most `max` of them, trimmed.
fn display_text(s: &str, max: usize) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect::<String>()
        .trim()
        .to_owned()
}

/// "Jane Doe Resume.pdf" -> "Jane Doe Resume".
fn default_name(filename: &str) -> String {
    let stem = match filename.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.len() <= 5 => stem,
        _ => filename,
    };
    let name = display_text(stem, 120);
    if name.is_empty() {
        "Resume".into()
    } else {
        name
    }
}

/// Keeps the original file, its text and links, and the content read from
/// it. The first resume a user uploads becomes their primary one. Returns
/// the new resume's id.
#[allow(clippy::too_many_arguments)]
pub async fn save(
    pool: &PgPool,
    files: &Files,
    user_id: Uuid,
    filename: &str,
    kind: Kind,
    bytes: Vec<u8>,
    extracted: &Extracted,
    content: &ResumeContent,
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    let key = format!("resumes/{user_id}/{id}.{}", kind.extension());
    let size = bytes.len() as i64;
    files
        .0
        .put(&Path::from(key.as_str()), PutPayload::from(bytes))
        .await?;
    // Only a display name: never used as a path.
    let filename = display_text(filename, 200);
    sqlx::query(
        "INSERT INTO resumes (id, user_id, file_key, filename, content_type, size_bytes, text,
                              links, name, is_primary, content, content_version)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                 NOT EXISTS (SELECT 1 FROM resumes WHERE user_id = $2 AND is_primary),
                 $10::jsonb, $11)",
    )
    .bind(id)
    .bind(user_id)
    .bind(&key)
    .bind(&filename)
    .bind(kind.content_type())
    .bind(size)
    .bind(&extracted.text)
    .bind(&extracted.links)
    .bind(default_name(&filename))
    .bind(serde_json::to_string(content)?)
    .bind(CONTENT_VERSION)
    .execute(pool)
    .await?;
    Ok(id)
}

/// Reads the original file again (or, without it, the stored text) and
/// returns the content parsed from it. CPU work runs on a blocking thread.
async fn read_again(
    pool: &PgPool,
    files: &Files,
    user_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<ResumeContent>> {
    let Some((_, stored)) = extracted(pool, user_id, id).await? else {
        return Ok(None);
    };
    let bytes = match file(pool, files, user_id, id).await {
        Ok(found) => found.map(|(_, b)| b),
        Err(e) => {
            tracing::warn!(error = %e, "reading resume file, using its stored text");
            None
        }
    };
    let content = tokio::task::spawn_blocking(move || {
        let x = bytes
            .and_then(|b| detect(&b).and_then(|k| extract(k, &b).ok()))
            .unwrap_or(stored);
        resume_content::from_parsed(&parse::parse(&x.text, &x.links))
    })
    .await?;
    Ok(Some(content))
}

async fn store_content(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    content: &ResumeContent,
    touch: bool,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE resumes SET content = $3::jsonb, content_version = $4,
                updated_at = CASE WHEN $5 THEN now() ELSE updated_at END
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .bind(serde_json::to_string(content)?)
    .bind(CONTENT_VERSION)
    .bind(touch)
    .execute(pool)
    .await?;
    Ok(())
}

/// A resume's content. Resumes uploaded before contents were kept are read
/// from their file the first time, and the result is stored.
pub async fn content(
    pool: &PgPool,
    files: &Files,
    user_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<ResumeContent>> {
    let row = sqlx::query_as::<_, (Option<String>, i32)>(
        "SELECT content::text, content_version FROM resumes WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    match row {
        None => Ok(None),
        Some((Some(json), version)) if version == CONTENT_VERSION => {
            Ok(Some(serde_json::from_str(&json)?))
        }
        Some(_) => {
            let Some(c) = read_again(pool, files, user_id, id).await? else {
                return Ok(None);
            };
            store_content(pool, user_id, id, &c, false).await?;
            Ok(Some(c))
        }
    }
}

/// Saves edited content after checking it; returns what was stored, or
/// `Ok(None)` when the resume isn't this user's.
pub async fn save_content(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    content: &ResumeContent,
) -> Result<Option<ResumeContent>, ProfileError> {
    let clean = resume_content::validate(content)?;
    let exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM resumes WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    if exists.is_none() {
        return Ok(None);
    }
    store_content(pool, user_id, id, &clean, true)
        .await
        .map_err(|e| ProfileError::Invalid(format!("Couldn't save the resume: {e}")))?;
    Ok(Some(clean))
}

/// "Read the file again": replaces the content with a fresh parse.
pub async fn reparse(
    pool: &PgPool,
    files: &Files,
    user_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<ResumeContent>> {
    let Some(c) = read_again(pool, files, user_id, id).await? else {
        return Ok(None);
    };
    store_content(pool, user_id, id, &c, true).await?;
    Ok(Some(c))
}

/// The primary resume first, then newest first.
pub async fn list(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<ResumeRow>> {
    // ROW_COLUMNS is one of our own constants.
    sqlx::query_as::<_, ResumeRow>(sqlx::AssertSqlSafe(format!(
        "SELECT {ROW_COLUMNS} FROM resumes WHERE user_id = $1
         ORDER BY is_primary DESC, uploaded_at DESC"
    )))
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Changes to a resume's details; `None` leaves a field as it is.
#[derive(Debug, Clone, Default, serde::Deserialize, utoipa::ToSchema)]
#[serde(default)]
pub struct ResumeUpdate {
    #[schema(required = false)]
    pub name: Option<String>,
    #[schema(required = false)]
    pub target_title: Option<String>,
    /// `true` makes this the primary resume (and the old one not).
    #[schema(required = false)]
    pub primary: Option<bool>,
}

/// Updates a resume's details. `Ok(None)` when it isn't this user's.
pub async fn update(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    change: &ResumeUpdate,
) -> Result<Option<ResumeRow>, ProfileError> {
    let name = change.name.as_deref().map(|n| display_text(n, 120));
    if name.as_deref() == Some("") {
        return Err(ProfileError::Field {
            field: "name".into(),
            message: "Give the resume a name.".into(),
        });
    }
    let title = change.target_title.as_deref().map(|t| display_text(t, 200));
    if change.primary == Some(false) {
        return Err(invalid(
            "Choose another resume as primary instead; there's always one.",
        ));
    }
    let mut tx = pool.begin().await?;
    if change.primary == Some(true) {
        sqlx::query(
            "UPDATE resumes SET is_primary = false
             WHERE user_id = $1 AND is_primary AND id <> $2
               AND EXISTS (SELECT 1 FROM resumes WHERE id = $2 AND user_id = $1)",
        )
        .bind(user_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    let row = sqlx::query_as::<_, ResumeRow>(sqlx::AssertSqlSafe(format!(
        "UPDATE resumes SET
             name = COALESCE($3, name),
             target_title = COALESCE($4, target_title),
             is_primary = is_primary OR $5,
             updated_at = now()
         WHERE id = $1 AND user_id = $2
         RETURNING {ROW_COLUMNS}"
    )))
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(title)
    .bind(change.primary == Some(true))
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row)
}

/// Deletes a resume and its file. If it was the primary one, the newest
/// remaining resume becomes primary. Returns false when it isn't this user's.
pub async fn delete(pool: &PgPool, files: &Files, user_id: Uuid, id: Uuid) -> anyhow::Result<bool> {
    let mut tx = pool.begin().await?;
    let Some((key, was_primary)) = sqlx::query_as::<_, (String, bool)>(
        "DELETE FROM resumes WHERE id = $1 AND user_id = $2 RETURNING file_key, is_primary",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(false);
    };
    if was_primary {
        sqlx::query(
            "UPDATE resumes SET is_primary = true
             WHERE id = (SELECT id FROM resumes WHERE user_id = $1 ORDER BY uploaded_at DESC LIMIT 1)",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    // The row is gone either way; a missing file only leaves nothing behind.
    if let Err(e) = files.0.delete(&Path::from(key.as_str())).await {
        tracing::warn!(error = %e, "deleting resume file");
    }
    Ok(true)
}

/// The text and links kept from a resume, to fill the profile from it again.
pub async fn extracted(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
) -> sqlx::Result<Option<(String, Extracted)>> {
    Ok(sqlx::query_as::<_, (String, String, Vec<String>)>(
        "SELECT filename, text, links FROM resumes WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .map(|(filename, text, links)| (filename, Extracted { text, links })))
}

/// The original file, if it belongs to this user.
pub async fn file(
    pool: &PgPool,
    files: &Files,
    user_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<(ResumeRow, Vec<u8>)>> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT file_key FROM resumes WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some((key,)) = row else {
        return Ok(None);
    };
    let Some(info) = list(pool, user_id).await?.into_iter().find(|r| r.id == id) else {
        return Ok(None);
    };
    let bytes = files
        .0
        .get(&Path::from(key.as_str()))
        .await?
        .bytes()
        .await?;
    Ok(Some((info, bytes.to_vec())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_content() {
        assert_eq!(detect(b"%PDF-1.7 ..."), Some(Kind::Pdf));
        assert_eq!(detect(b"PK\x03\x04 not really a zip"), None);
        assert_eq!(detect(b"hello"), None);
    }

    #[test]
    fn names_from_files() {
        assert_eq!(default_name("Jane Doe Resume.pdf"), "Jane Doe Resume");
        assert_eq!(default_name("v2.final.docx"), "v2.final");
        assert_eq!(default_name(".pdf"), ".pdf");
        assert_eq!(default_name("   "), "Resume");
    }
}
