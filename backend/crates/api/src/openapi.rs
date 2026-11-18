use utoipa::OpenApi;

/// Root of the OpenAPI spec served at `/openapi.json`. Handlers add their own
/// paths through `utoipa_axum::routes!`, so nothing needs listing here.
/// The web app generates its TypeScript types from this spec (`npm run typegen`).
#[derive(OpenApi)]
#[openapi(info(title = "yob API", description = "Personal job search app"))]
pub struct ApiDoc;
