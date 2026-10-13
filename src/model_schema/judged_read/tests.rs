//! Unit tests of the read the macro writes for a type that publishes `validate()`.

use quote::ToTokens as _;

use super::{of_enum, of_struct};
use crate::model_schema::exec_model_schema;

/// The whole expansion of `source` under `#[model_schema(...)]` written with `args`.
fn expansion_under(args: &str, source: &str) -> String {
    exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    )
    .to_string()
}

/// The item as it is left, and what its read gains, where it publishes `validate()`.
fn judged_struct(source: &str) -> (String, String) {
    let mut item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let read = of_struct(&mut item, true).to_string();
    (item.to_token_stream().to_string(), read)
}

#[test]
fn a_conversion_of_the_authors_own_is_refused_on_a_type_that_publishes_validate() {
    for key in ["remote", "from", "try_from", "into"] {
        let source = format!(
            "#[derive(Deserialize)] #[serde({key} = \"Wire\")] pub struct Sealed {{ pub label: \
             String }}"
        );
        let expanded = expansion_under("check = [\"withinSealed\"]", &source);
        assert!(expanded.contains("compile_error"), "for {key}: {expanded}");
        let refusal = format!(
            "model_schema: type `Sealed`: `#[serde({key} = ...)]` puts a conversion of its \
             author's between the type and what serde reads or writes for it, and `Sealed` \
             publishes `validate()`, so `#[model_schema()]` writes its `Deserialize`: the read \
             of its members, then `validate()`. It cannot put that judgement behind a conversion \
             it does not write. Take `{key}` off the type and convert where the value is read, \
             or take off the bounds and the checks that make the type publish `validate()`."
        );
        assert!(
            expanded.contains(&refusal),
            "for {key}, missing `{refusal}` in: {expanded}"
        );
        assert!(
            !expanded.contains("model_schema_read_unjudged"),
            "for {key}: {expanded}"
        );
        if key == "try_from" {
            let readme = include_str!("../../../README.md");
            assert!(
                readme.contains(&format!("error: {refusal}\n")),
                "the README no longer shows the refusal"
            );
        }

        let on_an_enum = format!(
            "#[derive(Deserialize)] #[serde({key} = \"Wire\")] pub enum Sealed {{ Open, Shut }}"
        );
        let mut item: syn::ItemEnum = syn::parse_str(&on_an_enum).unwrap();
        let refused = of_enum(&mut item, true).to_string();
        assert!(refused.contains("compile_error"), "for {key}: {refused}");
        assert!(
            refused.contains(&format!("`#[serde({key} = ...)]`")),
            "for {key}: {refused}"
        );
    }
}

#[test]
fn a_conversion_is_left_alone_where_the_macro_writes_no_read() {
    for source in [
        // No `validate()` is published.
        "#[derive(Deserialize)] #[serde(try_from = \"Wire\")] pub struct Sealed { pub label: String }",
        // Nothing is read.
        "#[derive(Serialize)] #[serde(into = \"Wire\")] pub struct Sealed { pub label: String }",
    ] {
        let expanded = expansion_under("", source);
        assert!(
            !expanded.contains("compile_error"),
            "for {source}: {expanded}"
        );
    }
    let written =
        "#[derive(Serialize)] #[serde(into = \"Wire\")] pub struct Sealed { pub label: String }";
    let mut item: syn::ItemStruct = syn::parse_str(written).unwrap();
    assert!(of_struct(&mut item, true).is_empty());
}

#[test]
fn an_item_the_macro_writes_no_read_for_is_left_as_it_was_written() {
    for (source, publishes_validate) in [
        (
            "#[derive(Debug, Deserialize)] #[serde(rename_all = \"camelCase\")] pub struct \
             Parcel { #[serde(default)] pub item_label: String }",
            false,
        ),
        (
            "#[derive(Debug, Serialize)] #[serde(rename_all = \"camelCase\")] pub struct Parcel \
             { #[serde(skip_serializing_if = \"String::is_empty\")] pub item_label: String }",
            true,
        ),
        ("pub struct Parcel { pub item_label: String }", true),
    ] {
        let mut item: syn::ItemStruct = syn::parse_str(source).unwrap();
        let written = item.to_token_stream().to_string();
        assert!(
            of_struct(&mut item, publishes_validate).is_empty(),
            "for {source}"
        );
        assert_eq!(item.to_token_stream().to_string(), written, "for {source}");
    }
    let source = "#[derive(Deserialize)] pub enum Dock { North, South }";
    let mut item: syn::ItemEnum = syn::parse_str(source).unwrap();
    let written = item.to_token_stream().to_string();
    assert!(of_enum(&mut item, false).is_empty());
    assert_eq!(item.to_token_stream().to_string(), written);
}

