//! What rustc prints for a crate tixschema refuses: a support library not written, an `enable`
//! list it cannot read, an initializer written twice or handed an argument, a type that lists
//! `"mongodb"` with a map keyed by a number, its own or a held model's, a map keyed by a number
//! or a `bool` under `#[serde(flatten)]` and in an enum serde buffers, untagged or under a tag
//! alone, an enum that declares no variant, under a tag or not, a member that holds an `i128`, a
//! type that does not list `"jsonschema"` asked for its document or held by one that does, and a
//! service whose `openapi` lacks `http_rest`, is written with a value, names a path that is no
//! route or that an operation answers with `GET`, writes a key it does not take, whose operation
//! names a type that does not list `"jsonschema"`, or whose `<Service>Schema` the crate root does
//! not re-export, and, in a build with no feature, a type that lists `"jsonschema"` handing a
//! const parameter to a written type and a string check on a `#[serde(transparent)]` struct,
//! which is no brand without the `serde` feature, and a path placeholder that names no member a
//! declared message reads under a key of its own.
//! Each case in `foundation_refusals/` is a crate of its own, held to the `.stderr` file beside
//! it; the initializer written twice, in `foundation_refusals/written_twice/`, is held to the code
//! and the name of each error, which rustc reports differently from one release to the next.

