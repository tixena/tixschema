//! The recovering decode `#[model_schema(decode_with)]` turns on.
//!
//! A flagged type gets `from_value_with`, which reads a `serde_json::Value` with plain serde,
//! walks it in the form serde writes that type in, and hands every issue the walk finds to a
//! callback once. A build with `bson` on adds `from_bson_with`, the same read of a
//! `bson::Document`, written with what both major versions of the `bson` library have. The
//! callback's types go into the type's own `{type}_schema` module. Two flagged types share no
//! declaration: each module declares the same aliases of standard types, and a walker builds
//! whatever issue type the constructor it is handed builds.

mod aliases;
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

use self::aliases::Reach;
use crate::features::serde::{
    NAMED_READ_HOOK_PREFIX, SerdeFieldHooks, has_serde_default, has_serde_read_hook,
    has_serde_skip_serializing, has_serde_transparent, parse_serde_field_attributes,
    parse_serde_field_hooks, parse_serde_key_omission, parse_serde_type_attributes,
    transparent_field,
};
use crate::field_type::{
    FieldDefType, get_field_def, is_refused_sequence_wrapper, is_sequence_wrapper,
    is_transparent_wrapper,
};
use crate::model_schema::{helper_name_stem, recovering_bound_check};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{
    Declared, declared, has_field_validator, ident_schema_module_name, type_parameters_in_scope,
    written_type,
};

