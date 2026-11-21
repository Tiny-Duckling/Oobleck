// Library crate: all modules are exposed here so integration tests in
// `tests/` (and future consumers) can import them. `src/main.rs` is a thin
// binary that wires this library into an HTTP server.
pub mod auth;
pub mod config;
pub mod error;
pub mod routes;
