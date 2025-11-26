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
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    propagate_header::PropagateHeaderLayer,
    ServiceBuilderExt,
};
use std::collections::HashSet;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub async fn create_app() -> anyhow::Result<Router> {
    let mut router = Router::new()
        .merge(create_route())
        .merge(routes::auth::create_route())
        .merge(routes::profile::create_route())
        .merge(routes::users::create_route())
        .merge(routes::stats::route::create_route())
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

    // Configure CORS
    // Common allowed headers and methods
    let allowed_headers = [
        header::CONTENT_TYPE,
        header::AUTHORIZATION,
        header::ACCEPT,
        header::ACCEPT_LANGUAGE,
    ];
    
    let allowed_methods = [
        http::Method::GET,
        http::Method::POST,
        http::Method::PUT,
        http::Method::DELETE,
        http::Method::PATCH,
        http::Method::OPTIONS,
    ];

    let cors_layer = if APP_CONFIG.cors_allowed_origins == "*" {
        // When allowing all origins (*), we cannot use credentials (CORS spec limitation)
        // If you need credentials, specify origins explicitly instead of using *
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(allowed_methods)
            .allow_headers(allowed_headers)
            .allow_credentials(false)
    } else {
        let allowed_origins: HashSet<String> = APP_CONFIG
            .cors_allowed_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let origins: Vec<http::HeaderValue> = allowed_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect();

        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(allowed_methods)
            .allow_headers(allowed_headers)
            .allow_credentials(true)
    };

    // Apply Tower middleware stack
    let middleware = ServiceBuilder::new()
        .layer(cors_layer)
        .layer(PropagateHeaderLayer::new(header::HeaderName::from_static(
            "x-request-id",
        )))
        .sensitive_request_headers(sensitive_headers.clone())
        .sensitive_response_headers(sensitive_headers)
        .compression();

    Ok(router.layer(middleware))
}
