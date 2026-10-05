//! The recovering decode `#[model_schema(decode_with)]` turns on.
//!
//! A flagged type gets `from_value_with`, which reads a `serde_json::Value` with plain serde,
//! walks it in the form serde writes that type in, and hands every issue the walk finds to a
//! callback once. A build with `bson` on adds `from_bson_with`, the same read of a
//! `bson::Document`, written with what both major versions of the `bson` library have. The
//! callback's types go into the type's own `{type}_schema` module. Two flagged types share no
//! declaration: each module declares the same aliases of standard types, and a walker builds
//! whatever issue type the constructor it is handed builds.

pub mod enums;

use core::iter::once;
use core::mem::take;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use proc_macro2::{Group, Spacing};
use proc_macro2::{Ident, Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens as _, format_ident, quote};
use syn::ext::IdentExt as _;
use syn::punctuated::Punctuated;
use syn::{
    Field, Fields, FieldsNamed, GenericArgument, GenericParam, Generics, ItemStruct, Lit, LitStr,
    PathArguments, PredicateType, Token, Type, TypeParamBound, TypePath, Variant, WherePredicate,
    parse_quote,
};

use crate::features::serde::{
    NAMED_READ_HOOK_PREFIX, SerdeFieldHooks, has_serde_default, has_serde_read_hook,
    has_serde_transparent, parse_serde_field_attributes, parse_serde_field_hooks,
    parse_serde_key_omission, parse_serde_type_attributes,
};
use crate::field_type::{
    FieldDefType, get_field_def, is_refused_sequence_wrapper, is_sequence_wrapper,
    is_transparent_wrapper,
};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{ident_schema_module_name, type_parameters_in_scope, written_type};

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

/// Where the walk of an object's fields leaves the keys that are the object's own.
enum Claimed {
    /// In the `declared` the walk binds: what a flattened type declares is known once it is walked.
    Bound,
    /// Nowhere: a flattened field takes every key nothing else declares, so every key is.
    Every,
    /// In these, known where the type is expanded.
    Listed(Vec<String>),
}

