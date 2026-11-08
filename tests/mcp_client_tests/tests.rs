//! Builds the two crates of `mcp_client_tests/` and runs the one that calls.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// What the calling crate prints: the seven outcomes of an answer, then a message the validator
/// refuses.
const PRINTED: &str = "\
ok version=7
declared not-found
fault failed-validation
fault handler-panic
fault transport-failure
fault undeserializable-payload
fault transport-failure
refused failed-validation sent=0
";

const CONTRACT_MANIFEST: &str = "\
[package]
name = \"contract\"
version = \"0.0.0\"
edition = \"2024\"
publish = false

[dependencies]
serde = { version = \"1\", features = [\"derive\"] }
serde_json = \"1\"
tixschema = { path = \"@TIXSCHEMA@\", default-features = false, features = [\"serde\"] }
";

const APP_MANIFEST: &str = "\
[package]
name = \"app\"
version = \"0.0.0\"
edition = \"2024\"
publish = false

[dependencies]
contract = { path = \"../contract\" }
serde = { version = \"1\", features = [\"derive\"] }
serde_json = \"1\"
";

#[test]
fn a_client_placed_in_another_crate_reads_each_kind_of_answer() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join("mcp_client_apart");
    fs::create_dir_all(at.join("contract/src")).unwrap();
    fs::create_dir_all(at.join("app/src")).unwrap();
    fs::write(
        at.join("contract/Cargo.toml"),
        CONTRACT_MANIFEST.replace("@TIXSCHEMA@", &manifest_dir.display().to_string()),
    )
    .unwrap();
    fs::write(at.join("app/Cargo.toml"), APP_MANIFEST).unwrap();
    fs::copy(
        manifest_dir.join("tests/mcp_client_tests/contract.rs"),
        at.join("contract/src/lib.rs"),
    )
    .unwrap();
    fs::copy(
        manifest_dir.join("tests/mcp_client_tests/app.rs"),
        at.join("app/src/main.rs"),
    )
    .unwrap();
    fs::copy(manifest_dir.join("Cargo.lock"), at.join("app/Cargo.lock")).unwrap();
    let ran = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["run", "--quiet", "--offline", "--manifest-path"])
        .arg(at.join("app/Cargo.toml"))
        .env("CARGO_TARGET_DIR", at.join("target"))
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&ran.stderr),
        "",
        "the calling crate did not build or run cleanly"
    );
    assert_eq!(String::from_utf8_lossy(&ran.stdout), PRINTED);
}
