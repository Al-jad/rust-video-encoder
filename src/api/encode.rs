use actix_web::{post, HttpRequest, web::Payload, HttpResponse, Error};
use actix_multipart::Multipart;
use uuid::Uuid;
use std::fs;
use std::io::Write;
use futures::StreamExt;

#[utoipa::path(
    post,
    path = "/video",
    request_body(content = FileUpload, content_type = "multipart/form-data", description = "Upload video file"),
    responses(
        (status = 200, description = "File uploaded successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/video")]
pub async fn upload_video(
    req: HttpRequest, bytes: Payload
) -> Result<HttpResponse, Error> {
    let mut multipart = Multipart::new(req.headers(), bytes);
    let name_id = Uuid::new_v4();
    let uploaded_name = format!("{}", name_id);
    let mut upload_file = fs::File::create(format!("uploads/{}", uploaded_name))?;

    while let Some(chunk) = multipart.next().await {
        let mut chunk = chunk?;
        for chunk_content in chunk.next().await {
            let content = chunk_content.ok().unwrap_or_default();
            upload_file.write(&content)?;
        }
    }

    Ok(HttpResponse::Ok().content_type("text/plain").body(uploaded_name))
}

