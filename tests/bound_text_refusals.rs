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
        let errors = errors_of("length_on_a_bool");
        let list_or_string = "this type holds no string or list for the bound to measure";
        let expected = [
            (
                "flag",
                "minLength",
                "bool",
                list_or_string,
                "    pub flag: bool,",
            ),
            (
                "held_flag",
                "minLength",
                "bool",
                list_or_string,
                "    pub held_flag: &'held bool,",
            ),
            (
                "version",
                "minLength",
                "Version",
                list_or_string,
                "    pub version: Version,",
            ),
            (
                "keywords",
                "pattern",
                "Keywords",
                "this type holds no string for the bound to measure",
                "    pub keywords: Keywords,",
            ),
        ];
        assert_eq!(errors.len(), expected.len(), "{errors:#?}");
        for (field, key, held, label, source) in expected {
            let message =
                format!("model_schema: field `{field}`: `{key}` cannot apply to `{held}`");
            assert!(
                errors.iter().any(|error| {
                    error.code == "E0277"
                        && error.message == message
                        && error.label == label
                        && error.source == source
                }),
                "no E0277 refuses `{field}`: {errors:#?}"
            );
        }
    }

    /// A length on a brand over a brand declared below it is refused in words that name the order
    /// that builds: the inner is moved above the brand that holds it.
    #[test]
    fn a_length_on_a_brand_over_a_later_brand_names_the_order_that_builds() {
        let errors = errors_of("length_on_a_later_brand");
        let refusals: Vec<&Reported> = errors
            .iter()
            .filter(|error| error.source == "pub struct Behind(pub Vec<String>);")
            .collect();
        assert_eq!(
            refusals.len(),
            1,
            "one refusal is spanned on `Behind`: {errors:#?}"
        );
        for refusal in refusals {
            assert!(
                refusal.message.starts_with(
                    "model_schema: branded newtype `SomeBehind` applies string constraints"
                ),
                "{refusal:#?}"
            );
            assert!(
                refusal.message.ends_with(
                    "only when it is declared above `SomeBehind`: move `Behind` above `SomeBehind`."
                ),
                "{refusal:#?}"
            );
        }
    }

    /// Each error rustc reports for the case, read from cargo's JSON messages, the closing count
    /// apart.
    fn errors_of(case: &str) -> Vec<Reported> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
        fs::create_dir_all(at.join("src")).unwrap();
        fs::write(
            at.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{case}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\
                 publish = false\n\n[dependencies]\n\
                 tixschema = {{ path = {:?}, default-features = false, features = [{FEATURES}] }}\n\
                 serde = {{ version = \"1\", features = [\"derive\"] }}\n\
                 serde_json = \"1\"\n",
                manifest_dir.display().to_string()
            ),
        )
        .unwrap();
        fs::copy(
            manifest_dir.join(format!("tests/bound_text_refusals/{case}/main.rs")),
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
