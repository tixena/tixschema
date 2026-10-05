use super::enums::enum_recovering_decode;
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

/// One enum per form serde writes an enum in, each holding what its form can: nothing, one plain
/// value, one model type, several values, and named fields.
const EXTERNAL: &str = "pub enum Outline { Circle { radius: f64 }, Empty, Label(String), \
     Pinned(Inner), To(i32, i32) }";
const INTERNAL: &str = "#[serde(tag = \"kind\")] pub enum Fill { Clear, Solid { color: String }, \
     Versioned(Inner) }";
const ADJACENT: &str = "#[serde(tag = \"kind\", content = \"data\")] pub enum Stroke { \
     Dashed { gap: u32 }, Hairline, Level(Option<Inner>), Span(u32, u32), Width(u32) }";
const UNTAGGED: &str = "#[serde(untagged)] pub enum Contact { \
     Email { #[serde(deserialize_with = \"contact_schema::deserialize_email_address\")] address: String }, \
     Versioned(Inner), Word(String) }";

/// One enum per form serde writes an enum in, each with a variant serde also reads under an alias
/// and a variant it never reads.
const ALIASED_EXTERNAL: &str = "pub enum Contour { #[serde(alias = \"Round\")] Circle { radius: f64 }, \
     #[serde(alias = \"Blank\")] Empty, #[serde(alias = \"Hop\", alias = \"Leap\")] Jump(i32, i32), \
     #[serde(skip)] Lost, #[serde(skip_serializing)] Old(String) }";
const ALIASED_INTERNAL: &str = "#[serde(tag = \"kind\")] pub enum Coating { \
     #[serde(alias = \"Blank\")] Clear, #[serde(skip_deserializing)] Hidden, Solid { color: String } }";
const ALIASED_ADJACENT: &str = "#[serde(tag = \"kind\", content = \"data\")] pub enum Dash { \
     #[serde(alias = \"Dotted\", alias = \"Broken\")] Dashed { gap: u32 }, \
     #[serde(skip_deserializing)] Faded(u32), Hairline }";
const UNREAD_UNTAGGED: &str = "#[serde(untagged)] pub enum Reach { Email { address: String }, \
     #[serde(skip_deserializing)] Fax(Inner), #[serde(skip)] Pager { number: i32 }, Word(String) }";

/// A struct, an optional struct and a tagged enum, each flattened beside a key of the type's own.
const FLATTENING: &str = "pub struct Entry { #[serde(flatten)] pub audit: Audit, \
     #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub extra: Option<Extra>, \
     #[serde(flatten)] pub fill: Fill, pub title: String }";

/// A flattened type parameter beside a key of the type's own.
const FLATTENING_A_PARAMETER: &str =
    "pub struct Envelope<T> { #[serde(flatten)] pub body: T, pub id: String }";

/// The `impl` the flag adds to `source`, as text.
fn type_impl_of(source: &str) -> String {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    struct_recovering_decode(&item).type_impl.to_string()
}

/// The `impl` the flag adds to the enum `source` declares, as text.
fn enum_impl_of(source: &str) -> String {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    enum_recovering_decode(&item).type_impl.to_string()
}

/// What the flag adds to the enum `source` declares for a JSON value, as text: everything ahead
/// of the BSON entry point.
fn enum_json_of(source: &str) -> String {
    let emitted = enum_impl_of(source);
    emitted
        .split_once("pub fn from_bson_with")
        .map_or(emitted.as_str(), |(json, _bson)| json)
        .to_owned()
}

/// What the flag adds to the enum `source` declares for a BSON document, as text.
#[cfg(feature = "bson")]
fn enum_bson_of(source: &str) -> String {
    let emitted = enum_impl_of(source);
    let (_json, bson) = emitted.split_once("pub fn from_bson_with").unwrap();
    bson.to_owned()
}

/// The statements `decode_with_value_fields` runs for `source`, as text.
fn fields_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, walk) = emitted
        .split_once("pub fn decode_with_value_fields")
        .unwrap();
    walk.to_owned()
}

/// The statements `decode_with_value_fields` runs for `source`, as text, up to the BSON entry
/// point.
fn json_fields_walk_of(source: &str) -> String {
    let walk = fields_walk_of(source);
    walk.split_once("pub fn from_bson_with")
        .map_or(walk.as_str(), |(json, _bson)| json)
        .to_owned()
}

/// The statements `decode_with_value_issues` runs for `source`, as text, up to the next method.
fn issues_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, walk) = emitted
        .split_once("pub fn decode_with_value_issues")
        .unwrap();
    walk.split_once("pub fn ")
        .map_or(walk, |(body, _)| body)
        .to_owned()
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

/// The `impl`s the flag adds to `source`: each one's header as text, and its methods in the order
/// written.
fn added_impls(source: &str) -> Vec<(String, Vec<String>)> {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    impls_in(struct_recovering_decode(&item).type_impl)
}

/// The `impl`s the flag adds to the enum `source` declares, as [`added_impls`] lists a struct's.
fn added_enum_impls(source: &str) -> Vec<(String, Vec<String>)> {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    impls_in(enum_recovering_decode(&item).type_impl)
}

