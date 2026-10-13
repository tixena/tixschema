//! Unit tests of what a `preprocess` is refused for, and of the calls written for one.

use proc_macro2::{TokenStream, TokenTree};

use crate::model_schema::exec_model_schema;

/// The struct each refusal is raised on, with `prop` written on its one field.
fn parcel(prop: &str) -> String {
    format!("#[derive(Deserialize)] pub struct Parcel {{ {prop} pub label: String }}")
}

/// The whole expansion of `source` under the arguments `args`.
fn expansion(args: &str, source: &str) -> TokenStream {
    exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    )
}

/// Every refusal `tokens` carries: what it says, and the text of the source it is spanned on.
fn refusals(tokens: TokenStream) -> Vec<(String, Option<String>)> {
    let mut found = Vec::new();
    for tree in tokens {
        match tree {
            TokenTree::Group(group) => found.extend(refusals(group.stream())),
            TokenTree::Literal(literal) => {
                if let syn::Lit::Str(text) = syn::Lit::new(literal.clone())
                    && text.value().starts_with("model_schema: ")
                {
                    found.push((text.value(), literal.span().source_text()));
                }
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) => {}
        }
    }
    found
}

/// The one refusal of `preprocess = {written}` on a type, and of the same on a field.
fn refused(written: &str) -> [(String, Option<String>); 2] {
    let on_the_type = refusals(expansion(&format!("preprocess = {written}"), &parcel("")));
    let on_a_field = refusals(expansion(
        "",
        &parcel(&format!("#[model_schema_prop(preprocess = {written})]")),
    ));
    assert_eq!(on_the_type.len(), 1, "for a type: {on_the_type:?}");
    assert_eq!(on_a_field.len(), 1, "for a field: {on_a_field:?}");
    [on_the_type[0].clone(), on_a_field[0].clone()]
}

/// The one refusal of `source` under `args`.
fn the_refusal(args: &str, source: &str) -> (String, Option<String>) {
    let found = refusals(expansion(args, source));
    assert_eq!(found.len(), 1, "got: {found:?}");
    found[0].clone()
}

/// Every identifier of `tokens` named `name`.
fn idents_named(tokens: TokenStream, name: &str) -> Vec<proc_macro2::Ident> {
    let mut found = Vec::new();
    for tree in tokens {
        match tree {
            TokenTree::Group(group) => found.extend(idents_named(group.stream(), name)),
            TokenTree::Ident(ident) => {
                if ident == name {
                    found.push(ident);
                }
            }
            TokenTree::Literal(_) | TokenTree::Punct(_) => {}
        }
    }
    found
}

/// The type's functions are called on the whole payload, ahead of the read of its members, and a
/// member's inside the function that member is read through.
#[test]
fn the_read_calls_the_functions_of_the_type_and_then_those_of_each_member() {
    let tokens = expansion(
        "preprocess = [\"unwrapped\", \"withoutFragile\"]",
        &parcel("#[model_schema_prop(preprocess = [\"trimmed\", \"lowered\"])]"),
    );
    assert_eq!(refusals(tokens.clone()), Vec::new(), "got: {tokens}");
    let rendered = tokens.to_string();
    for written in [
        "pub fn model_schema_read_unjudged < 'de , D > (deserializer : D) -> :: core :: result :: \
         Result < Self , D :: Error > where D : serde :: Deserializer < 'de > , { let payload = < \
         :: serde_json :: Value as serde :: Deserialize > :: deserialize (deserializer) ? ; let \
         corrected = without_fragile (unwrapped (payload)) ; Self :: model_schema_read_corrected \
         (corrected) . map_err (< D :: Error as serde :: de :: Error > :: custom) }",
        "fn read_corrected_label < 'de , D , Read > (deserializer : D) -> :: core :: result :: \
         Result < Read , D :: Error > where D : serde :: Deserializer < 'de > , Read : serde :: \
         Deserialize < 'de > , { let payload = < :: serde_json :: Value as serde :: Deserialize > \
         :: deserialize (deserializer) ? ; let corrected = lowered (trimmed (payload)) ; < Read \
         as serde :: Deserialize > :: deserialize (corrected) . map_err (< D :: Error as serde :: \
         de :: Error > :: custom) }",
        "struct ModelSchemaRemote { # [serde (deserialize_with = \"read_corrected_label\")] label \
         : String } < ModelSchemaRemote > :: deserialize (deserializer) } }",
        "{ Self :: model_schema_read_unjudged (deserializer) } }",
    ] {
        assert!(rendered.contains(written), "no `{written}` in: {rendered}");
    }
    for unwritten in [
        "allow",
        "expect",
        "doc (hidden)",
        "impl Trait",
        "validate (",
    ] {
        assert!(
            !rendered.contains(unwritten),
            "found `{unwritten}` in: {rendered}"
        );
    }
}

