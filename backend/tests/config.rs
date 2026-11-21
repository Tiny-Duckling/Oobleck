//! Integration tests for CORS origin parsing/validation.
//!
//! Lives in the separate `tests/` folder (integration tests, not inline unit
//! tests), exercising the public API of the `quiz_backend` library crate.

use oobleck_backend::config::parse_origin;

fn origin(raw: &str) -> String {
    parse_origin(raw).unwrap().to_str().unwrap().to_string()
}

#[test]
fn accepts_exact_origin() {
    assert_eq!(origin("http://localhost:5173"), "http://localhost:5173");
}

#[test]
fn normalizes_trailing_slash_and_case() {
    assert_eq!(origin("http://localhost:5173/"), "http://localhost:5173");
    assert_eq!(origin("HTTPS://EXAMPLE.COM:443/"), "https://example.com");
}

#[test]
fn drops_default_ports() {
    assert_eq!(origin("http://example.com:80"), "http://example.com");
    assert_eq!(origin("https://example.com:443"), "https://example.com");
}

#[test]
fn keeps_non_default_ports() {
    assert_eq!(origin("http://localhost:3000"), "http://localhost:3000");
    assert_eq!(
        origin("https://example.com:8443"),
        "https://example.com:8443"
    );
}

#[test]
fn rejects_non_http_schemes() {
    assert!(parse_origin("ftp://example.com").is_err());
    assert!(parse_origin("ws://example.com").is_err());
}

#[test]
fn rejects_missing_scheme() {
    assert!(parse_origin("localhost:5173").is_err());
}

#[test]
fn rejects_paths_queries_and_fragments() {
    assert!(parse_origin("http://localhost:5173/api").is_err());
    assert!(parse_origin("http://localhost:5173/?x=1").is_err());
    assert!(parse_origin("http://localhost:5173/#top").is_err());
}

#[test]
fn rejects_credentials() {
    assert!(parse_origin("http://user:pass@example.com").is_err());
    assert!(parse_origin("http://user@example.com").is_err());
}
