mod api;
use crate::api::task::__path_get_task;
use api::task::get_task;
use actix_web::{App, HttpServer, middleware::Logger};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // Derive the OpenApi struct with the properly registered path
    #[derive(OpenApi)]
    #[openapi(
        paths(
            get_task
        )
    )]
    struct ApiDoc;

    let openapi = ApiDoc::openapi();
    HttpServer::new(move || {
        let logger = Logger::default();
        App::new()
            .wrap(logger)
            .service(get_task)
            .service(SwaggerUi::new("/docs/{_:.*}").url(
                "/api-docs/openapi.json",
                openapi.clone(),
            ))
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