impl Claimed {
    /// The keys as a fields walker returns them for `object`.
    fn returned(&self, object: &Ident) -> TokenStream {
        match self {
            Self::Bound => quote! { declared },
            Self::Every => quote! { #object.keys().map(String::as_str).collect() },
            Self::Listed(keys) => quote! { vec![#(#keys),*] },
        }
    }

    /// What lists every key of `object`, held at `segments`, that is none of these as `Unknown`.
    fn undeclared(&self, object: &Ident, segments: &[TokenStream]) -> TokenStream {
        match self {
            Self::Bound => {
                let here = path_expression(&under(segments, &quote! { Ok(key.clone()) }));
                quote! {
                    for (key, held) in #object {
                        if !declared.contains(&key.as_str()) {
                            out.push(issue("Unknown", #here, &[], Some(held.clone()), None, Vec::new()));
                        }
                    }
                }
            }
            Self::Every => TokenStream::new(),
            Self::Listed(keys) => undeclared_keys(object, keys, segments),
        }
    }
}

/// A `#[serde(flatten)]` field, by how its keys are read out of the object that holds them.
enum Flattened<'item> {
    /// A map: every key nothing else declares, each value walked at its key.
    Entries(Walk<'item>),
    /// Another flagged type: its own fields walker, run in the same object.
    Model(&'item Type),
    /// An `Option` of a flagged type, which serde reads as absent where it does not read the type:
    /// that type, then the field's own, which an issue names as expected.
    Optional(&'item Type, &'item Type),
    /// A field no walk reaches: every key counts as its own, and serde's verdict is the read's.
    Unwalked,
    /// A value read whole from the keys nothing else declares.
    Whole(Walk<'item>),
}

impl<'item> Flattened<'item> {
    /// The keys it declares are its own alone, known once its type's fields walker has run.
    const fn declares_its_keys(&self) -> bool {
        matches!(self, Self::Model(_) | Self::Optional(_, _))
    }

    fn of(field: &'item Field, module_name: &str, parameters: &[String]) -> Self {
        // serde reads nothing into the field, and the keys it wrote for it are still in the object.
        if parse_serde_key_omission(&field.attrs).skips_deserializing {
            return Self::Unwalked;
        }
        let Walk { step, ty } = member_walk(field, module_name, parameters);
        match step {
            Step::Entries(values) => Self::Entries(*values),
            // Flattened, serde reads an `Option` as absent where the type's own reader refuses.
            Step::Leaf(whole) if whole.hooked || !get_field_def("", ty, "").is_optional() => {
                Self::Whole(Walk {
                    step: Step::Leaf(whole),
                    ty,
                })
            }
            Step::Model => Self::Model(ty),
            Step::Present(present) if matches!(present.step, Step::Model) => {
                Self::Optional(present.ty, ty)
            }
            Step::Items(_) | Step::Leaf(_) | Step::Positions(_) | Step::Present(_) => {
                Self::Unwalked
            }
        }
    }

    /// It reads the keys nothing else declares, so it needs to know which those are.
    const fn reads_the_rest(&self) -> bool {
        matches!(self, Self::Entries(_) | Self::Whole(_))
    }
}

/// The fields of an object as its walker reads them.
struct Keyed<'item> {
    /// Every key the fields are read under.
    declared: Vec<String>,
    fields: Vec<WalkedField<'item>>,
    /// The `#[serde(flatten)]` fields, whose keys sit among the object's own.
    flattened: Vec<Flattened<'item>>,
}

/// The walk of an object's fields, and where it leaves the object's keys.
struct KeyedWalk {
    claimed: Claimed,
    /// The walk lists an issue, so it reads the `Vec` it is handed.
    lists: bool,
    walk: TokenStream,
}

impl KeyedWalk {
    /// The walk, then every key of `object`, held at `segments`, that is not its own as `Unknown`.
    fn checked(&self, object: &Ident, segments: &[TokenStream]) -> TokenStream {
        let (walk, undeclared) = (&self.walk, self.claimed.undeclared(object, segments));
        quote! {
            #walk
            #undeclared
        }
    }

    /// The walk, then the keys of `object` that are its own, as a fields walker returns them.
    fn returning(&self, object: &Ident) -> TokenStream {
        let (walk, returned) = (&self.walk, self.claimed.returned(object));
        quote! {
            #walk
            #returned
        }
    }
}

/// The key a value is looked up under in the object that holds it.
struct Lookup<'key> {
    /// serde reads the value when its key is missing.
    absence_is_read: bool,
    aliases: &'key [String],
    key: &'key str,
}

impl Lookup<'_> {
    /// The path segment of the key the value is found under: the one stored, where it has aliases.
    fn segment(&self) -> TokenStream {
        if self.aliases.is_empty() {
            let key = self.key;
            quote! { Ok(#key.to_owned()) }
        } else {
            quote! { Ok(stored.to_owned()) }
        }
    }
}

/// What the flag adds to a type.
pub struct RecoveringDecode {
    /// The items added to the `{type}_schema` module a schema surface writes, or that module whole
    /// in a build where none writes one.
    pub schema_module: TokenStream,
    /// The `impl` holding the entry points and the walkers, or one per source on a type with a
    /// type parameter.
    pub type_impl: TokenStream,
}

/// What the walker of a struct, or of what a variant holds, walks: the form serde writes it in.
enum Shape<'item> {
    /// An object of the fields, under the keys they declare.
    Fields(Keyed<'item>),
    /// A single slot, or the one field of a `#[serde(transparent)]` struct: the value it holds.
    Held(Walk<'item>),
    /// No field: the `{}` tixschema makes a unit struct write, and nothing under a variant's name.
    Nothing,
    /// Several slots: an array of them.
    Slots(Vec<Slot<'item>>),
}

impl<'item> Shape<'item> {
    fn of(item_struct: &'item ItemStruct, module_name: &str, parameters: &[String]) -> Self {
        if has_serde_transparent(&item_struct.attrs)
            && let Some(only) = transparent_field(&item_struct.fields)
        {
            return Self::Held(held_walk(only, module_name, parameters));
        }
        let defaulted = has_serde_default(&item_struct.attrs);
        match &item_struct.fields {
            Fields::Named(named) => {
                let container = parse_serde_type_attributes(&item_struct.attrs);
                let mut keyed = walked_fields(
                    named,
                    container.rename_all.as_deref(),
                    defaulted,
                    module_name,
                    parameters,
                );
                // serde writes a struct's `tag` as a key of the object and reads past it.
                keyed.declared.extend(container.tag);
                Self::Fields(keyed)
            }
            Fields::Unit => Self::Nothing,
            Fields::Unnamed(slots) => {
                let mut written = slots.unnamed.iter();
                if let (Some(only), None) = (written.next(), written.next()) {
                    return Self::Held(held_walk(only, module_name, parameters));
                }
                Self::Slots(
                    slots
                        .unnamed
                        .iter()
                        .filter_map(|slot| walked_slot(slot, defaulted, module_name, parameters))
                        .collect(),
                )
            }
        }
    }

    /// What `variant` holds. Its fields are cased by its own `rename_all`, and by the enum's
    /// `rename_all_fields` where it writes none.
    fn of_variant(
        variant: &'item Variant,
        rename_all_fields: Option<&str>,
        module_name: &str,
        parameters: &[String],
    ) -> Self {
        match &variant.fields {
            Fields::Named(named) => {
                let own = parse_serde_type_attributes(&variant.attrs).rename_all;
                Self::Fields(walked_fields(
                    named,
                    own.as_deref().or(rename_all_fields),
                    false,
                    module_name,
                    parameters,
                ))
            }
            Fields::Unit => Self::Nothing,
            Fields::Unnamed(slots) => {
                let mut written = slots.unnamed.iter();
                if let (Some(only), None) = (written.next(), written.next()) {
                    // serde writes a variant whose one slot it neither writes nor reads as a unit
                    // variant.
                    if parse_serde_key_omission(&only.attrs).absent_from_wire() {
                        return Self::Nothing;
                    }
                    return Self::Held(member_walk(only, module_name, parameters));
                }
                Self::Slots(
                    slots
                        .unnamed
                        .iter()
                        .filter_map(|slot| walked_slot(slot, false, module_name, parameters))
                        .collect(),
                )
            }
        }
    }
}

/// One position of a tuple as its walker reads it.
struct Slot<'item> {
    /// serde reads the tuple when the position is missing.
    absence_is_read: bool,
    ty: &'item Type,
    walk: Walk<'item>,
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

    /// `generics` with what this source's walker reads and writes a value with joined to every type
    /// parameter and to the type itself, which carries whatever more serde's derive asks of one. A
    /// parameter stays bounded in one place: its `where` predicate if it has one, else its name.
    fn bounded(self, generics: &Generics) -> Generics {
        let bounds: Punctuated<TypeParamBound, Token![+]> = match self {
            #[cfg(feature = "bson")]
            Self::Bson => parse_quote!(serde::de::DeserializeOwned + serde::Serialize),
            Self::Json => parse_quote!(serde::de::DeserializeOwned),
        };
        let mut bounded = generics.clone();
        bounded
            .make_where_clause()
            .predicates
            .push(parse_quote!(Self: #bounds));
        let mut predicates: Vec<&mut PredicateType> = bounded
            .where_clause
            .iter_mut()
            .flat_map(|clause| &mut clause.predicates)
            .filter_map(|predicate| {
                if let WherePredicate::Type(bounding) = predicate {
                    Some(bounding)
                } else {
                    None
                }
            })
            .collect();
        for parameter in &mut bounded.params {
            let GenericParam::Type(declared) = parameter else {
                continue;
            };
            let bounding = predicates.iter_mut().find(|bounding| {
                matches!(&bounding.bounded_ty, Type::Path(named) if named.path.is_ident(&declared.ident))
            });
            if let Some(in_where) = bounding {
                in_where.bounds.extend(take(&mut declared.bounds));
                in_where.bounds.extend(bounds.clone());
            } else {
                declared.bounds.extend(bounds.clone());
            }
        }
        bounded
    }

    /// The pattern a map is held under, binding its entries.
    fn entries(self, entries: &Ident) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Bson::Document(#entries) },
            Self::Json => quote! { serde_json::Value::Object(#entries) },
        }
    }

    /// The entry point and the report it runs.
    fn entry_methods(self, module: &Ident, decider: &Ident) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => bson_entry_methods(module, decider),
            Self::Json => entry_methods(module, decider),
        }
    }

    /// What serde reads `found` from.
    fn found_reader(self) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Deserializer::new(found.clone()) },
            Self::Json => quote! { found },
        }
    }

    /// Whether this source's walker is bound to write back a value of a parameter's type, and the
    /// type itself where it has one: the BSON walker asks `Serialize` of both.
    const fn is_bound_to_write(self) -> bool {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => true,
            Self::Json => false,
        }
    }

    /// The pattern a list is held under, binding its items.
    fn items(self, items: &Ident) -> TokenStream {
        let value = self.value();
        quote! { #value::Array(#items) }
    }

    /// What binds `items` to the positions of `found`, when it holds an array.
    fn items_of_found(self) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Bson::Array(items) = found },
            Self::Json => quote! { Some(items) = found.as_array() },
        }
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

    /// The value that holds `entries` as an object.
    fn object_from(self, entries: &TokenStream) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Bson::Document(#entries) },
            Self::Json => quote! { serde_json::Value::Object(#entries) },
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

    /// What serde reads the whole of `object` from.
    fn object_reader(self, object: &Ident) -> TokenStream {
        let whole = self.object_value(object);
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Deserializer::new(#whole) },
            Self::Json => whole,
        }
    }

    /// The whole of `object` as one value.
    fn object_value(self, object: &Ident) -> TokenStream {
        self.object_from(&quote! { #object.clone() })
    }

    /// The source's name inside what the flag adds for it.
    const fn stem(self) -> &'static str {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => "bson",
            Self::Json => "value",
        }
    }

    /// The pattern text is held under, binding it.
    fn text(self, text: &Ident) -> TokenStream {
        let value = self.value();
        quote! { #value::String(#text) }
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

    /// [`Self::write_back`], or the closure that writes nothing back for a value whose type names
    /// one of the type's own parameters where this source's walker is not bound to write one.
    fn written(
        self,
        parameterized: bool,
        pinned: Option<&Type>,
        through: Option<&TokenStream>,
    ) -> TokenStream {
        if !parameterized || self.is_bound_to_write() {
            return self.write_back(pinned, through);
        }
        pinned.map_or_else(|| quote! { |_| None }, |ty| quote! { |_: &#ty| None })
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
    /// A tuple: each position at its index.
    Positions(Vec<Slot<'ty>>),
    /// An optional value, walked when it is not `null`.
    Present(Box<Walk<'ty>>),
}

struct Walk<'ty> {
    step: Step<'ty>,
    ty: &'ty Type,
}

impl Walk<'_> {
    /// A value of a parameter's type, which is one value whatever holds it.
    const fn is_of_a_parameter(&self) -> bool {
        matches!(&self.step, Step::Leaf(whole) if whole.parameterized)
    }

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
    /// The value's type names one of the type's own parameters, which nothing shows to be
    /// `Serialize` where the JSON walker reads it.
    parameterized: bool,
    read: TokenStream,
    /// The function a hook writes the value to a serializer with, and `None` for the type's own
    /// `Serialize`.
    write: Option<TokenStream>,
}

/// What the generated code names of the type it is generated for, and the source it walks.
struct Walker<'item> {
    /// The type parameter each walker method builds its issues as.
    issue_parameter: &'item Ident,
    module: &'item Ident,
    own_name: &'item str,
    /// The type's own type parameters.
    parameters: &'item [String],
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
                parameterized,
                read,
                write,
            }) => {
                let expected = self.expected(walk.ty);
                let leaf = source.leaf();
                let written =
                    source.written(*parameterized, hooked.then_some(walk.ty), write.as_ref());
                vec![Arm {
                    body: quote! {
                        out.extend(#module::#leaf(#held, #read, #written, #path, #expected, issue))
                    },
                    nothing: false,
                    pattern: quote! { #held },
                }]
            }
            Step::Model => vec![Arm {
                body: model_walker_call(source, walk.ty, held, segments),
                nothing: false,
                pattern: quote! { #held },
            }],
            Step::Positions(slots) => {
                let items = binding("items", depth);
                let positions = self.positions(slots, &items, segments, depth);
                let expected = self.expected(reported);
                vec![
                    Arm {
                        body: quote! {{ #positions }},
                        nothing: false,
                        pattern: source.items(&items),
                    },
                    not_the_shape(held, &path, &expected, "not an array"),
                ]
            }
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
        def.erase_type_parameters(self.parameters);
        let members = def
            .expected_members()
            .into_iter()
            .map(|(member, names, count)| {
                let under = Literal::usize_unsuffixed(count);
                quote! { (#member, &[#(#names),*], #under) }
            });
        quote! { &[#(#members),*] }
    }

    /// What a fields walker runs for one field of the object held as `object` at `segments`: its
    /// key looked up under its name and every alias, and its value walked when it is there.
    fn field(
        &self,
        field: &WalkedField<'_>,
        object: &Ident,
        segments: &[TokenStream],
    ) -> TokenStream {
        let Some(walk) = &field.walk else {
            return TokenStream::new();
        };
        let lookup = Lookup {
            absence_is_read: field.absence_is_read,
            aliases: &field.aliases,
            key: &field.key,
        };
        let held = Ident::new("held", Span::call_site());
        let arms = self.arms(walk, &held, &under(segments, &lookup.segment()), 0, walk.ty);
        self.looked_up(&lookup, object, segments, &arms, &self.expected(field.ty))
    }

    /// `decode_with_{source}_fields` running `body`. A parameter `body` does not read is bound as
    /// `_`: the object, the path and the constructor together, and the list of issues on its own.
    fn fields_method(&self, reads_object: bool, lists: bool, body: &TokenStream) -> TokenStream {
        let (module, source, built) = (self.module, self.source, self.issue_parameter);
        let (fields, value, object_type) =
            (source.method("fields"), source.value(), source.object());
        let bound = |name: &str, read: bool| {
            if read {
                let named = Ident::new(name, Span::call_site());
                quote! { #named }
            } else {
                quote! { _ }
            }
        };
        let (object, path, issue, out) = (
            bound("object", reads_object),
            bound("path", reads_object),
            bound("issue", reads_object),
            bound("out", lists),
        );
        quote! {
            /// Lists every issue in this type's fields inside `object`, and returns the keys that are
            /// its own.
            pub fn #fields<'a, #built>(
                #object: &'a #object_type,
                #path: &[core::result::Result<String, usize>],
                #issue: #module::IssueFromParts<#value, #built>,
                #out: &mut Vec<#built>,
            ) -> Vec<&'a str> {
                #body
            }
        }
    }

    /// What walks a flattened flagged type in `object`, held at `segments`: its own fields walker,
    /// whose keys are kept in `declared` where `collects`.
    fn flattened_model(
        &self,
        model: &Type,
        object: &Ident,
        segments: &[TokenStream],
        collects: bool,
    ) -> TokenStream {
        let walked = flattened_walker_call(self.source, model, object, segments, &quote! { out });
        if collects {
            quote! { declared.extend(#walked); }
        } else {
            quote! { #walked; }
        }
    }

    /// What walks a flattened `Option` of a flagged type in `object`, held at `segments`. serde
    /// reads it as absent where it does not read the type, so with some of its keys there the
    /// object holds what the field would not write.
    fn flattened_optional(
        &self,
        model: &Type,
        written: &Type,
        object: &Ident,
        segments: &[TokenStream],
        collects: bool,
    ) -> TokenStream {
        let source = self.source;
        let walked =
            flattened_walker_call(source, model, object, segments, &quote! { &mut nested });
        let (reader, whole) = (source.object_reader(object), source.object_value(object));
        let (here, expected) = (path_expression(segments), self.expected(written));
        let kept = if collects {
            quote! { declared.extend(keys); }
        } else {
            TokenStream::new()
        };
        quote! {
            {
                let mut nested = Vec::new();
                let keys = #walked;
                if keys.iter().any(|key| #object.contains_key(*key)) {
                    match <#model as serde::Deserialize>::deserialize(#reader) {
                        Ok(_) => out.append(&mut nested),
                        Err(_) => out.push(issue("Mistyped", #here, #expected, Some(#whole), None, Vec::new())),
                    }
                }
                #kept
            }
        }
    }

    /// What walks a flattened field that takes the keys of `object` nothing else declares, each
    /// one a key `unclaimed` holds of, and every one where there is no such test.
    fn flattened_rest(
        &self,
        flattened: &Flattened<'_>,
        object: &Ident,
        segments: &[TokenStream],
        unclaimed: Option<&TokenStream>,
    ) -> TokenStream {
        let (module, source) = (self.module, self.source);
        match flattened {
            Flattened::Entries(values) => {
                let item = Ident::new("item", Span::call_site());
                let each = self.statement(
                    values,
                    &item,
                    &under(segments, &quote! { Ok(key.clone()) }),
                    0,
                );
                unclaimed.map_or_else(
                    || quote! { for (key, #item) in #object { #each } },
                    |test| quote! { for (key, #item) in #object { if #test { #each } } },
                )
            }
            Flattened::Whole(Walk {
                step:
                    Step::Leaf(Whole {
                        hooked,
                        parameterized,
                        read,
                        write,
                    }),
                ty,
            }) => {
                let rest = unclaimed.map_or_else(
                    || source.object_value(object),
                    |test| {
                        source.object_from(&quote! {
                            #object
                                .iter()
                                .filter(|(key, _)| #test)
                                .map(|(key, held)| (key.clone(), held.clone()))
                                .collect()
                        })
                    },
                );
                let (leaf, here, expected) =
                    (source.leaf(), path_expression(segments), self.expected(ty));
                let written = source.written(*parameterized, hooked.then_some(*ty), write.as_ref());
                quote! {
                    out.extend(#module::#leaf(&#rest, #read, #written, #here, #expected, issue));
                }
            }
            Flattened::Model(_)
            | Flattened::Optional(_, _)
            | Flattened::Unwalked
            | Flattened::Whole(_) => TokenStream::new(),
        }
    }

    /// What binds the form `found` is walked in, as `held_as` binds it. A value held in any other
    /// form is read whole with the type's own reader, so what is listed for it is serde's verdict.
    fn held_as(&self, held_as: &TokenStream) -> TokenStream {
        let whole = self.read_whole(&self.own_model());
        quote! {
            let #held_as else {
                #whole;
                return;
            };
        }
    }

    /// The walker of a struct serde writes as the value one field of it holds: a single-slot tuple
    /// struct, and a `#[serde(transparent)]` struct of either kind.
    fn held_methods(&self, walk: &Walk<'_>) -> TokenStream {
        let source = self.source;
        let found = Ident::new("found", Span::call_site());
        match &walk.step {
            Step::Entries(_) | Step::Items(_) | Step::Positions(_) | Step::Present(_) => self
                .issues_method(&Self::listed(
                    &found,
                    &self.arms(walk, &found, &[], 0, walk.ty),
                )),
            // The type's own reader runs whatever hook its slot carries.
            Step::Leaf(_) => {
                let whole = self.read_whole(&self.expected(walk.ty));
                self.issues_method(&quote! { #whole; })
            }
            Step::Model => {
                let (ty, issues, fields) =
                    (walk.ty, source.method("issues"), source.method("fields"));
                let walked = self.issues_method(&quote! {
                    <#ty>::#issues(found, path, issue, out);
                });
                let keyed = self.fields_method(
                    true,
                    true,
                    &quote! { <#ty>::#fields(object, path, issue, out) },
                );
                quote! {
                    #walked
                    #keyed
                }
            }
        }
    }

    /// `decode_with_{source}_issues` running `body`.
    fn issues_method(&self, body: &TokenStream) -> TokenStream {
        let issues = self.source.method("issues");
        let signature = self.signature();
        quote! {
            /// Lists every issue in `found`, read as this type at `path`.
            pub fn #issues #signature {
                #body
            }
        }
    }

    /// What walks the fields of `object`, held at `segments`, the flattened ones after the ones
    /// read under a key of their own, and where that leaves the object's keys, `tag` first among
    /// them where the object carries one. A caller that reads the keys afterwards says so in
    /// `reads_keys`.
    fn keyed(
        &self,
        keyed: &Keyed<'_>,
        tag: Option<&str>,
        object: &Ident,
        segments: &[TokenStream],
        reads_keys: bool,
    ) -> KeyedWalk {
        let walks: Vec<TokenStream> = keyed
            .fields
            .iter()
            .map(|field| self.field(field, object, segments))
            .collect();
        let own: Vec<String> = tag
            .map(str::to_owned)
            .into_iter()
            .chain(keyed.declared.iter().cloned())
            .collect();
        let lists = !walks.iter().all(TokenStream::is_empty);
        let flattened = &keyed.flattened;
        if flattened.is_empty() {
            return KeyedWalk {
                claimed: Claimed::Listed(own),
                lists,
                walk: quote! { #(#walks)* },
            };
        }
        let declaring = flattened.iter().any(Flattened::declares_its_keys);
        let takes_the_rest = flattened.iter().any(|field| !field.declares_its_keys());
        // serde hands the first field that takes the rest every key the walker hands it. What that
        // one leaves for the next is nothing the walker can know, so no other is walked.
        let reader = flattened.iter().find(|field| field.reads_the_rest());
        // The keys the flattened types declare are kept only where something reads them: the
        // flattened field that reads the rest, or the caller.
        let collects = declaring && (reader.is_some() || (reads_keys && !takes_the_rest));
        let bound = if !collects {
            TokenStream::new()
        } else if own.is_empty() {
            quote! { let mut declared = Vec::new(); }
        } else {
            quote! { let mut declared = vec![#(#own),*]; }
        };
        let unclaimed = if collects {
            Some(quote! { !declared.contains(&key.as_str()) })
        } else if own.is_empty() {
            None
        } else {
            Some(quote! { !matches!(key.as_str(), #(#own)|*) })
        };
        let declared = flattened.iter().map(|field| match field {
            Flattened::Model(model) => self.flattened_model(model, object, segments, collects),
            Flattened::Optional(model, written) => {
                self.flattened_optional(model, written, object, segments, collects)
            }
            Flattened::Entries(_) | Flattened::Unwalked | Flattened::Whole(_) => TokenStream::new(),
        });
        let rest =
            reader.map(|field| self.flattened_rest(field, object, segments, unclaimed.as_ref()));
        KeyedWalk {
            claimed: if takes_the_rest {
                Claimed::Every
            } else {
                Claimed::Bound
            },
            lists: lists || declaring || reader.is_some(),
            walk: quote! {
                #bound
                #(#walks)*
                #(#declared)*
                #rest
            },
        }
    }

    /// `arms` as the statement listing the issues of the value held under `held`.
    fn listed(held: &Ident, arms: &[Arm]) -> TokenStream {
        if let [only] = arms {
            let body = &only.body;
            return quote! { #body; };
        }
        let listed = arms.iter().map(|arm| {
            let (pattern, body) = (&arm.pattern, &arm.body);
            quote! { #pattern => #body, }
        });
        quote! { match #held { #(#listed)* } }
    }

    /// What lists the issues of the value `object` holds under a key: `arms` when it is there, and
    /// `Missing` naming `expected` when serde needs it.
    fn looked_up(
        &self,
        lookup: &Lookup<'_>,
        object: &Ident,
        segments: &[TokenStream],
        arms: &[Arm],
        expected: &TokenStream,
    ) -> TokenStream {
        let key = lookup.key;
        let under_alias = !lookup.aliases.is_empty();
        let found = if under_alias {
            let aliases = lookup.aliases;
            quote! {
                [#key, #(#aliases),*]
                    .into_iter()
                    .find_map(|stored| #object.get(stored).map(|held| (stored, held)))
            }
        } else {
            quote! { #object.get(#key) }
        };
        let there = |pattern: &TokenStream| {
            if under_alias {
                quote! { Some((stored, #pattern)) }
            } else {
                quote! { Some(#pattern) }
            }
        };
        if lookup.absence_is_read
            && let [only] = arms
        {
            let (pattern, body) = (there(&only.pattern), &only.body);
            return quote! {
                if let #pattern = #found {
                    #body;
                }
            };
        }
        // An absent key and a `null` are the same `None` to serde, so one arm answers both.
        let merged =
            lookup.absence_is_read && !under_alias && arms.first().is_some_and(|arm| arm.nothing);
        let listed = arms.iter().skip(usize::from(merged)).map(|arm| {
            let (pattern, body) = (there(&arm.pattern), &arm.body);
            quote! { #pattern => #body, }
        });
        let absent = if merged {
            let null = self.source.null();
            quote! { None | Some(#null) => {} }
        } else if lookup.absence_is_read {
            quote! { None => {} }
        } else {
            let here = path_expression(&under(segments, &quote! { Ok(#key.to_owned()) }));
            quote! { None => out.push(issue("Missing", #here, #expected, None, None, Vec::new())), }
        };
        if merged {
            quote! { match #found { #absent #(#listed)* } }
        } else {
            quote! { match #found { #(#listed)* #absent } }
        }
    }

    /// The methods one type's walker is called through by another's.
    fn methods(&self, shape: &Shape<'_>) -> TokenStream {
        match shape {
            Shape::Fields(keyed) => self.object_methods(keyed),
            Shape::Held(walk) => self.held_methods(walk),
            Shape::Nothing => self.unit_methods(),
            Shape::Slots(slots) => self.positional_methods(slots),
        }
    }

    /// `decode_with_{source}_issues` of a type serde writes as an object: every key
    /// `decode_with_{source}_fields` does not return as the type's own is `Unknown`.
    fn object_issues_method(&self) -> TokenStream {
        let source = self.source;
        let fields = source.method("fields");
        let held_as = self.held_as(&source.object_of_found());
        self.issues_method(&quote! {
            #held_as
            let declared = Self::#fields(object, path, issue, out);
            for (key, held) in object {
                if !declared.contains(&key.as_str()) {
                    out.push(issue("Unknown", [path, &[Ok(key.clone())]].concat(), &[], Some(held.clone()), None, Vec::new()));
                }
            }
        })
    }

    /// The walker of a struct serde writes as an object of its fields.
    fn object_methods(&self, keyed: &Keyed<'_>) -> TokenStream {
        let object = Ident::new("object", Span::call_site());
        let keyed_walk = self.keyed(keyed, None, &object, &[], true);
        let walked = self.object_issues_method();
        // A type with no field to walk lists no issue of its own, so the `Vec` it is handed goes
        // unbound.
        let fields = self.fields_method(true, keyed_walk.lists, &keyed_walk.returning(&object));
        quote! {
            #walked

            #fields
        }
    }

    /// What an issue names as expected where the type itself is: `Model`, by the type's name.
    fn own_model(&self) -> TokenStream {
        let own_name = self.own_name;
        quote! { &[("Model", &[#own_name], 0)] }
    }

    /// What lists the issues of the position `at` among `items`: its value walked when it is
    /// there, and `Missing` when serde needs it.
    fn position(
        &self,
        slot: &Slot<'_>,
        items: &Ident,
        at: usize,
        segments: &[TokenStream],
        depth: usize,
    ) -> TokenStream {
        let held = binding("held", depth);
        let index = Literal::usize_unsuffixed(at);
        let here = under(segments, &quote! { Err(#index) });
        let lookup = if at == 0 {
            quote! { #items.first() }
        } else {
            quote! { #items.get(#index) }
        };
        let arms = self.arms(
            &slot.walk,
            &held,
            &here,
            depth.saturating_add(1),
            slot.walk.ty,
        );
        if slot.absence_is_read
            && let [only] = arms.as_slice()
        {
            let (pattern, body) = (&only.pattern, &only.body);
            return quote! {
                if let Some(#pattern) = #lookup {
                    #body;
                }
            };
        }
        // An absent position and a `null` are the same `None` to serde, so one arm answers both.
        let merged = slot.absence_is_read && arms.first().is_some_and(|arm| arm.nothing);
        let listed = arms.iter().skip(usize::from(merged)).map(|arm| {
            let (pattern, body) = (&arm.pattern, &arm.body);
            quote! { Some(#pattern) => #body, }
        });
        if merged {
            let null = self.source.null();
            return quote! { match #lookup { None | Some(#null) => {} #(#listed)* } };
        }
        let absent = if slot.absence_is_read {
            quote! { None => {} }
        } else {
            let path = path_expression(&here);
            let expected = self.expected(slot.ty);
            quote! { None => out.push(issue("Missing", #path, #expected, None, None, Vec::new())), }
        };
        quote! { match #lookup { #(#listed)* #absent } }
    }

    /// The walker of a tuple struct, which serde writes as an array of its slots.
    fn positional_methods(&self, slots: &[Slot<'_>]) -> TokenStream {
        let held_as = self.held_as(&self.source.items_of_found());
        let positions = self.positions(slots, &Ident::new("items", Span::call_site()), &[], 0);
        self.issues_method(&quote! {
            #held_as
            #positions
        })
    }

    /// What lists the issues of a tuple held as `items`: each position the tuple declares, then
    /// every one it does not as `Unknown`.
    fn positions(
        &self,
        slots: &[Slot<'_>],
        items: &Ident,
        segments: &[TokenStream],
        depth: usize,
    ) -> TokenStream {
        let declared = slots
            .iter()
            .enumerate()
            .map(|(at, slot)| self.position(slot, items, at, segments, depth));
        let (index, held) = (binding("index", depth), binding("held", depth));
        let here = path_expression(&under(segments, &quote! { Err(#index) }));
        let undeclared = if slots.is_empty() {
            quote! { #items.iter().enumerate() }
        } else {
            let count = Literal::usize_unsuffixed(slots.len());
            quote! { #items.iter().enumerate().skip(#count) }
        };
        quote! {
            #(#declared)*
            for (#index, #held) in #undeclared {
                out.push(issue("Unknown", #here, &[], Some(#held.clone()), None, Vec::new()));
            }
        }
    }

    /// What lists `found` read whole with the type's own reader: serde's verdict on it, at the
    /// value, naming `expected`.
    fn read_whole(&self, expected: &TokenStream) -> TokenStream {
        let (module, leaf) = (self.module, self.source.leaf());
        let written = self.written_whole();
        quote! {
            out.extend(#module::#leaf(found, <Self as serde::Deserialize>::deserialize, #written, path.to_vec(), #expected, issue))
        }
    }

    /// What every method listing the issues of one value takes, after its name.
    fn signature(&self) -> TokenStream {
        let (module, built) = (self.module, self.issue_parameter);
        let value = self.source.value();
        quote! {
            <#built>(
                found: &#value,
                path: &[core::result::Result<String, usize>],
                issue: #module::IssueFromParts<#value, #built>,
                out: &mut Vec<#built>,
            )
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
        Self::listed(
            held,
            &self.arms(walk, held, segments, depth.saturating_add(1), walk.ty),
        )
    }

    /// The walker of a unit struct: an object in which no key is the type's own.
    fn unit_methods(&self) -> TokenStream {
        let held_as = self.held_as(&self.source.object_of_found());
        let undeclared = undeclared_keys(&Ident::new("object", Span::call_site()), &[], &[]);
        let walked = self.issues_method(&quote! {
            #held_as
            #undeclared
        });
        let keyed = self.fields_method(false, false, &quote! { Vec::new() });
        quote! {
            #walked
            #keyed
        }
    }

    /// The closure writing back the type's own whole value, read with its own reader.
    fn written_whole(&self) -> TokenStream {
        self.source.written(!self.parameters.is_empty(), None, None)
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

/// The recovering decode of a struct, read off the item as it is emitted, so a read hook tixschema
/// hangs on a field is one the walker reads through.
pub fn struct_recovering_decode(item_struct: &ItemStruct) -> RecoveringDecode {
    let module_name = ident_schema_module_name(&item_struct.ident.to_string());
    let parameters = type_parameters_in_scope(&item_struct.generics);
    let shape = Shape::of(item_struct, &module_name, &parameters);
    added_to(
        &item_struct.ident,
        &item_struct.generics,
        &module_name,
        &parameters,
        &written_names(item_struct.to_token_stream()),
        |walker| walker.methods(&shape),
    )
}

/// What the flag adds to the type `name` declares under `generics`: the callback's types, and per
/// source the entry point beside what `methods` writes for that source's walker. `written` is
/// every name the type's item writes, none of which a method's own type parameter takes.
fn added_to<M>(
    name: &Ident,
    generics: &Generics,
    module_name: &str,
    parameters: &[String],
    written: &[String],
    methods: M,
) -> RecoveringDecode
where
    M: Fn(&Walker<'_>) -> TokenStream,
{
    let own_name = name.to_string();
    let module = Ident::new(module_name, Span::call_site());
    let decider = unclaimed_parameter("F", written);
    let issue_parameter = unclaimed_parameter("I", written);
    let of_sources: Vec<TokenStream> = Source::GENERATED
        .iter()
        .map(|&source| {
            let walker = Walker {
                issue_parameter: &issue_parameter,
                module: &module,
                own_name: &own_name,
                parameters,
                source,
            };
            let entry = source.entry_methods(&module, &decider);
            let walk_methods = methods(&walker);
            quote! {
                #entry
                #walk_methods
            }
        })
        .collect();
    RecoveringDecode {
        schema_module: placed_in_schema_module(&module, &module_items()),
        type_impl: type_impls(name, generics, !parameters.is_empty(), &of_sources),
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
fn bson_entry_methods(module: &Ident, decider: &Ident) -> TokenStream {
    quote! {
        /// Reads `document` as this type, handing every issue found in it to `decide`, once.
        pub fn from_bson_with<#decider>(
            mut document: bson::Document,
            decide: #decider,
        ) -> core::result::Result<Self, #module::Unrecovered<bson::Bson>>
        where
            #decider: FnOnce(&mut bson::Document, &[#module::Issue<bson::Bson>]) -> #module::Verdict,
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
fn entry_methods(module: &Ident, decider: &Ident) -> TokenStream {
    quote! {
        /// Reads `value` as this type, handing every issue found in it to `decide`, once.
        pub fn from_value_with<#decider>(
            mut value: serde_json::Value,
            decide: #decider,
        ) -> core::result::Result<Self, #module::Unrecovered<serde_json::Value>>
        where
            #decider: FnOnce(&mut serde_json::Value, &[#module::Issue<serde_json::Value>]) -> #module::Verdict,
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

/// Hands the object a flattened field's keys sit in to the fields walker of the model type the
/// field is declared as, which lists into `out` and returns the keys that are that type's own.
///
/// A type a flagged type flattens carries the flag too, and is one serde can flatten. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Deserialize, Serialize)]
/// pub struct Audit {
///     pub revision: i32,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Deserialize, Serialize)]
/// pub struct Note {
///     #[serde(flatten)]
///     pub audit: Audit,
///     pub text: String,
/// }
///
/// fn main() {}
/// ```
///
/// The run below is that one with the flag taken off `Audit`, and nothing else changed:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// pub struct Audit {
///     pub revision: i32,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Deserialize, Serialize)]
/// pub struct Note {
///     #[serde(flatten)]
///     pub audit: Audit,
///     pub text: String,
/// }
///
/// fn main() {}
/// ```
///
/// A `compile_fail` doctest asserts only that some error was raised, so the snippet was compiled
/// standalone as an ordinary test file, and this is the only error it earned, verbatim:
///
/// ```text
/// error[E0599]: no associated function or constant named `decode_with_value_fields` found for struct `Audit` in the current scope
///   --> tests/zz_probe.rs:10:1
///    |
///  6 | pub struct Audit {
///    | ---------------- associated function or constant `decode_with_value_fields` not found for this struct
/// ...
/// 10 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ associated function or constant not found in `Audit`
///    |
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// A build with `bson` on earns the same error a second time, naming `decode_with_bson_fields`.
fn flattened_walker_call(
    source: Source,
    model: &Type,
    object: &Ident,
    segments: &[TokenStream],
    out: &TokenStream,
) -> TokenStream {
    let fields = source.method("fields");
    if segments.is_empty() {
        return quote! { <#model>::#fields(#object, path, issue, #out) };
    }
    let path = path_expression(segments);
    quote! { <#model>::#fields(#object, &#path, issue, #out) }
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

/// `inner` held in a list or a map written as `ty`. What holds a value of a parameter's type is
/// read whole, as that value is.
fn held_in<'ty>(
    step: fn(Box<Walk<'ty>>) -> Step<'ty>,
    inner: Walk<'ty>,
    ty: &'ty Type,
    parameters: &[String],
) -> Walk<'ty> {
    if inner.is_of_a_parameter() {
        plain_value(ty, parameters)
    } else {
        Walk {
            step: step(Box::new(inner)),
            ty,
        }
    }
}

/// How the value a single-slot tuple struct holds is walked, or the one a `#[serde(transparent)]`
/// struct holds in its field. A hook on the slot is one the type's own reader runs, so the slot is
/// then one value whatever its type holds.
fn held_walk<'item>(slot: &'item Field, module_name: &str, parameters: &[String]) -> Walk<'item> {
    let hooks = parse_serde_field_hooks(&slot.attrs);
    if hooked_leaf(&slot.ty, &hooks, module_name, parameters).is_some() {
        plain_value(&slot.ty, parameters)
    } else {
        walk_of(&slot.ty, parameters)
    }
}

/// The functions a field is read and written back through when its author's serde attributes
/// name any, and `None` for a field its own type reads.
fn hooked_leaf(
    ty: &Type,
    hooks: &SerdeFieldHooks,
    module_name: &str,
    parameters: &[String],
) -> Option<Step<'static>> {
    let generated = format!("{module_name}::{NAMED_READ_HOOK_PREFIX}");
    let named = |hook: Option<&LitStr>| {
        let path = hook.filter(|path| !path.value().starts_with(&generated))?;
        path.parse::<syn::ExprPath>()
            .ok()
            .map(resolved_at_the_mixed_site)
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
        parameterized: names_a_parameter(ty, parameters),
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

/// How the value of a field or a slot is walked: through its author's hooks, or by its type.
fn member_walk<'item>(
    member: &'item Field,
    module_name: &str,
    parameters: &[String],
) -> Walk<'item> {
    let hooks = parse_serde_field_hooks(&member.attrs);
    hooked_leaf(&member.ty, &hooks, module_name, parameters).map_or_else(
        || walk_of(&member.ty, parameters),
        |step| Walk {
            step,
            ty: &member.ty,
        },
    )
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
///
/// An alias is the type it stands for, so the call reaches whatever the alias names. A field typed
/// with an alias of a flagged model type builds. One typed with an alias of a list does not, where
/// the first example, which writes the list in full, does:
///
/// ```rust,compile_fail
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
/// #[model_schema()]
/// pub type Versions = Vec<Version>;
///
/// #[model_schema(decode_with)]
/// #[derive(Deserialize, Serialize)]
/// pub struct Stock {
///     pub all: Versions,
/// }
///
/// fn main() {}
/// ```
///
/// Compiled standalone the same way, this is the only error it earned, verbatim but for a note
/// listing the constructors `Vec` has, left out where the dots are:
///
/// ```text
/// error[E0599]: no associated function or constant named `decode_with_value_issues` found for struct `Vec<Version>` in the current scope
///    --> tests/zz_probe.rs:13:1
///     |
///  13 | #[model_schema(decode_with)]
///     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ associated function or constant not found in `Vec<Version>`
///     |
/// ...
///     = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn model_walker_call(
    source: Source,
    ty: &Type,
    held: &Ident,
    segments: &[TokenStream],
) -> TokenStream {
    let issues = source.method("issues");
    if segments.is_empty() {
        return quote! { <#ty>::#issues(#held, path, issue, out) };
    }
    let path = path_expression(segments);
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

/// Whether `ty` names one of the type's own `parameters`, or the type itself where it has any.
fn names_a_parameter(ty: &Type, parameters: &[String]) -> bool {
    if parameters.is_empty() {
        return false;
    }
    let written = written_type(ty);
    if let Type::Path(type_path) = written {
        starts_at_a_parameter(type_path, parameters)
            || type_path.path.is_ident("Self")
            || type_path
                .qself
                .iter()
                .any(|qself| names_a_parameter(&qself.ty, parameters))
            || type_path.path.segments.iter().any(|segment| {
                type_arguments(&segment.arguments).any(|inner| names_a_parameter(inner, parameters))
            })
    } else if let Type::Tuple(tuple) = written {
        tuple
            .elems
            .iter()
            .any(|position| names_a_parameter(position, parameters))
    } else if let Type::Array(array) = written {
        names_a_parameter(&array.elem, parameters)
    } else if let Type::Slice(slice) = written {
        names_a_parameter(&slice.elem, parameters)
    } else if let Type::Paren(paren) = written {
        names_a_parameter(&paren.elem, parameters)
    } else {
        false
    }
}

/// The arm for a value that is not the list, the map or the tuple its type is.
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
    if segments.is_empty() {
        return quote! { path.to_vec() };
    }
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
fn path_step<'ty>(type_path: &'ty TypePath, ty: &'ty Type, parameters: &[String]) -> Walk<'ty> {
    let Some(segment) = type_path.path.segments.last() else {
        return plain_value(ty, parameters);
    };
    let name = segment.ident.to_string();
    let mut held = type_arguments(&segment.arguments);
    let (first, second) = (held.next(), held.next());
    if name == "Option"
        && let Some(inner) = first
    {
        let optional = walk_of(inner, parameters);
        return if optional.reads_whole() {
            plain_value(ty, parameters)
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
        return held_in(Step::Items, walk_of(inner, parameters), ty, parameters);
    }
    if matches!(name.as_str(), "BTreeMap" | "HashMap")
        && let Some(inner) = second
    {
        return held_in(Step::Entries, walk_of(inner, parameters), ty, parameters);
    }
    // serde writes the wrapper as the value it holds, so what is walked is that value.
    if is_transparent_wrapper(&name)
        && let Some(inner) = first
    {
        let wrapped = walk_of(inner, parameters);
        return if wrapped.reads_whole() {
            plain_value(ty, parameters)
        } else {
            wrapped
        };
    }
    if !starts_at_a_parameter(type_path, parameters)
        && matches!(
            get_field_def("", ty, "").field_type,
            FieldDefType::SiblingType(_, _)
        )
    {
        Walk {
            step: Step::Model,
            ty,
        }
    } else {
        plain_value(ty, parameters)
    }
}

/// A value read whole with its type's own `Deserialize`, and written back with its `Serialize`.
fn plain_value<'ty>(ty: &'ty Type, parameters: &[String]) -> Walk<'ty> {
    Walk {
        step: Step::Leaf(Whole {
            hooked: false,
            parameterized: names_a_parameter(ty, parameters),
            read: own_reader(ty),
            write: None,
        }),
        ty,
    }
}

/// `hook` with each of its names resolved at the macro's mixed site, and located where its author
/// wrote it. At the call site, a hook named like a value the walker binds would name that value.
fn resolved_at_the_mixed_site(mut hook: syn::ExprPath) -> syn::ExprPath {
    for segment in &mut hook.path.segments {
        let written = segment.ident.span();
        segment
            .ident
            .set_span(written.resolved_at(Span::mixed_site()));
    }
    hook
}

/// Whether a path names one of the type's own parameters outright, or a type projected from one.
fn starts_at_a_parameter(type_path: &TypePath, parameters: &[String]) -> bool {
    type_path.qself.is_none()
        && type_path.path.segments.first().is_some_and(|segment| {
            parameters
                .iter()
                .any(|parameter| segment.ident == parameter)
        })
}

/// The field serde's derive reads a `#[serde(transparent)]` struct as the value of, named or a
/// slot: the one it reads that has no `default` and is not written `PhantomData`. `None` for any
/// other count, which that derive refuses.
fn transparent_field(fields: &Fields) -> Option<&Field> {
    let mut read = fields.iter().filter(|field| {
        let omission = parse_serde_key_omission(&field.attrs);
        let marker = matches!(
            written_type(&field.ty),
            Type::Path(written)
                if written.path.segments.last().is_some_and(|last| last.ident == "PhantomData")
        );
        !omission.skips_deserializing && !omission.defaulted && !marker
    });
    if let (Some(only), None) = (read.next(), read.next()) {
        Some(only)
    } else {
        None
    }
}

/// The types written between a path segment's angle brackets.
fn type_arguments(arguments: &PathArguments) -> impl Iterator<Item = &Type> {
    let angled = if let PathArguments::AngleBracketed(angled) = arguments {
        Some(angled)
    } else {
        None
    };
    angled
        .into_iter()
        .flat_map(|written| &written.args)
        .filter_map(|argument| {
            if let GenericArgument::Type(inner) = argument {
                Some(inner)
            } else {
                None
            }
        })
}

/// The `impl`s holding `methods`, which lists what each generated source adds. A type with a type
/// parameter gets one `impl` per source, under the bounds that source reads and writes a value
/// with, and every other type one `impl` for them all.
fn type_impls(
    name: &Ident,
    generics: &Generics,
    generic: bool,
    methods: &[TokenStream],
) -> TokenStream {
    if !generic {
        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
        return quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #(#methods)*
            }
        };
    }
    let impls = Source::GENERATED
        .iter()
        .zip(methods)
        .map(|(source, of_source)| {
            let bounded = source.bounded(generics);
            let (impl_generics, type_generics, where_clause) = bounded.split_for_impl();
            quote! {
                impl #impl_generics #name #type_generics #where_clause {
                    #of_source
                }
            }
        });
    quote! { #(#impls)* }
}

/// A name for a type parameter of a method's own: `base`, numbered where the item writes that
/// name. The method's parameter would hide a type of its name wherever the method's body writes
/// one, and be refused beside a parameter of the type's own.
fn unclaimed_parameter(base: &str, written: &[String]) -> Ident {
    let mut name = base.to_owned();
    let mut number = 1_u32;
    while written.contains(&name) {
        number += 1;
        name = format!("{base}{number}");
    }
    Ident::new(&name, Span::call_site())
}

/// What lists every key of `object`, held at `segments`, that is none of `declared` as `Unknown`.
fn undeclared_keys(object: &Ident, declared: &[String], segments: &[TokenStream]) -> TokenStream {
    let here = path_expression(&under(segments, &quote! { Ok(key.clone()) }));
    let unknown = quote! {
        out.push(issue("Unknown", #here, &[], Some(held.clone()), None, Vec::new()));
    };
    if declared.is_empty() {
        return quote! { for (key, held) in #object { #unknown } };
    }
    quote! {
        for (key, held) in #object {
            if !matches!(key.as_str(), #(#declared)|*) {
                #unknown
            }
        }
    }
}

/// `segments` with one more after them.
fn under(segments: &[TokenStream], segment: &TokenStream) -> Vec<TokenStream> {
    let mut longer = segments.to_vec();
    longer.push(segment.clone());
    longer
}

/// How a value of type `ty` is walked.
fn walk_of<'ty>(ty: &'ty Type, parameters: &[String]) -> Walk<'ty> {
    let written = written_type(ty);
    if let Type::Array(array) = written {
        held_in(
            Step::Items,
            walk_of(&array.elem, parameters),
            written,
            parameters,
        )
    } else if let Type::Slice(slice) = written {
        held_in(
            Step::Items,
            walk_of(&slice.elem, parameters),
            written,
            parameters,
        )
    } else if let Type::Tuple(tuple) = written
        && !tuple.elems.is_empty()
    {
        let slots: Vec<Slot<'ty>> = tuple
            .elems
            .iter()
            .map(|position| Slot {
                absence_is_read: false,
                ty: position,
                walk: walk_of(position, parameters),
            })
            .collect();
        if slots.iter().any(|slot| slot.walk.is_of_a_parameter()) {
            plain_value(written, parameters)
        } else {
            Walk {
                step: Step::Positions(slots),
                ty: written,
            }
        }
    } else if let Type::Path(type_path) = written {
        path_step(type_path, written, parameters)
    } else {
        plain_value(written, parameters)
    }
}

/// One field as the walker reads it, or `None` for a field serde neither writes nor reads.
fn walked_field<'item>(
    field: &'item Field,
    rename_all: Option<&str>,
    container_defaulted: bool,
    module_name: &str,
    parameters: &[String],
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
    let walk = (!omission.skips_deserializing).then(|| member_walk(field, module_name, parameters));
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

/// The fields serde writes or reads among `named`: the ones read under a key of their own with
/// every such key, and the flattened ones apart.
fn walked_fields<'item>(
    named: &'item FieldsNamed,
    rename_all: Option<&str>,
    container_defaulted: bool,
    module_name: &str,
    parameters: &[String],
) -> Keyed<'item> {
    let mut fields: Vec<WalkedField<'item>> = Vec::new();
    let mut flattened: Vec<Flattened<'item>> = Vec::new();
    for field in &named.named {
        if !parse_serde_field_attributes(&field.attrs).flatten {
            fields.extend(walked_field(
                field,
                rename_all,
                container_defaulted,
                module_name,
                parameters,
            ));
        } else if !parse_serde_key_omission(&field.attrs).absent_from_wire() {
            flattened.push(Flattened::of(field, module_name, parameters));
        } else {
            // serde neither writes nor reads the field, so no key in the object is its own.
        }
    }
    let declared = fields
        .iter()
        .flat_map(|field| once(&field.key).chain(&field.aliases))
        .cloned()
        .collect();
    Keyed {
        declared,
        fields,
        flattened,
    }
}

/// One slot of a tuple as the walker reads it, or `None` for a slot serde does not read:
/// the array holds no position for it.
fn walked_slot<'item>(
    slot: &'item Field,
    container_defaulted: bool,
    module_name: &str,
    parameters: &[String],
) -> Option<Slot<'item>> {
    let omission = parse_serde_key_omission(&slot.attrs);
    (!omission.skips_deserializing).then(|| Slot {
        absence_is_read: omission.defaulted || container_defaulted,
        ty: &slot.ty,
        walk: member_walk(slot, module_name, parameters),
    })
}

/// Every name `tokens` write: each identifier, and each word of a string, which is where a serde
/// attribute writes the path of a hook.
fn written_names(tokens: TokenStream) -> Vec<String> {
    tokens
        .into_iter()
        .flat_map(|token| match token {
            TokenTree::Group(group) => written_names(group.stream()),
            TokenTree::Ident(ident) => vec![ident.unraw().to_string()],
            TokenTree::Literal(literal) => {
                if let Lit::Str(text) = Lit::new(literal) {
                    text.value()
                        .split(|letter: char| !letter.is_alphanumeric() && letter != '_')
                        .filter(|word| !word.is_empty())
                        .map(str::to_owned)
                        .collect()
                } else {
                    Vec::new()
                }
            }
            TokenTree::Punct(_) => Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
