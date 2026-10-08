#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::aliases::entry_of_items;
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::{ADDED_TYPE_NAMES, read_whole_items, reading_the_authors_scope};
use super::{RecoveringDecode, module_items, written_names};
use crate::utils::{Declared, record_declared};
use quote::ToTokens as _;

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

/// The enum emitter, run as it runs beside model types declared above the enum: every type the
/// item names is taken for one, so its walker is called as it stands.
fn enum_recovering_decode(item: &syn::ItemEnum) -> RecoveringDecode {
    seen_as_models(item.to_token_stream());
    super::enums::enum_recovering_decode(item)
}

/// Records every capitalized name `tokens` write as a model type declared above.
fn seen_as_models(tokens: proc_macro2::TokenStream) {
    for name in written_names(tokens) {
        if name.starts_with(char::is_uppercase) {
            record_declared(&name, Declared::Model);
        }
    }
}

/// The struct emitter, run the same way.
fn struct_recovering_decode(item: &syn::ItemStruct) -> RecoveringDecode {
    seen_as_models(item.to_token_stream());
    super::struct_recovering_decode(item)
}

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

/// The statements `decode_with_value_fields` runs for `source`, as text, to the end of the `impl`
/// with the typed paths `mongodb` writes after the walkers taken out of it.
fn fields_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, walk) = emitted
        .split_once("pub fn decode_with_value_fields")
        .unwrap();
    walk.split_once("# [doc = r\" The typed MongoDB paths of this type")
        .map_or_else(
            || walk.to_owned(),
            |(walkers, _paths)| format!("{walkers}}}"),
        )
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

/// The `impl`s the flag adds to `source`: each one's header as text, and its consts and methods
/// in the order written.
fn added_impls(source: &str) -> Vec<(String, Vec<String>)> {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    impls_in(struct_recovering_decode(&item).type_impl)
}

/// The `impl`s the flag adds to the enum `source` declares, as [`added_impls`] lists a struct's.
fn added_enum_impls(source: &str) -> Vec<(String, Vec<String>)> {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    impls_in(enum_recovering_decode(&item).type_impl)
}

/// Every `impl` among `type_impl`: its header as text, and its consts and methods in the order
/// written.
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
            let methods: Vec<String> = block.items.iter().filter_map(member_name).collect();
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

/// The name of a const or a method of an `impl`, and `None` for any other member of one.
fn member_name(member: &syn::ImplItem) -> Option<String> {
    if let syn::ImplItem::Fn(method) = member {
        Some(method.sig.ident.to_string())
    } else if let syn::ImplItem::Const(constant) = member {
        Some(constant.ident.to_string())
    } else {
        None
    }
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

/// What the typed paths add to every type, in the order written: under `mongodb`, the const that
/// holds them and the function that builds them, and nothing in a build without it.
fn paths_members() -> Vec<String> {
    if cfg!(feature = "mongodb") {
        vec!["MONGO_FIELDS".to_owned(), "mongo_fields_under".to_owned()]
    } else {
        Vec::new()
    }
}

/// What the typed paths of the struct `source` declares add to its schema module.
#[cfg(all(
    feature = "mongodb",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn paths_module_items(source: &str) -> proc_macro2::TokenStream {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    super::fields::struct_paths(&item, &[]).module_items
}

/// A build without `mongodb` writes no typed path.
#[cfg(all(
    not(feature = "mongodb"),
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn paths_module_items(_source: &str) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

/// The methods one source adds to every type, in the order written.
fn methods_of(stem: &str) -> Vec<String> {
    vec![
        format!("from_{stem}_with"),
        format!("from_{stem}_piped"),
        format!("decode_with_{stem}_report"),
        format!("decode_with_{stem}_issues"),
        format!("decode_with_{stem}_named"),
        format!("decode_with_{stem}_fields"),
    ]
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

/// The body of the method `named` among `type_impl`, as text.
fn body_of(type_impl: proc_macro2::TokenStream, named: &str) -> String {
    use quote::ToTokens as _;

    let added: syn::File = syn::parse2(type_impl).unwrap();
    let bodies: Vec<String> = added
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
            if let syn::ImplItem::Fn(method) = member
                && method.sig.ident == named
            {
                Some(method.block.to_token_stream().to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bodies.len(), 1, "for {named}");
    bodies.concat()
}

/// The `impl` the flag adds to one type of each shape, beside the source that declares it: structs
/// with named fields, with flattened fields, with slots and with none, and the five enum forms.
fn impls_of_every_shape() -> Vec<(&'static str, proc_macro2::TokenStream)> {
    let mut emitted = Vec::new();
    for source in [
        EVERY_WALK,
        FLATTENING,
        "pub struct Pair(String, u32);",
        "pub struct Ping;",
    ] {
        let item: syn::ItemStruct = syn::parse_str(source).unwrap();
        emitted.push((source, struct_recovering_decode(&item).type_impl));
    }
    for source in [
        EXTERNAL,
        INTERNAL,
        ADJACENT,
        UNTAGGED,
        "pub enum Status { Draft, Published }",
    ] {
        let item: syn::ItemEnum = syn::parse_str(source).unwrap();
        emitted.push((source, enum_recovering_decode(&item).type_impl));
    }
    emitted
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

/// The methods `added` declares in the inherent `impl` blocks of its types, each with the name of
/// the type it is declared on, in the order written.
#[cfg(feature = "mongodb")]
fn query_methods(added: &syn::File) -> Vec<(String, &syn::ImplItemFn)> {
    added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Impl(block) = item
                && block.trait_.is_none()
                && let syn::Type::Path(own) = &*block.self_ty
                && let Some(named) = own.path.segments.last()
            {
                Some((named.ident.to_string(), &block.items))
            } else {
                None
            }
        })
        .flat_map(|(on, members)| {
            members.iter().filter_map(move |member| {
                if let syn::ImplItem::Fn(method) = member {
                    Some((on.clone(), method))
                } else {
                    None
                }
            })
        })
        .collect()
}

/// Every const and method the flag adds carries the flag's name, so none can meet one the type's
/// author wrote. Each entry point is the one it is named for.
#[test]
fn every_added_method_but_the_entry_point_carries_the_flags_name() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Named { pub title: String, pub versions: Vec<Version> }")
            .unwrap();
    let added: syn::ItemImpl = syn::parse2(struct_recovering_decode(&item).type_impl).unwrap();
    let methods: Vec<String> = added.items.iter().filter_map(member_name).collect();
    assert_eq!(methods.len(), added.items.len());
    let mut named = vec![
        "from_value_with",
        "from_value_piped",
        "decode_with_value_report",
        "decode_with_value_issues",
        "decode_with_value_named",
        "decode_with_value_fields",
    ];
    if cfg!(feature = "bson") {
        named.extend([
            "from_bson_with",
            "from_bson_piped",
            "decode_with_bson_report",
            "decode_with_bson_issues",
            "decode_with_bson_named",
            "decode_with_bson_fields",
        ]);
    }
    // The typed paths carry the name of the feature that adds them.
    if cfg!(feature = "mongodb") {
        named.extend(["MONGO_FIELDS", "mongo_fields_under"]);
    }
    assert_eq!(methods, named);
}

/// The BSON entry points and walker are written as their JSON twins are: a named type parameter
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
            "pub fn from_bson_with < F > (document : bson :: Document , decide : F ,) \
             -> :: core :: result :: Result < Self , named_schema :: Unrecovered < bson :: Bson > > \
             where F : :: core :: ops :: FnOnce (& mut bson :: Document , & [named_schema :: Issue < bson :: Bson >]) \
             -> named_schema :: Verdict ,",
            "pub fn from_bson_piped (document : bson :: Document , resolvers : & [named_schema :: \
             Resolver < '_ , bson :: Document , bson :: Bson >] ,) -> :: core :: result :: Result < \
             Self , named_schema :: Unrecovered < bson :: Bson > >",
            "fn decode_with_bson_report (whole : & bson :: Bson , refused : :: core :: option :: \
             Option < :: std :: string :: String >) -> :: std :: vec :: Vec < named_schema :: Issue < \
             bson :: Bson > >",
            "pub fn decode_with_bson_issues < I > (found : & bson :: Bson , path : & [:: core :: \
             result :: Result < :: std :: string :: String , usize >] , issue : named_schema :: \
             IssueFromParts < bson :: Bson , I > , out : & mut :: std :: vec :: Vec < I > ,)",
            "pub fn decode_with_bson_named (object : & bson :: Document) -> bool",
            "pub fn decode_with_bson_fields < 'a , I > (object : & 'a bson :: Document , path : \
             & [:: core :: result :: Result < :: std :: string :: String , usize >] , issue : \
             named_schema :: IssueFromParts < bson :: Bson , I > , out : & mut :: std :: vec :: \
             Vec < I > ,) -> :: std :: vec :: Vec < & 'a str >",
        ]
    );
    // The JSON report is told what serde said as the BSON one is.
    let json_report = added.items.iter().find_map(|added_item| {
        if let syn::ImplItem::Fn(method) = added_item
            && method.sig.ident == "decode_with_value_report"
        {
            let signature = &method.sig;
            Some(quote::quote!(#signature).to_string())
        } else {
            None
        }
    });
    assert_eq!(
        json_report.as_deref(),
        Some(
            "fn decode_with_value_report (value : & serde_json :: Value , refused : :: core :: \
             option :: Option < :: std :: string :: String >) -> :: std :: vec :: Vec < named_schema \
             :: Issue < serde_json :: Value > >"
        )
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
        "bson :: Deserializer :: new (whole . clone ())",
    ] {
        assert!(
            emitted.contains(through),
            "missing `{through}` in: {emitted}"
        );
    }
}

/// `from_value_with` reads the value with serde once per decode and tells its report what serde
/// said, so the report reads nothing with serde on its own account, whatever the type's shape.
#[test]
fn from_value_with_reads_the_value_once_per_decode_and_its_report_reads_nothing() {
    for (source, type_impl) in impls_of_every_shape() {
        let entry = body_of(type_impl.clone(), "from_value_with");
        // The first read, and the one after the callback answers `Fixed`.
        assert_eq!(
            entry
                .matches("< Self as serde :: Deserialize > :: deserialize (& value)")
                .count(),
            2,
            "for {source}, got: {entry}"
        );
        assert_eq!(
            entry.matches("deserialize").count(),
            2,
            "for {source}, got: {entry}"
        );
        for told in [
            "Self :: decode_with_value_report (& value , :: core :: option :: Option :: None)",
            "Self :: decode_with_value_report (& value , :: core :: option :: Option :: Some \
             (refused . to_string ()))",
            "Self :: decode_with_value_report (& value , read . as_ref () . err () . map (:: std :: string :: ToString :: to_string))",
        ] {
            assert_eq!(entry.matches(told).count(), 1, "for {source}, got: {entry}");
        }

        let report = body_of(type_impl, "decode_with_value_report");
        assert!(
            report.contains("Self :: decode_with_value_issues (value , & [] ,"),
            "for {source}, got: {report}"
        );
        for absent in ["deserialize", "Deserialize"] {
            assert!(!report.contains(absent), "for {source}, got: {report}");
        }
    }
}

/// `bson::Deserializer::new` takes what it reads by value, so `from_bson_with` copies the document
/// once per read of it by serde. Its report borrows the document and is told what serde said, so
/// it copies nothing and reads nothing, whatever the type's shape.
#[cfg(feature = "bson")]
#[test]
fn from_bson_with_copies_the_document_once_per_read_by_serde_and_its_report_copies_nothing() {
    for (source, type_impl) in impls_of_every_shape() {
        let entry = body_of(type_impl.clone(), "from_bson_with");
        assert!(
            entry.contains("let mut whole = bson :: Bson :: Document (document) ;"),
            "for {source}, got: {entry}"
        );
        // The first read, and the one after the callback answers `Fixed`.
        assert_eq!(
            entry
                .matches("bson :: Deserializer :: new (whole . clone ())")
                .count(),
            2,
            "for {source}, got: {entry}"
        );
        for once_per_read in ["Deserializer", "clone"] {
            assert_eq!(
                entry.matches(once_per_read).count(),
                2,
                "for {source}, got: {entry}"
            );
        }

        let report = body_of(type_impl, "decode_with_bson_report");
        assert!(
            report.contains("Self :: decode_with_bson_issues (whole , & [] ,"),
            "for {source}, got: {report}"
        );
        for absent in ["clone", "Deserializer"] {
            assert!(!report.contains(absent), "for {source}, got: {report}");
        }
    }
}

/// A piped entry point opens with the read its callback twin opens with and closes with the read
/// that twin makes after `Fixed`. Between the two it runs the pipe once, over the value the
/// callback is handed, and refuses with what the pipe answers.
#[test]
fn a_piped_entry_point_makes_the_two_reads_of_its_callback_twin_around_the_pipe() {
    let mut sources = vec![("value", "& mut value")];
    if cfg!(feature = "bson") {
        sources.push(("bson", "object"));
    }
    for (source, type_impl) in impls_of_every_shape() {
        for (stem, raw) in &sources {
            let with = body_of(type_impl.clone(), &format!("from_{stem}_with"));
            let (first_read, decided) = with.split_once("match decide (").unwrap();
            let (_verdicts, after_fixed) = decided.split_once(":: Verdict :: Fixed => { ").unwrap();
            let second_read = after_fixed.strip_suffix(" } } }").unwrap();

            let piped = body_of(type_impl.clone(), &format!("from_{stem}_piped"));
            let (opened, piping) = piped.split_once("let left = ").unwrap();
            let (pipe, closing) = piping.split_once("{ issues : left }) ; } ").unwrap();
            assert_eq!(opened, first_read, "for {source}");
            assert_eq!(
                closing.strip_suffix(" }"),
                Some(second_read),
                "for {source}"
            );
            assert!(
                pipe.contains(&format!(
                    ":: unsettled ({raw} , & found , resolvers) ; if ! left . is_empty () {{ return"
                )),
                "for {source}, got: {pipe}"
            );
            assert_eq!(piped.matches("unsettled").count(), 1, "for {source}");
        }
    }
}

/// A resolver answers one issue with one of three states. The pipe takes its resolvers as trait
/// objects, which a closure holding state is one of, and names the standard items it reads in
/// full.
#[test]
fn the_schema_module_declares_what_a_resolver_answers_and_the_pipe_that_runs_them() {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    let answers: Vec<(bool, Vec<String>)> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Enum(declared) = item
                && declared.ident == "Resolution"
            {
                Some((
                    declared
                        .attrs
                        .iter()
                        .any(|attribute| attribute.path().is_ident("non_exhaustive")),
                    declared
                        .variants
                        .iter()
                        .map(|variant| variant.ident.to_string())
                        .collect(),
                ))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        answers,
        [(
            true,
            vec![
                "Settled".to_owned(),
                "Rejected".to_owned(),
                "NotTouched".to_owned()
            ]
        )]
    );
    let written: Vec<String> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Type(alias) = item
                && alias.ident == "Resolver"
            {
                let (generics, aliased) = (&alias.generics, &alias.ty);
                Some(quote::quote!(#generics = #aliased).to_string())
            } else if let syn::Item::Fn(function) = item
                && function.sig.ident == "unsettled"
            {
                let (visibility, signature) = (&function.vis, &function.sig);
                Some(quote::quote!(#visibility #signature).to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        written,
        [
            "< 'r , D , V > = & 'r dyn :: core :: ops :: Fn (& mut D , & Issue < V >) -> Resolution",
            "pub fn unsettled < D , V : :: core :: clone :: Clone > (raw : & mut D , issues : & \
             [Issue < V >] , resolvers : & [Resolver < '_ , D , V >] ,) -> :: std :: vec :: Vec < \
             Issue < V > >",
        ]
    );
}

/// The BSON items sit where the JSON ones do: the helpers inside the one `impl Path`, and the
/// function a value is read through in the module, once. Under `mongodb` the query types add one
/// function after them, which a path with no hook writes its values through.
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
            "unsettled",
            "expected_from_tokens",
            "issue_from_parts",
            "value_leaf",
            "same_bracket",
            "bson_leaf",
            "bson_remaining",
            "reads_an_option",
            "taken_keys",
            "value_remaining",
            "value_bound",
            "bson_bound",
            #[cfg(feature = "mongodb")]
            "write_plain",
        ]
    );
}

/// The query types are in the module in a build with `mongodb` and in no other, each one
/// `#[non_exhaustive]`, beside the alias of the error a value that cannot be written fails with.
#[test]
fn the_query_types_are_declared_under_mongodb_alone_and_none_is_exhaustive() {
    let query_types = [
        "MongoPath",
        "Filter",
        "Update",
        "Field",
        "OptionalField",
        "Element",
        "ListField",
        "Model",
        "OptionalModel",
        "ModelList",
    ];
    let added: syn::File = syn::parse2(module_items()).unwrap();
    let declared: Vec<(String, bool)> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Struct(declared) = item
                && query_types.contains(&declared.ident.to_string().as_str())
            {
                Some((
                    declared.ident.to_string(),
                    declared
                        .attrs
                        .iter()
                        .any(|attribute| attribute.path().is_ident("non_exhaustive")),
                ))
            } else {
                None
            }
        })
        .collect();
    let expected: Vec<(String, bool)> = if cfg!(feature = "mongodb") {
        query_types
            .iter()
            .map(|named| ((*named).to_owned(), true))
            .collect()
    } else {
        Vec::new()
    };
    assert_eq!(declared, expected);
    let aliased = added
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Type(alias) if alias.ident == "WriteError"));
    assert_eq!(aliased, cfg!(feature = "mongodb"));
}

/// Each query type has the operators of its kind and no other: a path every row holds compares
/// and sets, one a row may leave out adds `$exists` and `$unset`, a list has the operators over
/// its elements, and a nested model has the ones over its whole value.
#[cfg(feature = "mongodb")]
#[test]
fn each_query_type_has_the_operators_of_its_kind() {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    for (named, operators) in [
        ("MongoPath", &["under", "at", "key"][..]),
        ("Filter", &["raw", "and", "or", "negated", "into_document"]),
        ("Update", &["raw", "and", "into_document"]),
        (
            "Field",
            &[
                "plain",
                "hooked",
                "segments",
                "eq",
                "ne",
                "gt",
                "gte",
                "lt",
                "lte",
                "is_in",
                "not_in",
                "set",
                "set_on_insert",
                "regex",
            ],
        ),
        ("OptionalField", &["plain", "hooked", "exists", "unset"]),
        ("Element", &["eq", "ne", "gt", "gte", "lt", "lte"]),
        (
            "ListField",
            &[
                "plain",
                "hooked",
                "contains",
                "contains_any",
                "contains_none",
                "size",
                "element",
                "elem_match",
                "push",
                "pull",
                "set",
                "set_on_insert",
            ],
        ),
        (
            "Model",
            &[
                "plain",
                "eq",
                "ne",
                "is_in",
                "not_in",
                "set",
                "set_on_insert",
            ],
        ),
        ("OptionalModel", &["plain", "exists", "unset", "set"]),
        (
            "ModelList",
            &["plain", "elem_match", "size", "push", "pull", "set"],
        ),
    ] {
        let public: Vec<String> = query_methods(&added)
            .into_iter()
            .filter(|(on, method)| on == named && matches!(method.vis, syn::Visibility::Public(_)))
            .map(|(_on, method)| method.sig.ident.to_string())
            .collect();
        assert_eq!(public, operators, "for {named}");
    }
}

/// An operator that writes a value takes it by value, as the type its path is declared with, and
/// answers the error a value that cannot be written fails with. One that writes none answers the
/// filter or the update itself.
#[cfg(feature = "mongodb")]
#[test]
fn an_operator_that_writes_a_value_takes_it_by_value_and_answers_a_write_error() {
    let added: syn::File = syn::parse2(super::query::query_items()).unwrap();
    for (on, method) in query_methods(&added) {
        let signature = method.sig.to_token_stream().to_string();
        let answered = method.sig.output.to_token_stream().to_string();
        let writes = ["value : V", "value : M", "values : I"]
            .iter()
            .any(|taken| signature.contains(taken));
        assert_eq!(
            answered.ends_with(", WriteError >"),
            writes,
            "for {on}: {signature}"
        );
        assert!(
            !signature.contains("value : &") && !signature.contains("values : &"),
            "for {on}: {signature}"
        );
    }
}

/// A consumer denying clippy's `restriction` set denies it over the query types too: no function
/// takes `impl Trait`, and every bound sits in a `where` clause on a named type parameter.
#[cfg(feature = "mongodb")]
#[test]
fn every_bound_of_a_query_function_is_written_in_a_where_clause() {
    let added: syn::File = syn::parse2(super::query::query_items()).unwrap();
    let functions = added.items.iter().filter_map(|item| {
        if let syn::Item::Fn(function) = item {
            Some(&function.sig)
        } else {
            None
        }
    });
    let methods = query_methods(&added);
    for signature in functions.chain(methods.iter().map(|(_on, method)| &method.sig)) {
        let written = signature.to_token_stream().to_string();
        assert!(
            signature.generics.params.iter().all(|parameter| matches!(
                parameter,
                syn::GenericParam::Type(named) if named.bounds.is_empty()
            )),
            "for {written}"
        );
        assert!(!written.contains("impl "), "for {written}");
    }
}

