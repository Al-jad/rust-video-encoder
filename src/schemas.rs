#[derive(utoipa::ToSchema)]
pub struct FileUpload {
    #[schema(format = "binary")]
    file: String,
}
