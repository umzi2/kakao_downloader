use std::{ future::Future, time::Duration };

use crate::error::Result;

const ATTEMPTS: usize = 3;

const BASE_DELAY: Duration = Duration::from_millis(500);

pub const ATTEMPTS_LABEL: &str = "3 попытки";

pub async fn with_retries<T, F, Fut>(mut operation: F) -> Result<T>
    where F: FnMut() -> Fut, Fut: Future<Output = Result<T>>
{
    let mut attempt = 1;

    loop {
        match operation().await {
            Ok(value) => {
                return Ok(value);
            }
            Err(error) => {
                if attempt >= ATTEMPTS {
                    return Err(error);
                }

                tokio::time::sleep(BASE_DELAY * (attempt as u32)).await;
                attempt += 1;
            }
        }
    }
}