/// A filter carries the rows it is over and an update the rows it changes, as two standard types,
/// and every function that takes one asks for its marker: neither stands where the other is
/// asked. A list of models asks a filter over its element's rows.
#[cfg(feature = "mongodb")]
#[test]
fn a_filter_and_an_update_each_carry_a_marker_of_their_own() {
    let emitted = super::query::query_items();
    let packed: String = emitted.to_string().split_whitespace().collect();
    for carried in [
        "impl<Root>::core::convert::AsRef<::core::marker::PhantomData<Root>>forFilter<Root>",
        "impl<Root>::core::convert::AsRef<::core::marker::PhantomData<fn(Root)->Root>>forUpdate<Root>",
    ] {
        assert!(packed.contains(carried), "missing `{carried}`");
    }
    assert_eq!(packed.matches("AsRef<").count(), 8);
    let added: syn::File = syn::parse2(emitted).unwrap();
    let asked: Vec<(String, String)> = query_methods(&added)
        .into_iter()
        .filter_map(|(on, method)| {
            let bounds = method.sig.generics.where_clause.as_ref()?;
            let bound = bounds.to_token_stream().to_string();
            let (_taken, marker) =
                bound.split_once("AsRef < :: core :: marker :: PhantomData <")?;
            Some((
                format!("{on}::{}", method.sig.ident),
                marker.trim_matches([' ', ',', '>']).to_owned(),
            ))
        })
        .collect();
    let expected = [
        ("Filter::joined", "Root"),
        ("Filter::and", "Root"),
        ("Filter::or", "Root"),
        ("Update::and", "fn (Root) -> Root"),
        ("ModelList::elem_match", "M"),
        ("ModelList::pull", "M"),
    ]
    .map(|(function, marker)| (function.to_owned(), marker.to_owned()));
    assert_eq!(asked, expected);
}

