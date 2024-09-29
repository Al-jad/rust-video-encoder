use actix_web::{
    get,
    web::Json,
};

#[get("/test")]
pub async fn get_task(
) ->  Json<String> {
    Json("Hello, world!".to_string())
}