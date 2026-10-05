#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::{ADDED_TYPE_NAMES, reading_the_authors_scope};
use super::{module_items, struct_recovering_decode};

/// One of every way a field is walked: read whole, through each kind of hook, item by item, and by
/// another type's walker, under its name or an alias.
const EVERY_WALK: &str = "pub struct Walked { \
     #[serde(alias = \"old\")] pub aliased: Option<Inner>, \
     #[serde(deserialize_with = \"lenient\")] pub read_hooked: Inner, \
     #[serde(deserialize_with = \"parsed\", serialize_with = \"shown\")] pub both_hooked: u16, \
     #[serde(with = \"as_text\")] pub in_module: u32, \
     pub latest: Option<Inner>, pub lists: Vec<Vec<Inner>>, pub owners: HashMap<String, ObjectId>, \
     pub plain: String }";

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

/// The statements `decode_with_bson_fields` runs for `source`, as text.
#[cfg(feature = "bson")]
fn bson_fields_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, walk) = emitted
        .split_once("pub fn decode_with_bson_fields")
        .unwrap();
    walk.to_owned()
}

/// Everything the flag emits for `source`, as text: what goes into the module, then the `impl`.
fn emission_of(source: &str) -> String {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let added = struct_recovering_decode(&item);
    format!("{} {}", added.schema_module, added.type_impl)
}

/// Every token `tokens` is made of, in order, with each group opened and its delimiters left out.
#[cfg(feature = "bson")]
fn leaf_tokens(tokens: proc_macro2::TokenStream, leaves: &mut Vec<String>) {
    for token in tokens {
        if let proc_macro2::TokenTree::Group(group) = token {
            leaf_tokens(group.stream(), leaves);
        } else {
            leaves.push(token.to_string());
        }
    }
}

