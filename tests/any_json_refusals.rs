//! A `Value` written by its bare name, where the name is `serde_json`'s, fails the build at the
//! check written beside the holder, with E0080 and the refusal of a JSON value; where the name is
//! the crate's own type, declared below, the crate builds. The cases in `any_json_refusals/` are
//! one crate, its library the second case and its binary the first, held to the code, the message
//! and the source text of each error, the rest being worded by rustc differently from one release
//! to the next.

// The cases are built with every feature on, the build the gate runs.
#![cfg(all(
    feature = "serde",
    feature = "zod",
    feature = "jsonschema",
    feature = "typescript",
    feature = "dart",
    feature = "swift",
    feature = "kotlin",
    feature = "mongodb",
    feature = "chrono",
    feature = "indexmap"
))]

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use std::sync::OnceLock;

    use serde_json::Value;

    /// The features the cases are built with: the ones this file is compiled under.
    const FEATURES: &str = r#""serde", "zod", "jsonschema", "typescript", "dart", "swift", "kotlin", "mongodb", "chrono", "indexmap""#;

    /// One error rustc reports: its code, its message, and the source text its first span holds,
    /// read where the macro that wrote it was called.
    #[derive(Debug)]
    struct Reported {
        code: String,
        message: String,
        source: String,
    }

    #[test]
    fn a_bare_value_that_is_serde_jsons_fails_at_the_check_beside_its_holder() {
        let checked = checked("--bin=imported_value");
        assert!(!checked.status.success(), "the case compiled");
        let errors = errors(&checked);
        let refusal = "evaluation panicked: model_schema: type `Reading`: field `data` reaches \
                       `serde_json::Value`, which holds any JSON value, so no target can describe \
                       it. Declare the shape that value takes as a type of its own, and write that \
                       type in its place.";
        let evaluated: Vec<&Reported> = errors
            .iter()
            .filter(|error| error.code == "E0080")
            .collect();
        assert_eq!(evaluated.len(), 1, "one check fails: {errors:#?}");
        assert_eq!(
            evaluated[0].message, refusal,
            "the check fails with the refusal"
        );
        assert_eq!(
            evaluated[0].source, "    pub data: Value,",
            "the check is spanned on the field's type"
        );
        assert!(
            include_str!("../README.md").contains(&format!("error[E0080]: {refusal}")),
            "the README no longer quotes: {refusal}"
        );
    }

    #[test]
    fn the_crates_own_value_declared_below_builds_with_warnings_denied() {
        let checked = checked("--lib");
        assert!(
            checked.status.success(),
            "the case failed: {:#?}",
            errors(&checked)
        );
    }

    /// The crate of both cases, written once into this test binary's own directory: the two tests
    /// run at once, and neither rewrites a file the other's build reads.
    fn written() -> &'static Path {
        static WRITTEN: OnceLock<PathBuf> = OnceLock::new();
        WRITTEN.get_or_init(write)
    }

    /// Writes the crate of both cases, and answers where.
    fn write() -> PathBuf {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join("any_json_refusals");
        fs::create_dir_all(at.join("src/bin")).unwrap();
        fs::write(
            at.join("Cargo.toml"),
            format!(
                "[package]\nname = \"any_json_refusals\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\
                 publish = false\n\n[dependencies]\n\
                 tixschema = {{ path = {:?}, default-features = false, features = [{FEATURES}] }}\n\
                 indexmap = {{ version = \"2\", features = [\"serde\"] }}\n\
                 serde = {{ version = \"1\", features = [\"derive\"] }}\n\
                 serde_json = \"1\"\n",
                manifest_dir.display().to_string()
            ),
        )
        .unwrap();
        fs::copy(
            manifest_dir.join("tests/any_json_refusals/own_value_below/lib.rs"),
            at.join("src/lib.rs"),
        )
        .unwrap();
        fs::copy(
            manifest_dir.join("tests/any_json_refusals/imported_value/main.rs"),
            at.join("src/bin/imported_value.rs"),
        )
        .unwrap();
        fs::copy(manifest_dir.join("Cargo.lock"), at.join("Cargo.lock")).unwrap();
        at
    }

    /// `cargo check` of the target `selected` names, with cargo's JSON messages.
    fn checked(selected: &str) -> Output {
        let at = written();
        Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
            .args([
                "check",
                "--quiet",
                "--message-format=json",
                selected,
                "--manifest-path",
            ])
            .arg(at.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", at.join("target"))
            .output()
            .unwrap()
    }

    /// The span of the outermost macro call `span` was written by, or `span` where no macro wrote
    /// it.
    fn called_at(span: &Value) -> &Value {
        let mut outermost = span;
        while let Some(call) = outermost["expansion"].get("span") {
            outermost = call;
        }
        outermost
    }

    /// Each error rustc reports in `checked`, read from cargo's JSON messages, the closing count
    /// apart.
    fn errors(checked: &Output) -> Vec<Reported> {
        String::from_utf8_lossy(&checked.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|read| {
                read["reason"] == "compiler-message" && read["message"]["level"] == "error"
            })
            .map(|read| Reported {
                code: read["message"]["code"]["code"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                message: read["message"]["message"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                source: called_at(&read["message"]["spans"][0])["text"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            })
            .filter(|error| !error.message.starts_with("aborting due to"))
            .collect()
    }
}
