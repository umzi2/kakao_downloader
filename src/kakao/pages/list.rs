use crate::{
    error::{ Context, Result },
    kakao::{
        BFF_BASE_URL,
        KakaoClient,
        decode,
        ensure_authorized,
        pages::{ page::Page, response::ViewerResponse },
    },
};

impl KakaoClient {
    pub async fn page_list(&self, title_id: usize, chapter_id: usize) -> Result<Vec<Page>> {
        let response = self
            .http()
            .get(
                format!(
                    "{BFF_BASE_URL}/api/gateway/api/v1/viewer/data?series_id={title_id}&product_id={chapter_id}"
                )
            )
            .send().await
            .context(format!("запрос страниц главы {chapter_id}"))?;
        ensure_authorized(&response, &format!("страницы главы {chapter_id}"))?;

        let response = decode::<ViewerResponse>(
            response,
            &format!("страницы главы {chapter_id}")
        ).await?;

        let mut pages: Vec<Page> = response.viewer_data.image_download_data.files
            .into_iter()
            .map(Into::into)
            .collect();
        pages.sort_by_key(|page| page.number);
        Ok(pages)
    }
}
