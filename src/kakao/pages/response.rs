use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ViewerResponse {
    pub viewer_data: Viewer,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewer {
    pub image_download_data: ImageDownload,
}

#[derive(Debug, Deserialize)]
pub struct ImageDownload {
    pub files: Vec<ImageFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageFile {
    pub no: usize,
    pub secure_url: String,
}
