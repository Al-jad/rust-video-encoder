use std::{env, fs};
use actix_web::{post, HttpResponse, Responder};
use actix_multipart::Multipart;
use uuid::Uuid;
use std::fs::File;
use std::io::{Error, Write};
use std::path::{Path};
use std::process::Command;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use dotenv::dotenv;
use futures::StreamExt;
use tokio::io::AsyncReadExt;
use tokio::task;

struct Config {
    s3_bucket_name: String,
    aws_region: String,
}

impl Config {
    fn from_env() -> Self {
        dotenv().ok();

        Config {
            s3_bucket_name: env::var("s3_bucket_name").expect("s3_bucket_name must be set"),
            aws_region: env::var("AWS_REGION").expect("aws_region must be set"),
        }
    }
}



pub struct S3Constants {
    pub bucket_name: String,
    pub aws_region: String,
}

pub async fn upload_folder_to_s3(folder_path: &str, file_unique_key: &str, constants: S3Constants) -> Result<String, Box<dyn std::error::Error>> {
    let shared_config = aws_config::load_from_env().await;
    let s3_client = Client::new(&shared_config);

    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            continue;
        }
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        let mut file = tokio::fs::File::open(&path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let s3_key = format!("videos/{}/{}", file_unique_key, file_name);

        s3_client
            .put_object()
            .bucket(&constants.bucket_name)
            .key(&s3_key)
            .body(ByteStream::from(buffer))
            .acl("public-read".parse().unwrap())
            .send()
            .await?;
    }

    let generated_link = format!(
        "https://{}.s3.{}.amazonaws.com/videos/{}/master.m3u8",
        constants.bucket_name, constants.aws_region, file_unique_key
    );

    Ok(generated_link)
}



fn generate_thumbnails(uploaded_name: String) -> Result<(), String> {
    let input_path = format!("uploads/{}/{}", uploaded_name, uploaded_name);
    let thumbnail_output_pattern = format!("uploads/{}/thumbnail-%02d.png", uploaded_name);

    if !Path::new(&input_path).exists() {
        return Err(format!("Input file not found: {}", input_path));
    }

    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(&input_path)
        .arg("-vf")
        .arg("thumbnail,select='not(mod(n,50))'")
        .arg("-frames:v")
        .arg("5")
        .arg(&thumbnail_output_pattern)
        .output()
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    if output.status.success() {
        println!("Thumbnails generated successfully in: {}", thumbnail_output_pattern);
        Ok(())
    } else {
        Err(format!(
            "Error generating thumbnails: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

async fn compress_video(uploaded_name: String) -> Result<(), String> {
    let input_path = format!("uploads/{}/{}", uploaded_name, uploaded_name);
    if !Path::new(&input_path).exists() {
        return Err(format!("Input file not found: {}", input_path));
    }

    let sizes = vec![
        ("high", "23"),
        ("mid", "28"),
        ("low", "35"),
    ];


    let mut tasks = vec![];
    for (size_name, crf_value) in &sizes {
        let input_path = input_path.clone();
        let uploaded_name = uploaded_name.clone();
        let size_name = size_name.to_string();
        let crf_value = crf_value.to_string();
        tasks.push(task::spawn(async move {
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
                Ok(())
            } else {
                Err(format!(
                    "Error compressing video for {} quality: {}",
                    size_name,
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }));
    }

    let results = futures::future::join_all(tasks).await;
    for result in results {
        if let Err(e) = result.unwrap() {
            return Err(e);
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

        Command::new("ffmpeg")
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
    let config = Config::from_env();
    while let Some(Ok(mut field)) = payload.next().await {
        let filepath = format!("{}/{}/{}", target_directory, filename, filename);
        std::fs::create_dir_all(&format!("{}/{}", target_directory, filename)).expect("Failed to create directory");
        let mut file = File::create(filepath).expect("Failed to create file");
        while let Some(Ok(bytes)) = field.next().await {
            file.write_all(&bytes).expect("Failed to write bytes to file");
        }
    }

    match compress_video(filename.to_string()).await {
        Ok(_) => println!("Video compressed successfully"),
        Err(e) => {
            println!("Error compressing video: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    }

    if let Err(e) = convert_to_hls(filename.to_string()) {
        eprintln!("Error: {}", e);
    }

    match generate_thumbnails(filename.to_string()) {
        Ok(_) => println!("Thumbnails generated successfully"),
        Err(e) => {
            println!("Error generating thumbnails: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    }

    println!("S3 bucket name: {}", config.s3_bucket_name);

    let link = upload_folder_to_s3(&format!("uploads/{}", filename), &filename.to_string(), S3Constants {
        bucket_name: config.s3_bucket_name,
        aws_region: config.aws_region,
    }).await.unwrap();

    fs::remove_dir_all(format!("uploads/{}", filename)).expect("Failed to remove directory");
    HttpResponse::Ok().body(link)
}