/// The methods of the `impl Path` the flag puts into a schema module, in the order written.
fn path_methods() -> Vec<String> {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Impl(block) = item
                && block.trait_.is_none()
                && block.self_ty == syn::parse_quote!(Path)
            {
                Some(&block.items)
            } else {
                None
            }
        })
        .flatten()
        .filter_map(|member| {
            if let syn::ImplItem::Fn(method) = member {
                Some(method.sig.ident.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Every method the flag adds carries the flag's name, so none can meet one the type's author
/// wrote. Each entry point is the one it is named for.
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
    let mut named = vec![
        "from_value_with",
        "decode_with_value_report",
        "decode_with_value_issues",
        "decode_with_value_fields",
    ];
    if cfg!(feature = "bson") {
        named.extend([
            "from_bson_with",
            "decode_with_bson_report",
            "decode_with_bson_issues",
            "decode_with_bson_fields",
        ]);
    }
    assert_eq!(methods, named);
}

/// The BSON entry point and walker are written as their JSON twins are: a named type parameter
/// under a `where` clause, `core::result::Result` in full, and a report only the type calls.
#[cfg(feature = "bson")]
#[test]
fn the_bson_methods_carry_the_signatures_of_their_json_twins() {
    let item: syn::ItemStruct = syn::parse_str("pub struct Named { pub title: String }").unwrap();
    let added: syn::ItemImpl = syn::parse2(struct_recovering_decode(&item).type_impl).unwrap();
    let signatures: Vec<String> = added
        .items
        .iter()
        .filter_map(|added_item| {
            if let syn::ImplItem::Fn(method) = added_item
                && method.sig.ident.to_string().contains("bson")
            {
                let (visibility, signature) = (&method.vis, &method.sig);
                Some(quote::quote!(#visibility #signature).to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        signatures,
        [
            "pub fn from_bson_with < F > (mut document : bson :: Document , decide : F ,) \
             -> core :: result :: Result < Self , named_schema :: Unrecovered < bson :: Bson > > \
             where F : FnOnce (& mut bson :: Document , & [named_schema :: Issue < bson :: Bson >]) \
             -> named_schema :: Verdict ,",
            "fn decode_with_bson_report (document : & bson :: Document) \
             -> Vec < named_schema :: Issue < bson :: Bson > >",
            "pub fn decode_with_bson_issues < I > (found : & bson :: Bson , \
             path : & [core :: result :: Result < String , usize >] , \
             issue : named_schema :: IssueFromParts < bson :: Bson , I > , out : & mut Vec < I > ,)",
            "pub fn decode_with_bson_fields < 'a , I > (object : & 'a bson :: Document , \
             path : & [core :: result :: Result < String , usize >] , \
             issue : named_schema :: IssueFromParts < bson :: Bson , I > , out : & mut Vec < I > ,) \
             -> Vec < & 'a str >",
        ]
    );
}

/// A build without `bson` emits nothing of the BSON read: nothing names the library, and `Path`
/// fixes a JSON value only.
#[cfg(not(feature = "bson"))]
#[test]
fn without_bson_nothing_of_the_bson_read_is_emitted() {
    let emitted = emission_of(EVERY_WALK);
    for absent in ["bson", "same_bracket", "_document"] {
        assert!(!emitted.contains(absent), "found `{absent}` in: {emitted}");
    }
    assert_eq!(path_methods(), ["set_in_value", "remove_from_value"]);
}

/// Both major versions of the `bson` library have `Deserializer::new` and `Serializer::new`, and
/// each names the functions that read a whole document and write a whole value differently. The
/// emission names four items of the library and no other, so it names none of those.
#[cfg(feature = "bson")]
#[test]
fn the_bson_emission_names_only_what_both_major_versions_have() {
    let item: syn::ItemStruct = syn::parse_str(EVERY_WALK).unwrap();
    let added = struct_recovering_decode(&item);
    let mut leaves = Vec::new();
    leaf_tokens(added.schema_module, &mut leaves);
    leaf_tokens(added.type_impl, &mut leaves);
    let mut named: Vec<&str> = leaves
        .windows(4)
        .filter_map(|window| {
            if let [library, first, second, item_named] = window
                && library == "bson"
                && first == ":"
                && second == ":"
            {
                Some(item_named.as_str())
            } else {
                None
            }
        })
        .collect();
    named.sort_unstable();
    named.dedup();
    assert_eq!(named, ["Bson", "Deserializer", "Document", "Serializer"]);
    for one_version_only in [
        "deserialize_from_document",
        "from_document",
        "serialize_to_bson",
        "to_bson",
        "deserialize_from_bson",
        "from_bson",
    ] {
        assert!(
            leaves.iter().all(|leaf| leaf != one_version_only),
            "found `{one_version_only}`"
        );
    }
    let emitted = emission_of(EVERY_WALK);
    for through in [
        "bson :: Deserializer :: new (held . clone ())",
        "bson :: Serializer :: new ()",
        "bson :: Deserializer :: new (bson :: Bson :: Document (document . clone ()))",
    ] {
        assert!(
            emitted.contains(through),
            "missing `{through}` in: {emitted}"
        );
    }
}

/// The BSON items sit where the JSON ones do: the helpers inside the one `impl Path`, and the
/// function a value is read through in the module, once.
#[cfg(feature = "bson")]
#[test]
fn the_bson_items_are_emitted_beside_the_json_ones() {
    assert_eq!(
        path_methods(),
        [
            "set_in_value",
            "remove_from_value",
            "set_in_document",
            "remove_from_document",
        ]
    );
    let added: syn::File = syn::parse2(module_items()).unwrap();
    let functions: Vec<String> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Fn(function) = item {
                Some(function.sig.ident.to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        functions,
        [
            "expected_from_tokens",
            "issue_from_parts",
            "value_leaf",
            "same_bracket",
            "bson_leaf",
        ]
    );
}

/// The BSON walk is the JSON one over the library's own types: a list, a map and `null` are
/// matched as the members of `bson::Bson` that hold them, and another type is reached by its BSON
/// walker.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_matches_the_librarys_own_types() {
    let walk = bson_fields_walk_of(EVERY_WALK);
    for written in [
        "match [\"aliased\" , \"old\"] . into_iter () . find_map (| stored | object . get (stored) . map (| held | (stored , held)))",
        "Some ((stored , bson :: Bson :: Null)) => { }",
        "None | Some (bson :: Bson :: Null) => { }",
        "Some (bson :: Bson :: Array (items)) =>",
        "bson :: Bson :: Array (items_1) =>",
        "Some (bson :: Bson :: Document (entries)) =>",
        "< Inner > :: decode_with_bson_issues (item_1 ,",
        "walked_schema :: bson_leaf (item , < ObjectId as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    assert!(!walk.contains("serde_json"), "got: {walk}");
    assert!(!walk.contains("decode_with_value"), "got: {walk}");
}

/// A hook takes a deserializer and a serializer, not a value, so the BSON walker hands it the
/// library's own: the reader is passed as it is, and the writer is called with the serializer.
#[cfg(feature = "bson")]
#[test]
fn a_hook_is_handed_the_bson_deserializer_and_serializer() {
    let walk = bson_fields_walk_of(EVERY_WALK);
    for written in [
        "walked_schema :: bson_leaf (held , lenient , | read : & Inner , to | serde :: Serialize :: serialize (read , to) . ok () ,",
        "walked_schema :: bson_leaf (held , parsed , | read : & u16 , to | shown (read , to) . ok () ,",
        "walked_schema :: bson_leaf (held , as_text :: deserialize , | read : & u32 , to | as_text :: serialize (read , to) . ok () ,",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
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

/// The BSON walker binds what the JSON one binds: a parameter it does not read is `_`.
#[cfg(feature = "bson")]
#[test]
fn a_bson_walker_with_no_field_to_walk_leaves_the_issue_list_unbound() {
    let unwalked = bson_fields_walk_of("pub struct Blank {}");
    assert!(
        unwalked.starts_with(
            " < 'a , I > (object : & 'a bson :: Document , path : & [core :: result :: Result < String , usize >] , \
             issue : blank_schema :: IssueFromParts < bson :: Bson , I > , _ : & mut Vec < I > ,) \
             -> Vec < & 'a str > { vec ! []"
        ),
        "got: {unwalked}"
    );
    let walked = bson_fields_walk_of("pub struct Named { pub title: String }");
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
