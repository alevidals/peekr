//! Shared HTTP client and JSON decoding.
//!
//! One client for the whole app keeps a single connection pool and a single timeout, so no
//! caller can hang forever. Decoding lives here so every provider reports failed requests the
//! same way, including the body that GitHub and Bitbucket use to explain 401/403/429.

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult};

/// A provider that does not answer in this time is treated as down.
const TIMEOUT: Duration = Duration::from_secs(30);
pub const USER_AGENT: &str = "peekr";
/// Enough to identify the problem without dumping a whole HTML error page.
const BODY_SNIPPET: usize = 200;

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .expect("failed to build the HTTP client")
}

/// Decodes a successful response. On failure it keeps the status, the URL and the first
/// non-empty line of the body, where the services explain what went wrong.
pub async fn json<T: DeserializeOwned>(response: reqwest::Response) -> AppResult<T> {
    let status = response.status();
    if !status.is_success() {
        let url = response.url().to_string();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::HttpStatus {
            status: status.as_u16(),
            url,
            message: first_line(&body),
        });
    }
    Ok(response.json().await?)
}

fn first_line(body: &str) -> String {
    let line = body
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    let mut snippet: String = line.chars().take(BODY_SNIPPET).collect();
    if line.chars().count() > BODY_SNIPPET {
        snippet.push('…');
    }
    snippet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_first_meaningful_line_of_an_error_body() {
        assert_eq!(
            first_line("\n\n  Bad credentials  \nmore"),
            "Bad credentials"
        );
        assert_eq!(first_line(""), "");
        let long = "x".repeat(BODY_SNIPPET + 10);
        assert_eq!(first_line(&long).chars().count(), BODY_SNIPPET + 1);
    }
}
