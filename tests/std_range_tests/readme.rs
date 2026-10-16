//! The README's section on the ranges, held to the model it declares and to what each surface
//! writes for it.

use super::models::Spanned;
use super::validate::SIGNPOSTED_REPORTS;

/// The README, as this test reads it.
const README: &str = include_str!("../../README.md");

/// The declaration the README shows, which `models.rs` compiles.
const DECLARED: &str = "#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spanned {
    pub closed: RangeInclusive<u32>,
    pub from: RangeFrom<u32>,
    pub span: Range<u32>,
    pub to: RangeTo<u32>,
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
    assert!(
        README.contains(DECLARED),
        "the README no longer declares it"
    );
    assert!(
        include_str!("models.rs").contains(DECLARED),
        "the model compiled is another"
    );
    assert!(README.contains("use core::ops::{Range, RangeFrom, RangeInclusive, RangeTo};"));
}

#[test]
fn the_readme_shows_what_typescript_and_zod_write() {
    let typescript = without_docs(&Spanned::ts_definition());
    let declared = typescript
        .split_once("export type Spanned = {")
        .map(|(_, rest)| format!("export type Spanned = {{{rest}"))
        .unwrap();
    assert!(README.contains(declared.trim_end()), "{declared}");
    let zod = Spanned::zod_schema();
    let raw = zod.split("\n\n").next().unwrap();
    assert!(raw.starts_with("const Spanned$RawSchema"), "{zod}");
    assert!(README.contains(raw), "{raw}");
}

#[test]
fn the_readme_reports_what_validate_reports() {
    for report in &SIGNPOSTED_REPORTS[1..] {
        assert!(README.contains(&format!("`{report}`")), "{report}");
    }
}

#[cfg(feature = "dart")]
#[test]
fn the_readme_names_the_dart_record() {
    assert!(README.contains("`({int start, int end})`"));
    assert!(
        super::models::spanned_dart::dart_definition()
            .contains("final ({int start, int end}) span;")
    );
}

#[cfg(feature = "swift")]
#[test]
fn the_readme_names_the_swift_struct() {
    assert!(README.contains("`SpannedSpanRange`"));
    assert!(
        super::models::spanned_swift::swift_definition()
            .contains("public struct SpannedSpanRange: Codable, Sendable {")
    );
}
