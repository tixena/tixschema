//! The doc comments the expansion writes, made ready for rustdoc: each ` ```rust example ` block
//! is wrapped in hidden lines that run it inside a closure answering `Result`, as
//! `schema_examples()` runs it, so a `?` in the block compiles as a doctest too. The hidden lines
//! do not show in the rendered docs.

use proc_macro2::{Delimiter, Group, Literal, Spacing, TokenStream, TokenTree};

/// The hidden lines written after a block's opening fence.
const OPENING: [&str; 6] = [
    "# fn main() {",
    "# struct __TixschemaExampleFailure(::std::string::String);",
    "# impl<E: ::core::fmt::Debug> ::core::convert::From<E> for __TixschemaExampleFailure {",
    "#     fn from(error: E) -> Self { Self(::std::format!(\"{error:?}\")) }",
    "# }",
    "# let run = || -> ::core::result::Result<(), __TixschemaExampleFailure> { let _ = {",
];

/// The hidden lines written before a block's closing fence.
const CLOSING: [&str; 3] = [
    "# }; ::core::result::Result::Ok(()) };",
    "# if let ::core::result::Result::Err(__TixschemaExampleFailure(error)) = run() { ::core::panic!(\"{error}\"); }",
    "# }",
];

/// `tokens` with every run of `#[doc = ".."]` attributes, at every depth, rewritten so each closed
/// ` ```rust example ` block in it is wrapped in [`OPENING`] and [`CLOSING`].
pub fn ready_for_rustdoc(tokens: TokenStream) -> TokenStream {
    let trees: Vec<TokenTree> = tokens.into_iter().map(descended).collect();
    let mut written = Vec::with_capacity(trees.len());
    let mut at = 0;
    while at < trees.len() {
        let run_end = doc_run_end(&trees, at);
        if run_end == at {
            written.push(trees[at].clone());
            at += 1;
        } else {
            written.extend(wrapped_run(&trees[at..run_end]));
            at = run_end;
        }
    }
    written.into_iter().collect()
}

fn descended(tree: TokenTree) -> TokenTree {
    match tree {
        TokenTree::Group(group) => {
            let mut rebuilt = Group::new(group.delimiter(), ready_for_rustdoc(group.stream()));
            rebuilt.set_span(group.span());
            TokenTree::Group(rebuilt)
        }
        other @ (TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_)) => other,
    }
}

/// The index past the run of doc attributes that starts at `at`, or `at` where none starts there.
fn doc_run_end(trees: &[TokenTree], at: usize) -> usize {
    let mut end = at;
    while doc_text(trees, end).is_some() {
        end += 2;
    }
    end
}

/// The text of the outer `#[doc = ".."]` attribute at `at`, `#` and its bracket.
fn doc_text(trees: &[TokenTree], at: usize) -> Option<(String, proc_macro2::Span)> {
    let (TokenTree::Punct(pound), TokenTree::Group(bracket)) = (trees.get(at)?, trees.get(at + 1)?)
    else {
        return None;
    };
    if pound.as_char() != '#' || bracket.delimiter() != Delimiter::Bracket {
        return None;
    }
    let meta = syn::parse2::<syn::MetaNameValue>(bracket.stream()).ok()?;
    let syn::Expr::Lit(literal) = meta.value else {
        return None;
    };
    let syn::Lit::Str(text) = literal.lit else {
        return None;
    };
    meta.path
        .is_ident("doc")
        .then(|| (text.value(), text.span()))
}

