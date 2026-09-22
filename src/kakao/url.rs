use reqwest::Url;

use crate::error::{ Error, Result };

pub fn parse_url(input: &str) -> Result<Url> {
    Url::parse(input).map_err(|source| Error::new(format!("разбор URL {input}"), source))
}

pub fn parse_title_id(input: &str) -> Result<usize> {
    let trimmed = input.trim();

    let digits = match trimmed.find("/content/") {
        Some(position) => {
            let tail = &trimmed[position + "/content/".len()..];
            tail.split(['/', '?', '#']).next().unwrap_or_default()
        }
        None => trimmed,
    };

    digits
        .trim()
        .parse::<usize>()
        .map_err(|_|
            Error::unexpected(
                format!(
                    "не удалось получить id тайтла из {trimmed:?}: ожидается ссылка вида https://page.kakao.com/content/54801072/"
                )
            )
        )
}
