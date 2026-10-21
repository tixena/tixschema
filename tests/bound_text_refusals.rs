//! A length on a field that holds no string, owned or read through a reference, fails the build
//! with E0277 in the words the macro writes. The case in `bound_text_refusals/length_on_a_bool/`
//! is a crate of its own, held to the code, the message and the label of each error, the rest
//! being worded by rustc differently from one release to the next.

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

    /// One error rustc reports for the case: its code, its message, the label of its primary span
    /// and the source text that span holds.
    #[derive(Debug)]
    struct Reported {
        code: String,
        label: String,
        message: String,
        source: String,
    }

    #[test]
    fn a_length_on_a_bool_is_refused_owned_and_read_through_a_reference() {
        let errors = errors_of_length_on_a_bool();
        let expected = [
            ("flag", "    pub flag: bool,"),
            ("held_flag", "    pub held_flag: &'held bool,"),
        ];
        assert_eq!(errors.len(), expected.len(), "{errors:#?}");
        for (field, source) in expected {
            let message =
                format!("model_schema: field `{field}`: `minLength` cannot apply to `bool`");
            assert!(
                errors.iter().any(|error| {
                    error.code == "E0277"
                        && error.message == message
                        && error.label == "this type holds no string for the bound to measure"
                        && error.source == source
                }),
                "no E0277 refuses `{field}`: {errors:#?}"
            );
        }
    }

    /// Each error rustc reports for the case, read from cargo's JSON messages, the closing count
    /// apart.
    fn errors_of_length_on_a_bool() -> Vec<Reported> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join("length_on_a_bool");
        fs::create_dir_all(at.join("src")).unwrap();
        fs::write(
            at.join("Cargo.toml"),
            format!(
                "[package]\nname = \"length_on_a_bool\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\
                 publish = false\n\n[dependencies]\n\
                 tixschema = {{ path = {:?}, default-features = false, features = [{FEATURES}] }}\n\
                 serde = {{ version = \"1\", features = [\"derive\"] }}\n\
                 serde_json = \"1\"\n",
                manifest_dir.display().to_string()
            ),
        )
        .unwrap();
        fs::copy(
            manifest_dir.join("tests/bound_text_refusals/length_on_a_bool/main.rs"),
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
            .map(|read| {
                let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
                let span = read["message"]["spans"]
                    .as_array()
                    .and_then(|spans| spans.iter().find(|span| span["is_primary"] == true))
                    .unwrap_or(&Value::Null);
                Reported {
                    code: text(&read["message"]["code"]["code"]),
                    label: text(&span["label"]),
                    message: text(&read["message"]["message"]),
                    source: text(&span["text"][0]["text"]),
                }
            })
            .collect()
    }
}