/// A name is called as the snake case of what is written, at the place it is written: a missing
/// function is then reported at the string literal.
#[test]
fn a_call_is_spanned_on_the_name_as_written() {
    let tokens = expansion(
        "preprocess = [\"withoutFragile\"]",
        &parcel("#[model_schema_prop(preprocess = [\"trimmedLabel\"])]"),
    );
    for (function, written) in [
        ("without_fragile", "\"withoutFragile\""),
        ("trimmed_label", "\"trimmedLabel\""),
    ] {
        let called = idents_named(tokens.clone(), function);
        assert!(!called.is_empty(), "no call of `{function}` in: {tokens}");
        for ident in called {
            assert_eq!(ident.span().source_text().as_deref(), Some(written));
        }
    }
}

/// No local of a read is one of the functions it calls.
#[test]
fn the_names_a_read_declares_are_none_of_the_functions_it_calls() {
    let rendered = expansion(
        "preprocess = [\"payload\", \"deserializer\"]",
        &parcel("#[model_schema_prop(preprocess = [\"corrected\", \"payload\"])]"),
    )
    .to_string();
    for written in [
        "(deserializer2 : D) -> :: core :: result :: Result < Self , D :: Error > where D : serde \
         :: Deserializer < 'de > , { let payload2 = < :: serde_json :: Value as serde :: \
         Deserialize > :: deserialize (deserializer2) ? ; let corrected2 = deserializer (payload \
         (payload2)) ;",
        "let corrected2 = payload (corrected (payload2)) ;",
    ] {
        assert!(rendered.contains(written), "no `{written}` in: {rendered}");
    }
}

/// An item that is never read calls nothing, and is refused nothing a read would be.
#[test]
fn an_item_that_derives_no_deserialize_calls_no_function() {
    for source in [
        "#[derive(Serialize)] pub struct Parcel<'a> { #[model_schema_prop(preprocess = \
         [\"trimmed\"])] pub label: &'a str }",
        "pub struct Parcel { #[serde(deserialize_with = \"read_label\")] \
         #[model_schema_prop(preprocess = [\"trimmed\"])] pub label: String }",
    ] {
        let tokens = expansion("preprocess = [\"unwrapped\"]", source);
        assert_eq!(refusals(tokens.clone()), Vec::new(), "for {source}");
        for function in ["trimmed", "unwrapped"] {
            assert!(
                idents_named(tokens.clone(), function).is_empty(),
                "for {source}: {tokens}"
            );
        }
    }
}

#[test]
fn a_preprocess_that_is_no_array_is_refused() {
    for (said, located) in refused("\"trimmed\"") {
        assert!(
            said.contains("`preprocess` takes an array of string literals"),
            "got: {said}"
        );
        assert_eq!(located.as_deref(), Some("\"trimmed\""));
    }
    for (args, source) in [
        ("preprocess", parcel("")),
        ("", parcel("#[model_schema_prop(preprocess)]")),
    ] {
        let (said, located) = the_refusal(args, &source);
        assert!(
            said.contains("`preprocess` takes an array of string literals"),
            "got: {said}"
        );
        assert_eq!(located.as_deref(), Some("preprocess"));
    }
}

