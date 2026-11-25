//! The profile: read it whole, save it a section at a time, and turn an
//! uploaded resume into a draft to review. Every route needs a session.

use axum::http::StatusCode;
use axum::{
    Json,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderValue, header},
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{
    router::{OpenApiRouter, UtoipaMethodRouterExt},
    routes,
};
use uuid::Uuid;
use yob_core::profile::{
    self, Answers, Education, Experience, Link, Personal, Profile, Project, Skill, parse,
    resume::{self, MAX_BYTES, ResumeRow, ResumeUpdate},
};

use super::auth::CurrentUser;
use crate::{error::ApiError, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_profile))
        .routes(routes!(put_personal))
        .routes(routes!(put_links))
        .routes(routes!(put_educations))
        .routes(routes!(put_experiences))
        .routes(routes!(put_projects))
        .routes(routes!(put_skills))
        .routes(routes!(put_answers))
        .routes(routes!(list_resumes))
        .routes(routes!(update_resume, delete_resume))
        .routes(routes!(resume_draft))
        .routes(routes!(resume_file))
        // The multipart body is up to 5 MB plus form overhead.
        .routes(routes!(upload_resume).layer(DefaultBodyLimit::max(MAX_BYTES + 64 * 1024)))
}

/// Everything in the profile.
#[utoipa::path(
    get,
    path = "/profile",
    tag = "profile",
    responses((status = OK, body = Profile), (status = UNAUTHORIZED, body = ApiError))
)]
async fn get_profile(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Profile>, ApiError> {
    Ok(Json(
        profile::load(&state.db, &state.profile_key, user.id).await?,
    ))
}

/// Saves name, contact details, headline and summary.
#[utoipa::path(
    put,
    path = "/profile/personal",
    tag = "profile",
    request_body = Personal,
    responses((status = OK, body = Personal), (status = BAD_REQUEST, body = ApiError))
)]
async fn put_personal(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Personal>,
) -> Result<Json<Personal>, ApiError> {
    Ok(Json(
        profile::save_personal(&state.db, user.id, &body).await?,
    ))
}

/// Replaces the links, in the order given.
#[utoipa::path(
    put,
    path = "/profile/links",
    tag = "profile",
    request_body = Vec<Link>,
    responses((status = OK, body = Vec<Link>), (status = BAD_REQUEST, body = ApiError))
)]
async fn put_links(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Vec<Link>>,
) -> Result<Json<Vec<Link>>, ApiError> {
    Ok(Json(profile::save_links(&state.db, user.id, &body).await?))
}

/// Replaces the education entries, in the order given.
#[utoipa::path(
    put,
    path = "/profile/educations",
    tag = "profile",
    request_body = Vec<Education>,
    responses((status = OK, body = Vec<Education>), (status = BAD_REQUEST, body = ApiError))
)]
async fn put_educations(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Vec<Education>>,
) -> Result<Json<Vec<Education>>, ApiError> {
    Ok(Json(
        profile::save_educations(&state.db, user.id, &body).await?,
    ))
}

/// Replaces the work experience, in the order given.
#[utoipa::path(
    put,
    path = "/profile/experiences",
    tag = "profile",
    request_body = Vec<Experience>,
    responses((status = OK, body = Vec<Experience>), (status = BAD_REQUEST, body = ApiError))
)]
async fn put_experiences(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Vec<Experience>>,
) -> Result<Json<Vec<Experience>>, ApiError> {
    Ok(Json(
        profile::save_experiences(&state.db, user.id, &body).await?,
    ))
}

/// Replaces the projects, in the order given.
#[utoipa::path(
    put,
    path = "/profile/projects",
    tag = "profile",
    request_body = Vec<Project>,
    responses((status = OK, body = Vec<Project>), (status = BAD_REQUEST, body = ApiError))
)]
async fn put_projects(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Vec<Project>>,
) -> Result<Json<Vec<Project>>, ApiError> {
    Ok(Json(
        profile::save_projects(&state.db, user.id, &body).await?,
    ))
}

