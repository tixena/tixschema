//! `CLAUDE.md` is loaded whole into every Claude Code session started in this repository, so it
//! holds the working rules alone; a feature's behaviour is documented in `README.md` and `docs/`.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    const CLAUDE_MD: &str = include_str!("../CLAUDE.md");

    /// The smallest limit Claude Code applies to an instruction file, the one of a 200K window.
    const CEILING: usize = 40_000;

    /// Why `text` is refused as `CLAUDE.md`, or `None` when it fits under [`CEILING`].
    fn oversize(text: &str) -> Option<String> {
        let written = text.chars().count();
        (written > CEILING).then(|| {
            format!(
                "CLAUDE.md holds {written} characters, over {CEILING}: a feature's behaviour goes \
                 in README.md or a docs/ file, never in CLAUDE.md"
            )
        })
    }

    /// Each relative link of `text`, read from `dir`, that names no file or directory.
    fn broken_links(dir: &Path, text: &str) -> Vec<String> {
        text.split("](")
            .skip(1)
            .filter_map(|rest| rest.split_once(')').map(|(target, _)| target))
            .filter(|target| {
                !["http://", "https://", "mailto:", "#"]
                    .iter()
                    .any(|scheme| target.starts_with(scheme))
            })
            .map(|target| target.split_once('#').map_or(target, |(path, _)| path))
            .filter(|path| !dir.join(path).exists())
            .map(ToOwned::to_owned)
            .collect()
    }

    #[test]
    fn claude_md_fits_under_its_ceiling() {
        assert_eq!(oversize(CLAUDE_MD), None, "CLAUDE.md is over its ceiling");
    }

    #[test]
    fn a_text_one_character_over_the_ceiling_is_refused() {
        assert_eq!(
            oversize(&"x".repeat(CEILING)),
            None,
            "the ceiling itself fits"
        );
        assert!(
            oversize(&"x".repeat(CEILING + 1)).is_some(),
            "one character over the ceiling is refused"
        );
    }

    #[test]
    fn every_relative_link_names_a_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut broken: Vec<String> = broken_links(root, CLAUDE_MD)
            .into_iter()
            .map(|path| format!("CLAUDE.md: {path}"))
            .collect();
        let docs = root.join("docs");
        for entry in fs::read_dir(&docs).unwrap() {
            let file = entry.unwrap().path();
            let text = fs::read_to_string(&file).unwrap();
            broken.extend(
                broken_links(&docs, &text)
                    .into_iter()
                    .map(|path| format!("{}: {path}", file.display())),
            );
        }
        assert!(broken.is_empty(), "links to no file: {broken:#?}");
    }

    #[test]
    fn a_link_to_a_missing_file_is_named() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let text = "[a](docs/no-such-file.md) [b](Cargo.toml) [c](https://example.com) [d](#rules)";
        assert_eq!(
            broken_links(root, text),
            ["docs/no-such-file.md"],
            "only the link to a missing file is named"
        );
    }
}
