use actix_web::{post, HttpResponse, Responder};
use actix_multipart::Multipart;
use uuid::Uuid;
use std::fs::File;
use std::io::{Error, Write};
use std::path::{Path};
use std::process::Command;
use futures::StreamExt;




fn compress_video(uploaded_name: String) -> Result<(), String> {
    let input_path = format!("uploads/{}/{}", uploaded_name, uploaded_name);
    if !Path::new(&input_path).exists() {
        return Err(format!("Input file not found: {}", input_path));
    }

    let sizes = vec![
        ("high", "23"),
        ("mid", "28"),
        ("low", "35"),
    ];


    for (size_name, crf_value) in &sizes {
        let output_path = format!("uploads/{}/{}_{}.mp4", uploaded_name, uploaded_name, size_name);
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
            .arg(crf_value)
            .arg(&output_path)
            .output()
            .map_err(|e| format!("Failed to execute ffmpeg for {} quality: {}", size_name, e))?;

        if output.status.success() {
            println!("Video compressed successfully: {}", output_path);
        } else {
            return Err(format!(
                "Error compressing video for {} quality: {}",
                size_name,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    create_master_playlist(&sizes, &format!("uploads/{}", uploaded_name), &uploaded_name).unwrap();
    Ok(())
}

fn create_master_playlist(
    sizes: &Vec<(&str, &str)>,
    hls_output_dir: &str,
    uploaded_name: &str,
) -> Result<(), Error> {
    let master_playlist_path = format!("{}/master.m3u8", hls_output_dir);
    let mut master_playlist = File::create(&master_playlist_path)?;

    writeln!(master_playlist, "#EXTM3U")?;

    for (size_name, bitrate) in sizes {
        writeln!(
            master_playlist,
            "#EXT-X-STREAM-INF:BANDWIDTH={}",
            bitrate
        )?;
        writeln!(
            master_playlist,
            "{}_{}.m3u8",
            uploaded_name, size_name
        )?;
    }

    println!("Generated master playlist: {}", master_playlist_path);

    Ok(())
}

fn convert_to_hls(uploaded_name: String) -> Result<(), Box<dyn std::error::Error>> {
    let sizes = vec![
        ("high", "23"),
        ("mid", "28"),
        ("low", "35"),
    ];

    let hls_output_dir = format!("uploads/{}", uploaded_name);

    for (size_name, _) in &sizes {
        let input_path = format!("uploads/{}/{}_{}.mp4", uploaded_name, uploaded_name, size_name);

        let output_path = format!("{}/{}_{}.m3u8", hls_output_dir, uploaded_name, size_name);

        if !Path::new(&input_path).exists() {
            return Err(format!("Compressed video not found for {} quality: {}", size_name, input_path).into());
        }

        let output = Command::new("ffmpeg")
            .arg("-i")
            .arg(&input_path)
            .arg("-profile:v")
            .arg("baseline")
            .arg("-start_number")
            .arg("0")
            .arg("-hls_time")
            .arg("10")
            .arg("-hls_list_size")
            .arg("0")
            .arg("-f")
            .arg("hls")
            .arg(&output_path)
            .output()?
            .stdout;

        println!(
            "Converted {} quality video to HLS format: {}",
            size_name, output_path
        );
    }

    Ok(())

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
    // create a direcotry inside the target directory with filename
    // let target_directory = format!("{}/{}", target_directory, filename);
    // std::fs::create_dir_all(&target_directory).expect("Failed to create directory");

    while let Some(Ok(mut field)) = payload.next().await {
        let filepath = format!("{}/{}/{}", target_directory, filename, filename);
        std::fs::create_dir_all(&format!("{}/{}", target_directory, filename)).expect("Failed to create directory");
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

