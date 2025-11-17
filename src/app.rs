use crate::api_docs::ApiDoc;
use crate::config::APP_CONFIG;
use crate::middleware::http_logger::http_logger;
use crate::routes;
use crate::routes::health::route::create_route;
use axum::Router;
use axum::middleware;
use http::header;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{ServiceBuilderExt, propagate_header::PropagateHeaderLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub async fn create_app() -> anyhow::Result<Router> {
    let mut router = Router::new()
        .merge(create_route())
        .merge(routes::auth::create_route())
        .merge(routes::profile::create_route())
        .merge(routes::users::create_route())
        .merge(routes::departments::create_route())
        .merge(routes::majors::create_route())
        .merge(routes::managers::create_route())
        .merge(routes::students::create_route())
        .merge(routes::upload::route::create_route())
        .merge(routes::user_mfa::route::create_route())
        .merge(routes::documents::create_route())
        .merge(routes::requests::create_route());

    // Add Swagger UI
    if APP_CONFIG.swagger_enabled {
        let swagger_ui =
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());
        router = router.merge(swagger_ui);
    }

    // Apply middleware
    let sensitive_headers: Arc<[_]> = vec![header::AUTHORIZATION, header::COOKIE].into();

    // Axum middleware (middleware::from_fn) must be applied separately from ServiceBuilder
    // ServiceBuilder only works with Tower layers, not Axum middleware
    eprintln!("🔧 Applying HTTP logger middleware to router...");
    let router = router.layer(middleware::from_fn(http_logger));
    eprintln!("✅ HTTP logger middleware applied successfully!");

    // Apply Tower middleware stack
    let middleware = ServiceBuilder::new()
        .layer(PropagateHeaderLayer::new(header::HeaderName::from_static(
            "x-request-id",
        )))
        .sensitive_request_headers(sensitive_headers.clone())
        .sensitive_response_headers(sensitive_headers)
        .compression();

    Ok(router.layer(middleware))
}