/// Every `impl` among `type_impl`: its header as text, and its methods in the order written.
fn impls_in(type_impl: proc_macro2::TokenStream) -> Vec<(String, Vec<String>)> {
    let added: syn::File = syn::parse2(type_impl).unwrap();
    let impls: Vec<(String, Vec<String>)> = added
        .items
        .iter()
        .filter_map(|added_item| {
            let syn::Item::Impl(block) = added_item else {
                return None;
            };
            let (generics, self_ty, where_clause) = (
                &block.generics,
                &block.self_ty,
                &block.generics.where_clause,
            );
            let methods: Vec<String> = block
                .items
                .iter()
                .filter_map(|member| {
                    if let syn::ImplItem::Fn(method) = member {
                        Some(method.sig.ident.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(methods.len(), block.items.len());
            Some((
                quote::quote!(impl #generics #self_ty #where_clause).to_string(),
                methods,
            ))
        })
        .collect();
    assert_eq!(impls.len(), added.items.len());
    impls
}

/// The names the methods among `type_impl` give type parameters of their own, each once, sorted.
fn own_type_parameters(type_impl: proc_macro2::TokenStream) -> Vec<String> {
    let added: syn::File = syn::parse2(type_impl).unwrap();
    let mut named: Vec<String> = added
        .items
        .iter()
        .filter_map(|added_item| {
            if let syn::Item::Impl(block) = added_item {
                Some(&block.items)
            } else {
                None
            }
        })
        .flatten()
        .filter_map(|member| {
            if let syn::ImplItem::Fn(method) = member {
                Some(method.sig.generics.type_params())
            } else {
                None
            }
        })
        .flatten()
        .map(|parameter| parameter.ident.to_string())
        .collect();
    named.sort_unstable();
    named.dedup();
    named
}

/// The methods one source adds to a type that gets a fields walker, and to one that gets none.
fn methods_of(stem: &str, keyed: bool) -> Vec<String> {
    let mut named = vec![
        format!("from_{stem}_with"),
        format!("decode_with_{stem}_report"),
        format!("decode_with_{stem}_issues"),
    ];
    if keyed {
        named.push(format!("decode_with_{stem}_fields"));
    }
    named
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
    for source in [EXTERNAL, INTERNAL, ADJACENT, UNTAGGED] {
        let tagged: syn::ItemEnum = syn::parse_str(source).unwrap();
        leaf_tokens(enum_recovering_decode(&tagged).type_impl, &mut leaves);
    }
    for source in [FLATTENING, FLATTENING_A_PARAMETER] {
        let flattening: syn::ItemStruct = syn::parse_str(source).unwrap();
        leaf_tokens(struct_recovering_decode(&flattening).type_impl, &mut leaves);
    }
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
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    // Only `mongodb` makes `ObjectId` a type tixschema knows, and so one it reads whole.
    #[cfg(feature = "mongodb")]
    assert!(
        walk.contains(
            "walked_schema :: bson_leaf (item , < ObjectId as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,"
        ),
        "got: {walk}"
    );
    assert!(!walk.contains("serde_json"), "got: {walk}");
    assert!(!walk.contains("decode_with_value"), "got: {walk}");
}

/// Without `mongodb`, `ObjectId` is a type tixschema does not know, so the BSON walker reaches it
/// as it reaches any model type: by that type's own BSON walker, never read whole.
#[cfg(all(feature = "bson", not(feature = "mongodb")))]
#[test]
fn without_mongodb_the_bson_walker_reaches_an_id_as_a_model_type() {
    let walk = bson_fields_walk_of(EVERY_WALK);
    assert!(
        walk.contains(
            "for (key , item) in entries { < ObjectId > :: decode_with_bson_issues (item , & [path , & [Ok (\"owners\" . to_owned ()) , Ok (key . clone ())]] . concat () , issue , out) ; }"
        ),
        "got: {walk}"
    );
    assert!(
        !walk.contains("< ObjectId as serde :: Deserialize >"),
        "got: {walk}"
    );
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

/// Each struct shape gets the methods serde's form of it has a use for, each named for the flag,
/// in one `impl`. A type serde cannot flatten gets no fields walker.
#[test]
fn each_struct_shape_adds_the_methods_its_form_has_a_use_for() {
    for (source, keyed) in [
        ("pub struct Pair(pub String, pub u32);", false),
        ("pub struct Brand(pub String);", false),
        ("pub struct Tags(pub Vec<Inner>);", false),
        ("pub struct Pinned(pub Inner);", true),
        ("pub struct Ping;", true),
    ] {
        let mut named = methods_of("value", keyed);
        if cfg!(feature = "bson") {
            named.extend(methods_of("bson", keyed));
        }
        let header = format!("impl {}", source.split([' ', '(', ';']).nth(2).unwrap());
        assert_eq!(added_impls(source), [(header, named)], "for {source}");
    }
}

/// A tuple struct is the array serde writes: each slot at its position, an absent one `Missing`,
/// and every position past the last slot `Unknown`.
#[test]
fn a_tuple_struct_is_walked_by_position() {
    let walk = issues_walk_of("pub struct Pair(pub String, pub Inner);");
    for written in [
        "let Some (items) = found . as_array () else { out . extend (pair_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"Model\" , & [\"Pair\"] , 0)] , issue)) ; return ; } ;",
        "match items . first () { Some (held) => out . extend (pair_schema :: value_leaf (held , < String as serde :: Deserialize > :: deserialize ,",
        "None => out . push (issue (\"Missing\" , [path , & [Err (0)]] . concat () , & [(\"String\" , & [] , 0)] , None , None , Vec :: new ())) ,",
        "match items . get (1) { Some (held) => < Inner > :: decode_with_value_issues (held , & [path , & [Err (1)]] . concat () , issue , out) ,",
        "for (index , held) in items . iter () . enumerate () . skip (2) { out . push (issue (\"Unknown\" , [path , & [Err (index)]] . concat () , & [] , Some (held . clone ()) , None , Vec :: new ())) ; }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// The array holds no position for a slot serde does not read, and serde reads a `default` slot
/// when its position is absent. With no slot at all, no position is skipped past.
#[test]
fn a_slot_is_a_position_only_where_serde_reads_one() {
    let walk = issues_walk_of(
        "pub struct Loose(pub String, #[serde(skip)] pub u8, #[serde(default)] pub u32);",
    );
    assert!(
        walk.contains("if let Some (held) = items . get (1) { out . extend (loose_schema :: value_leaf (held , < u32 as serde :: Deserialize > :: deserialize ,"),
        "got: {walk}"
    );
    assert!(
        walk.contains("in items . iter () . enumerate () . skip (2)"),
        "got: {walk}"
    );
    assert!(!walk.contains("u8"), "got: {walk}");

    let empty = issues_walk_of("pub struct Empty();");
    assert!(
        empty.contains("for (index , held) in items . iter () . enumerate () {"),
        "got: {empty}"
    );
    assert!(!empty.contains("skip"), "got: {empty}");
}

/// A single-slot tuple struct is read with its own reader, which runs whatever hook its slot
/// carries, and the issue names what the slot holds. The hook itself is never named.
#[test]
fn a_single_slot_struct_is_read_with_the_types_own_reader() {
    for source in [
        "pub struct Code(pub String);",
        "#[serde(transparent)] pub struct Code(pub String);",
        "pub struct Code(#[serde(deserialize_with = \"code_schema::deserialize_value\")] pub String);",
        "pub struct Code(#[serde(with = \"as_text\")] pub Vec<Inner>);",
    ] {
        let walk = issues_walk_of(source);
        assert!(
            walk.contains(
                "{ out . extend (code_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & ["
            ),
            "for {source}, got: {walk}"
        );
        assert!(
            !walk.contains("deserialize_value") && !walk.contains("as_text"),
            "for {source}, got: {walk}"
        );
    }
    let plain = issues_walk_of("pub struct Code(pub String);");
    assert!(
        plain.contains("path . to_vec () , & [(\"String\" , & [] , 0)] , issue)) ; }"),
        "got: {plain}"
    );
}

/// Over a flagged model type, a single-slot tuple struct hands both walks to that type at the
/// path it sits at, through any wrapper serde writes as the value it holds.
#[test]
fn a_single_slot_struct_over_a_model_type_hands_both_walks_to_it() {
    for source in [
        "pub struct Pinned(pub Inner);",
        "pub struct Pinned(pub Box<Inner>);",
    ] {
        let emitted = type_impl_of(source);
        for written in [
            "{ < Inner > :: decode_with_value_issues (found , path , issue , out) ; }",
            "-> Vec < & 'a str > { < Inner > :: decode_with_value_fields (object , path , issue , out) }",
        ] {
            assert!(
                emitted.contains(written),
                "for {source}, missing `{written}` in: {emitted}"
            );
        }
    }
}

/// Over a list or an `Option`, what a single-slot tuple struct holds is walked as a field of that
/// type is, at the path the struct itself sits at.
#[test]
fn a_single_slot_struct_over_a_list_or_an_option_walks_what_it_holds() {
    let listed = issues_walk_of("pub struct Tags(pub Vec<Inner>);");
    for written in [
        "match found { serde_json :: Value :: Array (items) => { for (index , item) in items . iter () . enumerate () { < Inner > :: decode_with_value_issues (item , & [path , & [Err (index)]] . concat () , issue , out) ; } } ,",
        "found => out . push (issue (\"Invalid\" , path . to_vec () , & [(\"Array\" , & [] , 1) , (\"Model\" , & [\"Inner\"] , 0)] , Some (found . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) ,",
    ] {
        assert!(listed.contains(written), "missing `{written}` in: {listed}");
    }
    let optional = issues_walk_of("pub struct Latest(pub Option<Inner>);");
    assert!(
        optional.contains(
            "match found { serde_json :: Value :: Null => { } , found => < Inner > :: decode_with_value_issues (found , path , issue , out) , }"
        ),
        "got: {optional}"
    );
}

/// A unit struct is the `{}` tixschema makes it write: every key is `Unknown`, and the fields
/// walker reads none of what it is handed, so it binds none of it.
#[test]
fn a_unit_struct_lists_every_key_and_its_fields_walker_binds_nothing() {
    let emitted = type_impl_of("pub struct Ping;");
    for written in [
        "let Some (object) = found . as_object () else { out . extend (ping_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize ,",
        "for (key , held) in object { out . push (issue (\"Unknown\" , [path , & [Ok (key . clone ())]] . concat () , & [] , Some (held . clone ()) , None , Vec :: new ())) ; }",
        "pub fn decode_with_value_fields < 'a , I > (_ : & 'a serde_json :: Map < String , serde_json :: Value > , \
         _ : & [core :: result :: Result < String , usize >] , \
         _ : ping_schema :: IssueFromParts < serde_json :: Value , I > , _ : & mut Vec < I > ,) \
         -> Vec < & 'a str > { Vec :: new () }",
    ] {
        assert!(
            emitted.contains(written),
            "missing `{written}` in: {emitted}"
        );
    }
}

/// A tuple is one more step of the walk, so it is walked by position under a list, a map and an
/// `Option`, each level binding names of its own. `()` is no tuple to serde, which writes `null`.
#[test]
fn a_tuple_in_a_fields_type_is_walked_by_position_wherever_it_is_held() {
    let walk = fields_walk_of(
        "pub struct Placed { pub by_name: HashMap<String, (String, u32)>, \
         pub maybe: Option<(String, u32)>, pub nothing: (), pub spot: (String, Inner), \
         pub spots: Vec<(String, u32)> }",
    );
    for written in [
        "for (key , item) in entries { match item { serde_json :: Value :: Array (items_1) => { match items_1 . first () { Some (held_1) =>",
        "[path , & [Ok (\"by_name\" . to_owned ()) , Ok (key . clone ()) , Err (1)]] . concat ()",
        "match object . get (\"maybe\") { None | Some (serde_json :: Value :: Null) => { } Some (serde_json :: Value :: Array (items)) => { match items . first () {",
        "Some (held) => out . push (issue (\"Invalid\" , [path , & [Ok (\"maybe\" . to_owned ())]] . concat () , & [(\"Optional\" , & [] , 1) , (\"Tuple\" , & [] , 2) , (\"String\" , & [] , 0) , (\"U32\" , & [] , 0)] , Some (held . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) ,",
        "placed_schema :: value_leaf (held , < () as serde :: Deserialize > :: deserialize ,",
        "match items . get (1) { Some (held) => < Inner > :: decode_with_value_issues (held , & [path , & [Ok (\"spot\" . to_owned ()) , Err (1)]] . concat () , issue , out) ,",
        "None => out . push (issue (\"Missing\" , [path , & [Ok (\"spot\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"String\" , & [] , 0) , (\"Model\" , & [\"Inner\"] , 0)] , None , None , Vec :: new ())) ,",
        "for (index , item) in items . iter () . enumerate () { match item { serde_json :: Value :: Array (items_1) =>",
        "for (index_1 , held_1) in items_1 . iter () . enumerate () . skip (2) { out . push (issue (\"Unknown\" , [path , & [Ok (\"spots\" . to_owned ()) , Err (index) , Err (index_1)]] . concat () ,",
        "item => out . push (issue (\"Invalid\" , [path , & [Ok (\"spots\" . to_owned ()) , Err (index)]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"String\" , & [] , 0) , (\"U32\" , & [] , 0)] , Some (item . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) ,",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// A type with a type parameter gets one `impl` per source. Each joins what that source reads and
/// writes a value with to the bounds the type declares, on every parameter and on the type itself.
/// A parameter is bounded in one place: its `where` predicate where the type wrote one, taking
/// along what the type wrote beside its name, and beside its name otherwise.
#[test]
fn a_generic_type_gets_one_impl_per_source_under_that_sources_bounds() {
    let added = added_impls(
        "pub struct Page<T: Clone, const N: usize, U = String, V: Send, W> \
         where U: Copy, V: Sync, Vec<W>: Clone { pub items: Vec<T>, pub other: (U, V, W) }",
    );
    let mut headers = vec![(
        "impl < T : Clone + serde :: de :: DeserializeOwned , const N : usize , U , V , \
         W : serde :: de :: DeserializeOwned > Page < T , N , U , V , W > \
         where U : Copy + serde :: de :: DeserializeOwned , \
         V : Sync + Send + serde :: de :: DeserializeOwned , Vec < W > : Clone , \
         Self : serde :: de :: DeserializeOwned"
            .to_owned(),
        methods_of("value", true),
    )];
    if cfg!(feature = "bson") {
        headers.push((
            "impl < T : Clone + serde :: de :: DeserializeOwned + serde :: Serialize , \
             const N : usize , U , V , \
             W : serde :: de :: DeserializeOwned + serde :: Serialize > \
             Page < T , N , U , V , W > \
             where U : Copy + serde :: de :: DeserializeOwned + serde :: Serialize , \
             V : Sync + Send + serde :: de :: DeserializeOwned + serde :: Serialize , \
             Vec < W > : Clone , Self : serde :: de :: DeserializeOwned + serde :: Serialize"
                .to_owned(),
            methods_of("bson", true),
        ));
    }
    assert_eq!(added, headers);
}

/// A method's own type parameter cannot be named as one the type declares, so each takes the next
/// free name where the type declares `F` or `I`, and keeps its own everywhere else.
#[test]
fn a_methods_own_type_parameter_is_never_one_the_type_declares() {
    let clashing =
        type_impl_of("pub struct Keyed<I, const F: usize, F2> { pub id: I, pub kept: F2 }");
    for written in [
        "pub fn from_value_with < F3 > (mut value : serde_json :: Value , decide : F3 ,)",
        "where F3 : FnOnce (& mut serde_json :: Value , & [keyed_schema :: Issue < serde_json :: Value >]) -> keyed_schema :: Verdict ,",
        "pub fn decode_with_value_issues < I2 > (found : & serde_json :: Value , \
         path : & [core :: result :: Result < String , usize >] , \
         issue : keyed_schema :: IssueFromParts < serde_json :: Value , I2 > , out : & mut Vec < I2 > ,)",
        "pub fn decode_with_value_fields < 'a , I2 > (",
        "< I as serde :: Deserialize > :: deserialize , | _ | None ,",
    ] {
        assert!(
            clashing.contains(written),
            "missing `{written}` in: {clashing}"
        );
    }
    let plain = type_impl_of("pub struct Page<T> { pub items: Vec<T> }");
    for written in [
        "pub fn from_value_with < F > (",
        "pub fn decode_with_value_issues < I > (",
        "pub fn decode_with_value_fields < 'a , I > (",
    ] {
        assert!(plain.contains(written), "missing `{written}` in: {plain}");
    }
}

/// A method's own type parameter hides a type of its name wherever the method's body writes one,
/// so each takes the next free name where the item writes `F` or `I` anywhere: a field's type, a
/// type held inside one, the item's own name, the path of a hook. Every other item keeps both.
#[test]
fn a_methods_own_type_parameter_is_never_a_name_the_item_writes() {
    for (source, named) in [
        ("pub struct Holder { pub inner: Inner }", ["F", "I"]),
        ("pub struct Holder { pub inner: I }", ["F", "I2"]),
        ("pub struct Holder { pub inner: r#I }", ["F", "I2"]),
        (
            "pub struct Holder { pub keyed: HashMap<String, Vec<Option<(I, i32)>>> }",
            ["F", "I2"],
        ),
        ("pub struct I { pub number: i32 }", ["F", "I2"]),
        (
            "pub struct Holder { #[serde(deserialize_with = \"I::positive\")] pub score: i32 }",
            ["F", "I2"],
        ),
        (
            "pub struct Holder { #[serde(flatten)] pub inner: I }",
            ["F", "I2"],
        ),
        (
            "pub struct Holder { #[serde(deserialize_with = \"parsed::<I, _>\")] pub score: i32 }",
            ["F", "I2"],
        ),
        ("pub struct F(pub String);", ["F2", "I"]),
        (
            "pub struct Holder { pub frame: F, pub inner: I, pub other: I2 }",
            ["F2", "I3"],
        ),
    ] {
        let item: syn::ItemStruct = syn::parse_str(source).unwrap();
        assert_eq!(
            own_type_parameters(struct_recovering_decode(&item).type_impl),
            named,
            "for {source}"
        );
    }
    for (source, named) in [
        (EXTERNAL, ["F", "I"]),
        (
            "pub enum Carried { Alone(I), Named { inner: I }, Paired(I, i32) }",
            ["F", "I2"],
        ),
        (
            "#[serde(untagged)] pub enum Alternate { Model(I), Text(String) }",
            ["F", "I2"],
        ),
        (
            "#[serde(tag = \"kind\")] pub enum Framed { Held(F) }",
            ["F2", "I"],
        ),
    ] {
        let item: syn::ItemEnum = syn::parse_str(source).unwrap();
        assert_eq!(
            own_type_parameters(enum_recovering_decode(&item).type_impl),
            named,
            "for {source}"
        );
    }
    let walk = issues_walk_of("pub struct Holder(pub String, pub I);");
    for written in [
        " < I2 > (found : & serde_json :: Value , path : & [core :: result :: Result < String , usize >] , \
         issue : holder_schema :: IssueFromParts < serde_json :: Value , I2 > , out : & mut Vec < I2 > ,)",
        "Some (held) => < I > :: decode_with_value_issues (held , & [path , & [Err (1)]] . concat () , issue , out) ,",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// Only a type parameter asks for a bound: with none, one `impl` under the type's own generics
/// holds every method, and its values are written back as any other type's are.
#[test]
fn a_type_with_no_type_parameter_keeps_one_impl_under_its_own_generics() {
    let source = "pub struct Grid<const N: usize> { pub cells: Vec<u8> }";
    let headers: Vec<String> = added_impls(source)
        .into_iter()
        .map(|(header, _methods)| header)
        .collect();
    assert_eq!(headers, ["impl < const N : usize > Grid < N >"]);
    assert!(
        type_impl_of(source).contains(
            "< Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () ,"
        )
    );
}

/// The JSON walker of a generic type is bound to read a parameter's value and the type itself,
/// never to write them: each is read whole and nothing is written back to compare. What holds a
/// parameter's value is read whole with it, and every other value keeps its write-back.
#[test]
fn a_parameters_value_and_the_type_itself_are_never_written_back_from_json() {
    let source = "pub struct Parcel<T> { pub both: (T, u32), pub extra: HashMap<String, T>, \
                  #[serde(deserialize_with = \"lenient\")] pub hooked: T, pub inner: Vec<Self>, \
                  pub items: Vec<T>, pub one: T, pub page: Page<T>, pub projected: T::Item, \
                  pub sender: Option<T>, pub total: u32 }";
    let emitted = type_impl_of(source);
    let (json, _bson) = emitted
        .split_once("pub fn from_bson_with")
        .unwrap_or((emitted.as_str(), ""));
    for written in [
        "parcel_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | _ | None , path . to_vec () , & [(\"Model\" , & [\"Parcel\"] , 0)] , issue)",
        "< (T , u32) as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"both\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"TypeParam\" , & [\"T\"] , 0) , (\"U32\" , & [] , 0)] ,",
        "< HashMap < String , T > as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"extra\" . to_owned ())]] . concat () , & [(\"Map\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , 0)] ,",
        "parcel_schema :: value_leaf (held , lenient , | _ : & T | None ,",
        "< Self > :: decode_with_value_issues (item , & [path , & [Ok (\"inner\" . to_owned ()) , Err (index)]] . concat () , issue , out)",
        "< Vec < T > as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"items\" . to_owned ())]] . concat () , & [(\"Array\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , 0)] ,",
        "< T as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"one\" . to_owned ())]] . concat () , & [(\"TypeParam\" , & [\"T\"] , 0)] ,",
        "< Page < T > > :: decode_with_value_issues (held , & [path , & [Ok (\"page\" . to_owned ())]] . concat () , issue , out)",
        "< T :: Item as serde :: Deserialize > :: deserialize , | _ | None ,",
        "< Option < T > as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"sender\" . to_owned ())]] . concat () , & [(\"Optional\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , 0)] ,",
        "< u32 as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () ,",
    ] {
        assert!(json.contains(written), "missing `{written}` in: {json}");
    }
    assert_eq!(
        json.matches("serde_json :: to_value").count(),
        1,
        "got: {json}"
    );
}

/// The BSON walker of a generic type is bound `Serialize` throughout, so it writes a parameter's
/// value and the type itself back as it writes any other.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_of_a_generic_type_writes_every_value_back() {
    let emitted = type_impl_of("pub struct Page<T> { pub items: Vec<T>, pub total: u32 }");
    let (_json, bson) = emitted.split_once("pub fn from_bson_with").unwrap();
    for written in [
        "page_schema :: bson_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,",
        "page_schema :: bson_leaf (held , < Vec < T > as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,",
    ] {
        assert!(bson.contains(written), "missing `{written}` in: {bson}");
    }
    assert!(!bson.contains("| _ | None"), "got: {bson}");
}

/// The BSON walk of each struct shape is its JSON one over the library's own types.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_of_each_struct_shape_matches_the_librarys_own_types() {
    for (source, written) in [
        (
            "pub struct Pair(pub String, pub u32);",
            "let bson :: Bson :: Array (items) = found else { out . extend (pair_schema :: bson_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () , path . to_vec () , & [(\"Model\" , & [\"Pair\"] , 0)] , issue)) ; return ; } ;",
        ),
        (
            "pub struct Code(pub String);",
            "{ out . extend (code_schema :: bson_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () , path . to_vec () , & [(\"String\" , & [] , 0)] , issue)) ; }",
        ),
        (
            "pub struct Pinned(pub Inner);",
            "-> Vec < & 'a str > { < Inner > :: decode_with_bson_fields (object , path , issue , out) }",
        ),
        (
            "pub struct Ping;",
            "pub fn decode_with_bson_fields < 'a , I > (_ : & 'a bson :: Document , _ : & [core :: result :: Result < String , usize >] , _ : ping_schema :: IssueFromParts < bson :: Bson , I > , _ : & mut Vec < I > ,) -> Vec < & 'a str > { Vec :: new () }",
        ),
        (
            "pub struct Placed { pub spot: (String, u32) }",
            "Some (bson :: Bson :: Array (items)) => { match items . first () { Some (held) => out . extend (placed_schema :: bson_leaf (held ,",
        ),
    ] {
        let emitted = type_impl_of(source);
        let (_json, bson) = emitted.split_once("pub fn from_bson_with").unwrap();
        assert!(
            bson.contains(written),
            "for {source}, missing `{written}` in: {bson}"
        );
        assert!(!bson.contains("serde_json"), "for {source}, got: {bson}");
    }
}

/// Each form serde writes an enum in gets the methods that form has a use for, in one `impl`. A
/// tagged enum and an untagged one are objects serde can flatten, so each gets a fields walker; a
/// plain enum gets none, and an untagged one a method per variant it walks itself.
#[test]
fn each_enum_form_adds_the_methods_its_form_has_a_use_for() {
    let forms: [(&str, bool, &[&str]); 5] = [
        ("pub enum Status { Draft, Published }", false, &[]),
        (EXTERNAL, true, &[]),
        (INTERNAL, true, &[]),
        (ADJACENT, true, &[]),
        (UNTAGGED, true, &["email", "word"]),
    ];
    for (source, keyed, walked) in forms {
        let of_source = |stem: &str| {
            let mut named = methods_of(stem, keyed);
            named.extend(
                walked
                    .iter()
                    .map(|variant| format!("decode_with_{stem}_variant_{variant}")),
            );
            named
        };
        let mut named = of_source("value");
        if cfg!(feature = "bson") {
            named.extend(of_source("bson"));
        }
        let (_attributes, declared) = source.split_once("pub enum ").unwrap();
        let header = format!("impl {}", declared.split(' ').next().unwrap());
        assert_eq!(added_enum_impls(source), [(header, named)], "for {source}");
    }
}

/// A plain enum is the name serde writes: one value, read with the enum's own reader. An enum of
/// unit variants under a tag is an object, walked by its tag.
#[test]
fn a_plain_enum_is_read_whole_and_one_under_a_tag_is_walked_by_its_tag() {
    let plain = enum_json_of("pub enum Status { Draft, Published }");
    assert!(
        plain.contains(
            "out : & mut Vec < I > ,) { out . extend (status_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"Model\" , & [\"Status\"] , 0)] , issue)) ; }"
        ),
        "got: {plain}"
    );
    let tagged = enum_json_of("#[serde(tag = \"code\")] pub enum Fault { Db, Io }");
    for written in [
        "let Some (tag) = object . get (\"code\") else {",
        "match tag . as_str () { Some (\"Db\") => vec ! [\"code\"] , Some (\"Io\") => vec ! [\"code\"] , _ => {",
    ] {
        assert!(tagged.contains(written), "missing `{written}` in: {tagged}");
    }
}

/// An externally tagged enum is a unit variant's name as text, or an object whose one key names a
/// variant over what it holds: nothing, one value, a model type, several values, or named fields.
/// A value in any other form is read whole, naming the variants.
#[test]
fn an_externally_tagged_enum_is_walked_under_the_key_naming_the_variant() {
    let walk = enum_json_of(EXTERNAL);
    for written in [
        "match found { serde_json :: Value :: String (tag) if matches ! (tag . as_str () , \"Empty\") => { } serde_json :: Value :: Object (object) if object . len () == 1 && object . keys () . any (| key | matches ! (key . as_str () , \"Circle\" | \"Label\" | \"Pinned\" | \"To\")) => { Self :: decode_with_value_fields (object , path , issue , out) ; } _ => out . extend (outline_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"Variants\" , & [\"Circle\" , \"Empty\" , \"Label\" , \"Pinned\" , \"To\"] , 0)] , issue)) , }",
        "if let Some (content) = object . get (\"Circle\") { if let serde_json :: Value :: Object (inner) = content { match inner . get (\"radius\") { Some (held) => out . extend (outline_schema :: value_leaf (held , < f64 as serde :: Deserialize > :: deserialize ,",
        "None => out . push (issue (\"Missing\" , [path , & [Ok (\"Circle\" . to_owned ()) , Ok (\"radius\" . to_owned ())]] . concat () , & [(\"F64\" , & [] , 0)] , None , None , Vec :: new ())) , }",
        "for (key , held) in inner { if ! matches ! (key . as_str () , \"radius\") { out . push (issue (\"Unknown\" , [path , & [Ok (\"Circle\" . to_owned ()) , Ok (key . clone ())]] . concat () , & [] , Some (held . clone ()) , None , Vec :: new ())) ; } } } return vec ! [\"Circle\"] ; }",
        "if object . contains_key (\"Empty\") { return vec ! [\"Empty\"] ; }",
        "if let Some (content) = object . get (\"Label\") { out . extend (outline_schema :: value_leaf (content , < String as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [Ok (\"Label\" . to_owned ())]] . concat () , & [(\"String\" , & [] , 0)] , issue)) ; return vec ! [\"Label\"] ; }",
        "if let Some (content) = object . get (\"Pinned\") { < Inner > :: decode_with_value_issues (content , & [path , & [Ok (\"Pinned\" . to_owned ())]] . concat () , issue , out) ; return vec ! [\"Pinned\"] ; }",
        "if let Some (content) = object . get (\"To\") { match content { serde_json :: Value :: Array (items) => { match items . first () {",
        "for (index , held) in items . iter () . enumerate () . skip (2) { out . push (issue (\"Unknown\" , [path , & [Ok (\"To\" . to_owned ()) , Err (index)]] . concat () ,",
        "content => out . push (issue (\"Invalid\" , [path , & [Ok (\"To\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"I32\" , & [] , 0) , (\"I32\" , & [] , 0)] , Some (content . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) , } return vec ! [\"To\"] ; }",
        "out . push (issue (\"Missing\" , path . to_vec () , & [(\"Variants\" , & [\"Circle\" , \"Empty\" , \"Label\" , \"Pinned\" , \"To\"] , 0)] , None , None , Vec :: new ())) ; Vec :: new () }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// An internally tagged enum reads its tag from a key of the object and walks the variant's fields
/// in that same object. A variant holding a model type hands the object to that type's fields
/// walker and adds the tag's key to the keys it returns.
#[test]
fn an_internally_tagged_enum_walks_the_variant_its_tag_names_in_the_same_object() {
    let walk = enum_json_of(INTERNAL);
    for written in [
        "let Some (object) = found . as_object () else { out . extend (fill_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"Model\" , & [\"Fill\"] , 0)] , issue)) ; return ; } ; let declared = Self :: decode_with_value_fields (object , path , issue , out) ; for (key , held) in object { if ! declared . contains (& key . as_str ()) {",
        "let Some (tag) = object . get (\"kind\") else { out . push (issue (\"Missing\" , [path , & [Ok (\"kind\" . to_owned ())]] . concat () , & [(\"Variants\" , & [\"Clear\" , \"Solid\" , \"Versioned\"] , 0)] , None , None , Vec :: new ())) ; return object . keys () . map (String :: as_str) . collect () ; } ;",
        "match tag . as_str () { Some (\"Clear\") => vec ! [\"kind\"] , Some (\"Solid\") => { match object . get (\"color\") {",
        "None => out . push (issue (\"Missing\" , [path , & [Ok (\"color\" . to_owned ())]] . concat () , & [(\"String\" , & [] , 0)] , None , None , Vec :: new ())) , } vec ! [\"kind\" , \"color\"] } ,",
        "Some (\"Versioned\") => { let mut declared = < Inner > :: decode_with_value_fields (object , path , issue , out) ; declared . push (\"kind\") ; declared } ,",
        "_ => { let here = [path , & [Ok (\"kind\" . to_owned ())]] . concat () ; out . push (match < Self as serde :: Deserialize > :: deserialize (serde_json :: Value :: Object (object . clone ())) { Err (refused) => issue (\"Invalid\" , here , & [(\"Variants\" , & [\"Clear\" , \"Solid\" , \"Versioned\"] , 0)] , Some (tag . clone ()) , Some (refused . to_string ()) , Vec :: new ()) , Ok (_) => issue (\"Mistyped\" , here , & [(\"Variants\" , & [\"Clear\" , \"Solid\" , \"Versioned\"] , 0)] , Some (tag . clone ()) , None , Vec :: new ()) , }) ; object . keys () . map (String :: as_str) . collect () } } }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// serde reads what an internally tagged variant holds from the whole object. Where that is no
/// model type with a fields walker to hand the object to, nothing is walked and every key counts
/// as the variant's, so serde's verdict is the read's.
#[test]
fn an_internally_tagged_variant_over_no_model_type_walks_nothing() {
    let walk = enum_json_of("#[serde(tag = \"kind\")] pub enum Reply<T> { Lost, Sent(T) }");
    assert!(
        walk.contains(
            "match tag . as_str () { Some (\"Lost\") => vec ! [\"kind\"] , Some (\"Sent\") => object . keys () . map (String :: as_str) . collect () , _ => {"
        ),
        "got: {walk}"
    );
}

/// An adjacently tagged enum reads its tag from one key and walks what the variant holds under
/// another, both its own. A unit variant holds nothing, and serde reads a single optional value
/// where the content key is missing.
#[test]
fn an_adjacently_tagged_enum_walks_what_the_variant_holds_under_the_content_key() {
    let walk = enum_json_of(ADJACENT);
    for written in [
        "return vec ! [\"kind\" , \"data\"] ; } ; match tag . as_str () { Some (\"Dashed\") => match object . get (\"data\") { Some (content) => if let serde_json :: Value :: Object (inner) = content { match inner . get (\"gap\") {",
        "for (key , held) in inner { if ! matches ! (key . as_str () , \"gap\") { out . push (issue (\"Unknown\" , [path , & [Ok (\"data\" . to_owned ()) , Ok (key . clone ())]] . concat () ,",
        "None => out . push (issue (\"Missing\" , [path , & [Ok (\"data\" . to_owned ())]] . concat () , & [(\"Model\" , & [\"Stroke\"] , 0)] , None , None , Vec :: new ())) , } , Some (\"Hairline\") => { } ,",
        "Some (\"Level\") => match object . get (\"data\") { None | Some (serde_json :: Value :: Null) => { } Some (content) => < Inner > :: decode_with_value_issues (content , & [path , & [Ok (\"data\" . to_owned ())]] . concat () , issue , out) , } ,",
        "Some (\"Span\") => match object . get (\"data\") { Some (serde_json :: Value :: Array (items)) => { match items . first () {",
        "Some (content) => out . push (issue (\"Invalid\" , [path , & [Ok (\"data\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"U32\" , & [] , 0) , (\"U32\" , & [] , 0)] , Some (content . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) , None => out . push (issue (\"Missing\" , [path , & [Ok (\"data\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"U32\" , & [] , 0) , (\"U32\" , & [] , 0)] , None , None , Vec :: new ())) , } ,",
        "Some (\"Width\") => match object . get (\"data\") { Some (content) => out . extend (stroke_schema :: value_leaf (content , < u32 as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [Ok (\"data\" . to_owned ())]] . concat () , & [(\"U32\" , & [] , 0)] , issue)) , None => out . push (issue (\"Missing\" , [path , & [Ok (\"data\" . to_owned ())]] . concat () , & [(\"U32\" , & [] , 0)] , None , None , Vec :: new ())) , } ,",
        "Ok (_) => issue (\"Mistyped\" , here , & [(\"Variants\" , & [\"Dashed\" , \"Hairline\" , \"Level\" , \"Span\" , \"Width\"] , 0)] , Some (tag . clone ()) , None , Vec :: new ()) , }) ; } } vec ! [\"kind\" , \"data\"] }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// An untagged enum is walked as the variant serde reads the value as. Where serde reads it as
/// none, every variant's walk runs into a list of its own, and the lists go into one `NoVariant`
/// in the order declared. A variant holding a model type is that type's own walker, and a member
/// carrying a read hook is read through it.
#[test]
fn an_untagged_enum_is_walked_as_the_variant_serde_reads() {
    let walk = enum_json_of(UNTAGGED);
    for written in [
        "match < Self as serde :: Deserialize > :: deserialize (found) { Ok (Self :: Email { .. }) => Self :: decode_with_value_variant_email (found , path , issue , out) , Ok (Self :: Versioned (..)) => < Inner > :: decode_with_value_issues (found , path , issue , out) , Ok (Self :: Word (..)) => Self :: decode_with_value_variant_word (found , path , issue , out) ,",
        "Err (_) => { let mut as_email = Vec :: new () ; Self :: decode_with_value_variant_email (found , path , issue , & mut as_email) ; let mut as_versioned = Vec :: new () ; < Inner > :: decode_with_value_issues (found , path , issue , & mut as_versioned) ; let mut as_word = Vec :: new () ; Self :: decode_with_value_variant_word (found , path , issue , & mut as_word) ; out . push (issue (\"NoVariant\" , path . to_vec () , & [] , Some (found . clone ()) , None , vec ! [(\"Email\" , as_email) , (\"Versioned\" , as_versioned) , (\"Word\" , as_word)])) ; } } }",
        "fn decode_with_value_variant_email < I > (found : & serde_json :: Value , path : & [core :: result :: Result < String , usize >] , issue : contact_schema :: IssueFromParts < serde_json :: Value , I > , out : & mut Vec < I > ,) { let Some (object) = found . as_object () else { out . extend (contact_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"Model\" , & [\"Contact\"] , 0)] , issue)) ; return ; } ;",
        "match object . get (\"address\") { Some (held) => out . extend (contact_schema :: value_leaf (held , contact_schema :: deserialize_email_address , | read : & String | serde_json :: to_value (read) . ok () , [path , & [Ok (\"address\" . to_owned ())]] . concat () , & [(\"String\" , & [] , 0)] , issue)) ,",
        "for (key , held) in object { if ! matches ! (key . as_str () , \"address\") { out . push (issue (\"Unknown\" , [path , & [Ok (key . clone ())]] . concat () ,",
        "fn decode_with_value_variant_word < I > (found : & serde_json :: Value , path : & [core :: result :: Result < String , usize >] , issue : contact_schema :: IssueFromParts < serde_json :: Value , I > , out : & mut Vec < I > ,) { out . extend (contact_schema :: value_leaf (found , < String as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"String\" , & [] , 0)] , issue)) ; }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    assert!(
        !walk.contains("pub fn decode_with_value_variant")
            && !walk.contains("decode_with_value_variant_versioned"),
        "got: {walk}"
    );
}

/// A tag and a key are walked under the name serde writes: a variant by its own `rename` over the
/// enum's `rename_all`, and a variant's field by the variant's `rename_all` over the enum's
/// `rename_all_fields`. `Variants` lists those names in the order declared.
#[test]
fn a_variant_and_its_fields_are_walked_under_the_names_serde_writes() {
    let walk = enum_json_of(
        "#[serde(rename_all = \"snake_case\", rename_all_fields = \"camelCase\")] \
         pub enum Shipment { \
         #[serde(rename_all = \"SCREAMING_SNAKE_CASE\")] ByAir { flight_code: String }, \
         BySea { vessel_name: String }, #[serde(rename = \"pickup\")] InPerson }",
    );
    for written in [
        "if matches ! (tag . as_str () , \"pickup\") => { }",
        "matches ! (key . as_str () , \"by_air\" | \"by_sea\")",
        "& [(\"Variants\" , & [\"by_air\" , \"by_sea\" , \"pickup\"] , 0)]",
        "if let Some (content) = object . get (\"by_air\") { if let serde_json :: Value :: Object (inner) = content { match inner . get (\"FLIGHT_CODE\") {",
        "[path , & [Ok (\"by_sea\" . to_owned ()) , Ok (\"vesselName\" . to_owned ())]] . concat ()",
        "if object . contains_key (\"pickup\") { return vec ! [\"pickup\"] ; }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// serde writes a variant whose one slot is off the wire as a unit variant, and the array of a
/// variant holding several slots has no position for a slot serde does not read.
#[test]
fn a_variants_slot_is_walked_only_where_serde_reads_one() {
    let walk = enum_json_of(
        "pub enum Loose { Kept(String), Off(#[serde(skip)] String), \
         Pair(#[serde(skip)] u8, i32, #[serde(default)] i32) }",
    );
    for written in [
        "serde_json :: Value :: String (tag) if matches ! (tag . as_str () , \"Off\") => { }",
        "matches ! (key . as_str () , \"Kept\" | \"Pair\")",
        "if object . contains_key (\"Off\") { return vec ! [\"Off\"] ; }",
        "if let Some (held) = items . get (1) { out . extend (loose_schema :: value_leaf (held , < i32 as serde :: Deserialize > :: deserialize ,",
        "in items . iter () . enumerate () . skip (2)",
        "& [(\"Tuple\" , & [] , 2) , (\"I32\" , & [] , 0) , (\"I32\" , & [] , 0)]",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    assert!(!walk.contains("u8"), "got: {walk}");
}

/// A generic enum gets one `impl` per source under the bounds a generic struct gets. From a JSON
/// value, a parameter's value is read whole where it sits and nothing is written back, the enum
/// itself included.
#[test]
fn a_generic_enum_gets_its_methods_under_each_sources_bounds() {
    let source = "pub enum Answer<T> { Empty, Value(T) }";
    let mut headers = vec![(
        "impl < T : serde :: de :: DeserializeOwned > Answer < T > \
         where Self : serde :: de :: DeserializeOwned"
            .to_owned(),
        methods_of("value", true),
    )];
    if cfg!(feature = "bson") {
        headers.push((
            "impl < T : serde :: de :: DeserializeOwned + serde :: Serialize > Answer < T > \
             where Self : serde :: de :: DeserializeOwned + serde :: Serialize"
                .to_owned(),
            methods_of("bson", true),
        ));
    }
    assert_eq!(added_enum_impls(source), headers);
    let walk = enum_json_of(source);
    for written in [
        "_ => out . extend (answer_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | _ | None , path . to_vec () , & [(\"Variants\" , & [\"Empty\" , \"Value\"] , 0)] , issue)) ,",
        "if let Some (content) = object . get (\"Value\") { out . extend (answer_schema :: value_leaf (content , < T as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"Value\" . to_owned ())]] . concat () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue)) ; return vec ! [\"Value\"] ; }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    assert!(!walk.contains("serde_json :: to_value"), "got: {walk}");
}

/// The BSON walk of each enum form is its JSON one over the library's own types: text, a document
/// and a list are matched as the members of `bson::Bson` that hold them, and serde reads a whole
/// value through the library's deserializer.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_of_each_enum_form_matches_the_librarys_own_types() {
    for (source, written) in [
        (
            EXTERNAL,
            "match found { bson :: Bson :: String (tag) if matches ! (tag . as_str () , \"Empty\") => { } bson :: Bson :: Document (object) if object . len () == 1 && object . keys () . any (| key | matches ! (key . as_str () , \"Circle\" | \"Label\" | \"Pinned\" | \"To\")) => { Self :: decode_with_bson_fields (object , path , issue , out) ; } _ => out . extend (outline_schema :: bson_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,",
        ),
        (
            EXTERNAL,
            "if let Some (content) = object . get (\"Circle\") { if let bson :: Bson :: Document (inner) = content { match inner . get (\"radius\") {",
        ),
        (
            EXTERNAL,
            "if let Some (content) = object . get (\"To\") { match content { bson :: Bson :: Array (items) => {",
        ),
        (
            INTERNAL,
            "Some (\"Versioned\") => { let mut declared = < Inner > :: decode_with_bson_fields (object , path , issue , out) ; declared . push (\"kind\") ; declared } ,",
        ),
        (
            INTERNAL,
            "out . push (match < Self as serde :: Deserialize > :: deserialize (bson :: Deserializer :: new (bson :: Bson :: Document (object . clone ()))) { Err (refused) => issue (\"Invalid\" , here ,",
        ),
        (
            ADJACENT,
            "Some (\"Level\") => match object . get (\"data\") { None | Some (bson :: Bson :: Null) => { } Some (content) => < Inner > :: decode_with_bson_issues (content ,",
        ),
        (
            UNTAGGED,
            "match < Self as serde :: Deserialize > :: deserialize (bson :: Deserializer :: new (found . clone ())) { Ok (Self :: Email { .. }) => Self :: decode_with_bson_variant_email (found , path , issue , out) , Ok (Self :: Versioned (..)) => < Inner > :: decode_with_bson_issues (found , path , issue , out) ,",
        ),
        (
            UNTAGGED,
            "contact_schema :: bson_leaf (held , contact_schema :: deserialize_email_address , | read : & String , to | serde :: Serialize :: serialize (read , to) . ok () ,",
        ),
    ] {
        let bson = enum_bson_of(source);
        assert!(
            bson.contains(written),
            "for {source}, missing `{written}` in: {bson}"
        );
        assert!(!bson.contains("serde_json"), "for {source}, got: {bson}");
        assert!(
            !bson.contains("decode_with_value"),
            "for {source}, got: {bson}"
        );
    }
}

/// serde reads a tagged variant under its name and under each alias, so the arm of a variant with
/// aliases matches them all, and the arm of one with none is the name alone.
#[test]
fn a_tag_stored_as_an_alias_is_matched_by_its_variants_arm() {
    let internal = enum_json_of(ALIASED_INTERNAL);
    assert!(
        internal.contains(
            "match tag . as_str () { Some (\"Clear\" | \"Blank\") => vec ! [\"kind\"] , Some (\"Solid\") => { match object . get (\"color\") {"
        ),
        "got: {internal}"
    );
    let adjacent = enum_json_of(ALIASED_ADJACENT);
    for written in [
        "match tag . as_str () { Some (\"Dashed\" | \"Dotted\" | \"Broken\") => match object . get (\"data\") { Some (content) => if let serde_json :: Value :: Object (inner) = content { match inner . get (\"gap\") {",
        "[path , & [Ok (\"data\" . to_owned ()) , Ok (\"gap\" . to_owned ())]] . concat ()",
        ", Some (\"Hairline\") => { } , _ => { let here = [path , & [Ok (\"kind\" . to_owned ())]] . concat () ;",
    ] {
        assert!(
            adjacent.contains(written),
            "missing `{written}` in: {adjacent}"
        );
    }
}

/// An externally tagged variant with aliases is found under the first of its tags the object
/// holds, bound as `tag` apart from the `stored` a field under an alias binds: the path segment of
/// what the variant holds, and the key returned as the enum's own.
#[test]
fn an_externally_tagged_variant_is_looked_up_under_its_name_and_each_alias() {
    let walk = enum_json_of(ALIASED_EXTERNAL);
    for written in [
        "serde_json :: Value :: String (tag) if matches ! (tag . as_str () , \"Empty\" | \"Blank\") => { }",
        "object . keys () . any (| key | matches ! (key . as_str () , \"Circle\" | \"Round\" | \"Jump\" | \"Hop\" | \"Leap\" | \"Old\"))",
        "if let Some ((tag , content)) = [\"Circle\" , \"Round\"] . into_iter () . find_map (| tag | object . get (tag) . map (| content | (tag , content))) { if let serde_json :: Value :: Object (inner) = content { match inner . get (\"radius\") {",
        "None => out . push (issue (\"Missing\" , [path , & [Ok (tag . to_owned ()) , Ok (\"radius\" . to_owned ())]] . concat () , & [(\"F64\" , & [] , 0)] , None , None , Vec :: new ())) , }",
        "out . push (issue (\"Unknown\" , [path , & [Ok (tag . to_owned ()) , Ok (key . clone ())]] . concat () , & [] , Some (held . clone ()) , None , Vec :: new ())) ; } } } return vec ! [tag] ; }",
        "if let Some (tag) = [\"Empty\" , \"Blank\"] . into_iter () . find (| & tag | object . contains_key (tag)) { return vec ! [tag] ; }",
        "if let Some ((tag , content)) = [\"Jump\" , \"Hop\" , \"Leap\"] . into_iter () . find_map (| tag | object . get (tag) . map (| content | (tag , content))) { match content { serde_json :: Value :: Array (items) => { match items . first () {",
        "out . push (issue (\"Unknown\" , [path , & [Ok (tag . to_owned ()) , Err (index)]] . concat () ,",
        "content => out . push (issue (\"Invalid\" , [path , & [Ok (tag . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"I32\" , & [] , 0) , (\"I32\" , & [] , 0)] , Some (content . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) , } return vec ! [tag] ; }",
        "if let Some (content) = object . get (\"Old\") { out . extend (contour_schema :: value_leaf (content , < String as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [Ok (\"Old\" . to_owned ())]] . concat () , & [(\"String\" , & [] , 0)] , issue)) ; return vec ! [\"Old\"] ; }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    let twice_aliased = enum_json_of(
        "pub enum Trail { #[serde(alias = \"Lane\")] Road { #[serde(alias = \"len\")] length: u32 } }",
    );
    assert!(
        twice_aliased.contains(
            "match [\"length\" , \"len\"] . into_iter () . find_map (| stored | inner . get (stored) . map (| held | (stored , held))) { Some ((stored , held)) => out . extend (trail_schema :: value_leaf (held , < u32 as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [Ok (tag . to_owned ()) , Ok (stored . to_owned ())]] . concat () ,"
        ),
        "got: {twice_aliased}"
    );
}

/// `Variants` lists the tags serde reads: variant by variant in the order declared, a variant's
/// name and then its aliases in the order written. Nothing emitted names a variant under
/// `skip_deserializing` or `skip`, and one under `skip_serializing` alone is still read.
#[test]
fn variants_lists_every_tag_serde_reads_and_none_of_a_variant_it_never_reads() {
    for (source, variants, never_read) in [
        (
            ALIASED_EXTERNAL,
            "& [(\"Variants\" , & [\"Circle\" , \"Round\" , \"Empty\" , \"Blank\" , \"Jump\" , \"Hop\" , \"Leap\" , \"Old\"] , 0)]",
            "Lost",
        ),
        (
            ALIASED_INTERNAL,
            "& [(\"Variants\" , & [\"Clear\" , \"Blank\" , \"Solid\"] , 0)]",
            "Hidden",
        ),
        (
            ALIASED_ADJACENT,
            "& [(\"Variants\" , & [\"Dashed\" , \"Dotted\" , \"Broken\" , \"Hairline\"] , 0)]",
            "Faded",
        ),
    ] {
        let emitted = enum_impl_of(source);
        let listed = emitted.matches("(\"Variants\" ,").count();
        assert!(
            listed > 0 && listed == emitted.matches(variants).count(),
            "for {source}, got: {emitted}"
        );
        assert!(
            !emitted.contains(never_read),
            "for {source}, got: {emitted}"
        );
    }
}

/// An untagged variant serde never reads is no variant to the walker: it gets no walk, no method
/// and no list inside `NoVariant`. The match on what serde read stays exhaustive through one arm
/// that lists nothing, written only where the enum has such a variant.
#[test]
fn an_untagged_variant_serde_never_reads_is_matched_and_never_walked() {
    let walk = enum_json_of(UNREAD_UNTAGGED);
    for written in [
        "Ok (Self :: Word (..)) => Self :: decode_with_value_variant_word (found , path , issue , out) , Ok (Self :: Fax (..) | Self :: Pager { .. }) => { } Err (_) => { let mut as_email = Vec :: new () ;",
        "let mut as_word = Vec :: new () ; Self :: decode_with_value_variant_word (found , path , issue , & mut as_word) ; out . push (issue (\"NoVariant\" , path . to_vec () , & [] , Some (found . clone ()) , None , vec ! [(\"Email\" , as_email) , (\"Word\" , as_word)])) ; } } }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    for absent in [
        "as_fax",
        "as_pager",
        "variant_pager",
        "\"Fax\"",
        "\"Pager\"",
        "Inner",
    ] {
        assert!(!walk.contains(absent), "found `{absent}` in: {walk}");
    }
    let of_source = |stem: &str| {
        let mut named = methods_of(stem, true);
        named.extend(
            ["email", "word"].map(|variant| format!("decode_with_{stem}_variant_{variant}")),
        );
        named
    };
    let mut named = of_source("value");
    if cfg!(feature = "bson") {
        named.extend(of_source("bson"));
    }
    assert_eq!(
        added_enum_impls(UNREAD_UNTAGGED),
        [("impl Reach".to_owned(), named)]
    );
    let every_variant_read = enum_json_of(UNTAGGED);
    assert!(
        every_variant_read.contains(
            "Ok (Self :: Word (..)) => Self :: decode_with_value_variant_word (found , path , issue , out) , Err (_) => {"
        ),
        "got: {every_variant_read}"
    );
}

/// The BSON walk of an aliased or never-read variant is its JSON one over the library's own types.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_reads_a_variants_alias_and_skip_as_the_json_one_does() {
    for (source, written) in [
        (
            ALIASED_EXTERNAL,
            "bson :: Bson :: String (tag) if matches ! (tag . as_str () , \"Empty\" | \"Blank\") => { } bson :: Bson :: Document (object) if object . len () == 1 && object . keys () . any (| key | matches ! (key . as_str () , \"Circle\" | \"Round\" | \"Jump\" | \"Hop\" | \"Leap\" | \"Old\")) => {",
        ),
        (
            ALIASED_EXTERNAL,
            "if let Some ((tag , content)) = [\"Circle\" , \"Round\"] . into_iter () . find_map (| tag | object . get (tag) . map (| content | (tag , content))) { if let bson :: Bson :: Document (inner) = content { match inner . get (\"radius\") {",
        ),
        (
            ALIASED_EXTERNAL,
            "if let Some (tag) = [\"Empty\" , \"Blank\"] . into_iter () . find (| & tag | object . contains_key (tag)) { return vec ! [tag] ; }",
        ),
        (
            ALIASED_INTERNAL,
            "match tag . as_str () { Some (\"Clear\" | \"Blank\") => vec ! [\"kind\"] , Some (\"Solid\") => {",
        ),
        (
            ALIASED_ADJACENT,
            "Some (\"Dashed\" | \"Dotted\" | \"Broken\") => match object . get (\"data\") { Some (content) => if let bson :: Bson :: Document (inner) = content {",
        ),
        (
            UNREAD_UNTAGGED,
            "Ok (Self :: Word (..)) => Self :: decode_with_bson_variant_word (found , path , issue , out) , Ok (Self :: Fax (..) | Self :: Pager { .. }) => { } Err (_) => {",
        ),
    ] {
        let bson = enum_bson_of(source);
        assert!(
            bson.contains(written),
            "for {source}, missing `{written}` in: {bson}"
        );
        assert!(!bson.contains("serde_json"), "for {source}, got: {bson}");
    }
}

/// A flattened type's keys sit among the object's own, so its fields walker runs in that object
/// at that object's path, and the keys it returns are kept as declared beside the type's own. A
/// flattened `Option` runs it into a list of its own, kept where serde reads the type from the
/// object and replaced by one `Mistyped` at the object where it does not.
#[test]
fn a_flattened_type_is_walked_in_the_outer_object_and_its_keys_are_declared() {
    let walk = json_fields_walk_of(FLATTENING);
    for written in [
        "-> Vec < & 'a str > { let mut declared = vec ! [\"title\"] ; match object . get (\"title\") { Some (held) =>",
        "declared . extend (< Audit > :: decode_with_value_fields (object , path , issue , out)) ; { let mut nested = Vec :: new () ; let keys = < Extra > :: decode_with_value_fields (object , path , issue , & mut nested) ; if keys . iter () . any (| key | object . contains_key (* key)) { match < Extra as serde :: Deserialize > :: deserialize (serde_json :: Value :: Object (object . clone ())) { Ok (_) => out . append (& mut nested) , Err (_) => out . push (issue (\"Mistyped\" , path . to_vec () , & [(\"Optional\" , & [] , 1) , (\"Model\" , & [\"Extra\"] , 0)] , Some (serde_json :: Value :: Object (object . clone ())) , None , Vec :: new ())) , } } declared . extend (keys) ; } declared . extend (< Fill > :: decode_with_value_fields (object , path , issue , out)) ; declared }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    assert!(!walk.contains("_schema :: decode_with"), "got: {walk}");
    let wrapped = json_fields_walk_of(
        "pub struct Boxed { #[serde(flatten)] pub audit: Box<Audit>, \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub extra: Option<Box<Extra>> }",
    );
    for written in [
        "{ let mut declared = Vec :: new () ; declared . extend (< Audit > :: decode_with_value_fields (object , path , issue , out)) ;",
        "match < Extra as serde :: Deserialize > :: deserialize (serde_json :: Value :: Object (object . clone ())) {",
        "& [(\"Optional\" , & [] , 1) , (\"Model\" , & [\"Extra\"] , 0)]",
    ] {
        assert!(
            wrapped.contains(written),
            "missing `{written}` in: {wrapped}"
        );
    }
}

/// A flattened map takes every key nothing else declares and walks each value at its key, so the
/// type then returns every key of the object. What else declares a key is read off the keys kept
/// where a flattened type declares some, off the type's own where none does, and off nothing where
/// the type has none.
#[test]
fn a_flattened_map_walks_the_value_of_every_key_nothing_else_declares() {
    let beside_a_type = json_fields_walk_of(
        "pub struct Bag { #[serde(flatten)] pub audit: Audit, \
         #[serde(flatten)] pub extra: HashMap<String, Inner>, pub title: String }",
    );
    assert!(
        beside_a_type.contains(
            "declared . extend (< Audit > :: decode_with_value_fields (object , path , issue , out)) ; for (key , item) in object { if ! declared . contains (& key . as_str ()) { < Inner > :: decode_with_value_issues (item , & [path , & [Ok (key . clone ())]] . concat () , issue , out) ; } } object . keys () . map (String :: as_str) . collect () }"
        ),
        "got: {beside_a_type}"
    );
    let beside_a_key = json_fields_walk_of(
        "pub struct Bag { #[serde(flatten)] pub extra: HashMap<String, i32>, pub title: String }",
    );
    assert!(
        beside_a_key.contains(
            "for (key , item) in object { if ! matches ! (key . as_str () , \"title\") { out . extend (bag_schema :: value_leaf (item , < i32 as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [Ok (key . clone ())]] . concat () , & [(\"I32\" , & [] , 0)] , issue)) ; } } object . keys () . map (String :: as_str) . collect () }"
        ),
        "got: {beside_a_key}"
    );
    assert!(!beside_a_key.contains("declared"), "got: {beside_a_key}");
    let alone = json_fields_walk_of(
        "pub struct Open { #[serde(flatten)] pub extra: HashMap<String, i32> }",
    );
    assert!(
        alone.contains(
            "-> Vec < & 'a str > { for (key , item) in object { out . extend (open_schema :: value_leaf (item , < i32 as serde :: Deserialize > :: deserialize ,"
        ),
        "got: {alone}"
    );
}

/// A flattened type parameter is read whole, with its own reader, from an object of the keys
/// nothing else declares, which is the whole object where the type declares none. A flattened
/// field its author's hook reads is read the same way, through the hook.
#[test]
fn a_flattened_parameter_is_read_whole_from_the_keys_nothing_else_declares() {
    let walk = json_fields_walk_of(FLATTENING_A_PARAMETER);
    assert!(
        walk.contains(
            "out . extend (envelope_schema :: value_leaf (& serde_json :: Value :: Object (object . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"id\")) . map (| (key , held) | (key . clone () , held . clone ())) . collect ()) , < T as serde :: Deserialize > :: deserialize , | _ | None , path . to_vec () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue)) ; object . keys () . map (String :: as_str) . collect () }"
        ),
        "got: {walk}"
    );
    let alone = json_fields_walk_of("pub struct Only<T> { #[serde(flatten)] pub body: T }");
    assert!(
        alone.contains(
            "-> Vec < & 'a str > { out . extend (only_schema :: value_leaf (& serde_json :: Value :: Object (object . clone ()) , < T as serde :: Deserialize > :: deserialize , | _ | None , path . to_vec () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue)) ; object . keys () . map (String :: as_str) . collect () }"
        ),
        "got: {alone}"
    );
    let hooked = json_fields_walk_of(
        "pub struct Hooked { #[serde(flatten)] pub audit: Audit, \
         #[serde(flatten, with = \"as_text\")] pub rest: HashMap<String, i32> }",
    );
    assert!(
        hooked.contains(
            "out . extend (hooked_schema :: value_leaf (& serde_json :: Value :: Object (object . iter () . filter (| (key , _) | ! declared . contains (& key . as_str ())) . map (| (key , held) | (key . clone () , held . clone ())) . collect ()) , as_text :: deserialize , | read : & HashMap < String , i32 > | as_text :: serialize (read , serde_json :: value :: Serializer) . ok () , path . to_vec () , & [(\"Map\" , & [] , 1) , (\"I32\" , & [] , 0)] , issue)) ;"
        ),
        "got: {hooked}"
    );
}

/// serde hands the first flattened field that takes the rest every key the walker hands it, and
/// what that one leaves for the next is nothing the walker can know. Only the first is walked, and
/// serde's verdict stands for the others.
#[test]
fn only_the_first_flattened_field_that_takes_the_rest_is_walked() {
    let walk = json_fields_walk_of(
        "pub struct Packet<T> { #[serde(flatten)] pub body: T, \
         #[serde(flatten)] pub counts: HashMap<String, i32>, pub id: String }",
    );
    assert!(
        walk.contains(
            "Vec :: new ())) , } out . extend (packet_schema :: value_leaf (& serde_json :: Value :: Object (object . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"id\")) . map (| (key , held) | (key . clone () , held . clone ())) . collect ()) , < T as serde :: Deserialize > :: deserialize , | _ | None , path . to_vec () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue)) ; object . keys () . map (String :: as_str) . collect () }"
        ),
        "got: {walk}"
    );
    assert!(!walk.contains("for (key , item) in object"), "got: {walk}");
    let maps = json_fields_walk_of(
        "pub struct Twice { #[serde(flatten)] pub first: HashMap<String, i32>, \
         #[serde(flatten)] pub second: HashMap<String, String> }",
    );
    assert_eq!(
        maps.matches("for (key , item) in object").count(),
        1,
        "got: {maps}"
    );
    assert!(
        maps.contains("< i32 as serde :: Deserialize >") && !maps.contains("< String as serde"),
        "got: {maps}"
    );
}

/// No walk reaches a flattened field serde reads as absent where its type's own reader would
/// refuse, one it refuses outright, or one it never reads: every key of the object counts as the
/// field's own and nothing is listed for it, so serde's verdict is the read's. A flattened type
/// beside it is still walked, and its keys are kept nowhere, nothing being left to read them.
#[test]
fn a_flattened_field_no_walk_reaches_takes_every_key_and_lists_nothing() {
    for source in [
        "pub struct Loose { #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub rest: Option<HashMap<String, i32>> }",
        "pub struct Loose<T> { #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub rest: Option<T> }",
        "pub struct Loose { #[serde(flatten)] pub rest: Vec<i32> }",
        "pub struct Loose { #[serde(flatten, skip_deserializing)] pub rest: Audit }",
    ] {
        let walk = json_fields_walk_of(source);
        assert!(
            walk.contains(
                ", _ : & mut Vec < I > ,) -> Vec < & 'a str > { object . keys () . map (String :: as_str) . collect () }"
            ),
            "for {source}, got: {walk}"
        );
    }
    let beside_a_type = json_fields_walk_of(
        "pub struct Beside { #[serde(flatten)] pub audit: Audit, \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub rest: Option<HashMap<String, i32>> }",
    );
    assert!(
        beside_a_type.contains(
            ", out : & mut Vec < I > ,) -> Vec < & 'a str > { < Audit > :: decode_with_value_fields (object , path , issue , out) ; object . keys () . map (String :: as_str) . collect () }"
        ),
        "got: {beside_a_type}"
    );
    // serde neither writes nor reads a flattened field under `skip`, so no key is its own.
    let off_the_wire = json_fields_walk_of(
        "pub struct Kept { #[serde(flatten, skip)] pub cached: Audit, pub title: String }",
    );
    assert!(
        off_the_wire.contains("vec ! [\"title\"] }") && !off_the_wire.contains("Audit"),
        "got: {off_the_wire}"
    );
}

/// A variant's flattened field is walked in the object the variant's fields sit in, at that
/// object's path, wherever the enum's form puts it. The keys it returns are the variant's own
/// beside the ones the variant declares, so a key outside both is `Unknown`.
#[test]
fn a_variants_flattened_field_is_walked_in_the_object_its_fields_sit_in() {
    let external = enum_json_of(
        "pub enum Outline { Gone, Made { #[serde(flatten)] audit: Audit, title: String } }",
    );
    for written in [
        "if let Some (content) = object . get (\"Made\") { if let serde_json :: Value :: Object (inner) = content { let mut declared = vec ! [\"title\"] ; match inner . get (\"title\") {",
        "declared . extend (< Audit > :: decode_with_value_fields (inner , & [path , & [Ok (\"Made\" . to_owned ())]] . concat () , issue , out)) ; for (key , held) in inner { if ! declared . contains (& key . as_str ()) { out . push (issue (\"Unknown\" , [path , & [Ok (\"Made\" . to_owned ()) , Ok (key . clone ())]] . concat () , & [] , Some (held . clone ()) , None , Vec :: new ())) ; } } } return vec ! [\"Made\"] ; }",
    ] {
        assert!(
            external.contains(written),
            "missing `{written}` in: {external}"
        );
    }
    let internal = enum_json_of(
        "#[serde(tag = \"kind\")] pub enum Fill { Gone, Made { #[serde(flatten)] audit: Audit, title: String } }",
    );
    for written in [
        "Some (\"Made\") => { let mut declared = vec ! [\"kind\" , \"title\"] ; match object . get (\"title\") {",
        "declared . extend (< Audit > :: decode_with_value_fields (object , path , issue , out)) ; declared } ,",
    ] {
        assert!(
            internal.contains(written),
            "missing `{written}` in: {internal}"
        );
    }
    let adjacent = enum_json_of(
        "#[serde(tag = \"kind\", content = \"data\")] pub enum Stroke { Gone, Made { #[serde(flatten)] audit: Audit, title: String } }",
    );
    for written in [
        "Some (\"Made\") => match object . get (\"data\") { Some (content) => if let serde_json :: Value :: Object (inner) = content { let mut declared = vec ! [\"title\"] ;",
        "declared . extend (< Audit > :: decode_with_value_fields (inner , & [path , & [Ok (\"data\" . to_owned ())]] . concat () , issue , out)) ; for (key , held) in inner { if ! declared . contains (& key . as_str ()) { out . push (issue (\"Unknown\" , [path , & [Ok (\"data\" . to_owned ()) , Ok (key . clone ())]] . concat () ,",
    ] {
        assert!(
            adjacent.contains(written),
            "missing `{written}` in: {adjacent}"
        );
    }
    let untagged = enum_json_of(
        "#[serde(untagged)] pub enum Contact { Gone { at: u32 }, Made { #[serde(flatten)] audit: Audit, title: String } }",
    );
    assert!(
        untagged.contains(
            "issue)) ; return ; } ; let mut declared = vec ! [\"title\"] ; match object . get (\"title\") {"
        ) && untagged.contains(
            "declared . extend (< Audit > :: decode_with_value_fields (object , path , issue , out)) ; for (key , held) in object { if ! declared . contains (& key . as_str ()) { out . push (issue (\"Unknown\" , [path , & [Ok (key . clone ())]] . concat () ,"
        ),
        "got: {untagged}"
    );
    // A variant that flattens a map leaves no key of its object undeclared.
    let open = enum_json_of(
        "pub enum Outline { Gone, Made { #[serde(flatten)] extra: HashMap<String, i32>, title: String } }",
    );
    assert!(
        open.contains(
            "for (key , item) in inner { if ! matches ! (key . as_str () , \"title\") { out . extend (outline_schema :: value_leaf (item , < i32 as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [Ok (\"Made\" . to_owned ()) , Ok (key . clone ())]] . concat () , & [(\"I32\" , & [] , 0)] , issue)) ; } } } return vec ! [\"Made\"] ; }"
        ),
        "got: {open}"
    );
}

/// An untagged enum's fields walker walks, in the object it is handed, the fields of the variant
/// serde reads that object as, and returns that variant's keys. Where serde reads it as none, one
/// `NoVariant` holds each variant's own list, without the keys that variant does not declare, and
/// every key of the object is returned.
#[test]
fn an_untagged_enums_fields_walker_walks_the_variant_serde_reads_the_object_as() {
    let emitted = enum_json_of(UNTAGGED);
    let (_issues, walk) = emitted
        .split_once("pub fn decode_with_value_fields")
        .unwrap();
    for written in [
        "-> Vec < & 'a str > { match < Self as serde :: Deserialize > :: deserialize (serde_json :: Value :: Object (object . clone ())) { Ok (Self :: Email { .. }) => { match object . get (\"address\") { Some (held) => out . extend (contact_schema :: value_leaf (held , contact_schema :: deserialize_email_address ,",
        "vec ! [\"address\"] } , Ok (Self :: Versioned (..)) => < Inner > :: decode_with_value_fields (object , path , issue , out) , Ok (Self :: Word (..)) => object . keys () . map (String :: as_str) . collect () , Err (_) => { let found = & serde_json :: Value :: Object (object . clone ()) ; let mut as_email = Vec :: new () ; { let out = & mut as_email ; match object . get (\"address\") {",
        "Vec :: new ())) , } } let mut as_versioned = Vec :: new () ; < Inner > :: decode_with_value_fields (object , path , issue , & mut as_versioned) ; let mut as_word = Vec :: new () ; Self :: decode_with_value_variant_word (found , path , issue , & mut as_word) ; out . push (issue (\"NoVariant\" , path . to_vec () , & [] , Some (found . clone ()) , None , vec ! [(\"Email\" , as_email) , (\"Versioned\" , as_versioned) , (\"Word\" , as_word)])) ; object . keys () . map (String :: as_str) . collect () } } } fn decode_with_value_variant_email < I > (",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    let unread = enum_json_of(UNREAD_UNTAGGED);
    assert!(
        unread.contains(
            "Ok (Self :: Word (..)) => object . keys () . map (String :: as_str) . collect () , Ok (Self :: Fax (..) | Self :: Pager { .. }) => Vec :: new () , Err (_) => { let found = & serde_json :: Value :: Object (object . clone ()) ;"
        ),
        "got: {unread}"
    );
    // A variant with no field to walk lists nothing, so its list is bound and never written to.
    let blank = enum_json_of("#[serde(untagged)] pub enum Sparse { Blank {}, Word(String) }");
    for written in [
        "Ok (Self :: Blank { .. }) => { vec ! [] } ,",
        "let as_blank = Vec :: new () ; let mut as_word = Vec :: new () ;",
    ] {
        assert!(blank.contains(written), "missing `{written}` in: {blank}");
    }
}

/// The BSON walk of a flattened field is its JSON one over the library's own types: serde reads a
/// whole document through the library's deserializer, and a document is the member of
/// `bson::Bson` that holds one.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_of_a_flattened_field_matches_the_librarys_own_types() {
    let flattening = bson_fields_walk_of(FLATTENING);
    for written in [
        "declared . extend (< Audit > :: decode_with_bson_fields (object , path , issue , out)) ; { let mut nested = Vec :: new () ; let keys = < Extra > :: decode_with_bson_fields (object , path , issue , & mut nested) ;",
        "match < Extra as serde :: Deserialize > :: deserialize (bson :: Deserializer :: new (bson :: Bson :: Document (object . clone ()))) { Ok (_) => out . append (& mut nested) , Err (_) => out . push (issue (\"Mistyped\" , path . to_vec () , & [(\"Optional\" , & [] , 1) , (\"Model\" , & [\"Extra\"] , 0)] , Some (bson :: Bson :: Document (object . clone ())) , None , Vec :: new ())) , }",
    ] {
        assert!(
            flattening.contains(written),
            "missing `{written}` in: {flattening}"
        );
    }
    let parameter = bson_fields_walk_of(FLATTENING_A_PARAMETER);
    assert!(
        parameter.contains(
            "out . extend (envelope_schema :: bson_leaf (& bson :: Bson :: Document (object . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"id\")) . map (| (key , held) | (key . clone () , held . clone ())) . collect ()) , < T as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () , path . to_vec () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue)) ;"
        ),
        "got: {parameter}"
    );
    let untagged = enum_bson_of(UNTAGGED);
    for written in [
        "-> Vec < & 'a str > { match < Self as serde :: Deserialize > :: deserialize (bson :: Deserializer :: new (bson :: Bson :: Document (object . clone ()))) { Ok (Self :: Email { .. }) =>",
        "Err (_) => { let found = & bson :: Bson :: Document (object . clone ()) ; let mut as_email = Vec :: new () ;",
        "< Inner > :: decode_with_bson_fields (object , path , issue , & mut as_versioned) ;",
    ] {
        assert!(
            untagged.contains(written),
            "missing `{written}` in: {untagged}"
        );
    }
    for bson in [flattening, parameter, untagged] {
        assert!(!bson.contains("serde_json"), "got: {bson}");
        assert!(!bson.contains("decode_with_value"), "got: {bson}");
    }
}