/// Replaces the skills, in the order given. Duplicates are dropped.
#[utoipa::path(
    put,
    path = "/profile/skills",
    tag = "profile",
    request_body = Vec<Skill>,
    responses((status = OK, body = Vec<Skill>), (status = BAD_REQUEST, body = ApiError))
)]
async fn put_skills(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Vec<Skill>>,
) -> Result<Json<Vec<Skill>>, ApiError> {
    Ok(Json(profile::save_skills(&state.db, user.id, &body).await?))
}

/// Saves work authorization and the (encrypted) equal-employment answers.
#[utoipa::path(
    put,
    path = "/profile/answers",
    tag = "profile",
    request_body = Answers,
    responses((status = OK, body = Answers), (status = BAD_REQUEST, body = ApiError))
)]
async fn put_answers(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Answers>,
) -> Result<Json<Answers>, ApiError> {
    Ok(Json(
        profile::save_answers(&state.db, &state.profile_key, user.id, &body).await?,
    ))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResumeDraft {
    pub resume_id: String,
    pub filename: String,
    /// What the parser read. Nothing is saved until the sections are PUT.
    pub profile: Profile,
    /// Lines it couldn't place, for the user to copy where they belong.
    pub unsorted: Vec<String>,
}

#[derive(ToSchema)]
#[allow(dead_code)]
pub struct ResumeUpload {
    /// A PDF or DOCX, at most 5 MB.
    #[schema(value_type = String, format = Binary)]
    file: Vec<u8>,
}

/// Uploads a resume (PDF or DOCX) and returns a draft profile read from it.
/// The file is kept; the profile isn't changed.
#[utoipa::path(
    post,
    path = "/profile/resume",
    tag = "profile",
    request_body(content = ResumeUpload, content_type = "multipart/form-data"),
    responses(
        (status = OK, body = ResumeDraft),
        (status = BAD_REQUEST, body = ApiError),
        (status = PAYLOAD_TOO_LARGE),
    )
)]
async fn upload_resume(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    mut form: Multipart,
) -> Result<Json<ResumeDraft>, ApiError> {
    let mut upload = None;
    while let Some(field) = form
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Couldn't read the upload: {e}")))?
    {
        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("resume").to_owned();
            let bytes = field
                .bytes()
                .await
                .map_err(|_| ApiError::bad_request("The file is larger than 5 MB."))?;
            upload = Some((filename, bytes.to_vec()));
        }
    }
    let (filename, bytes) =
        upload.ok_or_else(|| ApiError::bad_request("Choose a file to upload."))?;
    if bytes.len() > MAX_BYTES {
        return Err(ApiError::bad_request("The file is larger than 5 MB."));
    }
    let kind = resume::detect(&bytes)
        .ok_or_else(|| ApiError::bad_request("Upload a PDF or a Word document (.docx)."))?;

    // Text extraction and parsing are CPU work.
    let (bytes, extracted, parsed) = tokio::task::spawn_blocking(move || {
        let extracted = resume::extract(kind, &bytes)?;
        let parsed = parse::parse(&extracted.text, &extracted.links);
        Ok::<_, profile::ProfileError>((bytes, extracted, parsed))
    })
    .await
    .map_err(|_| ApiError::internal("Reading the resume failed"))??;

    let id = resume::save(
        &state.db,
        &state.files,
        user.id,
        &filename,
        kind,
        bytes,
        &extracted,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "saving resume");
        ApiError::internal("Couldn't store the file")
    })?;
    tracing::info!(
        experiences = parsed.profile.experiences.len(),
        educations = parsed.profile.educations.len(),
        skills = parsed.profile.skills.len(),
        unsorted = parsed.unsorted.len(),
        "parsed resume"
    );
    Ok(Json(ResumeDraft {
        resume_id: id.to_string(),
        filename,
        profile: parsed.profile,
        unsorted: parsed.unsorted,
    }))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResumeInfo {
    pub id: String,
    /// Shown in the list; starts as the file name without its extension.
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub target_title: String,
    /// The resume used by default. Exactly one when there are any.
    pub is_primary: bool,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ResumeRow> for ResumeInfo {
    fn from(r: ResumeRow) -> Self {
        Self {
            id: r.id.to_string(),
            name: r.name,
            filename: r.filename,
            content_type: r.content_type,
            size_bytes: r.size_bytes,
            target_title: r.target_title,
            is_primary: r.is_primary,
            uploaded_at: r.uploaded_at,
            updated_at: r.updated_at,
        }
    }
}

fn resume_id(id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(id).map_err(|_| ApiError::not_found("No such resume"))
}

/// Uploaded resumes: the primary one first, then newest first.
#[utoipa::path(
    get,
    path = "/profile/resumes",
    tag = "profile",
    responses((status = OK, body = Vec<ResumeInfo>))
)]
async fn list_resumes(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<ResumeInfo>>, ApiError> {
    let rows = resume::list(&state.db, user.id).await?;
    Ok(Json(rows.into_iter().map(ResumeInfo::from).collect()))
}

/// Renames a resume, sets its target job title, or makes it the primary one.
#[utoipa::path(
    patch,
    path = "/profile/resumes/{id}",
    tag = "profile",
    params(("id" = String, Path)),
    request_body = ResumeUpdate,
    responses(
        (status = OK, body = ResumeInfo),
        (status = BAD_REQUEST, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
    )
)]
async fn update_resume(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
    Json(change): Json<ResumeUpdate>,
) -> Result<Json<ResumeInfo>, ApiError> {
    let row = resume::update(&state.db, user.id, resume_id(&id)?, &change)
        .await?
        .ok_or_else(|| ApiError::not_found("No such resume"))?;
    Ok(Json(row.into()))
}

