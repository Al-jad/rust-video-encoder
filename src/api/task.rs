use actix_web::{get, web::Json, Responder};

#[utoipa::path(
    get,
    path = "/test",
    responses(
        (status = 200, description = "Task found from database", body = String)
    )
)]
#[get("/test")]
pub async fn get_task() -> impl Responder {
    Json("Hello, world!".to_string())
}
