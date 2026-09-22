use crate::{
    error::{ Context, Result },
    kakao::{
        BFF_BASE_URL,
        KakaoClient,
        chapters::{ chapter::Chapter, response::ChapterList },
        decode,
        ensure_authorized,
        url::parse_url,
    },
};

const PAGE_SIZE: usize = 25;

pub struct ChapterPage {
    pub chapters: Vec<Chapter>,
    pub next_cursor: Option<usize>,
    pub total: usize,
}

impl KakaoClient {
    pub async fn chapter_page(&self, title_id: usize, cursor_index: usize) -> Result<ChapterPage> {
        let url = parse_url(
            &format!(
                "{BFF_BASE_URL}/api/gateway/api/v2/content/product/list?series_id={title_id}&cursor_index={cursor_index}&cursor_direction=NEXT&window_size={PAGE_SIZE}&sort_type=asc"
            )
        )?;

        let response = self
            .http()
            .get(url)
            .send().await
            .context(format!("запрос списка глав с cursor_index={cursor_index}"))?;
        ensure_authorized(&response, "список глав")?;

        let response = decode::<ChapterList>(
            response,
            &format!("список глав, cursor_index={cursor_index}")
        ).await?;

        let total = response.result.total_count;
        let chapters: Vec<Chapter> = response.result.list.into_iter().map(Into::into).collect();
        let loaded = cursor_index + chapters.len();
        let next_cursor = (loaded < total && !chapters.is_empty()).then_some(loaded);

        Ok(ChapterPage {
            chapters,
            next_cursor,
            total,
        })
    }

    pub async fn chapter(&self, title_id: usize, chapter_index: usize) -> Result<Chapter> {
        let response = self
            .http()
            .get(
                format!(
                    "{BFF_BASE_URL}/api/gateway/api/v2/content/product/list?series_id={title_id}&cursor_index={}&cursor_direction=NEXT&window_size=1&sort_type=asc",
                    chapter_index.saturating_sub(1)
                )
            )
            .send().await
            .context(format!("запрос главы {chapter_index} произведения {title_id}"))?;
        ensure_authorized(&response, &format!("глава {chapter_index}"))?;

        let response = decode::<ChapterList>(
            response,
            &format!("глава {chapter_index} произведения {title_id}")
        ).await?;

        response.result.list
            .into_iter()
            .next()
            .map(Chapter::from)
            .context(format!("глава {chapter_index} произведения {title_id} не найдена"))
    }
}
