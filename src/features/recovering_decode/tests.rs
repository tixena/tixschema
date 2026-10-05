use super::struct_recovering_decode;
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::{ADDED_TYPE_NAMES, module_items, reading_the_authors_scope};

/// The `impl` the flag adds to `source`, as text.
fn type_impl_of(source: &str) -> String {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    struct_recovering_decode(&item).type_impl.to_string()
}

/// The statements `decode_with_value_fields` runs for `source`, as text.
fn fields_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, walk) = emitted
        .split_once("pub fn decode_with_value_fields")
        .unwrap();
    walk.to_owned()
}

/// Every method the flag adds carries the flag's name, so none can meet one the type's author
/// wrote. The entry point is the one it is named for.
#[test]
fn every_added_method_but_the_entry_point_carries_the_flags_name() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Named { pub title: String, pub versions: Vec<Version> }")
            .unwrap();
    let added: syn::ItemImpl = syn::parse2(struct_recovering_decode(&item).type_impl).unwrap();
    let methods: Vec<String> = added
        .items
        .iter()
        .filter_map(|added_item| {
            if let syn::ImplItem::Fn(method) = added_item {
                Some(method.sig.ident.to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(methods.len(), added.items.len());
    assert_eq!(
        methods,
        [
            "from_value_with",
            "decode_with_value_report",
            "decode_with_value_issues",
            "decode_with_value_fields",
        ]
    );
}

/// The key a field is looked up under is the one serde reads: `rename` over `rename_all`, and a raw
/// identifier without its `r#`.
#[test]
fn a_field_is_looked_up_under_the_key_serde_reads() {
    let walk = fields_walk_of(
        "#[serde(rename_all = \"camelCase\")] pub struct Keyed { \
         pub created_at: String, #[serde(rename = \"kind\")] pub flavor: String, pub r#type: String }",
    );
    for key in ["createdAt", "kind", "type"] {
        assert!(
            walk.contains(&format!("match object . get (\"{key}\")")),
            "for {key}, got: {walk}"
        );
    }
    assert!(
        walk.ends_with("vec ! [\"createdAt\" , \"kind\" , \"type\"] } }"),
        "got: {walk}"
    );
}

/// A struct's own `tag` is a key serde writes into the object and reads past, so it is declared.
#[test]
fn a_structs_own_tag_is_a_declared_key() {
    let walk = fields_walk_of("#[serde(tag = \"kind\")] pub struct Tagged { pub name: String }");
    assert!(
        walk.ends_with("vec ! [\"name\" , \"kind\"] } }"),
        "got: {walk}"
    );
}

/// A type with no field to walk takes the `Vec` every walker is handed and binds nothing of it:
/// no statement is written to give the parameter a use.
#[test]
fn a_type_with_no_field_to_walk_leaves_the_issue_list_unbound() {
    for source in [
        "pub struct Blank {}",
        "pub struct Unwritten { #[serde(skip)] pub cached: u8 }",
        "pub struct Derived { #[serde(skip_deserializing)] pub total: u32 }",
    ] {
        let walk = fields_walk_of(source);
        assert!(
            walk.contains(", _ : & mut Vec < I > ,) -> Vec < & 'a str > { vec ! ["),
            "for {source}, got: {walk}"
        );
    }
    let walked = fields_walk_of("pub struct Named { pub title: String }");
    assert!(
        walked.contains(
            ", out : & mut Vec < I > ,) -> Vec < & 'a str > { match object . get (\"title\")"
        ),
        "got: {walked}"
    );
}

/// The read hook tixschema hangs to name a field in its type's refusal changes a message and no
/// verdict: the field is still walked by its type's own walker. The author's hook is read through.
#[test]
fn a_generated_named_hook_is_read_past_and_an_authors_hook_is_read_through() {
    let generated = fields_walk_of(
        "pub struct Holder { \
         #[serde(deserialize_with = \"holder_schema::deserialize_named_inner\")] pub inner: Inner }",
    );
    assert!(
        generated.contains("< Inner > :: decode_with_value_issues (held ,"),
        "got: {generated}"
    );
    let authored = fields_walk_of(
        "pub struct Holder { #[serde(deserialize_with = \"lenient\")] pub inner: Inner }",
    );
    assert!(
        authored.contains(
            "holder_schema :: value_leaf (held , lenient , | read : & Inner | serde_json :: to_value (read) . ok () ,"
        ),
        "got: {authored}"
    );
}

/// Only a bare use of an added name is written through `super`: one already under a path names
/// what its path says, and every other identifier is left alone.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn only_a_bare_added_name_is_read_from_the_authors_scope() {
    let scoped = reading_the_authors_scope(quote::quote! {
        fn hook(held: Vec<Issue>, at: &Path, kept: crate::Issue, other: Version) -> Segment {
            Issue::default()
        }
    })
    .to_string();
    assert_eq!(
        scoped,
        quote::quote! {
            fn hook(
                held: Vec<super::Issue>,
                at: &super::Path,
                kept: crate::Issue,
                other: Version
            ) -> super::Segment {
                super::Issue::default()
            }
        }
        .to_string()
    );
}

/// The names read past are the types the flag adds, all of them: one left out would be taken for
/// the author's type of that name by whatever the module already held.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn every_type_the_flag_adds_is_a_name_read_past() {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    let mut declared: Vec<String> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Enum(declared_enum) = item {
                Some(declared_enum.ident.to_string())
            } else if let syn::Item::Struct(declared_struct) = item {
                Some(declared_struct.ident.to_string())
            } else if let syn::Item::Type(declared_alias) = item {
                Some(declared_alias.ident.to_string())
            } else {
                None
            }
        })
        .collect();
    declared.sort();
    assert_eq!(declared, ADDED_TYPE_NAMES);
}
