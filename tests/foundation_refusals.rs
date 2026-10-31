//! What rustc prints for a crate tixschema refuses: a support library not written, an `enable`
//! list it cannot read, an initializer written twice or handed an argument, a type that lists
//! `"mongodb"` with a map keyed by a number, its own or a held model's, a map keyed by a number
//! or a `bool` under `#[serde(flatten)]` and in an enum serde buffers, untagged or under a tag
//! alone, an enum that declares no variant, under a tag or not, and a member that holds an
//! `i128`.
//! Each case in `foundation_refusals/` is a crate of its own, held to the `.stderr` file beside
//! it; the initializer written twice, in `foundation_refusals/written_twice/`, is held to the code
//! and the name of each error, which rustc reports differently from one release to the next.

// Every `.stderr` was written with every feature on, the build the gate runs.
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
    use trybuild::TestCases;

    /// The features the cases are built with: the ones this file is compiled under.
    const FEATURES: &str = r#""serde", "zod", "jsonschema", "typescript", "dart", "swift", "kotlin", "mongodb", "chrono", "indexmap""#;

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

    /// The code and the message of each error rustc reports for the case, read from cargo's JSON
    /// messages, the closing count apart.
    fn errors_of_written_twice() -> Vec<(String, String)> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join("written_twice");
        fs::create_dir_all(at.join("src")).unwrap();
        fs::write(
            at.join("Cargo.toml"),
            format!(
                "[package]\nname = \"written_twice\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\
                 publish = false\n\n[dependencies]\n\
                 tixschema = {{ path = {:?}, default-features = false, features = [{FEATURES}] }}\n\
                 bson2 = {{ package = \"bson\", version = \"2.15\", features = [\"chrono-0_4\"] }}\n\
                 chrono = {{ version = \">=0.4.40\", features = [\"serde\"] }}\n\
                 serde = {{ version = \"1\", features = [\"derive\"] }}\n\
                 serde_json = \"1\"\n",
                manifest_dir.display().to_string()
            ),
        )
        .unwrap();
        fs::copy(
            manifest_dir.join("tests/foundation_refusals/written_twice/main.rs"),
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
            .map(|read| {
                (
                    read["message"]["code"]["code"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                    read["message"]["message"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                )
            })
            .filter(|(_, text)| !text.starts_with("aborting due to"))
            .collect()
    }
}
