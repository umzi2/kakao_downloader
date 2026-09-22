use crate::{
    error::{ Context, Error, Result },
    kakao::{ BFF_BASE_URL, KakaoClient, describe_api_error, ensure_authorized },
};

const RENTAL_TICKET_TYPE: &str = "RT05";

impl KakaoClient {
    pub async fn rent_chapter(&self, chapter_id: usize) -> Result<bool> {
        let response = self
            .http()
            .post(format!("{BFF_BASE_URL}/api/gateway/api/v1/ticket/use"))
            .form(
                &[
                    ("product_id", chapter_id.to_string()),
                    ("ticket_type", RENTAL_TICKET_TYPE.to_owned()),
                ]
            )
            .send().await
            .context(format!("запрос аренды главы {chapter_id}"))?;
        ensure_authorized(&response, &format!("аренда главы {chapter_id}"))?;

        let status = response.status();
        let body = response
            .text().await
            .context(format!("чтение ответа аренды главы {chapter_id}"))?;

        if !status.is_success() {
            return Err(
                Error::rejected(
                    format!("аренда главы {chapter_id}: {}", describe_api_error(&body, status))
                )
            );
        }

        Ok(true)
    }
}