/// Each row of the README's table of refusals: what is written, and the words it is refused in.
#[test]
fn the_readme_states_each_refusal_in_the_words_it_is_raised_in() {
    let readme = include_str!("../../../README.md");
    for (written, located, sentence) in [
        (
            "\"trimmed\"",
            "\"trimmed\"",
            "takes an array of string literals, each naming a function: `preprocess = \
             [\"trimmed\"]`",
        ),
        (
            "[trimmed]",
            "trimmed",
            "takes string literals: each element is a string literal that names a function",
        ),
        (
            "[\"(v) => v\"]",
            "\"(v) => v\"",
            "names `(v) => v`, and a name is an identifier: ASCII letters, digits and `_`, not \
             led by a digit. The name is written as a function's name in every language",
        ),
        (
            "[]",
            "[]",
            "names no function: write one name at least, or drop the key",
        ),
        (
            "[\"trimmed\", \"trimmed\"]",
            "\"trimmed\"",
            "names `trimmed` twice: the name is written twice, and a function runs once",
        ),
    ] {
        for (said, spanned) in refused(written) {
            assert!(said.ends_with(sentence), "got: {said}");
            assert_eq!(spanned.as_deref(), Some(located), "for {written}");
        }
        assert!(
            readme.contains(&format!("| `preprocess = {written}` | {sentence} |")),
            "the README no longer shows the refusal of `preprocess = {written}`"
        );
    }
}

/// serde reads a member through one function, and the macro's own is the one that calls.
#[test]
fn a_member_read_through_a_function_of_its_authors_is_refused() {
    for (key, path) in [("deserialize_with", "read_label"), ("with", "stored_label")] {
        let (said, located) = the_refusal(
            "",
            &parcel(&format!(
                "#[serde({key} = \"{path}\")] #[model_schema_prop(preprocess = [\"trimmed\", \
                 \"lowered\"])]"
            )),
        );
        assert_eq!(
            said,
            format!(
                "model_schema: type `Parcel`: field `label`: `preprocess` cannot be written \
                 beside `#[serde({key} = \"{path}\")]`: serde reads a member through one \
                 function, and `#[model_schema()]` reads this one through a function of its own, \
                 which calls `trimmed`, `lowered`. Call `trimmed`, `lowered` from `{path}`, or \
                 take `{key}` off the member."
            )
        );
        assert_eq!(located.as_deref(), Some("\"trimmed\""));
    }
    let (said, _located) = the_refusal(
        "",
        "#[derive(Deserialize)] pub enum Step { Rename(String, #[serde(deserialize_with = \
         \"read_label\")] #[model_schema_prop(preprocess = [\"trimmed\"])] String) }",
    );
    assert!(
        said.starts_with("model_schema: type `Step`: slot 1 of variant `Rename`: `preprocess`"),
        "got: {said}"
    );
}

/// What a function answers is a payload of the read's own, which nothing can borrow from.
#[test]
fn a_value_that_borrows_from_the_payload_is_refused() {
    let lent = "#[derive(Deserialize)] pub struct Parcel<'a> { pub label: &'a str }";
    let (on_a_member, at_the_member) = the_refusal(
        "",
        "#[derive(Deserialize)] pub struct Parcel<'a> { #[model_schema_prop(preprocess = \
         [\"trimmed\"])] pub label: &'a str }",
    );
    assert_eq!(
        on_a_member,
        "model_schema: type `Parcel`: field `label`: `preprocess` cannot be written on a member \
         that borrows: the member is `&'a str`, and a function answers a payload of its own, \
         which nothing read from it can borrow from. Write the member as an owned type, or take \
         the `preprocess` off."
    );
    assert_eq!(at_the_member.as_deref(), Some("\"trimmed\""));

    let (on_the_type, at_the_type) = the_refusal("preprocess = [\"unwrapped\"]", lent);
    assert_eq!(
        on_the_type,
        "model_schema: type `Parcel`: field `label`: `preprocess` cannot be written on a type \
         that borrows: the member is `&'a str`, and a function answers a payload of its own, \
         which nothing read from it can borrow from. Write the member as an owned type, or take \
         the `preprocess` off."
    );
    assert_eq!(at_the_type.as_deref(), Some("\"unwrapped\""));

    let (under_the_attribute, _located) = the_refusal(
        "preprocess = [\"unwrapped\"]",
        "#[derive(Deserialize)] pub struct Parcel<'a> { #[serde(borrow)] pub label: Cow<'a, str> }",
    );
    assert!(
        under_the_attribute.contains("on a type that borrows: the member is `Cow<'a, str>`"),
        "got: {under_the_attribute}"
    );
    assert_eq!(refusals(expansion("", lent)), Vec::new());
}

