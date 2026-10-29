use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use lemonfiber_contract::adapter::{about_path, ready_path, About};
use serde_json::json;
use tower::ServiceExt as _;

use super::{about, kit, DIGEST, RELEASE, UPSTREAM};
use crate::homepage::Homepage;

fn a_key() -> String {
    ('e'..='t').collect()
}

/// A configuration directory of its own, removed when dropped.
struct Directory(PathBuf);

impl Directory {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("homepage-adapter-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn answered(
    homepage: Arc<Homepage>,
    method: &str,
    path: &str,
    key: Option<&str>,
    body: &str,
) -> (StatusCode, Vec<u8>) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(key) = key {
        request = request.header(header::AUTHORIZATION, format!("Bearer {key}"));
    }
    let Ok(request) = request.body(Body::from(body.to_owned())) else {
        return (StatusCode::IM_A_TEAPOT, Vec::new());
    };
    let Ok(response) = kit(a_key(), homepage).router().oneshot(request).await;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .map(|bytes| bytes.to_vec())
        .unwrap_or_default();
    (status, body)
}

#[test]
fn it_says_it_speaks_dashboard_show_for_the_homepage_its_recordings_were_taken_from() {
    let about = about();

    assert_eq!(about.speaks, vec!["dashboard.show@1".to_owned()]);
    assert_eq!(about.upstream, UPSTREAM);
    assert_eq!(
        about.releases.first().map(|one| one.version.as_str()),
        Some(RELEASE)
    );
    assert_eq!(
        about.releases.first().map(|one| one.digest.as_str()),
        Some(DIGEST)
    );
}

#[tokio::test]
async fn it_names_itself_under_the_key_and_to_nobody_else() {
    let directory = Directory::new("about");
    let homepage = Arc::new(Homepage::at(&directory.0));

    let (status, body) = answered(
        Arc::clone(&homepage),
        "GET",
        &about_path(),
        Some(&a_key()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(serde_json::from_slice::<About>(&body).ok(), Some(about()));

    let (status, _) = answered(homepage, "GET", &about_path(), Some("another"), "").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn it_is_ready_while_homepages_configuration_directory_is_there() {
    let directory = Directory::new("ready");
    let homepage = Arc::new(Homepage::at(&directory.0));
    let (_, before) = answered(
        Arc::clone(&homepage),
        "GET",
        &ready_path(),
        Some(&a_key()),
        "",
    )
    .await;

    assert!(std::fs::create_dir_all(&directory.0).is_ok());
    let (_, after) = answered(homepage, "GET", &ready_path(), Some(&a_key()), "").await;

    assert_eq!(before, b"false");
    assert_eq!(after, b"true");
}

#[tokio::test]
async fn panels_applied_over_the_contract_are_written_and_read_back() {
    let directory = Directory::new("apply");
    assert!(std::fs::create_dir_all(&directory.0).is_ok());
    let homepage = Arc::new(Homepage::at(&directory.0));
    let panels = json!({"groups": [{"name": "Watch", "entries": [{
        "name": "Jellyfin", "icon": "jellyfin.png", "link": "http://192.0.2.10:8096",
        "description": "Watch", "check": null, "widget": null
    }]}], "widgets": ["resources"]});

    let (status, _) = answered(
        Arc::clone(&homepage),
        "POST",
        "/lemonfiber/dashboard.show/v1/apply",
        Some(&a_key()),
        &json!({ "panels": panels }).to_string(),
    )
    .await;
    assert!(status.is_success(), "{status}");

    let (status, body) = answered(
        homepage,
        "POST",
        "/lemonfiber/dashboard.show/v1/panels",
        Some(&a_key()),
        "{}",
    )
    .await;
    assert!(status.is_success(), "{status}");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).ok(),
        Some(panels)
    );
}
