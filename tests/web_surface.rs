//! Pin the API contract used by the future Web UI.
//!
//! The actual Web UI lives in a separate downstream repo (this crate
//! stays WASM-target-agnostic per plan §6.1). This test runs the
//! `web_surface_smoke` example so any change that breaks the surface
//! breaks `cargo test --workspace` here, before downstream notices.

#[path = "../examples/web_surface_smoke.rs"]
mod surface;

#[test]
fn web_surface_smoke_runs_green() {
    let status = surface::run_surface();
    assert!(
        status.starts_with("ok:"),
        "web surface smoke returned: {status}",
    );
}
