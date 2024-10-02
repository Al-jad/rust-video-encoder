use actix_web::{post, HttpResponse, Responder};
use actix_multipart::Multipart;
use uuid::Uuid;
use std::fs::File;
use std::io::Write;
use std::path::{Path};
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
        .arg("+genpts")
        .arg("-i")
        .arg(&input_path)
        .arg("-analyzeduration")
        .arg("100M")
        .arg("-probesize")
        .arg("50M")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-vcodec")
        .arg("libx264")
        .arg("-crf")
        .arg("28")
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

fn convert_to_hls(uploaded_name: String) -> Result<(), Box<dyn std::error::Error>> {
    let input_path = format!("temp_results/{}.mp4", uploaded_name);
    let output_path = format!("hls/{}.m3u8", uploaded_name);


    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(&input_path)
        .arg("-preset")
        .arg("veryfast")
        .arg("-g")
        .arg("30")
        .arg("-sc_threshold")
        .arg("0")
        .arg("-f")
        .arg("hls")
        .arg("-hls_time")
        .arg("10") // Segment length in seconds
        .arg("-hls_list_size")
        .arg("0") // Infinite playlist
        .arg("-hls_flags")
        .arg("delete_segments")
        .arg(&output_path)
        .status()?;

    // Check if FFmpeg executed successfully
    if !status.success() {
        eprintln!("FFmpeg failed to convert the video.");
        return Err("FFmpeg conversion failed".into());
    }

    println!("Video successfully converted to HLS format.");
    Ok(())
}




//
//
//
// fn transcode_video_into_HLS(uploaded_name: String) -> std::process::Output {
//     let output = std::process::Command::new("ffmpeg")
//         .args(&["-i", format!("uploads/{}.mp4", uploaded_name).as_str(), "-c:v", "libx264", "-crf", "24", "-c:a", "aac", "-strict", "experimental", "-f", "hls", format!("uploads/{}.m3u8", uploaded_name).as_str()])
//         .output()
//         .expect("failed to execute process");
//     output
// }
//
// fn upload_video_to_s3(uploaded_name: String) -> std::process::Output {
//     let output = std::process::Command::new("aws")
//         .args(&["s3", "cp", format!("uploads/{}.m3u8", uploaded_name).as_str(), "s3://bucket-name"])
//         .output()
//         .expect("failed to execute process");
//     output
// }



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

    if let Err(e) = convert_to_hls(filename.to_string()) {
        eprintln!("Error: {}", e);
    }

    HttpResponse::Ok().body("Video uploaded successfully!")

}