/// The type names the flag adds to a schema module.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const ADDED_TYPE_NAMES: [&str; 13] = [
    "Asked",
    "EntryOf",
    "Expected",
    "ExpectedToken",
    "Issue",
    "IssueFromParts",
    "Path",
    "ReadWhole",
    "Segment",
    "Taken",
    "TakenProbe",
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
            Self::Every => quote! { #object.keys().map(::std::string::String::as_str).collect() },
            Self::Listed(keys) => quote! { vec![#(#keys),*] },
        }
    }

    /// What lists every key of `object`, held at `segments`, that is none of these as `Unknown`.
    fn undeclared(&self, object: &Ident, segments: &[TokenStream]) -> TokenStream {
        match self {
            Self::Bound => {
                let here = path_expression(&under(
                    segments,
                    &quote! { ::core::result::Result::Ok(key.clone()) },
                ));
                quote! {
                    for (key, held) in #object {
                        if !declared.contains(&key.as_str()) {
                            out.push(issue("Unknown", #here, &[], ::core::option::Option::Some(held.clone()), ::core::option::Option::None, ::std::vec::Vec::new()));
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
    /// Another flagged type: its own fields walker, run in what serde hands it of the object.
    Model(&'item Type),
    /// An `Option` of a flagged type, which serde reads as absent where it does not read the type:
    /// that type, then the field's own, which an issue names as expected.
    Optional(&'item Type, &'item Type),
    /// A field no walk reaches: every key counts as its own, and serde's verdict is the read's.
    /// It holds the field's type, and `None` for a field serde never reads.
    Unwalked(Option<&'item Type>),
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
            return Self::Unwalked(None);
        }
        let Walk { step, ty } = member_walk(field, module_name, parameters);
        match step {
            Step::Entries(values) => Self::Entries(*values),
            // Flattened, serde reads an `Option` as absent where the type's own reader refuses, and
            // an id from the object it writes for one: neither is read here.
            Step::Leaf(whole)
                if whole.hooked
                    || !(get_field_def("", ty, "").is_optional() || holds_an_id(ty)) =>
            {
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
                Self::Unwalked(Some(&field.ty))
            }
        }
    }

    /// It reads the keys nothing else declares, so it needs to know which those are.
    const fn reads_the_rest(&self) -> bool {
        matches!(self, Self::Entries(_) | Self::Whole(_))
    }

    /// The function serde reads the field through and the type that function reads, where reading
    /// it can take entries out of what is left for the flattened fields declared after it. A map
    /// takes none.
    fn taker(&self) -> Option<(TokenStream, &'item Type)> {
        match self {
            Self::Entries(_) | Self::Unwalked(None) => None,
            Self::Model(read) | Self::Optional(_, read) | Self::Unwalked(Some(read)) => {
                Some((own_reader(read), read))
            }
            Self::Whole(walk) => Some(match &walk.step {
                Step::Leaf(whole) => (whole.read.clone(), walk.ty),
                Step::Entries(_)
                | Step::Items(_)
                | Step::Model
                | Step::Positions(_)
                | Step::Present(_) => (own_reader(walk.ty), walk.ty),
            }),
        }
    }
}

/// What a flattened type's fields walker is handed, which is what serde hands the type: the
/// entries of the object that neither the fields read under a key of their own nor the flattened
/// types declared before it took.
struct Handed<'walk> {
    /// What is handed over now: the object, a copy the walk bound of what is left of it, or what
    /// an earlier flattened type left of either.
    held: Ident,
    object: &'walk Ident,
    /// `held` is a copy the walk owns, where every other is a reference.
    owned: bool,
}

impl<'walk> Handed<'walk> {
    /// What is handed over, as the argument of a call.
    fn argument(&self) -> TokenStream {
        let held = &self.held;
        if self.owned {
            quote! { &#held }
        } else {
            quote! { #held }
        }
    }

    /// What `object` hands over where the keys of `own` are taken out of it first: the copy
    /// bound as `rest` and what binds it, and the object itself where `own` is empty.
    fn leaving(source: Source, object: &'walk Ident, own: &[String]) -> (Self, TokenStream) {
        if own.is_empty() {
            return (Self::whole(object), TokenStream::new());
        }
        let (rest, object_type) = (Ident::new("rest", Span::call_site()), source.object());
        let bound = quote! {
            let #rest: #object_type = #object
                .iter()
                .filter(|(key, _)| !matches!(key.as_str(), #(#own)|*))
                .map(|(key, held)| (key.clone(), held.clone()))
                .collect();
        };
        let handed = Self {
            held: rest,
            object,
            owned: true,
        };
        (handed, bound)
    }

    /// `keys`, which a fields walker returned for what it was handed, as keys of the object: a
    /// key borrowed from anything else does not outlive the walk.
    fn of_the_object(&self, keys: &TokenStream) -> TokenStream {
        let object = self.object;
        if self.held == *object {
            return keys.clone();
        }
        quote! {
            #keys
                .into_iter()
                .filter_map(|key| #object.keys().find(|own| own.as_str() == key).map(::std::string::String::as_str))
        }
    }

    /// `object` itself, handed over as it is.
    fn whole(object: &'walk Ident) -> Self {
        Self {
            held: object.clone(),
            object,
            owned: false,
        }
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
            quote! { ::core::result::Result::Ok(#key.to_owned()) }
        } else {
            quote! { ::core::result::Result::Ok(stored.to_owned()) }
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
                    None,
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
                    Some(&variant.ident.to_string()),
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

    /// The function a value is held to a field's bound through.
    fn bound(self) -> Ident {
        format_ident!("{}_bound", self.stem())
    }

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
            Self::Json => quote! { ::core::option::Option::Some(items) = found.as_array() },
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
            Self::Json => quote! { serde_json::Map<::std::string::String, serde_json::Value> },
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
            Self::Json => quote! { ::core::option::Option::Some(object) = found.as_object() },
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

    /// The function answering what remains of an object once serde has read a type flattened
    /// there.
    fn remaining(self) -> Ident {
        format_ident!("{}_remaining", self.stem())
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

    /// The closure writing nothing back for a value read whole: a flattened value is held as the
    /// entries serde writes for it, whatever form it has under a key. A value a hook read is
    /// `pinned` to the field's type, as [`Self::write_back`] pins it.
    fn unwritten(self, pinned: Option<&Type>) -> TokenStream {
        let read = pinned.map_or_else(|| quote! { _ }, |ty| quote! { _: &#ty });
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { |#read, _| ::core::option::Option::None },
            Self::Json => quote! { |#read| ::core::option::Option::None },
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
        pinned.map_or_else(
            || quote! { |_| ::core::option::Option::None },
            |ty| quote! { |_: &#ty| ::core::option::Option::None },
        )
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

/// The validator tixschema published for a field's own bound, which serde's read does not run.
struct FieldValidator {
    /// The field's name, as the validator writes it in front of each violation.
    named: String,
    validator: Ident,
}

/// One field of the struct as its walker reads it.
struct WalkedField<'item> {
    /// serde reads the field when its key is missing.
    absence_is_read: bool,
    aliases: Vec<String>,
    key: String,
    ty: &'item Type,
    validator: Option<FieldValidator>,
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

/// What a type's item writes, as the walker's methods are written from it.
struct Written<'reach> {
    /// Every name the item writes.
    names: Vec<String>,
    /// What the item's fields reach through the aliases they are typed with.
    reach: &'reach Reach,
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
        validator: Option<&FieldValidator>,
    ) -> Vec<Arm> {
        let source = self.source;
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
                    &under(
                        segments,
                        &quote! { ::core::result::Result::Ok(#key.clone()) },
                    ),
                    depth,
                    None,
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
                    &under(segments, &quote! { ::core::result::Result::Err(#index) }),
                    depth,
                    validator,
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
            Step::Leaf(whole) => vec![self.leaf_arm(whole, walk.ty, held, &path, validator)],
            Step::Model => vec![self.model_arm(walk.ty, held, segments, validator)],
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
                arms.extend(self.arms(inner, held, segments, depth, walk.ty, validator));
                arms
            }
        }
    }

    /// `call`, a walker method called on `ty`. A struct or an enum `#[model_schema]` was written
    /// on above is called as it stands, so one with no flag fails the build. Any other type is
    /// asked with `ReadWhole` in scope, which answers for a type with no walker of its own; the
    /// call beside it keeps the trait in use where the type has one.
    fn asked(&self, ty: &Type, call: &TokenStream) -> TokenStream {
        if walks_itself(ty, 0) {
            return call.clone();
        }
        let module = self.module;
        quote! {
            {
                use #module::ReadWhole as _;
                <()>::decode_with_read_whole();
                #call
            }
        }
    }

    /// `listed` followed by what holds the value under `held`, read through `read` as `ty`, to the
    /// bound `validator` checks: one `Invalid` per violation, at the value. `listed` alone where
    /// the field has no validator, or its bound reaches nothing of `ty`.
    fn bounded(
        &self,
        listed: &TokenStream,
        validator: Option<&FieldValidator>,
        ty: &Type,
        held: &Ident,
        read: &TokenStream,
        path: &TokenStream,
    ) -> TokenStream {
        let (module, bound) = (self.module, self.source.bound());
        let Some(check) = validator.and_then(|field| {
            let checked_by = &field.validator;
            recovering_bound_check(&quote! { #module::#checked_by }, &field.named, ty)
        }) else {
            return listed.clone();
        };
        let expected = self.expected(ty);
        quote! {{
            #listed;
            out.extend(#module::#bound(#held, #read, #check, #path, #expected, issue));
        }}
    }

    /// `walked` beside what a type with no key of its own answers: no object names it, and its
    /// fields walker lists nothing and returns no key. A type that holds another calls both of it
    /// whatever its shape, so every shape has them.
    fn claiming_no_key(&self, walked: &TokenStream) -> TokenStream {
        let named = self.never_named();
        let keyed = self.fields_method(false, false, &quote! { ::std::vec::Vec::new() });
        quote! {
            #walked
            #named
            #keyed
        }
    }

    /// What walks each entry of `object`, held at `segments`, as an entry of a map of `values`:
    /// every one, or each one whose key `unclaimed` holds of.
    fn entries_walk(
        &self,
        values: &Walk<'_>,
        object: &Ident,
        segments: &[TokenStream],
        unclaimed: Option<&TokenStream>,
    ) -> TokenStream {
        let item = Ident::new("item", Span::call_site());
        let each = self.statement(
            values,
            &item,
            &under(
                segments,
                &quote! { ::core::result::Result::Ok(key.clone()) },
            ),
            0,
            None,
        );
        unclaimed.map_or_else(
            || quote! { for (key, #item) in #object { #each } },
            |test| quote! { for (key, #item) in #object { if #test { #each } } },
        )
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
        let arms = self.arms(
            walk,
            &held,
            &under(segments, &lookup.segment()),
            0,
            walk.ty,
            field.validator.as_ref(),
        );
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
                #path: &[::core::result::Result<::std::string::String, usize>],
                #issue: #module::IssueFromParts<#value, #built>,
                #out: &mut ::std::vec::Vec<#built>,
            ) -> ::std::vec::Vec<&'a str> {
                #body
            }
        }
    }

    /// What walks a flattened flagged type in what it is `handed`, held at `segments`: its own
    /// fields walker, whose keys are kept in `declared` where `collects`.
    fn flattened_model(
        &self,
        model: &Type,
        handed: &Handed<'_>,
        segments: &[TokenStream],
        collects: bool,
    ) -> TokenStream {
        let walked = self.asked(
            model,
            &flattened_walker_call(
                self.source,
                model,
                &handed.argument(),
                segments,
                &quote! { out },
            ),
        );
        if collects {
            let keys = handed.of_the_object(&walked);
            quote! { declared.extend(#keys); }
        } else {
            quote! { #walked; }
        }
    }

    /// What walks a flattened `Option` of a flagged type in what it is `handed`, held at
    /// `segments`, where the type answers that it is named there. serde reads it as absent where
    /// it does not read the type, so the object then holds what the field would not write.
    fn flattened_optional(
        &self,
        model: &Type,
        written: &Type,
        handed: &Handed<'_>,
        segments: &[TokenStream],
        collects: bool,
    ) -> TokenStream {
        let source = self.source;
        let argument = handed.argument();
        let walked = self.asked(
            model,
            &flattened_walker_call(source, model, &argument, segments, &quote! { &mut nested }),
        );
        let named = source.method("named");
        let is_named = self.asked(model, &quote! { <#model>::#named(#argument) });
        let (reader, whole) = (
            source.object_reader(&handed.held),
            source.object_value(handed.object),
        );
        let (here, expected) = (path_expression(segments), self.expected(written));
        let (walk, kept) = if collects {
            let keys = handed.of_the_object(&quote! { keys });
            (
                quote! { let keys = #walked; },
                quote! { declared.extend(#keys); },
            )
        } else {
            (quote! { #walked; }, TokenStream::new())
        };
        quote! {
            if #is_named {
                let mut nested = ::std::vec::Vec::new();
                #walk
                match <#model as serde::Deserialize>::deserialize(#reader) {
                    ::core::result::Result::Ok(_) => out.append(&mut nested),
                    ::core::result::Result::Err(_) => out.push(issue("Mistyped", #here, #expected, ::core::option::Option::Some(#whole), ::core::option::Option::None, ::std::vec::Vec::new())),
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
            Flattened::Entries(values) => self.entries_walk(values, object, segments, unclaimed),
            Flattened::Whole(Walk {
                step: Step::Leaf(whole),
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
                let (read, unwritten) =
                    (&whole.read, source.unwritten(whole.hooked.then_some(*ty)));
                let absent = self.read_as_absent(whole.hooked || whole.parameterized, read, ty);
                quote! {
                    out.extend(
                        #module::#leaf(&#rest, #read, #unwritten, #here, #expected, issue) #absent
                    );
                }
            }
            Flattened::Model(_)
            | Flattened::Optional(_, _)
            | Flattened::Unwalked(_)
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

    /// The fields walker of a struct serde writes as the value it holds, and what answers whether
    /// an object names it: each as the type that flattens that value itself walks and asks it.
    /// `None` where no key is the value's: serde refuses to flatten it, or reads it as absent.
    fn held_keyed(&self, walk: &Walk<'_>) -> Option<TokenStream> {
        let source = self.source;
        let object = Ident::new("object", Span::call_site());
        let every_key = Claimed::Every.returned(&object);
        let any_key = quote! { !object.is_empty() };
        let (asked, lists, body) = match &walk.step {
            Step::Entries(values) => {
                let walked = self.entries_walk(values, &object, &[], None);
                (any_key, true, quote! { #walked #every_key })
            }
            // A hook reads whatever it asks for, and a value of a parameter's type and a JSON value
            // are whatever fills them, an object among it. An `Option` of one is left to the arm
            // below: flattened, serde reads it as absent where its reader refuses.
            Step::Leaf(whole)
                if (whole.hooked || whole.parameterized || holds_any_value(walk.ty))
                    && !get_field_def("", walk.ty, "").is_optional() =>
            {
                let (whole_object, read) = (
                    source.object_value(&object),
                    self.read_whole_flattened(
                        whole.hooked || whole.parameterized,
                        &self.expected(walk.ty),
                    ),
                );
                let walked = quote! {
                    let found = &#whole_object;
                    #read;
                };
                (any_key, true, quote! { #walked #every_key })
            }
            Step::Model => {
                let (ty, named, fields) =
                    (walk.ty, source.method("named"), source.method("fields"));
                (
                    self.asked(ty, &quote! { <#ty>::#named(object) }),
                    true,
                    self.asked(ty, &quote! { <#ty>::#fields(object, path, issue, out) }),
                )
            }
            Step::Present(present) if matches!(present.step, Step::Model) => {
                let (model, named) = (present.ty, source.method("named"));
                let walked =
                    self.flattened_optional(model, walk.ty, &Handed::whole(&object), &[], true);
                (
                    self.asked(model, &quote! { <#model>::#named(object) }),
                    true,
                    quote! {
                        let mut declared = ::std::vec::Vec::new();
                        #walked
                        declared
                    },
                )
            }
            // No walk reaches any other value serde reads from the entries: every key counts as
            // its own, and serde's verdict is the read's.
            Step::Leaf(_) | Step::Present(_) if reads_entries(walk) => (any_key, false, every_key),
            Step::Items(_) | Step::Leaf(_) | Step::Positions(_) | Step::Present(_) => return None,
        };
        let (named, keyed) = (
            self.named_method(true, &asked),
            self.fields_method(true, lists, &body),
        );
        Some(quote! {
            #named
            #keyed
        })
    }

    /// The walker of a struct serde writes as the value one field of it holds: a single-slot tuple
    /// struct, and a `#[serde(transparent)]` struct of either kind.
    fn held_methods(&self, walk: &Walk<'_>) -> TokenStream {
        let found = Ident::new("found", Span::call_site());
        let walked = match &walk.step {
            Step::Entries(_) | Step::Items(_) | Step::Positions(_) | Step::Present(_) => self
                .issues_method(&Self::listed(
                    &found,
                    &self.arms(walk, &found, &[], 0, walk.ty, None),
                )),
            // The type's own reader runs whatever hook its slot carries.
            Step::Leaf(_) => {
                let whole = self.read_whole(&self.expected(walk.ty));
                self.issues_method(&quote! { #whole; })
            }
            Step::Model => {
                let (ty, issues) = (walk.ty, self.source.method("issues"));
                let walked = self.asked(ty, &quote! { <#ty>::#issues(found, path, issue, out) });
                self.issues_method(&quote! { #walked; })
            }
        };
        self.held_keyed(walk).map_or_else(
            || self.claiming_no_key(&walked),
            |keyed| {
                quote! {
                    #walked
                    #keyed
                }
            },
        )
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
            quote! { let mut declared = ::std::vec::Vec::new(); }
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
        let (mut handed, copied) = if declaring {
            Handed::leaving(self.source, object, &own)
        } else {
            (Handed::whole(object), TokenStream::new())
        };
        // serde reads the flattened fields in the order declared, each from what the ones before
        // it left: the readers of those since the last one walked are still to be asked.
        let mut earlier: Vec<(TokenStream, &Type)> = Vec::new();
        let mut declared: Vec<TokenStream> = Vec::new();
        for field in flattened {
            if field.declares_its_keys() {
                for (read, ty) in take(&mut earlier) {
                    declared.push(self.remaining_after(&mut handed, &read, ty));
                }
            }
            declared.push(match field {
                Flattened::Model(model) => self.flattened_model(model, &handed, segments, collects),
                Flattened::Optional(model, written) => {
                    self.flattened_optional(model, written, &handed, segments, collects)
                }
                Flattened::Entries(_) | Flattened::Unwalked(_) | Flattened::Whole(_) => {
                    TokenStream::new()
                }
            });
            earlier.extend(field.taker());
        }
        // The keys an earlier flattened type declares are none of the reader's already. A field
        // no walk reaches declares none, so what serde takes for it is taken out here.
        let rest = reader.map(|field| {
            let mut open = Handed::whole(object);
            let taken: Vec<TokenStream> = flattened
                .iter()
                .take_while(|earlier_field| !earlier_field.reads_the_rest())
                .filter(|earlier_field| matches!(earlier_field, Flattened::Unwalked(_)))
                .filter_map(Flattened::taker)
                .map(|(read, ty)| self.remaining_after(&mut open, &read, ty))
                .collect();
            let walked = self.flattened_rest(field, &open.held, segments, unclaimed.as_ref());
            quote! {
                #(#taken)*
                #walked
            }
        });
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
                #copied
                #(#declared)*
                #rest
            },
        }
    }

    /// The arm of a plain value of type `ty` held under `held`: read whole, and held to the bound
    /// `validator` checks.
    fn leaf_arm(
        &self,
        whole: &Whole,
        ty: &Type,
        held: &Ident,
        path: &TokenStream,
        validator: Option<&FieldValidator>,
    ) -> Arm {
        let (module, source) = (self.module, self.source);
        let (expected, leaf) = (self.expected(ty), source.leaf());
        let (read, pinned) = (&whole.read, whole.hooked.then_some(ty));
        let written = source.written(whole.parameterized, pinned, whole.write.as_ref());
        let listed = quote! {
            out.extend(#module::#leaf(#held, #read, #written, #path, #expected, issue))
        };
        Arm {
            body: self.bounded(&listed, validator, ty, held, read, path),
            nothing: false,
            pattern: quote! { #held },
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
                quote! { ::core::option::Option::Some((stored, #pattern)) }
            } else {
                quote! { ::core::option::Option::Some(#pattern) }
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
            quote! { ::core::option::Option::None | ::core::option::Option::Some(#null) => {} }
        } else if lookup.absence_is_read {
            quote! { ::core::option::Option::None => {} }
        } else {
            let here = path_expression(&under(
                segments,
                &quote! { ::core::result::Result::Ok(#key.to_owned()) },
            ));
            quote! { ::core::option::Option::None => out.push(issue("Missing", #here, #expected, ::core::option::Option::None, ::core::option::Option::None, ::std::vec::Vec::new())), }
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

    /// The arm of a value held under `held` whose type `ty` has a walker of its own, held to the
    /// bound `validator` checks as well.
    fn model_arm(
        &self,
        ty: &Type,
        held: &Ident,
        segments: &[TokenStream],
        validator: Option<&FieldValidator>,
    ) -> Arm {
        let asked = self.asked(ty, &model_walker_call(self.source, ty, held, segments));
        let path = path_expression(segments);
        Arm {
            body: self.bounded(&asked, validator, ty, held, &own_reader(ty), &path),
            nothing: false,
            pattern: quote! { #held },
        }
    }

    /// `decode_with_{source}_named` answering `body`. An `object` that `body` does not read is
    /// bound as `_`.
    fn named_method(&self, reads_object: bool, body: &TokenStream) -> TokenStream {
        let (named, object_type) = (self.source.method("named"), self.source.object());
        let object = if reads_object {
            quote! { object }
        } else {
            quote! { _ }
        };
        quote! {
            /// Whether `object` holds what names a value of this type.
            pub fn #named(#object: &#object_type) -> bool {
                #body
            }
        }
    }

    /// `decode_with_{source}_named` of a type no object names: one with no key of its own.
    fn never_named(&self) -> TokenStream {
        self.named_method(false, &quote! { false })
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
                    out.push(issue("Unknown", [path, &[::core::result::Result::Ok(key.clone())]].concat(), &[], ::core::option::Option::Some(held.clone()), ::core::option::Option::None, ::std::vec::Vec::new()));
                }
            }
        })
    }

    /// The walker of a struct serde writes as an object of its fields.
    fn object_methods(&self, keyed: &Keyed<'_>) -> TokenStream {
        let object = Ident::new("object", Span::call_site());
        let keyed_walk = self.keyed(keyed, None, &object, &[], true);
        let walked = self.object_issues_method();
        let named = self.object_named(keyed).map_or_else(
            || self.never_named(),
            |answer| self.named_method(true, &answer),
        );
        // A type with no field to walk lists no issue of its own, so the `Vec` it is handed goes
        // unbound.
        let fields = self.fields_method(true, keyed_walk.lists, &keyed_walk.returning(&object));
        quote! {
            #walked

            #named

            #fields
        }
    }

    /// Whether `object` names a struct of these fields: it holds a key one of them is read under,
    /// or names a flagged type the struct flattens. `None` where no key can name it.
    fn object_named(&self, keyed: &Keyed<'_>) -> Option<TokenStream> {
        // A flattened field that takes the rest owns every key, so any key names the struct.
        if keyed
            .flattened
            .iter()
            .any(|field| !field.declares_its_keys())
        {
            return Some(quote! { !object.is_empty() });
        }
        let named = self.source.method("named");
        let keys = &keyed.declared;
        let own = (!keys.is_empty())
            .then(|| quote! { object.keys().any(|key| matches!(key.as_str(), #(#keys)|*)) });
        let flattened = keyed.flattened.iter().filter_map(|field| {
            if let Flattened::Model(model) | Flattened::Optional(model, _) = field {
                Some(self.asked(model, &quote! { <#model>::#named(object) }))
            } else {
                None
            }
        });
        own.into_iter()
            .chain(flattened)
            .reduce(|answer, next| quote! { #answer || #next })
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
        let here = under(segments, &quote! { ::core::result::Result::Err(#index) });
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
            None,
        );
        if slot.absence_is_read
            && let [only] = arms.as_slice()
        {
            let (pattern, body) = (&only.pattern, &only.body);
            return quote! {
                if let ::core::option::Option::Some(#pattern) = #lookup {
                    #body;
                }
            };
        }
        // An absent position and a `null` are the same `None` to serde, so one arm answers both.
        let merged = slot.absence_is_read && arms.first().is_some_and(|arm| arm.nothing);
        let listed = arms.iter().skip(usize::from(merged)).map(|arm| {
            let (pattern, body) = (&arm.pattern, &arm.body);
            quote! { ::core::option::Option::Some(#pattern) => #body, }
        });
        if merged {
            let null = self.source.null();
            return quote! { match #lookup { ::core::option::Option::None | ::core::option::Option::Some(#null) => {} #(#listed)* } };
        }
        let absent = if slot.absence_is_read {
            quote! { ::core::option::Option::None => {} }
        } else {
            let path = path_expression(&here);
            let expected = self.expected(slot.ty);
            quote! { ::core::option::Option::None => out.push(issue("Missing", #path, #expected, ::core::option::Option::None, ::core::option::Option::None, ::std::vec::Vec::new())), }
        };
        quote! { match #lookup { #(#listed)* #absent } }
    }

    /// The walker of a tuple struct, which serde writes as an array of its slots.
    fn positional_methods(&self, slots: &[Slot<'_>]) -> TokenStream {
        let held_as = self.held_as(&self.source.items_of_found());
        let positions = self.positions(slots, &Ident::new("items", Span::call_site()), &[], 0);
        self.claiming_no_key(&self.issues_method(&quote! {
            #held_as
            #positions
        }))
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
        let here = path_expression(&under(
            segments,
            &quote! { ::core::result::Result::Err(#index) },
        ));
        let undeclared = if slots.is_empty() {
            quote! { #items.iter().enumerate() }
        } else {
            let count = Literal::usize_unsuffixed(slots.len());
            quote! { #items.iter().enumerate().skip(#count) }
        };
        quote! {
            #(#declared)*
            for (#index, #held) in #undeclared {
                out.push(issue("Unknown", #here, &[], ::core::option::Option::Some(#held.clone()), ::core::option::Option::None, ::std::vec::Vec::new()));
            }
        }
    }

    /// What keeps the issue of a value read whole as `ty` only where `read` reads no `Option`:
    /// flattening, serde reads an `Option` as absent wherever what it holds is refused. Nothing
    /// unless the `Option` would be `unseen` here, behind a hook or in what fills a parameter.
    fn read_as_absent(&self, unseen: bool, read: &TokenStream, ty: &Type) -> TokenStream {
        if !unseen {
            return TokenStream::new();
        }
        let module = self.module;
        quote! {
            .filter(|_| !#module::reads_an_option::<#ty, _>(#read, &#module::Asked::default()))
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

    /// [`Self::read_whole`] where `found` is the entries serde reads the type from flattened:
    /// nothing is written back, and an `Option` that is `unseen` here is absent where refused.
    fn read_whole_flattened(&self, unseen: bool, expected: &TokenStream) -> TokenStream {
        let (module, leaf) = (self.module, self.source.leaf());
        let own: Type = parse_quote! { Self };
        let read = own_reader(&own);
        let (unwritten, absent) = (
            self.source.unwritten(None),
            self.read_as_absent(unseen, &read, &own),
        );
        quote! {
            out.extend(#module::#leaf(found, #read, #unwritten, path.to_vec(), #expected, issue) #absent)
        }
    }

    /// What binds, as what is `handed` over from here on, what remains of it once serde has read
    /// a `ty` flattened there through `read`: the same object where serde takes nothing. `ty`
    /// pins what a hook generic over what it reads is asked for.
    fn remaining_after(
        &self,
        handed: &mut Handed<'_>,
        read: &TokenStream,
        ty: &Type,
    ) -> TokenStream {
        let (module, remaining_after) = (self.module, self.source.remaining());
        let argument = handed.argument();
        let remaining = Ident::new("remaining", Span::call_site());
        let bound = quote! {
            let taken =
                #module::#remaining_after::<#ty, _>(#read, &#module::Asked::default(), #argument);
            let #remaining = taken.as_ref().unwrap_or(#argument);
        };
        handed.held = remaining;
        handed.owned = false;
        bound
    }

    /// What every method listing the issues of one value takes, after its name.
    fn signature(&self) -> TokenStream {
        let (module, built) = (self.module, self.issue_parameter);
        let value = self.source.value();
        quote! {
            <#built>(
                found: &#value,
                path: &[::core::result::Result<::std::string::String, usize>],
                issue: #module::IssueFromParts<#value, #built>,
                out: &mut ::std::vec::Vec<#built>,
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
        validator: Option<&FieldValidator>,
    ) -> TokenStream {
        Self::listed(
            held,
            &self.arms(
                walk,
                held,
                segments,
                depth.saturating_add(1),
                walk.ty,
                validator,
            ),
        )
    }

    /// The walker of a unit struct: an object in which no key is the type's own.
    fn unit_methods(&self) -> TokenStream {
        let held_as = self.held_as(&self.source.object_of_found());
        let undeclared = undeclared_keys(&Ident::new("object", Span::call_site()), &[], &[]);
        self.claiming_no_key(&self.issues_method(&quote! {
            #held_as
            #undeclared
        }))
    }

    /// The closure writing back the type's own whole value, read with its own reader.
    fn written_whole(&self) -> TokenStream {
        self.source.written(!self.parameters.is_empty(), None, None)
    }
}

/// `ReadWhole`: what a field's type answers where it has no walker of its own.
///
/// A walker method called as `<Type>::decode_with_value_issues(..)` is the type's own where it has
/// one, and this trait's where it has none and the trait is in scope. So a field typed with a
/// type the walk cannot see into, such as an alias of a foreign map, builds, and is read whole.
fn read_whole_items() -> TokenStream {
    let object = Ident::new("object", Span::call_site());
    let own: Type = parse_quote! { Self };
    let methods = Source::GENERATED.iter().map(|&source| {
        let (issues, named, fields, leaf) = (
            source.method("issues"),
            source.method("named"),
            source.method("fields"),
            source.leaf(),
        );
        let (value, object_type) = (source.value(), source.object());
        let (reader, whole, unwritten) = (
            source.object_reader(&object),
            source.object_value(&object),
            source.unwritten(Some(&own)),
        );
        quote! {
            /// Lists serde's verdict on `found`, read whole as this type at `path`.
            fn #issues<I>(
                found: &#value,
                path: &[::core::result::Result<::std::string::String, usize>],
                issue: IssueFromParts<#value, I>,
                out: &mut ::std::vec::Vec<I>,
            ) {
                out.extend(#leaf(found, <Self as serde::Deserialize>::deserialize, #unwritten, path.to_vec(), &[("Unknown", &[], 0)], issue));
            }

            /// Whether `object` holds a key and serde reads this type from its entries.
            fn #named(object: &#object_type) -> bool {
                !object.is_empty() && <Self as serde::Deserialize>::deserialize(#reader).is_ok()
            }

            /// Lists serde's verdict on the entries of `object`, read whole as this type, and
            /// returns every key: with no walker to say otherwise, all are the type's own.
            fn #fields<'a, I>(
                object: &'a #object_type,
                path: &[::core::result::Result<::std::string::String, usize>],
                issue: IssueFromParts<#value, I>,
                out: &mut ::std::vec::Vec<I>,
            ) -> ::std::vec::Vec<&'a str> {
                Self::#issues(&#whole, path, issue, out);
                object.keys().map(::std::string::String::as_str).collect()
            }
        }
    });
    let every = Source::GENERATED.iter().map(|&source| {
        let (issues, named, fields) = (
            source.method("issues"),
            source.method("named"),
            source.method("fields"),
        );
        quote! { Self::#issues::<()>, Self::#named, Self::#fields::<()> }
    });
    quote! {
        /// What a field's type answers where it has no walker of its own: it is read whole, with
        /// its own `Deserialize`.
        pub trait ReadWhole: serde::de::DeserializeOwned {
            #(#methods)*

            /// Called beside every walker a field's type is asked for, which keeps this trait in
            /// use where the type answers with its own.
            fn decode_with_read_whole() {
                fn kept<U>(_: &U) {}
                kept(&(#(#every),*));
            }
        }

        impl<T: serde::de::DeserializeOwned> ReadWhole for T {}
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
    let mut reach = Reach::new(&module_name);
    let mut walked = item_struct.clone();
    reach.fields(&mut walked.fields);
    let shape = Shape::of(&walked, &module_name, &parameters);
    added_to(
        &item_struct.ident,
        &item_struct.generics,
        &module_name,
        &parameters,
        &Written {
            names: written_names(item_struct.to_token_stream()),
            reach: &reach,
        },
        |walker| walker.methods(&shape),
    )
}

/// What the flag adds to the type `name` declares under `generics`: the callback's types, and per
/// source the entry point beside what `methods` writes for that source's walker. `written` holds
/// every name the type's item writes, none of which a method's own type parameter takes, and what
/// its fields reach through the aliases they are typed with.
fn added_to<M>(
    name: &Ident,
    generics: &Generics,
    module_name: &str,
    parameters: &[String],
    written: &Written<'_>,
    methods: M,
) -> RecoveringDecode
where
    M: Fn(&Walker<'_>) -> TokenStream,
{
    let own_name = name.to_string();
    let module = Ident::new(module_name, Span::call_site());
    let decider = unclaimed_parameter("F", &written.names);
    let issue_parameter = unclaimed_parameter("I", &written.names);
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
    let mut type_impl = type_impls(name, generics, !parameters.is_empty(), &of_sources);
    let mut items = module_items();
    if written_names(type_impl.clone())
        .iter()
        .any(|written_name| written_name == "ReadWhole")
    {
        items.extend(read_whole_items());
    }
    items.extend(written.reach.module_items());
    type_impl.extend(written.reach.guards());
    RecoveringDecode {
        schema_module: placed_in_schema_module(&module, &items),
        type_impl,
    }
}

/// A deserializer that reads nothing and notes what a reader asks of it, and what it notes.
fn asked_items() -> TokenStream {
    quote! {
        /// What a read asks of the entries a flattened field is read from, as a [`TakenProbe`]
        /// notes it.
        #[derive(Default)]
        #[non_exhaustive]
        pub struct Asked {
            /// It asks for an `Option`, which serde reads as absent where what it holds is refused.
            optional: ::core::cell::Cell<bool>,
            taken: ::core::cell::Cell<Taken>,
        }

        /// What a read asks for says of the entries serde takes out for it.
        #[derive(Clone, Copy, Default)]
        #[non_exhaustive]
        enum Taken {
            /// The entries under these keys: a struct with named fields.
            Fields(&'static [&'static str]),
            /// None: a map, and whatever else reads the entries and leaves them.
            #[default]
            Nothing,
            /// The first entry one of these names keys: an enum written under its variant's name.
            Variant(&'static [&'static str]),
        }

        /// A deserializer that reads nothing: it notes what a read asks of the entries a flattened
        /// field is read from, and refuses it.
        #[non_exhaustive]
        pub struct TakenProbe<'asked>(&'asked Asked);

        impl<'de> serde::Deserializer<'de> for TakenProbe<'_> {
            type Error = serde::de::value::Error;

            fn deserialize_any<V: serde::de::Visitor<'de>>(
                self,
                _visitor: V,
            ) -> ::core::result::Result<V::Value, Self::Error> {
                ::core::result::Result::Err(serde::de::Error::custom("nothing is read"))
            }

            fn deserialize_struct<V: serde::de::Visitor<'de>>(
                self,
                _name: &'static str,
                fields: &'static [&'static str],
                _visitor: V,
            ) -> ::core::result::Result<V::Value, Self::Error> {
                self.0.taken.set(Taken::Fields(fields));
                ::core::result::Result::Err(serde::de::Error::custom("nothing is read"))
            }

            fn deserialize_enum<V: serde::de::Visitor<'de>>(
                self,
                _name: &'static str,
                variants: &'static [&'static str],
                _visitor: V,
            ) -> ::core::result::Result<V::Value, Self::Error> {
                self.0.taken.set(Taken::Variant(variants));
                ::core::result::Result::Err(serde::de::Error::custom("nothing is read"))
            }

            fn deserialize_option<V: serde::de::Visitor<'de>>(
                self,
                visitor: V,
            ) -> ::core::result::Result<V::Value, Self::Error> {
                self.0.optional.set(true);
                visitor.visit_some(self)
            }

            fn deserialize_newtype_struct<V: serde::de::Visitor<'de>>(
                self,
                _name: &'static str,
                visitor: V,
            ) -> ::core::result::Result<V::Value, Self::Error> {
                visitor.visit_newtype_struct(self)
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
                unit unit_struct seq tuple tuple_struct map identifier ignored_any
            }
        }
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

/// `value_bound` and `bson_bound`: what holds a value serde read to the bound its field declares.
fn bound_items() -> TokenStream {
    let bson_bound = bson_bound_items();
    quote! {
        /// What a field's bound refuses of one JSON value `read` reads: an issue per violation
        /// `check` answers, and none where the value does not read.
        pub fn value_bound<'a, T, I, R, C>(
            held: &'a serde_json::Value,
            read: R,
            check: C,
            path: ::std::vec::Vec<::core::result::Result<::std::string::String, usize>>,
            expected: &'static [ExpectedToken],
            issue: IssueFromParts<serde_json::Value, I>,
        ) -> ::std::vec::Vec<I>
        where
            R: ::core::ops::FnOnce(&'a serde_json::Value) -> ::core::result::Result<T, serde_json::Error>,
            C: ::core::ops::FnOnce(&T) -> ::std::vec::Vec<::std::string::String>,
        {
            let ::core::result::Result::Ok(read) = read(held) else {
                return ::std::vec::Vec::new();
            };
            check(&read)
                .into_iter()
                .map(|reason| issue("Invalid", path.clone(), expected, ::core::option::Option::Some(held.clone()), ::core::option::Option::Some(reason), ::std::vec::Vec::new()))
                .collect()
        }

        #bson_bound
    }
}

/// `bson_bound`.
#[cfg(feature = "bson")]
fn bson_bound_items() -> TokenStream {
    quote! {
        /// What a field's bound refuses of one BSON value `read` reads: an issue per violation
        /// `check` answers, and none where the value does not read.
        pub fn bson_bound<T, I, E, R, C>(
            held: &bson::Bson,
            read: R,
            check: C,
            path: ::std::vec::Vec<::core::result::Result<::std::string::String, usize>>,
            expected: &'static [ExpectedToken],
            issue: IssueFromParts<bson::Bson, I>,
        ) -> ::std::vec::Vec<I>
        where
            R: ::core::ops::FnOnce(bson::Deserializer) -> ::core::result::Result<T, E>,
            C: ::core::ops::FnOnce(&T) -> ::std::vec::Vec<::std::string::String>,
        {
            let ::core::result::Result::Ok(read) = read(bson::Deserializer::new(held.clone())) else {
                return ::std::vec::Vec::new();
            };
            check(&read)
                .into_iter()
                .map(|reason| issue("Invalid", path.clone(), expected, ::core::option::Option::Some(held.clone()), ::core::option::Option::Some(reason), ::std::vec::Vec::new()))
                .collect()
        }
    }
}

/// A build without `bson` reads no BSON value.
#[cfg(not(feature = "bson"))]
fn bson_bound_items() -> TokenStream {
    TokenStream::new()
}

/// `from_bson_with` and the report it runs. Every value is read through `bson::Deserializer::new`,
/// which both major versions of the `bson` library have, and which takes what it reads by value:
/// the document is held as the `bson::Bson` the walker borrows, and copied once per read by serde.
#[cfg(feature = "bson")]
fn bson_entry_methods(module: &Ident, decider: &Ident) -> TokenStream {
    quote! {
        /// Reads `document` as this type, handing every issue found in it to `decide`, once.
        pub fn from_bson_with<#decider>(
            document: bson::Document,
            decide: #decider,
        ) -> ::core::result::Result<Self, #module::Unrecovered<bson::Bson>>
        where
            #decider: ::core::ops::FnOnce(&mut bson::Document, &[#module::Issue<bson::Bson>]) -> #module::Verdict,
        {
            let mut whole = bson::Bson::Document(document);
            let found = match <Self as serde::Deserialize>::deserialize(bson::Deserializer::new(whole.clone())) {
                ::core::result::Result::Ok(decoded) => {
                    let found = Self::decode_with_bson_report(&whole, ::core::option::Option::None);
                    if found.is_empty() {
                        return ::core::result::Result::Ok(decoded);
                    }
                    found
                }
                ::core::result::Result::Err(refused) => Self::decode_with_bson_report(&whole, ::core::option::Option::Some(refused.to_string())),
            };
            // `whole` is the document it was built from, so the other arm is never taken.
            let bson::Bson::Document(object) = &mut whole else {
                return ::core::result::Result::Err(#module::Unrecovered { issues: found });
            };
            match decide(object, &found) {
                #module::Verdict::Reject => ::core::result::Result::Err(#module::Unrecovered { issues: found }),
                #module::Verdict::Fixed => {
                    let read = <Self as serde::Deserialize>::deserialize(bson::Deserializer::new(whole.clone()));
                    let again = Self::decode_with_bson_report(&whole, read.as_ref().err().map(::std::string::ToString::to_string));
                    match read {
                        ::core::result::Result::Ok(decoded) if again.is_empty() => ::core::result::Result::Ok(decoded),
                        _ => ::core::result::Result::Err(#module::Unrecovered { issues: again }),
                    }
                }
            }
        }

        fn decode_with_bson_report(whole: &bson::Bson, refused: ::core::option::Option<::std::string::String>) -> ::std::vec::Vec<#module::Issue<bson::Bson>> {
            let mut out = ::std::vec::Vec::new();
            Self::decode_with_bson_issues(whole, &[], #module::issue_from_parts, &mut out);
            if let ::core::option::Option::Some(reason) = refused
                && out.iter().all(|found| matches!(found, #module::Issue::Unknown { .. } | #module::Issue::Mistyped { .. }))
            {
                out.push(#module::Issue::Undescribed { reason });
            }
            out
        }
    }
}

/// `bson_leaf`, the bracket rule it decides `Mistyped` by, and `bson_remaining`.
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
                || ::std::mem::discriminant(stored) == ::std::mem::discriminant(written)
        }

        /// One BSON value read whole: refused by serde, or read but stored as a type `write`
        /// does not give back.
        pub fn bson_leaf<T, I, E, R, W>(
            held: &bson::Bson,
            read: R,
            write: W,
            path: ::std::vec::Vec<::core::result::Result<::std::string::String, usize>>,
            expected: &'static [ExpectedToken],
            issue: IssueFromParts<bson::Bson, I>,
        ) -> ::core::option::Option<I>
        where
            E: ::core::fmt::Display,
            R: ::core::ops::FnOnce(bson::Deserializer) -> ::core::result::Result<T, E>,
            W: ::core::ops::FnOnce(&T, bson::Serializer) -> ::core::option::Option<bson::Bson>,
        {
            match read(bson::Deserializer::new(held.clone())) {
                ::core::result::Result::Err(refused) => ::core::option::Option::Some(issue("Invalid", path, expected, ::core::option::Option::Some(held.clone()), ::core::option::Option::Some(refused.to_string()), ::std::vec::Vec::new())),
                ::core::result::Result::Ok(read) => match write(&read, bson::Serializer::new()) {
                    ::core::option::Option::Some(written) if !same_bracket(held, &written) => {
                        ::core::option::Option::Some(issue("Mistyped", path, expected, ::core::option::Option::Some(held.clone()), ::core::option::Option::None, ::std::vec::Vec::new()))
                    }
                    _ => ::core::option::Option::None,
                },
            }
        }

        /// What remains of `entries` once serde has read a type flattened there through `read`,
        /// and `None` where it takes none of them.
        pub fn bson_remaining<'asked, T, R>(
            read: R,
            asked: &'asked Asked,
            entries: &bson::Document,
        ) -> ::core::option::Option<bson::Document>
        where
            R: ::core::ops::FnOnce(TakenProbe<'asked>) -> ::core::result::Result<T, serde::de::value::Error>,
        {
            let taken = taken_keys(read, asked, entries.keys());
            if taken.is_empty() {
                return ::core::option::Option::None;
            }
            ::core::option::Option::Some(
                entries
                    .iter()
                    .filter(|(key, _)| !taken.contains(&key.as_str()))
                    .map(|(key, held)| (key.clone(), held.clone()))
                    .collect(),
            )
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
            let ::core::option::Option::Some((Segment::Key(first), rest)) = self.0.split_first() else { return false };
            let ::core::option::Option::Some((last, parents)) = rest.split_last() else {
                root.insert(first.clone(), value);
                return true;
            };
            let ::core::option::Option::Some(mut slot) = root.get_mut(first) else { return false };
            for segment in parents {
                let next = match (segment, slot) {
                    (Segment::Key(key), bson::Bson::Document(object)) => object.get_mut(key),
                    (Segment::Index(index), bson::Bson::Array(items)) => items.get_mut(*index),
                    _ => ::core::option::Option::None,
                };
                let ::core::option::Option::Some(next) = next else { return false };
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
            let ::core::option::Option::Some((Segment::Key(first), rest)) = self.0.split_first() else { return false };
            let ::core::option::Option::Some((last, parents)) = rest.split_last() else {
                return root.remove(first).is_some();
            };
            let ::core::option::Option::Some(mut slot) = root.get_mut(first) else { return false };
            for segment in parents {
                let next = match (segment, slot) {
                    (Segment::Key(key), bson::Bson::Document(object)) => object.get_mut(key),
                    (Segment::Index(index), bson::Bson::Array(items)) => items.get_mut(*index),
                    _ => ::core::option::Option::None,
                };
                let ::core::option::Option::Some(next) = next else { return false };
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
            Array(::std::boxed::Box<Expected>),
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
            Map(::std::boxed::Box<Expected>),
            /// A `#[model_schema]` type, by its Rust name as the field's type writes it.
            Model(&'static str),
            NaiveDate,
            NaiveDateTime,
            NaiveTime,
            NumberLiteral(f64),
            ObjectId,
            Optional(::std::boxed::Box<Expected>),
            String,
            StringLiteral(&'static str),
            Tuple(::std::vec::Vec<Expected>),
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
            Invalid { path: Path, expected: Expected, found: V, reason: ::std::string::String },
            /// The type requires the key, and it is not there.
            Missing { path: Path, expected: Expected },
            /// The key is there, and the type declares no field by that name.
            Unknown { path: Path, found: V },
            /// The value reads, but it is held in a different form than its field writes: another JSON
            /// shape, or another BSON type, which a MongoDB query for the field's own type does not match.
            Mistyped { path: Path, expected: Expected, found: V },
            /// An untagged enum none of whose variants reads the value: each variant's own list.
            NoVariant { path: Path, found: V, variants: ::std::vec::Vec<(&'static str, ::std::vec::Vec<Issue<V>>)> },
            /// Serde refused the value and the walk found nothing to say why.
            Undescribed { reason: ::std::string::String },
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
            pub issues: ::std::vec::Vec<Issue<V>>,
        }

        impl<V: ::core::fmt::Debug> ::core::fmt::Display for Unrecovered<V> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                let place = |path: &Path| {
                    if path.0.is_empty() {
                        ::std::string::String::from("the value itself")
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
                            let tried: ::std::vec::Vec<&str> = variants.iter().map(|(variant, _)| *variant).collect();
                            write!(f, "{}: no variant: found {found:?}, tried {}", place(path), tried.join(", "))?;
                        }
                        Issue::Undescribed { reason } => write!(f, "undescribed: {reason}")?,
                    }
                }
                ::core::result::Result::Ok(())
            }
        }

        impl<V: ::core::fmt::Debug> ::std::error::Error for Unrecovered<V> {}
    }
}

/// `from_value_with` and the report it runs. The report is told what serde said of the value, so
/// each decode reads the value with serde once.
fn entry_methods(module: &Ident, decider: &Ident) -> TokenStream {
    quote! {
        /// Reads `value` as this type, handing every issue found in it to `decide`, once.
        pub fn from_value_with<#decider>(
            mut value: serde_json::Value,
            decide: #decider,
        ) -> ::core::result::Result<Self, #module::Unrecovered<serde_json::Value>>
        where
            #decider: ::core::ops::FnOnce(&mut serde_json::Value, &[#module::Issue<serde_json::Value>]) -> #module::Verdict,
        {
            let found = match <Self as serde::Deserialize>::deserialize(&value) {
                ::core::result::Result::Ok(decoded) => {
                    let found = Self::decode_with_value_report(&value, ::core::option::Option::None);
                    if found.is_empty() {
                        return ::core::result::Result::Ok(decoded);
                    }
                    found
                }
                ::core::result::Result::Err(refused) => Self::decode_with_value_report(&value, ::core::option::Option::Some(refused.to_string())),
            };
            match decide(&mut value, &found) {
                #module::Verdict::Reject => ::core::result::Result::Err(#module::Unrecovered { issues: found }),
                #module::Verdict::Fixed => {
                    let read = <Self as serde::Deserialize>::deserialize(&value);
                    let again = Self::decode_with_value_report(&value, read.as_ref().err().map(::std::string::ToString::to_string));
                    match read {
                        ::core::result::Result::Ok(decoded) if again.is_empty() => ::core::result::Result::Ok(decoded),
                        _ => ::core::result::Result::Err(#module::Unrecovered { issues: again }),
                    }
                }
            }
        }

        fn decode_with_value_report(value: &serde_json::Value, refused: ::core::option::Option<::std::string::String>) -> ::std::vec::Vec<#module::Issue<serde_json::Value>> {
            let mut out = ::std::vec::Vec::new();
            Self::decode_with_value_issues(value, &[], #module::issue_from_parts, &mut out);
            if let ::core::option::Option::Some(reason) = refused
                && out.iter().all(|found| matches!(found, #module::Issue::Unknown { .. } | #module::Issue::Mistyped { .. }))
            {
                out.push(#module::Issue::Undescribed { reason });
            }
            out
        }
    }
}

/// Hands `object`, which is what serde hands a flattened field, to the fields walker of the model
/// type the field is declared as, which lists into `out` and returns the keys that are that
/// type's own.
///
/// A type a flagged type flattens carries the flag too. One serde refuses to flatten, a tuple
/// struct or a single-slot struct over text, a list or a tuple, builds as any other: its fields
/// walker lists nothing and returns no key, and the read carries serde's refusal. This builds:
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
/// standalone as an ordinary test file, and these are the errors it earned, verbatim: one where
/// `Note` asks `Audit` whether an object names it, and one where it hands `Audit` the walk.
///
/// ```text
/// error[E0599]: no associated function or constant named `decode_with_value_named` found for struct `Audit` in the current scope
///   --> tests/zz_probe.rs:10:1
///    |
///  6 | pub struct Audit {
///    | ---------------- associated function or constant `decode_with_value_named` not found for this struct
/// ...
/// 10 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ associated function or constant not found in `Audit`
///    |
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
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
/// error: could not compile `tixschema` (test "zz_probe") due to 2 previous errors
/// ```
///
/// A build with `bson` on earns each a second time, naming `decode_with_bson_named` and
/// `decode_with_bson_fields`.
fn flattened_walker_call(
    source: Source,
    model: &Type,
    object: &TokenStream,
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
            ::std::vec::Vec<::core::result::Result<::std::string::String, usize>>,
            &'static [ExpectedToken],
            ::core::option::Option<V>,
            ::core::option::Option<::std::string::String>,
            ::std::vec::Vec<(&'static str, ::std::vec::Vec<I>)>,
        ) -> I;

        fn expected_from_tokens(tokens: &mut ::std::slice::Iter<'_, ExpectedToken>) -> Expected {
            let ::core::option::Option::Some(&(member, names, count)) = tokens.next() else { return Expected::Unknown };
            let name = names.first().copied().unwrap_or_default();
            let mut under: ::std::vec::Vec<Expected> = (0..count).map(|_| expected_from_tokens(tokens)).collect();
            let first = |under: &mut ::std::vec::Vec<Expected>| ::std::boxed::Box::new(under.pop().unwrap_or(Expected::Unknown));
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
            path: ::std::vec::Vec<::core::result::Result<::std::string::String, usize>>,
            expected: &'static [ExpectedToken],
            found: ::core::option::Option<V>,
            reason: ::core::option::Option<::std::string::String>,
            variants: ::std::vec::Vec<(&'static str, ::std::vec::Vec<Issue<V>>)>,
        ) -> Issue<V> {
            let path = Path(
                path.into_iter()
                    .map(|segment| match segment {
                        ::core::result::Result::Ok(key) => Segment::Key(key),
                        ::core::result::Result::Err(index) => Segment::Index(index),
                    })
                    .collect(),
            );
            let expected = expected_from_tokens(&mut expected.iter());
            match (kind, found, reason) {
                ("Invalid", ::core::option::Option::Some(found), ::core::option::Option::Some(reason)) => Issue::Invalid { path, expected, found, reason },
                ("Missing", ::core::option::Option::None, ::core::option::Option::None) => Issue::Missing { path, expected },
                ("Unknown", ::core::option::Option::Some(found), ::core::option::Option::None) => Issue::Unknown { path, found },
                ("Mistyped", ::core::option::Option::Some(found), ::core::option::Option::None) => Issue::Mistyped { path, expected, found },
                ("NoVariant", ::core::option::Option::Some(found), ::core::option::Option::None) => Issue::NoVariant { path, found, variants },
                (_, _, reason) => Issue::Undescribed { reason: reason.unwrap_or_default() },
            }
        }

        /// One JSON value read whole: refused by serde, or read but held in a different shape
        /// than `write` gives back.
        pub fn value_leaf<'a, T, I, R, W>(
            held: &'a serde_json::Value,
            read: R,
            write: W,
            path: ::std::vec::Vec<::core::result::Result<::std::string::String, usize>>,
            expected: &'static [ExpectedToken],
            issue: IssueFromParts<serde_json::Value, I>,
        ) -> ::core::option::Option<I>
        where
            R: ::core::ops::FnOnce(&'a serde_json::Value) -> ::core::result::Result<T, serde_json::Error>,
            W: ::core::ops::FnOnce(&T) -> ::core::option::Option<serde_json::Value>,
        {
            match read(held) {
                ::core::result::Result::Err(refused) => ::core::option::Option::Some(issue("Invalid", path, expected, ::core::option::Option::Some(held.clone()), ::core::option::Option::Some(refused.to_string()), ::std::vec::Vec::new())),
                ::core::result::Result::Ok(read) => match write(&read) {
                    ::core::option::Option::Some(written) if ::std::mem::discriminant(held) != ::std::mem::discriminant(&written) => {
                        ::core::option::Option::Some(issue("Mistyped", path, expected, ::core::option::Option::Some(held.clone()), ::core::option::Option::None, ::std::vec::Vec::new()))
                    }
                    _ => ::core::option::Option::None,
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
    if hooked_leaf(&slot.ty, &hooks, module_name, parameters).is_none() {
        return walk_of(&slot.ty, parameters);
    }
    Walk {
        step: Step::Leaf(Whole {
            hooked: true,
            parameterized: names_a_parameter(&slot.ty, parameters),
            read: own_reader(&slot.ty),
            write: None,
        }),
        ty: &slot.ty,
    }
}

/// Whether `ty` is written as an id, or an `Option` of one. serde writes an id as an object, and
/// so reads one flattened.
fn holds_an_id(ty: &Type) -> bool {
    let def = get_field_def("", ty, "");
    def.array_depth == 0
        && def
            .expected_members()
            .last()
            .is_some_and(|(member, _, _)| *member == "ObjectId")
}

/// Whether `ty` is written as a type that holds any value, an object among them: a JSON value,
/// or an `Option` of one.
fn holds_any_value(ty: &Type) -> bool {
    let def = get_field_def("", ty, "");
    def.array_depth == 0 && matches!(def.field_type, FieldDefType::Unknown)
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

/// Whether serde neither writes nor reads `field`, so that no record it wrote holds a key of it.
/// One under `skip_serializing_if` is written wherever its predicate lets it be, read or not.
fn is_off_the_wire(field: &Field) -> bool {
    parse_serde_key_omission(&field.attrs).skips_deserializing
        && has_serde_skip_serializing(&field.attrs)
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
/// An alias is the type it stands for. A field typed with an alias of a flagged model type calls
/// that type's walker. One typed with an alias of a list or a map that `#[model_schema]` was
/// written on above is walked as the list or the map (see `aliases`), so this builds:
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
    let taken = taken_items();
    let bound = bound_items();
    quote! {
        #path
        #callback
        #handoff
        #taken
        #bound
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
            out.push(issue("Invalid", #path, #expected, ::core::option::Option::Some(#held.clone()), ::core::option::Option::Some(#reason.to_owned()), ::std::vec::Vec::new()))
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
            Key(::std::string::String),
            Index(usize),
        }

        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        #[non_exhaustive]
        pub struct Path(pub ::std::vec::Vec<Segment>);

        impl Path {
            /// Puts `value` at this path inside a JSON value, inserting the last key when it is absent.
            pub fn set_in_value(&self, root: &mut serde_json::Value, value: serde_json::Value) -> bool {
                let ::core::option::Option::Some((last, parents)) = self.0.split_last() else {
                    *root = value;
                    return true;
                };
                let mut slot = root;
                for segment in parents {
                    let next = match (segment, slot) {
                        (Segment::Key(key), serde_json::Value::Object(object)) => object.get_mut(key),
                        (Segment::Index(index), serde_json::Value::Array(items)) => items.get_mut(*index),
                        _ => ::core::option::Option::None,
                    };
                    let ::core::option::Option::Some(next) = next else { return false };
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
                let ::core::option::Option::Some((last, parents)) = self.0.split_last() else { return false };
                let mut slot = root;
                for segment in parents {
                    let next = match (segment, slot) {
                        (Segment::Key(key), serde_json::Value::Object(object)) => object.get_mut(key),
                        (Segment::Index(index), serde_json::Value::Array(items)) => items.get_mut(*index),
                        _ => ::core::option::Option::None,
                    };
                    let ::core::option::Option::Some(next) = next else { return false };
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

        impl ::core::fmt::Display for Path {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                for (position, segment) in self.0.iter().enumerate() {
                    match segment {
                        Segment::Key(key) if position == 0 => write!(f, "{key}")?,
                        Segment::Key(key) => write!(f, ".{key}")?,
                        Segment::Index(index) => write!(f, "[{index}]")?,
                    }
                }
                ::core::result::Result::Ok(())
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

/// Whether serde reads a value walked this way from the entries of an object it is flattened in.
/// It refuses every other value there, and reads an `Option` of one as absent.
fn reads_entries(walk: &Walk<'_>) -> bool {
    match &walk.step {
        Step::Entries(_) | Step::Model => true,
        Step::Items(_) | Step::Positions(_) => false,
        Step::Leaf(whole) => {
            whole.hooked || whole.parameterized || holds_any_value(walk.ty) || holds_an_id(walk.ty)
        }
        Step::Present(present) => reads_entries(present),
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

/// What answers what serde does with a type it reads flattened: the two questions asked of the
/// type's own reader through the deserializer of [`asked_items`]. A type that fills a parameter
/// carries no flag, so nothing but its own `Deserialize` can say.
fn taken_items() -> TokenStream {
    let asked = asked_items();
    quote! {
        #asked

        /// Whether `read` reads an `Option`. Flattening, serde reads one as absent wherever what
        /// it holds is refused.
        pub fn reads_an_option<'asked, T, R>(read: R, asked: &'asked Asked) -> bool
        where
            R: ::core::ops::FnOnce(TakenProbe<'asked>) -> ::core::result::Result<T, serde::de::value::Error>,
        {
            let _refused = read(TakenProbe(asked));
            asked.optional.get()
        }

        /// The keys among `keys` whose entries serde takes when it reads a type flattened there
        /// through `read`.
        fn taken_keys<'asked, 'key, T, R, K>(read: R, asked: &'asked Asked, mut keys: K) -> ::std::vec::Vec<&'key str>
        where
            R: ::core::ops::FnOnce(TakenProbe<'asked>) -> ::core::result::Result<T, serde::de::value::Error>,
            K: ::core::iter::Iterator<Item = &'key ::std::string::String>,
        {
            let _refused = read(TakenProbe(asked));
            match asked.taken.get() {
                Taken::Fields(fields) => keys
                    .map(::std::string::String::as_str)
                    .filter(|key| fields.contains(key))
                    .collect(),
                Taken::Nothing => ::std::vec::Vec::new(),
                Taken::Variant(variants) => keys
                    .find(|key| variants.contains(&key.as_str()))
                    .map(::std::string::String::as_str)
                    .into_iter()
                    .collect(),
            }
        }

        /// What remains of `entries` once serde has read a type flattened there through `read`,
        /// and `None` where it takes none of them.
        pub fn value_remaining<'asked, T, R>(
            read: R,
            asked: &'asked Asked,
            entries: &serde_json::Map<::std::string::String, serde_json::Value>,
        ) -> ::core::option::Option<serde_json::Map<::std::string::String, serde_json::Value>>
        where
            R: ::core::ops::FnOnce(TakenProbe<'asked>) -> ::core::result::Result<T, serde::de::value::Error>,
        {
            let taken = taken_keys(read, asked, entries.keys());
            if taken.is_empty() {
                return ::core::option::Option::None;
            }
            ::core::option::Option::Some(
                entries
                    .iter()
                    .filter(|(key, _)| !taken.contains(&key.as_str()))
                    .map(|(key, held)| (key.clone(), held.clone()))
                    .collect(),
            )
        }
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
    let here = path_expression(&under(
        segments,
        &quote! { ::core::result::Result::Ok(key.clone()) },
    ));
    let unknown = quote! {
        out.push(issue("Unknown", #here, &[], ::core::option::Option::Some(held.clone()), ::core::option::Option::None, ::std::vec::Vec::new()));
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
    variant: Option<&str>,
) -> Option<WalkedField<'item>> {
    let ident = field.ident.as_ref()?;
    if is_off_the_wire(field) {
        return None;
    }
    let omission = parse_serde_key_omission(&field.attrs);
    let meta = parse_serde_field_attributes(&field.attrs);
    let key = meta.rename.unwrap_or_else(|| {
        resolve_rename_rule(rename_all).apply_to_field(&ident.unraw().to_string())
    });
    let walk = (!omission.skips_deserializing).then(|| member_walk(field, module_name, parameters));
    let named = ident.unraw().to_string();
    let stem = helper_name_stem(&named, variant);
    // With a read hook on the field, serde's derive no longer reads a missing key as `None`.
    let optional =
        !has_serde_read_hook(&field.attrs) && get_field_def("", &field.ty, "").is_optional();
    Some(WalkedField {
        absence_is_read: omission.defaulted || container_defaulted || optional,
        aliases: meta.aliases,
        key,
        ty: &field.ty,
        validator: has_field_validator(module_name, &stem).then(|| FieldValidator {
            named,
            validator: format_ident!("validate_{stem}_value"),
        }),
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
    variant: Option<&str>,
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
                variant,
            ));
        } else if is_off_the_wire(field) {
            // serde neither writes nor reads the field, so no key in the object is its own.
        } else {
            flattened.push(Flattened::of(field, module_name, parameters));
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

/// Whether the walker of `ty` is called as it stands: `ty` is the type being walked, or a struct
/// or an enum `#[model_schema]` was written on above, under its own name or an alias of it.
fn walks_itself(ty: &Type, depth: usize) -> bool {
    let Type::Path(named) = written_type(ty) else {
        return false;
    };
    let Some(last) = named.path.segments.last() else {
        return false;
    };
    if last.ident == "Self" {
        return true;
    }
    match declared(&last.ident.unraw().to_string()) {
        Some(Declared::Model) => true,
        Some(Declared::Alias(aliased_as)) if depth < 8 => syn::parse_str::<Type>(&aliased_as)
            .is_ok_and(|aliased| walks_itself(&aliased, depth.saturating_add(1))),
        Some(Declared::Alias(_)) | None => false,
    }
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
