//! This package's manifest is held to tixschema's own: the lint tables entry for entry, and every
//! feature under the same name.
//!
//! The shared BSON sources read `cfg(feature = "...")` and are linted in whichever package compiles
//! them, so a manifest that drifts changes what they are without a line of them changing.

#[cfg(test)]
mod tests {
    use core::mem;

    /// This package's manifest.
    const OWN: &str = include_str!("../Cargo.toml");

    /// The manifest of tixschema itself.
    const ROOT: &str = include_str!("../../Cargo.toml");

    /// What `manifest` writes under `[table]`: each entry less its comments and its spacing, one
    /// written over several lines joined into one.
    fn entries(manifest: &str, table: &str) -> Vec<String> {
        let header = format!("[{table}]");
        let mut inside = false;
        let mut entry = String::new();
        let mut found = Vec::new();
        for line in manifest.lines() {
            let written: String = uncommented(line).split_whitespace().collect();
            if entry.is_empty() && written.starts_with('[') {
                inside = written == header;
            } else {
                entry.push_str(&written);
                if entry.matches('[').count() == entry.matches(']').count() {
                    let whole = mem::take(&mut entry);
                    if inside && !whole.is_empty() {
                        found.push(whole);
                    }
                }
            }
        }
        found
    }

    /// What `theirs` holds and `ours` does not, one to a line.
    fn lacking(ours: &[String], theirs: &[String]) -> String {
        theirs
            .iter()
            .filter(|entry| !ours.contains(entry))
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The name each of `entries` is written under.
    fn names(entries: &[String]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| {
                entry
                    .split_once('=')
                    .map_or(entry.as_str(), |(name, _value)| name)
                    .to_owned()
            })
            .collect()
    }

    /// `line` up to the comment it ends with, a `#` inside a quoted value being part of the value.
    fn uncommented(line: &str) -> &str {
        let mut quoted = false;
        for (at, read) in line.char_indices() {
            match read {
                '"' => quoted = !quoted,
                '#' if !quoted => return &line[..at],
                _ => {}
            }
        }
        line
    }

    #[test]
    fn the_lint_tables_are_the_ones_tixschema_writes_entry_for_entry() {
        assert_ne!(
            entries(ROOT, "lints.clippy"),
            Vec::<String>::new(),
            "no `[lints.clippy]` was read in tixschema's `Cargo.toml`"
        );
        for table in ["lints.rust", "lints.clippy"] {
            let (ours, theirs) = (entries(OWN, table), entries(ROOT, table));
            assert_eq!(
                lacking(&ours, &theirs),
                "",
                "`[{table}]` in `bson3/Cargo.toml` lacks an entry of tixschema's own `[{table}]`"
            );
            assert_eq!(
                lacking(&theirs, &ours),
                "",
                "`[{table}]` in `bson3/Cargo.toml` writes an entry tixschema's own `[{table}]` \
                 does not"
            );
        }
    }

    #[test]
    fn every_feature_of_tixschema_is_declared_here_under_its_name() {
        let (ours, theirs) = (
            names(&entries(OWN, "features")),
            names(&entries(ROOT, "features")),
        );
        assert_ne!(
            theirs,
            Vec::<String>::new(),
            "no `[features]` was read in tixschema's `Cargo.toml`"
        );
        assert_eq!(
            lacking(&ours, &theirs),
            "",
            "tixschema's `Cargo.toml` declares a feature `bson3/Cargo.toml` does not declare \
             under the same name"
        );
    }

    #[test]
    fn a_table_is_read_less_its_comments_and_the_tables_beside_it() {
        let manifest = r#"
            [package]
            description = "written with a # in it"

            [features] # the sets and the plain features
            # `serde` is here because the clients need it
            web = ["serde", "zod"]
            chrono = [
            ] # one written over two lines

            [lints.clippy]
            all = { level = "deny", priority = -1 }
        "#;
        assert_eq!(
            entries(manifest, "features"),
            [r#"web=["serde","zod"]"#, "chrono=[]"]
        );
        assert_eq!(names(&entries(manifest, "features")), ["web", "chrono"]);
        assert_eq!(
            entries(manifest, "package"),
            [r#"description="writtenwitha#init""#]
        );
        assert_eq!(
            entries(manifest, "lints.clippy"),
            [r#"all={level="deny",priority=-1}"#]
        );
        assert_eq!(entries(manifest, "lints.rust"), Vec::<String>::new());
    }
}
