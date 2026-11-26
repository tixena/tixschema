//! `chrono` and `jiff` are never on together: a crate that names `chrono` with tixschema's default
//! features on, `jiff` among them, stops at a `compile_error!` that says how to keep `chrono`, and
//! at no other error.
//! The case is a crate of its own, built once, in the build that has `jiff` on.

#![cfg(feature = "jiff")]

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use serde_json::Value;

    /// What tixschema answers a build with both date features on.
    const REFUSED: &str = "tixschema's `chrono` and `jiff` features cannot be on together; `jiff` \
                           is a default feature, so a crate that names `chrono` sets \
                           `default-features = false` on the `tixschema` dependency";

    #[test]
    fn a_crate_that_names_chrono_with_the_default_features_on_stops_at_the_refusal_alone() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join("both_date_features");
        fs::create_dir_all(at.join("src")).unwrap();
        fs::write(
            at.join("Cargo.toml"),
            format!(
                "[package]\nname = \"both_date_features\"\nversion = \"0.0.0\"\nedition = \
                 \"2024\"\npublish = false\n\n[dependencies]\ntixschema = {{ path = {:?}, \
                 features = [\"chrono\"] }}\n",
                manifest_dir.display().to_string()
            ),
        )
        .unwrap();
        fs::write(at.join("src/lib.rs"), "").unwrap();
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
        let errors: Vec<String> = String::from_utf8_lossy(&checked.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|read| {
                read["reason"] == "compiler-message" && read["message"]["level"] == "error"
            })
            .filter_map(|read| read["message"]["message"].as_str().map(str::to_owned))
            .filter(|message| !message.starts_with("aborting due to"))
            .collect();
        assert_eq!(
            errors,
            [REFUSED],
            "the build with both date features answered otherwise"
        );
    }
}
