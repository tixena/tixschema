//! The recovering decode `#[model_schema(decode_with)]` turns on.
//!
//! A flagged struct gets `from_value_with`, which reads a `serde_json::Value` with plain serde,
//! walks it, and hands every issue the walk finds to a callback once. A build with `bson` on adds
//! `from_bson_with`, the same read of a `bson::Document`, written with what both major versions of
//! the `bson` library have. The callback's types go into the type's own `{type}_schema` module. Two
//! flagged types share no declaration: each module declares the same aliases of standard types, and
//! a walker builds whatever issue type the constructor it is handed builds.

use core::iter::once;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use proc_macro2::{Group, Spacing, TokenTree};
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt as _;
use syn::{Field, GenericArgument, ItemStruct, LitStr, PathArguments, Type, TypePath};

use crate::features::serde::{
    NAMED_READ_HOOK_PREFIX, SerdeFieldHooks, has_serde_default, has_serde_read_hook,
    parse_serde_field_attributes, parse_serde_field_hooks, parse_serde_key_omission,
    parse_serde_type_attributes,
};
use crate::field_type::{
    FieldDefType, get_field_def, is_refused_sequence_wrapper, is_sequence_wrapper,
    is_transparent_wrapper,
};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{ident_schema_module_name, written_type};

/// The type names the flag adds to a schema module.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const ADDED_TYPE_NAMES: [&str; 8] = [
    "Expected",
    "ExpectedToken",
    "Issue",
    "IssueFromParts",
    "Path",
    "Segment",
    "Unrecovered",
    "Verdict",
];

/// One arm reading a value that is there: the pattern it is held under, and what is listed for it.
struct Arm {
    body: TokenStream,
    /// The arm lists nothing: a `null` where the field is optional.
    nothing: bool,
    pattern: TokenStream,
}

/// What the flag adds to a type.
pub struct RecoveringDecode {
    /// The items added to the `{type}_schema` module a schema surface writes, or that module whole
    /// in a build where none writes one.
    pub schema_module: TokenStream,
    /// The `impl` holding the entry point and the walker.
    pub type_impl: TokenStream,
}

/// What a value is read from. One walk serves every source: the types it names, the patterns it
/// matches and the function a plain value is read through are the source's own.
#[derive(Clone, Copy)]
enum Source {
    #[cfg(feature = "bson")]
    Bson,
    Json,
}