/// Of the `bson` library the query types name only what both of its major versions have: the
/// serializer, a value with five of its members, and a document. No `doc!` is written.
#[cfg(feature = "mongodb")]
#[test]
fn the_query_types_name_only_what_both_majors_of_the_bson_library_have() {
    let written = super::query::query_items().to_string();
    let named_after = |prefix: &str| {
        let mut found: Vec<&str> = written
            .split(prefix)
            .skip(1)
            .filter_map(|rest| {
                rest.split(|read: char| !(read.is_ascii_alphanumeric() || read == '_'))
                    .next()
            })
            .collect();
        found.sort_unstable();
        found.dedup();
        found
    };
    assert_eq!(named_after("bson :: "), ["Bson", "Document", "Serializer"]);
    assert_eq!(
        named_after("bson :: Bson :: "),
        ["Array", "Boolean", "Document", "Int64", "String"]
    );
    assert_eq!(named_after("bson :: Document :: "), ["new"]);
    assert_eq!(named_after("bson :: Serializer :: "), ["new"]);
    assert!(!written.contains("doc !"), "got: {written}");
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
        ":: core :: option :: Option :: Some ((stored , bson :: Bson :: Null)) => { }",
        ":: core :: option :: Option :: None | :: core :: option :: Option :: Some (bson :: Bson :: \
         Null) => { }",
        ":: core :: option :: Option :: Some (bson :: Bson :: Array (items)) =>",
        "bson :: Bson :: Array (items_1) =>",
        ":: core :: option :: Option :: Some (bson :: Bson :: Document (entries)) =>",
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
            walk.contains(
                ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { vec \
                 ! ["
            ),
            "for {source}, got: {walk}"
        );
    }
    let walked = fields_walk_of("pub struct Named { pub title: String }");
    assert!(
        walked.contains(
            ", out : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { match \
             object . get (\"title\")"
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
            " < 'a , I > (object : & 'a bson :: Document , path : & [:: core :: result :: Result < \
             :: std :: string :: String , usize >] , issue : blank_schema :: IssueFromParts < bson \
             :: Bson , I > , _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a \
             str > { vec ! []"
        ),
        "got: {unwalked}"
    );
    let walked = bson_fields_walk_of("pub struct Named { pub title: String }");
    assert!(
        walked.contains(
            ", out : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { match \
             object . get (\"title\")"
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
    let mut every = module_items();
    every.extend(read_whole_items());
    every.extend(entry_of_items());
    every.extend(paths_module_items("pub struct Named { pub title: String }"));
    let added: syn::File = syn::parse2(every).unwrap();
    let mut declared: Vec<String> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Enum(declared_enum) = item {
                Some(declared_enum.ident.to_string())
            } else if let syn::Item::Struct(declared_struct) = item {
                Some(declared_struct.ident.to_string())
            } else if let syn::Item::Trait(declared_trait) = item {
                Some(declared_trait.ident.to_string())
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

/// Every struct shape gets the same methods, each named for the flag, in one `impl`: a type that
/// holds it asks whether an object names it and calls both walker methods on it, whatever form
/// serde writes it in.
#[test]
fn every_struct_shape_adds_the_same_methods() {
    for source in [
        "pub struct Pair(pub String, pub u32);",
        "pub struct Brand(pub String);",
        "pub struct Tags(pub Vec<Inner>);",
        "pub struct Pinned(pub Inner);",
        "pub struct Ping;",
    ] {
        let mut named = methods_of("value");
        if cfg!(feature = "bson") {
            named.extend(methods_of("bson"));
        }
        named.extend(paths_members());
        let header = format!("impl {}", source.split([' ', '(', ';']).nth(2).unwrap());
        assert_eq!(added_impls(source), [(header, named)], "for {source}");
    }
}

/// The fields walker of a type serde refuses to flatten lists nothing and returns no key: a tuple
/// struct, and a single-slot struct over text, a list or a tuple. It reads none of what it is
/// handed, so it binds none of it.
#[test]
fn a_tuple_struct_and_a_slot_over_text_a_list_or_a_tuple_claim_no_key() {
    for source in [
        "pub struct Code(pub String, pub u32);",
        "pub struct Code(pub String);",
        "#[serde(transparent)] pub struct Code(pub String);",
        "pub struct Code(pub Vec<Inner>);",
        "pub struct Code(pub Option<Vec<Inner>>);",
        "pub struct Code(pub (String, u32));",
        "#[serde(transparent)] pub struct Code { pub inner: Vec<Inner> }",
    ] {
        let emitted = type_impl_of(source);
        assert!(
            emitted.contains(
                "pub fn decode_with_value_fields < 'a , I > (_ : & 'a serde_json :: Map < :: std \
                 :: string :: String , serde_json :: Value > , _ : & [:: core :: result :: Result \
                 < :: std :: string :: String , usize >] , _ : code_schema :: IssueFromParts < \
                 serde_json :: Value , I > , _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec \
                 :: Vec < & 'a str > { :: std :: vec :: Vec :: new () }"
            ),
            "for {source}, got: {emitted}"
        );
        assert_eq!(
            emitted.contains(
                "pub fn decode_with_bson_fields < 'a , I > (_ : & 'a bson :: Document , _ : & \
                 [:: core :: result :: Result < :: std :: string :: String , usize >] , _ : \
                 code_schema :: IssueFromParts < bson :: Bson , I > , _ : & mut :: std :: vec :: \
                 Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { :: std :: vec :: Vec :: new () }"
            ),
            cfg!(feature = "bson"),
            "for {source}, got: {emitted}"
        );
    }
}

/// A tuple struct is the array serde writes: each slot at its position, an absent one `Missing`,
/// and every position past the last slot `Unknown`.
#[test]
fn a_tuple_struct_is_walked_by_position() {
    let walk = issues_walk_of("pub struct Pair(pub String, pub Inner);");
    for written in [
        "let :: core :: option :: Option :: Some (items) = found . as_array () else { out . extend \
         (pair_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , \
         | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"Model\" , & \
         [\"Pair\"] , 0)] , issue)) ; return ; } ;",
        "match items . first () { :: core :: option :: Option :: Some (held) => out . extend \
         (pair_schema :: value_leaf (held , < String as serde :: Deserialize > :: deserialize ,",
        "None => out . push (issue (\"Missing\" , [path , & [:: core :: result :: Result :: Err \
         (0)]] . concat () , & [(\"String\" , & [] , 0)] , :: core :: option :: Option :: None , \
         :: core :: option :: Option :: None , :: std :: vec :: Vec :: new ())) ,",
        "match items . get (1) { :: core :: option :: Option :: Some (held) => < Inner > :: \
         decode_with_value_issues (held , & [path , & [:: core :: result :: Result :: Err (1)]] . \
         concat () , issue , out) ,",
        "for (index , held) in items . iter () . enumerate () . skip (2) { out . push (issue \
         (\"Unknown\" , [path , & [:: core :: result :: Result :: Err (index)]] . concat () , & [] \
         , :: core :: option :: Option :: Some (held . clone ()) , :: core :: option :: Option :: \
         None , :: std :: vec :: Vec :: new ())) ; }",
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
        walk.contains(
            "if let :: core :: option :: Option :: Some (held) = items . get (1) { out . extend \
             (loose_schema :: value_leaf (held , < u32 as serde :: Deserialize > :: deserialize \
             ,"
        ),
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
            "-> :: std :: vec :: Vec < & 'a str > { < Inner > :: decode_with_value_fields (object \
             , path , issue , out) }",
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
        "match found { serde_json :: Value :: Array (items) => { for (index , item) in items . \
         iter () . enumerate () { < Inner > :: decode_with_value_issues (item , & [path , & \
         [:: core :: result :: Result :: Err (index)]] . concat () , issue , out) ; } } ,",
        "found => out . push (issue (\"Invalid\" , path . to_vec () , & [(\"Array\" , & [] , 1) \
         , (\"Model\" , & [\"Inner\"] , 0)] , :: core :: option :: Option :: Some (found . clone \
         ()) , :: core :: option :: Option :: Some (\"not an array\" . to_owned ()) , :: std :: vec \
         :: Vec :: new ())) ,",
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

/// serde writes a `#[serde(transparent)]` struct with a named field as it writes a single-slot
/// tuple struct: the value held. Both get the same methods running the same walk, from every
/// source, whatever the field holds.
#[test]
fn a_transparent_struct_with_a_named_field_is_walked_as_a_single_slot_struct_is() {
    for (named, slot) in [
        ("{ pub inner: String }", "(pub String);"),
        ("{ pub inner: Inner }", "(pub Inner);"),
        ("{ pub inner: Box<Inner> }", "(pub Box<Inner>);"),
        ("{ pub inner: Vec<Inner> }", "(pub Vec<Inner>);"),
        ("{ pub inner: Option<Inner> }", "(pub Option<Inner>);"),
        ("{ pub inner: (String, u32) }", "(pub (String, u32));"),
        (
            "{ #[serde(with = \"as_text\")] pub inner: Vec<Inner> }",
            "(#[serde(with = \"as_text\")] pub Vec<Inner>);",
        ),
        (
            "{ #[serde(deserialize_with = \"code_schema::deserialize_named_inner\")] pub inner: Inner }",
            "(pub Inner);",
        ),
    ] {
        assert_eq!(
            type_impl_of(&format!("#[serde(transparent)] pub struct Code {named}")),
            type_impl_of(&format!("#[serde(transparent)] pub struct Code{slot}")),
            "for {named}"
        );
    }
    assert_eq!(
        type_impl_of("#[serde(transparent)] pub struct Code<T> { pub inner: T }"),
        type_impl_of("#[serde(transparent)] pub struct Code<T>(pub T);"),
    );
}

/// A `#[serde(transparent)]` struct with a named field gets a fields walker whatever its field
/// holds: the walk the type that flattens that value itself would run. Over a model type it
/// hands the walk to that type's own, and over an `Option` of one it walks it where the object
/// names it. Over a value serde refuses to flatten it lists nothing and returns no key.
#[test]
fn a_transparent_struct_with_a_named_field_gets_a_fields_walker_whatever_it_holds() {
    for (field, walked) in [
        ("String", ":: std :: vec :: Vec :: new ()"),
        ("Vec<Inner>", ":: std :: vec :: Vec :: new ()"),
        (
            "Option<Inner>",
            "let mut declared = :: std :: vec :: Vec :: new () ; if < Inner > :: \
             decode_with_value_named (object) { let mut nested = :: std :: vec :: Vec :: new () ; \
             let keys = < Inner > :: decode_with_value_fields (object , path , issue , & mut \
             nested) ; match < Inner as serde :: Deserialize > :: deserialize (serde_json :: \
             Value :: Object (object . clone ())) { :: core :: result :: Result :: Ok (_) => out . \
             append (& mut nested) , :: core :: result :: Result :: Err (_) => out . push (issue \
             (\"Mistyped\" , path . to_vec () , & [(\"Optional\" , & [] , 1) , (\"Model\" , & \
             [\"Inner\"] , 0)] , :: core :: option :: Option :: Some (serde_json :: Value :: \
             Object (object . clone ())) , :: core :: option :: Option :: None , :: std :: vec :: Vec \
             :: new ())) , } declared . extend (keys) ; } declared",
        ),
        (
            "Inner",
            "< Inner > :: decode_with_value_fields (object , path , issue , out)",
        ),
    ] {
        let mut named = methods_of("value");
        if cfg!(feature = "bson") {
            named.extend(methods_of("bson"));
        }
        named.extend(paths_members());
        let source = format!("#[serde(transparent)] pub struct Code {{ pub inner: {field} }}");
        assert_eq!(
            added_impls(&source),
            [("impl Code".to_owned(), named)],
            "for {source}"
        );
        let walk = json_fields_walk_of(&source);
        assert!(
            walk.contains(&format!(
                "-> :: std :: vec :: Vec < & 'a str > {{ {walked} }}"
            )),
            "for {source}, got: {walk}"
        );
    }
}

/// The field walked is the one serde's derive reads the struct as the value of: a field it never
/// reads, one with a `default` and a `PhantomData` are passed over, and `transparent` is read
/// wherever it is written. A struct that derive refuses is left to the walk of its keys.
#[test]
fn a_transparent_struct_is_walked_as_the_one_field_serde_reads_it_as() {
    let alone = type_impl_of("#[serde(transparent)] pub struct Code { pub inner: Inner }");
    for source in [
        "#[serde(transparent)] pub struct Code { #[serde(skip)] pub cached: u8, pub inner: Inner }",
        "#[serde(transparent)] pub struct Code { pub inner: Inner, #[serde(skip_deserializing)] pub seen: u8 }",
        "#[serde(transparent)] pub struct Code { #[serde(default, skip_serializing)] pub hits: u8, pub inner: Inner }",
        "#[serde(transparent)] pub struct Code { pub inner: Inner, pub marker: PhantomData<u8> }",
        "#[serde(transparent)] pub struct Code { pub inner: Inner, pub marker: core::marker::PhantomData<u8> }",
        "#[serde(rename = \"code\", transparent)] pub struct Code { pub inner: Inner }",
        "#[serde(bound(deserialize = \"\"), transparent)] pub struct Code { pub inner: Inner }",
    ] {
        assert_eq!(type_impl_of(source), alone, "for {source}");
    }
    for refused in [
        "#[serde(transparent)] pub struct Code { pub inner: Inner, pub other: Inner }",
        "#[serde(transparent)] pub struct Code { #[serde(default)] pub inner: Inner }",
    ] {
        let walk = fields_walk_of(refused);
        assert!(
            walk.contains("object . get (\"inner\")"),
            "for {refused}, got: {walk}"
        );
    }
}

/// serde's derive picks the value of a `#[serde(transparent)]` tuple struct as it picks a named
/// field's, so whatever other slots the struct declares, it gets the `impl` of the single-slot
/// one. A struct that derive refuses, and one with no `transparent`, are walked by position.
#[test]
fn a_transparent_tuple_struct_is_walked_as_the_one_slot_serde_reads_it_as() {
    for (slot, wider) in [
        (
            "(pub Vec<String>);",
            "(pub Vec<String>, #[serde(skip)] pub u8);",
        ),
        ("(pub String);", "(#[serde(skip)] pub u8, pub String);"),
        (
            "(pub Inner);",
            "(pub Inner, #[serde(skip_deserializing)] pub u8);",
        ),
        (
            "(pub Inner);",
            "(pub Inner, #[serde(default, skip_serializing)] pub u8);",
        ),
        ("(pub Inner);", "(pub Inner, pub PhantomData<u8>);"),
        (
            "(pub Inner);",
            "(pub core::marker::PhantomData<u8>, pub Inner);",
        ),
        (
            "(#[serde(with = \"as_text\")] pub Vec<Inner>);",
            "(#[serde(with = \"as_text\")] pub Vec<Inner>, #[serde(skip)] pub u8);",
        ),
    ] {
        assert_eq!(
            type_impl_of(&format!("#[serde(transparent)] pub struct Code{wider}")),
            type_impl_of(&format!("#[serde(transparent)] pub struct Code{slot}")),
            "for {wider}"
        );
    }
    assert_eq!(
        type_impl_of("#[serde(transparent)] pub struct Code<T>(pub T, pub PhantomData<T>);"),
        type_impl_of("#[serde(transparent)] pub struct Code<T>(pub T);"),
    );
    for positional in [
        "pub struct Code(pub Inner, #[serde(skip)] pub u8);",
        "#[serde(transparent)] pub struct Code(pub Inner, pub Inner);",
        "#[serde(transparent)] pub struct Code(#[serde(default)] pub Inner, #[serde(default)] pub u8);",
    ] {
        let walk = issues_walk_of(positional);
        assert!(
            walk.contains(
                "let :: core :: option :: Option :: Some (items) = found . as_array () else"
            ) && walk.contains("items . first ()"),
            "for {positional}, got: {walk}"
        );
    }
}

/// A unit struct is the `{}` tixschema makes it write: every key is `Unknown`, and the fields
/// walker reads none of what it is handed, so it binds none of it.
#[test]
fn a_unit_struct_lists_every_key_and_its_fields_walker_binds_nothing() {
    let emitted = type_impl_of("pub struct Ping;");
    for written in [
        "let :: core :: option :: Option :: Some (object) = found . as_object () else { out . \
         extend (ping_schema :: value_leaf (found , < Self as serde :: Deserialize > :: \
         deserialize ,",
        "for (key , held) in object { out . push (issue (\"Unknown\" , [path , & [:: core :: \
         result :: Result :: Ok (key . clone ())]] . concat () , & [] , :: core :: option :: \
         Option :: Some (held . clone ()) , :: core :: option :: Option :: None , :: std :: vec :: \
         Vec :: new ())) ; }",
        "pub fn decode_with_value_fields < 'a , I > (_ : & 'a serde_json :: Map < :: std :: string \
         :: String , serde_json :: Value > , _ : & [:: core :: result :: Result < :: std :: string :: \
         String , usize >] , _ : ping_schema :: IssueFromParts < serde_json :: Value , I > , _ \
         : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { :: std :: vec :: \
         Vec :: new () }",
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
        "for (key , item) in entries { match item { serde_json :: Value :: Array (items_1) => { \
         match items_1 . first () { :: core :: option :: Option :: Some (held_1) =>",
        "[path , & [:: core :: result :: Result :: Ok (\"by_name\" . to_owned ()) , :: core :: result \
         :: Result :: Ok (key . clone ()) , :: core :: result :: Result :: Err (1)]] . concat ()",
        "match object . get (\"maybe\") { :: core :: option :: Option :: None | :: core :: option :: \
         Option :: Some (serde_json :: Value :: Null) => { } :: core :: option :: Option :: Some \
         (serde_json :: Value :: Array (items)) => { match items . first () {",
        "Some (held) => out . push (issue (\"Invalid\" , [path , & [:: core :: result :: Result :: \
         Ok (\"maybe\" . to_owned ())]] . concat () , & [(\"Optional\" , & [] , 1) , (\"Tuple\" \
         , & [] , 2) , (\"String\" , & [] , 0) , (\"U32\" , & [] , 0)] , :: core :: option :: \
         Option :: Some (held . clone ()) , :: core :: option :: Option :: Some (\"not an array\" \
         . to_owned ()) , :: std :: vec :: Vec :: new ())) ,",
        "placed_schema :: value_leaf (held , < () as serde :: Deserialize > :: deserialize ,",
        "match items . get (1) { :: core :: option :: Option :: Some (held) => < Inner > :: \
         decode_with_value_issues (held , & [path , & [:: core :: result :: Result :: Ok (\"spot\" \
         . to_owned ()) , :: core :: result :: Result :: Err (1)]] . concat () , issue , out) ,",
        "None => out . push (issue (\"Missing\" , [path , & [:: core :: result :: Result :: Ok \
         (\"spot\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"String\" , & \
         [] , 0) , (\"Model\" , & [\"Inner\"] , 0)] , :: core :: option :: Option :: None , :: core \
         :: option :: Option :: None , :: std :: vec :: Vec :: new ())) ,",
        "for (index , item) in items . iter () . enumerate () { match item { serde_json :: Value :: Array (items_1) =>",
        "for (index_1 , held_1) in items_1 . iter () . enumerate () . skip (2) { out . push \
         (issue (\"Unknown\" , [path , & [:: core :: result :: Result :: Ok (\"spots\" . to_owned \
         ()) , :: core :: result :: Result :: Err (index) , :: core :: result :: Result :: Err \
         (index_1)]] . concat () ,",
        "item => out . push (issue (\"Invalid\" , [path , & [:: core :: result :: Result :: Ok \
         (\"spots\" . to_owned ()) , :: core :: result :: Result :: Err (index)]] . concat () , & \
         [(\"Tuple\" , & [] , 2) , (\"String\" , & [] , 0) , (\"U32\" , & [] , 0)] , :: core :: \
         option :: Option :: Some (item . clone ()) , :: core :: option :: Option :: Some (\"not \
         an array\" . to_owned ()) , :: std :: vec :: Vec :: new ())) ,",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// A type with a type parameter gets one `impl` per source. Each joins what that source reads and
/// writes a value with to the bounds the type declares, on every parameter and on the type itself.
/// A parameter is bounded in one place: its `where` predicate where the type wrote one, taking
/// along what the type wrote beside its name, and beside its name otherwise. Under `mongodb` its
/// typed paths get an `impl` of their own, under what a path writes a value with.
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
        methods_of("value"),
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
            methods_of("bson"),
        ));
    }
    if cfg!(feature = "mongodb") {
        headers.push((
            "impl < T : Clone + :: serde :: Serialize , const N : usize , U , V , \
             W : :: serde :: Serialize > Page < T , N , U , V , W > \
             where U : Copy + :: serde :: Serialize , V : Sync + Send + :: serde :: Serialize , \
             Vec < W > : Clone , Self : :: serde :: Serialize"
                .to_owned(),
            paths_members(),
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
        "where F3 : :: core :: ops :: FnOnce (& mut serde_json :: Value , & [keyed_schema :: Issue < serde_json :: Value >]) -> keyed_schema :: Verdict ,",
        "pub fn decode_with_value_issues < I2 > (found : & serde_json :: Value , path : & [:: core \
         :: result :: Result < :: std :: string :: String , usize >] , issue : keyed_schema :: \
         IssueFromParts < serde_json :: Value , I2 > , out : & mut :: std :: vec :: Vec < I2 > ,)",
        "pub fn decode_with_value_fields < 'a , I2 > (",
        "< I as serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option :: None \
         ,",
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
/// Under `mongodb` the function that builds the typed paths names the row type the same way:
/// `Root`, and the next free name where the item writes that.
#[test]
fn a_methods_own_type_parameter_is_never_a_name_the_item_writes() {
    let expected = |named: [&str; 2], root: &str| {
        let mut every: Vec<String> = named.map(str::to_owned).to_vec();
        if cfg!(feature = "mongodb") {
            every.push(root.to_owned());
        }
        every
    };
    for (source, named, root) in [
        ("pub struct Holder { pub inner: Inner }", ["F", "I"], "Root"),
        ("pub struct Holder { pub inner: I }", ["F", "I2"], "Root"),
        ("pub struct Holder { pub inner: r#I }", ["F", "I2"], "Root"),
        (
            "pub struct Holder { pub keyed: HashMap<String, Vec<Option<(I, i32)>>> }",
            ["F", "I2"],
            "Root",
        ),
        ("pub struct I { pub number: i32 }", ["F", "I2"], "Root"),
        (
            "pub struct Holder { #[serde(deserialize_with = \"I::positive\")] pub score: i32 }",
            ["F", "I2"],
            "Root",
        ),
        (
            "pub struct Holder { #[serde(flatten)] pub inner: I }",
            ["F", "I2"],
            "Root",
        ),
        (
            "pub struct Holder { #[serde(deserialize_with = \"parsed::<I, _>\")] pub score: i32 }",
            ["F", "I2"],
            "Root",
        ),
        ("pub struct F(pub String);", ["F2", "I"], "Root"),
        (
            "pub struct Holder { pub frame: F, pub inner: I, pub other: I2 }",
            ["F2", "I3"],
            "Root",
        ),
        ("pub struct Holder { pub top: Root }", ["F", "I"], "Root2"),
        ("pub struct Root { pub name: String }", ["F", "I"], "Root2"),
    ] {
        let item: syn::ItemStruct = syn::parse_str(source).unwrap();
        assert_eq!(
            own_type_parameters(struct_recovering_decode(&item).type_impl),
            expected(named, root),
            "for {source}"
        );
    }
    for (source, named, root) in [
        (EXTERNAL, ["F", "I"], "Root"),
        (
            "pub enum Carried { Alone(I), Named { inner: I }, Paired(I, i32) }",
            ["F", "I2"],
            "Root",
        ),
        (
            "#[serde(untagged)] pub enum Alternate { Model(I), Text(String) }",
            ["F", "I2"],
            "Root",
        ),
        (
            "#[serde(tag = \"kind\")] pub enum Framed { Held(F) }",
            ["F2", "I"],
            "Root",
        ),
        ("pub enum Tree { Leaf, Root(String) }", ["F", "I"], "Root2"),
    ] {
        let item: syn::ItemEnum = syn::parse_str(source).unwrap();
        assert_eq!(
            own_type_parameters(enum_recovering_decode(&item).type_impl),
            expected(named, root),
            "for {source}"
        );
    }
    let walk = issues_walk_of("pub struct Holder(pub String, pub I);");
    for written in [
        " < I2 > (found : & serde_json :: Value , path : & [:: core :: result :: Result < :: std :: \
         string :: String , usize >] , issue : holder_schema :: IssueFromParts < serde_json :: \
         Value , I2 > , out : & mut :: std :: vec :: Vec < I2 > ,)",
        ":: core :: option :: Option :: Some (held) => < I > :: decode_with_value_issues (held , & \
         [path , & [:: core :: result :: Result :: Err (1)]] . concat () , issue , out) ,",
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
        "parcel_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , \
         | _ | :: core :: option :: Option :: None , path . to_vec () , & [(\"Model\" , & \
         [\"Parcel\"] , 0)] , issue)",
        "< (T , u32) as serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option \
         :: None , [path , & [:: core :: result :: Result :: Ok (\"both\" . to_owned ())]] . \
         concat () , & [(\"Tuple\" , & [] , 2) , (\"TypeParam\" , & [\"T\"] , 0) , (\"U32\" , & \
         [] , 0)] ,",
        "< HashMap < String , T > as serde :: Deserialize > :: deserialize , | _ | :: core :: \
         option :: Option :: None , [path , & [:: core :: result :: Result :: Ok (\"extra\" . \
         to_owned ())]] . concat () , & [(\"Map\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , \
         0)] ,",
        "parcel_schema :: value_leaf (held , lenient , | _ : & T | :: core :: option :: Option :: \
         None ,",
        "< Self > :: decode_with_value_issues (item , & [path , & [:: core :: result :: Result :: \
         Ok (\"inner\" . to_owned ()) , :: core :: result :: Result :: Err (index)]] . concat () , \
         issue , out)",
        "< Vec < T > as serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option \
         :: None , [path , & [:: core :: result :: Result :: Ok (\"items\" . to_owned ())]] . \
         concat () , & [(\"Array\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , 0)] ,",
        "< T as serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option :: None \
         , [path , & [:: core :: result :: Result :: Ok (\"one\" . to_owned ())]] . concat () , & \
         [(\"TypeParam\" , & [\"T\"] , 0)] ,",
        "< Page < T > > :: decode_with_value_issues (held , & [path , & [:: core :: result :: \
         Result :: Ok (\"page\" . to_owned ())]] . concat () , issue , out)",
        "< T :: Item as serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option \
         :: None ,",
        "< Option < T > as serde :: Deserialize > :: deserialize , | _ | :: core :: option :: \
         Option :: None , [path , & [:: core :: result :: Result :: Ok (\"sender\" . to_owned \
         ())]] . concat () , & [(\"Optional\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , 0)] ,",
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
            "{ out . extend (code_schema :: bson_leaf (found , < Self as serde :: Deserialize > \
             :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () \
             , path . to_vec () , & [(\"String\" , & [] , 0)] , issue)) ; }",
        ),
        (
            "pub struct Pinned(pub Inner);",
            "-> :: std :: vec :: Vec < & 'a str > { < Inner > :: decode_with_bson_fields (object , \
             path , issue , out) }",
        ),
        (
            "pub struct Ping;",
            "pub fn decode_with_bson_fields < 'a , I > (_ : & 'a bson :: Document , _ : & [:: core \
             :: result :: Result < :: std :: string :: String , usize >] , _ : ping_schema :: \
             IssueFromParts < bson :: Bson , I > , _ : & mut :: std :: vec :: Vec < I > ,) -> :: std \
             :: vec :: Vec < & 'a str > { :: std :: vec :: Vec :: new () }",
        ),
        (
            "pub struct Placed { pub spot: (String, u32) }",
            ":: core :: option :: Option :: Some (bson :: Bson :: Array (items)) => { match items \
             . first () { :: core :: option :: Option :: Some (held) => out . extend \
             (placed_schema :: bson_leaf (held ,",
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

/// Every form serde writes an enum in gets the same methods, in one `impl`, a plain enum
/// included, and an untagged one a method per variant it walks itself beside them.
#[test]
fn every_enum_form_adds_the_same_methods_and_an_untagged_one_a_method_per_variant() {
    let forms: [(&str, &[&str]); 5] = [
        ("pub enum Status { Draft, Published }", &[]),
        (EXTERNAL, &[]),
        (INTERNAL, &[]),
        (ADJACENT, &[]),
        (UNTAGGED, &["email", "word"]),
    ];
    for (source, walked) in forms {
        let of_source = |stem: &str| {
            let mut named = methods_of(stem);
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
        named.extend(paths_members());
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
            "out : & mut :: std :: vec :: Vec < I > ,) { out . extend (status_schema :: value_leaf \
             (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: \
             to_value (read) . ok () , path . to_vec () , & [(\"Model\" , & [\"Status\"] , 0)] \
             , issue)) ; }"
        ),
        "got: {plain}"
    );
    let tagged = enum_json_of("#[serde(tag = \"code\")] pub enum Fault { Db, Io }");
    for written in [
        "let :: core :: option :: Option :: Some (tag) = object . get (\"code\") else {",
        "match tag . as_str () { :: core :: option :: Option :: Some (\"Db\") => vec ! [\"code\"] \
         , :: core :: option :: Option :: Some (\"Io\") => vec ! [\"code\"] , _ => {",
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
        "if let :: core :: option :: Option :: Some (content) = object . get (\"Circle\") { if let \
         serde_json :: Value :: Object (inner) = content { match inner . get (\"radius\") { \
         :: core :: option :: Option :: Some (held) => out . extend (outline_schema :: value_leaf \
         (held , < f64 as serde :: Deserialize > :: deserialize ,",
        ":: core :: option :: Option :: None => out . push (issue (\"Missing\" , [path , & [:: core \
         :: result :: Result :: Ok (\"Circle\" . to_owned ()) , :: core :: result :: Result :: Ok \
         (\"radius\" . to_owned ())]] . concat () , & [(\"F64\" , & [] , 0)] , :: core :: option \
         :: Option :: None , :: core :: option :: Option :: None , :: std :: vec :: Vec :: new ())) , \
         }",
        "for (key , held) in inner { if ! matches ! (key . as_str () , \"radius\") { out . push \
         (issue (\"Unknown\" , [path , & [:: core :: result :: Result :: Ok (\"Circle\" . to_owned \
         ()) , :: core :: result :: Result :: Ok (key . clone ())]] . concat () , & [] , :: core :: \
         option :: Option :: Some (held . clone ()) , :: core :: option :: Option :: None , :: std :: \
         vec :: Vec :: new ())) ; } } } return vec ! [\"Circle\"] ; }",
        "if object . contains_key (\"Empty\") { return vec ! [\"Empty\"] ; }",
        "if let :: core :: option :: Option :: Some (content) = object . get (\"Label\") { out . \
         extend (outline_schema :: value_leaf (content , < String as serde :: Deserialize > :: \
         deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [:: core :: \
         result :: Result :: Ok (\"Label\" . to_owned ())]] . concat () , & [(\"String\" , & [] \
         , 0)] , issue)) ; return vec ! [\"Label\"] ; }",
        "if let :: core :: option :: Option :: Some (content) = object . get (\"Pinned\") { < \
         Inner > :: decode_with_value_issues (content , & [path , & [:: core :: result :: Result \
         :: Ok (\"Pinned\" . to_owned ())]] . concat () , issue , out) ; return vec ! \
         [\"Pinned\"] ; }",
        "if let :: core :: option :: Option :: Some (content) = object . get (\"To\") { match \
         content { serde_json :: Value :: Array (items) => { match items . first () {",
        "for (index , held) in items . iter () . enumerate () . skip (2) { out . push (issue \
         (\"Unknown\" , [path , & [:: core :: result :: Result :: Ok (\"To\" . to_owned ()) , :: core \
         :: result :: Result :: Err (index)]] . concat () ,",
        "content => out . push (issue (\"Invalid\" , [path , & [:: core :: result :: Result :: Ok \
         (\"To\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"I32\" , & [] , \
         0) , (\"I32\" , & [] , 0)] , :: core :: option :: Option :: Some (content . clone ()) , \
         :: core :: option :: Option :: Some (\"not an array\" . to_owned ()) , :: std :: vec :: Vec \
         :: new ())) , } return vec ! [\"To\"] ; }",
        "out . push (issue (\"Missing\" , path . to_vec () , & [(\"Variants\" , & [\"Circle\" , \
         \"Empty\" , \"Label\" , \"Pinned\" , \"To\"] , 0)] , :: core :: option :: Option :: None \
         , :: core :: option :: Option :: None , :: std :: vec :: Vec :: new ())) ; :: std :: vec :: Vec \
         :: new () }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// An internally tagged enum reads its tag from a key of the object and walks the variant's fields
/// in that same object. A variant holding a model type hands that type's fields walker the object
/// without the tag's entry, as serde does, and adds the tag's key to the keys it returns.
#[test]
fn an_internally_tagged_enum_walks_the_variant_its_tag_names_in_the_same_object() {
    let walk = enum_json_of(INTERNAL);
    for written in [
        "let :: core :: option :: Option :: Some (object) = found . as_object () else { out . \
         extend (fill_schema :: value_leaf (found , < Self as serde :: Deserialize > :: \
         deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & \
         [(\"Model\" , & [\"Fill\"] , 0)] , issue)) ; return ; } ; let declared = Self :: \
         decode_with_value_fields (object , path , issue , out) ; for (key , held) in object { \
         if ! declared . contains (& key . as_str ()) {",
        "let :: core :: option :: Option :: Some (tag) = object . get (\"kind\") else { out . push \
         (issue (\"Missing\" , [path , & [:: core :: result :: Result :: Ok (\"kind\" . to_owned \
         ())]] . concat () , & [(\"Variants\" , & [\"Clear\" , \"Solid\" , \"Versioned\"] , 0)] \
         , :: core :: option :: Option :: None , :: core :: option :: Option :: None , :: std :: vec :: \
         Vec :: new ())) ; return object . keys () . map (:: std :: string :: String :: as_str) . \
         collect () ; } ;",
        "match tag . as_str () { :: core :: option :: Option :: Some (\"Clear\") => vec ! \
         [\"kind\"] , :: core :: option :: Option :: Some (\"Solid\") => { match object . get \
         (\"color\") {",
        "None => out . push (issue (\"Missing\" , [path , & [:: core :: result :: Result :: Ok \
         (\"color\" . to_owned ())]] . concat () , & [(\"String\" , & [] , 0)] , :: core :: option \
         :: Option :: None , :: core :: option :: Option :: None , :: std :: vec :: Vec :: new ())) , \
         } vec ! [\"kind\" , \"color\"] } ,",
        ":: core :: option :: Option :: Some (\"Versioned\") => { let rest : serde_json :: Map < \
         :: std :: string :: String , serde_json :: Value > = object . iter () . filter (| (key , \
         _) | ! matches ! (key . as_str () , \"kind\")) . map (| (key , held) | (key . clone () \
         , held . clone ())) . collect () ; let mut declared : :: std :: vec :: Vec < & str > = < \
         Inner > :: decode_with_value_fields (& rest , path , issue , out) . into_iter () . \
         filter_map (| key | object . keys () . find (| own | own . as_str () == key) . map \
         (:: std :: string :: String :: as_str)) . collect () ; declared . push (\"kind\") ; \
         declared } ,",
        "_ => { let here = [path , & [:: core :: result :: Result :: Ok (\"kind\" . to_owned ())]] \
         . concat () ; out . push (match < Self as serde :: Deserialize > :: deserialize \
         (serde_json :: Value :: Object (object . clone ())) { :: core :: result :: Result :: Err \
         (refused) => issue (\"Invalid\" , here , & [(\"Variants\" , & [\"Clear\" , \"Solid\" , \
         \"Versioned\"] , 0)] , :: core :: option :: Option :: Some (tag . clone ()) , :: core :: \
         option :: Option :: Some (refused . to_string ()) , :: std :: vec :: Vec :: new ()) , \
         :: core :: result :: Result :: Ok (_) => issue (\"Mistyped\" , here , & [(\"Variants\" , \
         & [\"Clear\" , \"Solid\" , \"Versioned\"] , 0)] , :: core :: option :: Option :: Some \
         (tag . clone ()) , :: core :: option :: Option :: None , :: std :: vec :: Vec :: new ()) , \
         }) ; object . keys () . map (:: std :: string :: String :: as_str) . collect () } } }",
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
            "match tag . as_str () { :: core :: option :: Option :: Some (\"Lost\") => vec ! \
             [\"kind\"] , :: core :: option :: Option :: Some (\"Sent\") => object . keys () . map \
             (:: std :: string :: String :: as_str) . collect () , _ => {"
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
        "return vec ! [\"kind\" , \"data\"] ; } ; match tag . as_str () { :: core :: option :: \
         Option :: Some (\"Dashed\") => match object . get (\"data\") { :: core :: option :: \
         Option :: Some (content) => if let serde_json :: Value :: Object (inner) = content { \
         match inner . get (\"gap\") {",
        "for (key , held) in inner { if ! matches ! (key . as_str () , \"gap\") { out . push \
         (issue (\"Unknown\" , [path , & [:: core :: result :: Result :: Ok (\"data\" . to_owned \
         ()) , :: core :: result :: Result :: Ok (key . clone ())]] . concat () ,",
        ":: core :: option :: Option :: None => out . push (issue (\"Missing\" , [path , & [:: core \
         :: result :: Result :: Ok (\"data\" . to_owned ())]] . concat () , & [(\"Model\" , & \
         [\"Stroke\"] , 0)] , :: core :: option :: Option :: None , :: core :: option :: Option :: \
         None , :: std :: vec :: Vec :: new ())) , } , :: core :: option :: Option :: Some \
         (\"Hairline\") => { } ,",
        ":: core :: option :: Option :: Some (\"Level\") => match object . get (\"data\") { :: core \
         :: option :: Option :: None | :: core :: option :: Option :: Some (serde_json :: Value :: \
         Null) => { } :: core :: option :: Option :: Some (content) => < Inner > :: \
         decode_with_value_issues (content , & [path , & [:: core :: result :: Result :: Ok \
         (\"data\" . to_owned ())]] . concat () , issue , out) , } ,",
        ":: core :: option :: Option :: Some (\"Span\") => match object . get (\"data\") { :: core :: \
         option :: Option :: Some (serde_json :: Value :: Array (items)) => { match items . \
         first () {",
        ":: core :: option :: Option :: Some (content) => out . push (issue (\"Invalid\" , [path , \
         & [:: core :: result :: Result :: Ok (\"data\" . to_owned ())]] . concat () , & \
         [(\"Tuple\" , & [] , 2) , (\"U32\" , & [] , 0) , (\"U32\" , & [] , 0)] , :: core :: \
         option :: Option :: Some (content . clone ()) , :: core :: option :: Option :: Some \
         (\"not an array\" . to_owned ()) , :: std :: vec :: Vec :: new ())) , :: core :: option :: \
         Option :: None => out . push (issue (\"Missing\" , [path , & [:: core :: result :: Result \
         :: Ok (\"data\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"U32\" , \
         & [] , 0) , (\"U32\" , & [] , 0)] , :: core :: option :: Option :: None , :: core :: option \
         :: Option :: None , :: std :: vec :: Vec :: new ())) , } ,",
        ":: core :: option :: Option :: Some (\"Width\") => match object . get (\"data\") { :: core \
         :: option :: Option :: Some (content) => out . extend (stroke_schema :: value_leaf \
         (content , < u32 as serde :: Deserialize > :: deserialize , | read | serde_json :: \
         to_value (read) . ok () , [path , & [:: core :: result :: Result :: Ok (\"data\" . \
         to_owned ())]] . concat () , & [(\"U32\" , & [] , 0)] , issue)) , :: core :: option :: \
         Option :: None => out . push (issue (\"Missing\" , [path , & [:: core :: result :: Result \
         :: Ok (\"data\" . to_owned ())]] . concat () , & [(\"U32\" , & [] , 0)] , :: core :: \
         option :: Option :: None , :: core :: option :: Option :: None , :: std :: vec :: Vec :: new \
         ())) , } ,",
        ":: core :: result :: Result :: Ok (_) => issue (\"Mistyped\" , here , & [(\"Variants\" , \
         & [\"Dashed\" , \"Hairline\" , \"Level\" , \"Span\" , \"Width\"] , 0)] , :: core :: \
         option :: Option :: Some (tag . clone ()) , :: core :: option :: Option :: None , :: std :: \
         vec :: Vec :: new ()) , }) ; } } vec ! [\"kind\" , \"data\"] }",
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
        "match < Self as serde :: Deserialize > :: deserialize (found) { :: core :: result :: \
         Result :: Ok (Self :: Email { .. }) => Self :: decode_with_value_variant_email (found \
         , path , issue , out) , :: core :: result :: Result :: Ok (Self :: Versioned (..)) => < \
         Inner > :: decode_with_value_issues (found , path , issue , out) , :: core :: result :: \
         Result :: Ok (Self :: Word (..)) => Self :: decode_with_value_variant_word (found , \
         path , issue , out) ,",
        ":: core :: result :: Result :: Err (_) => { let mut as_email = :: std :: vec :: Vec :: new \
         () ; Self :: decode_with_value_variant_email (found , path , issue , & mut as_email) ; \
         let mut as_versioned = :: std :: vec :: Vec :: new () ; < Inner > :: \
         decode_with_value_issues (found , path , issue , & mut as_versioned) ; let mut as_word \
         = :: std :: vec :: Vec :: new () ; Self :: decode_with_value_variant_word (found , path , \
         issue , & mut as_word) ; out . push (issue (\"NoVariant\" , path . to_vec () , & [] , \
         :: core :: option :: Option :: Some (found . clone ()) , :: core :: option :: Option :: None \
         , vec ! [(\"Email\" , as_email) , (\"Versioned\" , as_versioned) , (\"Word\" , \
         as_word)])) ; } } }",
        "fn decode_with_value_variant_email < I > (found : & serde_json :: Value , path : & \
         [:: core :: result :: Result < :: std :: string :: String , usize >] , issue : \
         contact_schema :: IssueFromParts < serde_json :: Value , I > , out : & mut :: std :: vec \
         :: Vec < I > ,) { let :: core :: option :: Option :: Some (object) = found . as_object () \
         else { out . extend (contact_schema :: value_leaf (found , < Self as serde :: \
         Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . \
         to_vec () , & [(\"Model\" , & [\"Contact\"] , 0)] , issue)) ; return ; } ;",
        "match object . get (\"address\") { :: core :: option :: Option :: Some (held) => out . \
         extend (contact_schema :: value_leaf (held , contact_schema :: \
         deserialize_email_address , | read : & String | serde_json :: to_value (read) . ok () \
         , [path , & [:: core :: result :: Result :: Ok (\"address\" . to_owned ())]] . concat () \
         , & [(\"String\" , & [] , 0)] , issue)) ,",
        "for (key , held) in object { if ! matches ! (key . as_str () , \"address\") { out . \
         push (issue (\"Unknown\" , [path , & [:: core :: result :: Result :: Ok (key . clone \
         ())]] . concat () ,",
        "fn decode_with_value_variant_word < I > (found : & serde_json :: Value , path : & \
         [:: core :: result :: Result < :: std :: string :: String , usize >] , issue : \
         contact_schema :: IssueFromParts < serde_json :: Value , I > , out : & mut :: std :: vec \
         :: Vec < I > ,) { out . extend (contact_schema :: value_leaf (found , < String as \
         serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () \
         , path . to_vec () , & [(\"String\" , & [] , 0)] , issue)) ; }",
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
        "if let :: core :: option :: Option :: Some (content) = object . get (\"by_air\") { if let \
         serde_json :: Value :: Object (inner) = content { match inner . get (\"FLIGHT_CODE\") \
         {",
        "[path , & [:: core :: result :: Result :: Ok (\"by_sea\" . to_owned ()) , :: core :: result \
         :: Result :: Ok (\"vesselName\" . to_owned ())]] . concat ()",
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
        "if let :: core :: option :: Option :: Some (held) = items . get (1) { out . extend \
         (loose_schema :: value_leaf (held , < i32 as serde :: Deserialize > :: deserialize ,",
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
        methods_of("value"),
    )];
    if cfg!(feature = "bson") {
        headers.push((
            "impl < T : serde :: de :: DeserializeOwned + serde :: Serialize > Answer < T > \
             where Self : serde :: de :: DeserializeOwned + serde :: Serialize"
                .to_owned(),
            methods_of("bson"),
        ));
    }
    if cfg!(feature = "mongodb") {
        headers.push((
            "impl < T : :: serde :: Serialize > Answer < T > where Self : :: serde :: Serialize"
                .to_owned(),
            paths_members(),
        ));
    }
    assert_eq!(added_enum_impls(source), headers);
    let walk = enum_json_of(source);
    for written in [
        "_ => out . extend (answer_schema :: value_leaf (found , < Self as serde :: Deserialize \
         > :: deserialize , | _ | :: core :: option :: Option :: None , path . to_vec () , & \
         [(\"Variants\" , & [\"Empty\" , \"Value\"] , 0)] , issue)) ,",
        "if let :: core :: option :: Option :: Some (content) = object . get (\"Value\") { out . \
         extend (answer_schema :: value_leaf (content , < T as serde :: Deserialize > :: \
         deserialize , | _ | :: core :: option :: Option :: None , [path , & [:: core :: result :: \
         Result :: Ok (\"Value\" . to_owned ())]] . concat () , & [(\"TypeParam\" , & [\"T\"] , \
         0)] , issue)) ; return vec ! [\"Value\"] ; }",
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
            "if let :: core :: option :: Option :: Some (content) = object . get (\"Circle\") { if \
             let bson :: Bson :: Document (inner) = content { match inner . get (\"radius\") {",
        ),
        (
            EXTERNAL,
            "if let :: core :: option :: Option :: Some (content) = object . get (\"To\") { match \
             content { bson :: Bson :: Array (items) => {",
        ),
        (
            INTERNAL,
            ":: core :: option :: Option :: Some (\"Versioned\") => { let rest : bson :: Document \
             = object . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \
             \"kind\")) . map (| (key , held) | (key . clone () , held . clone ())) . collect \
             () ; let mut declared : :: std :: vec :: Vec < & str > = < Inner > :: \
             decode_with_bson_fields (& rest , path , issue , out) . into_iter () . filter_map \
             (| key | object . keys () . find (| own | own . as_str () == key) . map (:: std :: \
             string :: String :: as_str)) . collect () ; declared . push (\"kind\") ; declared \
             } ,",
        ),
        (
            INTERNAL,
            "out . push (match < Self as serde :: Deserialize > :: deserialize (bson :: \
             Deserializer :: new (bson :: Bson :: Document (object . clone ()))) { :: core :: \
             result :: Result :: Err (refused) => issue (\"Invalid\" , here ,",
        ),
        (
            ADJACENT,
            ":: core :: option :: Option :: Some (\"Level\") => match object . get (\"data\") { \
             :: core :: option :: Option :: None | :: core :: option :: Option :: Some (bson :: Bson \
             :: Null) => { } :: core :: option :: Option :: Some (content) => < Inner > :: \
             decode_with_bson_issues (content ,",
        ),
        (
            UNTAGGED,
            "match < Self as serde :: Deserialize > :: deserialize (bson :: Deserializer :: new \
             (found . clone ())) { :: core :: result :: Result :: Ok (Self :: Email { .. }) => \
             Self :: decode_with_bson_variant_email (found , path , issue , out) , :: core :: \
             result :: Result :: Ok (Self :: Versioned (..)) => < Inner > :: \
             decode_with_bson_issues (found , path , issue , out) ,",
        ),
        (
            UNTAGGED,
            "contact_schema :: bson_leaf (held , contact_schema :: deserialize_email_address , \
             | read : & String , to | serde :: Serialize :: serialize (read , to) . ok () ,",
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
            "match tag . as_str () { :: core :: option :: Option :: Some (\"Clear\" | \"Blank\") \
             => vec ! [\"kind\"] , :: core :: option :: Option :: Some (\"Solid\") => { match \
             object . get (\"color\") {"
        ),
        "got: {internal}"
    );
    let adjacent = enum_json_of(ALIASED_ADJACENT);
    for written in [
        "match tag . as_str () { :: core :: option :: Option :: Some (\"Dashed\" | \"Dotted\" | \
         \"Broken\") => match object . get (\"data\") { :: core :: option :: Option :: Some \
         (content) => if let serde_json :: Value :: Object (inner) = content { match inner . \
         get (\"gap\") {",
        "[path , & [:: core :: result :: Result :: Ok (\"data\" . to_owned ()) , :: core :: result :: \
         Result :: Ok (\"gap\" . to_owned ())]] . concat ()",
        ", :: core :: option :: Option :: Some (\"Hairline\") => { } , _ => { let here = [path , & \
         [:: core :: result :: Result :: Ok (\"kind\" . to_owned ())]] . concat () ;",
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
        "if let :: core :: option :: Option :: Some ((tag , content)) = [\"Circle\" , \"Round\"] . \
         into_iter () . find_map (| tag | object . get (tag) . map (| content | (tag , \
         content))) { if let serde_json :: Value :: Object (inner) = content { match inner . \
         get (\"radius\") {",
        ":: core :: option :: Option :: None => out . push (issue (\"Missing\" , [path , & [:: core \
         :: result :: Result :: Ok (tag . to_owned ()) , :: core :: result :: Result :: Ok \
         (\"radius\" . to_owned ())]] . concat () , & [(\"F64\" , & [] , 0)] , :: core :: option \
         :: Option :: None , :: core :: option :: Option :: None , :: std :: vec :: Vec :: new ())) , \
         }",
        "out . push (issue (\"Unknown\" , [path , & [:: core :: result :: Result :: Ok (tag . \
         to_owned ()) , :: core :: result :: Result :: Ok (key . clone ())]] . concat () , & [] , \
         :: core :: option :: Option :: Some (held . clone ()) , :: core :: option :: Option :: None \
         , :: std :: vec :: Vec :: new ())) ; } } } return vec ! [tag] ; }",
        "if let :: core :: option :: Option :: Some (tag) = [\"Empty\" , \"Blank\"] . into_iter () \
         . find (| & tag | object . contains_key (tag)) { return vec ! [tag] ; }",
        "if let :: core :: option :: Option :: Some ((tag , content)) = [\"Jump\" , \"Hop\" , \
         \"Leap\"] . into_iter () . find_map (| tag | object . get (tag) . map (| content | \
         (tag , content))) { match content { serde_json :: Value :: Array (items) => { match \
         items . first () {",
        "out . push (issue (\"Unknown\" , [path , & [:: core :: result :: Result :: Ok (tag . \
         to_owned ()) , :: core :: result :: Result :: Err (index)]] . concat () ,",
        "content => out . push (issue (\"Invalid\" , [path , & [:: core :: result :: Result :: Ok \
         (tag . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"I32\" , & [] , 0) , \
         (\"I32\" , & [] , 0)] , :: core :: option :: Option :: Some (content . clone ()) , :: core \
         :: option :: Option :: Some (\"not an array\" . to_owned ()) , :: std :: vec :: Vec :: \
         new ())) , } return vec ! [tag] ; }",
        "if let :: core :: option :: Option :: Some (content) = object . get (\"Old\") { out . \
         extend (contour_schema :: value_leaf (content , < String as serde :: Deserialize > :: \
         deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [:: core :: \
         result :: Result :: Ok (\"Old\" . to_owned ())]] . concat () , & [(\"String\" , & [] , \
         0)] , issue)) ; return vec ! [\"Old\"] ; }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    let twice_aliased = enum_json_of(
        "pub enum Trail { #[serde(alias = \"Lane\")] Road { #[serde(alias = \"len\")] length: u32 } }",
    );
    assert!(
        twice_aliased.contains(
            "match [\"length\" , \"len\"] . into_iter () . find_map (| stored | inner . get \
             (stored) . map (| held | (stored , held))) { :: core :: option :: Option :: Some \
             ((stored , held)) => out . extend (trail_schema :: value_leaf (held , < u32 as \
             serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . \
             ok () , [path , & [:: core :: result :: Result :: Ok (tag . to_owned ()) , :: core :: \
             result :: Result :: Ok (stored . to_owned ())]] . concat () ,"
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
        ":: core :: result :: Result :: Ok (Self :: Word (..)) => Self :: \
         decode_with_value_variant_word (found , path , issue , out) , :: core :: result :: Result \
         :: Ok (Self :: Fax (..) | Self :: Pager { .. }) => { } :: core :: result :: Result :: Err \
         (_) => { let mut as_email = :: std :: vec :: Vec :: new () ;",
        "let mut as_word = :: std :: vec :: Vec :: new () ; Self :: decode_with_value_variant_word \
         (found , path , issue , & mut as_word) ; out . push (issue (\"NoVariant\" , path . \
         to_vec () , & [] , :: core :: option :: Option :: Some (found . clone ()) , :: core :: \
         option :: Option :: None , vec ! [(\"Email\" , as_email) , (\"Word\" , as_word)])) ; } \
         } }",
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
        let mut named = methods_of(stem);
        named.extend(
            ["email", "word"].map(|variant| format!("decode_with_{stem}_variant_{variant}")),
        );
        named
    };
    let mut named = of_source("value");
    if cfg!(feature = "bson") {
        named.extend(of_source("bson"));
    }
    named.extend(paths_members());
    assert_eq!(
        added_enum_impls(UNREAD_UNTAGGED),
        [("impl Reach".to_owned(), named)]
    );
    let every_variant_read = enum_json_of(UNTAGGED);
    assert!(
        every_variant_read.contains(
            ":: core :: result :: Result :: Ok (Self :: Word (..)) => Self :: \
             decode_with_value_variant_word (found , path , issue , out) , :: core :: result :: \
             Result :: Err (_) => {"
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
            "if let :: core :: option :: Option :: Some ((tag , content)) = [\"Circle\" , \
             \"Round\"] . into_iter () . find_map (| tag | object . get (tag) . map (| content \
             | (tag , content))) { if let bson :: Bson :: Document (inner) = content { match \
             inner . get (\"radius\") {",
        ),
        (
            ALIASED_EXTERNAL,
            "if let :: core :: option :: Option :: Some (tag) = [\"Empty\" , \"Blank\"] . \
             into_iter () . find (| & tag | object . contains_key (tag)) { return vec ! [tag] ; \
             }",
        ),
        (
            ALIASED_INTERNAL,
            "match tag . as_str () { :: core :: option :: Option :: Some (\"Clear\" | \"Blank\") \
             => vec ! [\"kind\"] , :: core :: option :: Option :: Some (\"Solid\") => {",
        ),
        (
            ALIASED_ADJACENT,
            ":: core :: option :: Option :: Some (\"Dashed\" | \"Dotted\" | \"Broken\") => match \
             object . get (\"data\") { :: core :: option :: Option :: Some (content) => if let \
             bson :: Bson :: Document (inner) = content {",
        ),
        (
            UNREAD_UNTAGGED,
            ":: core :: result :: Result :: Ok (Self :: Word (..)) => Self :: \
             decode_with_bson_variant_word (found , path , issue , out) , :: core :: result :: \
             Result :: Ok (Self :: Fax (..) | Self :: Pager { .. }) => { } :: core :: result :: \
             Result :: Err (_) => {",
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

/// A flattened type's keys sit among the object's own, so its fields walker runs in what the
/// type's own fields and the flattened types declared before it left of that object, at that
/// object's path, and the keys it returns are kept as declared beside the type's own. A
/// flattened `Option` is walked where its type answers that what is left names it: into a list of
/// its own, kept where serde reads the type from what is left and replaced by one `Mistyped` at
/// the object where it does not.
#[test]
fn a_flattened_type_is_walked_in_the_outer_object_and_its_keys_are_declared() {
    let walk = json_fields_walk_of(FLATTENING);
    for written in [
        "-> :: std :: vec :: Vec < & 'a str > { let mut declared = vec ! [\"title\"] ; match \
         object . get (\"title\") { :: core :: option :: Option :: Some (held) =>",
        "let rest : serde_json :: Map < :: std :: string :: String , serde_json :: Value > = \
         object . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"title\")) . \
         map (| (key , held) | (key . clone () , held . clone ())) . collect () ; declared . \
         extend (< Audit > :: decode_with_value_fields (& rest , path , issue , out) . \
         into_iter () . filter_map (| key | object . keys () . find (| own | own . as_str () == \
         key) . map (:: std :: string :: String :: as_str))) ; let taken = entry_schema :: \
         value_remaining :: < Audit , _ > (< Audit as serde :: Deserialize > :: deserialize , & \
         entry_schema :: Asked :: default () , & rest) ; let remaining = taken . as_ref () . \
         unwrap_or (& rest) ; if < Extra > :: decode_with_value_named (remaining) { let mut \
         nested = :: std :: vec :: Vec :: new () ; let keys = < Extra > :: \
         decode_with_value_fields (remaining , path , issue , & mut nested) ; match < Extra as \
         serde :: Deserialize > :: deserialize (serde_json :: Value :: Object (remaining . \
         clone ())) { :: core :: result :: Result :: Ok (_) => out . append (& mut nested) , :: core \
         :: result :: Result :: Err (_) => out . push (issue (\"Mistyped\" , path . to_vec () , \
         & [(\"Optional\" , & [] , 1) , (\"Model\" , & [\"Extra\"] , 0)] , :: core :: option :: \
         Option :: Some (serde_json :: Value :: Object (object . clone ())) , :: core :: option :: \
         Option :: None , :: std :: vec :: Vec :: new ())) , } declared . extend (keys . into_iter \
         () . filter_map (| key | object . keys () . find (| own | own . as_str () == key) . \
         map (:: std :: string :: String :: as_str))) ; } let taken = entry_schema :: \
         value_remaining :: < Option < Extra > , _ > (< Option < Extra > as serde :: \
         Deserialize > :: deserialize , & entry_schema :: Asked :: default () , remaining) ; \
         let remaining = taken . as_ref () . unwrap_or (remaining) ; declared . extend (< Fill \
         > :: decode_with_value_fields (remaining , path , issue , out) . into_iter () . \
         filter_map (| key | object . keys () . find (| own | own . as_str () == key) . map \
         (:: std :: string :: String :: as_str))) ; declared }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    assert!(!walk.contains("_schema :: decode_with"), "got: {walk}");
    let wrapped = json_fields_walk_of(
        "pub struct Boxed { #[serde(flatten)] pub audit: Box<Audit>, \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub extra: Option<Box<Extra>> }",
    );
    for written in [
        "{ let mut declared = :: std :: vec :: Vec :: new () ; declared . extend (< Audit > :: \
         decode_with_value_fields (object , path , issue , out)) ;",
        "let taken = boxed_schema :: value_remaining :: < Audit , _ > (< Audit as serde :: Deserialize > :: deserialize , & boxed_schema :: Asked :: default () , object) ; let remaining = taken . as_ref () . unwrap_or (object) ; if < Extra > :: decode_with_value_named (remaining) {",
        "match < Extra as serde :: Deserialize > :: deserialize (serde_json :: Value :: Object (remaining . clone ())) {",
        "& [(\"Optional\" , & [] , 1) , (\"Model\" , & [\"Extra\"] , 0)]",
    ] {
        assert!(
            wrapped.contains(written),
            "missing `{written}` in: {wrapped}"
        );
    }
}

/// serde hands a flattened type the entries the outer type's own fields did not take. The walk
/// binds a copy of them once, hands it to the first flattened type and to each later one what the
/// earlier ones left of it, and finds each key that comes back among the object's own. A type
/// with no key of its own hands over the object itself, and copies nothing.
#[test]
fn a_flattened_type_is_handed_what_the_types_own_fields_left_of_the_object() {
    let keyed = json_fields_walk_of(
        "pub struct Report { #[serde(flatten)] pub counts: Counts, pub id: String, \
         #[serde(flatten)] pub origin: Origin }",
    );
    assert!(
        keyed.contains(
            ":: std :: vec :: Vec :: new ())) , } let rest : serde_json :: Map < :: std :: string :: \
             String , serde_json :: Value > = object . iter () . filter (| (key , _) | ! \
             matches ! (key . as_str () , \"id\")) . map (| (key , held) | (key . clone () , \
             held . clone ())) . collect () ; declared . extend (< Counts > :: \
             decode_with_value_fields (& rest , path , issue , out) . into_iter () . filter_map \
             (| key | object . keys () . find (| own | own . as_str () == key) . map (:: std :: \
             string :: String :: as_str))) ; let taken = report_schema :: value_remaining :: < \
             Counts , _ > (< Counts as serde :: Deserialize > :: deserialize , & report_schema \
             :: Asked :: default () , & rest) ; let remaining = taken . as_ref () . unwrap_or \
             (& rest) ; declared . extend (< Origin > :: decode_with_value_fields (remaining , \
             path , issue , out) . into_iter () . filter_map (| key | object . keys () . find \
             (| own | own . as_str () == key) . map (:: std :: string :: String :: as_str))) ; \
             declared }"
        ),
        "got: {keyed}"
    );
    assert_eq!(keyed.matches("let rest").count(), 1, "got: {keyed}");

    let unkeyed = json_fields_walk_of("pub struct Only { #[serde(flatten)] pub counts: Counts }");
    assert!(
        unkeyed.contains(
            "-> :: std :: vec :: Vec < & 'a str > { let mut declared = :: std :: vec :: Vec :: new () \
             ; declared . extend (< Counts > :: decode_with_value_fields (object , path , issue \
             , out)) ; declared }"
        ),
        "got: {unkeyed}"
    );
    for absent in ["rest", "filter", "find", "_remaining"] {
        assert!(!unkeyed.contains(absent), "found `{absent}` in: {unkeyed}");
    }
}

/// A flattened `Option` is walked only where its type answers that what it is handed names it,
/// and the keys its walker returns are bound only where something reads them.
#[test]
fn a_flattened_option_is_walked_where_its_type_answers_that_the_object_names_it() {
    let alone = json_fields_walk_of(
        "pub struct Only { \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub extra: Option<Extra> }",
    );
    assert!(
        alone.contains(
            "-> :: std :: vec :: Vec < & 'a str > { let mut declared = :: std :: vec :: Vec :: new () \
             ; if < Extra > :: decode_with_value_named (object) { let mut nested = :: std :: vec \
             :: Vec :: new () ; let keys = < Extra > :: decode_with_value_fields (object , path \
             , issue , & mut nested) ; match < Extra as serde :: Deserialize > :: deserialize \
             (serde_json :: Value :: Object (object . clone ())) { :: core :: result :: Result :: \
             Ok (_) => out . append (& mut nested) , :: core :: result :: Result :: Err (_) => out \
             . push (issue (\"Mistyped\" , path . to_vec () , & [(\"Optional\" , & [] , 1) , \
             (\"Model\" , & [\"Extra\"] , 0)] , :: core :: option :: Option :: Some (serde_json :: \
             Value :: Object (object . clone ())) , :: core :: option :: Option :: None , :: std :: \
             vec :: Vec :: new ())) , } declared . extend (keys) ; } declared }"
        ),
        "got: {alone}"
    );
    let unread = json_fields_walk_of(
        "pub struct Beside { \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub extra: Option<Extra>, \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub more: Option<HashMap<String, i32>>, \
         pub title: String }",
    );
    assert!(
        unread.contains(
            "if < Extra > :: decode_with_value_named (& rest) { let mut nested = :: std :: vec :: \
             Vec :: new () ; < Extra > :: decode_with_value_fields (& rest , path , issue , & \
             mut nested) ; match < Extra as serde :: Deserialize > :: deserialize (serde_json \
             :: Value :: Object (rest . clone ())) {"
        ),
        "got: {unread}"
    );
    assert!(
        !unread.contains("keys =") && !unread.contains("declared"),
        "got: {unread}"
    );
}

/// `decode_with_value_named` as it is emitted, answering `answer`.
fn value_named(answer: &str) -> String {
    format!(
        "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: String , \
         serde_json :: Value >) -> bool {{ {answer} }}"
    )
}

/// Every flagged shape answers whether an object holds what names a value of it. A struct with
/// named fields is named by a key one of them is read under, a name or an alias, and by what
/// names a flagged type it flattens, and by any key where a flattened field takes the rest. A
/// struct serde writes as the value it holds answers as that value flattened would: it asks the
/// flagged type it holds, and any key names a map, a parameter's value, a JSON value or a slot a
/// hook reads. A shape serde does not flatten is named by nothing, so it binds no object.
#[test]
fn every_struct_shape_answers_whether_an_object_names_a_value_of_it() {
    for (source, answered) in [
        (
            "pub struct Titled { #[serde(alias = \"label\")] pub name: String, pub title: String }",
            "object . keys () . any (| key | matches ! (key . as_str () , \"name\" | \"label\" | \"title\"))",
        ),
        (
            "#[serde(tag = \"kind\")] pub struct Tagged { pub name: String }",
            "object . keys () . any (| key | matches ! (key . as_str () , \"name\" | \"kind\"))",
        ),
        (
            FLATTENING,
            "object . keys () . any (| key | matches ! (key . as_str () , \"title\")) || < Audit > :: decode_with_value_named (object) || < Extra > :: decode_with_value_named (object) || < Fill > :: decode_with_value_named (object)",
        ),
        (
            "pub struct Only { #[serde(flatten)] pub audit: Box<Audit> }",
            "< Audit > :: decode_with_value_named (object)",
        ),
        (
            "pub struct Bag { #[serde(flatten)] pub audit: Audit, #[serde(flatten)] pub extra: HashMap<String, i32>, pub title: String }",
            "! object . is_empty ()",
        ),
        (FLATTENING_A_PARAMETER, "! object . is_empty ()"),
        (
            "pub struct Pinned(pub Inner);",
            "< Inner > :: decode_with_value_named (object)",
        ),
        (
            "#[serde(transparent)] pub struct Code { pub inner: Box<Inner> }",
            "< Inner > :: decode_with_value_named (object)",
        ),
        (
            "pub struct Latest(pub Option<Inner>);",
            "< Inner > :: decode_with_value_named (object)",
        ),
        (
            "pub struct Extras(pub HashMap<String, i32>);",
            "! object . is_empty ()",
        ),
        (
            "#[serde(transparent)] pub struct Wrap<T>(pub T);",
            "! object . is_empty ()",
        ),
        (
            "pub struct Anything(pub serde_json::Value);",
            "! object . is_empty ()",
        ),
        (
            "pub struct Stamp(#[serde(deserialize_with = \"lenient\")] pub Inner);",
            "! object . is_empty ()",
        ),
    ] {
        let emitted = type_impl_of(source);
        let named = value_named(answered);
        assert!(
            emitted.contains(&named),
            "for {source}, missing `{named}` in: {emitted}"
        );
    }
    for source in [
        "pub struct Blank {}",
        "pub struct Unwritten { #[serde(skip)] pub cached: u8 }",
        "pub struct Ping;",
        "pub struct Pair(pub String, pub u32);",
        "pub struct Brand(pub String);",
        "pub struct Tags(pub Vec<Inner>);",
        "pub struct Spot(pub (String, u32));",
    ] {
        let emitted = type_impl_of(source);
        assert!(
            emitted.contains(
                "pub fn decode_with_value_named (_ : & serde_json :: Map < :: std :: string :: \
                 String , serde_json :: Value >) -> bool { false }"
            ),
            "for {source}, got: {emitted}"
        );
    }
}

/// An internally or adjacently tagged enum is named by its tag's key, an externally tagged one by
/// a key naming a variant serde reads, under its name or an alias, and an untagged one by serde
/// reading the object as one of its variants. A plain enum is flattened as an externally tagged
/// one is, so it is named as one is. An enum with no variant serde reads is named by nothing.
#[test]
fn every_enum_form_answers_whether_an_object_names_a_value_of_it() {
    for (source, answered) in [
        (
            INTERNAL,
            "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: \
             String , serde_json :: Value >) -> bool { object . contains_key (\"kind\") }",
        ),
        (
            ADJACENT,
            "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: \
             String , serde_json :: Value >) -> bool { object . contains_key (\"kind\") }",
        ),
        (
            EXTERNAL,
            "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: \
             String , serde_json :: Value >) -> bool { object . keys () . any (| key | matches \
             ! (key . as_str () , \"Circle\" | \"Empty\" | \"Label\" | \"Pinned\" | \"To\")) }",
        ),
        (
            ALIASED_EXTERNAL,
            "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: \
             String , serde_json :: Value >) -> bool { object . keys () . any (| key | matches \
             ! (key . as_str () , \"Circle\" | \"Round\" | \"Empty\" | \"Blank\" | \"Jump\" | \
             \"Hop\" | \"Leap\" | \"Old\")) }",
        ),
        (
            UNTAGGED,
            "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: \
             String , serde_json :: Value >) -> bool { < Self as serde :: Deserialize > :: \
             deserialize (serde_json :: Value :: Object (object . clone ())) . is_ok () }",
        ),
        (
            "pub enum Status { Draft, Published }",
            "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: \
             String , serde_json :: Value >) -> bool { object . keys () . any (| key | matches \
             ! (key . as_str () , \"Draft\" | \"Published\")) }",
        ),
        (
            "pub enum Never { #[serde(skip)] Lost { at: u32 } }",
            "pub fn decode_with_value_named (_ : & serde_json :: Map < :: std :: string :: String \
             , serde_json :: Value >) -> bool { false }",
        ),
    ] {
        let emitted = enum_json_of(source);
        assert!(
            emitted.contains(answered),
            "for {source}, missing `{answered}` in: {emitted}"
        );
    }
}

/// The BSON answer of each shape is its JSON one over the library's own types: the document a
/// type's fields are looked up in, and serde reading it through the library's deserializer.
#[cfg(feature = "bson")]
#[test]
fn the_bson_answer_of_each_shape_matches_the_librarys_own_types() {
    for (source, answered) in [
        (
            FLATTENING,
            "pub fn decode_with_bson_named (object : & bson :: Document) -> bool { object . keys () . any (| key | matches ! (key . as_str () , \"title\")) || < Audit > :: decode_with_bson_named (object) || < Extra > :: decode_with_bson_named (object) || < Fill > :: decode_with_bson_named (object) }",
        ),
        (
            "pub struct Ping;",
            "pub fn decode_with_bson_named (_ : & bson :: Document) -> bool { false }",
        ),
    ] {
        let emitted = type_impl_of(source);
        let (_json, bson) = emitted.split_once("pub fn from_bson_with").unwrap();
        assert!(
            bson.contains(answered),
            "for {source}, missing `{answered}` in: {bson}"
        );
        assert!(!bson.contains("serde_json"), "for {source}, got: {bson}");
    }
    for (source, answered) in [
        (
            INTERNAL,
            "pub fn decode_with_bson_named (object : & bson :: Document) -> bool { object . contains_key (\"kind\") }",
        ),
        (
            UNTAGGED,
            "pub fn decode_with_bson_named (object : & bson :: Document) -> bool { < Self as serde :: Deserialize > :: deserialize (bson :: Deserializer :: new (bson :: Bson :: Document (object . clone ()))) . is_ok () }",
        ),
    ] {
        let bson = enum_bson_of(source);
        assert!(
            bson.contains(answered),
            "for {source}, missing `{answered}` in: {bson}"
        );
        assert!(!bson.contains("serde_json"), "for {source}, got: {bson}");
    }
}

/// serde reads the flattened fields in the order declared, each from what the ones before it
/// left. The walk asks the schema module what serde leaves once it has read each earlier one
/// through that one's own reader, and hands the later type that, which is the same object where
/// serde took nothing. Nothing is asked for one flattened type alone, after a flattened map,
/// which takes nothing, or after a field serde never reads.
#[test]
fn a_later_flattened_type_is_handed_what_the_earlier_ones_left() {
    let unkeyed = json_fields_walk_of(
        "pub struct Two { #[serde(flatten)] pub audit: Audit, #[serde(flatten)] pub counts: Counts }",
    );
    assert!(
        unkeyed.contains(
            "declared . extend (< Audit > :: decode_with_value_fields (object , path , issue , \
             out)) ; let taken = two_schema :: value_remaining :: < Audit , _ > (< Audit as \
             serde :: Deserialize > :: deserialize , & two_schema :: Asked :: default () , \
             object) ; let remaining = taken . as_ref () . unwrap_or (object) ; declared . \
             extend (< Counts > :: decode_with_value_fields (remaining , path , issue , out) . \
             into_iter () . filter_map (| key | object . keys () . find (| own | own . as_str \
             () == key) . map (:: std :: string :: String :: as_str))) ; declared }"
        ),
        "got: {unkeyed}"
    );
    let parameter = json_fields_walk_of(
        "pub struct Parcel<T> { #[serde(flatten)] pub body: T, \
         #[serde(flatten)] pub counts: Counts, pub id: String }",
    );
    assert!(
        parameter.contains(
            "let taken = parcel_schema :: value_remaining :: < T , _ > (< T as serde :: Deserialize > :: deserialize , & parcel_schema :: Asked :: default () , & rest) ; let remaining = taken . as_ref () . unwrap_or (& rest) ; declared . extend (< Counts > :: decode_with_value_fields (remaining , path , issue , out)"
        ),
        "got: {parameter}"
    );
    let hooked = json_fields_walk_of(
        "pub struct Hooked { #[serde(flatten, with = \"as_text\")] pub first: HashMap<String, i32>, \
         #[serde(flatten)] pub counts: Counts }",
    );
    assert!(
        hooked.contains(
            "let taken = hooked_schema :: value_remaining :: < HashMap < String , i32 > , _ > \
             (as_text :: deserialize , & hooked_schema :: Asked :: default () , object) ; let \
             remaining = taken . as_ref () . unwrap_or (object) ; declared . extend (< Counts > \
             :: decode_with_value_fields (remaining , path , issue , out)"
        ),
        "got: {hooked}"
    );
    for source in [
        "pub struct One { #[serde(flatten)] pub counts: Counts, pub id: String }",
        "pub struct After { #[serde(flatten)] pub extra: HashMap<String, i32>, \
         #[serde(flatten)] pub counts: Counts, pub id: String }",
        "pub struct Unread { #[serde(flatten, skip_deserializing)] pub cached: Audit, \
         #[serde(flatten)] pub counts: Counts }",
    ] {
        let walk = json_fields_walk_of(source);
        assert!(!walk.contains("_remaining"), "for {source}, got: {walk}");
    }
}

/// The field that reads the keys nothing else declares reads none serde takes for a field no walk
/// reaches that is declared before it: such a field declares no key, so what serde leaves once
/// it has read it is asked for.
#[test]
fn the_rest_is_read_without_what_serde_takes_for_an_earlier_field_no_walk_reaches() {
    let walk = json_fields_walk_of(
        "pub struct Bundle<T> { \
         #[serde(flatten, skip_serializing_if = \"Option::is_none\")] pub body: Option<T>, \
         #[serde(flatten)] pub counts: HashMap<String, i32>, pub id: String }",
    );
    assert!(
        walk.contains(
            "let taken = bundle_schema :: value_remaining :: < Option < T > , _ > (< Option < T > as serde :: Deserialize > :: deserialize , & bundle_schema :: Asked :: default () , object) ; let remaining = taken . as_ref () . unwrap_or (object) ; for (key , item) in remaining { if ! matches ! (key . as_str () , \"id\") {"
        ),
        "got: {walk}"
    );
}

/// The schema module answers what remains of an object once serde has read a type flattened
/// there, and whether a reader reads an `Option`, through a deserializer that reads nothing and
/// notes what the reader asks for in a value its caller owns, so a question allocates nothing.
/// Where serde takes nothing the first answers `None`, and nothing is copied.
#[test]
fn the_schema_module_answers_what_serde_leaves_and_copies_nothing_where_it_takes_nothing() {
    let items = module_items().to_string();
    for written in [
        "pub struct Asked { # [doc = r\" It asks for an `Option`, which serde reads as absent where what it holds is refused.\"] optional : :: core :: cell :: Cell < bool > , taken : :: core :: cell :: Cell < Taken > , }",
        "pub struct TakenProbe < 'asked > (& 'asked Asked) ;",
        "impl < 'de > serde :: Deserializer < 'de > for TakenProbe < '_ > {",
        "self . 0 . taken . set (Taken :: Fields (fields)) ;",
        "self . 0 . taken . set (Taken :: Variant (variants)) ;",
        "self . 0 . optional . set (true) ; visitor . visit_some (self)",
        "visitor . visit_newtype_struct (self)",
        "pub fn reads_an_option < 'asked , T , R > (read : R , asked : & 'asked Asked) -> bool where R : :: core :: ops :: FnOnce (TakenProbe < 'asked >) -> :: core :: result :: Result < T , serde :: de :: value :: Error > , { let _refused = read (TakenProbe (asked)) ; asked . optional . get () }",
        "pub fn value_remaining < 'asked , T , R > (read : R , asked : & 'asked Asked , entries \
         : & serde_json :: Map < :: std :: string :: String , serde_json :: Value > ,) -> :: core :: \
         option :: Option < serde_json :: Map < :: std :: string :: String , serde_json :: Value \
         >> where R : :: core :: ops :: FnOnce (TakenProbe < 'asked >) -> :: core :: result :: Result < T , serde :: \
         de :: value :: Error > , { let taken = taken_keys (read , asked , entries . keys ()) ; \
         if taken . is_empty () { return :: core :: option :: Option :: None ; }",
    ] {
        assert!(items.contains(written), "missing `{written}` in: {items}");
    }
    assert_eq!(
        items.contains(
            "pub fn bson_remaining < 'asked , T , R > (read : R , asked : & 'asked Asked , \
             entries : & bson :: Document ,) -> :: core :: option :: Option < bson :: Document >"
        ),
        cfg!(feature = "bson"),
        "got: {items}"
    );
    for absent in ["Rc", "_left"] {
        assert!(!items.contains(absent), "found `{absent}` in: {items}");
    }
}

/// A single-slot struct serde flattens gets the fields walker of the value it holds, the one the
/// type that flattens that value itself would run: each entry of a map walked at its key, and a
/// parameter's value, a JSON value and a slot a hook reads each read whole, with every key its
/// own. An `Option` of a map, of a parameter's value, under a hook or of another `Option` is one
/// serde reads as absent where it does not read it: every key is its own, and nothing is listed.
#[test]
fn a_single_slot_struct_serde_flattens_walks_what_it_holds_as_a_flattened_field_of_it_is_walked() {
    for (source, walked) in [
        (
            "pub struct Extras(pub HashMap<String, Inner>);",
            "-> :: std :: vec :: Vec < & 'a str > { for (key , item) in object { < Inner > :: \
             decode_with_value_issues (item , & [path , & [:: core :: result :: Result :: Ok (key \
             . clone ())]] . concat () , issue , out) ; } object . keys () . map (:: std :: string \
             :: String :: as_str) . collect () }",
        ),
        (
            "#[serde(transparent)] pub struct Wrap<T>(pub T);",
            "-> :: std :: vec :: Vec < & 'a str > { let found = & serde_json :: Value :: Object \
             (object . clone ()) ; out . extend (wrap_schema :: value_leaf (found , < Self as \
             serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option :: None , \
             path . to_vec () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue) . filter (| _ | ! \
             wrap_schema :: reads_an_option :: < Self , _ > (< Self as serde :: Deserialize > \
             :: deserialize , & wrap_schema :: Asked :: default ()))) ; object . keys () . map \
             (:: std :: string :: String :: as_str) . collect () }",
        ),
        (
            "pub struct Anything(pub serde_json::Value);",
            "-> :: std :: vec :: Vec < & 'a str > { let found = & serde_json :: Value :: Object \
             (object . clone ()) ; out . extend (anything_schema :: value_leaf (found , < Self \
             as serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option :: None \
             , path . to_vec () , & [(\"Unknown\" , & [] , 0)] , issue)) ; object . keys () . \
             map (:: std :: string :: String :: as_str) . collect () }",
        ),
        (
            "pub struct Maybe(pub Option<HashMap<String, i32>>);",
            ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { object \
             . keys () . map (:: std :: string :: String :: as_str) . collect () }",
        ),
        (
            "#[serde(transparent)] pub struct Maybe<T>(pub Option<T>);",
            ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { object \
             . keys () . map (:: std :: string :: String :: as_str) . collect () }",
        ),
        (
            "pub struct Stamp(#[serde(deserialize_with = \"lenient\")] pub Inner);",
            "-> :: std :: vec :: Vec < & 'a str > { let found = & serde_json :: Value :: Object \
             (object . clone ()) ; out . extend (stamp_schema :: value_leaf (found , < Self as \
             serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option :: None , \
             path . to_vec () , & [(\"Model\" , & [\"Inner\"] , 0)] , issue) . filter (| _ | ! \
             stamp_schema :: reads_an_option :: < Self , _ > (< Self as serde :: Deserialize > \
             :: deserialize , & stamp_schema :: Asked :: default ()))) ; object . keys () . map \
             (:: std :: string :: String :: as_str) . collect () }",
        ),
        (
            "#[serde(transparent)] pub struct Stamp { #[serde(with = \"as_text\")] pub inner: HashMap<String, i32> }",
            "-> :: std :: vec :: Vec < & 'a str > { let found = & serde_json :: Value :: Object \
             (object . clone ()) ; out . extend (stamp_schema :: value_leaf (found , < Self as \
             serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option :: None , \
             path . to_vec () , & [(\"Map\" , & [] , 1) , (\"I32\" , & [] , 0)] , issue) . \
             filter (| _ | ! stamp_schema :: reads_an_option :: < Self , _ > (< Self as serde \
             :: Deserialize > :: deserialize , & stamp_schema :: Asked :: default ()))) ; \
             object . keys () . map (:: std :: string :: String :: as_str) . collect () }",
        ),
        (
            "pub struct Maybe(#[serde(deserialize_with = \"lenient\")] pub Option<Inner>);",
            ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { object \
             . keys () . map (:: std :: string :: String :: as_str) . collect () }",
        ),
        (
            "pub struct Deep(pub Option<Option<Inner>>);",
            ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { object \
             . keys () . map (:: std :: string :: String :: as_str) . collect () }",
        ),
    ] {
        let walk = json_fields_walk_of(source);
        assert!(
            walk.contains(walked),
            "for {source}, missing `{walked}` in: {walk}"
        );
    }
}

/// An id is the object serde writes for it, so serde flattens a single-slot struct over one. No
/// walk reads an id held as that object: every key is the struct's own, and nothing is listed.
#[cfg(feature = "mongodb")]
#[test]
fn a_single_slot_struct_over_an_id_takes_every_key_and_lists_nothing() {
    for source in [
        "pub struct Marker(pub ObjectId);",
        "#[serde(transparent)] pub struct Marker { pub inner: Option<ObjectId> }",
    ] {
        let emitted = type_impl_of(source);
        for written in [
            "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: \
             String , serde_json :: Value >) -> bool { ! object . is_empty () }",
            ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { object \
             . keys () . map (:: std :: string :: String :: as_str) . collect () }",
        ] {
            assert!(
                emitted.contains(written),
                "for {source}, missing `{written}` in: {emitted}"
            );
        }
    }
}

/// A plain enum is flattened as an externally tagged one is: its fields walker is that one's over
/// variants that hold nothing, so the key naming a variant is its own, and `Missing` is listed
/// where no key names one. With no variant serde reads, no key is its own.
#[test]
fn a_plain_enum_gets_the_fields_walker_of_an_externally_tagged_enum() {
    let walk = enum_json_of(
        "pub enum Status { #[serde(alias = \"Rough\")] Draft, #[serde(skip)] Lost, Published }",
    );
    assert!(
        walk.contains(
            "-> :: std :: vec :: Vec < & 'a str > { if let :: core :: option :: Option :: Some (tag) \
             = [\"Draft\" , \"Rough\"] . into_iter () . find (| & tag | object . contains_key \
             (tag)) { return vec ! [tag] ; } if object . contains_key (\"Published\") { return \
             vec ! [\"Published\"] ; } out . push (issue (\"Missing\" , path . to_vec () , & \
             [(\"Variants\" , & [\"Draft\" , \"Rough\" , \"Published\"] , 0)] , :: core :: option \
             :: Option :: None , :: core :: option :: Option :: None , :: std :: vec :: Vec :: new \
             ())) ; :: std :: vec :: Vec :: new () }"
        ),
        "got: {walk}"
    );
    let unread = enum_json_of("pub enum Never { #[serde(skip)] Lost }");
    for written in [
        "pub fn decode_with_value_named (_ : & serde_json :: Map < :: std :: string :: String , \
         serde_json :: Value >) -> bool { false }",
        ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { :: std :: vec \
         :: Vec :: new () }",
    ] {
        assert!(unread.contains(written), "missing `{written}` in: {unread}");
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
            "declared . extend (< Audit > :: decode_with_value_fields (& rest , path , issue , \
             out) . into_iter () . filter_map (| key | object . keys () . find (| own | own . \
             as_str () == key) . map (:: std :: string :: String :: as_str))) ; for (key , item) \
             in object { if ! declared . contains (& key . as_str ()) { < Inner > :: \
             decode_with_value_issues (item , & [path , & [:: core :: result :: Result :: Ok (key \
             . clone ())]] . concat () , issue , out) ; } } object . keys () . map (:: std :: \
             string :: String :: as_str) . collect () }"
        ),
        "got: {beside_a_type}"
    );
    let beside_a_key = json_fields_walk_of(
        "pub struct Bag { #[serde(flatten)] pub extra: HashMap<String, i32>, pub title: String }",
    );
    assert!(
        beside_a_key.contains(
            "for (key , item) in object { if ! matches ! (key . as_str () , \"title\") { out . \
             extend (bag_schema :: value_leaf (item , < i32 as serde :: Deserialize > :: \
             deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [:: core :: \
             result :: Result :: Ok (key . clone ())]] . concat () , & [(\"I32\" , & [] , 0)] , \
             issue)) ; } } object . keys () . map (:: std :: string :: String :: as_str) . collect \
             () }"
        ),
        "got: {beside_a_key}"
    );
    assert!(!beside_a_key.contains("declared"), "got: {beside_a_key}");
    let alone = json_fields_walk_of(
        "pub struct Open { #[serde(flatten)] pub extra: HashMap<String, i32> }",
    );
    assert!(
        alone.contains(
            "-> :: std :: vec :: Vec < & 'a str > { for (key , item) in object { out . extend \
             (open_schema :: value_leaf (item , < i32 as serde :: Deserialize > :: deserialize \
             ,"
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
            "out . extend (envelope_schema :: value_leaf (& serde_json :: Value :: Object \
             (object . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"id\")) \
             . map (| (key , held) | (key . clone () , held . clone ())) . collect ()) , < T as \
             serde :: Deserialize > :: deserialize , | _ | :: core :: option :: Option :: None , \
             path . to_vec () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue) . filter (| _ | ! \
             envelope_schema :: reads_an_option :: < T , _ > (< T as serde :: Deserialize > :: \
             deserialize , & envelope_schema :: Asked :: default ()))) ; object . keys () . map \
             (:: std :: string :: String :: as_str) . collect () }"
        ),
        "got: {walk}"
    );
    let alone = json_fields_walk_of("pub struct Only<T> { #[serde(flatten)] pub body: T }");
    assert!(
        alone.contains(
            "-> :: std :: vec :: Vec < & 'a str > { out . extend (only_schema :: value_leaf (& \
             serde_json :: Value :: Object (object . clone ()) , < T as serde :: Deserialize > \
             :: deserialize , | _ | :: core :: option :: Option :: None , path . to_vec () , & \
             [(\"TypeParam\" , & [\"T\"] , 0)] , issue) . filter (| _ | ! only_schema :: \
             reads_an_option :: < T , _ > (< T as serde :: Deserialize > :: deserialize , & \
             only_schema :: Asked :: default ()))) ; object . keys () . map (:: std :: string :: \
             String :: as_str) . collect () }"
        ),
        "got: {alone}"
    );
    let hooked = json_fields_walk_of(
        "pub struct Hooked { #[serde(flatten)] pub audit: Audit, \
         #[serde(flatten, with = \"as_text\")] pub rest: HashMap<String, i32> }",
    );
    assert!(
        hooked.contains(
            "out . extend (hooked_schema :: value_leaf (& serde_json :: Value :: Object (object \
             . iter () . filter (| (key , _) | ! declared . contains (& key . as_str ())) . map \
             (| (key , held) | (key . clone () , held . clone ())) . collect ()) , as_text :: \
             deserialize , | _ : & HashMap < String , i32 > | :: core :: option :: Option :: None \
             , path . to_vec () , & [(\"Map\" , & [] , 1) , (\"I32\" , & [] , 0)] , issue) . \
             filter (| _ | ! hooked_schema :: reads_an_option :: < HashMap < String , i32 > , _ \
             > (as_text :: deserialize , & hooked_schema :: Asked :: default ()))) ;"
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
            ":: std :: vec :: Vec :: new ())) , } out . extend (packet_schema :: value_leaf (& \
             serde_json :: Value :: Object (object . iter () . filter (| (key , _) | ! matches \
             ! (key . as_str () , \"id\")) . map (| (key , held) | (key . clone () , held . \
             clone ())) . collect ()) , < T as serde :: Deserialize > :: deserialize , | _ | \
             :: core :: option :: Option :: None , path . to_vec () , & [(\"TypeParam\" , & \
             [\"T\"] , 0)] , issue) . filter (| _ | ! packet_schema :: reads_an_option :: < T , \
             _ > (< T as serde :: Deserialize > :: deserialize , & packet_schema :: Asked :: \
             default ()))) ; object . keys () . map (:: std :: string :: String :: as_str) . \
             collect () }"
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
/// refuse, one it refuses outright, or one it never reads, written always or where a
/// `skip_serializing_if` lets it be: every key of the object counts as the field's own and nothing
/// is listed for it, so serde's verdict is the read's. A flattened type beside it is still
/// walked, and its keys are kept nowhere, nothing being left to read them.
#[test]
fn a_flattened_field_no_walk_reaches_takes_every_key_and_lists_nothing() {
    for source in [
        "pub struct Loose { #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub rest: Option<HashMap<String, i32>> }",
        "pub struct Loose<T> { #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub rest: Option<T> }",
        "pub struct Loose { #[serde(flatten)] pub rest: Vec<i32> }",
        "pub struct Loose { #[serde(flatten, skip_deserializing)] pub rest: Audit }",
        "pub struct Loose { #[serde(flatten, skip_deserializing, skip_serializing_if = \"Option::is_none\")] pub rest: Option<Audit> }",
    ] {
        let walk = json_fields_walk_of(source);
        assert!(
            walk.contains(
                ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { \
                 object . keys () . map (:: std :: string :: String :: as_str) . collect () }"
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
            ", out : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { < \
             Audit > :: decode_with_value_fields (object , path , issue , out) ; object . keys \
             () . map (:: std :: string :: String :: as_str) . collect () }"
        ),
        "got: {beside_a_type}"
    );
    // serde neither writes nor reads a flattened field under `skip`, or under `skip_serializing`
    // beside `skip_deserializing`, so no key is its own.
    for source in [
        "pub struct Kept { #[serde(flatten, skip)] pub cached: Audit, pub title: String }",
        "pub struct Kept { #[serde(flatten, skip_serializing, skip_deserializing)] pub cached: Audit, pub title: String }",
    ] {
        let off_the_wire = json_fields_walk_of(source);
        assert!(
            off_the_wire.contains("vec ! [\"title\"] }") && !off_the_wire.contains("Audit"),
            "for {source}, got: {off_the_wire}"
        );
    }
}

/// A flattened id is the object serde writes for one, which no walk reads: every key counts as
/// the field's own and nothing is listed for it, from either source. Behind a hook it is read
/// whole, through the hook.
#[cfg(feature = "mongodb")]
#[test]
fn a_flattened_id_is_read_by_no_walk() {
    for source in [
        "pub struct Keyed { #[serde(flatten)] pub oid: ObjectId }",
        "pub struct Keyed { #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub oid: Option<ObjectId> }",
    ] {
        let emitted = type_impl_of(source);
        assert_eq!(
            emitted
                .matches(
                    ", _ : & mut :: std :: vec :: Vec < I > ,) -> :: std :: vec :: Vec < & 'a str > { \
                     object . keys () . map (:: std :: string :: String :: as_str) . collect () }"
                )
                .count(),
            2,
            "for {source}, got: {emitted}"
        );
    }
    let hooked = json_fields_walk_of(
        "pub struct Keyed { #[serde(flatten, with = \"as_hex\")] pub oid: ObjectId }",
    );
    assert!(
        hooked.contains(
            "out . extend (keyed_schema :: value_leaf (& serde_json :: Value :: Object (object \
             . clone ()) , as_hex :: deserialize , | _ : & ObjectId | :: core :: option :: Option \
             :: None ,"
        ),
        "got: {hooked}"
    );
}

/// A value read whole from the entries a flattened field is read from is written back from
/// neither source: flattened, it is held as those entries, whatever form it has under a key.
/// Behind a hook or in what fills a parameter an `Option` is not seen here, so the reader is asked,
/// and the issue is kept only where it reads none. A JSON value's type is seen, and nothing is
/// asked.
#[test]
fn a_flattened_value_read_whole_is_not_written_back_and_an_option_not_seen_is_absent() {
    let hooked = type_impl_of(
        "pub struct Hooked { #[serde(flatten, default, deserialize_with = \"lenient\")] \
         pub version: Option<Inner>, pub id: String }",
    );
    assert!(
        hooked.contains(
            "lenient , | _ : & Option < Inner > | :: core :: option :: Option :: None , path . \
             to_vec () , & [(\"Optional\" , & [] , 1) , (\"Model\" , & [\"Inner\"] , 0)] , \
             issue) . filter (| _ | ! hooked_schema :: reads_an_option :: < Option < Inner > , \
             _ > (lenient , & hooked_schema :: Asked :: default ()))) ;"
        ),
        "got: {hooked}"
    );
    assert_eq!(
        hooked.contains(
            "lenient , | _ : & Option < Inner > , _ | :: core :: option :: Option :: None , path . \
             to_vec () ,"
        ),
        cfg!(feature = "bson"),
        "got: {hooked}"
    );
    let open = type_impl_of(
        "pub struct Open { pub id: String, #[serde(flatten)] pub rest: serde_json::Value }",
    );
    assert!(
        open.contains(
            "< serde_json :: Value as serde :: Deserialize > :: deserialize , | _ | :: core :: \
             option :: Option :: None , path . to_vec () , & [(\"Unknown\" , & [] , 0)] , \
             issue)) ;"
        ),
        "got: {open}"
    );
    assert!(!open.contains("reads_an_option"), "got: {open}");
    for emitted in [hooked, open] {
        assert!(!emitted.contains("_left"), "got: {emitted}");
    }
}

/// A variant's flattened field is walked in what the variant's own fields, and the tag an
/// internally tagged enum reads there, left of the object those fields sit in, at that object's
/// path, wherever the enum's form puts it. The keys it returns are the variant's own beside the
/// ones the variant declares, so a key outside both is `Unknown`.
#[test]
fn a_variants_flattened_field_is_walked_in_the_object_its_fields_sit_in() {
    let external = enum_json_of(
        "pub enum Outline { Gone, Made { #[serde(flatten)] audit: Audit, title: String } }",
    );
    for written in [
        "if let :: core :: option :: Option :: Some (content) = object . get (\"Made\") { if let \
         serde_json :: Value :: Object (inner) = content { let mut declared = vec ! [\"title\"] \
         ; match inner . get (\"title\") {",
        "let rest : serde_json :: Map < :: std :: string :: String , serde_json :: Value > = inner \
         . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"title\")) . map (| \
         (key , held) | (key . clone () , held . clone ())) . collect () ; declared . extend (< \
         Audit > :: decode_with_value_fields (& rest , & [path , & [:: core :: result :: Result :: \
         Ok (\"Made\" . to_owned ())]] . concat () , issue , out) . into_iter () . filter_map \
         (| key | inner . keys () . find (| own | own . as_str () == key) . map (:: std :: string \
         :: String :: as_str))) ; for (key , held) in inner { if ! declared . contains (& key . \
         as_str ()) { out . push (issue (\"Unknown\" , [path , & [:: core :: result :: Result :: \
         Ok (\"Made\" . to_owned ()) , :: core :: result :: Result :: Ok (key . clone ())]] . \
         concat () , & [] , :: core :: option :: Option :: Some (held . clone ()) , :: core :: option \
         :: Option :: None , :: std :: vec :: Vec :: new ())) ; } } } return vec ! [\"Made\"] ; }",
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
        ":: core :: option :: Option :: Some (\"Made\") => { let mut declared = vec ! [\"kind\" , \
         \"title\"] ; match object . get (\"title\") {",
        "let rest : serde_json :: Map < :: std :: string :: String , serde_json :: Value > = \
         object . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"kind\" | \
         \"title\")) . map (| (key , held) | (key . clone () , held . clone ())) . collect () ; \
         declared . extend (< Audit > :: decode_with_value_fields (& rest , path , issue , out) \
         . into_iter () . filter_map (| key | object . keys () . find (| own | own . as_str () \
         == key) . map (:: std :: string :: String :: as_str))) ; declared } ,",
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
        ":: core :: option :: Option :: Some (\"Made\") => match object . get (\"data\") { :: core :: \
         option :: Option :: Some (content) => if let serde_json :: Value :: Object (inner) = \
         content { let mut declared = vec ! [\"title\"] ;",
        "let rest : serde_json :: Map < :: std :: string :: String , serde_json :: Value > = inner \
         . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"title\")) . map (| \
         (key , held) | (key . clone () , held . clone ())) . collect () ; declared . extend (< \
         Audit > :: decode_with_value_fields (& rest , & [path , & [:: core :: result :: Result :: \
         Ok (\"data\" . to_owned ())]] . concat () , issue , out) . into_iter () . filter_map \
         (| key | inner . keys () . find (| own | own . as_str () == key) . map (:: std :: string \
         :: String :: as_str))) ; for (key , held) in inner { if ! declared . contains (& key . \
         as_str ()) { out . push (issue (\"Unknown\" , [path , & [:: core :: result :: Result :: \
         Ok (\"data\" . to_owned ()) , :: core :: result :: Result :: Ok (key . clone ())]] . \
         concat () ,",
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
            "let rest : serde_json :: Map < :: std :: string :: String , serde_json :: Value > = \
             object . iter () . filter (| (key , _) | ! matches ! (key . as_str () , \
             \"title\")) . map (| (key , held) | (key . clone () , held . clone ())) . collect \
             () ; declared . extend (< Audit > :: decode_with_value_fields (& rest , path , \
             issue , out) . into_iter () . filter_map (| key | object . keys () . find (| own | \
             own . as_str () == key) . map (:: std :: string :: String :: as_str))) ; for (key , \
             held) in object { if ! declared . contains (& key . as_str ()) { out . push (issue \
             (\"Unknown\" , [path , & [:: core :: result :: Result :: Ok (key . clone ())]] . \
             concat () ,"
        ),
        "got: {untagged}"
    );
    // A variant that flattens a map leaves no key of its object undeclared.
    let open = enum_json_of(
        "pub enum Outline { Gone, Made { #[serde(flatten)] extra: HashMap<String, i32>, title: String } }",
    );
    assert!(
        open.contains(
            "for (key , item) in inner { if ! matches ! (key . as_str () , \"title\") { out . \
             extend (outline_schema :: value_leaf (item , < i32 as serde :: Deserialize > :: \
             deserialize , | read | serde_json :: to_value (read) . ok () , [path , & [:: core :: \
             result :: Result :: Ok (\"Made\" . to_owned ()) , :: core :: result :: Result :: Ok \
             (key . clone ())]] . concat () , & [(\"I32\" , & [] , 0)] , issue)) ; } } } return \
             vec ! [\"Made\"] ; }"
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
        "-> :: std :: vec :: Vec < & 'a str > { match < Self as serde :: Deserialize > :: \
         deserialize (serde_json :: Value :: Object (object . clone ())) { :: core :: result :: \
         Result :: Ok (Self :: Email { .. }) => { match object . get (\"address\") { :: core :: \
         option :: Option :: Some (held) => out . extend (contact_schema :: value_leaf (held , \
         contact_schema :: deserialize_email_address ,",
        "vec ! [\"address\"] } , :: core :: result :: Result :: Ok (Self :: Versioned (..)) => < \
         Inner > :: decode_with_value_fields (object , path , issue , out) , :: core :: result :: \
         Result :: Ok (Self :: Word (..)) => object . keys () . map (:: std :: string :: String :: \
         as_str) . collect () , :: core :: result :: Result :: Err (_) => { let found = & \
         serde_json :: Value :: Object (object . clone ()) ; let mut as_email = :: std :: vec :: \
         Vec :: new () ; { let out = & mut as_email ; match object . get (\"address\") {",
        ":: std :: vec :: Vec :: new ())) , } } let mut as_versioned = :: std :: vec :: Vec :: new () \
         ; < Inner > :: decode_with_value_fields (object , path , issue , & mut as_versioned) ; \
         let mut as_word = :: std :: vec :: Vec :: new () ; Self :: decode_with_value_variant_word \
         (found , path , issue , & mut as_word) ; out . push (issue (\"NoVariant\" , path . \
         to_vec () , & [] , :: core :: option :: Option :: Some (found . clone ()) , :: core :: \
         option :: Option :: None , vec ! [(\"Email\" , as_email) , (\"Versioned\" , \
         as_versioned) , (\"Word\" , as_word)])) ; object . keys () . map (:: std :: string :: \
         String :: as_str) . collect () } } } fn decode_with_value_variant_email < I > (",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    let unread = enum_json_of(UNREAD_UNTAGGED);
    assert!(
        unread.contains(
            ":: core :: result :: Result :: Ok (Self :: Word (..)) => object . keys () . map (:: std \
             :: string :: String :: as_str) . collect () , :: core :: result :: Result :: Ok (Self \
             :: Fax (..) | Self :: Pager { .. }) => :: std :: vec :: Vec :: new () , :: core :: \
             result :: Result :: Err (_) => { let found = & serde_json :: Value :: Object \
             (object . clone ()) ;"
        ),
        "got: {unread}"
    );
    // A variant with no field to walk lists nothing, so its list is bound and never written to.
    let blank = enum_json_of("#[serde(untagged)] pub enum Sparse { Blank {}, Word(String) }");
    for written in [
        ":: core :: result :: Result :: Ok (Self :: Blank { .. }) => { vec ! [] } ,",
        "let as_blank = :: std :: vec :: Vec :: new () ; let mut as_word = :: std :: vec :: Vec :: \
         new () ;",
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
        "let rest : bson :: Document = object . iter () . filter (| (key , _) | ! matches ! \
         (key . as_str () , \"title\")) . map (| (key , held) | (key . clone () , held . clone \
         ())) . collect () ; declared . extend (< Audit > :: decode_with_bson_fields (& rest , \
         path , issue , out) . into_iter () . filter_map (| key | object . keys () . find (| \
         own | own . as_str () == key) . map (:: std :: string :: String :: as_str))) ; let taken \
         = entry_schema :: bson_remaining :: < Audit , _ > (< Audit as serde :: Deserialize > \
         :: deserialize , & entry_schema :: Asked :: default () , & rest) ; let remaining = \
         taken . as_ref () . unwrap_or (& rest) ; if < Extra > :: decode_with_bson_named \
         (remaining) { let mut nested = :: std :: vec :: Vec :: new () ; let keys = < Extra > :: \
         decode_with_bson_fields (remaining , path , issue , & mut nested) ;",
        "match < Extra as serde :: Deserialize > :: deserialize (bson :: Deserializer :: new \
         (bson :: Bson :: Document (remaining . clone ()))) { :: core :: result :: Result :: Ok \
         (_) => out . append (& mut nested) , :: core :: result :: Result :: Err (_) => out . push \
         (issue (\"Mistyped\" , path . to_vec () , & [(\"Optional\" , & [] , 1) , (\"Model\" , \
         & [\"Extra\"] , 0)] , :: core :: option :: Option :: Some (bson :: Bson :: Document \
         (object . clone ())) , :: core :: option :: Option :: None , :: std :: vec :: Vec :: new \
         ())) , }",
    ] {
        assert!(
            flattening.contains(written),
            "missing `{written}` in: {flattening}"
        );
    }
    let parameter = bson_fields_walk_of(FLATTENING_A_PARAMETER);
    assert!(
        parameter.contains(
            "out . extend (envelope_schema :: bson_leaf (& bson :: Bson :: Document (object . \
             iter () . filter (| (key , _) | ! matches ! (key . as_str () , \"id\")) . map (| \
             (key , held) | (key . clone () , held . clone ())) . collect ()) , < T as serde :: \
             Deserialize > :: deserialize , | _ , _ | :: core :: option :: Option :: None , path . \
             to_vec () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue) . filter (| _ | ! \
             envelope_schema :: reads_an_option :: < T , _ > (< T as serde :: Deserialize > :: \
             deserialize , & envelope_schema :: Asked :: default ()))) ;"
        ),
        "got: {parameter}"
    );
    let untagged = enum_bson_of(UNTAGGED);
    for written in [
        "-> :: std :: vec :: Vec < & 'a str > { match < Self as serde :: Deserialize > :: \
         deserialize (bson :: Deserializer :: new (bson :: Bson :: Document (object . clone \
         ()))) { :: core :: result :: Result :: Ok (Self :: Email { .. }) =>",
        ":: core :: result :: Result :: Err (_) => { let found = & bson :: Bson :: Document \
         (object . clone ()) ; let mut as_email = :: std :: vec :: Vec :: new () ;",
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

/// A type no `#[model_schema]` was written on above is asked with `ReadWhole` in scope, beside
/// the call that keeps the trait in use, and the trait is added to the module only then. A model
/// type seen above is called as it stands, so one with no flag still fails the build.
#[test]
fn a_field_of_a_type_not_seen_above_is_asked_with_the_whole_read_in_scope() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Page { pub properties: Properties }").unwrap();
    let unseen = super::struct_recovering_decode(&item);
    let (walker, module) = (
        unseen.type_impl.to_string(),
        unseen.schema_module.to_string(),
    );
    assert!(
        walker.contains(
            "{ use page_schema :: ReadWhole as _ ; < () > :: decode_with_read_whole () ; < \
             Properties > :: decode_with_value_issues (held ,"
        ),
        "got: {walker}"
    );
    assert!(
        module.contains("pub trait ReadWhole : serde :: de :: DeserializeOwned"),
        "got: {module}"
    );
    assert!(
        module.contains("impl < T : serde :: de :: DeserializeOwned > ReadWhole for T { }"),
        "got: {module}"
    );

    let seen = struct_recovering_decode(&item);
    let (strict, plain) = (seen.type_impl.to_string(), seen.schema_module.to_string());
    assert!(
        strict.contains("=> < Properties > :: decode_with_value_issues (held ,"),
        "got: {strict}"
    );
    assert!(!strict.contains("ReadWhole"), "got: {strict}");
    assert!(!plain.contains("ReadWhole"), "got: {plain}");
}

/// An alias seen above is written out as the type it names: a standard type by its full path, and
/// every other type under an alias that reaches it from the field's own type and keeps its name.
/// A check beside the type holds the two to one type.
#[test]
fn a_field_typed_with_an_alias_seen_above_is_walked_as_the_type_the_alias_names() {
    record_declared(
        "Marks",
        Declared::Alias("BTreeMap < Tier , Vec < Mark > >".to_owned()),
    );
    record_declared("Mark", Declared::Model);
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Tally { pub marks: types::Marks }").unwrap();
    let added = super::struct_recovering_decode(&item);
    let (walker, module) = (added.type_impl.to_string(), added.schema_module.to_string());
    for written in [
        "const _ : fn (types :: Marks) -> :: std :: collections :: BTreeMap < tally_schema :: \
         decode_with_at0 :: Tier , :: std :: vec :: Vec < tally_schema :: decode_with_at1 :: Mark > \
         > = | held | held ;",
        "< tally_schema :: decode_with_at1 :: Mark > :: decode_with_value_issues (",
    ] {
        assert!(walker.contains(written), "missing `{written}` in: {walker}");
    }
    assert!(!walker.contains("ReadWhole"), "got: {walker}");
    for written in [
        "pub trait EntryOf { type Key ; type Value ; }",
        "pub mod decode_with_at0 { pub type Tier = << super :: super :: types :: Marks as :: core \
         :: iter :: IntoIterator > :: Item as super :: EntryOf > :: Key ; }",
        "pub mod decode_with_at1 { pub type Mark = < << super :: super :: types :: Marks as \
         :: core :: iter :: IntoIterator > :: Item as super :: EntryOf > :: Value as :: core :: iter \
         :: IntoIterator > :: Item ; }",
    ] {
        assert!(module.contains(written), "missing `{written}` in: {module}");
    }
}

/// An alias of one model type is that type: the field is left as it is written, and the type's
/// own walker is called under the alias.
#[test]
fn a_field_typed_with_an_alias_of_a_model_type_is_left_as_it_is_written() {
    record_declared("Featured", Declared::Alias("Entry".to_owned()));
    record_declared("Entry", Declared::Model);
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Shelf { pub featured: Featured }").unwrap();
    let added = super::struct_recovering_decode(&item);
    let (walker, module) = (added.type_impl.to_string(), added.schema_module.to_string());
    assert!(
        walker.contains("=> < Featured > :: decode_with_value_issues (held ,"),
        "got: {walker}"
    );
    assert!(!walker.contains("const _"), "got: {walker}");
    assert!(!walker.contains("ReadWhole"), "got: {walker}");
    assert!(!module.contains("decode_with_at0"), "got: {module}");
}

/// What the typed paths add for the struct `source` declares, with every space taken out: what
/// goes into its module, then what goes into its `impl`. Each name of `seen` is a struct or an
/// enum `#[model_schema]` was written on above it, as the struct's own name is by then.
#[cfg(feature = "mongodb")]
fn typed_paths_of(source: &str, seen: &[&str]) -> (String, String) {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    record_declared(&item.ident.to_string(), Declared::Model);
    for name in seen {
        record_declared(name, Declared::Model);
    }
    let paths = super::fields::struct_paths(&item, &written_names(item.to_token_stream()));
    (
        paths.module_items.to_string().split_whitespace().collect(),
        paths.type_items.to_string().split_whitespace().collect(),
    )
}

/// [`typed_paths_of`] for the enum `source` declares.
#[cfg(feature = "mongodb")]
fn typed_enum_paths_of(source: &str, seen: &[&str]) -> (String, String) {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    record_declared(&item.ident.to_string(), Declared::Model);
    for name in seen {
        record_declared(name, Declared::Model);
    }
    let paths = super::fields::enum_paths(&item, &written_names(item.to_token_stream()));
    (
        paths.module_items.to_string().split_whitespace().collect(),
        paths.type_items.to_string().split_whitespace().collect(),
    )
}

/// A field is a member of the kind serde writes its value as: a value every row holds, one a row
/// may leave out, a list, and each of the three over a struct or an enum declared above, which
/// then holds that type's own struct of paths. A wrapper serde writes as the value it holds
/// changes the type an operator takes and nothing of the kind.
#[cfg(feature = "mongodb")]
#[test]
fn a_field_is_the_member_of_the_kind_serde_writes_it_as() {
    let nested = "super::inner_schema::MongoFields<Root>";
    for (field, member) in [
        ("String", "self::Field<Root,String>".to_owned()),
        ("Option<u8>", "self::OptionalField<Root,u8>".to_owned()),
        ("Vec<u8>", "self::ListField<Root,u8>".to_owned()),
        ("[u8; 4]", "self::ListField<Root,u8>".to_owned()),
        ("HashSet<u8>", "self::ListField<Root,u8>".to_owned()),
        ("Inner", format!("self::Model<Root,super::Inner,{nested}>")),
        ("Option<Inner>", format!("self::OptionalModel<Root,super::Inner,{nested}>")),
        ("Vec<Inner>", format!("self::ModelList<Root,super::Inner,{nested}>")),
        ("Box<Inner>", format!("self::Model<Root,Box<super::Inner>,{nested}>")),
        (
            "Option<Box<Inner>>",
            format!("self::OptionalModel<Root,Box<super::Inner>,{nested}>"),
        ),
        (
            "Wrapper<Inner>",
            "self::Model<Root,super::Wrapper<super::Inner>,super::wrapper_schema::MongoFields<Root,super::Inner>>"
                .to_owned(),
        ),
        ("Option<Vec<u8>>", "self::OptionalField<Root,Vec<u8>>".to_owned()),
        ("Vec<Vec<Inner>>", "self::ListField<Root,Vec<super::Inner>>".to_owned()),
        ("(u8, Inner)", "self::Field<Root,(u8,super::Inner)>".to_owned()),
    ] {
        let source = format!("pub struct Row {{ pub a: {field} }}");
        let (module, _on_the_type) = typed_paths_of(&source, &["Inner", "Wrapper"]);
        assert!(
            module.contains(&format!("{{puba:{member}}}")),
            "for {field}, got: {module}"
        );
    }
    let (_module, on_the_type) = typed_paths_of("pub struct Row { pub a: Inner }", &["Inner"]);
    assert!(
        on_the_type.contains(
            "row_schema::MongoFields{a:row_schema::Model::plain(row_schema::MongoPath::under(prefix,\"a\"),\
             <Inner>::mongo_fields_under(row_schema::MongoPath::under(prefix,\"a\").segments))}"
        ),
        "got: {on_the_type}"
    );
}

/// A type tixschema has not seen as a flagged model where the field is expanded is one whole
/// value: one declared below or in another crate, the type itself under either of its names, and
/// one a path the module cannot read the last name of leads to. The module writes a name in scope
/// beside the type through `super`, and a longer path as its author wrote it.
#[cfg(feature = "mongodb")]
#[test]
fn a_type_not_seen_above_is_one_whole_value_named_through_super() {
    for (field, member) in [
        ("Unseen", "self::Field<Root,super::Unseen>"),
        ("Option<Unseen>", "self::OptionalField<Root,super::Unseen>"),
        ("Vec<Self>", "self::ListField<Root,super::Row>"),
        (
            "Option<Box<Row>>",
            "self::OptionalField<Root,Box<super::Row>>",
        ),
        ("other::Thing", "self::Field<Root,other::Thing>"),
        (
            "self::other::Thing",
            "self::Field<Root,super::other::Thing>",
        ),
        ("super::Thing", "self::Field<Root,super::super::Thing>"),
        ("crate::Thing", "self::Field<Root,crate::Thing>"),
        (
            "::std::string::String",
            "self::Field<Root,::std::string::String>",
        ),
        (
            "DateTime<Utc>",
            "self::Field<Root,super::DateTime<super::Utc>>",
        ),
        (
            "Cow<'static, str>",
            "self::Field<Root,super::Cow<'static,str>>",
        ),
    ] {
        let source = format!("pub struct Row {{ pub a: {field} }}");
        let (module, on_the_type) = typed_paths_of(&source, &[]);
        assert!(
            module.contains(&format!("{{puba:{member}}}")),
            "for {field}, got: {module}"
        );
        assert!(
            !on_the_type.contains(">::mongo_fields_under("),
            "for {field}, got: {on_the_type}"
        );
    }
}

/// A key serde never writes has no member, and neither has a map, whose keys are data: bare, in
/// an `Option`, or flattened. A flattened value that is no flagged model has no key of its own to
/// be one whole value under.
#[cfg(feature = "mongodb")]
#[test]
fn a_key_serde_never_writes_and_a_map_have_no_member() {
    for field in [
        "#[serde(skip)] pub a: u8",
        "#[serde(skip_serializing)] pub a: u8",
        "pub a: HashMap<String, Inner>",
        "pub a: BTreeMap<String, u8>",
        "pub a: Option<HashMap<String, u8>>",
        "#[serde(flatten)] pub a: HashMap<String, u8>",
        "#[serde(flatten)] pub a: Unseen",
        "#[serde(flatten, with = \"hook\")] pub a: Inner",
    ] {
        let source = format!("pub struct Row {{ {field}, pub b: u8 }}");
        let (module, on_the_type) = typed_paths_of(&source, &["Inner"]);
        assert!(
            module.contains("pubstructMongoFields<Root>{pubb:self::Field<Root,u8>}"),
            "for {field}, got: {module}"
        );
        assert!(
            on_the_type.contains("row_schema::MongoFields{b:row_schema::Field::plain("),
            "for {field}, got: {on_the_type}"
        );
    }
}

/// A flagged type is asked for its own paths only where its walker is called as it stands. A
/// value a hook writes is handed to that hook by a function added to the type, under an `Option`
/// as `Some`, and is one whole value; so is a value serde never reads back, and one a hook reads.
#[cfg(feature = "mongodb")]
#[test]
fn a_hooked_field_and_one_serde_never_reads_back_are_one_whole_value() {
    for (field, member, built) in [
        (
            "#[serde(skip_deserializing)] pub a: Inner",
            "self::Field<Root,super::Inner>",
            "row_schema::Field::plain(",
        ),
        (
            "#[serde(deserialize_with = \"read\")] pub a: Option<Inner>",
            "self::OptionalField<Root,super::Inner>",
            "row_schema::OptionalField::plain(",
        ),
        (
            "#[serde(with = \"hook\")] pub a: Inner",
            "self::Field<Root,super::Inner>",
            "row_schema::Field::hooked(row_schema::MongoPath::under(prefix,\"a\"),Self::mongo_write_a)",
        ),
        (
            "#[serde(serialize_with = \"shown\")] pub a: Vec<u8>",
            "self::Field<Root,Vec<u8>>",
            "row_schema::Field::hooked(row_schema::MongoPath::under(prefix,\"a\"),Self::mongo_write_a)",
        ),
        (
            "#[serde(with = \"hook\")] pub a: Option<u8>",
            "self::OptionalField<Root,u8>",
            "row_schema::OptionalField::hooked(row_schema::MongoPath::under(prefix,\"a\"),Self::mongo_write_a)",
        ),
    ] {
        let source = format!("pub struct Row {{ {field} }}");
        let (module, on_the_type) = typed_paths_of(&source, &["Inner"]);
        assert!(
            module.contains(&format!("{{puba:{member}}}")),
            "for {field}, got: {module}"
        );
        assert!(
            on_the_type.contains(built),
            "for {field}, got: {on_the_type}"
        );
    }
    let (_module, bare) = typed_paths_of(
        "pub struct Row { #[serde(with = \"hook\")] pub a: u8 }",
        &[],
    );
    assert!(
        bare.contains(
            "fnmongo_write_a(value:u8)->::core::result::Result<bson::Bson,row_schema::WriteError>\
             {hook::serialize(&value,bson::Serializer::new())}"
        ),
        "got: {bare}"
    );
    let (_declared, optional) = typed_paths_of(
        "pub struct Row { #[serde(serialize_with = \"shown\")] pub a: Option<u8> }",
        &[],
    );
    assert!(
        optional.contains(
            "fnmongo_write_a(value:u8)->::core::result::Result<bson::Bson,row_schema::WriteError>\
             {shown(&::core::option::Option::Some(value),bson::Serializer::new())}"
        ),
        "got: {optional}"
    );
}

/// A member is under its field's own name and writes the key serde does: the field's `rename`,
/// or its name cased by the struct's `rename_all`. A flattened model has no key of its own, so
/// its struct of paths is built under the keys that lead to the struct that flattens it.
#[cfg(feature = "mongodb")]
#[test]
fn a_member_is_named_after_its_field_and_writes_the_key_serde_does() {
    let (module, on_the_type) = typed_paths_of(
        "#[serde(rename_all = \"camelCase\")] pub struct Row { #[serde(rename = \"_id\")] pub id: u8, \
         pub paid_at: u8, pub r#type: u8, #[serde(flatten)] pub audit: Inner, \
         #[serde(flatten)] pub extra: Option<Inner> }",
        &["Inner"],
    );
    assert!(
        module.contains(
            "{pubid:self::Field<Root,u8>,pubpaid_at:self::Field<Root,u8>,pubr#type:self::Field<Root,u8>,\
             pubaudit:super::inner_schema::MongoFields<Root>,\
             pubextra:super::inner_schema::MongoFields<Root>}"
        ),
        "got: {module}"
    );
    for built in [
        "id:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"_id\"))",
        "paid_at:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"paidAt\"))",
        "r#type:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"type\"))",
        "audit:<Inner>::mongo_fields_under(prefix)",
        "extra:<Inner>::mongo_fields_under(prefix)",
    ] {
        assert!(
            on_the_type.contains(built),
            "missing `{built}` in: {on_the_type}"
        );
    }
}

/// A struct serde writes as one value, or as nothing, is the path of that value: its struct of
/// paths holds nothing else and dereferences to it. The path is the struct's last parameter, with
/// no default, and the type's own `impl` writes it: the module names no type of its author's.
/// Only a value serde writes as text is matched by a pattern, on an `impl` over any such path.
#[cfg(feature = "mongodb")]
#[test]
fn a_type_serde_writes_as_one_value_is_the_path_of_that_value() {
    for (source, declared, arguments, text) in [
        (
            "#[serde(transparent)] pub struct Row(pub String);",
            "pubstructMongoFields<Root,Whole>{",
            "",
            true,
        ),
        (
            "pub struct Row(pub u32);",
            "pubstructMongoFields<Root,Whole>{",
            "",
            false,
        ),
        (
            "#[serde(transparent)] pub struct Row { pub held: String, #[serde(skip)] pub cached: u8 }",
            "pubstructMongoFields<Root,Whole>{",
            "",
            true,
        ),
        (
            "pub struct Row(#[serde(with = \"hook\")] pub String);",
            "pubstructMongoFields<Root,Whole>{",
            "",
            false,
        ),
        (
            "pub struct Row;",
            "pubstructMongoFields<Root,Whole>{",
            "",
            false,
        ),
        (
            "pub struct Row<T: Clone>(pub T);",
            "pubstructMongoFields<Root,T:Clone,Whole>{",
            ",T",
            false,
        ),
        (
            "pub struct Row<Whole>(pub Whole);",
            "pubstructMongoFields<Root,Whole,Whole2>{",
            ",Whole",
            false,
        ),
    ] {
        let (module, on_the_type) = typed_paths_of(source, &[]);
        assert!(module.contains(declared), "for {source}, got: {module}");
        assert!(
            module.contains("typeTarget=Whole") || module.contains("typeTarget=Whole2"),
            "for {source}, got: {module}"
        );
        // The type's name is written once, in what the struct is told as.
        assert!(
            !module.contains("super::") && module.matches("Row").count() == 1,
            "for {source}, got: {module}"
        );
        assert_eq!(
            module.contains("pubfnregex("),
            text,
            "for {source}, got: {module}"
        );
        for written in [
            format!(
                "pubconstMONGO_FIELDS:row_schema::MongoFields<Self{arguments},row_schema::Field<Self,Self>>="
            ),
            format!(
                "->row_schema::MongoFields<Root{arguments},row_schema::Field<Root,Self>>\
                 {{row_schema::MongoFields::built(row_schema::Field::plain(row_schema::MongoPath::at(prefix)))}}"
            ),
        ] {
            assert!(
                on_the_type.contains(&written),
                "for {source}, missing `{written}` in: {on_the_type}"
            );
        }
    }
    let (brand, _brand) = typed_paths_of("#[serde(transparent)] pub struct Row(pub String);", &[]);
    assert!(
        brand.contains("impl<Root,Value>MongoFields<Root,self::Field<Root,Value>>{"),
        "got: {brand}"
    );
    let (generic, _generic) = typed_paths_of(
        "#[serde(transparent)] pub struct Row<Value> { pub held: String, pub kept: PhantomData<Value> }",
        &[],
    );
    assert!(
        generic
            .contains("impl<Root,Value,Value2>MongoFields<Root,Value,self::Field<Root,Value2>>{"),
        "got: {generic}"
    );
    let (plain_enum, on_the_enum) = typed_enum_paths_of("pub enum Row { Draft, Paid }", &[]);
    assert!(
        plain_enum.contains("pubstructMongoFields<Root,Whole>{") && !plain_enum.contains("super::"),
        "got: {plain_enum}"
    );
    assert!(
        on_the_enum.contains(
            "pubconstMONGO_FIELDS:row_schema::MongoFields<Self,row_schema::Field<Self,Self>>="
        ),
        "got: {on_the_enum}"
    );
}

/// A type declared above that serde writes as one value is held with its path written out, since
/// its struct of paths names no type: bare and under an `Option` it keeps its own struct. A list
/// of one is a list of plain values, whose elements an operator is written over with no key. A
/// flattened one has no key to sit at the level of what holds it, and no member.
#[cfg(feature = "mongodb")]
#[test]
fn a_type_written_as_one_value_is_held_with_its_path_written_out() {
    record_declared("Brand", Declared::OneValue);
    record_declared("Sleeve", Declared::OneValue);
    let own =
        "super::brand_schema::MongoFields<Root,super::brand_schema::Field<Root,super::Brand>>";
    for (field, member) in [
        ("Brand", format!("self::Model<Root,super::Brand,{own}>")),
        (
            "Option<Brand>",
            format!("self::OptionalModel<Root,super::Brand,{own}>"),
        ),
        (
            "Box<Brand>",
            format!("self::Model<Root,Box<super::Brand>,{own}>"),
        ),
        ("Vec<Brand>", "self::ListField<Root,super::Brand>".to_owned()),
        ("[Brand; 2]", "self::ListField<Root,super::Brand>".to_owned()),
        (
            "Vec<Box<Brand>>",
            "self::ListField<Root,Box<super::Brand>>".to_owned(),
        ),
        (
            "Option<Vec<Brand>>",
            "self::OptionalField<Root,Vec<super::Brand>>".to_owned(),
        ),
        (
            "Sleeve<Inner>",
            "self::Model<Root,super::Sleeve<super::Inner>,super::sleeve_schema::MongoFields<Root,super::Inner,\
             super::sleeve_schema::Field<Root,super::Sleeve<super::Inner>>>>"
                .to_owned(),
        ),
        ("Vec<Inner>", "self::ModelList<Root,super::Inner,super::inner_schema::MongoFields<Root>>".to_owned()),
    ] {
        let source = format!("pub struct Row {{ pub a: {field} }}");
        let (module, _on_the_type) = typed_paths_of(&source, &["Inner"]);
        assert!(
            module.contains(&format!("{{puba:{member}}}")),
            "for {field}, got: {module}"
        );
    }
    let (_module, listed) = typed_paths_of("pub struct Row { pub a: Vec<Brand> }", &[]);
    assert!(
        listed.contains(
            "row_schema::MongoFields{a:row_schema::ListField::plain(row_schema::MongoPath::under(prefix,\"a\"))}"
        ),
        "got: {listed}"
    );
    let (flattened, _flattened) = typed_paths_of(
        "pub struct Row { #[serde(flatten)] pub a: Brand, pub b: u8 }",
        &[],
    );
    assert!(
        flattened.contains("pubstructMongoFields<Root>{pubb:self::Field<Root,u8>}"),
        "got: {flattened}"
    );
    let (_untagged, in_place) = typed_enum_paths_of(
        "#[serde(untagged)] pub enum Row { Held(Brand), Listed(Vec<Brand>) }",
        &[],
    );
    for built in [
        "held:row_schema::Model::plain(row_schema::MongoPath::at(prefix),<Brand>::mongo_fields_under(prefix))",
        "listed:row_schema::ListField::plain(row_schema::MongoPath::at(prefix))",
    ] {
        assert!(in_place.contains(built), "missing `{built}` in: {in_place}");
    }
    let (tagged, beside) = typed_enum_paths_of(
        "#[serde(tag = \"kind\")] pub enum Row { Wire(Brand), Cash }",
        &[],
    );
    let emitted = format!("{tagged}{beside}");
    assert!(!emitted.contains("wire:"), "got: {emitted}");
}

/// A path is under the name serde writes. A renaming written as a list counts by its `serialize`
/// side whatever it names for reading, and one that writes no `serialize` leaves the name as it
/// was: on a field, on a variant, and as the rule that cases either.
#[cfg(feature = "mongodb")]
#[test]
fn a_renaming_written_as_a_list_counts_by_what_serde_writes() {
    let (_module, on_the_struct) = typed_paths_of(
        "#[serde(rename_all(serialize = \"camelCase\"))] pub struct Row { \
         #[serde(rename(serialize = \"written\", deserialize = \"read\"))] pub both_ways: u8, \
         #[serde(rename(serialize = \"only_written\"))] pub write_way: u8, \
         #[serde(rename(deserialize = \"only_read\"))] pub read_way: u8, \
         #[serde(rename(serialize = \"apart\"))] #[serde(rename(deserialize = \"from\"))] pub two_lists: u8, \
         pub cased_by_rule: u8 }",
        &[],
    );
    for built in [
        "both_ways:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"written\"))",
        "write_way:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"only_written\"))",
        "read_way:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"readWay\"))",
        "two_lists:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"apart\"))",
        "cased_by_rule:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"casedByRule\"))",
    ] {
        assert!(
            on_the_struct.contains(built),
            "missing `{built}` in: {on_the_struct}"
        );
    }
    let (read_rule, on_the_read_rule) = typed_paths_of(
        "#[serde(rename_all(deserialize = \"camelCase\"))] pub struct Row { pub paid_at: u8 }",
        &[],
    );
    assert!(
        on_the_read_rule.contains("row_schema::MongoPath::under(prefix,\"paid_at\")"),
        "got: {read_rule}{on_the_read_rule}"
    );
    let (tagged, on_the_tagged) = typed_enum_paths_of(
        "#[serde(tag = \"kind\", rename_all(serialize = \"kebab-case\"), \
         rename_all_fields(serialize = \"camelCase\"))] pub enum Row { \
         #[serde(rename(serialize = \"byAir\", deserialize = \"air\"))] Air { flight_code: u8 }, \
         OverLand { road_name: u8 }, \
         #[serde(rename_all(serialize = \"SCREAMING_SNAKE_CASE\"))] Sea { ship_name: u8 } }",
        &[],
    );
    let by_tag = format!("{tagged}{on_the_tagged}");
    for written in [
        "pubfnis_air(&self)->self::Filter<Root>{self.tagged(\"byAir\")}",
        "pubfnis_over_land(&self)->self::Filter<Root>{self.tagged(\"over-land\")}",
        "pubfnis_sea(&self)->self::Filter<Root>{self.tagged(\"sea\")}",
        "flight_code:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"flightCode\"))",
        "road_name:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"roadName\"))",
        "ship_name:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"SHIP_NAME\"))",
    ] {
        assert!(by_tag.contains(written), "missing `{written}` in: {by_tag}");
    }
    let (keyed, on_the_keyed) = typed_enum_paths_of(
        "#[serde(rename_all(serialize = \"kebab-case\"))] pub enum Row { \
         #[serde(rename(serialize = \"byHand\"))] Courier { badge: u8 }, DropBox(u8), NotSent }",
        &[],
    );
    let by_key = format!("{keyed}{on_the_keyed}");
    for written in [
        "pubfnis_courier(&self)->self::Filter<Root>{self.keyed(\"byHand\")}",
        "row_schema::MongoPath::under(row_schema::MongoPath::under(prefix,\"byHand\").segments,\"badge\")",
        "drop_box:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"drop-box\"))",
        "bson::Bson::String(\"not-sent\".to_owned())",
    ] {
        assert!(by_key.contains(written), "missing `{written}` in: {by_key}");
    }
}

/// What serde writes as one value is what the registry records as one, at the seam every item's
/// declaration is recorded at: a struct written as the value of one slot, a unit struct, and an
/// enum no variant of which holds a value and no attribute tags.
#[test]
fn a_struct_or_an_enum_is_written_as_one_value_or_under_keys_of_its_own() {
    for (source, one_value) in [
        ("#[serde(transparent)] pub struct Row(pub String);", true),
        ("pub struct Row(pub u32);", true),
        ("pub struct Row;", true),
        (
            "#[serde(transparent)] pub struct Row { pub held: String, #[serde(skip)] pub cached: u8 }",
            true,
        ),
        ("pub enum Row { Draft, Paid }", true),
        ("pub struct Row { pub held: String }", false),
        ("pub struct Row(pub u32, pub u32);", false),
        ("pub struct Row {}", false),
        ("pub enum Row { Draft, Paid(u32) }", false),
        (
            "#[serde(tag = \"kind\")] pub enum Row { Draft, Paid }",
            false,
        ),
        (
            "#[serde(tag = \"t\", content = \"c\")] pub enum Row { Draft, Paid }",
            false,
        ),
        ("#[serde(untagged)] pub enum Row { Draft, Paid }", false),
        ("pub type Row = String;", false),
    ] {
        let item: syn::Item = syn::parse_str(source).unwrap();
        assert_eq!(
            super::written_as_one_value(&item),
            one_value,
            "for {source}"
        );
    }
}

/// A tuple struct's paths are a tuple struct, each slot's key the position serde writes it at. A
/// slot serde never writes keeps its place as `()`, so a member's position is its slot's.
#[cfg(feature = "mongodb")]
#[test]
fn a_tuple_structs_paths_are_by_position() {
    let (module, on_the_type) = typed_paths_of(
        "pub struct Row(pub f64, #[serde(skip)] pub u8, pub Inner);",
        &["Inner"],
    );
    assert!(
        module.contains(
            "pubstructMongoFields<Root>(pubself::Field<Root,f64>,pub(),\
             pubself::Model<Root,super::Inner,super::inner_schema::MongoFields<Root>>);"
        ),
        "got: {module}"
    );
    assert!(
        on_the_type.contains(
            "row_schema::MongoFields(row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"0\")),(),\
             row_schema::Model::plain(row_schema::MongoPath::under(prefix,\"1\"),"
        ),
        "got: {on_the_type}"
    );
}

/// A generic type's struct of paths carries the type's own parameters after the row type, under
/// the bounds the type declares. Members are public and what the struct keeps for itself is not,
/// so a struct that keeps anything holds its members in a struct of their own and dereferences
/// to it: here a marker of the parameter no member names. With no member at all the function
/// that builds the struct reads no key.
#[cfg(feature = "mongodb")]
#[test]
fn a_struct_of_paths_carries_the_types_own_parameters_and_never_mixes_what_it_keeps() {
    let (bounded, _on_the_type) = typed_paths_of(
        "pub struct Row<T: Clone + Shown, const N: usize, U = String> where T: Send \
         { pub a: T, pub b: [u8; N], pub c: Option<U> }",
        &[],
    );
    assert!(
        bounded.contains(
            "pubstructMongoFields<Root,T:Clone+super::Shown,constN:usize,U=String>whereT:Send\
             {puba:self::Field<Root,T>,pubb:self::ListField<Root,u8>,pubc:self::OptionalField<Root,U>}"
        ),
        "got: {bounded}"
    );
    let (marked, on_the_type) = typed_paths_of(
        "pub struct Row<T> { pub a: u8, pub b: HashMap<String, T> }",
        &[],
    );
    for written in [
        "pubstructMongoFields<Root,T>{members:self::mongo_members::Members<Root>,\
         unused:::core::marker::PhantomData<fn()->(T,)>,}",
        "pubconstfnbuilt(members:self::mongo_members::Members<Root>,)->Self",
        "impl<Root,T>::core::ops::DerefforMongoFields<Root,T>{typeTarget=self::mongo_members::Members<Root>;",
        "pubmodmongo_members{usesuper::*;",
        "pubstructMembers<Root>{puba:super::Field<Root,u8>}",
    ] {
        assert!(marked.contains(written), "missing `{written}` in: {marked}");
    }
    assert!(
        on_the_type.contains(
            "pubconstMONGO_FIELDS:row_schema::MongoFields<Self,T>=Self::mongo_fields_under(row_schema::MongoPath::ROOT);"
        ),
        "got: {on_the_type}"
    );
    assert!(
        on_the_type.contains(
            "->row_schema::MongoFields<Root,T>{row_schema::MongoFields::built(row_schema::mongo_members::Members{a:"
        ),
        "got: {on_the_type}"
    );
    let (empty, unread) = typed_paths_of("pub struct Row { pub b: HashMap<String, u8> }", &[]);
    assert!(
        empty.contains(
            "pubstructMongoFields<Root>{unused:::core::marker::PhantomData<fn()->(Root,)>,}"
        ),
        "got: {empty}"
    );
    assert!(
        unread.contains(
            "pubconstfnmongo_fields_under<Root>(_:[::core::option::Option<&'staticstr>;8],)\
             ->row_schema::MongoFields<Root>{row_schema::MongoFields::built()}"
        ),
        "got: {unread}"
    );
}

/// An enum's struct of paths keeps the path that names its variant, and answers `is_{variant}`
/// from it: the tag's key under a tag, and the enum's own under a variant's name. What a variant
/// holds is a member under the variant's name: beside the tag, under the content key, under the
/// name serde writes the variant as, or where the enum itself is.
#[cfg(feature = "mongodb")]
#[test]
fn an_enums_paths_are_where_its_form_writes_what_each_variant_holds() {
    for (source, asked, written) in [
        (
            "#[serde(tag = \"kind\", rename_all = \"camelCase\")] pub enum Row { Cash, \
             #[serde(rename_all = \"camelCase\")] Card { exp_month: u32 }, Wire(Inner), Odd(u8) }",
            "row_schema::MongoFields::built(row_schema::MongoPath::under(prefix,\"kind\"),",
            &[
                "card:row_schema::mongo_members::Card{exp_month:row_schema::Field::plain(\
                 row_schema::MongoPath::under(prefix,\"expMonth\"))}",
                "wire:<Inner>::mongo_fields_under(prefix)",
                "pubfnis_cash(&self)->self::Filter<Root>{self.tagged(\"cash\")}",
                "pubfnis_odd(&self)->self::Filter<Root>{self.tagged(\"odd\")}",
            ][..],
        ),
        (
            "#[serde(tag = \"t\", content = \"c\")] pub enum Row { Pickup, Locker(u32), \
             Door(Inner), Span(u32, u32) }",
            "row_schema::MongoFields::built(row_schema::MongoPath::under(prefix,\"t\"),",
            &[
                "locker:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"c\"))",
                "door:row_schema::Model::plain(row_schema::MongoPath::under(prefix,\"c\"),\
                 <Inner>::mongo_fields_under(row_schema::MongoPath::under(prefix,\"c\").segments))",
                "span:row_schema::mongo_members::Span(row_schema::Field::plain(row_schema::MongoPath::under(\
                 row_schema::MongoPath::under(prefix,\"c\").segments,\"0\")),",
                "pubfnis_pickup(&self)->self::Filter<Root>{self.tagged(\"Pickup\")}",
            ],
        ),
        (
            "#[serde(rename_all = \"camelCase\")] pub enum Row { NoDiscount, Percent(f64), \
             Coupon { code: String } }",
            "row_schema::MongoFields::built(row_schema::MongoPath::at(prefix),",
            &[
                "percent:row_schema::Field::plain(row_schema::MongoPath::under(prefix,\"percent\"))",
                "coupon:row_schema::mongo_members::Coupon{code:row_schema::Field::plain(row_schema::MongoPath::under(\
                 row_schema::MongoPath::under(prefix,\"coupon\").segments,\"code\"))}",
                "pubfnis_no_discount(&self)->self::Filter<Root>{self::Filter::held(self.asked.key(),\"$eq\",\
                 bson::Bson::String(\"noDiscount\".to_owned()))}",
                "pubfnis_coupon(&self)->self::Filter<Root>{self.keyed(\"coupon\")}",
            ],
        ),
        (
            "#[serde(untagged)] pub enum Row { Number(u32), Detailed { code: String }, Held(Inner) }",
            "row_schema::MongoFields{number:row_schema::Field::plain(row_schema::MongoPath::at(prefix)),",
            &[
                "detailed:row_schema::mongo_members::Detailed{code:row_schema::Field::plain(\
                 row_schema::MongoPath::under(prefix,\"code\"))}",
                "held:<Inner>::mongo_fields_under(prefix)",
            ],
        ),
    ] {
        let (module, on_the_type) = typed_enum_paths_of(source, &["Inner"]);
        assert!(
            on_the_type.contains(asked),
            "for {source}, got: {on_the_type}"
        );
        let emitted = format!("{module}{on_the_type}");
        for found in written {
            assert!(
                emitted.contains(found),
                "for {source}, missing `{found}` in: {emitted}"
            );
        }
        assert_eq!(
            module.contains("pubfnis_"),
            !source.contains("untagged"),
            "for {source}, got: {module}"
        );
    }
}

/// A variant serde never writes has neither member nor `is_{variant}`. A struct `mongo_members`
/// holds is never named as a prelude type or as a parameter, which every member beside it that
/// names one would otherwise read in its place.
#[cfg(feature = "mongodb")]
#[test]
fn a_variant_serde_never_writes_has_no_path_and_a_members_struct_takes_no_name_in_use() {
    let (module, on_the_type) = typed_enum_paths_of(
        "pub enum Row<T> { #[serde(skip)] Lost { at: u8 }, #[serde(skip_serializing)] Old(u8), \
         String { text: String }, T { held: T }, Members { count: u8 } }",
        &[],
    );
    let emitted = format!("{module}{on_the_type}");
    for absent in ["lost:", "old:", "is_lost", "is_old", "Lost", "Old"] {
        assert!(!emitted.contains(absent), "found `{absent}` in: {emitted}");
    }
    for written in [
        "pubstructString2<Root>{pubtext:super::Field<Root,String>}",
        "pubstructT2<Root,T>{pubheld:super::Field<Root,T>}",
        "pubstructMembers<Root>{pubcount:super::Field<Root,u8>}",
        "pubstructMembers2<Root,T>{pubstring:self::String2<Root>,pubt:self::T2<Root,T>,pubmembers:self::Members<Root>}",
    ] {
        assert!(
            emitted.contains(written),
            "missing `{written}` in: {emitted}"
        );
    }
}

/// Of the `bson` library the typed paths name only what both of its major versions have, the
/// serializer and two members of a value, and nothing they add is hidden from a lint or a reader.
#[cfg(feature = "mongodb")]
#[test]
fn the_typed_paths_name_only_what_both_majors_of_the_bson_library_have() {
    let (hooked, on_the_type) = typed_paths_of(
        "pub struct Row { #[serde(with = \"hook\")] pub a: u8, pub b: Inner, pub c: Vec<u8> }",
        &["Inner"],
    );
    let (external, _external) =
        typed_enum_paths_of("pub enum Row { Empty, Label(String), To(i32, i32) }", &[]);
    let (tagged, _tagged) = typed_enum_paths_of(
        "#[serde(tag = \"kind\")] pub enum Row { Clear, Solid { color: String } }",
        &[],
    );
    let written = [hooked, on_the_type, external, tagged].concat();
    let named_after = |prefix: &str| {
        let mut found: Vec<&str> = written
            .split(prefix)
            .skip(1)
            .filter_map(|rest| {
                rest.split(|read: char| !(read.is_ascii_alphanumeric() || read == '_'))
                    .next()
            })
            .collect();
        found.sort_unstable();
        found.dedup();
        found
    };
    assert_eq!(named_after("bson::"), ["Bson", "Serializer"]);
    assert_eq!(named_after("bson::Bson::"), ["Boolean", "String"]);
    assert_eq!(named_after("bson::Serializer::"), ["new"]);
    for absent in ["doc!", "#[allow", "#[expect", "doc(hidden)"] {
        assert!(!written.contains(absent), "found `{absent}` in: {written}");
    }
}
