//! Each case is a crate of its own, built and tested by the cargo that runs this file.

use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const MANIFEST: &str = "[package]\nname = \"example_rules\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\
                        publish = false\n\n[dependencies]\n\
                        tixschema = { path = \"TIXSCHEMA\", default-features = false, features = [\"serde\", \"SURFACE\"] }\n\
                        serde = { version = \"1\", features = [\"derive\"] }\n\
                        serde_json = \"1\"\n";

const MODELS: &str = r#"use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A label whose second example breaks its rule.
/// ```rust example
/// Label { text: "fine".to_string() }
/// ```
/// ```rust example
/// Label { text: String::new() }
/// ```
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    #[model_schema_prop(minLength = 1)]
    pub text: String,
}

/// A tag whose example meets its rule.
/// ```rust example
/// Tag { text: "fine".to_string() }
/// ```
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    #[model_schema_prop(minLength = 1)]
    pub text: String,
}
"#;

const WRITTEN: &str = r#"use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A tag whose example meets its rule.
/// ```rust example
/// use example_rules::Tag;
/// let _ = Tag { text: "fine".to_string() };
/// ```
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    #[model_schema_prop(minLength = 1)]
    pub text: String,
}

#[cfg(test)]
mod written {
    use super::*;

    #[test]
    fn the_document_of_the_type_and_of_its_module_hold_the_examples() {
        let examples = serde_json::json!([{ "text": "fine" }]);
        assert_eq!(Tag::json_schema()["examples"], examples);
        assert_eq!(tag_schema::Schema::json_schema()["examples"], examples);
    }
}
"#;

fn tested_models(surface: &str, models: &str, case: &str) -> Output {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("example_rules_{surface}_{case}"));
    fs::create_dir_all(at.join("src")).unwrap();
    fs::write(
        at.join("Cargo.toml"),
        MANIFEST
            .replace("TIXSCHEMA", &manifest_dir.display().to_string())
            .replace("SURFACE", surface),
    )
    .unwrap();
    fs::write(at.join("src/lib.rs"), models).unwrap();
    fs::copy(manifest_dir.join("Cargo.lock"), at.join("Cargo.lock")).unwrap();
    Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["test", "--quiet", "--manifest-path"])
        .arg(at.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", at.join("target"))
        .output()
        .unwrap()
}

fn assert_the_broken_example_fails_cargo_test(surface: &str) {
    let run = tested_models(surface, MODELS, "broken");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!run.status.success(), "the run passed: {printed}");
    assert!(
        printed.contains(
            "example 2 of `Label` breaks its rules: 'text': too short: minimum length is 1, got 0"
        ),
        "Got: {printed}"
    );
    assert!(
        printed.contains("label_schema_examples::every_example_meets_its_rules"),
        "Got: {printed}"
    );
    assert!(
        !printed.contains("tag_schema_examples"),
        "a type whose example meets its rule failed: {printed}"
    );
}

#[test]
fn an_example_that_breaks_a_rule_fails_cargo_test_naming_the_type_the_example_and_the_rule() {
    assert_the_broken_example_fails_cargo_test("zod");
}

#[test]
fn the_test_is_written_in_a_build_with_json_schema_and_no_zod() {
    assert_the_broken_example_fails_cargo_test("jsonschema");
}

#[test]
fn a_build_with_json_schema_and_no_zod_writes_the_examples_keyword_and_the_test() {
    let run = tested_models("jsonschema", WRITTEN, "written");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.status.success(), "the run failed: {printed}");
}