impl Source {
    /// Every source this build generates an entry point and a walker for.
    const GENERATED: &'static [Self] = &[
        Self::Json,
        #[cfg(feature = "bson")]
        Self::Bson,
    ];

    /// The pattern a map is held under, binding its entries.
    fn entries(self, entries: &Ident) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Bson::Document(#entries) },
            Self::Json => quote! { serde_json::Value::Object(#entries) },
        }
    }

    /// The entry point and the report it runs.
    fn entry_methods(self, module: &Ident) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => bson_entry_methods(module),
            Self::Json => entry_methods(module),
        }
    }

    /// The pattern a list is held under, binding its items.
    fn items(self, items: &Ident) -> TokenStream {
        let value = self.value();
        quote! { #value::Array(#items) }
    }

    /// The function a plain value is read through.
    fn leaf(self) -> Ident {
        format_ident!("{}_leaf", self.stem())
    }

    /// One of the walker's methods: `decode_with_value_issues`, `decode_with_bson_fields`.
    fn method(self, part: &str) -> Ident {
        format_ident!("decode_with_{}_{part}", self.stem())
    }

    fn null(self) -> TokenStream {
        let value = self.value();
        quote! { #value::Null }
    }

    /// The type of the object whose keys a type's fields are looked up in.
    fn object(self) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Document },
            Self::Json => quote! { serde_json::Map<String, serde_json::Value> },
        }
    }

    /// What binds `object` to the keys of `found`, when it holds an object.
    fn object_of_found(self) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Bson::Document(object) = found },
            Self::Json => quote! { Some(object) = found.as_object() },
        }
    }

    /// The source's name inside what the flag adds for it.
    const fn stem(self) -> &'static str {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => "bson",
            Self::Json => "value",
        }
    }

    /// The type of one value.
    fn value(self) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Bson },
            Self::Json => quote! { serde_json::Value },
        }
    }

    /// The closure writing back a value read whole, through a hook's own writer or with the type's
    /// own `Serialize`. A value a hook read is `pinned` to the field's type: a hook generic over
    /// what it reads is pinned by nothing else, as serde's derive pins it by the field.
    fn write_back(self, pinned: Option<&Type>, through: Option<&TokenStream>) -> TokenStream {
        let read = pinned.map_or_else(|| quote! { read }, |ty| quote! { read: &#ty });
        match (self, through) {
            #[cfg(feature = "bson")]
            (Self::Bson, Some(writer)) => quote! { |#read, to| #writer(read, to).ok() },
            #[cfg(feature = "bson")]
            (Self::Bson, None) => {
                quote! { |#read, to| serde::Serialize::serialize(read, to).ok() }
            }
            (Self::Json, Some(writer)) => {
                quote! { |#read| #writer(read, serde_json::value::Serializer).ok() }
            }
            (Self::Json, None) => quote! { |#read| serde_json::to_value(read).ok() },
        }
    }
}

/// How the value at one position of a field's type is walked.
enum Step<'ty> {
    /// A map: each value at its key.
    Entries(Box<Walk<'ty>>),
    /// A list: each item at its index.
    Items(Box<Walk<'ty>>),
    /// A plain value, read whole.
    Leaf(Whole),
    /// Another flagged type: its own walker.
    Model,
    /// An optional value, walked when it is not `null`.
    Present(Box<Walk<'ty>>),
}

struct Walk<'ty> {
    step: Step<'ty>,
    ty: &'ty Type,
}

impl Walk<'_> {
    const fn reads_whole(&self) -> bool {
        matches!(self.step, Step::Leaf(_))
    }
}

/// One field of the struct as its walker reads it.
struct WalkedField<'item> {
    /// serde reads the field when its key is missing.
    absence_is_read: bool,
    aliases: Vec<String>,
    key: String,
    ty: &'item Type,
    /// `None` for a key the type writes and never reads back.
    walk: Option<Walk<'item>>,
}

/// The function serde reads a plain value with, and the one that writes it back.
struct Whole {
    /// A hook of the field's reads the value.
    hooked: bool,
    read: TokenStream,
    /// The function a hook writes the value to a serializer with, and `None` for the type's own
    /// `Serialize`.
    write: Option<TokenStream>,
}

/// What the generated code names of the type it is generated for, and the source it walks.
struct Walker<'item> {
    module: &'item Ident,
    own_name: &'item str,
    source: Source,
}

impl Walker<'_> {
    /// The arms a value held under `held` is matched by. `reported` is the type named as expected
    /// by the arm for a value that is no list or map, which under an `Option` is the `Option`.
    fn arms(
        &self,
        walk: &Walk<'_>,
        held: &Ident,
        segments: &[TokenStream],
        depth: usize,
        reported: &Type,
    ) -> Vec<Arm> {
        let (module, source) = (self.module, self.source);
        let path = path_expression(segments);
        match &walk.step {
            Step::Entries(inner) => {
                let (entries, key, item) = (
                    binding("entries", depth),
                    binding("key", depth),
                    binding("item", depth),
                );
                let each = self.statement(
                    inner,
                    &item,
                    &under(segments, &quote! { Ok(#key.clone()) }),
                    depth,
                );
                let expected = self.expected(reported);
                vec![
                    Arm {
                        body: quote! {{ for (#key, #item) in #entries { #each } }},
                        nothing: false,
                        pattern: source.entries(&entries),
                    },
                    not_the_shape(held, &path, &expected, "not an object"),
                ]
            }
            Step::Items(inner) => {
                let (items, index, item) = (
                    binding("items", depth),
                    binding("index", depth),
                    binding("item", depth),
                );
                let each = self.statement(
                    inner,
                    &item,
                    &under(segments, &quote! { Err(#index) }),
                    depth,
                );
                let expected = self.expected(reported);
                vec![
                    Arm {
                        body: quote! {{ for (#index, #item) in #items.iter().enumerate() { #each } }},
                        nothing: false,
                        pattern: source.items(&items),
                    },
                    not_the_shape(held, &path, &expected, "not an array"),
                ]
            }
            Step::Leaf(Whole {
                hooked,
                read,
                write,
            }) => {
                let expected = self.expected(walk.ty);
                let leaf = source.leaf();
                let written = source.write_back(hooked.then_some(walk.ty), write.as_ref());
                vec![Arm {
                    body: quote! {
                        out.extend(#module::#leaf(#held, #read, #written, #path, #expected, issue))
                    },
                    nothing: false,
                    pattern: quote! { #held },
                }]
            }
            Step::Model => vec![Arm {
                body: model_walker_call(source, walk.ty, held, &path),
                nothing: false,
                pattern: quote! { #held },
            }],
            Step::Present(inner) => {
                let mut arms = vec![Arm {
                    body: quote! { {} },
                    nothing: true,
                    pattern: source.null(),
                }];
                arms.extend(self.arms(inner, held, segments, depth, walk.ty));
                arms
            }
        }
    }

    /// The field's type as the constant list of tokens an issue carries for it.
    fn expected(&self, ty: &Type) -> TokenStream {
        let mut def = get_field_def("", ty, "");
        def.resolve_self_references(self.own_name, &[]);
        let members = def
            .expected_members()
            .into_iter()
            .map(|(member, names, count)| {
                let under = Literal::usize_unsuffixed(count);
                quote! { (#member, &[#(#names),*], #under) }
            });
        quote! { &[#(#members),*] }
    }

    /// What the fields walker runs for one field: its key looked up under its name and every
    /// alias, the value walked when it is there, and `Missing` when serde needs it.
    fn field(&self, field: &WalkedField<'_>) -> TokenStream {
        let Some(walk) = &field.walk else {
            return TokenStream::new();
        };
        let key = &field.key;
        let held = Ident::new("held", Span::call_site());
        let under_alias = !field.aliases.is_empty();
        let (lookup, stored) = if under_alias {
            let aliases = &field.aliases;
            (
                quote! {
                    [#key, #(#aliases),*]
                        .into_iter()
                        .find_map(|stored| object.get(stored).map(|held| (stored, held)))
                },
                quote! { Ok(stored.to_owned()) },
            )
        } else {
            (quote! { object.get(#key) }, quote! { Ok(#key.to_owned()) })
        };
        let there = |pattern: &TokenStream| {
            if under_alias {
                quote! { Some((stored, #pattern)) }
            } else {
                quote! { Some(#pattern) }
            }
        };
        let arms = self.arms(walk, &held, &[stored], 0, walk.ty);
        if field.absence_is_read
            && let [only] = arms.as_slice()
        {
            let (pattern, body) = (there(&only.pattern), &only.body);
            return quote! {
                if let #pattern = #lookup {
                    #body;
                }
            };
        }
        // An absent key and a `null` are the same `None` to serde, so one arm answers both.
        let merged =
            field.absence_is_read && !under_alias && arms.first().is_some_and(|arm| arm.nothing);
        let listed = arms.iter().skip(usize::from(merged)).map(|arm| {
            let (pattern, body) = (there(&arm.pattern), &arm.body);
            quote! { #pattern => #body, }
        });
        let absent = if merged {
            let null = self.source.null();
            quote! { None | Some(#null) => {} }
        } else if field.absence_is_read {
            quote! { None => {} }
        } else {
            let here = path_expression(&[quote! { Ok(#key.to_owned()) }]);
            let expected = self.expected(field.ty);
            quote! { None => out.push(issue("Missing", #here, #expected, None, None, Vec::new())), }
        };
        if merged {
            quote! { match #lookup { #absent #(#listed)* } }
        } else {
            quote! { match #lookup { #(#listed)* #absent } }
        }
    }

    /// What lists the issues of a value held under `held`, one level under `depth`.
    fn statement(
        &self,
        walk: &Walk<'_>,
        held: &Ident,
        segments: &[TokenStream],
        depth: usize,
    ) -> TokenStream {
        let arms = self.arms(walk, held, segments, depth.saturating_add(1), walk.ty);
        if let [only] = arms.as_slice() {
            let body = &only.body;
            return quote! { #body; };
        }
        let listed = arms.iter().map(|arm| {
            let (pattern, body) = (&arm.pattern, &arm.body);
            quote! { #pattern => #body, }
        });
        quote! { match #held { #(#listed)* } }
    }
}

/// `tokens` with every bare use of a type name the flag adds written `super::Name`.
///
/// A schema module reads the author's scope through `use super::*`, which an item of the same
/// name beside it shadows. `super::` names the author's one whatever the module holds.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
pub fn reading_the_authors_scope(tokens: TokenStream) -> TokenStream {
    let mut scoped: Vec<TokenTree> = Vec::new();
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                let mut inner =
                    Group::new(group.delimiter(), reading_the_authors_scope(group.stream()));
                inner.set_span(group.span());
                scoped.push(TokenTree::Group(inner));
            }
            TokenTree::Ident(ident) => {
                if ADDED_TYPE_NAMES.contains(&ident.to_string().as_str())
                    && !follows_a_path_separator(&scoped)
                {
                    scoped.extend(quote! { super:: });
                }
                scoped.push(TokenTree::Ident(ident));
            }
            other @ (TokenTree::Punct(_) | TokenTree::Literal(_)) => scoped.push(other),
        }
    }
    scoped.into_iter().collect()
}

/// The recovering decode of a struct with named fields, read off the item as it is emitted, so a
/// read hook tixschema hangs on a field is one the walker reads through.
pub fn struct_recovering_decode(item_struct: &ItemStruct) -> RecoveringDecode {
    let own_name = item_struct.ident.to_string();
    let module_name = ident_schema_module_name(&own_name);
    let container = parse_serde_type_attributes(&item_struct.attrs);
    let defaulted = has_serde_default(&item_struct.attrs);
    let fields: Vec<WalkedField<'_>> = item_struct
        .fields
        .iter()
        .filter_map(|field| {
            walked_field(
                field,
                container.rename_all.as_deref(),
                defaulted,
                &module_name,
            )
        })
        .collect();
    let module = Ident::new(&module_name, Span::call_site());
    let mut declared: Vec<&str> = fields
        .iter()
        .flat_map(|field| once(&field.key).chain(&field.aliases))
        .map(String::as_str)
        .collect();
    // serde writes a struct's `tag` as a key of the object and reads past it.
    declared.extend(container.tag.as_deref());
    let methods = Source::GENERATED.iter().map(|&source| {
        let walker = Walker {
            module: &module,
            own_name: &own_name,
            source,
        };
        let walks: Vec<TokenStream> = fields.iter().map(|field| walker.field(field)).collect();
        let entry = source.entry_methods(&module);
        let walk_methods = walker_methods(&walker, &walks, &declared);
        quote! {
            #entry
            #walk_methods
        }
    });
    let name = &item_struct.ident;
    RecoveringDecode {
        schema_module: placed_in_schema_module(&module, &module_items()),
        type_impl: quote! {
            impl #name {
                #(#methods)*
            }
        },
    }
}

/// A loop's binding at `depth`: the plain name at the first level, and a numbered one under it,
/// where an outer index or key is still read to build the path.
fn binding(stem: &str, depth: usize) -> Ident {
    let name = if depth == 0 {
        stem.to_owned()
    } else {
        format!("{stem}_{depth}")
    };
    Ident::new(&name, Span::call_site())
}

/// `from_bson_with` and the report it runs. Every value is read through `bson::Deserializer::new`,
/// which both major versions of the `bson` library have.
#[cfg(feature = "bson")]
fn bson_entry_methods(module: &Ident) -> TokenStream {
    quote! {
        /// Reads `document` as this type, handing every issue found in it to `decide`, once.
        pub fn from_bson_with<F>(
            mut document: bson::Document,
            decide: F,
        ) -> core::result::Result<Self, #module::Unrecovered<bson::Bson>>
        where
            F: FnOnce(&mut bson::Document, &[#module::Issue<bson::Bson>]) -> #module::Verdict,
        {
            let found = match <Self as serde::Deserialize>::deserialize(bson::Deserializer::new(bson::Bson::Document(document.clone()))) {
                Ok(decoded) => {
                    let found = Self::decode_with_bson_report(&document);
                    if found.is_empty() {
                        return Ok(decoded);
                    }
                    found
                }
                Err(_) => Self::decode_with_bson_report(&document),
            };
            match decide(&mut document, &found) {
                #module::Verdict::Reject => Err(#module::Unrecovered { issues: found }),
                #module::Verdict::Fixed => {
                    let again = Self::decode_with_bson_report(&document);
                    match <Self as serde::Deserialize>::deserialize(bson::Deserializer::new(bson::Bson::Document(document.clone()))) {
                        Ok(decoded) if again.is_empty() => Ok(decoded),
                        _ => Err(#module::Unrecovered { issues: again }),
                    }
                }
            }
        }

        fn decode_with_bson_report(document: &bson::Document) -> Vec<#module::Issue<bson::Bson>> {
            let mut out = Vec::new();
            let found = bson::Bson::Document(document.clone());
            Self::decode_with_bson_issues(&found, &[], #module::issue_from_parts, &mut out);
            if out.iter().all(|found| matches!(found, #module::Issue::Unknown { .. } | #module::Issue::Mistyped { .. }))
                && let Err(refused) = <Self as serde::Deserialize>::deserialize(bson::Deserializer::new(found))
            {
                out.push(#module::Issue::Undescribed { reason: refused.to_string() });
            }
            out
        }
    }
}

/// `bson_leaf` and the bracket rule it decides `Mistyped` by.
#[cfg(feature = "bson")]
fn bson_leaf_items() -> TokenStream {
    quote! {
        /// Whether a query for `written` matches `stored`: numbers compare across their types, strings and
        /// symbols are one type, and every other BSON type matches only itself.
        pub fn same_bracket(stored: &bson::Bson, written: &bson::Bson) -> bool {
            let numeric = |b: &bson::Bson| matches!(b, bson::Bson::Int32(_) | bson::Bson::Int64(_) | bson::Bson::Double(_) | bson::Bson::Decimal128(_));
            let textual = |b: &bson::Bson| matches!(b, bson::Bson::String(_) | bson::Bson::Symbol(_));
            (numeric(stored) && numeric(written))
                || (textual(stored) && textual(written))
                || std::mem::discriminant(stored) == std::mem::discriminant(written)
        }

        /// One BSON value read whole: refused by serde, or read but stored as a type `write`
        /// does not give back.
        pub fn bson_leaf<T, I, E, R, W>(
            held: &bson::Bson,
            read: R,
            write: W,
            path: Vec<core::result::Result<String, usize>>,
            expected: &'static [ExpectedToken],
            issue: IssueFromParts<bson::Bson, I>,
        ) -> Option<I>
        where
            E: core::fmt::Display,
            R: FnOnce(bson::Deserializer) -> core::result::Result<T, E>,
            W: FnOnce(&T, bson::Serializer) -> Option<bson::Bson>,
        {
            match read(bson::Deserializer::new(held.clone())) {
                Err(refused) => Some(issue("Invalid", path, expected, Some(held.clone()), Some(refused.to_string()), Vec::new())),
                Ok(read) => match write(&read, bson::Serializer::new()) {
                    Some(written) if !same_bracket(held, &written) => {
                        Some(issue("Mistyped", path, expected, Some(held.clone()), None, Vec::new()))
                    }
                    _ => None,
                },
            }
        }
    }
}

/// A build without `bson` reads no BSON value.
#[cfg(not(feature = "bson"))]
fn bson_leaf_items() -> TokenStream {
    TokenStream::new()
}

/// The helpers a callback fixes a BSON document through.
#[cfg(feature = "bson")]
fn bson_path_helpers() -> TokenStream {
    quote! {
        /// Puts `value` at this path inside a BSON document, inserting the last key when it is absent.
        pub fn set_in_document(&self, root: &mut bson::Document, value: bson::Bson) -> bool {
            let Some((Segment::Key(first), rest)) = self.0.split_first() else { return false };
            let Some((last, parents)) = rest.split_last() else {
                root.insert(first.clone(), value);
                return true;
            };
            let Some(mut slot) = root.get_mut(first) else { return false };
            for segment in parents {
                let next = match (segment, slot) {
                    (Segment::Key(key), bson::Bson::Document(object)) => object.get_mut(key),
                    (Segment::Index(index), bson::Bson::Array(items)) => items.get_mut(*index),
                    _ => None,
                };
                let Some(next) = next else { return false };
                slot = next;
            }
            match (last, slot) {
                (Segment::Key(key), bson::Bson::Document(object)) => {
                    object.insert(key.clone(), value);
                    true
                }
                (Segment::Index(index), bson::Bson::Array(items)) if *index < items.len() => {
                    items[*index] = value;
                    true
                }
                _ => false,
            }
        }

        /// Removes the key or item at this path inside a BSON document.
        pub fn remove_from_document(&self, root: &mut bson::Document) -> bool {
            let Some((Segment::Key(first), rest)) = self.0.split_first() else { return false };
            let Some((last, parents)) = rest.split_last() else {
                return root.remove(first).is_some();
            };
            let Some(mut slot) = root.get_mut(first) else { return false };
            for segment in parents {
                let next = match (segment, slot) {
                    (Segment::Key(key), bson::Bson::Document(object)) => object.get_mut(key),
                    (Segment::Index(index), bson::Bson::Array(items)) => items.get_mut(*index),
                    _ => None,
                };
                let Some(next) = next else { return false };
                slot = next;
            }
            match (last, slot) {
                (Segment::Key(key), bson::Bson::Document(object)) => object.remove(key).is_some(),
                (Segment::Index(index), bson::Bson::Array(items)) if *index < items.len() => {
                    items.remove(*index);
                    true
                }
                _ => false,
            }
        }
    }
}

/// A build without `bson` fixes no BSON document.
#[cfg(not(feature = "bson"))]
fn bson_path_helpers() -> TokenStream {
    TokenStream::new()
}

/// The types a callback works with.
fn callback_items() -> TokenStream {
    quote! {
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub enum Expected {
            Array(Box<Expected>),
            Boolean,
            BooleanLiteral(bool),
            Char,
            DateTime,
            F32,
            F64,
            I8,
            I16,
            I32,
            I64,
            Isize,
            /// The type of the map's values.
            Map(Box<Expected>),
            /// A `#[model_schema]` type, by its Rust name as the field's type writes it.
            Model(&'static str),
            NaiveDate,
            NaiveDateTime,
            NaiveTime,
            NumberLiteral(f64),
            ObjectId,
            Optional(Box<Expected>),
            String,
            StringLiteral(&'static str),
            Tuple(Vec<Expected>),
            /// One of the type's own type parameters, by its name.
            TypeParam(&'static str),
            Unknown,
            U8,
            U16,
            U32,
            U64,
            Usize,
            /// The tag values a tagged enum accepts, for a tag naming none of them.
            Variants(&'static [&'static str]),
        }

        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub enum Issue<V> {
            /// The key is there, and its field cannot be read from what it holds.
            Invalid { path: Path, expected: Expected, found: V, reason: String },
            /// The type requires the key, and it is not there.
            Missing { path: Path, expected: Expected },
            /// The key is there, and the type declares no field by that name.
            Unknown { path: Path, found: V },
            /// The value reads, but it is held in a different form than its field writes: another JSON
            /// shape, or another BSON type, which a MongoDB query for the field's own type does not match.
            Mistyped { path: Path, expected: Expected, found: V },
            /// An untagged enum none of whose variants reads the value: each variant's own list.
            NoVariant { path: Path, found: V, variants: Vec<(&'static str, Vec<Issue<V>>)> },
            /// Serde refused the value and the walk found nothing to say why.
            Undescribed { reason: String },
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum Verdict {
            Reject,
            Fixed,
        }

        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub struct Unrecovered<V> {
            pub issues: Vec<Issue<V>>,
        }

        impl<V: core::fmt::Debug> core::fmt::Display for Unrecovered<V> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let place = |path: &Path| {
                    if path.0.is_empty() {
                        String::from("the value itself")
                    } else {
                        path.to_string()
                    }
                };
                for (position, issue) in self.issues.iter().enumerate() {
                    if position > 0 {
                        writeln!(f)?;
                    }
                    match issue {
                        Issue::Invalid { path, expected, found, reason } => {
                            write!(f, "{}: invalid: expected {expected:?}, found {found:?}: {reason}", place(path))?;
                        }
                        Issue::Missing { path, expected } => {
                            write!(f, "{}: missing: expected {expected:?}", place(path))?;
                        }
                        Issue::Unknown { path, found } => {
                            write!(f, "{}: unknown: found {found:?}", place(path))?;
                        }
                        Issue::Mistyped { path, expected, found } => {
                            write!(f, "{}: mistyped: expected {expected:?}, found {found:?}", place(path))?;
                        }
                        Issue::NoVariant { path, found, variants } => {
                            let tried: Vec<&str> = variants.iter().map(|(variant, _)| *variant).collect();
                            write!(f, "{}: no variant: found {found:?}, tried {}", place(path), tried.join(", "))?;
                        }
                        Issue::Undescribed { reason } => write!(f, "undescribed: {reason}")?,
                    }
                }
                Ok(())
            }
        }

        impl<V: core::fmt::Debug> std::error::Error for Unrecovered<V> {}
    }
}

/// `from_value_with` and the report it runs.
fn entry_methods(module: &Ident) -> TokenStream {
    quote! {
        /// Reads `value` as this type, handing every issue found in it to `decide`, once.
        pub fn from_value_with<F>(
            mut value: serde_json::Value,
            decide: F,
        ) -> core::result::Result<Self, #module::Unrecovered<serde_json::Value>>
        where
            F: FnOnce(&mut serde_json::Value, &[#module::Issue<serde_json::Value>]) -> #module::Verdict,
        {
            let found = match <Self as serde::Deserialize>::deserialize(&value) {
                Ok(decoded) => {
                    let found = Self::decode_with_value_report(&value);
                    if found.is_empty() {
                        return Ok(decoded);
                    }
                    found
                }
                Err(_) => Self::decode_with_value_report(&value),
            };
            match decide(&mut value, &found) {
                #module::Verdict::Reject => Err(#module::Unrecovered { issues: found }),
                #module::Verdict::Fixed => {
                    let again = Self::decode_with_value_report(&value);
                    match <Self as serde::Deserialize>::deserialize(&value) {
                        Ok(decoded) if again.is_empty() => Ok(decoded),
                        _ => Err(#module::Unrecovered { issues: again }),
                    }
                }
            }
        }

        fn decode_with_value_report(value: &serde_json::Value) -> Vec<#module::Issue<serde_json::Value>> {
            let mut out = Vec::new();
            Self::decode_with_value_issues(value, &[], #module::issue_from_parts, &mut out);
            if out.iter().all(|found| matches!(found, #module::Issue::Unknown { .. } | #module::Issue::Mistyped { .. }))
                && let Err(refused) = <Self as serde::Deserialize>::deserialize(value)
            {
                out.push(#module::Issue::Undescribed { reason: refused.to_string() });
            }
            out
        }
    }
}

/// The items one type's walker hands issues to another's with.
fn handoff_items() -> TokenStream {
    let bson_leaf = bson_leaf_items();
    quote! {
        /// One member of an `Expected`, written before the members under it: its name, the names
        /// it carries, and how many members follow under it.
        pub type ExpectedToken = (&'static str, &'static [&'static str], usize);

        /// Builds one issue of type `I` from standard parts: kind, path (`Ok` a key, `Err` an
        /// index), expected type, held value, reason, and an untagged enum's lists.
        pub type IssueFromParts<V, I> = fn(
            &'static str,
            Vec<core::result::Result<String, usize>>,
            &'static [ExpectedToken],
            Option<V>,
            Option<String>,
            Vec<(&'static str, Vec<I>)>,
        ) -> I;

        fn expected_from_tokens(tokens: &mut std::slice::Iter<'_, ExpectedToken>) -> Expected {
            let Some(&(member, names, count)) = tokens.next() else { return Expected::Unknown };
            let name = names.first().copied().unwrap_or_default();
            let mut under: Vec<Expected> = (0..count).map(|_| expected_from_tokens(tokens)).collect();
            let first = |under: &mut Vec<Expected>| Box::new(under.pop().unwrap_or(Expected::Unknown));
            match member {
                "Array" => Expected::Array(first(&mut under)),
                "Boolean" => Expected::Boolean,
                "BooleanLiteral" => Expected::BooleanLiteral(name == "true"),
                "Char" => Expected::Char,
                "DateTime" => Expected::DateTime,
                "F32" => Expected::F32,
                "F64" => Expected::F64,
                "I8" => Expected::I8,
                "I16" => Expected::I16,
                "I32" => Expected::I32,
                "I64" => Expected::I64,
                "Isize" => Expected::Isize,
                "Map" => Expected::Map(first(&mut under)),
                "Model" => Expected::Model(name),
                "NaiveDate" => Expected::NaiveDate,
                "NaiveDateTime" => Expected::NaiveDateTime,
                "NaiveTime" => Expected::NaiveTime,
                "NumberLiteral" => Expected::NumberLiteral(name.parse().unwrap_or_default()),
                "ObjectId" => Expected::ObjectId,
                "Optional" => Expected::Optional(first(&mut under)),
                "String" => Expected::String,
                "StringLiteral" => Expected::StringLiteral(name),
                "Tuple" => Expected::Tuple(under),
                "TypeParam" => Expected::TypeParam(name),
                "U8" => Expected::U8,
                "U16" => Expected::U16,
                "U32" => Expected::U32,
                "U64" => Expected::U64,
                "Usize" => Expected::Usize,
                "Variants" => Expected::Variants(names),
                _ => Expected::Unknown,
            }
        }

        /// This type's own issue from the standard parts every walker writes.
        pub fn issue_from_parts<V>(
            kind: &'static str,
            path: Vec<core::result::Result<String, usize>>,
            expected: &'static [ExpectedToken],
            found: Option<V>,
            reason: Option<String>,
            variants: Vec<(&'static str, Vec<Issue<V>>)>,
        ) -> Issue<V> {
            let path = Path(
                path.into_iter()
                    .map(|segment| match segment {
                        Ok(key) => Segment::Key(key),
                        Err(index) => Segment::Index(index),
                    })
                    .collect(),
            );
            let expected = expected_from_tokens(&mut expected.iter());
            match (kind, found, reason) {
                ("Invalid", Some(found), Some(reason)) => Issue::Invalid { path, expected, found, reason },
                ("Missing", None, None) => Issue::Missing { path, expected },
                ("Unknown", Some(found), None) => Issue::Unknown { path, found },
                ("Mistyped", Some(found), None) => Issue::Mistyped { path, expected, found },
                ("NoVariant", Some(found), None) => Issue::NoVariant { path, found, variants },
                (_, _, reason) => Issue::Undescribed { reason: reason.unwrap_or_default() },
            }
        }

        /// One JSON value read whole: refused by serde, or read but held in a different shape
        /// than `write` gives back.
        pub fn value_leaf<'a, T, I, R, W>(
            held: &'a serde_json::Value,
            read: R,
            write: W,
            path: Vec<core::result::Result<String, usize>>,
            expected: &'static [ExpectedToken],
            issue: IssueFromParts<serde_json::Value, I>,
        ) -> Option<I>
        where
            R: FnOnce(&'a serde_json::Value) -> core::result::Result<T, serde_json::Error>,
            W: FnOnce(&T) -> Option<serde_json::Value>,
        {
            match read(held) {
                Err(refused) => Some(issue("Invalid", path, expected, Some(held.clone()), Some(refused.to_string()), Vec::new())),
                Ok(read) => match write(&read) {
                    Some(written) if std::mem::discriminant(held) != std::mem::discriminant(&written) => {
                        Some(issue("Mistyped", path, expected, Some(held.clone()), None, Vec::new()))
                    }
                    _ => None,
                },
            }
        }

        #bson_leaf
    }
}

/// The functions a field is read and written back through when its author's serde attributes
/// name any, and `None` for a field its own type reads.
fn hooked_leaf(ty: &Type, hooks: &SerdeFieldHooks, module_name: &str) -> Option<Step<'static>> {
    let generated = format!("{module_name}::{NAMED_READ_HOOK_PREFIX}");
    let named = |hook: Option<&LitStr>| {
        let path = hook.filter(|path| !path.value().starts_with(&generated))?;
        path.parse::<syn::ExprPath>().ok()
    };
    let module = named(hooks.with.as_ref());
    let reader = named(hooks.deserialize_with.as_ref());
    let writer = named(hooks.serialize_with.as_ref());
    if module.is_none() && reader.is_none() && writer.is_none() {
        return None;
    }
    let read = reader.map_or_else(
        || {
            module.as_ref().map_or_else(
                || own_reader(ty),
                |through| quote! { #through::deserialize },
            )
        },
        |through| quote! { #through },
    );
    let write = writer.map_or_else(
        || {
            module
                .as_ref()
                .map(|through| quote! { #through::serialize })
        },
        |through| Some(quote! { #through }),
    );
    Some(Step::Leaf(Whole {
        hooked: true,
        read,
        write,
    }))
}

/// Whether the next token continues a path: the tokens end on `::`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn follows_a_path_separator(tokens: &[TokenTree]) -> bool {
    let [.., TokenTree::Punct(first), TokenTree::Punct(second)] = tokens else {
        return false;
    };
    first.as_char() == ':' && first.spacing() == Spacing::Joint && second.as_char() == ':'
}

/// Hands a value to the walker of the model type its field is declared as.
///
/// Every type a flagged type's fields reach carries the flag too. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Deserialize, Serialize)]
/// pub struct Version {
///     pub number: i32,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Deserialize, Serialize)]
/// pub struct Record {
///     pub versions: Vec<Version>,
/// }
///
/// fn main() {}
/// ```
///
/// The run below is that one with the flag taken off `Version`, and nothing else changed:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// pub struct Version {
///     pub number: i32,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Deserialize, Serialize)]
/// pub struct Record {
///     pub versions: Vec<Version>,
/// }
///
/// fn main() {}
/// ```
///
/// A `compile_fail` doctest asserts only that some error was raised, so the snippet was compiled
/// standalone as an ordinary test file, and this is the only error it earned, verbatim:
///
/// ```text
/// error[E0599]: no associated function or constant named `decode_with_value_issues` found for struct `Version` in the current scope
///   --> tests/zz_probe.rs:10:1
///    |
///  6 | pub struct Version {
///    | ------------------ associated function or constant `decode_with_value_issues` not found for this struct
/// ...
/// 10 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ associated function or constant not found in `Version`
///    |
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// A build with `bson` on earns the same error a second time, naming `decode_with_bson_issues`.
fn model_walker_call(source: Source, ty: &Type, held: &Ident, path: &TokenStream) -> TokenStream {
    let issues = source.method("issues");
    quote! { <#ty>::#issues(#held, &#path, issue, out) }
}

/// Everything the flag puts into the type's `{type}_schema` module.
fn module_items() -> TokenStream {
    let path = path_items();
    let callback = callback_items();
    let handoff = handoff_items();
    quote! {
        #path
        #callback
        #handoff
    }
}

/// The arm for a value that is neither the list nor the map its field is.
fn not_the_shape(held: &Ident, path: &TokenStream, expected: &TokenStream, reason: &str) -> Arm {
    Arm {
        body: quote! {
            out.push(issue("Invalid", #path, #expected, Some(#held.clone()), Some(#reason.to_owned()), Vec::new()))
        },
        nothing: false,
        pattern: quote! { #held },
    }
}

fn own_reader(ty: &Type) -> TokenStream {
    quote! { <#ty as serde::Deserialize>::deserialize }
}

/// `items` as they are emitted: beside what a schema surface writes into `{type}_schema`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn placed_in_schema_module(_module: &Ident, items: &TokenStream) -> TokenStream {
    items.clone()
}

/// `items` in a `{type}_schema` module written here, no schema surface writing one in this build.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
fn placed_in_schema_module(module: &Ident, items: &TokenStream) -> TokenStream {
    quote! {
        pub mod #module {
            #items
        }
    }
}

/// Where a value sits: the walker's own path with `segments` after it.
fn path_expression(segments: &[TokenStream]) -> TokenStream {
    quote! { [path, &[#(#segments),*]].concat() }
}

/// `Segment` and `Path`, with the helpers a callback fixes a value through.
fn path_items() -> TokenStream {
    let in_documents = bson_path_helpers();
    quote! {
        #[derive(Clone, Debug, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum Segment {
            Key(String),
            Index(usize),
        }

        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        #[non_exhaustive]
        pub struct Path(pub Vec<Segment>);

        impl Path {
            /// Puts `value` at this path inside a JSON value, inserting the last key when it is absent.
            pub fn set_in_value(&self, root: &mut serde_json::Value, value: serde_json::Value) -> bool {
                let Some((last, parents)) = self.0.split_last() else {
                    *root = value;
                    return true;
                };
                let mut slot = root;
                for segment in parents {
                    let next = match (segment, slot) {
                        (Segment::Key(key), serde_json::Value::Object(object)) => object.get_mut(key),
                        (Segment::Index(index), serde_json::Value::Array(items)) => items.get_mut(*index),
                        _ => None,
                    };
                    let Some(next) = next else { return false };
                    slot = next;
                }
                match (last, slot) {
                    (Segment::Key(key), serde_json::Value::Object(object)) => {
                        object.insert(key.clone(), value);
                        true
                    }
                    (Segment::Index(index), serde_json::Value::Array(items)) if *index < items.len() => {
                        items[*index] = value;
                        true
                    }
                    _ => false,
                }
            }

            /// Removes the key or item at this path inside a JSON value.
            pub fn remove_from_value(&self, root: &mut serde_json::Value) -> bool {
                let Some((last, parents)) = self.0.split_last() else { return false };
                let mut slot = root;
                for segment in parents {
                    let next = match (segment, slot) {
                        (Segment::Key(key), serde_json::Value::Object(object)) => object.get_mut(key),
                        (Segment::Index(index), serde_json::Value::Array(items)) => items.get_mut(*index),
                        _ => None,
                    };
                    let Some(next) = next else { return false };
                    slot = next;
                }
                match (last, slot) {
                    (Segment::Key(key), serde_json::Value::Object(object)) => object.remove(key).is_some(),
                    (Segment::Index(index), serde_json::Value::Array(items)) if *index < items.len() => {
                        items.remove(*index);
                        true
                    }
                    _ => false,
                }
            }

            #in_documents
        }

        impl core::fmt::Display for Path {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                for (position, segment) in self.0.iter().enumerate() {
                    match segment {
                        Segment::Key(key) if position == 0 => write!(f, "{key}")?,
                        Segment::Key(key) => write!(f, ".{key}")?,
                        Segment::Index(index) => write!(f, "[{index}]")?,
                    }
                }
                Ok(())
            }
        }
    }
}

/// The step a type written as a path takes: a container the walk goes into, another model type,
/// or a plain value.
fn path_step<'ty>(type_path: &'ty TypePath, ty: &'ty Type) -> Walk<'ty> {
    let Some(segment) = type_path.path.segments.last() else {
        return plain_value(ty);
    };
    let name = segment.ident.to_string();
    if let PathArguments::AngleBracketed(arguments) = &segment.arguments {
        let mut held = arguments.args.iter().filter_map(|argument| {
            if let GenericArgument::Type(inner) = argument {
                Some(inner)
            } else {
                None
            }
        });
        let (first, second) = (held.next(), held.next());
        if name == "Option"
            && let Some(inner) = first
        {
            let optional = walk_of(inner);
            return if optional.reads_whole() {
                plain_value(ty)
            } else {
                Walk {
                    step: Step::Present(Box::new(optional)),
                    ty,
                }
            };
        }
        if (is_sequence_wrapper(&name) || is_refused_sequence_wrapper(&name))
            && let Some(inner) = first
        {
            return Walk {
                step: Step::Items(Box::new(walk_of(inner))),
                ty,
            };
        }
        if matches!(name.as_str(), "BTreeMap" | "HashMap")
            && let Some(inner) = second
        {
            return Walk {
                step: Step::Entries(Box::new(walk_of(inner))),
                ty,
            };
        }
        // serde writes the wrapper as the value it holds, so what is walked is that value.
        if is_transparent_wrapper(&name)
            && let Some(inner) = first
        {
            let wrapped = walk_of(inner);
            return if wrapped.reads_whole() {
                plain_value(ty)
            } else {
                wrapped
            };
        }
    }
    if matches!(
        get_field_def("", ty, "").field_type,
        FieldDefType::SiblingType(_, _)
    ) {
        Walk {
            step: Step::Model,
            ty,
        }
    } else {
        plain_value(ty)
    }
}

/// A value read whole with its type's own `Deserialize`, and written back with its `Serialize`.
fn plain_value(ty: &Type) -> Walk<'_> {
    Walk {
        step: Step::Leaf(Whole {
            hooked: false,
            read: own_reader(ty),
            write: None,
        }),
        ty,
    }
}

/// `segments` with one more after them.
fn under(segments: &[TokenStream], segment: &TokenStream) -> Vec<TokenStream> {
    let mut longer = segments.to_vec();
    longer.push(segment.clone());
    longer
}

/// How a value of type `ty` is walked.
fn walk_of(ty: &Type) -> Walk<'_> {
    let written = written_type(ty);
    if let Type::Array(array) = written {
        Walk {
            step: Step::Items(Box::new(walk_of(&array.elem))),
            ty: written,
        }
    } else if let Type::Slice(slice) = written {
        Walk {
            step: Step::Items(Box::new(walk_of(&slice.elem))),
            ty: written,
        }
    } else if let Type::Path(type_path) = written {
        path_step(type_path, written)
    } else {
        plain_value(written)
    }
}

/// One field as the walker reads it, or `None` for a field serde neither writes nor reads.
fn walked_field<'item>(
    field: &'item Field,
    rename_all: Option<&str>,
    container_defaulted: bool,
    module_name: &str,
) -> Option<WalkedField<'item>> {
    let ident = field.ident.as_ref()?;
    let omission = parse_serde_key_omission(&field.attrs);
    if omission.absent_from_wire() {
        return None;
    }
    let meta = parse_serde_field_attributes(&field.attrs);
    let key = meta.rename.unwrap_or_else(|| {
        resolve_rename_rule(rename_all).apply_to_field(&ident.unraw().to_string())
    });
    let walk = (!omission.skips_deserializing).then(|| {
        hooked_leaf(
            &field.ty,
            &parse_serde_field_hooks(&field.attrs),
            module_name,
        )
        .map_or_else(
            || walk_of(&field.ty),
            |step| Walk {
                step,
                ty: &field.ty,
            },
        )
    });
    // With a read hook on the field, serde's derive no longer reads a missing key as `None`.
    let optional =
        !has_serde_read_hook(&field.attrs) && get_field_def("", &field.ty, "").is_optional();
    Some(WalkedField {
        absence_is_read: omission.defaulted || container_defaulted || optional,
        aliases: meta.aliases,
        key,
        ty: &field.ty,
        walk,
    })
}

/// The two methods one type's walker is called through by another's.
fn walker_methods(walker: &Walker<'_>, walks: &[TokenStream], declared: &[&str]) -> TokenStream {
    let (module, own_name, source) = (walker.module, walker.own_name, walker.source);
    let (issues, fields, leaf) = (
        source.method("issues"),
        source.method("fields"),
        source.leaf(),
    );
    let (value, object, object_of_found) =
        (source.value(), source.object(), source.object_of_found());
    let written = source.write_back(None, None);
    // A type with no field to walk lists no issue of its own, so the `Vec` it is handed goes
    // unbound.
    let out = if walks.iter().all(TokenStream::is_empty) {
        quote! { _ }
    } else {
        quote! { out }
    };
    quote! {
        /// Lists every issue in `found`, read as this type at `path`.
        pub fn #issues<I>(
            found: &#value,
            path: &[core::result::Result<String, usize>],
            issue: #module::IssueFromParts<#value, I>,
            out: &mut Vec<I>,
        ) {
            let #object_of_found else {
                out.extend(#module::#leaf(found, <Self as serde::Deserialize>::deserialize, #written, path.to_vec(), &[("Model", &[#own_name], 0)], issue));
                return;
            };
            let declared = Self::#fields(object, path, issue, out);
            for (key, held) in object {
                if !declared.contains(&key.as_str()) {
                    out.push(issue("Unknown", [path, &[Ok(key.clone())]].concat(), &[], Some(held.clone()), None, Vec::new()));
                }
            }
        }

        /// Lists every issue in this type's fields inside `object`, and returns the keys that are
        /// its own.
        pub fn #fields<'a, I>(
            object: &'a #object,
            path: &[core::result::Result<String, usize>],
            issue: #module::IssueFromParts<#value, I>,
            #out: &mut Vec<I>,
        ) -> Vec<&'a str> {
            #(#walks)*
            vec![#(#declared),*]
        }
    }
}

#[cfg(test)]
mod tests;
