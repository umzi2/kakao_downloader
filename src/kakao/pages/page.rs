use crate::kakao::pages::response::ImageFile;

#[derive(Debug)]
pub struct Page {
    pub number: usize,
    pub url: String,
}

impl From<ImageFile> for Page {
    fn from(file: ImageFile) -> Self {
        Self {
            number: file.no,
            url: file.secure_url,
        }
    }
}