/// The attributes of one run, each closed ` ```rust example ` block wrapped.
fn wrapped_run(run: &[TokenTree]) -> Vec<TokenTree> {
    let texts: Vec<(String, proc_macro2::Span)> = (0..run.len())
        .step_by(2)
        .filter_map(|at| doc_text(run, at))
        .collect();
    let mut lines: Vec<Vec<String>> = texts
        .iter()
        .map(|(text, _)| text.split('\n').map(str::to_owned).collect())
        .collect();
    let mut open: Option<(usize, usize, String)> = None;
    let mut blocks = Vec::new();
    for (attribute, held) in lines.iter().enumerate() {
        for (line, text) in held.iter().enumerate() {
            let trimmed = text.trim();
            let cleaned = trimmed.strip_prefix('*').unwrap_or(trimmed).trim();
            match (&open, cleaned) {
                (None, "```rust example") => {
                    let indent = text[..text.len() - text.trim_start().len()].to_owned();
                    open = Some((attribute, line, indent));
                }
                (Some(_), "```") => {
                    if let Some((opened_in, opened_at, indent)) = open.take() {
                        blocks.push(((opened_in, opened_at), (attribute, line), indent));
                    }
                }
                _ => {}
            }
        }
    }
    if blocks.is_empty() {
        return run.to_vec();
    }
    for ((opened_in, opened_at), (closed_in, closed_at), indent) in blocks.into_iter().rev() {
        let hidden = |written: &[&str]| -> Vec<String> {
            written
                .iter()
                .map(|line| format!("{indent}{line}"))
                .collect()
        };
        lines[closed_in].splice(closed_at..closed_at, hidden(&CLOSING));
        let after = opened_at + 1;
        lines[opened_in].splice(after..after, hidden(&OPENING));
    }
    let mut written = Vec::with_capacity(run.len());
    for (at, (held, (_, span))) in lines.iter().zip(&texts).enumerate() {
        let mut pound = proc_macro2::Punct::new('#', Spacing::Alone);
        pound.set_span(run[at * 2].span());
        let mut text = Literal::string(&held.join("\n"));
        text.set_span(*span);
        let doc = proc_macro2::Ident::new("doc", *span);
        let mut equals = proc_macro2::Punct::new('=', Spacing::Alone);
        equals.set_span(*span);
        let inner: TokenStream = [
            TokenTree::Ident(doc),
            TokenTree::Punct(equals),
            TokenTree::Literal(text),
        ]
        .into_iter()
        .collect();
        let mut bracket = Group::new(Delimiter::Bracket, inner);
        bracket.set_span(run[at * 2 + 1].span());
        written.push(TokenTree::Punct(pound));
        written.push(TokenTree::Group(bracket));
    }
    written
}

#[cfg(test)]
mod tests {
    use super::ready_for_rustdoc;
    use quote::quote;

    fn docs_of(tokens: proc_macro2::TokenStream) -> Vec<String> {
        let item: syn::ItemStruct = syn::parse2(tokens).unwrap();
        item.attrs
            .iter()
            .filter_map(|attribute| {
                let named = attribute.meta.require_name_value().ok()?;
                let syn::Expr::Lit(literal) = &named.value else {
                    return None;
                };
                let syn::Lit::Str(text) = &literal.lit else {
                    return None;
                };
                Some(text.value())
            })
            .collect()
    }

    /// A `rust example` block is wrapped in hidden lines; the rest of the doc is left as written.
    #[test]
    fn an_example_block_is_wrapped_in_hidden_lines_that_answer_result() {
        let written = ready_for_rustdoc(quote! {
            /// A label.
            ///
            /// ```rust example
            /// Label::new("Shipping")?
            /// ```
            pub struct Label;
        });
        let docs = docs_of(written);
        assert_eq!(docs[0], " A label.");
        assert_eq!(docs[1], "");
        let opening: Vec<&str> = docs[2].split('\n').collect();
        assert_eq!(opening[0], " ```rust example");
        assert_eq!(opening[1], " # fn main() {");
        assert_eq!(opening.len(), 7);
        assert_eq!(docs[3], " Label::new(\"Shipping\")?");
        let closing: Vec<&str> = docs[4].split('\n').collect();
        assert_eq!(closing[0], " # }; ::core::result::Result::Ok(()) };");
        assert_eq!(closing.last(), Some(&" ```"));
    }

    /// A block of another language, and a `rust example` fence never closed, are left as written.
    #[test]
    fn other_blocks_and_an_unclosed_fence_are_left_as_written() {
        let other = quote! {
            /// ```rust
            /// let x = 1;
            /// ```
            pub struct Other;
        };
        assert_eq!(docs_of(ready_for_rustdoc(other.clone())), docs_of(other));
        let unclosed = quote! {
            /// ```rust example
            /// Label::new("Shipping")?
            pub struct Unclosed;
        };
        assert_eq!(
            docs_of(ready_for_rustdoc(unclosed.clone())),
            docs_of(unclosed)
        );
    }
}
