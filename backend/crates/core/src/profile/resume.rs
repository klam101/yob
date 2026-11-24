//! Uploaded resumes: checking the file type, pulling out the text, and
//! keeping the original file in local file storage.

use std::{io::Read, sync::Arc};

use chrono::{DateTime, Utc};
use object_store::{ObjectStore, ObjectStoreExt, PutPayload, local::LocalFileSystem, path::Path};
use quick_xml::events::Event;
use sqlx::PgPool;
use uuid::Uuid;

use super::{ProfileError, invalid};

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

/// The resume's text, one line per paragraph or PDF text line. CPU-bound, so
/// call it from `spawn_blocking`.
pub fn extract_text(kind: Kind, bytes: &[u8]) -> Result<String, ProfileError> {
    let text = match kind {
        Kind::Pdf => {
            // pdf-extract panics on some malformed PDFs; treat that as unreadable.
            std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(bytes))
                .ok()
                .and_then(Result::ok)
                .ok_or_else(|| {
                    invalid("Couldn't read that PDF. Try exporting it again, or upload a DOCX.")
                })?
        }
        Kind::Docx => docx_text(bytes).ok_or_else(|| {
            invalid("Couldn't read that DOCX. Try saving it again, or upload a PDF.")
        })?,
    };
    let text = text.replace('\r', "").replace('\u{a0}', " ");
    if text.split_whitespace().count() < 20 {
        return Err(invalid(
            "That file has almost no text. If it's a scanned image, upload a PDF exported from Word or Google Docs instead.",
        ));
    }
    Ok(text)
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

#[derive(Debug, Clone)]
pub struct ResumeRow {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub uploaded_at: DateTime<Utc>,
}

/// Keeps the original file and its text. Returns the new resume's id.
pub async fn save(
    pool: &PgPool,
    files: &Files,
    user_id: Uuid,
    filename: &str,
    kind: Kind,
    bytes: Vec<u8>,
    text: &str,
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    let key = format!("resumes/{user_id}/{id}.{}", kind.extension());
    let size = bytes.len() as i64;
    files
        .0
        .put(&Path::from(key.as_str()), PutPayload::from(bytes))
        .await?;
    // Only a display name: never used as a path.
    let filename: String = filename
        .chars()
        .filter(|c| !c.is_control())
        .take(200)
        .collect();
    sqlx::query(
        "INSERT INTO resumes (id, user_id, file_key, filename, content_type, size_bytes, text)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(user_id)
    .bind(&key)
    .bind(&filename)
    .bind(kind.content_type())
    .bind(size)
    .bind(text)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn list(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<ResumeRow>> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, i64, DateTime<Utc>)>(
        "SELECT id, filename, content_type, size_bytes, uploaded_at
         FROM resumes WHERE user_id = $1 ORDER BY uploaded_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ResumeRow {
            id: r.0,
            filename: r.1,
            content_type: r.2,
            size_bytes: r.3,
            uploaded_at: r.4,
        })
        .collect())
}

/// The original file, if it belongs to this user.
pub async fn file(
    pool: &PgPool,
    files: &Files,
    user_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<(ResumeRow, Vec<u8>)>> {
    let row = sqlx::query_as::<_, (String, String, String, i64, DateTime<Utc>)>(
        "SELECT file_key, filename, content_type, size_bytes, uploaded_at
         FROM resumes WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some((key, filename, content_type, size_bytes, uploaded_at)) = row else {
        return Ok(None);
    };
    let bytes = files
        .0
        .get(&Path::from(key.as_str()))
        .await?
        .bytes()
        .await?;
    Ok(Some((
        ResumeRow {
            id,
            filename,
            content_type,
            size_bytes,
            uploaded_at,
        },
        bytes.to_vec(),
    )))
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
}
