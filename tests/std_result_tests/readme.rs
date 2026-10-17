//! The README's section on the std `Result`, held to the model it declares and to what each
//! surface writes for it.

use super::models::Reply;
use super::validate::MISSIVE_REPORTS;

/// The README, as this test reads it.
const README: &str = include_str!("../../README.md");

/// The declaration the README shows, which `models.rs` compiles.
const DECLARED: &str = "#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reply {
    pub outcome: Result<u32, String>,
    pub stored: Result<(), String>,
}";

/// `written` without the lines of its doc comments.
fn without_docs(written: &str) -> String {
    written
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("/**") || trimmed.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_readme_declares_the_model_this_binary_compiles() {
    let section = README
        .split_once("### Results")
        .and_then(|(ranges, rest)| ranges.contains("### Ranges").then_some(rest))
        .and_then(|rest| rest.split_once("### Enums"))
        .map(|(section, _)| section)
        .unwrap();
    assert!(
        section.contains(DECLARED),
        "the README no longer declares it"
    );
    assert!(
        include_str!("models.rs").contains(DECLARED),
        "the model compiled is another"
    );
}

#[test]
fn the_readme_shows_what_typescript_and_zod_write() {
    let typescript = without_docs(&Reply::ts_definition());
    let declared = typescript
        .split_once("export type Reply = {")
        .map(|(_, rest)| format!("export type Reply = {{{rest}"))
        .unwrap();
    assert!(README.contains(declared.trim_end()), "{declared}");
    let zod = Reply::zod_schema();
    let raw = zod.split("\n\n").next().unwrap();
    assert!(raw.starts_with("const Reply$RawSchema"), "{zod}");
    assert!(README.contains(raw), "{raw}");
}

#[test]
fn the_readme_reports_what_validate_reports() {
    assert!(README.contains(&format!("`{}`", MISSIVE_REPORTS[1])));
}

#[cfg(feature = "dart")]
#[test]
fn the_readme_names_the_dart_class() {
    assert!(README.contains("`DartResult{N}`"));
    let dart = super::models::reply_dart::dart_definition();
    assert!(dart.contains("sealed class DartResult"), "{dart}");
    assert!(dart.contains("<int, String> outcome;"), "{dart}");
}

#[cfg(feature = "swift")]
#[test]
fn the_readme_names_the_swift_enum() {
    assert!(README.contains("`ReplyOutcomeResult`"));
    assert!(
        super::models::reply_swift::swift_definition()
            .contains("public enum ReplyOutcomeResult: Codable, Sendable {")
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn the_readme_names_the_kotlin_interface() {
    assert!(README.contains("`KotlinResult{N}`"));
    assert!(
        super::models::reply_kotlin::kotlin_definition().contains("sealed interface KotlinResult")
    );
}
