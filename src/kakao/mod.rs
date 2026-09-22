pub mod chapters;
mod cookies;
pub mod pages;
pub mod url;

use std::{ sync::Arc, time::Duration };

use reqwest::{
    Client,
    ClientBuilder,
    StatusCode,
    Url,
    cookie::Jar,
    header::{ HeaderMap, HeaderValue },
};
use serde::{ Deserialize, de::DeserializeOwned };
use tokio::{ sync::mpsc::UnboundedSender, task::JoinHandle };

use crate::{
    config::{ CONFIG_PATH, Config },
    error::{ Context, Error, Result },
    kakao::url::parse_url,
};

pub fn ensure_authorized(response: &reqwest::Response, context: &str) -> Result<()> {
    if matches!(response.status(), StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        return Err(
            Error::unauthorized(
                format!("{context}: сервер ответил {} — нужно войти заново", response.status())
            )
        );
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ApiError {
    result_code: i64,
    #[serde(default)]
    message: String,
    #[serde(default)]
    message_key: String,
}

pub async fn decode<T: DeserializeOwned>(response: reqwest::Response, context: &str) -> Result<T> {
    let status = response.status();
    let body = response.text().await.context(format!("{context}: чтение ответа"))?;

    match serde_json::from_str::<T>(&body) {
        Ok(value) => Ok(value),
        Err(_) =>
            Err(Error::unexpected(format!("{context}: {}", describe_api_error(&body, status)))),
    }
}

pub fn describe_api_error(body: &str, status: StatusCode) -> String {
    match serde_json::from_str::<ApiError>(body) {
        Ok(api) => {
            let hint = match api.message_key.as_str() {
                "api_content_not_purchased_item" => " — глава не куплена и не арендована",
                _ => "",
            };
            format!("{} ({}, result_code {}){hint}", api.message, api.message_key, api.result_code)
        }
        Err(_) => {
            let body: String = body.chars().take(200).collect();
            format!("HTTP {status}, неожиданный ответ: {body}")
        }
    }
}

const BASE_URL: &str = "https://page.kakao.com";

pub const BFF_BASE_URL: &str = "https://bff-page.kakao.com";

const REFRESH_TOKEN_URL: &str = "https://bff-page.kakao.com/api/refresh_token";

const TOKEN_REFRESH_INTERVAL: Duration = Duration::from_secs(7200);

const REFRESH_ATTEMPTS: usize = 3;

const REFRESH_RETRY_DELAY: Duration = Duration::from_secs(2);

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:153.0) Gecko/20100101 Firefox/153.0";

#[derive(Debug, Clone)]
pub enum ClientEvent {
    TokenRefreshed,
    RefreshFailed(String),
    CookieWarning(String),
}

pub struct KakaoClient {
    http: Client,
    refresh_task: JoinHandle<()>,
}

impl Drop for KakaoClient {
    fn drop(&mut self) {
        self.refresh_task.abort();
    }
}

impl KakaoClient {
    pub fn new(config: &Config, events: Option<UnboundedSender<ClientEvent>>) -> Result<Self> {
        let base_url = parse_url(BASE_URL)?;
        let cookie_jar = Arc::new(Jar::default());

        for warning in cookies::load_into_jar(&cookie_jar, &base_url, &config.cookie) {
            match &events {
                Some(sender) => {
                    let _ = sender.send(ClientEvent::CookieWarning(warning));
                }
                None => eprintln!("[cookie] {warning}"),
            }
        }

        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", HeaderValue::from_static(USER_AGENT));
        headers.insert("Origin", HeaderValue::from_static(BASE_URL));
        headers.insert("Referer", HeaderValue::from_static(BASE_URL));

        let http = ClientBuilder::new()
            .default_headers(headers)
            .cookie_provider(cookie_jar.clone())
            .build()
            .context("создание HTTP-клиента")?;

        let refresh_task = spawn_token_refresh(
            http.clone(),
            cookie_jar.clone(),
            base_url.clone(),
            events
        );

        Ok(Self { http, refresh_task })
    }

    pub fn http(&self) -> &Client {
        &self.http
    }
}

fn spawn_token_refresh(
    http: Client,
    cookie_jar: Arc<Jar>,
    base_url: Url,
    events: Option<UnboundedSender<ClientEvent>>
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let start = tokio::time::Instant::now() + TOKEN_REFRESH_INTERVAL;
        let mut interval = tokio::time::interval_at(start, TOKEN_REFRESH_INTERVAL);

        loop {
            interval.tick().await;

            let mut attempt = 1;
            let mut reported = None;

            while attempt <= REFRESH_ATTEMPTS {
                match http.post(REFRESH_TOKEN_URL).send().await {
                    Ok(response) if response.status().is_success() => {
                        match cookies::sync_to_file(&cookie_jar, &base_url) {
                            Ok(()) => {
                                notify(
                                    &events,
                                    ClientEvent::TokenRefreshed,
                                    &format!(
                                        "[refresh_token] cookies обновлены, {CONFIG_PATH} сохранён"
                                    )
                                );
                            }
                            Err(error) => {
                                notify(
                                    &events,
                                    ClientEvent::RefreshFailed(
                                        format!(
                                            "не удалось сохранить cookie в {CONFIG_PATH}: {error}"
                                        )
                                    ),
                                    &format!("[refresh_token] {error}")
                                );
                            }
                        }
                        reported = None;
                        break;
                    }
                    Ok(response) => {
                        reported = Some(format!("refresh_token вернул {}", response.status()));
                    }
                    Err(error) => {
                        reported = Some(format!("ошибка запроса refresh_token: {error}"));
                    }
                }

                if attempt < REFRESH_ATTEMPTS {
                    tokio::time::sleep(REFRESH_RETRY_DELAY * (attempt as u32)).await;
                }
                attempt += 1;
            }

            if let Some(message) = reported {
                notify(
                    &events,
                    ClientEvent::RefreshFailed(message.clone()),
                    &format!("[refresh_token] {message}")
                );
            }
        }
    })
}

fn notify(events: &Option<UnboundedSender<ClientEvent>>, event: ClientEvent, fallback: &str) {
    match events {
        Some(sender) => {
            let _ = sender.send(event);
        }
        None => println!("{fallback}"),
    }
}
