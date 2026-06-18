//! Tests for the 0.5.0 polish derives: `#[derive(HealthCheck)]` and the
//! `#[cli]` shell-completion / man-page methods.

#![cfg(all(feature = "health", feature = "completions"))]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use server_less::{HealthCheck, cli};
use tower::ServiceExt;

#[derive(Clone, HealthCheck)]
struct DefaultProbe;

#[derive(Clone, HealthCheck)]
#[health(path = "/healthz", status = "alive")]
struct CustomProbe;

async fn body_string(router: axum::Router, uri: &str) -> (StatusCode, String) {
    let resp = router
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn health_router_default_path_and_body() {
    let (status, body) = body_string(DefaultProbe.health_router(), "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn health_router_custom_path_and_body() {
    let (status, body) = body_string(CustomProbe.health_router(), "/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "alive");
}

#[tokio::test]
async fn health_router_default_path_absent_when_overridden() {
    let (status, _) = body_string(CustomProbe.health_router(), "/health").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ─── shell completions + man page ────────────────────────────────────────────

#[derive(Clone)]
struct CompApp;

#[cli(name = "comp-app", description = "Completion demo")]
impl CompApp {
    /// Greet someone
    pub fn greet(&self, name: String) -> String {
        format!("Hello, {name}")
    }
}

#[test]
fn completions_bash_script_mentions_binary_and_subcommand() {
    let mut buf = Vec::new();
    CompApp::cli_completions(server_less::clap_complete::Shell::Bash, &mut buf);
    let script = String::from_utf8(buf).unwrap();
    assert!(script.contains("comp-app"), "script: {script}");
    assert!(script.contains("greet"), "script: {script}");
}

#[test]
fn completions_zsh_script_generated() {
    let mut buf = Vec::new();
    CompApp::cli_completions(server_less::clap_complete::Shell::Zsh, &mut buf);
    assert!(!buf.is_empty());
}

#[test]
fn manpage_renders_name_and_description() {
    let mut buf = Vec::new();
    CompApp::cli_manpage(&mut buf).unwrap();
    let man = String::from_utf8(buf).unwrap();
    assert!(man.contains("comp-app"), "man: {man}");
}
