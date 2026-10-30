//! Unit tests of what `#[model_schema(enable = [...])]` emits, read off the expansion: the names it
//! adds, the entry points, the walkers and the MongoDB operations.

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::aliases::entry_of_items;
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::{ADDED_TYPE_NAMES, Source, read_whole_items, reading_the_authors_scope};
use super::{
    DECODE_WITH_IMPL_MACRO, DECODE_WITH_LIBRARY_NAMES, RecoveringDecode, decode_with_foundation,
    decode_with_library, written_names,
};
#[cfg(feature = "mongodb")]
use super::{MONGODB_LIBRARY_NAMES, mongodb_foundation, mongodb_library};
use crate::features::model_schema_prop::FunctionName;
use crate::model_schema::preprocess;
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

/// An enum under a tag written beside what each variant holds, and one under a tag and a content
/// key: each has one arm per variant serde writes.
const SHIPPING: &str = "#[serde(tag = \"type\")] pub enum Shipping { \
     #[serde(rename = \"by-courier\")] ByCourier(Courier), \
     #[serde(rename = \"by-rail\")] ByRail(Box<Wagon>), Collected, \
     #[serde(rename_all = \"camelCase\")] Locker { bay_number: String } }";
const PAPER: &str = "#[serde(tag = \"docType\", content = \"body\", rename_all = \"kebab-case\")] \
     pub enum Paper { PastDue(Notice), Receipt }";

/// The read every walk of a flagged type starts from. It is written once, after the methods of
/// the first source.
const LISTED_READ: &str = "decode_with_read_unjudged";

/// What `mongodb` adds to every type beside its BSON entry points, in the order written: the
/// reads and the count, the writes, and the read and the write of one stored row they share.
const OPERATIONS: [&str; 12] = [
    "find_one",
    "find_one_with",
    "find",
    "find_with",
    "count",
    "insert_one",
    "update_one",
    "update_many",
    "delete_one",
    "delete_many",
    "mongo_read_row",
    "mongo_written_row",
];

/// The `impl`s the structs of paths of each enum form are written in beside the type under
/// `mongodb`, each with its members: every function in one, an enum under a tag key's arm
/// functions among them, and the types of a tuple's slots filled in one of their own.
const PATHS_BESIDE: [(&str, ImplsBeside); 4] = [
    (
        EXTERNAL,
        &[
            (
                "impl < Root > outline_schema :: mongo_members :: Circle < Root >",
                &["radius"],
            ),
            (
                "impl < Root > outline_schema :: mongo_members :: ToSlots < Root > for ()",
                &["Slot0", "Slot1"],
            ),
            (
                "impl < Root > outline_schema :: MongoFields < Root >",
                &[
                    "circle",
                    "label",
                    "pinned",
                    "to",
                    "is_circle",
                    "is_empty",
                    "is_label",
                    "is_pinned",
                    "is_to",
                ],
            ),
        ],
    ),
    (
        INTERNAL,
        &[
            (
                "impl < Root > fill_schema :: mongo_members :: Solid < Root >",
                &["color"],
            ),
            (
                "impl fill_schema :: ArmPaths for fill_schema :: mongo_arms :: Solid",
                &["Paths", "paths"],
            ),
            (
                "impl < Root > fill_schema :: MongoFields < Root >",
                &[
                    "solid",
                    "versioned",
                    "is_clear",
                    "is_solid",
                    "is_versioned",
                    "kind__clear",
                    "kind__solid",
                    "kind__versioned",
                ],
            ),
        ],
    ),
    (
        ADJACENT,
        &[
            (
                "impl < Root > stroke_schema :: mongo_members :: Dashed < Root >",
                &["gap"],
            ),
            (
                "impl < Root > stroke_schema :: mongo_members :: SpanSlots < Root > for ()",
                &["Slot0", "Slot1"],
            ),
            (
                "impl stroke_schema :: ArmPaths for stroke_schema :: mongo_arms :: Dashed",
                &["Paths", "paths"],
            ),
            (
                "impl stroke_schema :: ArmPaths for stroke_schema :: mongo_arms :: Span where () : stroke_schema :: mongo_members :: SpanSlots < stroke_schema :: mongo_arms :: Span >",
                &["Paths", "paths"],
            ),
            (
                "impl < Root > stroke_schema :: MongoFields < Root >",
                &[
                    "dashed",
                    "level",
                    "span",
                    "width",
                    "is_dashed",
                    "is_hairline",
                    "is_level",
                    "is_span",
                    "is_width",
                    "kind__dashed",
                    "kind__hairline",
                    "kind__level",
                    "kind__span",
                    "kind__width",
                ],
            ),
        ],
    ),
    (
        UNTAGGED,
        &[
            (
                "impl < Root > contact_schema :: mongo_members :: Email < Root >",
                &["address"],
            ),
            (
                "impl < Root > contact_schema :: MongoFields < Root >",
                &["email", "versioned", "word"],
            ),
        ],
    ),
];

/// The `impl`s written beside a type, each as its header and its members.
type ImplsBeside = &'static [(&'static str, &'static [&'static str])];

/// The enum emitter, run as it runs beside model types declared above the enum: every type the
/// item names is taken for one, so its walker is called as it stands. The enum lists "mongodb".
fn enum_recovering_decode(item: &syn::ItemEnum) -> RecoveringDecode {
    seen_as_models(item.to_token_stream());
    super::enums::enum_recovering_decode(item, true)
}

/// The items of both support libraries, as one module would hold them.
fn module_items() -> proc_macro2::TokenStream {
    let decode_with = decode_with_library();
    #[cfg(feature = "mongodb")]
    let mongodb = mongodb_library();
    #[cfg(not(feature = "mongodb"))]
    let mongodb = proc_macro2::TokenStream::new();
    quote::quote!(#decode_with #mongodb)
}

/// The names an item of `items` declares at their top, each with whether it is public.
fn declared_names(items: proc_macro2::TokenStream) -> Vec<(String, bool)> {
    let parsed: syn::File = syn::parse2(items).unwrap();
    parsed
        .items
        .iter()
        .filter_map(|item| {
            let (ident, visibility) = if let syn::Item::Enum(declared) = item {
                (&declared.ident, &declared.vis)
            } else if let syn::Item::Fn(declared) = item {
                (&declared.sig.ident, &declared.vis)
            } else if let syn::Item::Struct(declared) = item {
                (&declared.ident, &declared.vis)
            } else if let syn::Item::Trait(declared) = item {
                (&declared.ident, &declared.vis)
            } else if let syn::Item::Type(declared) = item {
                (&declared.ident, &declared.vis)
            } else if let syn::Item::Const(declared) = item {
                (&declared.ident, &declared.vis)
            } else if let syn::Item::Static(declared) = item {
                (&declared.ident, &declared.vis)
            } else if let syn::Item::Mod(declared) = item {
                (&declared.ident, &declared.vis)
            } else {
                return None;
            };
            Some((
                ident.to_string(),
                matches!(visibility, syn::Visibility::Public(_)),
            ))
        })
        .collect()
}

/// The names `items` make public, sorted.
fn public_names(items: proc_macro2::TokenStream) -> Vec<String> {
    let mut public: Vec<String> = declared_names(items)
        .into_iter()
        .filter_map(|(name, public)| public.then_some(name))
        .collect();
    public.sort();
    public
}

/// Records every capitalized name `tokens` write as a model type declared above.
fn seen_as_models(tokens: proc_macro2::TokenStream) {
    for name in written_names(tokens) {
        if name.starts_with(char::is_uppercase) {
            record_declared(&name, Declared::model(0));
        }
    }
}

/// The struct emitter, run the same way.
fn struct_recovering_decode(item: &syn::ItemStruct) -> RecoveringDecode {
    seen_as_models(item.to_token_stream());
    super::struct_recovering_decode(item, true)
}

/// The inherent `impl` the flag adds to `source`, as text.
fn type_impl_of(source: &str) -> String {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    inherent_text(struct_recovering_decode(&item).type_impl)
}

/// Every `impl` the flag adds to `source`, as text.
fn whole_impl_of(source: &str) -> String {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    struct_recovering_decode(&item).type_impl.to_string()
}

/// `type_impl` as a file, with the items inside each call of the macro that wraps a `DecodeWith`
/// impl written in place of the call.
fn unwrapped_file(type_impl: proc_macro2::TokenStream) -> syn::File {
    let mut file: syn::File = syn::parse2(type_impl).unwrap();
    file.items = file
        .items
        .into_iter()
        .flat_map(|item| {
            if let syn::Item::Macro(call) = &item
                && call
                    .mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|last| last.ident == DECODE_WITH_IMPL_MACRO)
            {
                syn::parse2::<syn::File>(call.mac.tokens.clone())
                    .unwrap()
                    .items
            } else {
                vec![item]
            }
        })
        .collect();
    file
}

/// The inherent `impl` the flag adds to the enum `source` declares, as text.
fn enum_impl_of(source: &str) -> String {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    inherent_text(enum_recovering_decode(&item).type_impl)
}

/// `type_impl` as text, without the `impl`s of `DecodeWith`.
fn inherent_text(type_impl: proc_macro2::TokenStream) -> String {
    use quote::ToTokens as _;

    let added = unwrapped_file(type_impl);
    added
        .items
        .iter()
        .filter(|added_item| {
            !matches!(added_item, syn::Item::Impl(block)
            if block.trait_.as_ref().is_some_and(|(implemented, _for)| {
                implemented.segments.last().is_some_and(|last| last.ident == "DecodeWith")
            }))
        })
        .map(|added_item| added_item.to_token_stream().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// What the flag adds to the enum `source` declares for a JSON value, as text: everything ahead
/// of the BSON entry point.
fn enum_json_of(source: &str) -> String {
    let emitted = enum_impl_of(source);
    emitted
        .split_once("fn decode_with_bson_report")
        .map_or(emitted.as_str(), |(json, _bson)| json)
        .to_owned()
}

/// What the flag adds to the enum `source` declares for a BSON document, as text.
#[cfg(feature = "bson")]
fn enum_bson_of(source: &str) -> String {
    let emitted = enum_impl_of(source);
    let (_json, bson) = emitted.split_once("fn decode_with_bson_report").unwrap();
    bson.to_owned()
}

/// The statements `decode_with_value_fields` runs for `source`, as text, to the end of the `impl`
/// with the typed paths `mongodb` writes after the walkers taken out of it.
fn fields_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, methods) = emitted
        .split_once("pub fn decode_with_value_fields")
        .unwrap();
    // The read every walk starts from closes the `impl` where the JSON source is the only one.
    let walk = methods
        .split_once(&format!("pub fn {LISTED_READ}"))
        .and_then(|(walkers, _read)| walkers.split_once(" # [doc"))
        .filter(|_| !cfg!(feature = "bson"))
        .map_or_else(
            || methods.to_owned(),
            |(walker, _docs)| format!("{walker} }}"),
        );
    walk.split_once("# [doc = r\" The typed MongoDB paths of this type")
        .map_or_else(|| walk.clone(), |(walkers, _paths)| format!("{walkers}}}"))
}

/// The statements `decode_with_value_fields` runs for `source`, as text, up to the BSON entry
/// point.
fn json_fields_walk_of(source: &str) -> String {
    let walk = fields_walk_of(source);
    walk.split_once("fn decode_with_bson_report")
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
    reads_under_their_types(impls_in(struct_recovering_decode(&item).type_impl))
}

/// `impls` with each `impl` of `DecodeWith` folded into the inherent `impl` before it, which it
/// must match in everything but the trait: the reads are listed last among that one's members.
fn reads_under_their_types(impls: Vec<(String, Vec<String>)>) -> Vec<(String, Vec<String>)> {
    let mut folded: Vec<(String, Vec<String>)> = Vec::new();
    for (header, members) in impls {
        let Some((path, after)) = header.split_once("DecodeWith for ") else {
            folded.push((header, members));
            continue;
        };
        let (own_header, own) = folded.last_mut().unwrap();
        let module_path = path.trim_end().strip_suffix("::").unwrap().trim_end();
        let (generics, _module) = module_path.rsplit_once(' ').unwrap();
        assert_eq!(*own_header, format!("{generics} {after}"));
        own.extend(members);
    }
    folded
}

/// The type's own `impl` among `type_impl`: the first, ahead of what is written beside the type.
fn own_impl(type_impl: proc_macro2::TokenStream) -> syn::ItemImpl {
    let added = unwrapped_file(type_impl);
    added
        .items
        .into_iter()
        .find_map(|added_item| {
            if let syn::Item::Impl(block) = added_item {
                Some(block)
            } else {
                None
            }
        })
        .unwrap()
}

/// The `impl`s the flag adds to the enum `source` declares, as [`added_impls`] lists a struct's.
fn added_enum_impls(source: &str) -> Vec<(String, Vec<String>)> {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    reads_under_their_types(impls_in(enum_recovering_decode(&item).type_impl))
}

