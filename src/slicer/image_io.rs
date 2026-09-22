use std::{ io::Cursor, path::Path };

use image::{ GrayImage, ImageFormat, RgbImage };
use tokio::fs;

use crate::error::{ Context, Result };

pub async fn read_rgb8(path: &Path) -> Result<RgbImage> {
    let data = fs::read(path).await.context(format!("чтение {}", path.display()))?;
    let source = path.to_path_buf();

    tokio::task
        ::spawn_blocking(move || {
            image
                ::load_from_memory(&data)
                .map(|image| image.to_rgb8())
                .context(format!("декодирование {}", source.display()))
        }).await
        .context("задача декодирования изображения")?
}

pub async fn read_luma8(path: &Path) -> Result<GrayImage> {
    let data = fs::read(path).await.context(format!("чтение {}", path.display()))?;

    image
        ::load_from_memory(&data)
        .map(|image| image.to_luma8())
        .context(format!("декодирование {}", path.display()))
}

pub async fn save_png(path: &Path, image: RgbImage) -> Result<()> {
    let bytes = tokio::task
        ::spawn_blocking(
            move || -> Result<Vec<u8>> {
                let mut buffer = Vec::new();
                image
                    .write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)
                    .context("кодирование PNG")?;
                Ok(buffer)
            }
        ).await
        .context("задача кодирования PNG")??;

    fs::write(path, bytes).await.context(format!("запись {}", path.display()))
}
