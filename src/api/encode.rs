use actix_web::{post, HttpResponse, Responder};
use actix_multipart::Multipart;
use uuid::Uuid;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use futures::StreamExt;


fn compress_video(uploaded_name: String) -> Result<(), String> {
    let input_path = format!("uploads/{}", uploaded_name);
    let output_path = format!("temp_results/{}.mp4", uploaded_name);
    if !Path::new(&input_path).exists() {
        return Err(format!("Input file not found: {}", input_path));
    }
    let output = Command::new("ffmpeg")
        .arg("-fflags")
        .arg("+genpts")  // Generate missing presentation timestamps
        .arg("-i")
        .arg(&input_path)
        .arg("-analyzeduration")
        .arg("100M")  // Increase analyzeduration
        .arg("-probesize")
        .arg("50M")  // Increase probesize
        .arg("-pix_fmt")
        .arg("yuv420p")  // Set pixel format explicitly
        .arg("-vcodec")
        .arg("libx264")  // Use H.264 codec
        .arg("-crf")
        .arg("28")  // Constant rate factor for compression (higher means more compression)
        .arg(&output_path)
        .output()
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;
    if output.status.success() {
        println!("Video compressed successfully: {}", output_path);
        Ok(())
    } else {
        Err(format!(
            "Error compressing video: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}



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
pub async fn upload_video(mut payload: Multipart) -> impl Responder {

    let target_directory = "uploads";
    let filename = Uuid::new_v4();
    while let Some(Ok(mut field)) = payload.next().await {
        let filepath = format!("{}/{}", target_directory, filename);
        let mut file = File::create(filepath).expect("Failed to create file");
        while let Some(Ok(bytes)) = field.next().await {
            file.write_all(&bytes).expect("Failed to write bytes to file");
        }
    }

    match compress_video(filename.to_string()) {
            Ok(_) => println!("Video compressed successfully"),
            Err(e) => {
                println!("Error compressing video: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }


    HttpResponse::Ok().body("Video uploaded successfully!")

}

