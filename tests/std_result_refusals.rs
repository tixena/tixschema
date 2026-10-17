//! A std `Result` written by its bare name, where the name is another type: the check written
//! beside the holder fails the build with E0308. The case in `std_result_refusals/foreign_result/`
//! is a crate of its own, held to the code of each error, which rustc words differently from one
//! release to the next.

// The case is built with every feature on, the build the gate runs.
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
    use std::path::Path;
    use std::process::Command;

    use serde_json::Value;

    /// The features the case is built with: the ones this file is compiled under.
    const FEATURES: &str = r#""serde", "zod", "jsonschema", "typescript", "dart", "swift", "kotlin", "mongodb", "chrono", "indexmap""#;

    /// One error rustc reports for the case: its code, and the source text its first span holds.
    struct Reported {
        code: String,
        source: String,
    }

    #[test]
    fn a_result_named_after_another_type_fails_at_its_identity_check() {
        let errors = errors_of_foreign_result();
        assert!(!errors.is_empty(), "the case reported no error");
        assert!(
            errors.iter().all(|error| error.code == "E0308"),
            "an error other than the identity check's: {:?}",
            errors.iter().map(|error| &error.code).collect::<Vec<_>>()
        );
        assert!(
            errors
                .iter()
                .any(|error| error.source.contains("pub outcome: Result<u32, String>,")),
            "no error is spanned on the field: {:?}",
            errors.iter().map(|error| &error.source).collect::<Vec<_>>()
        );
    }

    /// Each error rustc reports for the case, read from cargo's JSON messages, the closing count
    /// apart.
    fn errors_of_foreign_result() -> Vec<Reported> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join("foreign_result");
        fs::create_dir_all(at.join("src")).unwrap();
        fs::write(
            at.join("Cargo.toml"),
            format!(
                "[package]\nname = \"foreign_result\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\
                 publish = false\n\n[dependencies]\n\
                 tixschema = {{ path = {:?}, default-features = false, features = [{FEATURES}] }}\n\
                 serde = {{ version = \"1\", features = [\"derive\"] }}\n\
                 serde_json = \"1\"\n",
                manifest_dir.display().to_string()
            ),
        )
        .unwrap();
        fs::copy(
            manifest_dir.join("tests/std_result_refusals/foreign_result/main.rs"),
            at.join("src/main.rs"),
        )
        .unwrap();
        fs::copy(manifest_dir.join("Cargo.lock"), at.join("Cargo.lock")).unwrap();
        let checked = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
            .args([
                "check",
                "--quiet",
                "--message-format=json",
                "--manifest-path",
            ])
            .arg(at.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", at.join("target"))
            .output()
            .unwrap();
        assert!(!checked.status.success(), "the case compiled");
        String::from_utf8_lossy(&checked.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|read| {
                read["reason"] == "compiler-message" && read["message"]["level"] == "error"
            })
            .filter(|read| {
                !read["message"]["message"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("aborting due to")
            })
            .map(|read| Reported {
                code: read["message"]["code"]["code"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                source: read["message"]["spans"][0]["text"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            })
            .collect()
    }
}
