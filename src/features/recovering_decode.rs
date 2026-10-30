//! The recovering decode `#[model_schema(enable = ["decode_with"])]` turns on, from JSON and, under
//! `bson`, from BSON, and the MongoDB layer `"mongodb"` adds to it.

mod aliases;
mod checks;
pub mod enums;
#[cfg(feature = "mongodb")]
mod fields;
#[cfg(feature = "mongodb")]
mod operations;
#[cfg(feature = "mongodb")]
mod query;

use core::iter::once;
use core::mem::take;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use proc_macro2::{Group, Spacing, TokenTree};
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{ToTokens as _, format_ident, quote};
use syn::ext::IdentExt as _;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, DeriveInput, Field, Fields, FieldsNamed, GenericArgument, GenericParam, Generics,
    Item, ItemStruct, LitStr, PathArguments, PredicateType, Token, Type, TypeParamBound, TypePath,
    Variant, WherePredicate, parse_quote,
};

use self::aliases::Reach;
use crate::features::model_schema_prop::FunctionName;
use crate::features::serde::read_bounds::serialize_bound;
use crate::features::serde::{
    Direction, NAMED_READ_HOOK_PREFIX, SerdeFieldHooks, Tagging, has_serde_default,
    has_serde_read_hook, has_serde_skip_serializing, has_serde_transparent, parse_read_renames,
    parse_serde_field_attributes, parse_serde_field_hooks, parse_serde_key_omission,
    parse_serde_type_attributes, parse_written_renames, transparent_field,
};
use crate::field_type::{
    FieldDefType, RESULT_ARMS, get_field_def, is_map_wrapper, is_refused_sequence_wrapper,
    is_sequence_wrapper, is_transparent_wrapper, written_std_result,
};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use crate::model_schema::judged_read::bounded_listed_read;
use crate::model_schema::judged_read::{
    HookScope, authored_read_bounds, listed_read, listed_reader,
};
use crate::model_schema::preprocess;
use crate::model_schema::{helper_name_stem, recovering_bound_check};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{
    Declared, asked_of, asked_trait, asking_more, asks_one_of, declared,
    field_validator_judges_items, field_validator_name, holds_field_validator,
    ident_schema_module_name, to_snake_case, type_parameters_in_scope, written_names, written_type,
};

#[cfg(all(
    any(feature = "typescript", feature = "zod", feature = "jsonschema"),
    not(feature = "mongodb")
))]
const ADDED_TYPE_COUNT: usize = 17;

#[cfg(all(
    any(feature = "typescript", feature = "zod", feature = "jsonschema"),
    feature = "mongodb"
))]
const ADDED_TYPE_COUNT: usize = 39;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const ADDED_TYPE_NAMES: [&str; ADDED_TYPE_COUNT] = [
    #[cfg(feature = "mongodb")]
    "Arm",
    #[cfg(feature = "mongodb")]
    "ArmFilter",
    #[cfg(feature = "mongodb")]
    "ArmUpdate",
    #[cfg(feature = "mongodb")]
    "AsFilter",
    #[cfg(feature = "mongodb")]
    "AsUpdate",
    "Asked",
    "DecodeWith",
    #[cfg(feature = "mongodb")]
    "Element",
    "EntryOf",
    "Expected",
    "ExpectedToken",
    #[cfg(feature = "mongodb")]
    "Field",
    #[cfg(feature = "mongodb")]
    "Filter",
    "Issue",
    "IssueFromParts",
    #[cfg(feature = "mongodb")]
    "ListField",
    #[cfg(feature = "mongodb")]
    "Model",
    #[cfg(feature = "mongodb")]
    "ModelList",
    #[cfg(feature = "mongodb")]
    "MongoFields",
    #[cfg(feature = "mongodb")]
    "MongoPath",
    #[cfg(feature = "mongodb")]
    "MongoSlots",
    #[cfg(feature = "mongodb")]
    "Nested",
    #[cfg(feature = "mongodb")]
    "Nests",
    #[cfg(feature = "mongodb")]
    "OperationError",
    #[cfg(feature = "mongodb")]
    "OptionalField",
    #[cfg(feature = "mongodb")]
    "OptionalModel",
    "Path",
    #[cfg(feature = "mongodb")]
    "Read",
    "ReadWhole",
    "Resolution",
    "Resolver",
    "Segment",
    "Stored",
    "Taken",
    "TakenProbe",
    "Unrecovered",
    #[cfg(feature = "mongodb")]
    "Update",
    "Verdict",
    #[cfg(feature = "mongodb")]
    "WriteError",
];

/// The crate-root module `decode_with_foundation!()` writes.
const DECODE_WITH_LIBRARY: &str = "__tixschema_decode_with";

/// The reason the `#[expect(clippy::pub_use)]` on each re-export of a support library gives.
const REEXPORT_REASON: &str = "tixschema re-exports the crate's support library in each schema \
                               module by name, so every `{type}_schema::Name` path stays \
                               reachable from this crate and from another";

/// Every name the `decode_with` library makes public in this build, each of which a `{type}_schema`
/// module re-exports by name.
const DECODE_WITH_LIBRARY_NAMES: &[&str] = &[
    "Asked",
    "DecodeWith",
    "Expected",
    "ExpectedToken",
    "Issue",
    "IssueFromParts",
    "Path",
    "Resolution",
    "Resolver",
    "Segment",
    "Stored",
    "Taken",
    "TakenProbe",
    "Unrecovered",
    "Verdict",
    #[cfg(feature = "bson")]
    "bson_bound",
    #[cfg(feature = "bson")]
    "bson_digits_settled",
    #[cfg(feature = "bson")]
    "bson_leaf",
    #[cfg(feature = "bson")]
    "bson_remaining",
    "issue_from_parts",
    "reads_an_option",
    #[cfg(feature = "bson")]
    "same_bracket",
    "taken_keys",
    "unsettled",
    "value_bound",
    "value_digits_settled",
    "value_leaf",
    "value_remaining",
];

/// The crate-root module `mongodb_foundation!()` writes.
#[cfg(feature = "mongodb")]
const MONGODB_LIBRARY: &str = "__tixschema_mongodb";

/// Every name the MongoDB library makes public, each of which the `{type}_schema` module of a type
/// that lists "mongodb" re-exports by name.
#[cfg(feature = "mongodb")]
const MONGODB_LIBRARY_NAMES: &[&str] = &[
    "ArmFilter",
    "ArmUpdate",
    "AsFilter",
    "AsUpdate",
    "Element",
    "Field",
    "Filter",
    "ListField",
    "Model",
    "ModelList",
    "MongoPath",
    "Nested",
    "Nests",
    "OperationError",
    "OptionalField",
    "OptionalModel",
    "Read",
    "Update",
    "WriteError",
];

/// A side's bounds, and the predicates the container's `bound` writes for it in their place.
type Sides<'bound> = (
    Punctuated<TypeParamBound, Token![+]>,
    Option<&'bound [WherePredicate]>,
);

/// One arm reading a value that is there: the pattern it is held under, and what is listed for it.
struct Arm {
    body: TokenStream,
    /// The arm lists nothing: a `null` where the field is optional.
    nothing: bool,
    pattern: TokenStream,
}