/// Deletes a resume and its file. If it was the primary one, the newest
/// remaining resume becomes primary. The profile isn't changed.
#[utoipa::path(
    delete,
    path = "/profile/resumes/{id}",
    tag = "profile",
    params(("id" = String, Path)),
    responses((status = NO_CONTENT), (status = NOT_FOUND, body = ApiError))
)]
async fn delete_resume(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = resume::delete(&state.db, &state.files, user.id, resume_id(&id)?)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "deleting resume");
            ApiError::internal("Couldn't delete the resume")
        })?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("No such resume"))
    }
}

/// Reads a saved resume again and returns a draft profile from it, to fill
/// the profile from that resume. Nothing is saved.
#[utoipa::path(
    get,
    path = "/profile/resumes/{id}/draft",
    tag = "profile",
    params(("id" = String, Path)),
    responses((status = OK, body = ResumeDraft), (status = NOT_FOUND, body = ApiError))
)]
async fn resume_draft(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<ResumeDraft>, ApiError> {
    let id = resume_id(&id)?;
    let (filename, stored) = resume::extracted(&state.db, user.id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("No such resume"))?;
    // Read the original file again when it's there, so resumes uploaded
    // before an extraction fix get it too; otherwise use the stored text.
    let bytes = match resume::file(&state.db, &state.files, user.id, id).await {
        Ok(found) => found.map(|(_, bytes)| bytes),
        Err(e) => {
            tracing::warn!(error = %e, "reading resume file, using its stored text");
            None
        }
    };
    let parsed = tokio::task::spawn_blocking(move || {
        let extracted = bytes
            .and_then(|b| resume::detect(&b).and_then(|k| resume::extract(k, &b).ok()))
            .unwrap_or(stored);
        parse::parse(&extracted.text, &extracted.links)
    })
    .await
    .map_err(|_| ApiError::internal("Reading the resume failed"))?;
    Ok(Json(ResumeDraft {
        resume_id: id.to_string(),
        filename,
        profile: parsed.profile,
        unsorted: parsed.unsorted,
    }))
}

/// Downloads an uploaded resume.
#[utoipa::path(
    get,
    path = "/profile/resumes/{id}/file",
    tag = "profile",
    params(("id" = String, Path)),
    responses((status = OK, content_type = "application/octet-stream"), (status = NOT_FOUND, body = ApiError))
)]
async fn resume_file(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let not_found = || ApiError::not_found("No such resume");
    let id = resume_id(&id)?;
    let (row, bytes) = resume::file(&state.db, &state.files, user.id, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "reading resume file");
            ApiError::internal("Couldn't read the file")
        })?
        .ok_or_else(not_found)?;
    // ASCII-only name for the header; the original stays in the database.
    let safe: String = row
        .filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut resp = Response::new(Body::from(bytes));
    let headers = resp.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&row.content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{safe}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, v);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(resp)
}
