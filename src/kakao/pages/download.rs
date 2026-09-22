use std::{ io::Cursor, path::Path };

use tokio::fs;

use crate::{
    error::{ Context, Result },
    kakao::{ KakaoClient, ensure_authorized, pages::page::Page },
};

impl KakaoClient {
    pub async fn download_page(&self, page: &Page, save_path: &Path) -> Result<u32> {
        let response = self
            .http()
            .get(&page.url)
            .send().await
            .context(format!("запрос изображения страницы {}", page.number))?;
        ensure_authorized(&response, &format!("страница {}", page.number))?;

        let data = response
            .bytes().await
            .context(format!("чтение изображения страницы {}", page.number))?;

        let (_, height) = image::ImageReader
            ::new(Cursor::new(&data))
            .with_guessed_format()
            .context(format!("определение формата страницы {}", page.number))?
            .into_dimensions()
            .context(format!("чтение размеров страницы {}", page.number))?;

        fs::write(save_path, data).await.context(format!("запись {}", save_path.display()))?;

        Ok(height)
    }
}