/// The casing rule a container gives its fields in each of serde's two directions.
#[derive(Clone, Copy)]
struct Casing<'rule> {
    read: Option<&'rule str>,
    written: Option<&'rule str>,
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
    Entries(Walk<'item>),
    Model(&'item Type),
    /// An `Option` of a flagged type, which serde reads as absent where it does not read the type:
    /// that type, then the field's own, which an issue names as expected.
    Optional(&'item Type, &'item Type),
    /// A field no walk reaches: every key counts as its own, and serde's verdict is the read's.
    /// It holds the field's type, and `None` for a field serde never reads.
    Unwalked(Option<&'item Type>),
    Whole(Walk<'item>),
}

impl<'item> Flattened<'item> {
    /// The keys it declares are its own alone, known once its type's fields walker has run.
    const fn declares_its_keys(&self) -> bool {
        matches!(self, Self::Model(_) | Self::Optional(_, _))
    }

    fn of(
        field: &'item Field,
        module_name: &str,
        parameters: &[String],
        corrected: &[FunctionName],
    ) -> Self {
        // serde reads nothing into the field, and the keys it wrote for it are still in the object.
        if parse_serde_key_omission(&field.attrs).skips_deserializing {
            return Self::Unwalked(None);
        }
        let Walk { step, ty } = member_walk(field, module_name, parameters, corrected);
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
            Step::Arms(_, _)
            | Step::Items(_)
            | Step::Leaf(_)
            | Step::Positions(_)
            | Step::Present(_) => Self::Unwalked(Some(&field.ty)),
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
                Step::Arms(_, _)
                | Step::Entries(_)
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
    declared: Vec<String>,
    fields: Vec<WalkedField<'item>>,
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
    /// The `impl` holding `decode_with_check_messages`, under the bounds the type's author wrote:
    /// the entry points of every source run it on the value serde built.
    pub check_messages: TokenStream,
    /// What refuses a crate that has not written the support libraries the type re-exports,
    /// written beside the type.
    pub probes: TokenStream,
    /// The items added to the `{type}_schema` module a schema surface writes, or that module whole
    /// in a build where none writes one.
    pub schema_module: TokenStream,
    /// The `impl` holding the entry points and the walkers, or one per source on a type with a
    /// type parameter, and whatever else is written beside the type.
    pub type_impl: TokenStream,
}

/// What the walker of a struct, or of what a variant holds, walks: the form serde writes it in.
enum Shape<'item> {
    Fields(Keyed<'item>),
    Held(Walk<'item>),
    Nothing,
    Slots(Vec<Slot<'item>>),
}

impl<'item> Shape<'item> {
    fn of(item_struct: &'item ItemStruct, module_name: &str, parameters: &[String]) -> Self {
        if has_serde_transparent(&item_struct.attrs)
            && let Some(only) = transparent_field(&item_struct.fields, Direction::Read)
        {
            // The slot of a brand names no function: a `preprocess` there is refused.
            return Self::Held(held_walk(only, module_name, parameters, &[]));
        }
        let named_on = preprocess::named();
        let defaulted = has_serde_default(&item_struct.attrs);
        match &item_struct.fields {
            Fields::Named(named) => {
                let container = parse_serde_type_attributes(&item_struct.attrs);
                let (read, written) = (
                    parse_read_renames(&item_struct.attrs).rename_all,
                    parse_written_renames(&item_struct.attrs).rename_all,
                );
                let mut keyed = walked_fields(
                    named,
                    Casing {
                        read: read.as_deref(),
                        written: written.as_deref(),
                    },
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
                    return Self::Held(held_walk(
                        only,
                        module_name,
                        parameters,
                        named_on.of_member(None, 0),
                    ));
                }
                Self::Slots(
                    slots
                        .unnamed
                        .iter()
                        .enumerate()
                        .filter_map(|(index, slot)| {
                            let validator = slot_validator(module_name, None, index);
                            walked_slot(
                                slot,
                                defaulted,
                                module_name,
                                parameters,
                                validator,
                                named_on.of_member(None, index),
                            )
                        })
                        .collect(),
                )
            }
        }
    }

    /// What `variant` holds. Its fields are cased by its own `rename_all`, and by the enum's
    /// `rename_all_fields` where it writes none.
    fn of_variant(
        variant: &'item Variant,
        rename_all_fields: Casing<'_>,
        module_name: &str,
        parameters: &[String],
    ) -> Self {
        match &variant.fields {
            Fields::Named(named) => {
                let (read, written) = (
                    parse_read_renames(&variant.attrs).rename_all,
                    parse_written_renames(&variant.attrs).rename_all,
                );
                Self::Fields(walked_fields(
                    named,
                    Casing {
                        read: read.as_deref().or(rename_all_fields.read),
                        written: written.as_deref().or(rename_all_fields.written),
                    },
                    false,
                    module_name,
                    parameters,
                    Some(&variant.ident.to_string()),
                ))
            }
            Fields::Unit => Self::Nothing,
            Fields::Unnamed(slots) => {
                let named = variant.ident.to_string();
                let named_on = preprocess::named();
                let mut written = slots.unnamed.iter();
                if let (Some(only), None) = (written.next(), written.next()) {
                    // serde writes a variant whose one slot it neither writes nor reads as a unit
                    // variant.
                    if parse_serde_key_omission(&only.attrs).absent_from_wire() {
                        return Self::Nothing;
                    }
                    return Self::Held(member_walk(
                        only,
                        module_name,
                        parameters,
                        named_on.of_member(Some(&named), 0),
                    ));
                }
                Self::Slots(
                    slots
                        .unnamed
                        .iter()
                        .enumerate()
                        .filter_map(|(index, slot)| {
                            let validator = slot_validator(module_name, Some(&named), index);
                            walked_slot(
                                slot,
                                false,
                                module_name,
                                parameters,
                                validator,
                                named_on.of_member(Some(&named), index),
                            )
                        })
                        .collect(),
                )
            }
        }
    }
}

/// One position of a tuple as its walker reads it.
struct Slot<'item> {
    absence_is_read: bool,
    ty: &'item Type,
    /// `None` for a position of a tuple-typed field, which declares no bound.
    validator: Option<FieldValidator>,
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

/// What one source's entry points write: `reads`, the methods of the type's `DecodeWith` impl,
/// and `helpers`, the inherent functions they run.
struct Entry {
    helpers: TokenStream,
    reads: TokenStream,
}

impl Source {
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
    /// parameter stays bounded in one place: its `where` predicate if it has one, else its name. A
    /// side the container's `bound` writes joins nothing to the parameters.
    fn bounded(self, generics: &Generics, bound: &WrittenBound) -> Generics {
        let read: Sides<'_> = (
            parse_quote!(serde::de::DeserializeOwned),
            bound.read_written.then_some(&[][..]),
        );
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => bounded_by(
                generics,
                &[
                    read,
                    (parse_quote!(serde::Serialize), bound.write.as_deref()),
                ],
            ),
            Self::Json => bounded_by(generics, &[read]),
        }
    }

    /// The pattern a map is held under, binding its entries.
    fn entries(self, entries: &Ident) -> TokenStream {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => quote! { bson::Bson::Document(#entries) },
            Self::Json => quote! { serde_json::Value::Object(#entries) },
        }
    }

    /// The entry point and the report it runs. `written.names` holds every name the type's item
    /// writes, none of which a method's own type parameter takes.
    fn entry_methods(self, module: &Ident, written: &Written<'_>, corrects_own: bool) -> Entry {
        match self {
            #[cfg(feature = "bson")]
            Self::Bson => bson_entry_methods(module, &written.names, corrects_own, written.mongodb),
            Self::Json => entry_methods(
                module,
                &unclaimed_parameter("F", &written.names),
                corrects_own,
            ),
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

    /// The sources a type's walkers read: every source this build generates.
    const fn read_by() -> &'static [Self] {
        Self::GENERATED
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
    /// A std `Result`: the walk of its `Ok` and that of its `Err`.
    Arms(Box<Walk<'ty>>, Box<Walk<'ty>>),
    Entries(Box<Walk<'ty>>),
    Items(Box<Walk<'ty>>),
    Leaf(Whole),
    Model,
    Positions(Vec<Slot<'ty>>),
    Present(Box<Walk<'ty>>),
}

/// What `mongodb` adds for a type's typed paths: beside the type, in its schema module, and in
/// its own `impl`. Empty in a build without it.
#[derive(Default)]
struct TypedPaths {
    beside: TokenStream,
    module_items: TokenStream,
    type_items: TokenStream,
}

/// How one value is walked: the step taken at it, and the type it is read as.
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
    /// The validator judges each item of the field's list, and none of the list itself.
    items: bool,
    /// The name the validator writes in front of each violation: the steps serde writes on the
    /// way to the field. `None` for a validator that writes no name.
    named: Option<String>,
    validator: Ident,
}

/// One field of the struct as its walker reads it.
struct WalkedField<'item> {
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
    /// The value is read from the payload a `preprocess` of its member corrected, so what it is
    /// stored as is not held to what the type writes back.
    corrected: bool,
    hooked: bool,
    /// The value's type names one of the type's own parameters, which nothing shows to be
    /// `Serialize` where the JSON walker reads it.
    parameterized: bool,
    read: TokenStream,
    /// The function a hook writes the value to a serializer with, and `None` for the type's own
    /// `Serialize`.
    write: Option<TokenStream>,
}

/// What the container's `#[serde(bound)]` writes of the two sides.
struct WrittenBound {
    /// The `deserialize` side is written: `read_under` joins its predicates.
    read_written: bool,
    /// The predicates of the `serialize` side, where written.
    write: Option<Vec<WherePredicate>>,
}

impl WrittenBound {
    fn of(attrs: &[Attribute], read_written: bool) -> Self {
        Self {
            read_written,
            write: serialize_bound(attrs),
        }
    }
}

/// What a type's item writes, as the walker's methods are written from it.
struct Written<'reach> {
    /// What the container's `bound` writes, which the generated impls take in place of serde's
    /// own bounds on the type parameters.
    bound: WrittenBound,
    /// `decode_with_check_messages`, written from the checks the item and its members name.
    check_messages: TokenStream,
    /// The item lists "mongodb" in `enable`: it is stored, and gets the MongoDB layer under
    /// `mongodb`.
    mongodb: bool,
    names: Vec<String>,
    reach: &'reach Reach,
    /// `decode_with_read_unjudged`, the read every source lists from.
    read: TokenStream,
    /// What serde's derive asks of the type's parameters to read it, beside `Deserialize`: the
    /// read every source lists from is written under the same.
    read_predicates: Vec<WherePredicate>,
    /// What a model the item holds is read as in the walkers' methods, written under the item's
    /// own parameters and bounds.
    scope: HookScope,
}

/// What the generated code names of the type it is generated for, and the source it walks.
struct Walker<'item> {
    /// The type names a `preprocess` of its own: its walkers read a value its functions have
    /// corrected, and the methods another type's walker calls correct the value first.
    corrects_own: bool,
    issue_parameter: &'item Ident,
    module: &'item Ident,
    own_name: &'item str,
    parameters: &'item [String],
    scope: &'item HookScope,
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
            Step::Arms(ok, err) => self.result_arms([ok, err], held, segments, depth, reported),
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
                // A bound written on the field measures the list, and none of its items, but a
                // literal, which each item holds.
                let (of_items, of_list) = match validator {
                    Some(judged) if judged.items => (Some(judged), None),
                    _ => (None, validator),
                };
                let each = self.statement(
                    inner,
                    &item,
                    &under(segments, &quote! { ::core::result::Result::Err(#index) }),
                    depth,
                    of_items,
                );
                let expected = self.expected(reported);
                let walked =
                    quote! {{ for (#index, #item) in #items.iter().enumerate() { #each } }};
                let whole = read_as_a_list(walk.ty);
                let matched = source.items(&items);
                vec![
                    Arm {
                        // The items are walked one by one, so the list is read with none judged.
                        body: self.bounded(
                            &walked,
                            of_list,
                            &whole,
                            held,
                            &(!inner.reads_whole())
                                .then(|| listed_reader(&whole, self.scope))
                                .flatten()
                                .unwrap_or_else(|| own_reader(&whole)),
                            &path,
                        ),
                        nothing: false,
                        // The bound reads the list whole, so the arm keeps hold of it.
                        pattern: if of_list.is_some() {
                            quote! { #held @ #matched }
                        } else {
                            matched
                        },
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

    /// The closure answering what `validator`'s bound refuses of a value of type `ty`, or `None`
    /// where the member has no validator, or its bound reaches nothing of `ty`.
    fn bound_check(&self, validator: Option<&FieldValidator>, ty: &Type) -> Option<TokenStream> {
        let member = validator?;
        let (module, checked_by) = (self.module, &member.validator);
        recovering_bound_check(
            &quote! { #module::#checked_by },
            member.named.as_deref(),
            ty,
        )
    }

    /// `listed` followed by what holds the value under `held`, read through `read` as `ty`, to the
    /// bound `validator` checks: one `Invalid` at the value, its violations joined. `listed` alone
    /// where the field has no validator, or its bound reaches nothing of `ty`.
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
        let Some(check) = self.bound_check(validator, ty) else {
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

    /// What a type that names a `preprocess` of its own adds for this source: the function that
    /// answers a value as the type's functions correct it, and the three methods another type's
    /// walker calls, each of which corrects a copy of what it is handed and walks that. A key the
    /// functions took out of an object is the type's own, as each key its fields are read from.
    fn corrected_entries(&self, functions: &[FunctionName]) -> TokenStream {
        let (module, source, built) = (self.module, self.source, self.issue_parameter);
        let (value, object_type, expected) = (source.value(), source.object(), self.own_model());
        let [issues, named, fields] = ["issues", "named", "fields"].map(|part| source.method(part));
        let [of_issues, of_named, of_fields] =
            ["issues", "named", "fields"].map(|part| self.own_method(part));
        let (corrector, signature) = (source.method("corrected"), self.signature());
        let (correction, as_a_value, as_an_object) = match source {
            #[cfg(feature = "bson")]
            Source::Bson => (
                quote! {
                    /// What the functions `preprocess` names on the type answer for `whole`, which
                    /// crosses to a `serde_json::Value` and back through serde. `Err` holds what
                    /// serde says of a value one of the two cannot hold.
                    fn #corrector(whole: bson::Bson) -> ::core::result::Result<bson::Bson, ::std::string::String> {
                        let crossed = <::serde_json::Value as ::serde::Deserialize>::deserialize(bson::Deserializer::new(whole))
                            .map_err(|refused| refused.to_string())?;
                        ::serde::Serialize::serialize(&Self::decode_with_value_corrected(crossed), bson::Serializer::new())
                            .map_err(|refused| refused.to_string())
                    }
                },
                quote! { Self::#corrector(found.clone()) },
                quote! { ::core::result::Result::Ok(bson::Bson::Document(corrected)) = Self::#corrector(bson::Bson::Document(object.clone())) },
            ),
            Source::Json => {
                let taken: Vec<String> = functions
                    .iter()
                    .map(|function| to_snake_case(&function.name))
                    .collect();
                let handed = unclaimed_parameter("value", &taken);
                let answered = preprocess::corrected(functions, quote! { #handed });
                (
                    quote! {
                        /// What the functions `preprocess` names on the type answer for a value.
                        fn #corrector(#handed: serde_json::Value) -> serde_json::Value {
                            #answered
                        }
                    },
                    quote! { ::core::result::Result::<serde_json::Value, ::std::string::String>::Ok(Self::#corrector(found.clone())) },
                    quote! { serde_json::Value::Object(corrected) = Self::#corrector(serde_json::Value::Object(object.clone())) },
                )
            }
        };
        quote! {
            #correction

            /// Lists every issue in `found`, read as this type at `path`, once the functions
            /// `preprocess` names on the type have corrected it.
            pub fn #issues #signature {
                match #as_a_value {
                    ::core::result::Result::Ok(corrected) => Self::#of_issues(&corrected, path, issue, out),
                    ::core::result::Result::Err(reason) => out.push(issue("Invalid", path.to_vec(), #expected, ::core::option::Option::Some(found.clone()), ::core::option::Option::Some(reason), ::std::vec::Vec::new())),
                }
            }

            /// Whether `object`, once the functions `preprocess` names on the type have corrected
            /// it, holds what names a value of this type.
            pub fn #named(object: &#object_type) -> bool {
                let #as_an_object else {
                    return false;
                };
                Self::#of_named(&corrected)
            }

            /// Lists every issue in this type's fields inside `object`, once the functions
            /// `preprocess` names on the type have corrected it, and returns the keys that are its
            /// own: each one its fields are read from, and each one the functions took out.
            pub fn #fields<'a, #built>(
                object: &'a #object_type,
                path: &[::core::result::Result<::std::string::String, usize>],
                issue: #module::IssueFromParts<#value, #built>,
                out: &mut ::std::vec::Vec<#built>,
            ) -> ::std::vec::Vec<&'a str> {
                let #as_an_object else {
                    return ::std::vec::Vec::new();
                };
                let declared = Self::#of_fields(&corrected, path, issue, out);
                object
                    .keys()
                    .map(::std::string::String::as_str)
                    .filter(|key| declared.contains(key) || !corrected.contains_key(*key))
                    .collect()
            }
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
            (self.own_method("fields"), source.value(), source.object());
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
            // A hook reads whatever it asks for, and a value of a parameter's type is whatever
            // fills it, an object among it.
            Step::Leaf(whole)
                if (whole.hooked || whole.parameterized)
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
            Step::Arms(_, _)
            | Step::Items(_)
            | Step::Leaf(_)
            | Step::Positions(_)
            | Step::Present(_) => return None,
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
    /// struct, and a `#[serde(transparent)]` struct of either kind. `validator` checks the bound
    /// written on a tuple struct's one slot.
    fn held_methods(&self, walk: &Walk<'_>, validator: Option<&FieldValidator>) -> TokenStream {
        let found = Ident::new("found", Span::call_site());
        let walked = match &walk.step {
            Step::Arms(_, _)
            | Step::Entries(_)
            | Step::Items(_)
            | Step::Model
            | Step::Positions(_)
            | Step::Present(_) => self.issues_method(&Self::listed(
                &found,
                &self.arms(walk, &found, &[], 0, walk.ty, validator),
            )),
            // The type's own reader runs whatever hook its slot carries.
            Step::Leaf(_) => {
                let whole = self.read_whole(&self.expected(walk.ty));
                let bound = self.lone_slot_bound(validator, walk.ty);
                self.issues_method(&quote! { #whole; #bound })
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
        let issues = self.own_method("issues");
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
        let written = if whole.corrected {
            source.unwritten(pinned)
        } else {
            source.written(whole.parameterized, pinned, whole.write.as_ref())
        };
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

    /// What holds `found`, read whole with the type's own reader, to the bound `validator` checks
    /// on its one slot, typed `ty`: one `Invalid` at the value, its violations joined. Nothing
    /// where the slot has no validator, or its bound reaches nothing of `ty`.
    fn lone_slot_bound(&self, validator: Option<&FieldValidator>, ty: &Type) -> TokenStream {
        let (module, bound) = (self.module, self.source.bound());
        let Some(check) = self.bound_check(validator, ty) else {
            return TokenStream::new();
        };
        let (expected, own_read) = (self.expected(ty), self.own_read());
        quote! {
            out.extend(#module::#bound(
                found,
                Self::#own_read,
                |held: &Self| {
                    let check = #check;
                    check(&held.0)
                },
                path.to_vec(),
                #expected,
                issue,
            ));
        }
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

    /// The methods one type's walker is called through by another's. `lone_bound` is the validator
    /// of the bound written on the one slot of a tuple struct.
    fn methods(&self, shape: &Shape<'_>, lone_bound: Option<&FieldValidator>) -> TokenStream {
        match shape {
            Shape::Fields(keyed) => self.object_methods(keyed),
            Shape::Held(walk) => self.held_methods(walk, lone_bound),
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
        let (named, object_type) = (self.own_method("named"), self.source.object());
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
        let fields = self.own_method("fields");
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

    /// One of the methods that walk a value of the type itself: where the type corrects its own
    /// payload, the one that walks a value its functions have corrected.
    fn own_method(&self, part: &str) -> Ident {
        if self.corrects_own {
            self.source.method(&format!("corrected_{part}"))
        } else {
            self.source.method(part)
        }
    }

    /// What an issue names as expected where the type itself is: `Model`, by the type's name.
    fn own_model(&self) -> TokenStream {
        let own_name = self.own_name;
        quote! { &[("Model", &[#own_name], 0)] }
    }

    /// The read of the type itself its walkers list from: where the type corrects its own
    /// payload, the read of a payload its functions have corrected.
    fn own_read(&self) -> Ident {
        own_read(self.corrects_own)
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
            slot.validator.as_ref(),
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
        let (module, leaf, own_read) = (self.module, self.source.leaf(), self.own_read());
        let written = self.written_whole();
        quote! {
            out.extend(#module::#leaf(found, Self::#own_read, #written, path.to_vec(), #expected, issue))
        }
    }

    /// [`Self::read_whole`] where `found` is the entries serde reads the type from flattened:
    /// nothing is written back, and an `Option` that is `unseen` here is absent where refused.
    fn read_whole_flattened(&self, unseen: bool, expected: &TokenStream) -> TokenStream {
        let (module, leaf) = (self.module, self.source.leaf());
        let own: Type = parse_quote! { Self };
        let own_read = self.own_read();
        let read = quote! { Self::#own_read };
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

    /// The arms a `Result` held under `held` is matched by: an object of one key, `Ok` or `Err`,
    /// whose value is walked under that key as the arm holds it, and one issue for any other.
    fn result_arms(
        &self,
        [ok, err]: [&Walk<'_>; 2],
        held: &Ident,
        segments: &[TokenStream],
        depth: usize,
        reported: &Type,
    ) -> Vec<Arm> {
        let path = path_expression(segments);
        let (object, held_by_arm) = (binding("arms", depth), binding("arm", depth));
        let expected = self.expected(reported);
        let walked = RESULT_ARMS.into_iter().zip([ok, err]).map(|(arm, inner)| {
            let each = self.statement(
                inner,
                &held_by_arm,
                &under(
                    segments,
                    &quote! { ::core::result::Result::Ok(#arm.to_owned()) },
                ),
                depth,
                None,
            );
            quote! {
                if let (1, ::core::option::Option::Some(#held_by_arm)) = (#object.len(), #object.get(#arm)) {
                    #each
                } else
            }
        });
        let refused = not_the_shape(
            held,
            &path,
            &expected,
            "not an object of one key, Ok or Err",
        )
        .body;
        let matched = self.source.entries(&object);
        vec![
            Arm {
                body: quote! {{ #(#walked)* { #refused; } }},
                nothing: false,
                pattern: quote! { #held @ #matched },
            },
            not_the_shape(held, &path, &expected, "not an object"),
        ]
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
/// It answers for each of `sources`, the ones the type's own walkers read.
fn read_whole_items(sources: &[Source]) -> TokenStream {
    let object = Ident::new("object", Span::call_site());
    let own: Type = parse_quote! { Self };
    let methods = sources.iter().map(|&source| {
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
    let every = sources.iter().map(|&source| {
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
/// hangs on a field is one the walker reads through. `mongodb` says whether the struct lists
/// "mongodb" in `enable`.
pub fn struct_recovering_decode(item_struct: &ItemStruct, mongodb: bool) -> RecoveringDecode {
    listed_from(
        item_struct,
        listed_read(&DeriveInput::from(item_struct.clone())),
        mongodb,
    )
}

/// [`struct_recovering_decode`] of a brand that carries a bound: the read its issues are listed
/// from refuses a value that breaks the bound.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
pub fn brand_recovering_decode(item_struct: &ItemStruct, mongodb: bool) -> RecoveringDecode {
    listed_from(
        item_struct,
        bounded_listed_read(&DeriveInput::from(item_struct.clone())),
        mongodb,
    )
}

/// The recovering decode of a struct whose issues are listed from `read`.
fn listed_from(item_struct: &ItemStruct, read: TokenStream, mongodb: bool) -> RecoveringDecode {
    let module_name = ident_schema_module_name(&item_struct.ident.to_string());
    let input = DeriveInput::from(item_struct.clone());
    let authored = authored_read_bounds(&input);
    let scope = HookScope::of(&input, &authored);
    let parameters = type_parameters_in_scope(&item_struct.generics);
    let mut reach = Reach::new(&module_name);
    let mut walked = item_struct.clone();
    reach.fields(&mut walked.fields);
    let shape = Shape::of(&walked, &module_name, &parameters);
    let lone_bound = value_slot(&walked)
        .filter(|only| !parse_serde_key_omission(&only.attrs).skips_deserializing)
        .and_then(|_| slot_validator(&module_name, None, 0));
    let names = written_names(item_struct.to_token_stream());
    let paths = if mongodb {
        struct_paths(item_struct, &names)
    } else {
        TypedPaths::default()
    };
    added_to(
        &item_struct.ident,
        &item_struct.generics,
        &module_name,
        &parameters,
        &Written {
            check_messages: checks::struct_check_messages(item_struct, &module_name, &parameters),
            mongodb,
            names,
            reach: &reach,
            read,
            read_predicates: authored.predicates,
            scope,
            bound: WrittenBound::of(&item_struct.attrs, authored.written_by_container),
        },
        &paths,
        |walker| walker.methods(&shape, lone_bound.as_ref()),
    )
}

/// The slot serde writes a struct as the value of: the field `#[serde(transparent)]` reads the
/// struct as, or the only slot of a tuple struct.
pub fn value_slot(item_struct: &ItemStruct) -> Option<&Field> {
    let transparent = if has_serde_transparent(&item_struct.attrs) {
        transparent_field(&item_struct.fields, Direction::Read)
    } else {
        None
    };
    transparent.or_else(|| match &item_struct.fields {
        Fields::Unnamed(slots) if slots.unnamed.len() == 1 => slots.unnamed.first(),
        Fields::Named(_) | Fields::Unit | Fields::Unnamed(_) => None,
    })
}

/// Whether serde writes `item` as one value, under no key of its own: a struct written as the
/// value of one slot, a unit struct, or an enum no variant of which holds a value and no
/// attribute tags.
pub fn written_as_one_value(item: &Item) -> bool {
    if let Item::Enum(item_enum) = item {
        matches!(Tagging::of(item_enum), Tagging::Plain)
    } else if let Item::Struct(item_struct) = item {
        matches!(item_struct.fields, Fields::Unit) || value_slot(item_struct).is_some()
    } else {
        false
    }
}

/// What the flag adds to the type `name` declares under `generics`: the callback's types, and per
/// source the entry point beside what `methods` writes for that source's walker. `written` holds
/// every name the type's item writes, none of which a method's own type parameter takes, and what
/// its fields reach through the aliases they are typed with. `paths` holds the type's typed paths.
fn added_to<M>(
    name: &Ident,
    generics: &Generics,
    module_name: &str,
    parameters: &[String],
    written: &Written<'_>,
    paths: &TypedPaths,
    methods: M,
) -> RecoveringDecode
where
    M: Fn(&Walker<'_>) -> TokenStream,
{
    let own_name = name.to_string();
    let module = Ident::new(module_name, Span::call_site());
    let issue_parameter = unclaimed_parameter("I", &written.names);
    let own_functions = preprocess::named().own;
    let corrects_own = !own_functions.is_empty();
    let sources = Source::read_by();
    let of_sources: Vec<(TokenStream, TokenStream)> = sources
        .iter()
        .enumerate()
        .map(|(at, &source)| {
            let walker = Walker {
                corrects_own,
                issue_parameter: &issue_parameter,
                module: &module,
                own_name: &own_name,
                parameters,
                scope: &written.scope,
                source,
            };
            let Entry { helpers, reads } = source.entry_methods(&module, written, corrects_own);
            let walk_methods = methods(&walker);
            let corrected = corrects_own.then(|| walker.corrected_entries(&own_functions));
            // Every source reads through it, and the first one's bounds are the fewest.
            let read = (at == 0).then_some(&written.read);
            let inherent = quote! {
                #helpers
                #walk_methods
                #corrected
                #read
            };
            (inherent, reads)
        })
        .collect();
    let mut type_impl = type_impls(
        name,
        generics,
        written,
        &module,
        (sources, &of_sources),
        &paths.type_items,
    );
    let mut items = library_reexport(written.mongodb);
    items.extend(paths.module_items.clone());
    if written_names(type_impl.clone())
        .iter()
        .any(|written_name| written_name == "ReadWhole")
    {
        items.extend(read_whole_items(sources));
    }
    items.extend(written.reach.module_items());
    type_impl.extend(paths.beside.clone());
    type_impl.extend(written.reach.guards());
    RecoveringDecode {
        check_messages: checks::type_impl(name, generics, &module, &written.check_messages),
        probes: foundation_probes(written.mongodb),
        schema_module: placed_in_schema_module(&module, &items),
        type_impl,
    }
}

/// What a `{type}_schema` module holds of its crate's support libraries: every name each one makes
/// public, written out, so each shadows a name the module's `use super::*` reads, as an item the
/// module declares does. The MongoDB library is held where the type lists "mongodb", under
/// `mongodb`.
fn library_reexport(mongodb: bool) -> TokenStream {
    let mut reexport = reexported(DECODE_WITH_LIBRARY, DECODE_WITH_LIBRARY_NAMES);
    if mongodb {
        reexport.extend(mongodb_reexport());
    }
    reexport
}

/// `pub use crate::{library}::{...};`, every one of `names` written out, expecting the
/// `clippy::pub_use` a consumer who denies the restriction group would otherwise meet on it.
fn reexported(library: &str, names: &[&str]) -> TokenStream {
    let module = Ident::new(library, Span::call_site());
    let each = names.iter().map(|name| Ident::new(name, Span::call_site()));
    quote! {
        #[expect(clippy::pub_use, reason = #REEXPORT_REASON)]
        pub use crate::#module::{#(#each),*};
    }
}

/// The MongoDB library, re-exported by name.
#[cfg(feature = "mongodb")]
fn mongodb_reexport() -> TokenStream {
    reexported(MONGODB_LIBRARY, MONGODB_LIBRARY_NAMES)
}

/// A build without `mongodb` writes no MongoDB library.
#[cfg(not(feature = "mongodb"))]
fn mongodb_reexport() -> TokenStream {
    TokenStream::new()
}

/// What refuses a crate that holds this type and has not written the support libraries it
/// re-exports: the `decode_with` one always, and the MongoDB one where the type lists "mongodb",
/// under `mongodb`.
fn foundation_probes(mongodb: bool) -> TokenStream {
    let mut probes = foundation_probe(
        "__tixschema_decode_with_foundation",
        "tixschema: this crate has `#[model_schema(enable = [\"decode_with\"])]` types but no \
         `tixschema::decode_with_foundation!();`",
        "this type needs the decode_with support library, which \
         `tixschema::decode_with_foundation!();` writes once per crate",
        "Fix: add the line `tixschema::decode_with_foundation!();` at the crate root (src/lib.rs \
         or src/main.rs), on its own line, anywhere outside a function. Add it once per crate; \
         nothing else is needed.",
    );
    if mongodb {
        probes.extend(mongodb_foundation_probe());
    }
    probes
}

/// The probe of the MongoDB library.
#[cfg(feature = "mongodb")]
fn mongodb_foundation_probe() -> TokenStream {
    foundation_probe(
        "__tixschema_mongodb_foundation",
        "tixschema: this crate has `#[model_schema(enable = [\"decode_with\", \"mongodb\"])]` \
         types but no `tixschema::mongodb_foundation!();`",
        "this type needs the MongoDB support library, which \
         `tixschema::mongodb_foundation!();` writes once per crate",
        "Fix: add the line `tixschema::mongodb_foundation!();` at the crate root (src/lib.rs or \
         src/main.rs), on its own line, anywhere outside a function. Add it once per crate; \
         nothing else is needed.",
    )
}

/// A build without `mongodb` writes no MongoDB library, so nothing asks for it.
#[cfg(not(feature = "mongodb"))]
fn mongodb_foundation_probe() -> TokenStream {
    TokenStream::new()
}

/// One probe, in an anonymous const so it adds no name. With the library written, the method
/// `marker` names is the crate root's marker's, read through `use crate::*`, and answers `()`;
/// without it, the local fallback answers `Missing`, and `message` is the error.
fn foundation_probe(marker: &str, message: &str, label: &str, note: &str) -> TokenStream {
    let method = Ident::new(marker, Span::call_site());
    quote! {
        const _: () = {
            use crate::*;
            struct Missing;
            #[diagnostic::on_unimplemented(message = #message, label = #label, note = #note)]
            trait FoundationWritten {}
            impl FoundationWritten for () {}
            trait Fallback {
                fn #method(&self) -> Missing {
                    Missing
                }
            }
            impl Fallback for &() {}
            fn need<T: FoundationWritten>(_: T) {}
            fn probe() {
                need((&()).#method());
            }
            let _ = probe;
        };
    }
}

/// What `decode_with_foundation!()` writes at a crate root: the `decode_with` library in its
/// module, kept out of the consumer's documentation, and the marker its probe finds.
pub fn decode_with_foundation() -> TokenStream {
    let library = Ident::new(DECODE_WITH_LIBRARY, Span::call_site());
    let items = decode_with_library();
    quote! {
        #[doc(hidden)]
        mod #library {
            #items
        }

        trait __TixschemaDecodeWithFoundation {
            fn __tixschema_decode_with_foundation(&self) {}
        }

        impl __TixschemaDecodeWithFoundation for () {}
    }
}

/// What `mongodb_foundation!()` writes at a crate root: the MongoDB library in its module, which
/// reads the `decode_with` library's names and is kept out of the consumer's documentation, and
/// the marker its probe finds.
#[cfg(feature = "mongodb")]
pub fn mongodb_foundation() -> TokenStream {
    let (first, library) = (
        Ident::new(DECODE_WITH_LIBRARY, Span::call_site()),
        Ident::new(MONGODB_LIBRARY, Span::call_site()),
    );
    let items = mongodb_library();
    quote! {
        #[doc(hidden)]
        mod #library {
            use crate::#first::*;

            #items
        }

        trait __TixschemaMongodbFoundation {
            fn __tixschema_mongodb_foundation(&self) {}
        }

        impl __TixschemaMongodbFoundation for () {}
    }
}

/// A build without `mongodb` has no MongoDB library, so the call writes nothing and stays in a
/// crate whatever it builds with.
#[cfg(not(feature = "mongodb"))]
pub fn mongodb_foundation() -> TokenStream {
    TokenStream::new()
}

/// The `decode_with` library: what a callback and a resolver work with, what one type's walker
/// hands another's with, and under `bson` what reads a BSON value. Without `bson` it names no
/// `bson` path.
fn decode_with_library() -> TokenStream {
    let path = path_items();
    let callback = callback_items();
    let handoff = handoff_items();
    let taken = taken_items();
    let bound = bound_items();
    let bson = bson_library_items();
    let reads = trait_items();
    quote! {
        #reads
        #path
        #callback
        #handoff
        #taken
        #bound
        #bson
    }
}

/// `DecodeWith`, the trait every flagged type of the crate reads itself through.
fn trait_items() -> TokenStream {
    let bson = bson_trait_methods();
    quote! {
        /// What every flagged type of this crate reads itself through, so code generic over
        /// several of them names one bound.
        pub trait DecodeWith: ::core::marker::Sized {
            /// Reads `value` as this type, handing every issue found in it to `decide`, once.
            fn from_value_with<F>(
                value: serde_json::Value,
                decide: F,
            ) -> ::core::result::Result<Self, Unrecovered<serde_json::Value>>
            where
                F: ::core::ops::FnOnce(&mut serde_json::Value, &[Issue<serde_json::Value>]) -> Verdict;

            /// Reads `value` as this type, running `resolvers` in order over every issue found in
            /// it, once, and reading it again when every issue was settled.
            fn from_value_piped(
                value: serde_json::Value,
                resolvers: &[Resolver<'_, serde_json::Value, serde_json::Value>],
            ) -> ::core::result::Result<Self, Unrecovered<serde_json::Value>>;

            #bson
        }
    }
}

/// The BSON reads of `DecodeWith`.
#[cfg(feature = "bson")]
fn bson_trait_methods() -> TokenStream {
    quote! {
        /// Reads `document` as this type, handing every issue found in it to `decide`, once.
        fn from_bson_with<F>(
            document: bson::Document,
            decide: F,
        ) -> ::core::result::Result<Self, Unrecovered<bson::Bson>>
        where
            F: ::core::ops::FnOnce(&mut bson::Document, &[Issue<bson::Bson>]) -> Verdict;

        /// Reads `document` as this type, running `resolvers` in order over every issue found in
        /// it, once, and reading it again when every issue was settled.
        fn from_bson_piped(
            document: bson::Document,
            resolvers: &[Resolver<'_, bson::Document, bson::Bson>],
        ) -> ::core::result::Result<Self, Unrecovered<bson::Bson>>;
    }
}

/// A build without `bson` has no BSON reads.
#[cfg(not(feature = "bson"))]
fn bson_trait_methods() -> TokenStream {
    TokenStream::new()
}

/// What reads a BSON value: `bson_bound`, `bson_leaf` and its helpers, and `Stored` for a document.
#[cfg(feature = "bson")]
fn bson_library_items() -> TokenStream {
    let bound = bson_bound_items();
    let leaf = bson_leaf_items();
    let path = bson_stored_items();
    quote! {
        #bound
        #leaf

        #path
    }
}

/// A build without `bson` reads no BSON value.
#[cfg(not(feature = "bson"))]
fn bson_library_items() -> TokenStream {
    TokenStream::new()
}

/// The MongoDB library under `mongodb`: the query types and what the operations answer.
#[cfg(feature = "mongodb")]
fn mongodb_library() -> TokenStream {
    let query = query_items();
    let operation = operation_items();
    quote! {
        #query
        #operation
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
        pub enum Taken {
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

/// What brings the author's scope into a `{type}_schema` module written here: a struct of paths
/// names the types its author wrote as that author wrote them.
#[cfg(all(
    feature = "mongodb",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
fn authors_scope() -> TokenStream {
    quote! { use super::*; }
}

/// A build without `mongodb` writes no struct of paths, and its module names nothing of the
/// author's.
#[cfg(not(any(
    feature = "mongodb",
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema"
)))]
fn authors_scope() -> TokenStream {
    TokenStream::new()
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

/// `bounds` joined to every type parameter of `bounded` but `skipped`, each bound only where the
/// parameter does not ask it already. A parameter stays bounded in one place: its `where`
/// predicate if it has one, else its name.
fn bound_parameters(
    bounded: &mut Generics,
    bounds: &Punctuated<TypeParamBound, Token![+]>,
    skipped: Option<&Ident>,
) {
    let asked: Vec<(Ident, Vec<String>)> = bounded
        .type_params()
        .map(|parameter| (parameter.ident.clone(), asked_of(bounded, &parameter.ident)))
        .collect();
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
        if skipped.is_some_and(|left_out| *left_out == declared.ident) {
            continue;
        }
        let bounding = predicates.iter_mut().find(|bounding| {
            matches!(&bounding.bounded_ty, Type::Path(named) if named.path.is_ident(&declared.ident))
        });
        let already = asked
            .iter()
            .find(|(named, _)| *named == declared.ident)
            .map_or(&[][..], |(_, traits)| traits.as_slice());
        let more = bounds
            .iter()
            .filter(|bound| !asks_one_of(bound, already))
            .cloned();
        if let Some(in_where) = bounding {
            let in_place: Vec<String> = in_where.bounds.iter().filter_map(asked_trait).collect();
            let moved = take(&mut declared.bounds)
                .into_iter()
                .filter(|bound| !asks_one_of(bound, &in_place));
            in_where.bounds.extend(moved);
            in_where.bounds.extend(more);
        } else {
            declared.bounds.extend(more);
        }
    }
}

/// `generics` with every side's bounds joined to the type itself, which carries whatever more
/// serde's derive asks of one, and to every type parameter unless the container writes the side.
fn bounded_by(generics: &Generics, sides: &[Sides<'_>]) -> Generics {
    let mut bounded = generics.clone();
    let itself: Punctuated<TypeParamBound, Token![+]> = sides
        .iter()
        .flat_map(|(bounds, _)| bounds.iter().cloned())
        .collect();
    bounded
        .make_where_clause()
        .predicates
        .push(parse_quote!(Self: #itself));
    for (bounds, written) in sides {
        if let Some(predicates) = written {
            bounded
                .make_where_clause()
                .predicates
                .extend(predicates.iter().cloned());
        } else {
            bound_parameters(&mut bounded, bounds, None);
        }
    }
    bounded
}

/// `generics` with what serde's derive asks to read the type beside `Deserialize` of a parameter,
/// which a source's own bounds already ask: each of `predicates` that asks more, held at every
/// lifetime serde reads at.
fn read_under(
    generics: &Generics,
    predicates: &[WherePredicate],
    source_joined_read: bool,
) -> Generics {
    let asked_already: Vec<String> = if source_joined_read {
        generics
            .type_params()
            .map(|parameter| format!("{} : serde :: Deserialize < 'de >", parameter.ident))
            .collect()
    } else {
        Vec::new()
    };
    let mut bounded = generics.clone();
    for predicate in predicates {
        if asked_already.contains(&predicate.to_token_stream().to_string()) {
            continue;
        }
        let Some(mut at_every_read) = asking_more(&bounded, predicate) else {
            continue;
        };
        if let WherePredicate::Type(of_type) = &mut at_every_read
            && of_type.lifetimes.is_none()
            && of_type.to_token_stream().to_string().contains("'de")
        {
            of_type.lifetimes = Some(parse_quote!(for<'de>));
        }
        bounded.make_where_clause().predicates.push(at_every_read);
    }
    bounded
}

/// `value_bound`: what holds a JSON value serde read to the bound its field declares.
fn bound_items() -> TokenStream {
    quote! {
        /// What a field's bound refuses of one JSON value `read` reads: one issue whose reason is
        /// every violation `check` answers, joined by `; `, and none where the value keeps the
        /// bound or does not read.
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
            let violations = check(&read);
            if violations.is_empty() {
                return ::std::vec::Vec::new();
            }
            ::std::vec![issue("Invalid", path, expected, ::core::option::Option::Some(held.clone()), ::core::option::Option::Some(violations.join("; ")), ::std::vec::Vec::new())]
        }
    }
}

/// `bson_bound`: what holds a BSON value serde read to the bound its field declares.
#[cfg(feature = "bson")]
fn bson_bound_items() -> TokenStream {
    quote! {
        /// What a field's bound refuses of one BSON value `read` reads: one issue whose reason is
        /// every violation `check` answers, joined by `; `, and none where the value keeps the
        /// bound or does not read.
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
            let violations = check(&read);
            if violations.is_empty() {
                return ::std::vec::Vec::new();
            }
            ::std::vec![issue("Invalid", path, expected, ::core::option::Option::Some(held.clone()), ::core::option::Option::Some(violations.join("; ")), ::std::vec::Vec::new())]
        }
    }
}

/// The statement that binds `whole` to the document an entry point takes: the functions a type
/// names for itself are called once, on the document as it is taken.
#[cfg(feature = "bson")]
fn bson_document_taken(module: &Ident, corrects_own: bool) -> TokenStream {
    if corrects_own {
        quote! {
            let mut whole = match Self::decode_with_bson_corrected(bson::Bson::Document(document)) {
                ::core::result::Result::Ok(corrected) => corrected,
                ::core::result::Result::Err(reason) => {
                    return ::core::result::Result::Err(#module::Unrecovered {
                        issues: ::std::vec::Vec::from([#module::Issue::Undescribed { reason }]),
                    });
                }
            };
        }
    } else {
        quote! { let mut whole = bson::Bson::Document(document); }
    }
}

/// `from_bson_with`, `from_bson_piped` and the report they run. Every value is read through
/// `bson::Deserializer::new`, which both major versions of the `bson` library have, and which
/// takes what it reads by value: the document is held as the `bson::Bson` the walker borrows, and
/// copied once per read by serde. Both entry points open with the same read and close with the
/// same one. Under `mongodb` the operations that read a stored row through them sit beside them,
/// on a type that lists "mongodb".
#[cfg(feature = "bson")]
fn bson_entry_methods(
    module: &Ident,
    written: &[String],
    corrects_own: bool,
    mongodb: bool,
) -> Entry {
    let decider = unclaimed_parameter("F", written);
    let operations = mongodb.then(|| operation_methods(module, written));
    let (own_read, walked) = (
        own_read(corrects_own),
        own_walker(Source::Bson, corrects_own),
    );
    // The read lists what it finds, so nothing below it is judged as it is read.
    let read = quote! { Self::#own_read(bson::Deserializer::new(whole.clone())) };
    let taken = bson_document_taken(module, corrects_own);
    let attempt = quote! {
        match #read {
            ::core::result::Result::Ok(decoded) => {
                let mut found = Self::decode_with_bson_report(&whole, ::core::option::Option::None);
                found.extend(Self::decode_with_bson_check_issues(&whole, &decoded.decode_with_check_messages()));
                if found.is_empty() {
                    return ::core::result::Result::Ok(decoded);
                }
                found
            }
            ::core::result::Result::Err(refused) => Self::decode_with_bson_report(&whole, ::core::option::Option::Some(refused.to_string())),
        }
    };
    // A `u64` or an `i64` stored as the string of its digits, as a build of tixschema that wrote
    // them so stored it, is read as its integer before any issue is handed on.
    let first_read = quote! {
        #taken
        let mut settled = false;
        let found = loop {
            let found = #attempt;
            if settled {
                break found;
            }
            settled = match &mut whole {
                bson::Bson::Document(object) => #module::bson_digits_settled(object, &found),
                _ => false,
            };
            if !settled {
                break found;
            }
        };
        // `whole` is the document it was built from, so the other arm is never taken.
        let bson::Bson::Document(object) = &mut whole else {
            return ::core::result::Result::Err(#module::Unrecovered { issues: found });
        };
    };
    let second_read = quote! {
        let read = #read;
        let mut again = Self::decode_with_bson_report(&whole, read.as_ref().err().map(::std::string::ToString::to_string));
        if let ::core::result::Result::Ok(decoded) = &read {
            again.extend(Self::decode_with_bson_check_issues(&whole, &decoded.decode_with_check_messages()));
        }
        match read {
            ::core::result::Result::Ok(decoded) if again.is_empty() => ::core::result::Result::Ok(decoded),
            _ => ::core::result::Result::Err(#module::Unrecovered { issues: again }),
        }
    };
    let reads = quote! {
        fn from_bson_with<#decider>(
            document: bson::Document,
            decide: #decider,
        ) -> ::core::result::Result<Self, #module::Unrecovered<bson::Bson>>
        where
            #decider: ::core::ops::FnOnce(&mut bson::Document, &[#module::Issue<bson::Bson>]) -> #module::Verdict,
        {
            #first_read
            match decide(object, &found) {
                #module::Verdict::Reject => ::core::result::Result::Err(#module::Unrecovered { issues: found }),
                #module::Verdict::Fixed => {
                    #second_read
                }
            }
        }

        fn from_bson_piped(
            document: bson::Document,
            resolvers: &[#module::Resolver<'_, bson::Document, bson::Bson>],
        ) -> ::core::result::Result<Self, #module::Unrecovered<bson::Bson>> {
            #first_read
            let left = #module::unsettled(object, &found, resolvers);
            if !left.is_empty() {
                return ::core::result::Result::Err(#module::Unrecovered { issues: left });
            }
            #second_read
        }
    };
    let helpers = quote! {
        fn decode_with_bson_report(whole: &bson::Bson, refused: ::core::option::Option<::std::string::String>) -> ::std::vec::Vec<#module::Issue<bson::Bson>> {
            let mut out = ::std::vec::Vec::new();
            Self::#walked(whole, &[], #module::issue_from_parts, &mut out);
            if let ::core::option::Option::Some(reason) = refused
                && out.iter().all(|found| matches!(found, #module::Issue::Unknown { .. } | #module::Issue::Mistyped { .. }))
            {
                out.push(#module::Issue::Undescribed { reason });
            }
            out
        }

        #operations
    };
    Entry { helpers, reads }
}

/// `bson_leaf`, the bracket rule it decides `Mistyped` by, `bson_digits_settled` and
/// `bson_remaining`.
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

        /// `found`, a value of `expected`, with each `u64` and `i64` it holds as the string of its
        /// decimal digits stored as the integer, or `None` where such a string holds no value of
        /// its type, or one BSON stores no integer of.
        fn bson_digits_as_numbers(expected: &Expected, found: &bson::Bson) -> ::core::option::Option<bson::Bson> {
            match (expected, found) {
                (Expected::I64, bson::Bson::String(digits)) => digits.parse::<i64>().ok().map(bson::Bson::Int64),
                (Expected::U64, bson::Bson::String(digits)) => digits
                    .parse::<u64>()
                    .ok()
                    .and_then(|whole| <i64 as ::core::convert::TryFrom<u64>>::try_from(whole).ok())
                    .map(bson::Bson::Int64),
                (Expected::Optional(held), _) => bson_digits_as_numbers(held, found),
                (Expected::Array(held), bson::Bson::Array(items)) => items
                    .iter()
                    .map(|item| bson_digits_as_numbers(held, item))
                    .collect::<::core::option::Option<::std::vec::Vec<bson::Bson>>>()
                    .map(bson::Bson::Array),
                (Expected::Tuple(held), bson::Bson::Array(items)) if held.len() == items.len() => held
                    .iter()
                    .zip(items)
                    .map(|(position, item)| bson_digits_as_numbers(position, item))
                    .collect::<::core::option::Option<::std::vec::Vec<bson::Bson>>>()
                    .map(bson::Bson::Array),
                (Expected::Map(held) | Expected::Range(_, held), bson::Bson::Document(entries)) => entries
                    .iter()
                    .map(|(key, item)| bson_digits_as_numbers(held, item).map(|stored| (key.clone(), stored)))
                    .collect::<::core::option::Option<bson::Document>>()
                    .map(bson::Bson::Document),
                _ => ::core::option::Option::Some(found.clone()),
            }
        }

        /// Writes in `root`, in place of the value of each `Invalid` issue in `issues`, that value
        /// with each `u64` and `i64` it holds as the string of its decimal digits stored as the
        /// integer, and answers whether it wrote one. A build of tixschema that wrote the two as
        /// their digits stored such a value.
        pub fn bson_digits_settled(root: &mut bson::Document, issues: &[Issue<bson::Bson>]) -> bool {
            let mut wrote = false;
            for issue in issues {
                if let Issue::Invalid { path, expected, found, .. } = issue {
                    match bson_digits_as_numbers(expected, found) {
                        ::core::option::Option::Some(stored) if stored != *found => {
                            wrote |= Stored::set_at(root, path, stored);
                        }
                        _ => {}
                    }
                }
            }
            wrote
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

/// `Stored` for a BSON document.
#[cfg(feature = "bson")]
fn bson_stored_items() -> TokenStream {
    quote! {
        impl Stored for bson::Document {
            type Value = bson::Bson;

            fn set_at(&mut self, path: &Path, value: bson::Bson) -> bool {
                let ::core::option::Option::Some((Segment::Key(first), rest)) = path.0.split_first() else { return false };
                let ::core::option::Option::Some((last, parents)) = rest.split_last() else {
                    self.insert(first.clone(), value);
                    return true;
                };
                let ::core::option::Option::Some(mut slot) = self.get_mut(first) else { return false };
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

            fn remove_at(&mut self, path: &Path) -> bool {
                let ::core::option::Option::Some((Segment::Key(first), rest)) = path.0.split_first() else { return false };
                let ::core::option::Option::Some((last, parents)) = rest.split_last() else {
                    return self.remove(first).is_some();
                };
                let ::core::option::Option::Some(mut slot) = self.get_mut(first) else { return false };
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

            fn at(&self, path: &Path) -> ::core::option::Option<&bson::Bson> {
                let ::core::option::Option::Some((Segment::Key(first), rest)) = path.0.split_first() else {
                    return ::core::option::Option::None;
                };
                let mut slot = self.get(first)?;
                for segment in rest {
                    slot = match (segment, slot) {
                        (Segment::Key(key), bson::Bson::Document(object)) => object.get(key)?,
                        (Segment::Index(index), bson::Bson::Array(items)) => items.get(*index)?,
                        _ => return ::core::option::Option::None,
                    };
                }
                ::core::option::Option::Some(slot)
            }

            fn text(value: &bson::Bson) -> ::core::option::Option<&str> {
                value.as_str()
            }

            fn whole(value: &bson::Bson) -> ::core::option::Option<i64> {
                match value {
                    bson::Bson::Int32(number) => ::core::option::Option::Some(i64::from(*number)),
                    bson::Bson::Int64(number) => ::core::option::Option::Some(*number),
                    _ => ::core::option::Option::None,
                }
            }

            fn real(value: &bson::Bson) -> ::core::option::Option<f64> {
                match value {
                    bson::Bson::Double(number) => ::core::option::Option::Some(*number),
                    _ => ::core::option::Option::None,
                }
            }

            fn from_text(text: ::std::string::String) -> bson::Bson {
                bson::Bson::String(text)
            }

            fn from_whole(number: i64) -> bson::Bson {
                bson::Bson::Int64(number)
            }

            fn from_real(number: f64) -> bson::Bson {
                bson::Bson::Double(number)
            }
        }
    }
}

/// The types a callback works with, and beside its verdict what a resolver answers with.
fn callback_items() -> TokenStream {
    let resolvers = resolver_items();
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
            /// A std range, by the name it is declared under in `core::ops`, and the type of its
            /// bounds.
            Range(&'static str, ::std::boxed::Box<Expected>),
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
            /// The key is there, and either serde refuses what it holds for its field, or serde reads it
            /// and it breaks a bound the field declares, or a check refuses it.
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

        #resolvers

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

/// The read of the type itself its recovering read lists from: the read of a payload its
/// functions have corrected where it `corrects_own`, and its unjudged read where it names none.
fn own_read(corrects_own: bool) -> Ident {
    if corrects_own {
        format_ident!("decode_with_read_corrected")
    } else {
        format_ident!("decode_with_read_unjudged")
    }
}

/// The method that lists the issues of a value the entry points of `source` hold: the walker of
/// a corrected value where the type `corrects_own`, the entry point having called its functions.
fn own_walker(source: Source, corrects_own: bool) -> Ident {
    if corrects_own {
        source.method("corrected_issues")
    } else {
        source.method("issues")
    }
}

/// `from_value_with`, `from_value_piped` and the report they run. The report is told what serde
/// said of the value, so each decode reads the value with serde once. Both entry points open
/// with the same read and close with the same one.
fn entry_methods(module: &Ident, decider: &Ident, corrects_own: bool) -> Entry {
    let (own_read, walked) = (
        own_read(corrects_own),
        own_walker(Source::Json, corrects_own),
    );
    // The read lists what it finds, so nothing below it is judged as it is read. A check takes
    // the value serde built, so each read serde accepts is followed by the checks.
    let read = quote! { Self::#own_read(&value) };
    // The functions a type names for itself are called once, on the value as it is taken.
    let taken = corrects_own.then(|| quote! { value = Self::decode_with_value_corrected(value); });
    let attempt = quote! {
        match #read {
            ::core::result::Result::Ok(decoded) => {
                let mut found = Self::decode_with_value_report(&value, ::core::option::Option::None);
                found.extend(Self::decode_with_value_check_issues(&value, &decoded.decode_with_check_messages()));
                if found.is_empty() {
                    return ::core::result::Result::Ok(decoded);
                }
                found
            }
            ::core::result::Result::Err(refused) => Self::decode_with_value_report(&value, ::core::option::Option::Some(refused.to_string())),
        }
    };
    // A `u64` or an `i64` written as the string of its digits, as a build of tixschema that wrote
    // them so wrote it, is read as its number before any issue is handed on.
    let first_read = quote! {
        #taken
        let mut settled = false;
        let found = loop {
            let found = #attempt;
            if settled || !#module::value_digits_settled(&mut value, &found) {
                break found;
            }
            settled = true;
        };
    };
    let second_read = quote! {
        let read = #read;
        let mut again = Self::decode_with_value_report(&value, read.as_ref().err().map(::std::string::ToString::to_string));
        if let ::core::result::Result::Ok(decoded) = &read {
            again.extend(Self::decode_with_value_check_issues(&value, &decoded.decode_with_check_messages()));
        }
        match read {
            ::core::result::Result::Ok(decoded) if again.is_empty() => ::core::result::Result::Ok(decoded),
            _ => ::core::result::Result::Err(#module::Unrecovered { issues: again }),
        }
    };
    let reads = quote! {
        fn from_value_with<#decider>(
            mut value: serde_json::Value,
            decide: #decider,
        ) -> ::core::result::Result<Self, #module::Unrecovered<serde_json::Value>>
        where
            #decider: ::core::ops::FnOnce(&mut serde_json::Value, &[#module::Issue<serde_json::Value>]) -> #module::Verdict,
        {
            #first_read
            match decide(&mut value, &found) {
                #module::Verdict::Reject => ::core::result::Result::Err(#module::Unrecovered { issues: found }),
                #module::Verdict::Fixed => {
                    #second_read
                }
            }
        }

        fn from_value_piped(
            mut value: serde_json::Value,
            resolvers: &[#module::Resolver<'_, serde_json::Value, serde_json::Value>],
        ) -> ::core::result::Result<Self, #module::Unrecovered<serde_json::Value>> {
            #first_read
            let left = #module::unsettled(&mut value, &found, resolvers);
            if !left.is_empty() {
                return ::core::result::Result::Err(#module::Unrecovered { issues: left });
            }
            #second_read
        }
    };
    let helpers = quote! {
        fn decode_with_value_report(value: &serde_json::Value, refused: ::core::option::Option<::std::string::String>) -> ::std::vec::Vec<#module::Issue<serde_json::Value>> {
            let mut out = ::std::vec::Vec::new();
            Self::#walked(value, &[], #module::issue_from_parts, &mut out);
            if let ::core::option::Option::Some(reason) = refused
                && out.iter().all(|found| matches!(found, #module::Issue::Unknown { .. } | #module::Issue::Mistyped { .. }))
            {
                out.push(#module::Issue::Undescribed { reason });
            }
            out
        }
    };
    Entry { helpers, reads }
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
/// tixschema::decode_with_foundation!();
///
/// #[model_schema(enable = ["decode_with"])]
/// #[derive(Deserialize, Serialize)]
/// pub struct Audit {
///     pub revision: i32,
/// }
///
/// #[model_schema(enable = ["decode_with"])]
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
/// tixschema::decode_with_foundation!();
///
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// pub struct Audit {
///     pub revision: i32,
/// }
///
/// #[model_schema(enable = ["decode_with"])]
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
///   --> tests/zz_probe.rs:12:1
///    |
///  8 | pub struct Audit {
///    | ---------------- associated function or constant `decode_with_value_named` not found for this struct
/// ...
/// 12 | #[model_schema(enable = ["decode_with"])]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ associated function or constant not found in `Audit`
///    |
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error[E0599]: no associated function or constant named `decode_with_value_fields` found for struct `Audit` in the current scope
///   --> tests/zz_probe.rs:12:1
///    |
///  8 | pub struct Audit {
///    | ---------------- associated function or constant `decode_with_value_fields` not found for this struct
/// ...
/// 12 | #[model_schema(enable = ["decode_with"])]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ associated function or constant not found in `Audit`
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
    let leaf = leaf_items();
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
                "Range" => Expected::Range(name, first(&mut under)),
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

        #leaf
    }
}

/// `value_leaf`, the check of a JSON value read whole, and `value_digits_settled`.
fn leaf_items() -> TokenStream {
    quote! {
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

        /// `found`, a value of `expected`, with each `u64` and `i64` it holds as the string of its
        /// decimal digits written as the number, or `None` where such a string holds no value of
        /// its type.
        fn value_digits_as_numbers(expected: &Expected, found: &serde_json::Value) -> ::core::option::Option<serde_json::Value> {
            match (expected, found) {
                (Expected::I64, serde_json::Value::String(digits)) => digits.parse::<i64>().ok().map(serde_json::Value::from),
                (Expected::U64, serde_json::Value::String(digits)) => digits.parse::<u64>().ok().map(serde_json::Value::from),
                (Expected::Optional(held), _) => value_digits_as_numbers(held, found),
                (Expected::Array(held), serde_json::Value::Array(items)) => items
                    .iter()
                    .map(|item| value_digits_as_numbers(held, item))
                    .collect::<::core::option::Option<::std::vec::Vec<serde_json::Value>>>()
                    .map(serde_json::Value::Array),
                (Expected::Tuple(held), serde_json::Value::Array(items)) if held.len() == items.len() => held
                    .iter()
                    .zip(items)
                    .map(|(position, item)| value_digits_as_numbers(position, item))
                    .collect::<::core::option::Option<::std::vec::Vec<serde_json::Value>>>()
                    .map(serde_json::Value::Array),
                (Expected::Map(held) | Expected::Range(_, held), serde_json::Value::Object(entries)) => entries
                    .iter()
                    .map(|(key, item)| value_digits_as_numbers(held, item).map(|written| (key.clone(), written)))
                    .collect::<::core::option::Option<serde_json::Map<::std::string::String, serde_json::Value>>>()
                    .map(serde_json::Value::Object),
                _ => ::core::option::Option::Some(found.clone()),
            }
        }

        /// Writes in `root`, in place of the value of each `Invalid` issue in `issues`, that value
        /// with each `u64` and `i64` it holds as the string of its decimal digits written as the
        /// number, and answers whether it wrote one. A build of tixschema that wrote the two as
        /// their digits wrote such a value.
        pub fn value_digits_settled(root: &mut serde_json::Value, issues: &[Issue<serde_json::Value>]) -> bool {
            let mut wrote = false;
            for issue in issues {
                if let Issue::Invalid { path, expected, found, .. } = issue {
                    match value_digits_as_numbers(expected, found) {
                        ::core::option::Option::Some(written) if written != *found => {
                            wrote |= Stored::set_at(root, path, written);
                        }
                        _ => {}
                    }
                }
            }
            wrote
        }
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
fn held_walk<'item>(
    slot: &'item Field,
    module_name: &str,
    parameters: &[String],
    corrected: &[FunctionName],
) -> Walk<'item> {
    let hooks = parse_serde_field_hooks(&slot.attrs);
    // A slot that names a `preprocess` is read through a hook of the type's own reader as well.
    if corrected.is_empty() && hooked_leaf(&slot.ty, &hooks, module_name, parameters).is_none() {
        return walk_of(&slot.ty, parameters);
    }
    Walk {
        step: Step::Leaf(Whole {
            corrected: false,
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
        corrected: false,
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

/// How the value of a field or a slot is walked: through its author's hooks, or by its type. A
/// member that names a `preprocess` is one value, read from the payload the functions `corrected`
/// name answer for what is stored, through the hook it carries or with its own type's reader.
fn member_walk<'item>(
    member: &'item Field,
    module_name: &str,
    parameters: &[String],
    corrected: &[FunctionName],
) -> Walk<'item> {
    let hooks = parse_serde_field_hooks(&member.attrs);
    let hooked = hooked_leaf(&member.ty, &hooks, module_name, parameters);
    if corrected.is_empty() {
        return hooked.map_or_else(
            || walk_of(&member.ty, parameters),
            |step| Walk {
                step,
                ty: &member.ty,
            },
        );
    }
    let through = match hooked {
        Some(Step::Leaf(whole)) => whole.read,
        Some(
            Step::Arms(_, _)
            | Step::Entries(_)
            | Step::Items(_)
            | Step::Model
            | Step::Positions(_)
            | Step::Present(_),
        )
        | None => own_reader(&member.ty),
    };
    Walk {
        step: Step::Leaf(Whole {
            corrected: true,
            hooked: true,
            parameterized: names_a_parameter(&member.ty, parameters),
            read: preprocess::corrected_reader(corrected, &through),
            write: None,
        }),
        ty: &member.ty,
    }
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
/// tixschema::decode_with_foundation!();
///
/// #[model_schema(enable = ["decode_with"])]
/// #[derive(Deserialize, Serialize)]
/// pub struct Version {
///     pub number: i32,
/// }
///
/// #[model_schema(enable = ["decode_with"])]
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
/// tixschema::decode_with_foundation!();
///
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// pub struct Version {
///     pub number: i32,
/// }
///
/// #[model_schema(enable = ["decode_with"])]
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
///   --> tests/zz_probe.rs:12:1
///    |
///  8 | pub struct Version {
///    | ------------------ associated function or constant `decode_with_value_issues` not found for this struct
/// ...
/// 12 | #[model_schema(enable = ["decode_with"])]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ associated function or constant not found in `Version`
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
/// tixschema::decode_with_foundation!();
///
/// #[model_schema(enable = ["decode_with"])]
/// #[derive(Deserialize, Serialize)]
/// pub struct Version {
///     pub number: i32,
/// }
///
/// #[model_schema()]
/// pub type Versions = Vec<Version>;
///
/// #[model_schema(enable = ["decode_with"])]
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

/// `OperationError`, which every operation on the type fails with, and `Read`, which its reads
/// answer.
#[cfg(feature = "mongodb")]
fn operation_items() -> TokenStream {
    let mut items = operations::error_items();
    items.extend(operations::read_items());
    items
}

/// The operations on the type. Each takes its filter and its update as a type parameter of its
/// own, under a name `written`, every name the type's item writes, does not hold.
#[cfg(feature = "mongodb")]
fn operation_methods(module: &Ident, written: &[String]) -> TokenStream {
    operations::methods(
        module,
        &unclaimed_parameter("F", written),
        &unclaimed_parameter("U", written),
    )
}

/// A build without `mongodb` gives a type no operation.
#[cfg(all(feature = "bson", not(feature = "mongodb")))]
fn operation_methods(_module: &Ident, _written: &[String]) -> TokenStream {
    TokenStream::new()
}

fn own_reader(ty: &Type) -> TokenStream {
    quote! { <#ty as serde::Deserialize>::deserialize }
}

/// The type a walked list is read whole as, to be held to its field's bound: the list's own, and
/// a boxed slice for a slice, which no reader hands back unsized.
fn read_as_a_list(ty: &Type) -> Type {
    if matches!(ty, Type::Slice(_)) {
        parse_quote! { ::std::boxed::Box<#ty> }
    } else {
        ty.clone()
    }
}

/// `items` as they are emitted: beside what a schema surface writes into `{type}_schema`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn placed_in_schema_module(_module: &Ident, items: &TokenStream) -> TokenStream {
    items.clone()
}

/// `items` in a `{type}_schema` module written here, no schema surface writing one in this build.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
fn placed_in_schema_module(module: &Ident, items: &TokenStream) -> TokenStream {
    let scope = authors_scope();
    quote! {
        pub mod #module {
            #scope
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

/// The `Stored` trait.
fn stored_trait_items() -> TokenStream {
    quote! {
        /// A value a resolver repairs at an issue's path: a JSON value, or a BSON document.
        pub trait Stored {
            /// What one place inside it holds.
            type Value;
            /// Puts `value` at `path`, inserting the last key when it is absent; whether anything changed.
            fn set_at(&mut self, path: &Path, value: Self::Value) -> bool;
            /// Takes out what is at `path`; whether anything changed.
            fn remove_at(&mut self, path: &Path) -> bool;
            /// What is at `path`, if anything.
            fn at(&self, path: &Path) -> ::core::option::Option<&Self::Value>;
            /// The text `value` holds.
            fn text(value: &Self::Value) -> ::core::option::Option<&str>;
            /// The whole number `value` holds.
            fn whole(value: &Self::Value) -> ::core::option::Option<i64>;
            /// The real number `value` holds.
            fn real(value: &Self::Value) -> ::core::option::Option<f64>;
            /// The value that holds `text`.
            fn from_text(text: ::std::string::String) -> Self::Value;
            /// The value that holds `number`.
            fn from_whole(number: i64) -> Self::Value;
            /// The value that holds `number`.
            fn from_real(number: f64) -> Self::Value;
        }
    }
}

/// `Stored` for a JSON value.
fn stored_json_items() -> TokenStream {
    quote! {
        impl Stored for serde_json::Value {
            type Value = serde_json::Value;

            fn set_at(&mut self, path: &Path, value: serde_json::Value) -> bool {
                let ::core::option::Option::Some((last, parents)) = path.0.split_last() else {
                    *self = value;
                    return true;
                };
                let mut slot = self;
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

            fn remove_at(&mut self, path: &Path) -> bool {
                let ::core::option::Option::Some((last, parents)) = path.0.split_last() else { return false };
                let mut slot = self;
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

            fn at(&self, path: &Path) -> ::core::option::Option<&serde_json::Value> {
                let mut slot = self;
                for segment in &path.0 {
                    slot = match (segment, slot) {
                        (Segment::Key(key), serde_json::Value::Object(object)) => object.get(key)?,
                        (Segment::Index(index), serde_json::Value::Array(items)) => items.get(*index)?,
                        _ => return ::core::option::Option::None,
                    };
                }
                ::core::option::Option::Some(slot)
            }

            fn text(value: &serde_json::Value) -> ::core::option::Option<&str> {
                value.as_str()
            }

            fn whole(value: &serde_json::Value) -> ::core::option::Option<i64> {
                value.as_i64()
            }

            fn real(value: &serde_json::Value) -> ::core::option::Option<f64> {
                value.as_f64()
            }

            fn from_text(text: ::std::string::String) -> serde_json::Value {
                serde_json::Value::String(text)
            }

            fn from_whole(number: i64) -> serde_json::Value {
                serde_json::Value::from(number)
            }

            fn from_real(number: f64) -> serde_json::Value {
                serde_json::Value::from(number)
            }
        }
    }
}

/// `Segment` and `Path`, with the `Stored` trait and its JSON impl.
fn path_items() -> TokenStream {
    let stored_trait = stored_trait_items();
    let stored_json = stored_json_items();
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

        #stored_trait

        #stored_json

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
    if is_map_wrapper(&name)
        && let Some(inner) = second
    {
        return held_in(Step::Entries, walk_of(inner, parameters), ty, parameters);
    }
    // serde writes a `Result` as the object whose one key names its arm, which is walked.
    if let Some((ok_type, err_type)) = written_std_result(type_path) {
        let (ok, err) = (walk_of(ok_type, parameters), walk_of(err_type, parameters));
        return if ok.is_of_a_parameter() || err.is_of_a_parameter() {
            plain_value(ty, parameters)
        } else {
            Walk {
                step: Step::Arms(Box::new(ok), Box::new(err)),
                ty,
            }
        };
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
            corrected: false,
            hooked: false,
            parameterized: names_a_parameter(ty, parameters),
            read: own_reader(ty),
            write: None,
        }),
        ty,
    }
}

/// `Filter`, `Update` and the typed paths that build them.
#[cfg(feature = "mongodb")]
fn query_items() -> TokenStream {
    query::query_items()
}

/// Whether serde reads a value walked this way from the entries of an object it is flattened in.
/// It refuses every other value there, and reads an `Option` of one as absent.
fn reads_entries(walk: &Walk<'_>) -> bool {
    match &walk.step {
        Step::Entries(_) | Step::Model => true,
        Step::Arms(_, _) | Step::Items(_) | Step::Positions(_) => false,
        Step::Leaf(whole) => whole.hooked || whole.parameterized || holds_an_id(walk.ty),
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

/// `Resolution`, `Resolver`, and `unsettled`, the pipe that runs resolvers over a read's issues.
fn resolver_items() -> TokenStream {
    quote! {
        /// What a resolver answers for one issue.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum Resolution {
            /// Repaired; no later resolver sees it.
            Settled,
            /// Known, and not to be repaired: the read fails; no later resolver sees it.
            Rejected,
            /// Not this resolver's; the next one sees it.
            NotTouched,
        }

        /// A resolver over a raw value `D` whose issues hold values `V`. It is `Sync`, so that a
        /// read which waits for a row can hold its resolvers and still move to another thread.
        pub type Resolver<'r, D, V> = &'r (dyn ::core::ops::Fn(&mut D, &Issue<V>) -> Resolution + ::core::marker::Sync);

        /// Runs `resolvers` in order, each seeing only the issues every resolver before it left
        /// `NotTouched`, and answers the issues none settled, in the order they were found.
        pub fn unsettled<D, V: ::core::clone::Clone>(
            raw: &mut D,
            issues: &[Issue<V>],
            resolvers: &[Resolver<'_, D, V>],
        ) -> ::std::vec::Vec<Issue<V>> {
            // `None` while every resolver so far answered `NotTouched`.
            let mut answers: ::std::vec::Vec<::core::option::Option<Resolution>> = ::std::vec![::core::option::Option::None; issues.len()];
            for resolve in resolvers {
                for (issue, answer) in issues.iter().zip(answers.iter_mut()) {
                    if answer.is_none() {
                        match resolve(raw, issue) {
                            Resolution::NotTouched => {}
                            decided => *answer = ::core::option::Option::Some(decided),
                        }
                    }
                }
            }
            issues
                .iter()
                .zip(answers)
                .filter(|(_issue, answer)| *answer != ::core::option::Option::Some(Resolution::Settled))
                .map(|(issue, _answer)| issue.clone())
                .collect()
        }
    }
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

/// The typed MongoDB paths of a struct: `MongoFields` for its module, and for its `impl` the const
/// and the function that build it.
#[cfg(feature = "mongodb")]
fn struct_paths(item_struct: &ItemStruct, written: &[String]) -> TypedPaths {
    fields::struct_paths(item_struct, written)
}

/// A build without `mongodb` writes no typed path.
#[cfg(not(feature = "mongodb"))]
fn struct_paths(_item_struct: &ItemStruct, _written: &[String]) -> TypedPaths {
    TypedPaths::default()
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
        pub fn taken_keys<'asked, 'key, T, R, K>(read: R, asked: &'asked Asked, mut keys: K) -> ::std::vec::Vec<&'key str>
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

/// The `impl`s holding `methods`, which lists what each of the type's `sources` adds and the
/// `DecodeWith` reads it adds, and `paths`, the type's typed paths. A type with a type parameter
/// gets one inherent `impl` per source, under the bounds that source reads and writes a value
/// with, one `impl` of `DecodeWith` under the last source's bounds, which the others' are among,
/// and one for its paths, under the bound a path writes a value with. Every other type gets one
/// of each for them all.
fn type_impls(
    name: &Ident,
    generics: &Generics,
    written: &Written<'_>,
    module: &Ident,
    (sources, methods): (&[Source], &[(TokenStream, TokenStream)]),
    paths: &TokenStream,
) -> TokenStream {
    if type_parameters_in_scope(generics).is_empty() {
        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
        let inherent = methods.iter().map(|(inherent, _)| inherent);
        let reads = methods.iter().map(|(_, reads)| reads);
        return quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #(#inherent)*
                #paths
            }

            impl #impl_generics #module::DecodeWith for #name #type_generics #where_clause {
                #(#reads)*
            }
        };
    }
    let bounded_by_source = |source: &Source| {
        read_under(
            &source.bounded(generics, &written.bound),
            &written.read_predicates,
            !written.bound.read_written,
        )
    };
    let impls = sources.iter().zip(methods).map(|(source, (inherent, _))| {
        let bounded = bounded_by_source(source);
        let (impl_generics, type_generics, where_clause) = bounded.split_for_impl();
        quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #inherent
            }
        }
    });
    let reads = methods.iter().map(|(_, reads)| reads);
    let of_reads = sources.last().map(|last| {
        let bounded = bounded_by_source(last);
        let (impl_generics, type_generics, where_clause) = bounded.split_for_impl();
        quote! {
            impl #impl_generics #module::DecodeWith for #name #type_generics #where_clause {
                #(#reads)*
            }
        }
    });
    let of_paths = (!paths.is_empty()).then(|| {
        let bounded = bounded_by(
            generics,
            &[(
                parse_quote!(::serde::Serialize),
                written.bound.write.as_deref(),
            )],
        );
        let (impl_generics, type_generics, where_clause) = bounded.split_for_impl();
        quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #paths
            }
        }
    });
    quote! {
        #(#impls)*
        #of_reads
        #of_paths
    }
}

/// `base`, numbered where `taken` already holds it.
fn unclaimed(base: &str, taken: &[String]) -> String {
    let mut name = base.to_owned();
    let mut number = 1_u32;
    while taken.contains(&name) {
        number = number.saturating_add(1);
        name = format!("{base}{number}");
    }
    name
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
                validator: None,
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
    rename_all: Casing<'_>,
    container_defaulted: bool,
    module_name: &str,
    parameters: &[String],
    variant: Option<&str>,
    corrected: &[FunctionName],
) -> Option<WalkedField<'item>> {
    let ident = field.ident.as_ref()?;
    if is_off_the_wire(field) {
        return None;
    }
    let omission = parse_serde_key_omission(&field.attrs);
    let meta = parse_serde_field_attributes(&field.attrs);
    // The key of a field serde never reads is the one serde writes it under.
    let (renamed, rule) = if omission.skips_deserializing {
        (
            parse_written_renames(&field.attrs).rename,
            rename_all.written,
        )
    } else {
        (parse_read_renames(&field.attrs).rename, rename_all.read)
    };
    let key = renamed
        .unwrap_or_else(|| resolve_rename_rule(rule).apply_to_field(&ident.unraw().to_string()));
    let walk = (!omission.skips_deserializing)
        .then(|| member_walk(field, module_name, parameters, corrected));
    // With a read hook on the field, serde's derive no longer reads a missing key as `None`. The
    // hook a `preprocess` is called from is hung with the `default` that reads it so.
    let optional = !has_serde_read_hook(&field.attrs)
        && get_field_def("", &field.ty, "").stands_for_an_option();
    Some(WalkedField {
        absence_is_read: omission.defaulted || container_defaulted || optional,
        aliases: meta.aliases,
        key,
        ty: &field.ty,
        validator: recorded_validator(module_name, &ident.unraw().to_string(), variant),
        walk,
    })
}

/// The fields serde writes or reads among `named`: the ones read under a key of their own with
/// every such key, and the flattened ones apart.
fn walked_fields<'item>(
    named: &'item FieldsNamed,
    rename_all: Casing<'_>,
    container_defaulted: bool,
    module_name: &str,
    parameters: &[String],
    variant: Option<&str>,
) -> Keyed<'item> {
    let mut fields: Vec<WalkedField<'item>> = Vec::new();
    let mut flattened: Vec<Flattened<'item>> = Vec::new();
    let named_on = preprocess::named();
    for (index, field) in named.named.iter().enumerate() {
        let corrected = named_on.of_member(variant, index);
        if !parse_serde_field_attributes(&field.attrs).flatten {
            fields.extend(walked_field(
                field,
                rename_all,
                container_defaulted,
                module_name,
                parameters,
                variant,
                corrected,
            ));
        } else if is_off_the_wire(field) {
            // serde neither writes nor reads the field, so no key in the object is its own.
        } else {
            flattened.push(Flattened::of(field, module_name, parameters, corrected));
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

/// One slot of a tuple as the walker reads it, held to the bound `validator` checks, or `None`
/// for a slot serde does not read: the array holds no position for it.
fn walked_slot<'item>(
    slot: &'item Field,
    container_defaulted: bool,
    module_name: &str,
    parameters: &[String],
    validator: Option<FieldValidator>,
    corrected: &[FunctionName],
) -> Option<Slot<'item>> {
    let omission = parse_serde_key_omission(&slot.attrs);
    (!omission.skips_deserializing).then(|| Slot {
        absence_is_read: omission.defaulted || container_defaulted,
        ty: &slot.ty,
        validator,
        walk: member_walk(slot, module_name, parameters, corrected),
    })
}

/// The validator recorded for the bound the member `declared_as` declares, where it declares
/// one, with the name the validator writes.
fn recorded_validator(
    module_name: &str,
    declared_as: &str,
    variant: Option<&str>,
) -> Option<FieldValidator> {
    let stem = helper_name_stem(declared_as, variant);
    holds_field_validator(module_name, &stem).then(|| FieldValidator {
        items: field_validator_judges_items(module_name, &stem),
        named: field_validator_name(module_name, &stem),
        validator: format_ident!("validate_{stem}_value"),
    })
}

/// [`recorded_validator`] for slot `index` of a tuple struct, or of the tuple variant `variant`,
/// named by its position.
fn slot_validator(
    module_name: &str,
    variant: Option<&str>,
    index: usize,
) -> Option<FieldValidator> {
    recorded_validator(module_name, &index.to_string(), variant)
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
        Some(Declared::Model(..) | Declared::OneValue(..)) => true,
        Some(Declared::Alias(aliased_as)) if depth < 8 => syn::parse_str::<Type>(&aliased_as)
            .is_ok_and(|aliased| walks_itself(&aliased, depth.saturating_add(1))),
        Some(Declared::Alias(_)) | None => false,
    }
}

#[cfg(test)]
mod tests;
