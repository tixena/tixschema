//! Generated Rust lands in a module the consumer wrote, where a type of theirs may be named `Vec`,
//! `Ok` or `Send`. So every name the standard prelude would have supplied is written by its full
//! path, and this reads the crate's own sources to hold every emitter to it.

use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream, TokenTree};
use std::fs;
use std::path::{Path, PathBuf};

/// What the standard prelude puts in scope: the names generated code may not write bare.
const PRELUDE: [&str; 40] = [
    "AsMut",
    "AsRef",
    "Box",
    "Clone",
    "Copy",
    "Default",
    "DoubleEndedIterator",
    "Drop",
    "Eq",
    "Err",
    "ExactSizeIterator",
    "Extend",
    "Fn",
    "FnMut",
    "FnOnce",
    "From",
    "FromIterator",
    "Future",
    "Into",
    "IntoFuture",
    "IntoIterator",
    "Iterator",
    "None",
    "Ok",
    "Option",
    "Ord",
    "PartialEq",
    "PartialOrd",
    "Result",
    "Send",
    "Sized",
    "Some",
    "String",
    "Sync",
    "ToOwned",
    "ToString",
    "TryFrom",
    "TryInto",
    "Unpin",
    "Vec",
];

/// The crates a path may start at, which a module of the consumer's by that name takes over
/// unless the path is written from the crate root.
const ROOTS: [&str; 2] = ["core", "std"];

/// The macros whose body is tokens to emit.
const QUOTING: [&str; 4] = [
    "parse_quote",
    "parse_quote_spanned",
    "quote",
    "quote_spanned",
];

/// Where a walk stands: inside tokens to emit or not, and at the names an emitted enum declares.
#[derive(Clone, Copy)]
struct Standing {
    declaring_variants: bool,
    emitted: bool,
}

/// One source file's walk, and every bare name it has met.
struct Walk<'file> {
    file: &'file Path,
    found: Vec<String>,
}

impl Walk<'_> {
    /// How a group is read, from the two tokens written before it, or `None` to pass over it.
    fn entering(
        group: &Group,
        before: [Option<&TokenTree>; 2],
        standing: Standing,
    ) -> Option<Standing> {
        let [earlier, last] = before;
        let quoting = match (earlier, last) {
            (Some(TokenTree::Ident(named)), Some(TokenTree::Punct(bang)))
                if bang.as_char() == '!' =>
            {
                QUOTING.iter().find(|quoting| named == *quoting).copied()
            }
            _ => None,
        };
        if let Some(named) = quoting {
            // A `parse_quote!` holding one name alone is a type handed to the analysis, which
            // reads it by that name.
            let alone = matches!(
                group.stream().into_iter().collect::<Vec<_>>().as_slice(),
                [TokenTree::Ident(_)]
            );
            return (!(alone && named.starts_with("parse_quote"))).then_some(Standing {
                declaring_variants: false,
                emitted: true,
            });
        }
        if !standing.emitted {
            return Some(standing);
        }
        // A derive is named in the macro namespace, where no type of the consumer's reaches.
        if matches!(last, Some(TokenTree::Ident(named)) if named == "derive") {
            return None;
        }
        Some(Standing {
            declaring_variants: group.delimiter() == Delimiter::Brace
                && matches!(earlier, Some(TokenTree::Ident(named)) if named == "enum"),
            emitted: true,
        })
    }

    fn name(&mut self, named: &Ident, before: [Option<&TokenTree>; 2]) {
        let [earlier, last] = before;
        let follows = |written: char, token: Option<&TokenTree>| matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == written);
        let qualified = follows(':', earlier) && follows(':', last);
        // A name written after `type` is declared there, and reads nothing in scope: an `impl`
        // of a standard trait declares the trait's own associated type under its name.
        let declared = matches!(last, Some(TokenTree::Ident(keyword)) if keyword == "type");
        let watched = PRELUDE.iter().chain(&ROOTS).any(|bare| named == bare);
        if qualified || declared || follows('#', last) || !watched {
            return;
        }
        self.found.push(format!(
            "{}:{}: `{named}`",
            self.file.display(),
            named.span().start().line
        ));
    }

    /// A path a serde attribute carries as text, such as `"Option::is_none"`.
    fn text(&mut self, literal: &Literal) {
        let written = literal.to_string();
        let Some(path) = written.strip_prefix('"') else {
            return;
        };
        if PRELUDE.iter().chain(&ROOTS).any(|bare| {
            path.strip_prefix(bare)
                .is_some_and(|rest| rest.starts_with("::"))
        }) {
            self.found.push(format!(
                "{}:{}: {written}",
                self.file.display(),
                literal.span().start().line
            ));
        }
    }

    fn walk(&mut self, stream: TokenStream, standing: Standing) {
        let tokens: Vec<TokenTree> = stream.into_iter().collect();
        let mut before: [Option<&TokenTree>; 2] = [None, None];
        for token in &tokens {
            match token {
                TokenTree::Group(group) => {
                    if let Some(inside) = Self::entering(group, before, standing) {
                        self.walk(group.stream(), inside);
                    }
                }
                TokenTree::Ident(named) => {
                    if standing.emitted && !standing.declaring_variants {
                        self.name(named, before);
                    }
                }
                TokenTree::Literal(literal) => {
                    if standing.emitted {
                        self.text(literal);
                    }
                }
                TokenTree::Punct(_) => {}
            }
            before = [before[1], Some(token)];
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

#[test]
fn generated_code_names_what_the_prelude_supplies_by_its_full_path() {
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
        let mut walk = Walk {
            file: file.strip_prefix(&root).unwrap(),
            found: Vec::new(),
        };
        walk.walk(
            stream,
            Standing {
                declaring_variants: false,
                emitted: false,
            },
        );
        found.append(&mut walk.found);
    }
    assert!(
        found.is_empty(),
        "generated code writes a standard name bare, which a type of the consumer's by that name \
         takes over:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_walk_finds_a_bare_name_and_passes_over_what_is_not_one() {
    let source = r#"
        fn emitter(held: &syn::Type) -> TokenStream {
            let read: Vec<String> = Vec::new();
            let handed: syn::Type = syn::parse_quote!(String);
            quote! {
                #[derive(Clone, Default)]
                pub enum Expected { String, Number }
                #[serde(skip_serializing_if = "Option::is_none")]
                #[serde(default = "std::string::String::new")]
                fn made(from: #held) -> ::core::option::Option<std::vec::Vec<Box<u8>>> {
                    Some(Expected::String)
                }
                impl ::core::future::IntoFuture for Expected {
                    type IntoFuture = ::core::future::Ready<()>;
                    type Output = ();
                    fn into_future(self) -> <Self as IntoFuture>::IntoFuture {
                        ::core::future::ready(())
                    }
                }
            }
        }
    "#;
    let mut walk = Walk {
        file: Path::new("sample.rs"),
        found: Vec::new(),
    };
    walk.walk(
        source.parse().unwrap(),
        Standing {
            declaring_variants: false,
            emitted: false,
        },
    );
    assert_eq!(
        walk.found,
        [
            "sample.rs:8: \"Option::is_none\"",
            "sample.rs:9: \"std::string::String::new\"",
            "sample.rs:10: `std`",
            "sample.rs:10: `Box`",
            "sample.rs:11: `Some`",
            "sample.rs:16: `IntoFuture`",
        ]
    );
}
