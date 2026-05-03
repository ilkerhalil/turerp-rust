//! OpenAPI JSON generator binary
//!
//! Usage: cargo run --bin gen_openapi
//!
//! Writes `openapi.json` to the crate root (`CARGO_MANIFEST_DIR`).

use std::path::PathBuf;
use utoipa::OpenApi;

use turerp::api::ApiDoc;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let out_path = PathBuf::from(manifest_dir).join("openapi.json");

    let api = ApiDoc::openapi();
    let json =
        serde_json::to_string_pretty(&api).expect("Failed to serialize OpenAPI specification");

    std::fs::write(&out_path, json).expect("Failed to write openapi.json");
    println!("Generated: {}", out_path.display());
}