// Every `.stderr` but the ones in `foundation_refusals/const_argument_without_a_feature/` and
// `foundation_refusals/brand_without_serde/`, whose crates turn no feature on, was written with
// every feature on, the build the gate runs.
#![cfg(all(
    feature = "serde",
    feature = "zod",
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
    use trybuild::TestCases;

    /// The features the cases are built with: the ones this file is compiled under.
    const FEATURES: &str = r#""serde", "zod", "typescript", "dart", "swift", "kotlin", "mongodb", "chrono", "indexmap""#;

    #[test]
    fn each_case_fails_with_the_errors_written_beside_it() {
        TestCases::new().compile_fail("tests/foundation_refusals/*.rs");
    }

    #[test]
    fn an_initializer_written_twice_is_refused_for_each_name_it_writes() {
        let errors = errors_of_written_twice();
        for name in ["__tixschema_decode_with", "__TixschemaDecodeWithFoundation"] {
            assert!(
                errors
                    .iter()
                    .any(|(code, text)| code == "E0428" && text.contains(&format!("`{name}`"))),
                "no E0428 names `{name}` among: {errors:?}"
            );
        }
        assert!(
            errors
                .iter()
                .any(|(code, text)| code == "E0252"
                    && text.contains("`__tixschema_decode_with_impl`")),
            "no E0252 names the macro among: {errors:?}"
        );
        // E0119, the second marker's `impl`, is reported by some releases and not by others.
        assert!(
            errors
                .iter()
                .all(|(code, _)| code == "E0428" || code == "E0252" || code == "E0119"),
            "an error a second initializer does not draw: {errors:?}"
        );
    }

    #[test]
    fn a_const_argument_is_refused_in_a_build_with_no_feature() {
        assert_rendered_with_no_feature(
            "const_argument_without_a_feature",
            &format!(
                "tixschema = {{ path = {:?}, default-features = false }}\nserde_json = \"1\"\n",
                env!("CARGO_MANIFEST_DIR")
            ),
        );
    }

    #[test]
    fn a_string_check_on_a_transparent_struct_is_refused_in_a_build_with_no_feature() {
        assert_rendered_with_no_feature(
            "brand_without_serde",
            &format!(
                "tixschema = {{ path = {:?}, default-features = false }}\n\
                 serde = {{ version = \"1\", features = [\"derive\"] }}\n\
                 serde_json = \"1\"\n",
                env!("CARGO_MANIFEST_DIR")
            ),
        );
    }

    /// Held to the code and the message of each error, the rest being worded by rustc differently
    /// from one release to the next.
    #[test]
    fn a_placeholder_that_names_no_member_of_a_declared_message_is_refused() {
        let errors = errors_of(
            "path_names_no_member",
            &format!(
                "tixschema = {{ path = {:?}, default-features = false, features = [{FEATURES}] }}\n\
                 bson2 = {{ package = \"bson\", version = \"2.15\", features = [\"chrono-0_4\"] }}\n\
                 chrono = {{ version = \">=0.4.40\", features = [\"serde\"] }}\n\
                 serde = {{ version = \"1\", features = [\"derive\"] }}\n\
                 serde_json = \"1\"\n",
                env!("CARGO_MANIFEST_DIR")
            ),
        );
        let refused = |operation: &str, placeholder: &str| {
            format!(
                "evaluation panicked: service_schema: operation `{operation}`'s path names \
                 `{{{placeholder}}}`, and its message has no field named `{placeholder}` that \
                 serde reads under a key of its own. A path placeholder names the field as Rust \
                 writes it, whatever key serde writes it under."
            )
        };
        let reported: Vec<(&str, &str)> = errors
            .iter()
            .map(|error| {
                (
                    error["code"]["code"].as_str().unwrap_or_default(),
                    error["message"].as_str().unwrap_or_default(),
                )
            })
            .collect();
        let expected = [
            refused("by_wire_name", "documentId"),
            refused("flattened_member", "title"),
            refused("skipped_member", "cached_at"),
        ];
        assert_eq!(
            reported.len(),
            expected.len(),
            "another count of errors: {reported:#?}"
        );
        for message in &expected {
            assert!(
                reported.contains(&("E0080", message.as_str())),
                "not reported: {message}\nin: {reported:#?}"
            );
        }
        assert!(
            include_str!("../README.md").contains(&format!("error[E0080]: {}", expected[0])),
            "the README no longer quotes: {}",
            expected[0]
        );
    }

    /// Asserts that the errors rustc reports for `case`, built as a crate whose manifest lists
    /// `dependencies`, render as the `main.stderr` beside it.
    fn assert_rendered_with_no_feature(case: &str, dependencies: &str) {
        let rendered: String = errors_of(case, dependencies)
            .iter()
            .filter_map(|error| error["rendered"].as_str())
            .collect();
        let expected = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/foundation_refusals/{case}/main.stderr")),
        )
        .unwrap();
        assert_eq!(rendered.trim_end(), expected.trim_end());
    }

    /// The code and the message of each error rustc reports for the case, the closing count apart.
    fn errors_of_written_twice() -> Vec<(String, String)> {
        errors_of(
            "written_twice",
            &format!(
                "tixschema = {{ path = {:?}, default-features = false, features = [{FEATURES}] }}\n\
                 bson2 = {{ package = \"bson\", version = \"2.15\", features = [\"chrono-0_4\"] }}\n\
                 chrono = {{ version = \">=0.4.40\", features = [\"serde\"] }}\n\
                 serde = {{ version = \"1\", features = [\"derive\"] }}\n\
                 serde_json = \"1\"\n",
                env!("CARGO_MANIFEST_DIR")
            ),
        )
        .iter()
        .map(|error| {
            (
                error["code"]["code"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                error["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
    }

    /// Each error rustc reports for `foundation_refusals/{case}/main.rs`, built as a crate whose
    /// manifest lists `dependencies`, as cargo's JSON message holds it, the closing count apart.
    fn errors_of(case: &str, dependencies: &str) -> Vec<Value> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
        fs::create_dir_all(at.join("src")).unwrap();
        fs::write(
            at.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{case}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\
                 publish = false\n\n[dependencies]\n{dependencies}"
            ),
        )
        .unwrap();
        fs::copy(
            manifest_dir.join(format!("tests/foundation_refusals/{case}/main.rs")),
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
            .map(|read| read["message"].clone())
            .filter(|error| {
                !error["message"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("aborting due to")
            })
            .collect()
    }
}
