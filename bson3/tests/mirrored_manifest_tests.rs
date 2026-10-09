//! This package's manifest is held to tixschema's own: the lint tables entry for entry, every
//! feature under the same name, and every advisory floor at its version or above.
//!
//! The shared BSON sources read `cfg(feature = "...")` and are linted in whichever package compiles
//! them, so a manifest that drifts changes what they are without a line of them changing. The two
//! packages resolve into two lock files, so a floor pinned in one manifest holds one of them.

#[cfg(test)]
mod tests {
    use core::mem;

    const FLOORS: &str = "# Transitive dep floors";

    const OWN: &str = include_str!("../Cargo.toml");

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

    /// An entry written `name = ">=1.2.3"`, as its name and the numbers of that version.
    fn floor(entry: &str) -> Option<(String, Vec<u64>)> {
        let (name, held) = entry.split_once('=')?;
        let version = held.trim().trim_matches('"').strip_prefix(">=")?;
        let numbers = version
            .split('.')
            .map(str::parse)
            .collect::<Result<Vec<u64>, _>>()
            .ok()?;
        Some((name.trim().to_owned(), numbers))
    }

    /// The entries `manifest` lists under the comment [`FLOORS`] opens, up to the first empty line.
    fn floors(manifest: &str) -> Vec<&str> {
        manifest
            .lines()
            .skip_while(|line| !line.trim_start().starts_with(FLOORS))
            .take_while(|line| !line.trim().is_empty())
            .map(uncommented)
            .filter(|written| !written.trim().is_empty())
            .collect()
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

    /// The floors of `theirs` that `ours` does not pin under `[dev-dependencies]` at that version
    /// or above, one to a line. A floor written in any other form than [`floor`] reads is one.
    fn unheld_floors(ours: &str, theirs: &str) -> String {
        let pinned: Vec<(String, Vec<u64>)> = entries(ours, "dev-dependencies")
            .iter()
            .filter_map(|entry| floor(entry))
            .collect();
        floors(theirs)
            .into_iter()
            .filter(|entry| {
                !floor(entry).is_some_and(|(name, least)| {
                    pinned
                        .iter()
                        .any(|(own, held)| *own == name && *held >= least)
                })
            })
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
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
    fn every_advisory_floor_of_tixschema_is_pinned_here_at_its_version_or_above() {
        assert_ne!(
            floors(ROOT),
            Vec::<&str>::new(),
            "no advisory floor was read in tixschema's `Cargo.toml`"
        );
        assert_eq!(
            unheld_floors(OWN, ROOT),
            "",
            "tixschema's `Cargo.toml` pins an advisory floor `bson3/Cargo.toml` lacks or pins lower"
        );
    }

    #[test]
    fn a_floor_is_unheld_where_it_is_missing_pinned_lower_or_written_another_way() {
        let theirs = r#"
            [dev-dependencies]
            serde = "1"

            # Transitive dep floors, pinned to fix known vulnerabilities:
            # - bytes >=1.11.1: an advisory
            bytes = ">=1.11.1"
            time = ">=0.3.47"
            url = { version = ">=2.5.4" }
            idna = ">=1.0.3" # added to this manifest alone

            [features]
        "#;
        assert_eq!(
            floors(theirs).len(),
            4,
            "the block ends at the first empty line"
        );
        let ours = |bytes: &str| {
            format!(
                "[dev-dependencies]\nbytes = \"{bytes}\"\ntime = \">=0.3.47\"\nurl = \">=2.5.4\"\n"
            )
        };
        let unheld = r#"url = { version = ">=2.5.4" }
idna = ">=1.0.3""#;
        assert_eq!(unheld_floors(&ours(">=1.11.1"), theirs), unheld);
        assert_eq!(unheld_floors(&ours(">=1.12"), theirs), unheld);
        assert_eq!(
            unheld_floors(&ours(">=1.11.0"), theirs),
            format!("bytes = \">=1.11.1\"\n{unheld}")
        );
        assert_eq!(
            unheld_floors(&ours("1"), theirs),
            format!("bytes = \">=1.11.1\"\n{unheld}")
        );
        assert_eq!(
            unheld_floors("[dev-dependencies]\n", theirs),
            format!("bytes = \">=1.11.1\"\ntime = \">=0.3.47\"\n{unheld}")
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