#[test]
fn the_read_is_serdes_derive_on_a_copy_of_the_item_followed_by_validate() {
    let (item, read) = judged_struct(
        "#[derive(Debug, Serialize, Deserialize)] #[serde(rename_all = \"camelCase\")] pub \
         struct Parcel { #[serde(default)] pub item_label: String }",
    );
    // `Serialize` stays the derive's, and reads the attributes as they were written.
    assert_eq!(
        item,
        "# [derive (Debug , Serialize)] # [serde (rename_all = \"camelCase\")] pub struct Parcel \
         { # [serde (default)] pub item_label : String }"
    );
    for written in [
        "impl Parcel { # [doc = \" The read alone, for a holder that judges the whole value \
         once.\"] pub fn model_schema_read_unjudged < 'de , D > (deserializer : D) -> :: core :: \
         result :: Result < Self , D :: Error > where D : serde :: Deserializer < 'de > , {",
        "# [derive (Deserialize)] # [serde (remote = \"Parcel\")] # [serde (bound (deserialize = \
         \"\"))] # [serde (rename_all = \"camelCase\")] struct ModelSchemaRemote { # [serde \
         (default)] item_label : String }",
        "< ModelSchemaRemote > :: deserialize (deserializer) } }",
        "impl < 'de > serde :: Deserialize < 'de > for Parcel { fn deserialize < D > \
         (deserializer : D) -> :: core :: result :: Result < Self , D :: Error > where D : \
         serde :: Deserializer < 'de > , { let read = Self :: model_schema_read_unjudged \
         (deserializer) ? ; read . validate () . map_err (",
        "serde :: de :: Error :: custom (violations . join (\"; \"))",
        "? ; :: core :: result :: Result :: Ok (read) } }",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
    for unwritten in ["allow", "expect", "doc (hidden)", "impl Trait"] {
        assert!(!read.contains(unwritten), "found `{unwritten}` in: {read}");
    }
}

#[test]
fn an_item_that_only_reads_keeps_no_attribute_only_the_read_understood() {
    let (item, read) = judged_struct(
        "#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] pub struct Parcel { \
         #[serde(alias = \"label\")] pub item_label: String }",
    );
    assert_eq!(item, "pub struct Parcel { pub item_label : String }");
    for written in [
        "# [derive (serde :: Deserialize)] # [serde (remote = \"Parcel\")]",
        "# [serde (deny_unknown_fields)] struct ModelSchemaRemote { # [serde (alias = \
         \"label\")] item_label : String }",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn the_read_of_a_generic_item_is_written_under_the_bounds_of_serdes_derive() {
    let (_, read) = judged_struct(
        "#[derive(Deserialize)] pub struct Lent<'a, T: Clone> where T: Send { pub held: T, \
         pub marked: PhantomData<T>, pub text: &'a str }",
    );
    for written in [
        "impl < 'a , T : Clone > Lent < 'a , T > where T : Send {",
        "pub fn model_schema_read_unjudged < 'de , D > (deserializer : D) -> :: core :: result \
         :: Result < Self , D :: Error > where D : serde :: Deserializer < 'de > , 'de : 'a , T \
         : serde :: Deserialize < 'de > , {",
        "# [serde (remote = \"Lent\")] # [serde (bound (deserialize = \"T : serde :: \
         Deserialize < 'de >\"))] struct ModelSchemaRemote < 'a , T : Clone > where T : Send {",
        "< ModelSchemaRemote < 'a , T > > :: deserialize (deserializer)",
        "impl < 'de , 'a , T : Clone > serde :: Deserialize < 'de > for Lent < 'a , T > where T \
         : Send , 'de : 'a , T : serde :: Deserialize < 'de > {",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn a_type_that_borrows_for_static_is_read_at_static() {
    let (_, read) =
        judged_struct("#[derive(Deserialize)] pub struct Fixed { pub name: &'static str }");
    for written in [
        "pub fn model_schema_read_unjudged < D > (deserializer : D) -> :: core :: result :: \
         Result < Self , D :: Error > where D : serde :: Deserializer < 'static > , {",
        "impl serde :: Deserialize < 'static > for Fixed {",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn the_copy_names_the_type_wherever_the_item_writes_self() {
    let (_, read) = judged_struct(
        "#[derive(Deserialize)] #[serde(default)] pub struct Crate { pub nested: \
         Vec<Self>, pub label: String }",
    );
    for written in [
        "where D : serde :: Deserializer < 'de > , Self : :: core :: default :: Default , {",
        "# [serde (bound (deserialize = \"Crate : :: core :: default :: Default\"))] # [serde \
         (default)] struct ModelSchemaRemote { nested : Vec < Crate > , label : String }",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn the_reader_is_never_a_name_the_item_writes() {
    let (_, read) =
        judged_struct("#[derive(Deserialize)] pub struct Docked<D> { pub dock: D, pub d2: D2 }");
    for written in [
        "pub fn model_schema_read_unjudged < 'de , D3 > (deserializer : D3) -> :: core :: \
         result :: Result < Self , D3 :: Error > where D3 : serde :: Deserializer < 'de > , D : \
         serde :: Deserialize < 'de > , {",
        "fn deserialize < D3 > (deserializer : D3)",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn an_enum_is_read_on_a_copy_that_keeps_what_serde_reads_of_each_variant() {
    let mut item: syn::ItemEnum = syn::parse_str(
        "#[derive(Deserialize)] #[serde(tag = \"kind\")] pub enum Step { #[serde(rename = \
         \"rename\")] Rename { #[serde(default)] to: String } = 1, Stop }",
    )
    .unwrap();
    let read = of_enum(&mut item, true).to_string();
    assert_eq!(
        item.to_token_stream().to_string(),
        "pub enum Step { Rename { to : String } = 1 , Stop }"
    );
    let copy = "# [derive (Deserialize)] # [serde (remote = \"Step\")] # [serde (bound \
                (deserialize = \"\"))] # [serde (tag = \"kind\")] enum ModelSchemaRemote { # \
                [serde (rename = \"rename\")] Rename { # [serde (default)] to : String } , Stop }";
    assert!(read.contains(copy), "missing `{copy}` in: {read}");
}