/// An alias is another name for its target: it has no read of its own to call a function from.
#[test]
fn a_preprocess_on_a_type_alias_is_refused() {
    let (said, located) = the_refusal(
        "preprocess = [\"unwrapped\"]",
        "pub type Parcels = Vec<Parcel>;",
    );
    assert_eq!(
        said,
        "model_schema: type `Parcels`: `preprocess` cannot be written on a type alias: `Parcels` \
         is another name for `Vec<Parcel>`, so it has no read of its own to call `unwrapped` \
         from. Write the `preprocess` on `Vec<Parcel>`, or on a field that holds a `Parcels`, in \
         `#[model_schema_prop(...)]`."
    );
    assert_eq!(located.as_deref(), Some("\"unwrapped\""));
}

/// A brand publishes its inner's own schema, so every key written on its slot is refused: a
/// brand's `preprocess` is written on the type.
#[test]
fn a_preprocess_on_the_slot_of_a_brand_is_refused() {
    let (said, located) = the_refusal(
        "",
        "#[derive(Deserialize)] #[serde(transparent)] pub struct Code(#[model_schema_prop(\
         preprocess = [\"trimmed\"])] pub String);",
    );
    assert!(
        said.contains(
            "type `Code`: `#[model_schema_prop]` is unread on the slot of a `#[serde(transparent)]` \
             newtype"
        ),
        "got: {said}"
    );
    assert_eq!(
        located.as_deref(),
        Some("[model_schema_prop(preprocess = [\"trimmed\"])]")
    );
    let on_the_type = expansion(
        "preprocess = [\"trimmed\"]",
        "#[derive(Deserialize)] #[serde(transparent)] pub struct Code(pub String);",
    );
    assert_eq!(refusals(on_the_type.clone()), Vec::new());
    assert!(
        !idents_named(on_the_type.clone(), "trimmed").is_empty(),
        "no call of `trimmed` in: {on_the_type}"
    );
}

/// The macro writes the read of a type that names a `preprocess`, and cannot put it behind a
/// conversion of its author's.
#[test]
fn a_conversion_of_the_authors_own_is_refused_on_a_type_that_names_a_preprocess() {
    for key in ["remote", "from", "try_from", "into"] {
        for (args, prop) in [
            ("preprocess = [\"unwrapped\"]", ""),
            ("", "#[model_schema_prop(preprocess = [\"trimmed\"])]"),
        ] {
            let (said, _located) = the_refusal(
                args,
                &format!(
                    "#[derive(Deserialize)] #[serde({key} = \"Wire\")] pub struct Sealed {{ \
                     {prop} pub label: String }}"
                ),
            );
            assert_eq!(
                said,
                format!(
                    "model_schema: type `Sealed`: `#[serde({key} = ...)]` puts a conversion of \
                     its author's between the type and what serde reads or writes for it, and \
                     `Sealed` names a `preprocess`, so `#[model_schema()]` writes its \
                     `Deserialize`: each function called on the payload, then the read of its \
                     members. It cannot put that read behind a conversion it does not write. \
                     Take `{key}` off the type and convert where the value is read, or take the \
                     `preprocess` off the type and its members."
                ),
                "for {key}"
            );
        }
    }
}