/// Every `impl` among `type_impl`: its header as text, with the trait it implements, and its
/// members in the order written.
fn impls_in(type_impl: proc_macro2::TokenStream) -> Vec<(String, Vec<String>)> {
    let added = unwrapped_file(type_impl);
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
            let of_trait = block
                .trait_
                .as_ref()
                .map(|(implemented, _for)| quote::quote!(#implemented for));
            let methods: Vec<String> = block.items.iter().filter_map(member_name).collect();
            assert_eq!(methods.len(), block.items.len());
            Some((
                quote::quote!(impl #generics #of_trait #self_ty #where_clause).to_string(),
                methods,
            ))
        })
        .collect();
    assert_eq!(impls.len(), added.items.len());
    impls
}

/// The name of a const, a method or an associated type of an `impl`, and `None` for any other
/// member of one.
fn member_name(member: &syn::ImplItem) -> Option<String> {
    if let syn::ImplItem::Fn(method) = member {
        Some(method.sig.ident.to_string())
    } else if let syn::ImplItem::Const(constant) = member {
        Some(constant.ident.to_string())
    } else if let syn::ImplItem::Type(associated) = member {
        Some(associated.ident.to_string())
    } else {
        None
    }
}

/// The names the methods among `type_impl` give type parameters of their own, each once, sorted.
fn own_type_parameters(type_impl: proc_macro2::TokenStream) -> Vec<String> {
    let added = unwrapped_file(type_impl);
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

/// The parameter the struct of paths in `module_items` carries the row type as: its first.
#[cfg(feature = "mongodb")]
fn row_parameter(module_items: proc_macro2::TokenStream) -> String {
    let declared: syn::File = syn::parse2(module_items).unwrap();
    parameter_name(declared.items.iter().find_map(|item| {
        if let syn::Item::Struct(paths) = item
            && paths.ident == "MongoFields"
        {
            paths.generics.type_params().next()
        } else {
            None
        }
    }))
}

#[cfg(feature = "mongodb")]
#[rustversion::since(1.98)]
fn parameter_name(found: Option<&syn::TypeParam>) -> String {
    found.map_or_default(|row| row.ident.to_string())
}

#[cfg(feature = "mongodb")]
#[rustversion::before(1.98)]
fn parameter_name(found: Option<&syn::TypeParam>) -> String {
    found.map(|row| row.ident.to_string()).unwrap_or_default()
}

/// What the typed paths add to every type: under `mongodb`, the const that holds them, and
/// nothing in a build without it.
fn paths_members() -> Vec<String> {
    if cfg!(feature = "mongodb") {
        vec!["MONGO_FIELDS".to_owned()]
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

/// What the typed paths of the enum `source` declares add to its schema module.
#[cfg(all(
    feature = "mongodb",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn enum_paths_module_items(source: &str) -> proc_macro2::TokenStream {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    super::fields::enum_paths(&item, &[]).module_items
}

/// A build without `mongodb` writes no typed path.
#[cfg(all(
    not(feature = "mongodb"),
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn enum_paths_module_items(_source: &str) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

/// The operations the flag adds to a struct of named fields, each as its visibility and its
/// signature, in the order written.
#[cfg(feature = "mongodb")]
fn operation_signatures() -> Vec<String> {
    let item: syn::ItemStruct = syn::parse_str("pub struct Named { pub title: String }").unwrap();
    let added = own_impl(struct_recovering_decode(&item).type_impl);
    added
        .items
        .iter()
        .filter_map(|added_item| {
            if let syn::ImplItem::Fn(method) = added_item
                && OPERATIONS.contains(&method.sig.ident.to_string().as_str())
            {
                let (visibility, signature) = (&method.vis, &method.sig);
                Some(quote::quote!(#visibility #signature).to_string())
            } else {
                None
            }
        })
        .collect()
}

/// The methods one source adds to every type, in the order written. Under `mongodb` the
/// operations sit beside the BSON entry points.
fn methods_of(stem: &str) -> Vec<String> {
    let mut named = vec![format!("decode_with_{stem}_report")];
    if stem == "bson" && cfg!(feature = "mongodb") {
        named.extend(OPERATIONS.map(str::to_owned));
    }
    named.extend([
        format!("decode_with_{stem}_issues"),
        format!("decode_with_{stem}_named"),
        format!("decode_with_{stem}_fields"),
    ]);
    named
}

/// The reads of `DecodeWith` for the source `stem`.
fn reads_of(stem: &str) -> Vec<String> {
    vec![format!("from_{stem}_with"), format!("from_{stem}_piped")]
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

    let added = unwrapped_file(type_impl);
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

/// The types `Stored` is implemented for in the support libraries, in the order written.
fn stored_impls() -> Vec<String> {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Impl(block) = item
                && let Some((trait_path, _)) = &block.trait_
                && trait_path.is_ident("Stored")
            {
                let self_ty = &block.self_ty;
                Some(quote::quote!(#self_ty).to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Every token `tokens` is made of, in order, with each group opened between two parentheses,
/// whatever its own delimiters.
#[cfg(feature = "mongodb")]
fn bounded_tokens(tokens: proc_macro2::TokenStream, bounded: &mut Vec<String>) {
    for token in tokens {
        if let proc_macro2::TokenTree::Group(group) = token {
            bounded.push("(".to_owned());
            bounded_tokens(group.stream(), bounded);
            bounded.push(")".to_owned());
        } else {
            bounded.push(token.to_string());
        }
    }
}

/// The module or crate each path among `emitted` starts at, each once, sorted: a lowercase name
/// ahead of `::` that no earlier segment of a path leads to.
#[cfg(feature = "mongodb")]
fn path_roots(emitted: proc_macro2::TokenStream) -> Vec<String> {
    const KEYWORDS: [&str; 6] = ["as", "dyn", "impl", "let", "match", "mut"];
    let mut leaves = Vec::new();
    bounded_tokens(emitted, &mut leaves);
    // Each `::` as one token. Of three colons in a row the first ends a name and its type apart.
    let mut tokens: Vec<String> = Vec::new();
    for leaf in leaves {
        match (leaf.as_str(), tokens.last().map(String::as_str)) {
            (":", Some(":")) => {
                tokens.pop();
                tokens.push("::".to_owned());
            }
            (":", Some("::")) => {
                tokens.pop();
                tokens.extend([":".to_owned(), "::".to_owned()]);
            }
            _ => tokens.push(leaf),
        }
    }
    let named = |token: &str| {
        token.starts_with(|first: char| first.is_alphabetic() || first == '_')
            && !KEYWORDS.contains(&token)
    };
    let mut roots: Vec<String> = (0..tokens.len().saturating_sub(1))
        .filter(|&at| {
            let led = at > 0 && tokens[at - 1] == "::";
            let continued = led && at > 1 && (named(&tokens[at - 2]) || tokens[at - 2] == ">");
            named(&tokens[at])
                && tokens[at].starts_with(|first: char| first.is_ascii_lowercase())
                && tokens[at + 1] == "::"
                && !continued
        })
        .map(|at| tokens[at].clone())
        .collect();
    roots.sort_unstable();
    roots.dedup();
    roots
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

#[test]
fn every_added_method_but_the_entry_point_carries_the_flags_name() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Named { pub title: String, pub versions: Vec<Version> }")
            .unwrap();
    let emitted = struct_recovering_decode(&item);
    let added = own_impl(emitted.type_impl);
    // What every source's entry points run on the value serde built is in an `impl` of its own,
    // under the bounds the type's author wrote.
    let checked: syn::ItemImpl = syn::parse2(emitted.check_messages).unwrap();
    let methods: Vec<String> = added
        .items
        .iter()
        .chain(&checked.items)
        .filter_map(member_name)
        .collect();
    assert_eq!(methods.len(), added.items.len() + checked.items.len());
    let mut named = vec![
        "decode_with_value_report",
        "decode_with_value_issues",
        "decode_with_value_named",
        "decode_with_value_fields",
        LISTED_READ,
    ];
    if cfg!(feature = "bson") {
        named.push("decode_with_bson_report");
        if cfg!(feature = "mongodb") {
            named.extend(OPERATIONS);
        }
        named.extend([
            "decode_with_bson_issues",
            "decode_with_bson_named",
            "decode_with_bson_fields",
        ]);
    }
    // The typed paths carry the name of the feature that adds them.
    if cfg!(feature = "mongodb") {
        named.push("MONGO_FIELDS");
    }
    named.extend([
        "decode_with_check_messages",
        "decode_with_value_check_issues",
    ]);
    if cfg!(feature = "bson") {
        named.push("decode_with_bson_check_issues");
    }
    assert_eq!(methods, named);
}

/// Under `mongodb` a type that lists `"decode_with"` alone is read from BSON, and gets no
/// operation, no typed path and nothing of the MongoDB library.
#[cfg(feature = "mongodb")]
#[test]
fn a_type_that_lists_decode_with_alone_is_read_from_bson_and_gets_no_mongodb_layer() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Version { pub number: u32, pub held: Inner }").unwrap();
    seen_as_models(item.to_token_stream());
    let added = super::struct_recovering_decode(&item, false);
    let members: Vec<String> = impls_in(added.type_impl.clone())
        .into_iter()
        .chain(impls_in(added.check_messages.clone()))
        .flat_map(|(_header, members)| members)
        .collect();
    assert_eq!(
        members,
        [
            "decode_with_value_report",
            "decode_with_value_issues",
            "decode_with_value_named",
            "decode_with_value_fields",
            LISTED_READ,
            "decode_with_bson_report",
            "decode_with_bson_issues",
            "decode_with_bson_named",
            "decode_with_bson_fields",
            "from_value_with",
            "from_value_piped",
            "from_bson_with",
            "from_bson_piped",
            "decode_with_check_messages",
            "decode_with_value_check_issues",
            "decode_with_bson_check_issues",
        ]
    );
    let enum_item: syn::ItemEnum = syn::parse_str(INTERNAL).unwrap();
    seen_as_models(enum_item.to_token_stream());
    let enum_added = super::enums::enum_recovering_decode(&enum_item, false);
    let enum_impl = enum_added.type_impl.to_string();
    for bson_read in ["fn from_bson_with", "fn from_bson_piped"] {
        assert!(
            enum_impl.contains(bson_read),
            "missing `{bson_read}` in: {enum_impl}"
        );
    }
    for emitted in [added, enum_added] {
        let everything = [
            emitted.schema_module.to_string(),
            emitted.type_impl.to_string(),
            emitted.probes.to_string(),
            emitted.check_messages.to_string(),
        ]
        .concat();
        let operations = OPERATIONS.map(|operation| format!("fn {operation} "));
        let typed_paths = ["MONGO_FIELDS", "MongoFields", "__tixschema_mongodb"];
        for name in operations.iter().map(String::as_str).chain(typed_paths) {
            assert!(
                !everything.contains(name),
                "found `{name}` in: {everything}"
            );
        }
    }
}

/// A type that names a `preprocess` on itself keeps each walk under its name, over a value its
/// functions corrected, for a holder to call, and adds the walk of a value already corrected.
#[test]
fn a_type_that_names_a_preprocess_adds_the_walk_of_a_corrected_value() {
    let item: syn::ItemStruct =
        syn::parse_str("#[derive(Deserialize)] pub struct Named { pub title: String }").unwrap();
    let named_on_the_type = [FunctionName {
        name: "unwrapped".to_owned(),
        written_at: proc_macro2::Span::call_site(),
    }];
    preprocess::record(&syn::Item::Struct(item.clone()), &named_on_the_type);
    let emitted = struct_recovering_decode(&item);
    // The record is the item's own: a test run after this one on the same thread reads none.
    preprocess::record(&syn::Item::Struct(item), &[]);
    let added = own_impl(emitted.type_impl);
    let methods: Vec<String> = added.items.iter().filter_map(member_name).collect();
    let mut named = vec![
        "decode_with_value_report",
        "decode_with_value_corrected_issues",
        "decode_with_value_corrected_named",
        "decode_with_value_corrected_fields",
        "decode_with_value_corrected",
        "decode_with_value_issues",
        "decode_with_value_named",
        "decode_with_value_fields",
        LISTED_READ,
        "decode_with_read_corrected",
    ];
    if cfg!(feature = "bson") {
        named.push("decode_with_bson_report");
        if cfg!(feature = "mongodb") {
            named.extend(OPERATIONS);
        }
        named.extend([
            "decode_with_bson_corrected_issues",
            "decode_with_bson_corrected_named",
            "decode_with_bson_corrected_fields",
            "decode_with_bson_corrected",
            "decode_with_bson_issues",
            "decode_with_bson_named",
            "decode_with_bson_fields",
        ]);
    }
    if cfg!(feature = "mongodb") {
        named.push("MONGO_FIELDS");
    }
    assert_eq!(methods, named);
}

#[cfg(feature = "bson")]
#[test]
fn the_bson_methods_carry_the_signatures_of_their_json_twins() {
    let item: syn::ItemStruct = syn::parse_str("pub struct Named { pub title: String }").unwrap();
    let type_impl = struct_recovering_decode(&item).type_impl;
    let added = own_impl(type_impl.clone());
    let every = unwrapped_file(type_impl);
    let signatures: Vec<String> = every
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
            "fn from_bson_with < F > (document : bson :: Document , decide : F ,) \
             -> :: core :: result :: Result < Self , named_schema :: Unrecovered < bson :: Bson > > \
             where F : :: core :: ops :: FnOnce (& mut bson :: Document , & [named_schema :: Issue < bson :: Bson >]) \
             -> named_schema :: Verdict ,",
            "fn from_bson_piped (document : bson :: Document , resolvers : & [named_schema :: \
             Resolver < '_ , bson :: Document , bson :: Bson >] ,) -> :: core :: result :: Result < \
             Self , named_schema :: Unrecovered < bson :: Bson > >",
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

#[cfg(not(feature = "bson"))]
#[test]
fn without_bson_nothing_of_the_bson_read_is_emitted() {
    let emitted = emission_of(EVERY_WALK);
    for absent in ["bson", "same_bracket", "_document"] {
        assert!(!emitted.contains(absent), "found `{absent}` in: {emitted}");
    }
    assert_eq!(stored_impls(), ["serde_json :: Value"]);
}

#[cfg(feature = "bson")]
#[test]
fn the_bson_emission_names_only_what_both_major_versions_have() {
    let item: syn::ItemStruct = syn::parse_str(EVERY_WALK).unwrap();
    let added = struct_recovering_decode(&item);
    let mut leaves = Vec::new();
    leaf_tokens(module_items(), &mut leaves);
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
    let emitted = format!("{} {}", module_items(), emission_of(EVERY_WALK));
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

#[test]
fn from_value_with_reads_the_value_once_per_decode_and_its_report_reads_nothing() {
    for (source, type_impl) in impls_of_every_shape() {
        let entry = body_of(type_impl.clone(), "from_value_with");
        // The first read, and the one after the callback answers `Fixed`.
        assert_eq!(
            entry
                .matches("Self :: decode_with_read_unjudged (& value)")
                .count(),
            2,
            "for {source}, got: {entry}"
        );
        assert_eq!(
            entry.matches("read_unjudged").count(),
            2,
            "for {source}, got: {entry}"
        );
        assert!(!entry.contains("deserialize"), "for {source}, got: {entry}");
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

/// A resolver answers one issue with one of three states.
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
            "< 'r , D , V > = & 'r (dyn :: core :: ops :: Fn (& mut D , & Issue < V >) -> \
             Resolution + :: core :: marker :: Sync)",
            "pub fn unsettled < D , V : :: core :: clone :: Clone > (raw : & mut D , issues : & \
             [Issue < V >] , resolvers : & [Resolver < '_ , D , V >] ,) -> :: std :: vec :: Vec < \
             Issue < V > >",
        ]
    );
}

#[cfg(feature = "bson")]
#[test]
fn the_bson_items_are_emitted_beside_the_json_ones() {
    assert_eq!(stored_impls(), ["serde_json :: Value", "bson :: Document"]);
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
            "value_digits_as_numbers",
            "value_digits_settled",
            "reads_an_option",
            "taken_keys",
            "value_remaining",
            "value_bound",
            "bson_bound",
            "same_bracket",
            "bson_leaf",
            "bson_digits_as_numbers",
            "bson_digits_settled",
            "bson_remaining",
            #[cfg(feature = "mongodb")]
            "write_plain",
            #[cfg(feature = "mongodb")]
            "nested_key",
            #[cfg(feature = "mongodb")]
            "nested_filter",
            #[cfg(feature = "mongodb")]
            "nested_update",
        ]
    );
}

#[test]
fn the_query_types_are_declared_under_mongodb_alone_and_none_is_exhaustive() {
    let query_types = [
        "MongoPath",
        "Filter",
        "ArmFilter",
        "Update",
        "ArmUpdate",
        "AsFilter",
        "AsUpdate",
        "Field",
        "OptionalField",
        "Element",
        "ListField",
        "Model",
        "OptionalModel",
        "ModelList",
        "Nested",
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

#[cfg(feature = "mongodb")]
#[test]
fn each_query_type_has_the_operators_of_its_kind() {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    for (named, operators) in [
        ("MongoPath", &["at", "key"][..]),
        ("Filter", &["raw", "and", "or", "negated", "into_document"]),
        (
            "ArmFilter",
            &[
                "of",
                "and",
                "update",
                "tag_on_insert",
                "into_document",
                "with",
            ],
        ),
        ("Update", &["raw", "and", "into_document"]),
        ("ArmUpdate", &["and", "tag_on_insert", "into_document"]),
        (
            "Field",
            &[
                "plain",
                "hooked",
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
                "nest",
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
                "nest",
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
                "nest",
            ],
        ),
        ("OptionalModel", &["plain", "exists", "unset"]),
        (
            "ModelList",
            &["plain", "elem_match", "size", "push", "pull", "set", "nest"],
        ),
        ("Nested", &["at", "nest"]),
    ] {
        let public: Vec<String> = query_methods(&added)
            .into_iter()
            .filter(|(on, method)| on == named && matches!(method.vis, syn::Visibility::Public(_)))
            .map(|(_on, method)| method.sig.ident.to_string())
            .collect();
        assert_eq!(public, operators, "for {named}");
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn an_operator_that_writes_a_value_takes_it_by_value_and_answers_a_write_error() {
    let added: syn::File = syn::parse2(super::query::query_items()).unwrap();
    for (on, method) in query_methods(&added) {
        let signature = method.sig.to_token_stream().to_string();
        let answered = method.sig.output.to_token_stream().to_string();
        let writes = ["value : V", "value : M", "values : I", "condition : B"]
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

#[cfg(feature = "mongodb")]
#[test]
fn a_filter_and_an_update_each_carry_a_marker_of_their_own() {
    let emitted = super::query::query_items();
    let packed: String = emitted.to_string().split_whitespace().collect();
    for carried in [
        "impl<Root>::core::convert::AsRef<::core::marker::PhantomData<Root>>forFilter<Root>",
        "impl<Root,M:?::core::marker::Sized>::core::convert::AsRef<::core::marker::PhantomData<Root>>\
         forArmFilter<Root,M>",
        "impl<Root>::core::convert::AsRef<::core::marker::PhantomData<fn(Root)->Root>>forUpdate<Root>",
        "impl<Root,M:?::core::marker::Sized>\
         ::core::convert::AsRef<::core::marker::PhantomData<fn(Root)->Root>>forArmUpdate<Root,M>",
    ] {
        assert!(packed.contains(carried), "missing `{carried}`");
    }
    // `nest` takes a filter and an update by the same two markers, over the nested model.
    for taken in [
        "impl<Root,N,E>Nests<Root,N,AsFilter>forEwhereN:?::core::marker::Sized,\
         E:::core::convert::Into<bson::Document>+::core::convert::AsRef<::core::marker::PhantomData<N>>,",
        "impl<Root,N,E>Nests<Root,N,AsUpdate>forEwhere\
         E:::core::convert::Into<bson::Document>+::core::convert::AsRef<::core::marker::PhantomData<fn(N)->N>>,",
    ] {
        assert!(packed.contains(taken), "missing `{taken}`");
    }
    assert_eq!(packed.matches("AsRef<").count(), 16);
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
        ("ArmFilter::and", "M"),
        ("ArmFilter::update", "fn (M) -> M"),
        ("ArmFilter::with", "M"),
        ("Update::and", "fn (Root) -> Root"),
        ("ArmUpdate::and", "fn (M) -> M"),
        ("ModelList::elem_match", "N"),
        ("ModelList::pull", "N"),
    ]
    .map(|(function, marker)| (function.to_owned(), marker.to_owned()));
    assert_eq!(asked, expected);
    let nesting: Vec<String> = query_methods(&added)
        .into_iter()
        .filter(|(_on, method)| method.sig.ident == "nest")
        .map(|(on, method)| {
            let bounds = method.sig.generics.where_clause.to_token_stream();
            format!("{on}: {bounds}")
        })
        .collect();
    assert_eq!(
        nesting,
        ["Field", "ListField", "Model", "ModelList", "Nested"]
            .map(|on| format!("{on}: where E : Nests < Root , N , Kind > ,"))
    );
}

/// The loop that joins the keys of an operator is written once, and each merge calls it.
#[cfg(feature = "mongodb")]
#[test]
fn an_update_and_an_arm_update_merge_through_one_function() {
    let packed: String = super::query::query_items()
        .to_string()
        .split_whitespace()
        .collect();
    assert_eq!(packed.matches("existing.insert(key,value);").count(), 1);
    assert_eq!(packed.matches("fnmerged_update(").count(), 1);
    for written in [
        // `Update::and`.
        "Self::raw(Self::merged_update(self.document,other.into()))",
        // `ArmFilter::update`: the model's keys at the arm's place, and no condition.
        "ArmUpdate{tag:self.tag,value:self.value,place:self.place,\
         document:nested_update(self.place.key(),change.into()),of:::core::marker::PhantomData,}",
        // `ArmUpdate::and`.
        "self.document=Update::<Root>::merged_update(self.document,\
         nested_update(self.place.key(),change.into()));",
        // `tag_on_insert`: the tag under its own key, at the enum's level.
        "tagged.insert(self.tag,bson::Bson::String(self.value.to_owned()));",
        "written.insert(\"$setOnInsert\",bson::Bson::Document(tagged));\
         self.document=Update::<Root>::merged_update(self.document,written);",
    ] {
        assert!(packed.contains(written), "missing `{written}`");
    }
}

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

#[test]
fn the_operation_error_is_declared_under_mongodb_alone() {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    let declared: Vec<(Vec<String>, Vec<String>)> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Enum(declared) = item
                && declared.ident == "OperationError"
            {
                Some((
                    declared
                        .attrs
                        .iter()
                        .filter(|attribute| !attribute.path().is_ident("doc"))
                        .map(|attribute| attribute.meta.to_token_stream().to_string())
                        .collect(),
                    declared
                        .variants
                        .iter()
                        .map(|variant| {
                            let (named, held) = (&variant.ident, &variant.fields);
                            quote::quote!(#named #held).to_string()
                        })
                        .collect(),
                ))
            } else {
                None
            }
        })
        .collect();
    let implemented: Vec<String> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Impl(block) = item
                && block.self_ty == syn::parse_quote!(OperationError)
            {
                block
                    .trait_
                    .as_ref()
                    .map(|(implemented, _for)| implemented.to_token_stream().to_string())
            } else {
                None
            }
        })
        .collect();
    if cfg!(feature = "mongodb") {
        assert_eq!(
            declared,
            [(
                vec!["derive (Debug)".to_owned(), "non_exhaustive".to_owned()],
                vec![
                    "Database (mongodb :: error :: Error)".to_owned(),
                    "Unreadable { row : :: std :: string :: String , issues : :: std :: vec :: Vec \
                     < Issue < bson :: Bson > > }"
                        .to_owned(),
                    "Unwritable (WriteError)".to_owned(),
                ]
            )]
        );
        assert_eq!(
            implemented,
            [":: core :: fmt :: Display", ":: std :: error :: Error"]
        );
    } else {
        assert!(declared.is_empty(), "got: {declared:?}");
        assert!(implemented.is_empty(), "got: {implemented:?}");
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn the_operation_error_tells_each_failure_and_answers_its_source() {
    let emitted = super::operations::error_items().to_string();
    for written in [
        "Self :: Database (refused) => write ! (f , \"MongoDB refused the operation or could not \
         be reached: {refused}\") ,",
        "Self :: Unreadable { row , issues } => { write ! (f , \"the row {row} does not read as \
         expected: \") ? ; :: core :: fmt :: Display :: fmt (& Unrecovered { issues : issues . \
         clone () } , f) }",
        "Self :: Unwritable (refused) => write ! (f , \"a value could not be written as BSON: \
         {refused}\") ,",
        "fn source (& self) -> :: core :: option :: Option < & (dyn :: std :: error :: Error + \
         'static) > { match self { Self :: Database (refused) => :: core :: option :: Option :: \
         Some (refused) , Self :: Unreadable { .. } => :: core :: option :: Option :: None , Self \
         :: Unwritable (refused) => :: core :: option :: Option :: Some (refused) , } }",
    ] {
        assert!(
            emitted.contains(written),
            "missing `{written}` in: {emitted}"
        );
    }
    assert!(!emitted.contains("From <"), "got: {emitted}");
}

#[cfg(feature = "mongodb")]
#[test]
fn a_read_and_a_count_take_any_filter_over_the_types_rows_and_answer_a_read() {
    let filtered = "where F : :: core :: convert :: Into < bson :: Document > + :: core :: \
                    convert :: AsRef < :: core :: marker :: PhantomData < Self > > ,";
    let plain = |named: &str, answers: &str| {
        format!(
            "pub fn {named} < F > (collection : & mongodb :: Collection < bson :: Document > , \
             filter : F ,) -> named_schema :: Read < '_ , Self , {answers} > {filtered}"
        )
    };
    let resolved = |named: &str, answers: &str| {
        format!(
            "pub fn {named} < 'c , F > (collection : & 'c mongodb :: Collection < bson :: \
             Document > , filter : F , resolvers : & 'c [named_schema :: Resolver < 'c , bson :: \
             Document , bson :: Bson >] ,) -> named_schema :: Read < 'c , Self , {answers} > \
             {filtered}"
        )
    };
    let one = ":: core :: option :: Option < Self >";
    let every = ":: std :: vec :: Vec < Self >";
    assert_eq!(
        operation_signatures()[..5],
        [
            plain("find_one", one),
            resolved("find_one_with", one),
            plain("find", every),
            resolved("find_with", every),
            plain("count", "u64"),
        ]
    );
}

#[cfg(feature = "mongodb")]
#[test]
fn a_write_answers_what_the_drivers_own_method_answers() {
    let filtered = "F : :: core :: convert :: Into < bson :: Document > + :: core :: convert :: \
                    AsRef < :: core :: marker :: PhantomData < Self > > ,";
    let update = |named: &str| {
        format!(
            "pub async fn {named} < F , U > (collection : & mongodb :: Collection < bson :: \
             Document > , filter : F , update : U ,) -> :: core :: result :: Result < mongodb :: \
             results :: UpdateResult , named_schema :: OperationError > where {filtered} U : :: \
             core :: convert :: Into < bson :: Document > + :: core :: convert :: AsRef < :: core \
             :: marker :: PhantomData < fn (Self) -> Self > > ,"
        )
    };
    let delete = |named: &str| {
        format!(
            "pub async fn {named} < F > (collection : & mongodb :: Collection < bson :: Document \
             > , filter : F ,) -> :: core :: result :: Result < mongodb :: results :: \
             DeleteResult , named_schema :: OperationError > where {filtered}"
        )
    };
    assert_eq!(
        operation_signatures()[5..],
        [
            "pub fn insert_one < 'c > (& self , collection : & 'c mongodb :: Collection < bson :: \
             Document > ,) -> :: core :: pin :: Pin < :: std :: boxed :: Box < dyn :: core :: \
             future :: Future < Output = :: core :: result :: Result < mongodb :: results :: \
             InsertOneResult , named_schema :: OperationError > > + :: core :: marker :: Send + 'c \
             > >"
            .to_owned(),
            update("update_one"),
            update("update_many"),
            delete("delete_one"),
            delete("delete_many"),
            "fn mongo_read_row (row : bson :: Document , resolvers : & [named_schema :: Resolver \
             < '_ , bson :: Document , bson :: Bson >] ,) -> :: core :: result :: Result < Self , \
             named_schema :: OperationError >"
                .to_owned(),
            "fn mongo_written_row (& self) -> :: core :: result :: Result < bson :: Document , \
             named_schema :: OperationError >"
                .to_owned(),
        ]
    );
}

#[cfg(feature = "mongodb")]
#[test]
fn a_read_is_built_over_the_read_of_one_stored_row() {
    let filtered = ":: Read :: new (collection , :: core :: convert :: Into :: < bson :: Document \
                    > :: into (filter) ,";
    for (source, type_impl) in impls_of_every_shape() {
        for (plain, twin) in [("find_one", "find_one_with"), ("find", "find_with")] {
            assert_eq!(
                body_of(type_impl.clone(), plain),
                format!("{{ Self :: {twin} (collection , filter , & []) }}"),
                "for {source}"
            );
            let built = body_of(type_impl.clone(), twin);
            assert!(
                built.contains(filtered)
                    && built.ends_with(" resolvers , Self :: mongo_read_row) }"),
                "for {source}, got: {built}"
            );
        }
        let counted = body_of(type_impl.clone(), "count");
        assert!(
            counted.contains(filtered) && counted.ends_with(" & [] , Self :: mongo_read_row) }"),
            "for {source}, got: {counted}"
        );
        let row = body_of(type_impl.clone(), "mongo_read_row");
        assert!(
            row.starts_with(
                "{ let id = row . get (\"_id\") . map_or_else (| | :: std :: string :: String :: \
                 from (\"without an _id\") , :: std :: string :: ToString :: to_string) ; < Self as "
            ) && row.contains(
                "_schema :: DecodeWith > :: from_bson_piped (row , resolvers) . map_err (| refused |"
            ) && row.ends_with(
                ":: OperationError :: Unreadable { row : id , issues : refused . issues }) }"
            ),
            "for {source}, got: {row}"
        );
        for operation in OPERATIONS {
            let body = body_of(type_impl.clone(), operation);
            assert!(!body.contains("eserialize"), "for {source}, got: {body}");
        }
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_write_hands_the_driver_documents_and_an_insert_writes_its_row_first() {
    let as_document = ":: core :: convert :: Into :: < bson :: Document > :: into";
    let refused = ":: OperationError :: Database) }";
    let refused_as = ":: WriteError as :: serde :: ser :: Error > :: custom (\"a row is stored as \
                      a document, and this value is not written as one\" ,) ,)) , } }";
    for (source, type_impl) in impls_of_every_shape() {
        let inserted = body_of(type_impl.clone(), "insert_one");
        assert!(
            inserted.starts_with(
                "{ let written = self . mongo_written_row () ; :: std :: boxed :: Box :: pin \
                 (async move { collection . insert_one (written ?) . await . map_err ("
            ) && inserted.ends_with(":: OperationError :: Database) }) }"),
            "for {source}, got: {inserted}"
        );
        for updated in ["update_one", "update_many"] {
            let body = body_of(type_impl.clone(), updated);
            assert!(
                body.starts_with(&format!(
                    "{{ collection . {updated} ({as_document} (filter) , {as_document} (update)) \
                     . await . map_err ("
                )) && body.ends_with(refused),
                "for {source}, got: {body}"
            );
        }
        for deleted in ["delete_one", "delete_many"] {
            let body = body_of(type_impl.clone(), deleted);
            assert!(
                body.starts_with(&format!(
                    "{{ collection . {deleted} ({as_document} (filter)) . await . map_err ("
                )) && body.ends_with(refused),
                "for {source}, got: {body}"
            );
        }
        let written = body_of(type_impl, "mongo_written_row");
        assert!(
            written.starts_with(
                "{ match :: serde :: Serialize :: serialize (self , bson :: Serializer :: new ()) \
                 . map_err ("
            ) && written.contains(
                ":: OperationError :: Unwritable) ? { bson :: Bson :: Document (row) => :: core \
                 :: result :: Result :: Ok (row) , _ => :: core :: result :: Result :: Err ("
            ) && written.ends_with(refused_as),
            "for {source}, got: {written}"
        );
    }
}

#[test]
fn the_read_is_declared_under_mongodb_alone() {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    let declared: Vec<(Vec<String>, String, bool)> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Struct(declared) = item
                && declared.ident == "Read"
            {
                Some((
                    declared
                        .attrs
                        .iter()
                        .filter(|attribute| !attribute.path().is_ident("doc"))
                        .map(|attribute| attribute.meta.to_token_stream().to_string())
                        .collect(),
                    declared.generics.to_token_stream().to_string(),
                    declared
                        .fields
                        .iter()
                        .all(|member| matches!(member.vis, syn::Visibility::Inherited)),
                ))
            } else {
                None
            }
        })
        .collect();
    let implemented: Vec<String> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Impl(block) = item
                && let syn::Type::Path(own) = &*block.self_ty
                && own.path.segments.last()?.ident == "Read"
                && let Some((implemented, _for)) = &block.trait_
            {
                let on = &block.self_ty;
                Some(quote::quote!(#implemented for #on).to_string())
            } else {
                None
            }
        })
        .collect();
    if cfg!(feature = "mongodb") {
        assert_eq!(
            declared,
            [(
                vec![
                    "must_use = \"a read asks nothing of MongoDB until it is awaited\"".to_owned(),
                    "non_exhaustive".to_owned(),
                ],
                "< 'c , T , O >".to_owned(),
                true,
            )]
        );
        assert_eq!(
            implemented,
            [
                ":: core :: fmt :: Debug for Read < '_ , T , O >",
                ":: core :: future :: IntoFuture for Read < 'c , T , :: std :: vec :: Vec < T > >",
                ":: core :: future :: IntoFuture for Read < 'c , T , :: core :: option :: Option < \
                 T > >",
                ":: core :: future :: IntoFuture for Read < 'c , T , u64 >",
            ]
        );
    } else {
        assert!(declared.is_empty(), "got: {declared:?}");
        assert!(implemented.is_empty(), "got: {implemented:?}");
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_read_takes_five_options_each_as_the_drivers_own_takes_it() {
    let added: syn::File = syn::parse2(super::operations::read_items()).unwrap();
    let signatures: Vec<String> = query_methods(&added)
        .into_iter()
        .map(|(on, method)| {
            let (visibility, signature) = (&method.vis, &method.sig);
            format!("{on}: {}", quote::quote!(#visibility #signature))
        })
        .collect();
    assert_eq!(
        signatures,
        [
            "Read: pub fn new (collection : & 'c mongodb :: Collection < bson :: Document > , \
             filter : bson :: Document , resolvers : & 'c [Resolver < 'c , bson :: Document , \
             bson :: Bson >] , read : fn (bson :: Document , & [Resolver < '_ , bson :: Document \
             , bson :: Bson >]) -> :: core :: result :: Result < T , OperationError > ,) -> Self",
            "Read: pub fn sort (mut self , sort : bson :: Document) -> Self",
            "Read: pub fn limit (mut self , limit : i64) -> Self",
            "Read: pub fn skip (mut self , skip : u64) -> Self",
            "Read: pub fn hint (mut self , hint : mongodb :: options :: Hint) -> Self",
            "Read: pub fn collation (mut self , collation : mongodb :: options :: Collation) -> \
             Self",
        ]
    );
    let written = added.to_token_stream().to_string();
    for option in ["sort", "limit", "skip", "hint", "collation"] {
        assert!(
            written.contains(&format!(
                "{{ self . {option} = :: core :: option :: Option :: Some ({option}) ; self }}"
            )),
            "for {option}, got: {written}"
        );
    }
    assert!(!written.contains("projection"), "got: {written}");
}

/// What awaits a read is a boxed future that is `Send`, written with standard names alone.
#[cfg(feature = "mongodb")]
#[test]
fn each_answer_awaits_the_drivers_own_method_under_the_options_it_has() {
    let written = super::operations::read_items().to_string();
    let awaited: Vec<&str> = written
        .split("impl < 'c , T > :: core :: future :: IntoFuture for Read < 'c , T , ")
        .skip(1)
        .collect();
    let expected = [
        (
            ":: std :: vec :: Vec < T > > where T : :: core :: marker :: Send + 'c , {",
            "find",
            vec!["sort", "limit", "skip", "hint", "collation"],
        ),
        (
            ":: core :: option :: Option < T > > where T : 'c , {",
            "find_one",
            vec!["sort", "skip", "hint", "collation"],
        ),
        (
            "u64 > where T : 'c , {",
            "count_documents",
            vec!["limit", "skip", "hint", "collation"],
        ),
    ];
    assert_eq!(awaited.len(), expected.len(), "got: {written}");
    for (block, (answer, asked, sent)) in awaited.into_iter().zip(expected) {
        assert!(block.starts_with(answer), "for {asked}, got: {block}");
        for through in [
            "type IntoFuture = :: core :: pin :: Pin < :: std :: boxed :: Box < dyn :: core :: \
             future :: Future < Output = Self :: Output > + :: core :: marker :: Send + 'c >> ;"
                .to_owned(),
            "fn into_future (self) -> Self :: IntoFuture { :: std :: boxed :: Box :: pin (async \
             move {"
                .to_owned(),
            format!("let mut asked = self . collection . {asked} (self . filter) ;"),
        ] {
            assert!(block.contains(&through), "for {asked}, got: {block}");
        }
        let handed: Vec<&str> = block
            .split("asked = asked . ")
            .skip(1)
            .filter_map(|rest| rest.split(' ').next())
            .collect();
        assert_eq!(handed, sent, "for {asked}");
    }
    assert!(
        written.contains(
            "self . limit . map (:: core :: primitive :: i64 :: unsigned_abs) . filter (| limit | \
             * limit != 0) { asked = asked . limit (limit) ; }"
        ),
        "got: {written}"
    );
}

#[cfg(feature = "mongodb")]
#[test]
fn a_read_of_many_rows_reads_the_cursor_row_by_row_and_ends_at_the_first_refused() {
    let written = super::operations::read_items().to_string();
    for through in [
        "let mut cursor = asked . await . map_err (OperationError :: Database) ? ; let mut rows = \
         :: std :: vec :: Vec :: new () ; while cursor . advance () . await . map_err \
         (OperationError :: Database) ? { let row = cursor . deserialize_current () . map_err \
         (OperationError :: Database) ? ; rows . push ((self . read) (row , self . resolvers) ?) \
         ; } :: core :: result :: Result :: Ok (rows) })",
        "let found = asked . await . map_err (OperationError :: Database) ? ; found . map (| row \
         | (self . read) (row , self . resolvers)) . transpose () })",
        "{ asked = asked . collation (collation) ; } asked . await . map_err (OperationError :: \
         Database) })",
    ] {
        assert!(
            written.contains(through),
            "missing `{through}` in: {written}"
        );
    }
    for absent in ["futures", "Stream", "try_collect", "next ("] {
        assert!(!written.contains(absent), "found `{absent}` in: {written}");
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn the_operations_name_only_what_both_majors_of_the_bson_library_have() {
    let module: proc_macro2::Ident = syn::parse_quote!(row_schema);
    let filter: proc_macro2::Ident = syn::parse_quote!(F);
    let update: proc_macro2::Ident = syn::parse_quote!(U);
    let mut emitted = super::operations::error_items();
    emitted.extend(super::operations::read_items());
    emitted.extend(super::operations::methods(&module, &filter, &update));
    let written = emitted.to_string();
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
    assert_eq!(
        named_after("mongodb :: "),
        ["Collection", "error", "options", "results"]
    );
    assert_eq!(named_after("mongodb :: error :: "), ["Error"]);
    assert_eq!(named_after("mongodb :: options :: "), ["Collation", "Hint"]);
    assert_eq!(
        named_after("mongodb :: results :: "),
        ["DeleteResult", "InsertOneResult", "UpdateResult"]
    );
    assert_eq!(named_after("bson :: "), ["Bson", "Document", "Serializer"]);
    assert_eq!(named_after("bson :: Bson :: "), ["Document"]);
    assert_eq!(named_after("bson :: Serializer :: "), ["new"]);
    for absent in [
        "doc !",
        "# [allow",
        "# [expect",
        "doc (hidden)",
        ": impl ",
        "futures",
        "tokio",
    ] {
        assert!(!written.contains(absent), "found `{absent}` in: {written}");
    }
    for inline in ["< F :", "< U :", "< T :", "< O :"] {
        assert!(!written.contains(inline), "found `{inline}` in: {written}");
    }
    assert_eq!(
        path_roots(emitted),
        ["bson", "core", "mongodb", "row_schema", "serde", "std"],
        "a path starts at the type's own module or at a crate a consumer already lists"
    );
}

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

#[cfg(all(feature = "bson", not(feature = "mongodb")))]
#[test]
fn without_mongodb_the_bson_walker_reaches_an_id_as_a_model_type() {
    let walk = bson_fields_walk_of(EVERY_WALK);
    assert!(
        walk.contains(
            "for (key , item) in entries { < ObjectId > :: decode_with_bson_issues (item , & [path , & [:: core :: result :: Result :: Ok (\"owners\" . to_owned ()) , :: core :: result :: Result :: Ok (key . clone ())]] . concat () , issue , out) ; }"
        ),
        "got: {walk}"
    );
    assert!(
        !walk.contains("< ObjectId as serde :: Deserialize >"),
        "got: {walk}"
    );
}

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

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn every_type_the_flag_adds_is_a_name_read_past() {
    // The module of a type that lists "mongodb" holds every type of both libraries through the
    // re-export, beside the types written for it alone.
    let library_types = declared_types(module_items());
    #[cfg(feature = "mongodb")]
    let reexported = [DECODE_WITH_LIBRARY_NAMES, MONGODB_LIBRARY_NAMES].concat();
    #[cfg(not(feature = "mongodb"))]
    let reexported = DECODE_WITH_LIBRARY_NAMES.to_vec();
    for library_type in &library_types {
        assert!(
            reexported.contains(&library_type.as_str()),
            "`{library_type}` is not re-exported"
        );
    }
    let mut own = read_whole_items(Source::GENERATED);
    own.extend(entry_of_items());
    own.extend(paths_module_items("pub struct Named { pub title: String }"));
    own.extend(paths_module_items("pub struct Pair(pub String, pub u32);"));
    own.extend(enum_paths_module_items(INTERNAL));
    let mut declared = library_types;
    declared.extend(declared_types(own));
    declared.sort();
    // Each struct and the enum declare their own `MongoFields`.
    declared.dedup();
    assert_eq!(declared, ADDED_TYPE_NAMES);
}

/// The types, the traits and the aliases `items` declare at their top.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn declared_types(items: proc_macro2::TokenStream) -> Vec<String> {
    let added: syn::File = syn::parse2(items).unwrap();
    added
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
        .collect()
}

/// Each `{type}_schema` module re-exports every name the `decode_with` library makes public, and
/// nothing else of it: what stays private there is read by the library alone.
#[test]
fn the_decode_with_library_names_are_every_name_it_makes_public() {
    let mut listed = DECODE_WITH_LIBRARY_NAMES.to_vec();
    listed.sort_unstable();
    assert_eq!(public_names(decode_with_library()), listed);
    let private: Vec<String> = declared_names(decode_with_library())
        .into_iter()
        .filter_map(|(name, public)| (!public).then_some(name))
        .collect();
    assert_eq!(
        private,
        [
            "expected_from_tokens",
            "value_digits_as_numbers",
            #[cfg(feature = "bson")]
            "bson_digits_as_numbers",
        ]
    );
}

/// The module of a type that lists "mongodb" re-exports every name the MongoDB library makes
/// public.
#[cfg(feature = "mongodb")]
#[test]
fn the_mongodb_library_names_are_every_name_it_makes_public() {
    let mut listed = MONGODB_LIBRARY_NAMES.to_vec();
    listed.sort_unstable();
    assert_eq!(public_names(mongodb_library()), listed);
    let private: Vec<String> = declared_names(mongodb_library())
        .into_iter()
        .filter_map(|(name, public)| (!public).then_some(name))
        .collect();
    assert_eq!(
        private,
        [
            "write_plain",
            "nested_key",
            "nested_filter",
            "nested_update",
        ]
    );
}

/// The two libraries share no name: a module of a type that lists "mongodb" re-exports both.
#[cfg(feature = "mongodb")]
#[test]
fn the_two_libraries_share_no_name() {
    for name in MONGODB_LIBRARY_NAMES {
        assert!(!DECODE_WITH_LIBRARY_NAMES.contains(name), "{name}");
    }
}

/// The `decode_with` library names no `bson` path in a build without `bson`, so a crate whose
/// types list `"decode_with"` alone needs no `bson` there.
#[cfg(not(feature = "bson"))]
#[test]
fn without_bson_the_decode_with_library_names_no_bson_path() {
    let written = decode_with_foundation().to_string();
    assert!(!written.contains("bson"), "got: {written}");
}

/// Under `bson` the `decode_with` library holds what reads a BSON value: what holds a BSON value
/// to a bound, the leaf and its helpers, and `Stored` for a document. It holds nothing of the
/// MongoDB layer.
#[cfg(feature = "bson")]
#[test]
fn under_bson_the_decode_with_library_holds_what_reads_a_bson_value() {
    for name in ["bson_bound", "bson_leaf", "bson_remaining", "same_bracket"] {
        assert!(DECODE_WITH_LIBRARY_NAMES.contains(&name), "{name}");
    }
    let written = decode_with_library().to_string();
    for declared in [
        "pub fn bson_bound <",
        "pub fn same_bracket (",
        "pub fn bson_leaf <",
        "pub fn bson_remaining <",
        "impl Stored for bson :: Document",
    ] {
        assert!(
            written.contains(declared),
            "missing `{declared}` in: {written}"
        );
    }
    for absent in ["Filter", "Update", "OperationError", "Read", "MongoPath"] {
        assert!(
            !written.contains(&format!("pub struct {absent}"))
                && !written.contains(&format!("pub enum {absent}")),
            "found `{absent}` in: {written}"
        );
    }
}

/// The attributes on each item an initializer writes at the crate root, as text, by the item's
/// position: a module, a trait, an `impl`, a macro or a `use`, and any other item as itself.
fn crate_root_attributes(written: &syn::File) -> Vec<String> {
    written
        .items
        .iter()
        .map(|item| {
            let attributes = if let syn::Item::Mod(declared) = item {
                Some(&declared.attrs)
            } else if let syn::Item::Trait(declared) = item {
                Some(&declared.attrs)
            } else if let syn::Item::Impl(declared) = item {
                Some(&declared.attrs)
            } else if let syn::Item::Macro(declared) = item {
                Some(&declared.attrs)
            } else if let syn::Item::Use(declared) = item {
                Some(&declared.attrs)
            } else {
                None
            };
            attributes.map_or_else(
                || format!("unexpected item: {}", item.to_token_stream()),
                |held| {
                    held.iter()
                        .map(|attribute| attribute.to_token_stream().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                },
            )
        })
        .collect()
}

/// Each initializer writes its module and the marker its probe finds, at the crate root. The
/// module alone is kept out of the consumer's documentation.
#[test]
fn each_initializer_writes_its_module_and_its_marker() {
    let written: syn::File = syn::parse2(decode_with_foundation()).unwrap();
    let declared: Vec<String> = declared_names(written.to_token_stream())
        .into_iter()
        .map(|(name, public)| {
            assert!(!public, "{name} is public");
            name
        })
        .collect();
    assert_eq!(
        declared,
        ["__tixschema_decode_with", "__TixschemaDecodeWithFoundation"]
    );
    assert_eq!(
        crate_root_attributes(&written),
        ["# [doc (hidden)]", "", "", "", ""]
    );
    #[cfg(feature = "mongodb")]
    {
        let second: syn::File = syn::parse2(mongodb_foundation()).unwrap();
        assert_eq!(
            declared_names(second.to_token_stream()),
            [
                ("__tixschema_mongodb".to_owned(), false),
                ("__TixschemaMongodbFoundation".to_owned(), false),
            ]
        );
        assert_eq!(crate_root_attributes(&second), ["# [doc (hidden)]", "", ""]);
        assert!(
            mongodb_foundation()
                .to_string()
                .contains("mod __tixschema_mongodb { use crate :: __tixschema_decode_with :: * ;"),
            "got: {}",
            mongodb_foundation()
        );
    }
}

/// A build without `mongodb` writes no MongoDB library, so the call writes nothing.
#[cfg(not(feature = "mongodb"))]
#[test]
fn the_mongodb_initializer_writes_nothing_without_mongodb() {
    assert!(super::mongodb_foundation().is_empty());
}

/// A type re-exports the `decode_with` library by name, and the MongoDB one only where it lists
/// "mongodb", under `mongodb`; it carries no copy of either. Each re-export expects
/// `clippy::pub_use`, and nothing else the type adds carries an `allow`, an `expect` or a
/// `doc(hidden)`.
#[test]
fn a_type_reexports_its_crates_libraries_by_name() {
    let item: syn::ItemStruct = syn::parse_str("pub struct Lookup { pub number: String }").unwrap();
    let names = |listed: &[&str]| listed.join(" , ");
    let expected = "# [expect (clippy :: pub_use , reason = \"tixschema re-exports the crate's \
                    support library in each schema module by name, so every \
                    `{type}_schema::Name` path stays reachable from this crate and from \
                    another\")]";
    for mongodb in [false, true] {
        let added = super::struct_recovering_decode(&item, mongodb);
        let module = added.schema_module.to_string();
        assert!(
            module.contains(&format!(
                "{expected} pub use crate :: __tixschema_decode_with :: {{ {} }} ;",
                names(DECODE_WITH_LIBRARY_NAMES)
            )),
            "got: {module}"
        );
        #[cfg(feature = "mongodb")]
        assert_eq!(
            module.contains(&format!(
                "{expected} pub use crate :: __tixschema_mongodb :: {{ {} }} ;",
                names(MONGODB_LIBRARY_NAMES)
            )),
            mongodb,
            "got: {module}"
        );
        for copied in ["pub enum Issue", "pub struct Path", "pub fn value_leaf"] {
            assert!(!module.contains(copied), "found `{copied}` in: {module}");
        }
        let reexports = if mongodb && cfg!(feature = "mongodb") {
            2
        } else {
            1
        };
        assert_eq!(
            module.matches("# [expect").count(),
            reexports,
            "got: {module}"
        );
        assert_eq!(
            module.matches("pub use").count(),
            reexports,
            "got: {module}"
        );
        let everything = [
            module,
            added.type_impl.to_string(),
            added.probes.to_string(),
            added.check_messages.to_string(),
        ]
        .concat();
        for quieting in ["# [allow", "doc (hidden)"] {
            assert!(
                !everything.contains(quieting),
                "found `{quieting}` in: {everything}"
            );
        }
        assert_eq!(everything.matches("# [expect").count(), reexports);
        assert_eq!(
            added.type_impl.to_string().contains("fn from_bson_with"),
            cfg!(feature = "bson"),
        );
    }
}

/// Every type re-exports the `decode_with` library and probes for it; a type that lists "mongodb"
/// probes for the MongoDB library as well, under `mongodb`.
#[test]
fn a_type_probes_for_the_libraries_it_reexports() {
    let item: syn::ItemStruct = syn::parse_str("pub struct Lookup { pub number: String }").unwrap();
    for mongodb in [false, true] {
        let written = super::struct_recovering_decode(&item, mongodb)
            .probes
            .to_string();
        assert_eq!(
            written
                .matches("const _ : () = { use crate :: * ; struct Missing ;")
                .count(),
            if mongodb && cfg!(feature = "mongodb") {
                2
            } else {
                1
            },
            "got: {written}"
        );
        assert!(
            written.contains(
                "# [diagnostic :: on_unimplemented (message = \"tixschema: this crate has \
                 `#[model_schema(enable = [\\\"decode_with\\\"])]` types but no \
                 `tixschema::decode_with_foundation!();`\" , label = \"this type needs the \
                 decode_with support library, which `tixschema::decode_with_foundation!();` \
                 writes once per crate\" , note = \"Fix: add the line \
                 `tixschema::decode_with_foundation!();` at the crate root (src/lib.rs or \
                 src/main.rs), on its own line, anywhere outside a function. Add it once per \
                 crate; nothing else is needed.\")]"
            ),
            "got: {written}"
        );
        assert!(
            written.contains("need ((& ()) . __tixschema_decode_with_foundation ()) ;"),
            "got: {written}"
        );
        assert_eq!(
            written.contains("need ((& ()) . __tixschema_mongodb_foundation ()) ;"),
            mongodb && cfg!(feature = "mongodb"),
            "got: {written}"
        );
    }
}

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
        named.push(LISTED_READ.to_owned());
        if cfg!(feature = "bson") {
            named.extend(methods_of("bson"));
        }
        named.extend(paths_members());
        named.extend(reads_of("value"));
        if cfg!(feature = "bson") {
            named.extend(reads_of("bson"));
        }
        let header = format!("impl {}", source.split([' ', '(', ';']).nth(2).unwrap());
        let mut impls = vec![(header, named)];
        // The types of a tuple's slots are filled beside the type.
        if cfg!(feature = "mongodb") && source.contains(", ") {
            impls.push((
                "impl < Root > pair_schema :: MongoSlots < Root > for ()".to_owned(),
                vec!["Slot0".to_owned(), "Slot1".to_owned()],
            ));
        }
        assert_eq!(added_impls(source), impls, "for {source}");
    }
}

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

#[test]
fn a_tuple_struct_is_walked_by_position() {
    let walk = issues_walk_of("pub struct Pair(pub String, pub Inner);");
    for written in [
        "let :: core :: option :: Option :: Some (items) = found . as_array () else { out . extend \
         (pair_schema :: value_leaf (found , Self :: decode_with_read_unjudged , \
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
                "{ out . extend (code_schema :: value_leaf (found , Self :: decode_with_read_unjudged , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & ["
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
        named.push(LISTED_READ.to_owned());
        if cfg!(feature = "bson") {
            named.extend(methods_of("bson"));
        }
        named.extend(paths_members());
        named.extend(reads_of("value"));
        if cfg!(feature = "bson") {
            named.extend(reads_of("bson"));
        }
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

#[test]
fn a_unit_struct_lists_every_key_and_its_fields_walker_binds_nothing() {
    let emitted = type_impl_of("pub struct Ping;");
    for written in [
        "let :: core :: option :: Option :: Some (object) = found . as_object () else { out . \
         extend (ping_schema :: value_leaf (found , Self :: decode_with_read_unjudged ,",
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

/// A bound the container writes for the read is the only one the parameter takes, written once
/// beside the type's own, and the source joins nothing of its own to the parameter.
#[test]
fn a_deserialize_bound_the_container_writes_is_the_only_one_the_parameter_takes() {
    let header_of = |bound: &str, at: usize| {
        added_impls(&format!(
            "#[serde(bound(deserialize = \"{bound}\"))] \
             pub struct Crate<CodeType: Display> {{ pub code: CodeType }}"
        ))
        .into_iter()
        .nth(at)
        .map(|(header, _)| header)
    };
    let written_once = |predicate: &str| {
        format!(
            "impl < CodeType : Display > Crate < CodeType > \
             where Self : serde :: de :: DeserializeOwned , {predicate}"
        )
    };
    for (written, predicate) in [
        (
            "CodeType: serde::de::DeserializeOwned",
            "CodeType : serde :: de :: DeserializeOwned",
        ),
        ("CodeType: DeserializeOwned", "CodeType : DeserializeOwned"),
        (
            "CodeType: serde::Deserialize<'de>",
            "for < 'de > CodeType : serde :: Deserialize < 'de >",
        ),
        (
            "CodeType: serde::de::DeserializeOwned + Clone",
            "CodeType : serde :: de :: DeserializeOwned + Clone",
        ),
    ] {
        assert_eq!(
            header_of(written, 0),
            Some(written_once(predicate)),
            "for `{written}`"
        );
    }
    if cfg!(feature = "bson") {
        assert_eq!(
            header_of("CodeType: serde::de::DeserializeOwned", 1).as_deref(),
            Some(
                "impl < CodeType : Display + serde :: Serialize > Crate < CodeType > \
                 where Self : serde :: de :: DeserializeOwned + serde :: Serialize , \
                 CodeType : serde :: de :: DeserializeOwned"
            )
        );
    }
}

/// A serde bound a parameter declares, on its name or in its `where` predicate and in whatever
/// path, is written once in each header that asks it of every parameter.
#[test]
fn a_bound_a_parameter_declares_is_joined_once() {
    let headers_of = |declaration: &str| -> Vec<String> {
        added_impls(declaration)
            .into_iter()
            .map(|(header, _)| header)
            .collect()
    };
    let mut on_its_name = vec![
        "impl < CodeType : Display + Serialize + serde :: de :: DeserializeOwned > \
         Crate < CodeType > where Self : serde :: de :: DeserializeOwned",
    ];
    let mut read_declared = vec![
        "impl < CodeType : Display + DeserializeOwned > Crate < CodeType > \
         where Self : serde :: de :: DeserializeOwned",
    ];
    let mut in_where = vec![
        "impl < CodeType > Crate < CodeType > \
         where CodeType : Display + serde :: Serialize + serde :: de :: DeserializeOwned , \
         Self : serde :: de :: DeserializeOwned",
    ];
    let mut in_both = vec![
        "impl < CodeType > Crate < CodeType > \
         where CodeType : Display + Serialize + serde :: de :: DeserializeOwned , \
         Self : serde :: de :: DeserializeOwned",
    ];
    if cfg!(feature = "bson") {
        on_its_name.push(
            "impl < CodeType : Display + Serialize + serde :: de :: DeserializeOwned > \
             Crate < CodeType > \
             where Self : serde :: de :: DeserializeOwned + serde :: Serialize",
        );
        read_declared.push(
            "impl < CodeType : Display + DeserializeOwned + serde :: Serialize > \
             Crate < CodeType > \
             where Self : serde :: de :: DeserializeOwned + serde :: Serialize",
        );
        in_where.push(
            "impl < CodeType > Crate < CodeType > \
             where CodeType : Display + serde :: Serialize + serde :: de :: DeserializeOwned , \
             Self : serde :: de :: DeserializeOwned + serde :: Serialize",
        );
        in_both.push(
            "impl < CodeType > Crate < CodeType > \
             where CodeType : Display + Serialize + serde :: de :: DeserializeOwned , \
             Self : serde :: de :: DeserializeOwned + serde :: Serialize",
        );
    }
    if cfg!(feature = "mongodb") {
        on_its_name.extend([
            "impl < CodeType : Display + Serialize > Crate < CodeType > \
             where Self : :: serde :: Serialize",
            "impl < Root , CodeType : Display + Serialize > \
             crate_schema :: MongoFields < Root , CodeType > \
             where Crate < CodeType > : :: serde :: Serialize",
        ]);
        read_declared.extend([
            "impl < CodeType : Display + DeserializeOwned + :: serde :: Serialize > \
             Crate < CodeType > where Self : :: serde :: Serialize",
            "impl < Root , CodeType : Display + DeserializeOwned + :: serde :: Serialize > \
             crate_schema :: MongoFields < Root , CodeType > \
             where Crate < CodeType > : :: serde :: Serialize",
        ]);
        in_where.extend([
            "impl < CodeType > Crate < CodeType > \
             where CodeType : Display + serde :: Serialize , Self : :: serde :: Serialize",
            "impl < Root , CodeType > crate_schema :: MongoFields < Root , CodeType > \
             where CodeType : Display + serde :: Serialize , \
             Crate < CodeType > : :: serde :: Serialize",
        ]);
        in_both.extend([
            "impl < CodeType > Crate < CodeType > \
             where CodeType : Display + Serialize , Self : :: serde :: Serialize",
            "impl < Root , CodeType > crate_schema :: MongoFields < Root , CodeType > \
             where CodeType : Display + Serialize , Crate < CodeType > : :: serde :: Serialize",
        ]);
    }
    assert_eq!(
        headers_of("pub struct Crate<CodeType: Display + Serialize> { pub code: CodeType }"),
        on_its_name
    );
    assert_eq!(
        headers_of("pub struct Crate<CodeType: Display + DeserializeOwned> { pub code: CodeType }"),
        read_declared
    );
    assert_eq!(
        headers_of(
            "pub struct Crate<CodeType> where CodeType: Display + serde::Serialize \
             { pub code: CodeType }"
        ),
        in_where
    );
    assert_eq!(
        headers_of(
            "pub struct Crate<CodeType: Serialize> where CodeType: Display + Serialize \
             { pub code: CodeType }"
        ),
        in_both
    );
}

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
        [
            methods_of("value"),
            vec![LISTED_READ.to_owned()],
            if cfg!(feature = "bson") {
                Vec::new()
            } else {
                reads_of("value")
            },
        ]
        .concat(),
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
            [methods_of("bson"), reads_of("value"), reads_of("bson")].concat(),
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
        headers.push((
            "impl < Root , T : Clone + :: serde :: Serialize , const N : usize , U , V , \
             W : :: serde :: Serialize > page_schema :: MongoFields < Root , T , N , U , V , W > \
             where U : Copy + :: serde :: Serialize , V : Sync + Send + :: serde :: Serialize , \
             Vec < W > : Clone , Page < T , N , U , V , W > : :: serde :: Serialize"
                .to_owned(),
            vec!["items".to_owned(), "other".to_owned()],
        ));
    }
    assert_eq!(added, headers);
}

#[test]
fn a_methods_own_type_parameter_is_never_one_the_type_declares() {
    let clashing =
        whole_impl_of("pub struct Keyed<I, const F: usize, F2> { pub id: I, pub kept: F2 }");
    for written in [
        "fn from_value_with < F3 > (mut value : serde_json :: Value , decide : F3 ,)",
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
    let plain = whole_impl_of("pub struct Page<T> { pub items: Vec<T> }");
    for written in [
        "fn from_value_with < F > (",
        "pub fn decode_with_value_issues < I > (",
        "pub fn decode_with_value_fields < 'a , I > (",
    ] {
        assert!(plain.contains(written), "missing `{written}` in: {plain}");
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn an_operations_filter_and_update_are_never_names_the_item_writes() {
    for (source, named) in [
        ("pub struct Holder { pub unit: U }", ["F", "I", "U2"]),
        ("pub struct U { pub name: String }", ["F", "I", "U2"]),
        (
            "pub struct Holder { pub frame: F, pub unit: U, pub other: U2 }",
            ["F2", "I", "U3"],
        ),
    ] {
        let item: syn::ItemStruct = syn::parse_str(source).unwrap();
        let type_impl = struct_recovering_decode(&item).type_impl;
        assert_eq!(
            own_type_parameters(type_impl.clone()),
            [&["D"], named.as_slice()].concat(),
            "for {source}"
        );
        let [filter, _issue, update] = named;
        let emitted = type_impl.to_string();
        for written in [
            format!("pub async fn update_one < {filter} , {update} > ("),
            format!("filter : {filter} , update : {update} ,)"),
            format!("pub async fn delete_many < {filter} > ("),
            format!("pub fn find_with < 'c , {filter} > ("),
        ] {
            assert!(
                emitted.contains(&written),
                "for {source}, missing `{written}` in: {emitted}"
            );
        }
    }
    let carried: syn::ItemEnum = syn::parse_str("pub enum Carried { Held(U), Unit }").unwrap();
    assert_eq!(
        own_type_parameters(enum_recovering_decode(&carried).type_impl),
        ["D", "F", "I", "U2"]
    );
}

#[test]
fn the_reader_of_the_unjudged_read_is_never_a_name_the_item_writes() {
    let docked: syn::ItemStruct = syn::parse_str("pub struct Holder { pub dock: D }").unwrap();
    assert_eq!(
        own_type_parameters(struct_recovering_decode(&docked).type_impl)
            .first()
            .map(String::as_str),
        Some("D2")
    );
}

#[test]
fn a_methods_own_type_parameter_is_never_a_name_the_item_writes() {
    // `D` is the reader the unjudged read is handed.
    let expected = |[frame, issue]: [&str; 2]| {
        let mut every: Vec<String> = ["D", frame, issue].map(str::to_owned).to_vec();
        if cfg!(feature = "mongodb") {
            every.push("U".to_owned());
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
            expected(named),
            "for {source}"
        );
        #[cfg(feature = "mongodb")]
        {
            let names = written_names(item.to_token_stream());
            let paths = super::fields::struct_paths(&item, &names).module_items;
            assert_eq!(row_parameter(paths), root, "for {source}");
        }
        #[cfg(not(feature = "mongodb"))]
        assert!(root.starts_with("Root"), "for {source}");
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
            expected(named),
            "for {source}"
        );
        #[cfg(feature = "mongodb")]
        {
            let names = written_names(item.to_token_stream());
            let paths = super::fields::enum_paths(&item, &names).module_items;
            assert_eq!(row_parameter(paths), root, "for {source}");
        }
        #[cfg(not(feature = "mongodb"))]
        assert!(root.starts_with("Root"), "for {source}");
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

#[test]
fn a_type_with_no_type_parameter_keeps_one_impl_under_its_own_generics() {
    let source = "pub struct Grid<const N: usize> { pub cells: Vec<u8> }";
    let headers: Vec<String> = added_impls(source)
        .into_iter()
        .map(|(header, _methods)| header)
        .collect();
    let mut expected = vec!["impl < const N : usize > Grid < N >"];
    // The functions of its struct of paths are in an `impl` of that struct, beside the type.
    if cfg!(feature = "mongodb") {
        expected.push("impl < Root , const N : usize > grid_schema :: MongoFields < Root , N >");
    }
    assert_eq!(headers, expected);
    assert!(type_impl_of(source).contains(
        "Self :: decode_with_read_unjudged , | read | serde_json :: to_value (read) . ok () ,"
    ));
}

#[test]
fn a_parameters_value_and_the_type_itself_are_never_written_back_from_json() {
    let source = "pub struct Parcel<T> { pub both: (T, u32), pub extra: HashMap<String, T>, \
                  #[serde(deserialize_with = \"lenient\")] pub hooked: T, pub inner: Vec<Self>, \
                  pub items: Vec<T>, pub one: T, pub page: Page<T>, pub projected: T::Item, \
                  pub sender: Option<T>, pub total: u32 }";
    let emitted = type_impl_of(source);
    let (json, _bson) = emitted
        .split_once("fn decode_with_bson_report")
        .unwrap_or((emitted.as_str(), ""));
    for written in [
        "parcel_schema :: value_leaf (found , Self :: decode_with_read_unjudged , \
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

#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_of_a_generic_type_writes_every_value_back() {
    let emitted = type_impl_of("pub struct Page<T> { pub items: Vec<T>, pub total: u32 }");
    let (_json, bson) = emitted.split_once("fn decode_with_bson_report").unwrap();
    for written in [
        "page_schema :: bson_leaf (found , Self :: decode_with_read_unjudged , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,",
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
            "let bson :: Bson :: Array (items) = found else { out . extend (pair_schema :: bson_leaf (found , Self :: decode_with_read_unjudged , | read , to | serde :: Serialize :: serialize (read , to) . ok () , path . to_vec () , & [(\"Model\" , & [\"Pair\"] , 0)] , issue)) ; return ; } ;",
        ),
        (
            "pub struct Code(pub String);",
            "{ out . extend (code_schema :: bson_leaf (found , Self :: decode_with_read_unjudged , | read , to | serde :: Serialize :: serialize (read , to) . ok () \
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
        let (_json, bson) = emitted.split_once("fn decode_with_bson_report").unwrap();
        assert!(
            bson.contains(written),
            "for {source}, missing `{written}` in: {bson}"
        );
        assert!(!bson.contains("serde_json"), "for {source}, got: {bson}");
    }
}

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
        named.push(LISTED_READ.to_owned());
        if cfg!(feature = "bson") {
            named.extend(of_source("bson"));
        }
        named.extend(paths_members());
        named.extend(reads_of("value"));
        if cfg!(feature = "bson") {
            named.extend(reads_of("bson"));
        }
        let (_attributes, declared) = source.split_once("pub enum ").unwrap();
        let name = declared.split(' ').next().unwrap();
        let mut impls = vec![(format!("impl {name}"), named)];
        let beside = PATHS_BESIDE
            .iter()
            .find(|(form, _impls)| *form == source)
            .map_or(&[][..], |(_form, written)| *written);
        if cfg!(feature = "mongodb") {
            impls.extend(beside.iter().map(|(header, members)| {
                (
                    (*header).to_owned(),
                    members.iter().map(|member| (*member).to_owned()).collect(),
                )
            }));
        }
        assert_eq!(added_enum_impls(source), impls, "for {source}");
    }
}

/// A plain enum is the name serde writes: one value, read with the enum's own reader.
#[test]
fn a_plain_enum_is_read_whole_and_one_under_a_tag_is_walked_by_its_tag() {
    let plain = enum_json_of("pub enum Status { Draft, Published }");
    assert!(
        plain.contains(
            "out : & mut :: std :: vec :: Vec < I > ,) { out . extend (status_schema :: value_leaf \
             (found , Self :: decode_with_read_unjudged , | read | serde_json :: \
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

#[test]
fn an_externally_tagged_enum_is_walked_under_the_key_naming_the_variant() {
    let walk = enum_json_of(EXTERNAL);
    for written in [
        "match found { serde_json :: Value :: String (tag) if matches ! (tag . as_str () , \"Empty\") => { } serde_json :: Value :: Object (object) if object . len () == 1 && object . keys () . any (| key | matches ! (key . as_str () , \"Circle\" | \"Label\" | \"Pinned\" | \"To\")) => { Self :: decode_with_value_fields (object , path , issue , out) ; } _ => out . extend (outline_schema :: value_leaf (found , Self :: decode_with_read_unjudged , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"Variants\" , & [\"Circle\" , \"Empty\" , \"Label\" , \"Pinned\" , \"To\"] , 0)] , issue)) , }",
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

#[test]
fn an_internally_tagged_enum_walks_the_variant_its_tag_names_in_the_same_object() {
    let walk = enum_json_of(INTERNAL);
    for written in [
        "let :: core :: option :: Option :: Some (object) = found . as_object () else { out . \
         extend (fill_schema :: value_leaf (found , Self :: decode_with_read_unjudged , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & \
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
         . concat () ; out . push (match Self :: decode_with_read_unjudged \
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

/// serde reads what an internally tagged variant holds from the whole object.
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

#[test]
fn an_untagged_enum_is_walked_as_the_variant_serde_reads() {
    let walk = enum_json_of(UNTAGGED);
    for written in [
        "match Self :: decode_with_read_unjudged (found) { :: core :: result :: \
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
         else { out . extend (contact_schema :: value_leaf (found , Self :: decode_with_read_unjudged , | read | serde_json :: to_value (read) . ok () , path . \
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

/// A generic enum gets one `impl` per source under the bounds a generic struct gets.
#[test]
fn a_generic_enum_gets_its_methods_under_each_sources_bounds() {
    let source = "pub enum Answer<T> { Empty, Value(T) }";
    let mut headers = vec![(
        "impl < T : serde :: de :: DeserializeOwned > Answer < T > \
         where Self : serde :: de :: DeserializeOwned"
            .to_owned(),
        [
            methods_of("value"),
            vec![LISTED_READ.to_owned()],
            if cfg!(feature = "bson") {
                Vec::new()
            } else {
                reads_of("value")
            },
        ]
        .concat(),
    )];
    if cfg!(feature = "bson") {
        headers.push((
            "impl < T : serde :: de :: DeserializeOwned + serde :: Serialize > Answer < T > \
             where Self : serde :: de :: DeserializeOwned + serde :: Serialize"
                .to_owned(),
            [methods_of("bson"), reads_of("value"), reads_of("bson")].concat(),
        ));
    }
    if cfg!(feature = "mongodb") {
        headers.push((
            "impl < T : :: serde :: Serialize > Answer < T > where Self : :: serde :: Serialize"
                .to_owned(),
            paths_members(),
        ));
        headers.push((
            "impl < Root , T : :: serde :: Serialize > answer_schema :: MongoFields < Root , T > \
             where Answer < T > : :: serde :: Serialize"
                .to_owned(),
            ["value", "is_empty", "is_value"]
                .map(str::to_owned)
                .to_vec(),
        ));
    }
    assert_eq!(added_enum_impls(source), headers);
    let walk = enum_json_of(source);
    for written in [
        "_ => out . extend (answer_schema :: value_leaf (found , Self :: decode_with_read_unjudged , | _ | :: core :: option :: Option :: None , path . to_vec () , & \
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

#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_of_each_enum_form_matches_the_librarys_own_types() {
    for (source, written) in [
        (
            EXTERNAL,
            "match found { bson :: Bson :: String (tag) if matches ! (tag . as_str () , \"Empty\") => { } bson :: Bson :: Document (object) if object . len () == 1 && object . keys () . any (| key | matches ! (key . as_str () , \"Circle\" | \"Label\" | \"Pinned\" | \"To\")) => { Self :: decode_with_bson_fields (object , path , issue , out) ; } _ => out . extend (outline_schema :: bson_leaf (found , Self :: decode_with_read_unjudged , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,",
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
            "out . push (match Self :: decode_with_read_unjudged (bson :: \
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
            "match Self :: decode_with_read_unjudged (bson :: Deserializer :: new \
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
        // The typed paths name every variant serde writes, one it never reads back among them.
        let walkers = emitted
            .split_once("# [doc = r\" The typed MongoDB paths of this type")
            .map_or(emitted.as_str(), |(walkers, _paths)| walkers);
        assert!(
            !walkers.contains(never_read),
            "for {source}, got: {walkers}"
        );
    }
}

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
    named.push(LISTED_READ.to_owned());
    if cfg!(feature = "bson") {
        named.extend(of_source("bson"));
    }
    named.extend(paths_members());
    named.extend(reads_of("value"));
    if cfg!(feature = "bson") {
        named.extend(reads_of("bson"));
    }
    let mut impls = vec![("impl Reach".to_owned(), named)];
    if cfg!(feature = "mongodb") {
        impls.extend([
            (
                "impl < Root > reach_schema :: mongo_members :: Email < Root >".to_owned(),
                vec!["address".to_owned()],
            ),
            (
                "impl < Root > reach_schema :: MongoFields < Root >".to_owned(),
                ["email", "fax", "word"].map(str::to_owned).to_vec(),
            ),
        ]);
    }
    assert_eq!(added_enum_impls(UNREAD_UNTAGGED), impls);
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

/// serde hands a flattened type the entries the outer type's own fields did not take.
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
}

/// `decode_with_value_named` as it is emitted, answering `answer`.
fn value_named(answer: &str) -> String {
    format!(
        "pub fn decode_with_value_named (object : & serde_json :: Map < :: std :: string :: String , \
         serde_json :: Value >) -> bool {{ {answer} }}"
    )
}

/// Every flagged shape answers whether an object holds what names a value of it.
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
            "pub struct Open { #[serde(flatten)] pub extra: HashMap<String, i32> }",
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
             String , serde_json :: Value >) -> bool { Self :: decode_with_read_unjudged (serde_json :: Value :: Object (object . clone ())) . is_ok () }",
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
        let (_json, bson) = emitted.split_once("fn decode_with_bson_report").unwrap();
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
            "pub fn decode_with_bson_named (object : & bson :: Document) -> bool { Self :: decode_with_read_unjudged (bson :: Deserializer :: new (bson :: Bson :: Document (object . clone ()))) . is_ok () }",
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

/// serde reads the flattened fields in the order declared, each from what the ones before it left.
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
        "pub struct Hooked { #[serde(flatten, with = \"as_text\")] pub first: Audit, \
         #[serde(flatten)] pub counts: Counts }",
    );
    assert!(
        hooked.contains(
            "let taken = hooked_schema :: value_remaining :: < Audit , _ > (as_text :: \
             deserialize , & hooked_schema :: Asked :: default () , object) ; let remaining = \
             taken . as_ref () . unwrap_or (object) ; declared . extend (< Counts > :: \
             decode_with_value_fields (remaining , path , issue , out)"
        ),
        "got: {hooked}"
    );
    for source in [
        "pub struct One { #[serde(flatten)] pub counts: Counts, pub id: String }",
        "pub struct Unread { #[serde(flatten, skip_deserializing)] pub cached: Audit, \
         #[serde(flatten)] pub counts: Counts }",
    ] {
        let walk = json_fields_walk_of(source);
        assert!(!walk.contains("_remaining"), "for {source}, got: {walk}");
    }
}

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
             (object . clone ()) ; out . extend (wrap_schema :: value_leaf (found , Self :: decode_with_read_unjudged , | _ | :: core :: option :: Option :: None , \
             path . to_vec () , & [(\"TypeParam\" , & [\"T\"] , 0)] , issue) . filter (| _ | ! \
             wrap_schema :: reads_an_option :: < Self , _ > (Self :: decode_with_read_unjudged , & wrap_schema :: Asked :: default ()))) ; object . keys () . map \
             (:: std :: string :: String :: as_str) . collect () }",
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
             (object . clone ()) ; out . extend (stamp_schema :: value_leaf (found , Self :: decode_with_read_unjudged , | _ | :: core :: option :: Option :: None , \
             path . to_vec () , & [(\"Model\" , & [\"Inner\"] , 0)] , issue) . filter (| _ | ! \
             stamp_schema :: reads_an_option :: < Self , _ > (Self :: decode_with_read_unjudged , & stamp_schema :: Asked :: default ()))) ; object . keys () . map \
             (:: std :: string :: String :: as_str) . collect () }",
        ),
        (
            "#[serde(transparent)] pub struct Stamp { #[serde(with = \"as_text\")] pub inner: HashMap<String, i32> }",
            "-> :: std :: vec :: Vec < & 'a str > { let found = & serde_json :: Value :: Object \
             (object . clone ()) ; out . extend (stamp_schema :: value_leaf (found , Self :: decode_with_read_unjudged , | _ | :: core :: option :: Option :: None , \
             path . to_vec () , & [(\"Map\" , & [] , 1) , (\"I32\" , & [] , 0)] , issue) . \
             filter (| _ | ! stamp_schema :: reads_an_option :: < Self , _ > (Self :: decode_with_read_unjudged , & stamp_schema :: Asked :: default ()))) ; \
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

/// An id is the object serde writes for it, so serde flattens a single-slot struct over one.
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

#[test]
fn a_flattened_map_alone_walks_the_value_of_every_key() {
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
}

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
    assert!(!hooked.contains("_left"), "got: {hooked}");
}

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
}

#[test]
fn an_untagged_enums_fields_walker_walks_the_variant_serde_reads_the_object_as() {
    let emitted = enum_json_of(UNTAGGED);
    let (_issues, walk) = emitted
        .split_once("pub fn decode_with_value_fields")
        .unwrap();
    for written in [
        "-> :: std :: vec :: Vec < & 'a str > { match Self :: decode_with_read_unjudged (serde_json :: Value :: Object (object . clone ())) { :: core :: result :: \
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
        "-> :: std :: vec :: Vec < & 'a str > { match Self :: decode_with_read_unjudged (bson :: Deserializer :: new (bson :: Bson :: Document (object . clone \
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

#[test]
fn a_field_of_a_type_not_seen_above_is_asked_with_the_whole_read_in_scope() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Page { pub properties: Properties }").unwrap();
    let unseen = super::struct_recovering_decode(&item, true);
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

#[test]
fn a_field_typed_with_an_alias_seen_above_is_walked_as_the_type_the_alias_names() {
    record_declared(
        "Marks",
        Declared::Alias("BTreeMap < Tier , Vec < Mark > >".to_owned()),
    );
    record_declared("Mark", Declared::model(0));
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Tally { pub marks: types::Marks }").unwrap();
    let added = super::struct_recovering_decode(&item, true);
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

#[cfg(feature = "indexmap")]
#[test]
fn an_alias_of_an_index_map_or_an_index_set_is_walked_as_the_type_indexmap_declares() {
    record_declared(
        "Ranked",
        Declared::Alias("IndexMap < String , IndexSet < Mark > >".to_owned()),
    );
    record_declared("Mark", Declared::model(0));
    let item: syn::ItemStruct = syn::parse_str("pub struct Tally { pub ranked: Ranked }").unwrap();
    let added = super::struct_recovering_decode(&item, true);
    let (walker, module) = (added.type_impl.to_string(), added.schema_module.to_string());
    for written in [
        "const _ : fn (Ranked) -> indexmap :: IndexMap < :: std :: string :: String , indexmap :: \
         IndexSet < tally_schema :: decode_with_at0 :: Mark > > = | held | held ;",
        "< tally_schema :: decode_with_at0 :: Mark > :: decode_with_value_issues (",
    ] {
        assert!(walker.contains(written), "missing `{written}` in: {walker}");
    }
    let reached = "pub mod decode_with_at0 { pub type Mark = < << super :: super :: Ranked as :: \
                   core :: iter :: IntoIterator > :: Item as super :: EntryOf > :: Value as :: core \
                   :: iter :: IntoIterator > :: Item ; }";
    assert!(module.contains(reached), "missing `{reached}` in: {module}");
}

/// With the feature off the two names are types of the author's, which no walk goes into.
#[cfg(not(feature = "indexmap"))]
#[test]
fn an_alias_of_a_name_the_indexmap_feature_reads_is_left_as_it_is_written_without_it() {
    for (alias, stands_for) in [
        ("Ranked", "IndexMap < String , i32 >"),
        ("Listed", "IndexSet < i32 >"),
    ] {
        record_declared(alias, Declared::Alias(stands_for.to_owned()));
        let item: syn::ItemStruct =
            syn::parse_str(&format!("pub struct Tally {{ pub held: {alias} }}")).unwrap();
        let added = super::struct_recovering_decode(&item, true);
        let walker = added.type_impl.to_string();
        assert!(!walker.contains("const _"), "for {alias}, got: {walker}");
        assert!(!walker.contains("indexmap"), "for {alias}, got: {walker}");
    }
}

#[test]
fn a_field_typed_with_an_alias_of_a_model_type_is_left_as_it_is_written() {
    record_declared("Featured", Declared::Alias("Entry".to_owned()));
    record_declared("Entry", Declared::model(0));
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Shelf { pub featured: Featured }").unwrap();
    let added = super::struct_recovering_decode(&item, true);
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
/// goes into its module, then what goes into its `impl` and beside it. Each name of `seen` is a
/// struct or an enum `#[model_schema]` was written on above it, as the struct's own name is by
/// then.
#[cfg(feature = "mongodb")]
fn typed_paths_of(source: &str, seen: &[&str]) -> (String, String) {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    record_declared(&item.ident.to_string(), Declared::model(0));
    for name in seen {
        record_declared(name, Declared::model(0));
    }
    let paths = super::fields::struct_paths(&item, &written_names(item.to_token_stream()));
    let (in_the_impl, beside) = (paths.type_items, paths.beside);
    (
        paths.module_items.to_string().split_whitespace().collect(),
        quote::quote!(#in_the_impl #beside)
            .to_string()
            .split_whitespace()
            .collect(),
    )
}

/// [`typed_paths_of`] for the enum `source` declares.
#[cfg(feature = "mongodb")]
fn typed_enum_paths_of(source: &str, seen: &[&str]) -> (String, String) {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    record_declared(&item.ident.to_string(), Declared::model(0));
    for name in seen {
        record_declared(name, Declared::model(0));
    }
    let paths = super::fields::enum_paths(&item, &written_names(item.to_token_stream()));
    let (in_the_impl, beside) = (paths.type_items, paths.beside);
    (
        paths.module_items.to_string().split_whitespace().collect(),
        quote::quote!(#in_the_impl #beside)
            .to_string()
            .split_whitespace()
            .collect(),
    )
}

/// The function `named` of a struct of paths of `Row`, in its `impl` beside the type, answering
/// `kind` at `key`.
#[cfg(feature = "mongodb")]
fn path_function(named: &str, kind: &str, of: &str, key: &str) -> String {
    format!(
        "pubconstfn{named}(&self)->row_schema::{kind}<Root,{of}>\
         {{row_schema::{kind}::plain(row_schema::MongoPath::at(\"{key}\"))}}"
    )
}

#[cfg(feature = "mongodb")]
#[test]
fn a_field_is_the_function_that_answers_the_kind_serde_writes_it_as() {
    for (field, answered) in [
        ("String", "row_schema::Field<Root,String>"),
        ("Option<u8>", "row_schema::OptionalField<Root,u8>"),
        ("Vec<u8>", "row_schema::ListField<Root,u8>"),
        ("[u8; 4]", "row_schema::ListField<Root,u8>"),
        ("HashSet<u8>", "row_schema::ListField<Root,u8>"),
        #[cfg(feature = "indexmap")]
        ("IndexSet<u8>", "row_schema::ListField<Root,u8>"),
        #[cfg(feature = "indexmap")]
        (
            "IndexSet<u8, RandomState>",
            "row_schema::ListField<Root,u8>",
        ),
        ("Inner", "row_schema::Model<Root,Inner>"),
        ("Option<Inner>", "row_schema::OptionalModel<Root,Inner>"),
        ("Vec<Inner>", "row_schema::ModelList<Root,Inner>"),
        ("Wrapper<Inner>", "row_schema::Model<Root,Wrapper<Inner>>"),
        ("Option<Vec<u8>>", "row_schema::OptionalField<Root,Vec<u8>>"),
        ("Vec<Vec<Inner>>", "row_schema::ListField<Root,Vec<Inner>>"),
        ("(u8, Inner)", "row_schema::Field<Root,(u8,Inner)>"),
    ] {
        let source = format!("pub struct Row {{ pub a: {field} }}");
        let (_module, on_the_type) = typed_paths_of(&source, &["Inner", "Wrapper"]);
        assert!(
            on_the_type.contains(&format!("pubconstfna(&self)->{answered}{{")),
            "for {field}, got: {on_the_type}"
        );
    }
    let (module, on_the_type) = typed_paths_of("pub struct Row { pub a: Inner }", &["Inner"]);
    assert!(
        on_the_type.contains(&path_function("a", "Model", "Inner", "a")),
        "got: {on_the_type}"
    );
    // The schema module declares the struct, and names no type of the author's.
    assert!(
        !module.contains("Inner") && !module.contains("fna("),
        "got: {module}"
    );
    // The struct holds a marker and no path, so the type's const builds it from nothing, and
    // neither names anything another type's expansion wrote.
    assert!(
        module.contains("pubrow:::core::marker::PhantomData<fn()->(Root,)>,}"),
        "got: {module}"
    );
    assert!(
        on_the_type.contains(
            "pubconstMONGO_FIELDS:row_schema::MongoFields<Self>=row_schema::MongoFields{row:::core::marker::PhantomData};"
        ),
        "got: {on_the_type}"
    );
    let emitted = format!("{module}{on_the_type}");
    for absent in ["inner_schema", "prefix"] {
        assert!(!emitted.contains(absent), "found `{absent}` in: {emitted}");
    }
}

/// A wrapper serde writes as the value it holds is the value an operator takes, and what it
/// holds is the model `nest` reaches.
#[cfg(feature = "mongodb")]
#[test]
fn a_wrapped_value_names_the_model_nest_reaches_after_the_value_it_takes() {
    for (field, answered) in [
        ("Box<Inner>", "row_schema::Model<Root,Box<Inner>,Inner>"),
        (
            "Option<Box<Inner>>",
            "row_schema::OptionalModel<Root,Box<Inner>,Inner>",
        ),
        (
            "Vec<Arc<Inner>>",
            "row_schema::ModelList<Root,Arc<Inner>,Inner>",
        ),
        (
            "Option<Box<Row>>",
            "row_schema::OptionalField<Root,Box<Row>,Row>",
        ),
        ("Box<Unseen>", "row_schema::Field<Root,Box<Unseen>,Unseen>"),
        ("Box<str>", "row_schema::Field<Root,Box<str>,str>"),
        (
            "Cow<'static, str>",
            "row_schema::Field<Root,Cow<'static,str>,str>",
        ),
    ] {
        let source = format!("pub struct Row {{ pub a: {field} }}");
        let (_module, on_the_type) = typed_paths_of(&source, &["Inner"]);
        assert!(
            on_the_type.contains(&format!("pubconstfna(&self)->{answered}{{")),
            "for {field}, got: {on_the_type}"
        );
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_type_not_seen_above_is_one_whole_value_named_as_its_author_wrote_it() {
    for (field, answered) in [
        ("Unseen", "row_schema::Field<Root,Unseen>"),
        ("Option<Unseen>", "row_schema::OptionalField<Root,Unseen>"),
        ("Vec<Self>", "row_schema::ListField<Root,Row>"),
        ("other::Thing", "row_schema::Field<Root,other::Thing>"),
        (
            "self::other::Thing",
            "row_schema::Field<Root,self::other::Thing>",
        ),
        ("super::Thing", "row_schema::Field<Root,super::Thing>"),
        ("crate::Thing", "row_schema::Field<Root,crate::Thing>"),
        (
            "::std::string::String",
            "row_schema::Field<Root,::std::string::String>",
        ),
        ("DateTime<Utc>", "row_schema::Field<Root,DateTime<Utc>>"),
    ] {
        let source = format!("pub struct Row {{ pub a: {field} }}");
        let (_module, on_the_type) = typed_paths_of(&source, &[]);
        assert!(
            on_the_type.contains(&format!("pubconstfna(&self)->{answered}{{")),
            "for {field}, got: {on_the_type}"
        );
        assert!(
            !on_the_type.contains("Model"),
            "for {field}, got: {on_the_type}"
        );
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_key_serde_never_writes_and_a_map_have_no_function() {
    for field in [
        "#[serde(skip)] pub a: u8",
        "#[serde(skip_serializing)] pub a: u8",
        "pub a: HashMap<String, Inner>",
        "pub a: BTreeMap<String, u8>",
        #[cfg(feature = "indexmap")]
        "pub a: IndexMap<String, Inner>",
        #[cfg(feature = "indexmap")]
        "pub a: IndexMap<String, u8, RandomState>",
        #[cfg(feature = "indexmap")]
        "pub a: Option<IndexMap<String, u8>>",
        "pub a: Option<HashMap<String, u8>>",
        "#[serde(flatten)] pub a: Unseen",
        "#[serde(flatten, with = \"hook\")] pub a: Inner",
    ] {
        let source = format!("pub struct Row {{ {field}, pub b: u8 }}");
        let (_module, on_the_type) = typed_paths_of(&source, &["Inner"]);
        assert!(
            on_the_type.contains(&path_function("b", "Field", "u8", "b")),
            "for {field}, got: {on_the_type}"
        );
        assert_eq!(
            on_the_type.matches("pubconstfn").count(),
            1,
            "for {field}, got: {on_the_type}"
        );
    }
}

/// A flagged type is a model only where its walker is called as it stands.
#[cfg(feature = "mongodb")]
#[test]
fn a_field_serde_never_reads_back_as_it_stands_is_one_whole_value() {
    for (field, answered, built) in [
        (
            "#[serde(skip_deserializing)] pub a: Inner",
            "row_schema::Field<Root,Inner>",
            "row_schema::Field::plain(row_schema::MongoPath::at(\"a\"))",
        ),
        (
            "#[serde(deserialize_with = \"read\")] pub a: Option<Inner>",
            "row_schema::OptionalField<Root,Inner>",
            "row_schema::OptionalField::plain(row_schema::MongoPath::at(\"a\"))",
        ),
    ] {
        let source = format!("pub struct Row {{ {field} }}");
        let (_module, on_the_type) = typed_paths_of(&source, &["Inner"]);
        assert!(
            on_the_type.contains(&format!("pubconstfna(&self)->{answered}{{{built}}}")),
            "for {field}, got: {on_the_type}"
        );
    }
}

/// A hook is called by a function of the type's own. The function of a path that writes through
/// one is in the `impl` beside the type, as every function of a struct of paths is, which names the
/// type where its schema module could not.
#[cfg(feature = "mongodb")]
#[test]
fn a_hooked_path_is_a_function_written_beside_the_type() {
    for (field, answered, kind) in [
        (
            "#[serde(with = \"hook\")] pub a: Inner",
            "row_schema::Field<Root,Inner>",
            "Field",
        ),
        (
            "#[serde(serialize_with = \"shown\")] pub a: Vec<u8>",
            "row_schema::Field<Root,Vec<u8>>",
            "Field",
        ),
        (
            "#[serde(with = \"hook\")] pub a: Option<u8>",
            "row_schema::OptionalField<Root,u8>",
            "OptionalField",
        ),
    ] {
        let source = format!("pub struct Row {{ {field}, pub b: u8 }}");
        let (module, on_the_type) = typed_paths_of(&source, &["Inner"]);
        let written = format!(
            "pubconstfna(&self)->{answered}\
             {{row_schema::{kind}::hooked(row_schema::MongoPath::at(\"a\"),<Row>::mongo_write_a)}}"
        );
        assert!(
            on_the_type.contains("impl<Root>row_schema::MongoFields<Root>{")
                && on_the_type.contains(&written),
            "for {field}, missing `{written}` in: {on_the_type}"
        );
        assert!(
            on_the_type.contains(&path_function("b", "Field", "u8", "b"))
                && !module.contains("fna("),
            "for {field}, got: {module}"
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
    // The slot of a tuple is built in the type's own `impl`, and what a variant holds beside it.
    let (tuple, on_the_tuple) = typed_paths_of(
        "pub struct Row<T>(#[serde(with = \"hook\")] pub u8, pub T);",
        &[],
    );
    assert!(!tuple.contains("mongo_write_0"), "got: {tuple}");
    assert!(
        on_the_tuple.contains(
            "row_schema::Field::hooked(row_schema::MongoPath::at(\"0\"),<Row<T>>::mongo_write_0)"
        ),
        "got: {on_the_tuple}"
    );
    let (generic, on_the_generic) = typed_enum_paths_of(
        "pub enum Row<T> { Kept { #[serde(with = \"hook\")] at: u8, by: T }, \
         Pair(#[serde(with = \"hook\")] u8, T) }",
        &[],
    );
    for written in [
        "impl<Root,T:::serde::Serialize>row_schema::mongo_members::Kept<Root,T>whereRow<T>:::serde::Serialize{",
        "pubconstfnat(&self)->row_schema::Field<Root,u8>{row_schema::Field::hooked(\
         row_schema::MongoPath::at(\"Kept.at\"),<Row<T>>::mongo_write_kept_at)}",
        "impl<Root,T:::serde::Serialize>row_schema::MongoFields<Root,T>whereRow<T>:::serde::Serialize{",
        "pubconstfnpair(&self)->row_schema::mongo_members::Pair<Root,T>{row_schema::mongo_members::Pair(\
         row_schema::Field::hooked(row_schema::MongoPath::at(\"Pair.0\"),<Row<T>>::mongo_write_pair_0),\
         row_schema::Field::plain(row_schema::MongoPath::at(\"Pair.1\")))}",
    ] {
        assert!(
            on_the_generic.contains(written),
            "missing `{written}` in: {on_the_generic}"
        );
    }
    // What calls no hook is in the same `impl` beside the type.
    for written in [
        "pubconstfnkept(&self)->row_schema::mongo_members::Kept<Root,T>{",
        "pubconstfnby(&self)->row_schema::Field<Root,T>{",
    ] {
        assert!(
            on_the_generic.contains(written),
            "missing `{written}` in: {on_the_generic}"
        );
    }
    assert!(!generic.contains("mongo_write"), "got: {generic}");
}

/// Beside the type `Self` is the struct of paths the `impl` is of. What is written there names
/// the type where its author wrote `Self`: the value of a hooked path, and a bound of its own.
#[cfg(feature = "mongodb")]
#[test]
fn what_is_written_beside_the_type_names_the_type_where_its_author_wrote_self() {
    for (source, written) in [
        (
            "pub struct Row { #[serde(with = \"hook\")] pub a: Vec<Self> }",
            "impl<Root>row_schema::MongoFields<Root>{",
        ),
        (
            "pub struct Row { #[serde(with = \"hook\")] pub a: Vec<Self> }",
            "pubconstfna(&self)->row_schema::Field<Root,Vec<Row>>{row_schema::Field::hooked(\
             row_schema::MongoPath::at(\"a\"),<Row>::mongo_write_a)}",
        ),
        (
            "pub struct Row { #[serde(with = \"hook\")] pub a: Option<Box<Self>> }",
            "pubconstfna(&self)->row_schema::OptionalField<Root,Box<Row>,Row>{\
             row_schema::OptionalField::hooked(row_schema::MongoPath::at(\"a\"),<Row>::mongo_write_a)}",
        ),
        (
            "pub struct Row<T> { #[serde(with = \"hook\")] pub a: Vec<Self>, pub b: T }",
            "pubconstfna(&self)->row_schema::Field<Root,Vec<Row<T>>>{row_schema::Field::hooked(\
             row_schema::MongoPath::at(\"a\"),<Row<T>>::mongo_write_a)}",
        ),
        (
            "pub struct Row where Self: Marked { #[serde(with = \"hook\")] pub a: u8 }",
            "impl<Root>row_schema::MongoFields<Root>whereRow:Marked{",
        ),
    ] {
        let (_module, on_the_type) = typed_paths_of(source, &[]);
        assert!(
            on_the_type.contains(written),
            "for {source}, missing `{written}` in: {on_the_type}"
        );
    }
    // The function a hooked path writes through is in the type's own `impl`, as its author wrote
    // the value.
    let (_declared, in_its_impl) = typed_paths_of(
        "pub struct Row { #[serde(with = \"hook\")] pub a: Vec<Self> }",
        &[],
    );
    assert!(
        in_its_impl.contains("fnmongo_write_a(value:Vec<Self>)->"),
        "got: {in_its_impl}"
    );
    for (source, written) in [
        (
            "pub enum Row { Kept { #[serde(with = \"hook\")] at: Vec<Self> }, Lost }",
            "impl<Root>row_schema::mongo_members::Kept<Root>{",
        ),
        (
            "pub enum Row { Kept { #[serde(with = \"hook\")] at: Vec<Self> }, Lost }",
            "pubconstfnat(&self)->row_schema::Field<Root,Vec<Row>>{row_schema::Field::hooked(\
             row_schema::MongoPath::at(\"Kept.at\"),<Row>::mongo_write_kept_at)}",
        ),
        // The arms of an enum under a tag are written beside it as well.
        (
            "#[serde(tag = \"kind\")] pub enum Row where Self: Marked { Kept { at: u8 }, Lost }",
            "impl<Root>row_schema::MongoFields<Root>whereRow:Marked{",
        ),
    ] {
        let (_module, on_the_type) = typed_enum_paths_of(source, &[]);
        assert!(
            on_the_type.contains(written),
            "for {source}, missing `{written}` in: {on_the_type}"
        );
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_function_is_named_after_its_field_and_writes_the_key_serde_does() {
    let (_module, on_the_type) = typed_paths_of(
        "#[serde(rename_all = \"camelCase\")] pub struct Row { #[serde(rename = \"_id\")] pub id: u8, \
         pub paid_at: u8, pub r#type: u8, #[serde(flatten)] pub audit: Inner, \
         #[serde(flatten)] pub extra: Option<Box<Inner>> }",
        &["Inner"],
    );
    for written in [
        path_function("id", "Field", "u8", "_id"),
        path_function("paid_at", "Field", "u8", "paidAt"),
        path_function("r#type", "Field", "u8", "type"),
        "pubconstfnaudit(&self)->row_schema::Nested<Root,Inner>\
         {row_schema::Nested::at(row_schema::MongoPath::ROOT)}"
            .to_owned(),
        "pubconstfnextra(&self)->row_schema::Nested<Root,Inner>\
         {row_schema::Nested::at(row_schema::MongoPath::ROOT)}"
            .to_owned(),
    ] {
        assert!(
            on_the_type.contains(&written),
            "missing `{written}` in: {on_the_type}"
        );
    }
}

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
            "pubstructMongoFields<Root,T,Whole>{",
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
        let written = format!(
            "pubconstMONGO_FIELDS:row_schema::MongoFields<Self{arguments},row_schema::Field<Self,Self>>=\
             row_schema::MongoFields::built(row_schema::Field::plain(row_schema::MongoPath::ROOT));"
        );
        assert!(
            on_the_type.contains(&written),
            "for {source}, missing `{written}` in: {on_the_type}"
        );
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

#[cfg(feature = "mongodb")]
#[test]
fn a_type_written_as_one_value_is_a_plain_value_where_another_type_holds_it() {
    record_declared("Brand", Declared::OneValue(0));
    record_declared("Sleeve", Declared::OneValue(0));
    for (field, answered) in [
        ("Brand", "row_schema::Field<Root,Brand>"),
        ("Option<Brand>", "row_schema::OptionalField<Root,Brand>"),
        ("Box<Brand>", "row_schema::Field<Root,Box<Brand>,Brand>"),
        ("Vec<Brand>", "row_schema::ListField<Root,Brand>"),
        ("[Brand; 2]", "row_schema::ListField<Root,Brand>"),
        (
            "Vec<Box<Brand>>",
            "row_schema::ListField<Root,Box<Brand>,Brand>",
        ),
        (
            "Option<Vec<Brand>>",
            "row_schema::OptionalField<Root,Vec<Brand>>",
        ),
        ("Sleeve<Inner>", "row_schema::Field<Root,Sleeve<Inner>>"),
        ("Vec<Inner>", "row_schema::ModelList<Root,Inner>"),
    ] {
        let source = format!("pub struct Row {{ pub a: {field} }}");
        let (_module, on_the_type) = typed_paths_of(&source, &["Inner"]);
        assert!(
            on_the_type.contains(&format!("pubconstfna(&self)->{answered}{{")),
            "for {field}, got: {on_the_type}"
        );
    }
    let (flattened, on_the_flattened) = typed_paths_of(
        "pub struct Row { #[serde(flatten)] pub a: Brand, pub b: u8 }",
        &[],
    );
    assert!(
        on_the_flattened.contains(&path_function("b", "Field", "u8", "b"))
            && !format!("{flattened}{on_the_flattened}").contains("fna("),
        "got: {flattened}{on_the_flattened}"
    );
    let (_in_place, untagged) = typed_enum_paths_of(
        "#[serde(untagged)] pub enum Row { Held(Brand), Listed(Vec<Brand>) }",
        &[],
    );
    for built in [
        "pubconstfnheld(&self)->row_schema::Field<Root,Brand>\
         {row_schema::Field::plain(row_schema::MongoPath::ROOT)}",
        "pubconstfnlisted(&self)->row_schema::ListField<Root,Brand>\
         {row_schema::ListField::plain(row_schema::MongoPath::ROOT)}",
    ] {
        assert!(untagged.contains(built), "missing `{built}` in: {untagged}");
    }
    let (tagged, beside) = typed_enum_paths_of(
        "#[serde(tag = \"kind\")] pub enum Row { Wire(Brand), Cash }",
        &[],
    );
    let emitted = format!("{tagged}{beside}");
    assert!(!emitted.contains("fnwire("), "got: {emitted}");
}

/// A path is under the name serde writes.
#[cfg(feature = "mongodb")]
#[test]
fn a_renaming_written_as_a_list_counts_by_what_serde_writes() {
    let (_on_the_struct, on_the_type) = typed_paths_of(
        "#[serde(rename_all(serialize = \"camelCase\"))] pub struct Row { \
         #[serde(rename(serialize = \"written\", deserialize = \"read\"))] pub both_ways: u8, \
         #[serde(rename(serialize = \"only_written\"))] pub write_way: u8, \
         #[serde(rename(deserialize = \"only_read\"))] pub read_way: u8, \
         #[serde(rename(serialize = \"apart\"))] #[serde(rename(deserialize = \"from\"))] pub two_lists: u8, \
         pub cased_by_rule: u8 }",
        &[],
    );
    for (named, key) in [
        ("both_ways", "written"),
        ("write_way", "only_written"),
        ("read_way", "readWay"),
        ("two_lists", "apart"),
        ("cased_by_rule", "casedByRule"),
    ] {
        let built = path_function(named, "Field", "u8", key);
        assert!(
            on_the_type.contains(&built),
            "missing `{built}` in: {on_the_type}"
        );
    }
    let (_read_rule, on_the_read_rule) = typed_paths_of(
        "#[serde(rename_all(deserialize = \"camelCase\"))] pub struct Row { pub paid_at: u8 }",
        &[],
    );
    assert!(
        on_the_read_rule.contains("row_schema::MongoPath::at(\"paid_at\")"),
        "got: {on_the_read_rule}"
    );
    let (_by_tag, on_the_tagged) = typed_enum_paths_of(
        "#[serde(tag = \"kind\", rename_all(serialize = \"kebab-case\"), \
         rename_all_fields(serialize = \"camelCase\"))] pub enum Row { \
         #[serde(rename(serialize = \"byAir\", deserialize = \"air\"))] Air { flight_code: u8 }, \
         OverLand { road_name: u8 }, \
         #[serde(rename_all(serialize = \"SCREAMING_SNAKE_CASE\"))] Sea { ship_name: u8 } }",
        &[],
    );
    for written in [
        "pubfnis_air(&self)->row_schema::Filter<Root>\
         {row_schema::Filter::held(\"kind\",\"$eq\",bson::Bson::String(\"byAir\".to_owned()))}",
        "pubfnis_over_land(&self)->row_schema::Filter<Root>\
         {row_schema::Filter::held(\"kind\",\"$eq\",bson::Bson::String(\"over-land\".to_owned()))}",
        "pubfnis_sea(&self)->row_schema::Filter<Root>\
         {row_schema::Filter::held(\"kind\",\"$eq\",bson::Bson::String(\"sea\".to_owned()))}",
        "pubconstfnflight_code(&self)->row_schema::Field<Root,u8>\
         {row_schema::Field::plain(row_schema::MongoPath::at(\"flightCode\"))}",
        "pubconstfnroad_name(&self)->row_schema::Field<Root,u8>\
         {row_schema::Field::plain(row_schema::MongoPath::at(\"roadName\"))}",
        "pubconstfnship_name(&self)->row_schema::Field<Root,u8>\
         {row_schema::Field::plain(row_schema::MongoPath::at(\"SHIP_NAME\"))}",
    ] {
        assert!(
            on_the_tagged.contains(written),
            "missing `{written}` in: {on_the_tagged}"
        );
    }
    let (_by_key, on_the_keyed) = typed_enum_paths_of(
        "#[serde(rename_all(serialize = \"kebab-case\"))] pub enum Row { \
         #[serde(rename(serialize = \"byHand\"))] Courier { badge: u8 }, DropBox(u8), NotSent }",
        &[],
    );
    for written in [
        "pubfnis_courier(&self)->row_schema::Filter<Root>\
         {row_schema::Filter::held(\"byHand\",\"$exists\",bson::Bson::Boolean(true))}",
        "row_schema::Field::plain(row_schema::MongoPath::at(\"byHand.badge\"))",
        path_function("drop_box", "Field", "u8", "drop-box").as_str(),
        "pubfnis_not_sent(&self)->row_schema::Filter<Root>\
         {row_schema::Filter::held(\"\",\"$eq\",bson::Bson::String(\"not-sent\".to_owned()))}",
    ] {
        assert!(
            on_the_keyed.contains(written),
            "missing `{written}` in: {on_the_keyed}"
        );
    }
}

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

/// A tuple struct's paths are a tuple struct, each slot's key the position serde writes it at.
/// Each slot's type is read through a trait of the schema module, which the type's own scope
/// fills where every name the slot's type writes resolves.
#[cfg(feature = "mongodb")]
#[test]
fn a_tuple_structs_paths_are_by_position() {
    let (module, on_the_type) = typed_paths_of(
        "pub struct Row(pub f64, #[serde(skip)] pub u8, pub Inner);",
        &["Inner"],
    );
    for written in [
        "pubtraitMongoSlots<Root>{",
        "pubstructMongoFields<Root>(pub<()asself::MongoSlots<Root>>::Slot0,\
         pub<()asself::MongoSlots<Root>>::Slot1,pub<()asself::MongoSlots<Root>>::Slot2)\
         where():self::MongoSlots<Root>;",
    ] {
        assert!(module.contains(written), "missing `{written}` in: {module}");
    }
    assert!(!module.contains("Inner"), "got: {module}");
    for written in [
        "pubconstMONGO_FIELDS:row_schema::MongoFields<Self>=row_schema::MongoFields(\
         row_schema::Field::plain(row_schema::MongoPath::at(\"0\")),(),\
         row_schema::Model::plain(row_schema::MongoPath::at(\"1\")));",
        "impl<Root>row_schema::MongoSlots<Root>for(){typeSlot0=row_schema::Field<Root,f64>;\
         typeSlot1=();typeSlot2=row_schema::Model<Root,Inner>;}",
    ] {
        assert!(
            on_the_type.contains(written),
            "missing `{written}` in: {on_the_type}"
        );
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_struct_of_paths_carries_the_types_own_parameters_and_never_mixes_what_it_keeps() {
    let (bounded, on_the_bounded) = typed_paths_of(
        "pub struct Row<T: Clone + Shown, const N: usize, U = String> where T: Send \
         { pub a: T, pub b: [u8; N], pub c: Option<U> }",
        &[],
    );
    // The module declares the parameters with no bound of the author's.
    for written in [
        "pubstructMongoFields<Root,T,constN:usize,U>{",
        "pubrow:::core::marker::PhantomData<fn()->(Root,T,U,)>,}",
    ] {
        assert!(
            bounded.contains(written),
            "missing `{written}` in: {bounded}"
        );
    }
    for absent in ["Clone", "Shown", "Send", "String", "Serialize"] {
        assert!(!bounded.contains(absent), "found `{absent}` in: {bounded}");
    }
    for written in [
        "pubconstMONGO_FIELDS:row_schema::MongoFields<Self,T,N,U>=\
         row_schema::MongoFields{row:::core::marker::PhantomData};",
        // The functions are under what the type's own `impl` of its paths is under.
        "impl<Root,T,constN:usize,U:::serde::Serialize>row_schema::MongoFields<Root,T,N,U>\
         whereT:Send+Clone+Shown+::serde::Serialize,Row<T,N,U>:::serde::Serialize{",
        "pubconstfna(&self)->row_schema::Field<Root,T>{",
        "pubconstfnb(&self)->row_schema::ListField<Root,u8>{",
        "pubconstfnc(&self)->row_schema::OptionalField<Root,U>{",
    ] {
        assert!(
            on_the_bounded.contains(written),
            "missing `{written}` in: {on_the_bounded}"
        );
    }
    // A struct of functions marks every parameter, so one no member names needs nothing more.
    let (marked, _on_the_marked) = typed_paths_of(
        "pub struct Row<T> { pub a: u8, pub b: HashMap<String, T> }",
        &[],
    );
    assert!(
        marked.contains("pubrow:::core::marker::PhantomData<fn()->(Root,T,)>,}")
            && !marked.contains("mongo_members")
            && !marked.contains("Deref"),
        "got: {marked}"
    );
    // A tuple's slots are public, so a parameter no slot names is marked beside them.
    let (kept, on_the_kept) = typed_paths_of(
        "pub struct Row<T>(pub u8, #[serde(skip)] pub PhantomData<T>, pub u16);",
        &[],
    );
    for written in [
        "pubstructMongoFields<Root,T>where():self::mongo_members::MembersSlots<Root>\
         {members:self::mongo_members::Members<Root>,\
         unused:::core::marker::PhantomData<fn()->(T,)>,}",
        "pubconstfnbuilt(members:self::mongo_members::Members<Root>,)->Self",
        "impl<Root,T>::core::ops::DerefforMongoFields<Root,T>\
         where():self::mongo_members::MembersSlots<Root>\
         {typeTarget=self::mongo_members::Members<Root>;",
        "pubmodmongo_members{usesuper::*;",
        "pubtraitMembersSlots<Root>{",
        "pubstructMembers<Root>(pub<()asself::MembersSlots<Root>>::Slot0,\
         pub<()asself::MembersSlots<Root>>::Slot1,pub<()asself::MembersSlots<Root>>::Slot2)\
         where():self::MembersSlots<Root>;",
    ] {
        assert!(kept.contains(written), "missing `{written}` in: {kept}");
    }
    for written in [
        "pubconstMONGO_FIELDS:row_schema::MongoFields<Self,T>=row_schema::MongoFields::built(\
         row_schema::mongo_members::Members(row_schema::Field::plain(row_schema::MongoPath::at(\"0\")),(),\
         row_schema::Field::plain(row_schema::MongoPath::at(\"1\"))));",
        "impl<Root>row_schema::mongo_members::MembersSlots<Root>for()\
         {typeSlot0=row_schema::Field<Root,u8>;typeSlot1=();typeSlot2=row_schema::Field<Root,u16>;}",
    ] {
        assert!(
            on_the_kept.contains(written),
            "missing `{written}` in: {on_the_kept}"
        );
    }
    let (empty, unread) = typed_paths_of("pub struct Row { pub b: HashMap<String, u8> }", &[]);
    assert!(
        empty.contains("pubrow:::core::marker::PhantomData<fn()->(Root,)>,}")
            && !format!("{empty}{unread}").contains("pubconstfn"),
        "got: {empty}{unread}"
    );
    assert!(
        unread.contains(
            "pubconstMONGO_FIELDS:row_schema::MongoFields<Self>=row_schema::MongoFields{row:::core::marker::PhantomData};"
        ),
        "got: {unread}"
    );
}

/// The `impl` of the slots declares no parameter no slot names, so a bound naming one is left
/// out, inline or in a `where` predicate, and every other bound beside it is kept. A predicate on
/// a type naming one is left out whole.
#[cfg(feature = "mongodb")]
#[test]
fn the_impl_of_the_slots_keeps_no_bound_that_names_a_parameter_no_slot_names() {
    for (source, header) in [
        (
            "pub struct Row<A: Tr<B>, B>(pub A, #[serde(skip)] pub PhantomData<B>, pub u16);",
            "impl<Root,A>row_schema::mongo_members::MembersSlots<Root,A>for()",
        ),
        (
            "pub struct Row<A: Tr<B> + Clone, B>(pub A, #[serde(skip)] pub PhantomData<B>, pub u16);",
            "impl<Root,A:Clone>row_schema::mongo_members::MembersSlots<Root,A>for()",
        ),
        (
            "pub struct Row<A, B>(pub A, #[serde(skip)] pub PhantomData<B>, pub u16) where A: Tr<B> + Clone;",
            "impl<Root,A>row_schema::mongo_members::MembersSlots<Root,A>for()whereA:Clone{",
        ),
        (
            "pub struct Row<A, B>(pub A, #[serde(skip)] pub PhantomData<B>, pub u16) where A: Tr<B>;",
            "impl<Root,A>row_schema::mongo_members::MembersSlots<Root,A>for(){",
        ),
        (
            "pub struct Row<A, B>(pub A, #[serde(skip)] pub PhantomData<B>, pub u16) where Vec<B>: Clone;",
            "impl<Root,A>row_schema::mongo_members::MembersSlots<Root,A>for(){",
        ),
    ] {
        let (_module, on_the_type) = typed_paths_of(source, &[]);
        assert!(
            on_the_type.contains(header)
                && !on_the_type.contains("Tr<B>")
                && !on_the_type.contains("Vec<B>"),
            "missing `{header}`, or found `Tr<B>` or `Vec<B>`, in: {on_the_type}"
        );
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn an_enums_paths_are_where_its_form_writes_what_each_variant_holds() {
    for (source, written) in [
        (
            "#[serde(tag = \"kind\", rename_all = \"camelCase\")] pub enum Row { Cash, \
             #[serde(rename_all = \"camelCase\")] Card { exp_month: u32 }, Wire(Inner), Odd(u8) }",
            &[
                "pubconstfncard(&self)->row_schema::mongo_members::Card<Root>\
                 {row_schema::mongo_members::Card{row:::core::marker::PhantomData}}",
                "pubconstfnexp_month(&self)->row_schema::Field<Root,u32>\
                 {row_schema::Field::plain(row_schema::MongoPath::at(\"expMonth\"))}",
                "pubconstfnwire(&self)->row_schema::Nested<Root,Inner>{row_schema::Nested::at(row_schema::MongoPath::ROOT)}",
                "pubfnis_cash(&self)->row_schema::Filter<Root>\
                 {row_schema::Filter::held(\"kind\",\"$eq\",bson::Bson::String(\"cash\".to_owned()))}",
                "pubfnis_odd(&self)->row_schema::Filter<Root>\
                 {row_schema::Filter::held(\"kind\",\"$eq\",bson::Bson::String(\"odd\".to_owned()))}",
            ][..],
        ),
        (
            "#[serde(tag = \"t\", content = \"c\")] pub enum Row { Pickup, Locker(u32), \
             Door(Inner), Span(u32, u32) }",
            &[
                "pubconstfnlocker(&self)->row_schema::Field<Root,u32>{row_schema::Field::plain(row_schema::MongoPath::at(\"c\"))}",
                "pubconstfndoor(&self)->row_schema::Model<Root,Inner>{row_schema::Model::plain(row_schema::MongoPath::at(\"c\"))}",
                "pubconstfnspan(&self)->row_schema::mongo_members::Span<Root>{row_schema::mongo_members::Span(\
                 row_schema::Field::plain(row_schema::MongoPath::at(\"c.0\")),row_schema::Field::plain(row_schema::MongoPath::at(\"c.1\")))}",
                "impl<Root>row_schema::mongo_members::SpanSlots<Root>for()\
                 {typeSlot0=row_schema::Field<Root,u32>;typeSlot1=row_schema::Field<Root,u32>;}",
                "pubfnis_pickup(&self)->row_schema::Filter<Root>\
                 {row_schema::Filter::held(\"t\",\"$eq\",bson::Bson::String(\"Pickup\".to_owned()))}",
            ],
        ),
        (
            "#[serde(rename_all = \"camelCase\")] pub enum Row { NoDiscount, Percent(f64), \
             Coupon { code: String } }",
            &[
                "pubconstfnpercent(&self)->row_schema::Field<Root,f64>\
                 {row_schema::Field::plain(row_schema::MongoPath::at(\"percent\"))}",
                "pubconstfncoupon(&self)->row_schema::mongo_members::Coupon<Root>\
                 {row_schema::mongo_members::Coupon{row:::core::marker::PhantomData}}",
                "pubconstfncode(&self)->row_schema::Field<Root,String>\
                 {row_schema::Field::plain(row_schema::MongoPath::at(\"coupon.code\"))}",
                "pubfnis_no_discount(&self)->row_schema::Filter<Root>\
                 {row_schema::Filter::held(\"\",\"$eq\",bson::Bson::String(\"noDiscount\".to_owned()))}",
                "pubfnis_coupon(&self)->row_schema::Filter<Root>\
                 {row_schema::Filter::held(\"coupon\",\"$exists\",bson::Bson::Boolean(true))}",
            ],
        ),
        (
            "#[serde(untagged)] pub enum Row { Number(u32), Detailed { code: String }, Held(Inner) }",
            &[
                "pubconstfnnumber(&self)->row_schema::Field<Root,u32>{row_schema::Field::plain(row_schema::MongoPath::ROOT)}",
                "pubconstfndetailed(&self)->row_schema::mongo_members::Detailed<Root>\
                 {row_schema::mongo_members::Detailed{row:::core::marker::PhantomData}}",
                "pubconstfncode(&self)->row_schema::Field<Root,String>{row_schema::Field::plain(row_schema::MongoPath::at(\"code\"))}",
                "pubconstfnheld(&self)->row_schema::Nested<Root,Inner>{row_schema::Nested::at(row_schema::MongoPath::ROOT)}",
            ],
        ),
    ] {
        let (module, on_the_type) = typed_enum_paths_of(source, &["Inner"]);
        assert!(
            on_the_type.contains(
                "pubconstMONGO_FIELDS:row_schema::MongoFields<Self>=\
                 row_schema::MongoFields{row:::core::marker::PhantomData};"
            ),
            "for {source}, got: {on_the_type}"
        );
        for found in written {
            assert!(
                on_the_type.contains(found),
                "for {source}, missing `{found}` in: {on_the_type}"
            );
        }
        assert_eq!(
            on_the_type.contains("pubfnis_"),
            !source.contains("untagged"),
            "for {source}, got: {on_the_type}"
        );
        // serde refuses a plain value beside a tag, so it has no path.
        assert!(
            !format!("{module}{on_the_type}").contains("fnodd("),
            "for {source}, got: {module}{on_the_type}"
        );
        // The module declares the structs of paths, which name no type of the author's.
        assert!(!module.contains("Inner"), "for {source}, got: {module}");
    }
}

/// An `is_{variant}` keeps its name, and the function of a variant named `Is{Variant}` is the one
/// that is numbered.
#[cfg(feature = "mongodb")]
#[test]
fn the_function_of_a_variant_that_meets_an_ask_is_numbered() {
    let (_module, on_the_type) = typed_enum_paths_of(
        "#[serde(tag = \"kind\")] pub enum Row { Card { last4: String }, IsCard { code: u8 }, \
         IsCard2 { code: u8 } }",
        &[],
    );
    for written in [
        "pubfnis_card(&self)->row_schema::Filter<Root>\
         {row_schema::Filter::held(\"kind\",\"$eq\",bson::Bson::String(\"Card\".to_owned()))}",
        "pubfnis_is_card(&self)->row_schema::Filter<Root>{",
        "pubfnis_is_card2(&self)->row_schema::Filter<Root>{",
        "pubconstfncard(&self)->row_schema::mongo_members::Card<Root>{",
        "pubconstfnis_card2(&self)->row_schema::mongo_members::IsCard<Root>{",
        "pubconstfnis_card22(&self)->row_schema::mongo_members::IsCard2<Root>{",
    ] {
        assert!(
            on_the_type.contains(written),
            "missing `{written}` in: {on_the_type}"
        );
    }
    // Nothing names a variant of an untagged enum, so no ask takes a name from its functions.
    let (_untagged, on_the_untagged) = typed_enum_paths_of(
        "#[serde(untagged)] pub enum Row { IsCard { code: u8 }, Card { last4: String } }",
        &[],
    );
    assert!(
        on_the_untagged
            .contains("pubconstfnis_card(&self)->row_schema::mongo_members::IsCard<Root>{"),
        "got: {on_the_untagged}"
    );
}

/// A variant serde never writes has neither function nor `is_{variant}`.
#[cfg(feature = "mongodb")]
#[test]
fn a_variant_serde_never_writes_has_no_path_and_a_members_struct_takes_no_name_in_use() {
    let (module, on_the_type) = typed_enum_paths_of(
        "pub enum Row<T> { #[serde(skip)] Lost { at: u8 }, #[serde(skip_serializing)] Old(u8), \
         String { text: String }, T { held: T }, Members { count: u8 } }",
        &[],
    );
    let emitted = format!("{module}{on_the_type}");
    for absent in ["fnlost(", "fnold(", "is_lost", "is_old", "Lost", "Old"] {
        assert!(!emitted.contains(absent), "found `{absent}` in: {emitted}");
    }
    for written in [
        "pubstructString2<Root,T>{",
        "pubstructT2<Root,T>{",
        "pubstructMembers<Root,T>{",
        "pubconstfnstring(&self)->row_schema::mongo_members::String2<Root,T>{",
        "pubconstfnt(&self)->row_schema::mongo_members::T2<Root,T>{",
        "pubconstfnmembers(&self)->row_schema::mongo_members::Members<Root,T>{",
        "pubconstfntext(&self)->row_schema::Field<Root,String>{",
        "pubconstfnheld(&self)->row_schema::Field<Root,T>{",
        "pubconstfncount(&self)->row_schema::Field<Root,u8>{",
    ] {
        assert!(
            emitted.contains(written),
            "missing `{written}` in: {emitted}"
        );
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn the_typed_paths_name_only_what_both_majors_of_the_bson_library_have() {
    let (hooked, on_the_type) = typed_paths_of(
        "pub struct Row { #[serde(with = \"hook\")] pub a: u8, pub b: Inner, pub c: Vec<u8> }",
        &["Inner"],
    );
    let (external, on_the_external) =
        typed_enum_paths_of("pub enum Row { Empty, Label(String), To(i32, i32) }", &[]);
    let (tagged, on_the_tagged) = typed_enum_paths_of(
        "#[serde(tag = \"kind\")] pub enum Row { Clear, Solid { color: String } }",
        &[],
    );
    let written = [
        hooked,
        on_the_type,
        external,
        on_the_external,
        tagged,
        on_the_tagged,
    ]
    .concat();
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

/// The tag key of the enum `source` declares and its arms, each as its function, the value serde
/// writes for it and its variant's name. `None` for an enum serde writes under no tag key.
fn arms_of(source: &str) -> Option<(String, Vec<[String; 3]>)> {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    let (tag, arms) = super::enums::tag_arms(&item)?;
    let listed = arms
        .into_iter()
        .map(|arm| [arm.function, arm.value, arm.variant.ident.to_string()])
        .collect();
    Some((tag, listed))
}

/// What the enum `source` declares is refused, each refusal as its sentence and what it is
/// spanned on.
fn arm_refusals_of(source: &str) -> Vec<(String, String)> {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    super::enums::tag_arm_refusals(&item)
        .iter()
        .map(|refusal| {
            (
                refusal.to_string(),
                refusal.span().source_text().unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn snake_form_writes_the_words_of_a_tag_key_and_of_a_value_in_lower_case() {
    for (text, snake) in [
        ("type", "type"),
        ("docType", "doc_type"),
        ("by-courier", "by_courier"),
        ("PastDue", "past_due"),
        ("pastDue", "past_due"),
        ("PAST_DUE", "past_due"),
        ("past.due", "past_due"),
        ("HTTPServer", "http_server"),
        ("v2Beta", "v2_beta"),
        ("@type", "type"),
        ("_id", "id"),
    ] {
        assert_eq!(super::enums::snake_form(text), snake, "for {text}");
    }
    // The README states the rule with four of these.
    for shown in [
        "`docType` is `doc_type`",
        "`HTTPServer` is `http_server`",
        "`v2Beta` is `v2_beta`",
        "`@type` is `type`",
    ] {
        assert!(
            include_str!("../../../README.md").contains(shown),
            "the README no longer says: {shown}"
        );
    }
}

#[test]
fn an_enum_under_a_tag_key_has_one_arm_per_variant_serde_writes() {
    let arm = |function: &str, value: &str, variant: &str| {
        [function.to_owned(), value.to_owned(), variant.to_owned()]
    };
    assert_eq!(
        arms_of(SHIPPING),
        Some((
            "type".to_owned(),
            vec![
                arm("type__by_courier", "by-courier", "ByCourier"),
                arm("type__by_rail", "by-rail", "ByRail"),
                arm("type__collected", "Collected", "Collected"),
                arm("type__locker", "Locker", "Locker"),
            ]
        ))
    );
    assert_eq!(
        arms_of(PAPER),
        Some((
            "docType".to_owned(),
            vec![
                arm("doc_type__past_due", "past-due", "PastDue"),
                arm("doc_type__receipt", "receipt", "Receipt"),
            ]
        ))
    );
    // The value is the one serde writes, and a variant it never writes is no arm.
    assert_eq!(
        arms_of(
            "#[serde(tag = \"kind\")] pub enum Gate { #[serde(skip)] Lost, Open, \
             #[serde(rename(serialize = \"sent\", deserialize = \"received\"))] Posted, \
             #[serde(skip_serializing)] Shut, #[serde(skip_deserializing)] Stuck }"
        ),
        Some((
            "kind".to_owned(),
            vec![
                arm("kind__open", "Open", "Open"),
                arm("kind__sent", "sent", "Posted"),
                arm("kind__stuck", "Stuck", "Stuck"),
            ]
        ))
    );
    for source in [EXTERNAL, UNTAGGED, "pub enum Status { Draft, Published }"] {
        assert_eq!(arms_of(source), None, "for {source}");
    }
}

#[test]
fn a_variant_whose_name_holds_two_underscores_is_refused_under_a_tag_key() {
    for (source, function) in [
        (
            "#[serde(tag = \"type\")] pub enum Shipping { By__Courier(Courier), Collected }",
            "type",
        ),
        (
            "#[serde(tag = \"docType\", content = \"body\")] pub enum Shipping { \
             By__Courier(Courier), Collected }",
            "doc_type",
        ),
    ] {
        assert_eq!(
            arm_refusals_of(source),
            [(
                format!(
                    "variant `By__Courier` holds `__` in its name. Under a tag and \
                     `decode_with`, `__` parts the tag from the value in the function of each \
                     arm, `{function}__<value>()`. Rename the variant"
                ),
                "By__Courier".to_owned()
            )],
            "for {source}"
        );
    }
}

#[test]
fn two_values_that_read_as_one_in_snake_case_are_refused_under_a_tag_key() {
    assert_eq!(
        arm_refusals_of(
            "#[serde(tag = \"type\")] pub enum Shipping { \
             #[serde(rename = \"by-courier\")] ByCourier(Courier), Collected, \
             #[serde(rename = \"by_courier\")] Couriered }"
        ),
        [(
            "variants `ByCourier` and `Couriered` are written `by-courier` and `by_courier` \
             under the tag `type`, and both read `by_courier` in snake case, so \
             `type__by_courier()` would answer for both. Rename one of the two values"
                .to_owned(),
            "Couriered".to_owned()
        )]
    );
}

#[test]
fn an_arm_function_that_meets_an_ask_is_refused() {
    let met = || {
        (
            "the arm function `is__parcel()` of variant `Parcel` and `is__parcel()`, which asks \
             for variant `_Parcel`, would be one function. Rename the variant `_Parcel`"
                .to_owned(),
            "_Parcel".to_owned(),
        )
    };
    assert_eq!(
        arm_refusals_of(
            "#[serde(tag = \"is\")] pub enum Shipping { Parcel, \
             #[serde(rename = \"boxed\")] _Parcel }"
        ),
        [met()]
    );
    // Left to its own name, `_Parcel` is written under a value that reads as `Parcel` does too.
    assert_eq!(
        arm_refusals_of("#[serde(tag = \"is\")] pub enum Shipping { Parcel, _Parcel }"),
        [
            (
                "variants `Parcel` and `_Parcel` are written `Parcel` and `_Parcel` under the \
                 tag `is`, and both read `parcel` in snake case, so `is__parcel()` would answer \
                 for both. Rename one of the two values"
                    .to_owned(),
                "_Parcel".to_owned()
            ),
            met()
        ]
    );
}

/// No function is named after a variant serde never writes, and none after a tag where no tag key
/// is stored.
#[test]
fn an_enum_whose_paths_share_no_name_is_refused_nothing() {
    for source in [
        SHIPPING,
        PAPER,
        "#[serde(tag = \"type\")] pub enum Shipping { \
         #[serde(skip_serializing)] By__Courier(Courier), Collected }",
        "pub enum Shipping { By__Courier(Courier), Collected }",
        "#[serde(untagged)] pub enum Shipping { By__Courier(Courier), Collected }",
        "pub enum Is { Parcel, _Parcel }",
    ] {
        assert_eq!(arm_refusals_of(source), [], "for {source}");
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn an_arm_is_a_function_written_beside_the_type_that_answers_its_tag() {
    let packed = |text: &str| text.split_whitespace().collect::<String>();
    let (module, on_the_type) = typed_enum_paths_of(SHIPPING, &[]);
    for written in [
        "impl<Root>shipping_schema::MongoFields<Root>{",
        "#[must_use]pubconstfntype__by_courier(&self)->shipping_schema::ArmFilter<Root,Courier>\
         {shipping_schema::ArmFilter::of(\"type\",\"by-courier\",shipping_schema::MongoPath::ROOT)}",
        "#[must_use]pubconstfntype__by_rail(&self)->shipping_schema::ArmFilter<Root,Wagon>\
         {shipping_schema::ArmFilter::of(\"type\",\"by-rail\",shipping_schema::MongoPath::ROOT)}",
        "#[must_use]pubconstfntype__collected(&self)->shipping_schema::ArmFilter<Root,()>\
         {shipping_schema::ArmFilter::of(\"type\",\"Collected\",shipping_schema::MongoPath::ROOT)}",
        "#[must_use]pubconstfntype__locker(&self)->shipping_schema::ArmFilter<Root,shipping_schema::mongo_arms::Locker>\
         {shipping_schema::ArmFilter::of(\"type\",\"Locker\",shipping_schema::MongoPath::ROOT)}",
    ] {
        assert!(
            on_the_type.contains(written),
            "missing `{written}` in: {on_the_type}"
        );
    }
    for told in [
        "# [doc = \"Rows of the arm serde writes as `by-courier`: the tag is asked, and `and` \
         takes conditions on the `Courier` it holds.\"]",
        "# [doc = \"Rows of the arm serde writes as `by-rail`: the tag is asked, and `and` takes \
         conditions on the `Wagon` it holds.\"]",
        "# [doc = \"Rows of the arm serde writes as `Collected`: the tag alone.\"]",
        "# [doc = \"Rows of the arm serde writes as `Locker`: the tag is asked, and `with` takes \
         conditions on the fields written in the variant.\"]",
    ] {
        assert!(
            on_the_type.contains(&packed(told)),
            "missing `{told}` in: {on_the_type}"
        );
    }
    let arms = packed(
        "# [doc = \"The arms of `Shipping`, one per variant serde writes.\"] \
         # [derive (Clone , Copy , Debug , Eq , PartialEq)] # [non_exhaustive] \
         pub enum Arm { ByCourier , ByRail , Collected , Locker , }",
    );
    assert!(module.contains(&arms), "missing `{arms}` in: {module}");
    assert!(!module.contains("__"), "got: {module}");
    assert!(!module.contains("#[serde"), "got: {module}");
}

/// An arm whose fields are written in the variant is answered over a marker, which `ArmPaths`
/// roots the variant's paths at, and whose place is the enum's own: the paths carry the content
/// key.
#[cfg(feature = "mongodb")]
#[test]
fn an_arm_whose_fields_are_written_in_the_variant_has_a_marker_that_roots_its_paths() {
    let (module, on_the_type) = typed_enum_paths_of(
        "#[serde(tag = \"docType\", content = \"body\")] pub enum Slip<T> { \
         Signed { clerk: String, load: T }, Pair(String, u32), Blank }",
        &[],
    );
    for written in [
        "pubmodmongo_arms{",
        "pubstructSigned<T>{of:::core::marker::PhantomData<fn()->(T,)>,}",
        "pubstructPair<T>{of:::core::marker::PhantomData<fn()->(T,)>,}",
    ] {
        assert!(module.contains(written), "missing `{written}` in: {module}");
    }
    for written in [
        "impl<T>slip_schema::ArmPathsforslip_schema::mongo_arms::Signed<T>{\
         typePaths=slip_schema::mongo_members::Signed<slip_schema::mongo_arms::Signed<T>,T>;\
         fnpaths()->Self::Paths{slip_schema::mongo_members::Signed{row:::core::marker::PhantomData}}}",
        "impl<T>slip_schema::ArmPathsforslip_schema::mongo_arms::Pair<T>\
         where():slip_schema::mongo_members::PairSlots<slip_schema::mongo_arms::Pair<T>>{",
        "pubconstfndoc_type__signed(&self)->slip_schema::ArmFilter<Root,slip_schema::mongo_arms::Signed<T>>\
         {slip_schema::ArmFilter::of(\"docType\",\"Signed\",slip_schema::MongoPath::ROOT)}",
        "pubconstfndoc_type__blank(&self)->slip_schema::ArmFilter<Root,()>\
         {slip_schema::ArmFilter::of(\"docType\",\"Blank\",slip_schema::MongoPath::at(\"body\"))}",
    ] {
        assert!(
            on_the_type.contains(written),
            "missing `{written}` in: {on_the_type}"
        );
    }
}

/// What an arm holds is written under the content key, which is the arm's place.
#[cfg(feature = "mongodb")]
#[test]
fn an_arm_of_an_adjacently_tagged_enum_is_placed_at_the_content_key() {
    let (module, on_the_type) = typed_enum_paths_of(PAPER, &[]);
    for written in [
        "impl<Root>paper_schema::MongoFields<Root>{",
        "pubconstfndoc_type__past_due(&self)->paper_schema::ArmFilter<Root,Notice>\
         {paper_schema::ArmFilter::of(\"docType\",\"past-due\",paper_schema::MongoPath::at(\"body\"))}",
        "pubconstfndoc_type__receipt(&self)->paper_schema::ArmFilter<Root,()>\
         {paper_schema::ArmFilter::of(\"docType\",\"receipt\",paper_schema::MongoPath::at(\"body\"))}",
    ] {
        assert!(
            on_the_type.contains(written),
            "missing `{written}` in: {on_the_type}"
        );
    }
    assert!(
        module.contains("pubenumArm{PastDue,Receipt,}"),
        "got: {module}"
    );
}

/// The model is the one `nest` reaches below a path over the slot, named as its author wrote it.
#[cfg(feature = "mongodb")]
#[test]
fn an_arm_holds_the_model_a_path_over_its_one_slot_reaches() {
    let (module, on_the_type) = typed_enum_paths_of(
        "#[serde(tag = \"kind\", content = \"held\")] pub enum Row<T> { Boxed(Box<Inner>), \
         Filled(Wrapper<T>), Listed(Vec<Inner>), Mapped(HashMap<String, Inner>), \
         Maybe(Option<Arc<Inner>>), Named { held: Inner }, Nothing, Of(T), Pair(u8, u8), \
         Then(Box<Self>), Unwritten(#[serde(skip_serializing)] Inner) }",
        &["Inner"],
    );
    assert!(
        on_the_type.contains(
            "impl<Root,T:::serde::Serialize>row_schema::MongoFields<Root,T>\
             whereRow<T>:::serde::Serialize{"
        ),
        "got: {on_the_type}"
    );
    for (arm, model) in [
        ("boxed", "Inner"),
        ("filled", "Wrapper<T>"),
        ("listed", "Inner"),
        ("mapped", "()"),
        ("maybe", "Inner"),
        ("named", "row_schema::mongo_arms::Named<T>"),
        ("nothing", "()"),
        ("of", "T"),
        ("pair", "row_schema::mongo_arms::Pair<T>"),
        ("then", "Row<T>"),
        ("unwritten", "()"),
    ] {
        let written =
            format!("pubconstfnkind__{arm}(&self)->row_schema::ArmFilter<Root,{model}>{{");
        assert!(
            on_the_type.contains(&written),
            "missing `{written}` in: {on_the_type}"
        );
    }
    assert!(
        module.contains(
            "pubenumArm{Boxed,Filled,Listed,Mapped,Maybe,Named,Nothing,Of,Pair,Then,Unwritten,}"
        ),
        "got: {module}"
    );
}

#[cfg(feature = "mongodb")]
#[test]
fn an_enum_serde_writes_under_no_tag_key_has_no_arm() {
    for source in [EXTERNAL, UNTAGGED, "pub enum Status { Draft, Published }"] {
        let (module, on_the_type) = typed_enum_paths_of(source, &["Inner"]);
        let emitted = format!("{module}{on_the_type}");
        for absent in ["__", "enumArm", "ArmFilter"] {
            assert!(
                !emitted.contains(absent),
                "for {source}, found `{absent}` in: {emitted}"
            );
        }
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_variant_serde_never_writes_has_no_arm() {
    let (module, on_the_type) = typed_enum_paths_of(
        "#[serde(tag = \"kind\")] pub enum Gate { #[serde(skip)] Lost, Open, \
         #[serde(skip_serializing)] Shut(Inner) }",
        &["Inner"],
    );
    assert!(module.contains("pubenumArm{Open,}"), "got: {module}");
    let emitted = format!("{module}{on_the_type}");
    for absent in ["kind__lost", "kind__shut", "Lost", "Shut"] {
        assert!(!emitted.contains(absent), "found `{absent}` in: {emitted}");
    }
}

/// A tag key whose snake case starts with a digit names its arm functions after one `_`.
#[cfg(feature = "mongodb")]
#[test]
fn a_tag_key_that_starts_with_a_digit_has_its_arm_functions_after_a_leading_underscore() {
    let (module, on_the_type) =
        typed_enum_paths_of("#[serde(tag = \"2fa\")] pub enum Gate { Open, Shut }", &[]);
    for written in [
        "pubconstfn_2fa__open(&self)->gate_schema::ArmFilter<Root,()>",
        "pubconstfn_2fa__shut(&self)->gate_schema::ArmFilter<Root,()>",
        "pubfnis_open(",
    ] {
        assert!(
            on_the_type.contains(written),
            "missing `{written}` in: {on_the_type}"
        );
    }
    assert!(module.contains("pubenumArm{Open,Shut,}"), "got: {module}");
    assert_eq!(
        arms_of("#[serde(tag = \"2fa\")] pub enum Gate { Open, Shut }"),
        Some((
            "2fa".to_owned(),
            vec![
                [
                    "_2fa__open".to_owned(),
                    "Open".to_owned(),
                    "Open".to_owned()
                ],
                [
                    "_2fa__shut".to_owned(),
                    "Shut".to_owned(),
                    "Shut".to_owned()
                ],
            ]
        ))
    );
}

#[test]
fn two_values_that_read_as_one_under_a_digit_led_tag_key_are_refused_by_the_name_emitted() {
    assert_eq!(
        arm_refusals_of(
            "#[serde(tag = \"2fa\")] pub enum Gate { \
             #[serde(rename = \"a-b\")] First, #[serde(rename = \"a_b\")] Second }"
        ),
        [(
            "variants `First` and `Second` are written `a-b` and `a_b` under the tag `2fa`, and \
             both read `a_b` in snake case, so `_2fa__a_b()` would answer for both. Rename one \
             of the two values"
                .to_owned(),
            "Second".to_owned()
        )]
    );
}
