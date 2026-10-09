//! Unit tests of the Kotlin emitter: a unit struct renders as an `object`, a braced empty struct as
//! a class.

use super::*;

/// A unit struct is Kotlin's own no-data type: `@Serializable object`, never a `class`.
#[test]
fn a_unit_struct_renders_as_an_object() {
    let item: ItemStruct = syn::parse_quote! {
        struct Ping;
    };
    let rendered = struct_kotlin_tokens(&item, None).to_string();
    assert!(rendered.contains("object Ping"), "got: {rendered}");
    assert!(!rendered.contains("class Ping"), "got: {rendered}");
}

#[test]
fn a_braced_empty_struct_still_renders_as_a_class() {
    let item: ItemStruct = syn::parse_quote! {
        struct BracedEmpty {}
    };
    let rendered = struct_kotlin_tokens(&item, None).to_string();
    assert!(rendered.contains("class BracedEmpty"), "got: {rendered}");
    assert!(!rendered.contains("object BracedEmpty"), "got: {rendered}");
}
