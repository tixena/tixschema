//! Every generated function that reports broken rules hands them over as a list. This reads the
//! crate's own sources and fails where generated code joins messages with `; ` or cuts a joined text
//! back on `; '`: the Dart, Swift and Kotlin text the emitters write, and the Rust inside a `quote!`.

use proc_macro2::{Delimiter, Literal, TokenStream, TokenTree};
use std::fs;
use std::path::{Path, PathBuf};

/// What the Dart, Swift and Kotlin text joins messages with, or cuts a joined text back with.
const JOINED: [&str; 8] = [
    "join('; ')",
    "join(\"; \")",
    "joined(separator: \"; \")",
    "joinToString(\"; \")",
    "joinToString(separator = \"; \")",
    "split(\"; '\")",
    "split('; \\'')",
    "components(separatedBy: \"; '\")",
];

/// The files whose string literals hold Dart, Swift or Kotlin text.
const CLIENT_TEXT: [&str; 6] = [
    "features/dart",
    "features/swift",
    "features/kotlin",
    "features/service_schema/dart_",
    "features/service_schema/kotlin_",
    "features/service_schema/swift_",
];

/// The functions, as the walk names the one a `join` is in, that join messages on purpose: serde's
/// `Deserialize` answers serde's error, which holds one message; and an example test's panic prints
/// one line.
const JOINING: [&str; 2] = ["refusal_from_violations", "written_seen_from"];

const QUOTING: [&str; 4] = [
    "parse_quote",
    "parse_quote_spanned",
    "quote",
    "quote_spanned",
];

/// One source file's walk, and every join it has met.
struct Walk<'file> {
    client_text: bool,
    file: &'file Path,
    found: Vec<String>,
}

impl Walk<'_> {
    /// `tokens` as Rust a `quote!` writes, where `join` is called with `"; "`.
    fn emitted_join(&mut self, tokens: &[TokenTree], function: &str) {
        for pair in tokens.windows(2) {
            let [TokenTree::Ident(called), TokenTree::Group(arguments)] = pair else {
                continue;
            };
            let written: Vec<TokenTree> = arguments.stream().into_iter().collect();
            if called == "join"
                && arguments.delimiter() == Delimiter::Parenthesis
                && matches!(written.as_slice(), [TokenTree::Literal(separator)] if separator.to_string() == "\"; \"")
                && !JOINING.contains(&function)
            {
                self.found.push(format!(
                    "{}:{}: join(\"; \") in `{function}`",
                    self.file.display(),
                    called.span().start().line
                ));
            }
        }
    }

    /// A string literal of client text that joins messages or cuts them back.
    fn text(&mut self, literal: &Literal) {
        if !self.client_text {
            return;
        }
        let Ok(written) = syn::parse_str::<syn::LitStr>(&literal.to_string()) else {
            return;
        };
        let value = written.value();
        for joined in JOINED.iter().filter(|joined| value.contains(*joined)) {
            self.found.push(format!(
                "{}:{}: {joined}",
                self.file.display(),
                literal.span().start().line
            ));
        }
    }

    fn walk(&mut self, stream: TokenStream, emitted: bool, function: &str) {
        let tokens: Vec<TokenTree> = stream.into_iter().collect();
        if emitted {
            self.emitted_join(&tokens, function);
        }
        let mut named = function.to_owned();
        for (at, token) in tokens.iter().enumerate() {
            match token {
                TokenTree::Group(group) => {
                    let quoting = at >= 2
                        && matches!(&tokens[at - 1], TokenTree::Punct(bang) if bang.as_char() == '!')
                        && matches!(&tokens[at - 2], TokenTree::Ident(macro_name) if QUOTING.iter().any(|quoting| macro_name == quoting));
                    self.walk(group.stream(), emitted || quoting, &named);
                }
                TokenTree::Ident(declared) => {
                    if at > 0
                        && matches!(&tokens[at - 1], TokenTree::Ident(keyword) if keyword == "fn")
                    {
                        named = declared.to_string();
                    }
                }
                TokenTree::Literal(literal) => self.text(literal),
                TokenTree::Punct(_) => {}
            }
        }
    }
}

/// Every source file under `at` that is not a test's.
fn sources(at: &Path, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(at).unwrap() {
        let path = entry.unwrap().path();
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        if stem.ends_with("tests") {
            continue;
        }
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        } else {
            // Not Rust: the examples the documentation includes.
        }
    }
}

/// Whether the source file at `relative` writes Dart, Swift or Kotlin text.
fn writes_client_text(relative: &Path) -> bool {
    let written = relative.to_string_lossy();
    CLIENT_TEXT
        .iter()
        .any(|opening| written.starts_with(opening))
}

#[test]
fn no_generated_code_joins_the_messages_it_reports() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    assert!(
        files.len() > 20,
        "found too few sources under {}",
        root.display()
    );
    let mut found = Vec::new();
    for file in &files {
        let stream: TokenStream = fs::read_to_string(file).unwrap().parse().unwrap();
        let relative = file.strip_prefix(&root).unwrap();
        let mut walk = Walk {
            client_text: writes_client_text(relative),
            file: relative,
            found: Vec::new(),
        };
        walk.walk(stream, false, "");
        found.append(&mut walk.found);
    }
    assert!(
        found.is_empty(),
        "generated code joins the messages it reports, which it hands over as a list:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_walk_finds_a_join_and_passes_over_what_is_not_one() {
    let source = r##"
        fn emitter() -> TokenStream {
            let rules = ["a", "b"].join("; ");
            let dart = "throw FormatException(found.join('; '));";
            let kotlin = "SerializationException(violations.joinToString(\"; \"))";
            let swift = r#"said.components(separatedBy: "; '")"#;
            let listed = "throw ParcelRefusal(found);";
            quote! {
                let said = violations.join("; ");
                let steps = path.join(".");
            }
        }
        fn refusal_from_violations() -> TokenStream {
            quote! { violations.join("; ") }
        }
    "##;
    let mut walk = Walk {
        client_text: true,
        file: Path::new("sample.rs"),
        found: Vec::new(),
    };
    walk.walk(source.parse().unwrap(), false, "");
    assert_eq!(
        walk.found,
        [
            "sample.rs:4: join('; ')",
            "sample.rs:5: joinToString(\"; \")",
            "sample.rs:6: components(separatedBy: \"; '\")",
            "sample.rs:9: join(\"; \") in `emitter`",
        ]
    );
}
