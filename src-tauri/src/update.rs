//! Manual update check against the public GitHub releases API.
//!
//! Nothing runs in the background: the user presses a button in Settings, one
//! request is made to a fixed host, and the result is display-only. No download
//! or install happens inside the app.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::http;

pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/alevidals/peekr/releases/latest";
pub const RELEASES_URL: &str = "https://github.com/alevidals/peekr/releases";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateStatus {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    /// Release page for the newest published version.
    pub release_url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    draft: bool,
}

/// One request to the latest published release, using the shared HTTP client.
///
/// The public API is tried first. A private repository answers 404 without
/// authentication, so the configured GitHub token (when there is one) is used as
/// a fallback. The token never leaves the Rust process.
pub async fn check(client: &reqwest::Client, token: Option<&str>) -> AppResult<UpdateStatus> {
    let release = match http::send_json(client, latest_request(client, None)).await {
        Ok(release) => release,
        Err(error) if token.is_some() && unauthenticated(&error) => {
            log::debug!("Update check fell back to the configured GitHub token: {error}");
            http::send_json(client, latest_request(client, token)).await?
        }
        Err(error) => return Err(explain(error)),
    };
    parse_release(release, env!("CARGO_PKG_VERSION"))
}

fn latest_request(client: &reqwest::Client, token: Option<&str>) -> reqwest::RequestBuilder {
    let request = client
        .get(LATEST_RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    match token {
        Some(token) => request.bearer_auth(token),
        None => request,
    }
}

fn unauthenticated(error: &AppError) -> bool {
    matches!(
        error,
        AppError::HttpStatus {
            status: 401 | 403 | 404,
            ..
        }
    )
}

fn explain(error: AppError) -> AppError {
    match error {
        AppError::HttpStatus { status: 404, .. } => AppError::Provider(
            "No published release was found. If the repository is private, add a GitHub account that can read it.".into(),
        ),
        other => other,
    }
}

fn parse_release(release: ReleaseResponse, current_version: &str) -> AppResult<UpdateStatus> {
    if release.draft {
        return Err(AppError::Provider(
            "The newest release is still a draft".into(),
        ));
    }

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    Ok(UpdateStatus {
        update_available: compare_versions(&latest_version, current_version) == Ordering::Greater,
        release_url: release.html_url.unwrap_or_else(|| RELEASES_URL.to_string()),
        current_version: current_version.to_string(),
        latest_version,
    })
}

/// Compares two version strings numerically. A pre-release is older than the
/// matching release; only the shapes peekr publishes are supported.
pub fn compare_versions(left: &str, right: &str) -> Ordering {
    fn split(version: &str) -> (&str, Option<&str>) {
        match version.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (version, None),
        }
    }

    let (left_core, left_pre) = split(left.trim_start_matches('v'));
    let (right_core, right_pre) = split(right.trim_start_matches('v'));

    let left_parts: Vec<u64> = left_core
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    let right_parts: Vec<u64> = right_core
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();

    let length = left_parts.len().max(right_parts.len());
    for index in 0..length {
        let left = left_parts.get(index).copied().unwrap_or(0);
        let right = right_parts.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            Ordering::Equal => {}
            other => return other,
        }
    }

    match (left_pre, right_pre) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left), Some(right)) => left.cmp(right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_release_versions() {
        assert_eq!(compare_versions("0.2.0", "0.1.0"), Ordering::Greater);
        assert_eq!(compare_versions("v0.2.0", "0.2.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.1.9", "0.1.10"), Ordering::Less);
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.2.0", "0.2"), Ordering::Equal);
    }

    #[test]
    fn pre_releases_are_older_than_the_release() {
        assert_eq!(compare_versions("0.9.0-beta.1", "0.9.0"), Ordering::Less);
        assert_eq!(compare_versions("0.9.0", "0.9.0-beta.1"), Ordering::Greater);
        assert_eq!(
            compare_versions("0.9.0-beta.2", "0.9.0-beta.1"),
            Ordering::Greater
        );
    }

    #[test]
    fn parses_a_newer_release() {
        let release: ReleaseResponse = serde_json::from_str(
            r#"{
                "tag_name": "v0.2.0",
                "html_url": "https://github.com/alevidals/peekr/releases/tag/v0.2.0",
                "draft": false
            }"#,
        )
        .unwrap();

        let status = parse_release(release, "0.1.0").unwrap();
        assert_eq!(status.latest_version, "0.2.0");
        assert_eq!(status.current_version, "0.1.0");
        assert!(status.update_available);
        assert!(status.release_url.ends_with("v0.2.0"));
    }

    #[test]
    fn reports_when_the_running_version_is_current() {
        let release: ReleaseResponse =
            serde_json::from_str(r#"{"tag_name": "v0.1.0", "draft": false}"#).unwrap();
        let status = parse_release(release, "0.1.0").unwrap();
        assert!(!status.update_available);
        assert_eq!(status.release_url, RELEASES_URL);
    }

    #[test]
    fn rejects_drafts_and_garbage() {
        let draft: ReleaseResponse =
            serde_json::from_str(r#"{"tag_name": "v0.3.0", "draft": true}"#).unwrap();
        assert!(parse_release(draft, "0.1.0").is_err());

        assert!(serde_json::from_str::<ReleaseResponse>("not json").is_err());
    }

    #[test]
    fn private_release_errors_are_actionable() {
        let unauthorized = AppError::HttpStatus {
            status: 404,
            url: LATEST_RELEASE_API.into(),
            message: "Not Found".into(),
        };
        assert!(unauthenticated(&unauthorized));
        assert!(matches!(
            explain(unauthorized),
            AppError::Provider(message) if message.contains("private")
        ));

        let server_error = AppError::HttpStatus {
            status: 503,
            url: LATEST_RELEASE_API.into(),
            message: String::new(),
        };
        assert!(!unauthenticated(&server_error));
    }
}
