//! Expands `#[model_schema]`: routes a struct, an enum, a type alias or a branded newtype to its
//! expansion, and generates the `ts_definition()`, `zod_schema()`, `json_schema()`,
//! `schema_example()` and `validate()` each one publishes.

extern crate alloc;

pub mod checks;
pub mod judged_read;
pub mod preprocess;
pub mod ranges;

use alloc::borrow::ToOwned;
use core::fmt::Write as _;
use core::iter;
#[cfg(all(feature = "zod", feature = "typescript"))]
use core::ops::Range;
use core::ptr;

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt as _;
use syn::parse::{Parse, ParseStream, Parser as _};
use syn::punctuated::Punctuated;
use syn::{Field, Item, ItemType, Meta, Token};

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use quote::quote_spanned;

#[cfg(any(
    feature = "serde",
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema"
))]
use syn::spanned::Spanned as _;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use syn::Ident;

#[cfg(all(feature = "zod", feature = "typescript"))]
use crate::field_type::ZOD_SCHEMA_BOUND;
#[cfg(any(feature = "serde", feature = "jsonschema"))]
use crate::field_type::{RESULT_ARMS, RangeKind};
#[cfg(feature = "zod")]
use crate::field_type::{zod_check_wrap, zod_check_wrap_handing, zod_preprocess_wrap};
use crate::{
    field_type::{
        BoundKind, FieldDef, FieldDefType, VariantKind, classify_variant, format_number_literal,
        get_field_def, is_plain_enum,
    },
    utils::{get_field_docs, get_variant_docs, strip_examples_from_docs},
};

#[cfg(any(feature = "zod", feature = "jsonschema"))]
use crate::field_type::is_undescribable_primitive;

#[cfg(any(feature = "serde", feature = "zod"))]
use crate::bound_message::Bound;
#[cfg(feature = "serde")]
use crate::bound_message::VIOLATION_STEMS;
#[cfg(feature = "serde")]
use crate::bound_message::rust_violation;
#[cfg(feature = "zod")]
use crate::bound_message::zod_error_arg;

#[cfg(any(feature = "typescript", feature = "zod"))]
use crate::utils::escape_js_double_quoted;
#[cfg(feature = "typescript")]
use crate::utils::ts_generic_params;
use crate::utils::{
    constraining_pattern, emittable_pattern, portable_pattern, ts_quoted, type_parameters_in_scope,
    written_type,
};

#[cfg(feature = "zod")]
use crate::utils::{
    escape_js_regex_literal, extract_examples_tokens, publishes_zod_factory,
    record_zod_default_arguments, record_zod_factory, record_zod_preprocess, zod_default_arguments,
    zod_factory_argument,
};

#[cfg(feature = "archie")]
use crate::features::archie;

#[cfg(all(feature = "zod", feature = "mongodb"))]
use crate::features::object_id::get_object_id_zod_schema_with;

// The 24-character hex an `ObjectId`'s `$oid` member holds, as a JSON-schema `pattern`.
#[cfg(all(feature = "jsonschema", feature = "mongodb"))]
use crate::features::object_id::OBJECT_ID_HEX_PATTERN;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use crate::utils::{
    AliasKind, MapKeyWire, PublishedShape, ShapeQuestion, lookup_alias_info, record_key_wire,
    record_shape_question, record_value_shape, shape_questions_for,
};

#[cfg(feature = "serde")]
use crate::utils::{
    Declared, NonObjectWire, TrivialPattern, UnionParameter, WireLeaf, WrittenAs,
    forget_field_validators, is_recorded_plain_enum, record_declared, record_field_validator,
    record_flattened_beside_parameters, record_non_object_wire, record_open_map,
    record_open_map_parameters, record_plain_enum, record_text_key_parameters,
    record_union_parameters, record_value_parameter, recorded_non_object_wire, trivial_pattern,
    union_parameters, value_parameter,
};

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use crate::features::serde::brand_slot;
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
use crate::features::serde::rename_direction_rejection;
// `has_serde_read_hook` asks whether a field already reads itself through a function of the
// author's own, which only matters where a generated reader might displace one.
#[cfg(feature = "serde")]
use crate::features::serde::{
    NAMED_READ_HOOK_PREFIX, SerdeFieldMeta, SerdeTypeMeta, Tagging, derives_deserialize,
    has_serde_read_hook, parse_read_renames, parse_written_renames, struct_tag,
};
use crate::features::serde::{
    brand_field, has_serde_default, member_steps, parse_serde_key_omission,
    transparent_fields_apart, variant_on_the_wire, variant_steps, written_and_read_by_the_author,
};

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
use crate::features::recovering_decode::{brand_recovering_decode, reading_the_authors_scope};
#[cfg(feature = "serde")]
use crate::features::recovering_decode::{
    enums::{enum_recovering_decode, tag_arm_refusals},
    struct_recovering_decode, written_as_one_value,
};
// The type is named where a positional slot's own omission is read: the tuple-struct walk, which
// only a describing build performs, and the variant walk, which every build performs.
use crate::features::serde::SerdeKeyOmission;
#[cfg(any(feature = "zod", feature = "jsonschema"))]
use crate::features::serde::defaults::SerdeDefaults;
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
use crate::features::serde::defaults::default_refusals;
#[cfg(feature = "zod")]
use crate::features::serde::defaults::{holds_a_placeholder, substituted};

#[cfg(feature = "serde")]
use crate::field_type::{parse_serde_field_attributes, parse_serde_type_attributes};

#[cfg(feature = "serde")]
use crate::field_type::{is_interior_mutability_wrapper, is_refused_sequence_wrapper};
use crate::field_type::{
    is_map_wrapper, is_ownership_wrapper, is_sequence_wrapper, is_transparent_wrapper,
    written_std_range, written_std_result,
};

use crate::features::model_schema_prop::{
    CHECK_TAKES, FunctionName, LiteralValue, ModelSchemaPropMeta, PREPROCESS_TAKES, function_names,
    parse_model_schema_prop_attributes,
};

#[cfg(feature = "jsonschema")]
use crate::features::jsonschema::{
    MergeDiagnostic, MergedSource, SchemaParameter, UNIT_MEMBER_RECORD,
    generate_plain_enum_json_schema_method as generate_plain_enum_json_schema_method_impl,
    generate_struct_json_schema_method as generate_struct_json_schema_method_impl, in_flight_type,
    json_schema_methods, merged_object_value,
};
#[cfg(feature = "jsonschema")]
use crate::utils::json_argument_binding;

#[cfg(feature = "dart")]
use crate::features::dart::{DartTypeArgs, dart_schema_dispatch};

#[cfg(feature = "swift")]
use crate::features::swift::{SwiftTypeArgs, refuses_swift, swift_schema_dispatch};

#[cfg(feature = "kotlin")]
use crate::features::kotlin::{KotlinArgs, kotlin_refused_width, kotlin_schema_dispatch};

#[cfg(any(
    feature = "typescript",
    feature = "zod",
    all(feature = "serde", feature = "jsonschema")
))]
use crate::utils::get_enum_docs;
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use crate::utils::get_struct_docs;

#[cfg(any(
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema",
    feature = "serde"
))]
use crate::utils::ident_schema_module_name;

use crate::utils::{
    claim_published_name, compute_alias_export_name, compute_item_export_name, is_declared_ident,
};

#[cfg(feature = "typescript")]
use crate::utils::get_item_docs;
#[cfg(feature = "typescript")]
use crate::utils::ident_reexport_ts;

#[cfg(feature = "zod")]
use crate::utils::ident_reexport_zod;

#[cfg(feature = "zod")]
use crate::utils::record_zod_union_members;

#[cfg(all(feature = "serde", feature = "zod"))]
use crate::utils::ZodUnionMember;

#[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
use crate::utils::{FlattenVariant, record_flatten_variants, record_wire_leaves};

#[cfg(all(feature = "serde", feature = "typescript"))]
use crate::utils::record_ts_union_members;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use crate::utils::register_alias_info;
use crate::utils::{
    bson_refused_holder, forwarded_key_maps, forwarded_key_parameter, is_recorded_bson_refused_key,
    is_recorded_untagged_enum, is_wire_scalar_type, record_bson_refused_holder,
    record_bson_refused_key, record_forwarded_key_maps, record_forwarded_key_parameter,
    record_untagged_enum, record_wire_scalar,
};

#[cfg(feature = "serde")]
use crate::utils::record_unit_struct;

#[cfg(feature = "serde")]
use crate::utils::to_snake_case;
#[cfg(feature = "serde")]
use crate::utils::{forget_checks, names_a_check, record_member_checks, record_own_checks};

use crate::rename_rule::resolve_rename_rule;

/// Every argument the `model_schema` parser reads, in the order the unknown-argument rejection
/// names them.
const KNOWN_ARGS: &[&str] = &[
    "name",
    "pattern",
    "minLength",
    "maxLength",
    "no_display",
    "default_types",
    "enable",
    "check",
    "preprocess",
];

/// What `enable` takes, as a refusal of anything else says it.
const ENABLE_TAKES: &str = "a list of string literals, written `enable = [\"decode_with\"]` or \
                            `enable = [\"decode_with\", \"mongodb\"]`";

/// What a refusal of a `check` written in `#[model_schema(...)]` opens with.
const TYPE_CHECK: &str = "`model_schema` argument `check`";

/// What a refusal of a `preprocess` written in `#[model_schema(...)]` opens with.
const TYPE_PREPROCESS: &str = "`model_schema` argument `preprocess`";

/// The types serde writes a key of as a number or a `bool`. The `bson` library refuses to write
/// such a key, and reads a stored one back as text, which the type refuses.
const BSON_REFUSED_KEY_TYPES: [&str; 15] = [
    "bool", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
    "u128", "usize",
];

/// The text an integer map key is read from, as Zod's record reads it: digits under an optional
/// sign, then a fraction of zeros at most, which Zod reads as that integer.
#[cfg(feature = "jsonschema")]
const INTEGER_KEY_PATTERN: &str = "^-?[0-9]+(\\.0+)?$";

/// The text a floating-point map key is read from, as Zod's record reads it.
#[cfg(feature = "jsonschema")]
const NUMBER_KEY_PATTERN: &str = "^-?[0-9]+(\\.[0-9]+)?$";

/// The text a `bool` map key is read from.
#[cfg(feature = "jsonschema")]
const BOOLEAN_KEY_PATTERN: &str = "^(true|false)$";

/// What a [`parameter_placeholder`] opens with, ahead of the position.
const PLACEHOLDER_OPENS: &str = "__tixschema_parameter_";

/// What a [`parameter_placeholder`] closes with, after the position.
const PLACEHOLDER_CLOSES: &str = "__";

/// What every plain-enum flatten diagnostic says about its own reach, so an author who fixes the
/// one declaration named there knows what was and was not checked around it.
#[cfg(feature = "serde")]
const FLATTENED_PLAIN_ENUM_SCOPE: &str = "This refusal holds in every build with serde, and only a \
     plain enum declared above reaches it: one declared below, or in another crate, is not \
     refused here, and where a schema surface is on it keeps its TypeScript and Zod \
     intersection, and is refused by the JSON surface when the merge runs.";

/// What the refusal of a flattened value written in place, which no name stands for, says about
/// its own reach.
#[cfg(feature = "serde")]
const FLATTENED_IN_PLACE_SCOPE: &str = "This refusal holds in every build with serde.";

/// What the refusal of a flattened union with a member over `()` says about its own reach: the
/// JSON Schema merge refuses a union declared below by the record the union publishes, which no
/// member its expansion does not see as `()` sets.
#[cfg(feature = "serde")]
const FLATTENED_UNIT_MEMBER_SCOPE: &str = "This refusal holds in every build with serde, and only \
     a type declared above reaches it: one declared below, or in another crate, is not refused \
     here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is \
     refused by the JSON surface when the merge runs, but where the member is the union's own \
     type parameter filled at `()`, or a type declared below the union that serde writes as `()`, \
     which the JSON surface reads as no key.";

/// What the refusal of a flattened type serde writes as no object says about its own reach.
#[cfg(feature = "serde")]
const FLATTENED_NON_OBJECT_SCOPE: &str = "This refusal holds in every build with serde, and only a \
     type declared above reaches it: one declared below, or in another crate, is not refused \
     here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is \
     refused by the JSON surface when the merge runs.";

/// What a refusal of a newtype variant under a tag alone over a value serde writes as `()` says
/// serde and the surfaces make of it.
#[cfg(feature = "serde")]
const TAGGED_UNIT_WRITTEN: &str = "beside the tag serde writes the variant as the tag alone, and \
     reads it back, while the TypeScript type and the Zod schema of the variant intersect the \
     tag's object with `null`, which admits no value.";

/// What the refusal of a tagged newtype variant over a type serde writes as an `Option` says
/// about its own reach, which is the reach of the refusal of a value written as no object.
#[cfg(feature = "serde")]
const TAGGED_OPTIONAL_SCOPE: &str = FLATTENED_NON_OBJECT_SCOPE;

/// The list `validate()` collects each message in.
const VALIDATE_SINK: &str = "errors";

/// The indent every member of an emitted object is written at, and so the indent its `JSDoc` block
/// is written at.
const MEMBER_INDENT: &str = "  ";

/// Why a `#[model_schema_prop]` on a brand's slot is refused: what a brand's schema is built from,
/// and where the checks it does carry are written instead.
const BRANDED_SLOT_PROP_MESSAGE: &str = "`#[model_schema_prop]` is unread on the slot of a \
     `#[serde(transparent)]` newtype -- a brand publishes its inner's own schema with a `.brand()` \
     written onto it, so no key written here reaches any surface. The checks a brand does carry \
     are written on the type itself: #[model_schema(pattern = \"...\", minLength = N, maxLength = \
     N)]. Move the check there, or drop the attribute.";

/// Why a `#[model_schema_prop]` on any other field of a brand is refused: serde neither reads nor
/// writes that field.
const UNREAD_BRAND_FIELD_PROP_MESSAGE: &str = "`#[model_schema_prop]` is unread on a field serde \
     neither reads nor writes -- a `#[serde(transparent)]` struct is read and written as the value \
     of one field alone, and a brand publishes that field's own schema with a `.brand()` written \
     onto it, so no key written here reaches any surface. Drop the attribute.";

/// The names the `indexmap` feature reads: each with what it is read as, and the fewest type
/// arguments it is read with.
#[cfg(not(feature = "indexmap"))]
const INDEXMAP_NAMES: [(&str, &str, usize); 2] =
    [("IndexMap", "a map", 2), ("IndexSet", "a list", 1)];

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const WRITTEN_AS_ARRAY: &str = "a JSON array";
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const WRITTEN_AS_NULL: &str = "null";
const WRITTEN_AS_OBJECT: &str = "a JSON object";

/// What the object joins in place of a source read as the entries of a map, where serde reads it
/// as `None`: every key it was handed, with any value, and none of them in the value read.
#[cfg(feature = "zod")]
const ZOD_DROPPED_ENTRIES: &str = "z.record(z.string(), z.unknown()).transform(() => ({}))";

/// One variant of a discriminated enum, carrying everything its union member is rendered from.
struct DiscriminatedVariant {
    discriminator_value: String,
    docs: String,
    field_defs: Vec<FieldDef>,
    /// The variant's own `#[serde(flatten)]` sources, apart from `field_defs`: they write no key of
    /// their own.
    flattened_fields: Vec<FieldDef>,
    kind: VariantKind,
}

/// Per-variant data collected from a discriminated enum, plus the collected serde validators, the
/// `compile_error!` tokens for any field-level guard violations, and the `validate()` match arms,
/// empty where no variant carries a constrained member and there is nothing to aggregate.
type DiscriminatedVariantData = (
    Vec<DiscriminatedVariant>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
);

/// Rendered per-variant output for a discriminated enum: TypeScript fragments, Zod fragments, and
/// JSON-schema fragments.
type RenderedVariants = (Vec<String>, Vec<String>, Vec<proc_macro2::TokenStream>);

/// What an untagged enum's members contribute to an object that merges it, which only the Zod
/// surface multiplies out and so only it has a member type for. The other tables carry the same
/// slot spelled as what they never read.
#[cfg(all(feature = "serde", feature = "zod"))]
type ZodMergeParts = Vec<ZodUnionMember>;
#[cfg(all(feature = "serde", not(feature = "zod")))]
type ZodMergeParts = Vec<String>;

/// Per-member data from an untagged enum's member walk: TypeScript member types, their merged
/// spelling, Zod member schemas, their merged contribution, JSON-schema value tokens,
/// `compile_error!` tokens for guard violations, per-member serde validators, and the
/// `validate()` match arms those validators run from.
#[cfg(feature = "serde")]
type UntaggedMemberData = (
    Vec<String>,
    Vec<String>,
    Vec<String>,
    ZodMergeParts,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
);

/// Per-field data collected from a struct: the regular field defs, the `#[serde(flatten)]` field
/// defs, the serde validation functions, the `validate()` body fragments, and the
/// `compile_error!` tokens for any field-level guard violations.
type StructFieldData = (
    Vec<FieldDef>,
    Vec<FieldDef>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
);

/// Borrowed pieces needed to assemble the final token stream for a branded newtype.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
struct BrandedNewtypeOutput<'parts> {
    /// `model_schema_validate_bound`, where the brand carries a bound.
    bound_alone: &'parts proc_macro2::TokenStream,
    /// `model_schema_bound_text`, read off the brand as its author wrote it.
    bound_text_method: &'parts proc_macro2::TokenStream,
    decode_with: &'parts DecodeWithParts,
    delegate_impl_items: &'parts [proc_macro2::TokenStream],
    display_tokens: &'parts proc_macro2::TokenStream,
    item_struct: &'parts syn::ItemStruct,
    module_ident: &'parts Ident,
    name: &'parts Ident,
    schema_example_tokens: &'parts proc_macro2::TokenStream,
    schema_impl_items: &'parts [proc_macro2::TokenStream],
    validate_method: &'parts proc_macro2::TokenStream,
    validation_tokens: &'parts proc_macro2::TokenStream,
}

/// What a brand's own bound adds to it: the functions of its schema module, and what holds a
/// value of the brand to the bound. Nothing for a brand that carries none.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[derive(Default)]
struct BrandBound {
    /// `model_schema_validate_bound`, the method that answers for the bound alone.
    alone: proc_macro2::TokenStream,
    /// What `validate()` runs: the validator, on the value `self` holds.
    held: Option<proc_macro2::TokenStream>,
    module_items: proc_macro2::TokenStream,
}

/// A constrained brand's generated validation: the schema-module functions, and the expression
/// through which `validate()` reaches the inner value it hands `validator`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
struct BrandedValidation {
    checked_inner: proc_macro2::TokenStream,
    /// The validator, and beside it what a brand that declares a parameter asks its filling with.
    validate_fn: proc_macro2::TokenStream,
    /// The schema-module function `validate()` calls: `validate_value`, or for a brand that
    /// declares a parameter the one that runs it where the filling is the declared default.
    validator: proc_macro2::Ident,
}

/// The JSON schema shape a branded newtype's inner field describes.
#[cfg(feature = "jsonschema")]
enum BrandedJsonInner {
    /// The string a chrono type writes, named by the `"format"` keyword that says which instant it
    /// spells — the one field position carries for the same type.
    #[cfg(feature = "chrono")]
    Chrono(&'static str),
    /// The `$oid` object an `ObjectId` writes, whose hex member carries the brand's string
    /// constraints.
    #[cfg(feature = "mongodb")]
    ObjectId,
    /// A `"type"` keyword those constraints sit beside.
    Scalar(String),
    /// An inner no one keyword names, carried by the dispatch that describes the same type wherever
    /// else it is written.
    Slot(Box<FieldDef>),
}

/// Whether the schema a brand narrows already states a `pattern` of its own.
#[cfg(feature = "jsonschema")]
#[derive(Clone, Copy)]
enum BasePattern {
    /// The base states none, so the brand's is the schema's own `pattern` keyword.
    Absent,
    #[cfg(feature = "mongodb")]
    Stated,
}

/// What a field bottoms out in, under the wrappers it was written beneath.
#[cfg(feature = "serde")]
struct ConstrainedShape {
    leaf: ConstraintLeaf,
    /// Every non-`'static` lifetime the field's type spells, deduplicated.
    lifetimes: Vec<syn::Lifetime>,
    wraps: Vec<ConstraintWrap>,
}

/// What the serde hook of a list held to a count is declared with: the type it reads, the type
/// parameters that type is written from, and the bounds it holds them to.
#[cfg(feature = "serde")]
struct ListHookHeader {
    /// What holds `L` to being a list whose items are counted.
    counted: proc_macro2::TokenStream,
    /// What holds each transparent wrapper to what it derefs to.
    held_to: Vec<proc_macro2::TokenStream>,
    /// One type parameter per transparent wrapper the list is written under.
    holders: Vec<proc_macro2::Ident>,
    read: proc_macro2::TokenStream,
}

/// The value a constrained field ultimately writes, and how it is spelled.
#[cfg(feature = "serde")]
enum ConstraintLeaf {
    /// A list, a value of its own: `minLength` and `maxLength` count its items, and the walk goes
    /// no further into it.
    List,
    /// A type declared elsewhere, as the field writes it: a brand measured on the text its own
    /// expansion publishes, or an alias of a string.
    Named(Box<syn::Type>),
    /// The bare Rust numeric type, which is the validator's parameter type.
    Number(&'static str),
    /// A filesystem path, whose checks read its `to_string_lossy` rendering.
    Path,
    Str,
}

/// Where a field's constraints are enforced.
///
/// Both readings publish the same two helpers into the schema module. What differs is whether the
/// field itself is hung with the `#[serde(deserialize_with = …)]` that runs the check as the
/// payload is read.
#[cfg(feature = "serde")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConstraintGate {
    /// The deserializer as well as the validator: on an untagged union's member, whether the member
    /// is admissible is what chooses which variant the payload is.
    Deserializer,
    /// The validator alone: every position where a constraint decides only whether a value is
    /// admissible, never what it is.
    Validator,
}

/// What the end of a constraint walk does with a violation: `validate()` collects every one, a
/// `Deserializer` fails at the first.
#[cfg(feature = "serde")]
#[derive(Clone, Copy)]
enum CheckSink {
    Collect,
    Fail,
}

/// How a member's check reaches its value: a struct field off `self`, a variant member off the
/// binding its match arm introduced.
#[derive(Clone, Copy)]
enum MemberAccess {
    SelfField,
    VariantBinding,
}

/// Where a member is read, and the name it is reported under.
pub struct MemberPlace {
    /// The expression the member is read from.
    pub checked: proc_macro2::TokenStream,
    /// The steps serde writes on the way to the member, joined by dots. `None` for a member
    /// serde writes as the value of what holds it.
    pub reported: Option<String>,
}

/// A constrained member as its helpers are written.
#[cfg(feature = "serde")]
struct CheckedMember {
    /// The expression `validate()` reads the member from.
    checked: proc_macro2::TokenStream,
    /// The member's ident as a guard message names it, empty for a slot.
    label: String,
    /// The name each violation is reported under: the steps serde writes on the way to the
    /// member, joined by dots. `None` for a member reported under none.
    reported: Option<String>,
    /// What `validate_{stem}_value` and `deserialize_{stem}` are named from.
    stem: String,
}

/// A wrapper a value is reached through, outermost first: `Option` yields its `Some`, a transparent
/// wrapper yields what it derefs to, and a reference yields what it borrows, which only the
/// validator reaches. A sequence yields each element and a map each entry's value, on the
/// walk to what a field's own type declares alone: a field's own bound stops at a list and reads
/// no map.
#[cfg(feature = "serde")]
#[derive(Clone, Copy, Eq, PartialEq)]
enum ConstraintWrap {
    Borrowed,
    Map,
    Optional,
    /// A std range, on the walk alone: each bound it holds, under the key serde writes it under.
    Range(RangeKind),
    Sequence,
    Transparent,
}

/// What the walk below a member ends on.
#[cfg(feature = "serde")]
enum HeldEnd {
    /// A std `Result`: each arm that holds something the walk reaches, by the key serde writes it
    /// under, with the walk below it.
    Arms(Vec<(&'static str, HeldShape)>),
    /// A value of a declared type, whose own `validate()` is run.
    Declared,
    /// A tuple: each position that holds something the walk reaches, with the walk below it.
    Positions(Vec<(usize, HeldShape)>),
}

/// The walk from a member's value to every value below it whose own type may publish `validate()`.
#[cfg(feature = "serde")]
struct HeldShape {
    end: HeldEnd,
    /// The wrappers the walk reads through to its end, outermost first.
    wraps: Vec<ConstraintWrap>,
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
impl BrandBound {
    fn of(validation: Option<&BrandedValidation>, module_ident: &Ident) -> Self {
        validation.map_or_else(Self::default, |held_to| {
            let (validator, checked_inner) = (&held_to.validator, &held_to.checked_inner);
            let bound_alone =
                proc_macro2::Ident::new(judged_read::BOUND_ALONE, proc_macro2::Span::call_site());
            Self {
                alone: quote! {
                    /// What the brand's own bound answers of the value, the checks it names apart.
                    pub fn #bound_alone(&self) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
                        #module_ident::#validator(#checked_inner)
                    }
                },
                held: Some(branded_bound_call(held_to, module_ident)),
                module_items: held_to.validate_fn.clone(),
            }
        })
    }
}

#[cfg(feature = "serde")]
impl HeldShape {
    /// Whether the walk reads a map, at any depth.
    fn walks_a_map(&self) -> bool {
        self.wraps
            .iter()
            .any(|wrap| matches!(*wrap, ConstraintWrap::Map))
            || match &self.end {
                HeldEnd::Arms(arms) => arms.iter().any(|(_, held)| held.walks_a_map()),
                HeldEnd::Declared => false,
                HeldEnd::Positions(positions) => {
                    positions.iter().any(|(_, held)| held.walks_a_map())
                }
            }
    }
}

/// One step of the place a walk holds a value at.
#[cfg(feature = "serde")]
#[derive(Clone)]
enum PlaceStep {
    /// The depth a list level binds its item's index at.
    Index(usize),
    /// The depth a map level binds its entry at, whose key is written as serde writes it, where
    /// serde does.
    Key(usize),
    /// A key or a position serde writes, read as the walk is written.
    Written(String),
}

/// Holds the generated validation code for a single field.
#[cfg(feature = "serde")]
struct FieldValidationCode {
    /// Functions to emit into the schema module (static validator + serde deserializer).
    pub module_items: proc_macro2::TokenStream,
    /// Code to contribute to the type-level `validate()` method body.
    pub validate_body: proc_macro2::TokenStream,
}

/// A map member's rendering with the slot wraps still to apply: a `json!` literal fragment, which
/// only a caller writing inside `serde_json::json!` can inline, or a standalone `serde_json::Value`
/// expression, which either caller can take as it stands.
#[cfg(feature = "jsonschema")]
enum MapMemberItem {
    Fragment(proc_macro2::TokenStream),
    Value(proc_macro2::TokenStream),
}

/// Why a value in a slot has no rendering, and where it was written: a map key with no rendering,
/// whatever reason it has and at whatever depth the value walk reached it. The field name the
/// message needs belongs to the caller rather than to the dispatch, so the reason travels out and
/// the message is formatted once at the top. The span travels with it because the callers that
/// format it hold a slice of slots and cannot tell which one the dispatch refused.
#[cfg(feature = "jsonschema")]
#[derive(Debug)]
enum MapMemberRejection {
    Key(MapKeyRejection, proc_macro2::Span),
}

#[cfg(feature = "jsonschema")]
impl MapMemberRejection {
    /// The written tokens the diagnostic points at, behind the guards rather than in front of
    /// them: every authoring position a map key is written in is refused upstream by a guard of its
    /// own, spanned on the type as written. A key under a sequence wrapper still carries the
    /// element's span here, `get_field_def` having collapsed the wrapper onto it, and no reachable
    /// position draws that caret.
    const fn span(&self) -> proc_macro2::Span {
        match *self {
            Self::Key(_, span) => span,
        }
    }
}

/// What a map key opens: an open member set, the members it enumerates, members under the text the
/// key is written as, or no object at all — read the same way at every position a map is written
/// in.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
enum MapKeyPath<'key> {
    /// A key named by a type path, whose `enum_members()` become the object's keys.
    Enumerated(&'key str),
    /// A key serde writes as a bare string, which enumerates nothing — one schema stands for every
    /// member. `String` is one, and so is a brand the registry proves serde writes as a string.
    Open,
    /// A key with no object for any surface to describe, carrying the reason.
    Refused(MapKeyRejection),
    /// A key serde writes as text of its own, which enumerates no member: a number, a `bool`, a
    /// `char`, a date, and a name over one.
    Unnarrowed,
}

/// Why a map key a field reaches has no rendering. Every reason carries the name the author can act
/// on, and every one stands on every surface, the key being read off the field all three render
/// from.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[derive(Debug)]
enum MapKeyRejection {
    /// The registry proves the key type carries no `enum_members()`.
    NoEnumMembers(String),
    /// The key is written as an `Option`, named by the inner it holds. A `Some` writes what the
    /// inner writes; a `None` writes nothing a key can be.
    Optional(String),
    /// The key is written under a sequence wrapper, named by the element it holds.
    Sequenced(String),
    /// The key's own type is one serde writes as neither a string nor anything it will stringify.
    Unwritable {
        /// The key as it was written, by its own name where it has one and by its shape where it
        /// has none.
        key_name: String,
        /// What serde writes for it instead, as the message names it.
        written_as: &'static str,
    },
}

/// A type this crate describes no schema for that a written type reaches, at any depth. Every
/// position that reads a written type asks this, and each spans and names its own subject.
#[derive(Debug)]
pub enum UndescribableStd<'name> {
    /// `serde_json::Value` and `serde_json::Map`: any JSON value, which no target can describe.
    /// Refused by its full path in every member and filling of an item, ahead of every shape
    /// ([`any_json_errors`]), and in each type a service operation puts on the wire; the message
    /// of the check a bare name earns ([`any_json_checks`]).
    AnyJson(&'name str),
    /// `OsString`/`OsStr`: serde writes an externally tagged enum naming the target platform.
    PlatformString(&'name str),
    /// `LinkedList`: serde writes the array a `Vec` writes, and the covered spelling is that one.
    Sequence(&'name str),
    /// A cell/lock/lazy-init wrapper or borrow guard: serde implements neither direction.
    Wrapper(&'name str),
}

/// Where a member reaches a map under the key a walk asks for: one the `bson` library refuses, or
/// one of the holder's own type parameters.
enum RefusedKey<'key> {
    /// A model or an alias declared above that holds such a map.
    Held(HeldMap),
    /// A map the member's own type writes, under this key.
    Written(&'key syn::Type),
}

/// A model or an alias declared above that holds a map under the key a walk asks for.
struct HeldMap {
    /// The map, after the member that holds it where a model holds it.
    described: String,
    /// The model's or the alias's name, written with its arguments where they put the map in its
    /// row.
    model: String,
}

/// What a walk for the maps a type holds asks of each key.
#[derive(Clone, Copy)]
enum KeyAsked {
    /// The holder's own type parameter at this position, written as [`parameter_placeholder`]
    /// spells it.
    Parameter(usize),
    /// A key the `bson` library refuses.
    Refused,
}

/// What a newtype variant's inner value is when the tag is written beside it rather than around
/// it.
#[cfg(feature = "serde")]
enum TaggedContent {
    /// A named type: what it writes are members, and they are the ones that join the tag, unless
    /// it writes a map, which [`ranges::tagged_slot_map_error`] refuses.
    Flattened,
    /// serde refuses to write it beside the tag, and names the shape the way this names it.
    Refused(&'static str),
    /// `()`, which serde writes beside the tag as the tag alone.
    Unit,
    /// serde writes it, but its members are not a set the expansion can name, so no schema closed
    /// around the tag can admit them.
    Unnameable(&'static str),
}

/// The lone slot of a newtype variant under a tag alone: its type as written, and the def it is
/// read as.
#[cfg(feature = "serde")]
#[derive(Clone, Copy)]
struct TaggedSlot<'slot> {
    inner: &'slot FieldDef,
    written: &'slot syn::Type,
}

#[cfg(feature = "serde")]
impl<'slot> TaggedSlot<'slot> {
    /// The name a refusal quotes the slot's type by, with the arguments written for it, where
    /// serde writes its value as the bare name of a plain enum's variant with no `Option` on the
    /// way: a key holding null beside the tag. `None` for every other slot.
    fn plain_enum(self) -> Option<String> {
        let FieldDefType::SiblingType(inner_name, _) = &self.inner.field_type else {
            return None;
        };
        let written = value_filling(self.inner).unwrap_or(self.inner);
        (written_as_a_plain_enum(self.inner) && !writes_an_optional(written))
            .then(|| filled_spelling(self.written, inner_name))
    }

    /// The name a refusal quotes the slot's type by, beside the filling serde writes its value as:
    /// the type's [`union_spelling`] and no filling, or, where serde writes the type as what fills
    /// its type parameter, the type with the arguments written for it beside that filling. `None`
    /// for a slot that names no type.
    fn read(self) -> Option<(String, Option<&'slot FieldDef>)> {
        let FieldDefType::SiblingType(inner_name, _) = &self.inner.field_type else {
            return None;
        };
        Some(value_filling(self.inner).map_or_else(
            || (union_spelling(self.written, inner_name), None),
            |filling| (filled_spelling(self.written, inner_name), Some(filling)),
        ))
    }
}

/// What `decode_with` adds to an item, both empty without the flag.
#[derive(Default)]
struct DecodeWithParts {
    /// The items added to the schema module a surface writes, or that module whole where no
    /// surface writes one.
    schema_module: TokenStream,
    type_impl: TokenStream,
}

/// What an `enable = [...]` list turns on for a type.
#[derive(Clone, Copy, Default)]
pub struct Enabled {
    /// `"archie"`: the conversions to and from archie's frontmatter `Value`, under the `archie`
    /// feature.
    pub archie: bool,
    /// `"decode_with"`: `from_value_with`, `from_value_piped` and their walkers, and under the
    /// `bson` feature `from_bson_with`, `from_bson_piped` and theirs.
    pub decode_with: bool,
    /// `"mongodb"`: the MongoDB operations and the typed paths, under the `mongodb` feature.
    pub mongodb: bool,
}

/// One `default_types(IdType = String)` entry as written: the parameter it names, and the type
/// declared for that parameter.
struct DefaultTypeEntry {
    name: syn::Ident,
    ty: syn::Type,
}

/// The arguments written in `#[model_schema(...)]`, as parsed.
#[derive(Default, Clone)]
struct ModelSchemaArgs {
    /// The parser's refusal of the attribute's arguments — one it does not read, or a value it
    /// cannot read — spanned on the tokens that earned it.
    arg_rejection: Option<syn::Error>,
    /// The functions `check` names, in the order written: each takes the whole value.
    check: Vec<FunctionName>,
    /// The default type declared per type parameter, in written order: `default_types(IdType =
    /// String, DateType = f64)`.
    default_types: Vec<(syn::Ident, syn::Type)>,
    /// What `enable = [...]` turns on.
    enable: Enabled,
    max_length: Option<usize>,
    min_length: Option<usize>,
    name_override: Option<String>,
    /// Opt-out of the branded newtype `Display` impl (and its inner-type assertion) for brands
    /// whose inner type is a container rather than a scalar.
    no_display: bool,
    /// `pattern` in the spelling every surface reads the same way, or as it was written when it
    /// earned a `pattern_rejection`.
    pattern: Option<String>,
    /// What keeps `pattern` off the surfaces it was written for, spanned on the literal it was
    /// written as.
    pattern_rejection: Option<syn::Error>,
    /// The functions `preprocess` names, in the order written: each takes the whole payload.
    preprocess: Vec<FunctionName>,
    /// The first of `pattern`, `minLength` and `maxLength` written, and where: a refusal of the
    /// check is spanned there.
    string_check: Option<(&'static str, proc_macro2::Span)>,
}

/// One `#[serde(flatten)]` source as a surface writes it: the spelling of what its members are, and
/// whether the object writes them at all.
#[cfg(any(feature = "typescript", feature = "zod"))]
struct MergedOperand {
    absence: SourceAbsence,
    /// What the source contributes per branch when it names a registered choice — one operand per
    /// branch, empty otherwise (falls back to `spelling`).
    #[cfg(feature = "zod")]
    branches: Vec<String>,
    /// Whether serde reads the source as the entries of a map, which its absence leaves in the
    /// object (see [`flattened_as_entries`]).
    #[cfg(feature = "zod")]
    entries: bool,
    spelling: String,
}

/// Which absence a merged source offers beside its members, and none where it always writes them.
#[cfg(any(feature = "typescript", feature = "zod"))]
#[derive(Clone, Copy)]
enum SourceAbsence {
    /// The field's own `Option` says the source writes all of its members or none of them.
    Field,
    /// The source writes its members for every value it holds.
    Never,
    /// The item the source names publishes the absence beside its value, one name away.
    Published,
}

#[cfg(any(feature = "typescript", feature = "zod"))]
impl SourceAbsence {
    /// Which of the two the source offers, for the merge that only has to count the key sets.
    #[cfg(feature = "zod")]
    const fn offered(self) -> bool {
        !matches!(self, Self::Never)
    }

    /// What a flattened source offers: the field's own `Option` is checked first (an
    /// `Option<Name>` writes nothing for `None` regardless of what `Name` publishes), then falls
    /// back to what the name itself publishes.
    fn written(fld: &FieldDef) -> Self {
        if fld.is_optional() {
            Self::Field
        } else if flattened_name_offers_absence(fld) {
            Self::Published
        } else {
            Self::Never
        }
    }
}

/// Both answers the registry holds about what a `#[model_schema()]` item publishes, built together
/// from one reading of the declaration so the two cannot come apart.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
struct Surface {
    shape: PublishedShape,
    #[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
    wire: RecordedWire,
}

/// What a [`Surface`] carries for the merge: the leaves a written type published, or the one
/// keyword a surface read off no written type names.
#[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
enum RecordedWire {
    Leaves(Vec<WireLeaf>),
    Named(Option<&'static str>),
}

/// What a flattened field's type writes as no object, in the words the flatten refusal names it
/// by: the JSON type keyword of the whole value, or of the union member at a branch.
#[cfg(feature = "serde")]
#[derive(Debug, PartialEq, Eq)]
enum NonObject {
    Branch(String, &'static str),
    Whole(&'static str),
    /// A union member at a branch that serde writes as [`WrittenAs`] says, which a refusal names.
    Written(String, WrittenAs),
}

/// What a refusal says of a union member serde writes as one [`WrittenAs`], flattened or held
/// beside a tag: the member, the keyword its branch describes, what serde writes for it there,
/// and the scope sentence the refusal closes with.
#[cfg(feature = "serde")]
struct WrittenWords {
    keyword: &'static str,
    member: &'static str,
    scope: &'static str,
    serde_writes: &'static str,
}

#[cfg(feature = "serde")]
impl WrittenWords {
    const fn of(written: WrittenAs, beside_tag: bool) -> Self {
        let named = if beside_tag {
            "and serde writes that name beside the tag as a key holding null, which a schema \
             closed around the tag rejects"
        } else {
            "and serde writes that name as a key holding null, which a schema closed around the \
             remaining fields rejects"
        };
        match written {
            WrittenAs::PlainEnumName => Self {
                keyword: "string",
                member: "a plain enum's variant name",
                scope: FLATTENED_PLAIN_ENUM_SCOPE,
                serde_writes: named,
            },
            WrittenAs::UnitVariantName => Self {
                keyword: "string",
                member: "the name of an externally tagged enum's unit variant",
                scope: FLATTENED_NON_OBJECT_SCOPE,
                serde_writes: named,
            },
            WrittenAs::Unit => Self {
                keyword: "null",
                member: "`()`",
                scope: if beside_tag {
                    FLATTENED_NON_OBJECT_SCOPE
                } else {
                    FLATTENED_UNIT_MEMBER_SCOPE
                },
                serde_writes: if beside_tag {
                    "for which serde writes the tag alone, and which it does not read back as \
                     that member"
                } else {
                    "for which serde writes no key, and which it does not read back as that \
                     member"
                },
            },
        }
    }
}

#[cfg(feature = "serde")]
impl NonObject {
    /// The union member at `branch` that is no object, where `non_object` names its JSON type
    /// keyword, and `None` for a member that is an object.
    fn member(
        branch: &str,
        non_object: Option<&'static str>,
        written_as: Option<WrittenAs>,
    ) -> Option<Self> {
        let named = non_object?;
        Some(written_as.map_or_else(
            || Self::Branch(branch.to_owned(), named),
            |written| Self::Written(branch.to_owned(), written),
        ))
    }
}

/// Where the leaves a name publishes are read from: the record a schema surface writes, or the one
/// every build with serde writes.
#[cfg(feature = "serde")]
type LeavesOf = fn(&str) -> Option<Vec<WireLeaf>>;

/// What a tuple struct publishes, which is not always the tuple of its declared slots.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
enum TupleStructShape {
    /// The fixed-arity array serde writes for every other declared arity, holding the slots the
    /// wire carries.
    Array(Vec<FieldDef>),
    /// The bare value serde writes for a struct declaring exactly one slot.
    BareValue(Box<FieldDef>),
}

/// What walking a tuple struct's slots yields.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
struct TupleSlots {
    /// The `compile_error!` tokens of every guard a slot violates.
    guard_errors: Vec<proc_macro2::TokenStream>,
    shape: TupleStructShape,
    /// What each constrained slot contributes to `validate()`.
    validate_bodies: Vec<proc_macro2::TokenStream>,
    /// The validator and the hook of each constrained slot.
    validation_fns: Vec<proc_macro2::TokenStream>,
}

/// One constrained member of a variant as the arm that matched it binds it: where the pattern finds
/// it, and the name the check reads it under.
pub struct BoundMember {
    pub binding: proc_macro2::Ident,
    /// The member's declaration index, which is what places a positional binding in a tuple
    /// pattern — a named member is placed by its ident instead and never consults this.
    pub index: usize,
    /// The member's field ident, or `None` for a positional slot, which the pattern matches by
    /// position because it has no name to match by.
    pub named: Option<proc_macro2::Ident>,
}

/// What one untagged variant's member walk produces: the members its surfaces are rendered from,
/// the bindings its constrained members are checked under, and the three enum-wide lists the walk
/// adds to, which the caller joins onto its own in declaration order.
#[cfg(feature = "serde")]
struct UntaggedVariantMembers {
    bound: Vec<BoundMember>,
    checks: Vec<proc_macro2::TokenStream>,
    deferred_attrs: Vec<Vec<syn::Attribute>>,
    field_defs: Vec<FieldDef>,
    /// The variant's own `#[serde(flatten)]` sources, held apart from `field_defs`: they write no
    /// key of their own, so every surface joins them onto the object the rest of the fields close.
    flattened_fields: Vec<FieldDef>,
    guard_errors: Vec<proc_macro2::TokenStream>,
    validation_fns: Vec<proc_macro2::TokenStream>,
}

/// Where the default serde reads for the absent key of a member of an enum's variant sits among the
/// enum's, by variant and member.
type VariantDefaultAt = dyn Fn(&str, &str) -> Option<usize>;

/// What an untagged enum hands the walk of each of its variants' members.
#[cfg(feature = "serde")]
struct UntaggedEnum<'walk> {
    args: &'walk ModelSchemaArgs,
    name: &'walk str,
    rename_all_fields: Option<&'walk str>,
    schema_module_name: Option<&'walk str>,
    /// Where the default serde reads for the absent key of a member, by variant and member, sits
    /// among the enum's.
    serde_default: &'walk VariantDefaultAt,
    type_parameters: &'walk [String],
}

/// The two casing rules an enum's container declares, which reach two different things: `variants`
/// renames the variant names, `variant_fields` the members inside every struct variant. Bundled
/// because two loose `Option<&str>` neighbours meaning different things transpose silently.
#[derive(Clone, Copy)]
struct EnumCasing<'ctx> {
    variant_fields: Option<&'ctx str>,
    variants: Option<&'ctx str>,
}

/// What the item around a field says about it: everything [`process_field`] needs that is not the
/// field itself, bundled so the walk hands one context down instead of six loose parameters.
struct FieldContext<'ctx> {
    /// Whether the container carries `#[serde(default)]`, which supplies a value for every field
    /// under it whose key the payload leaves out.
    container_defaulted: bool,
    /// Whether the container is read back at all — whether `Deserialize` is among what it derives.
    container_read_back: bool,
    /// The arguments of the enum the field's variant belongs to: a string check written there is
    /// the field's own as well, where it reaches the field.
    enum_args: Option<&'ctx ModelSchemaArgs>,
    /// The steps serde writes on the way to each member of the container, by its place among the
    /// ones declared.
    member_steps: &'ctx [Vec<String>],
    /// Whether serde writes or reads the container: a member of a variant it neither writes nor
    /// reads has no key for a guard to read.
    on_the_wire: bool,
    rename_all: Option<&'ctx str>,
    schema_module_name: Option<&'ctx str>,
    type_name: &'ctx str,
    type_parameters: &'ctx [String],
    /// The variant this field belongs to, or `None` for a struct's own field.
    variant_ident: Option<&'ctx str>,
}

impl FieldContext<'_> {
    /// The steps serde writes on the way to the member declared at `index`.
    fn steps(&self, index: usize) -> &[String] {
        self.member_steps.get(index).map_or(&[], Vec::as_slice)
    }
}

/// Mutable output buffers shared by the discriminated-enum variant writers. Bundled so each writer
/// takes a single always-mutated `&mut`, keeping its conditionally-written fields (Zod schema, JSON
/// fields) from tripping `needless_pass_by_ref_mut` under feature subsets.
struct VariantParts {
    json_fields: Vec<proc_macro2::TokenStream>,
    schema_code: String,
    type_code: String,
}

/// The two spellings [`zod_default_block`] has on hand for a constrained bare-parameter brand's
/// check chain — see [`branded_zod_string_checks`] and [`branded_zod_base_checks`] for what each
/// renders and why both exist.
#[cfg(feature = "zod")]
struct BrandedDefaultChecks {
    /// Composed inside a deferred argument's `z.lazy(...)` thunk, onto whatever it resolves to,
    /// which is only guaranteed to expose the base `.check(...)`.
    base: String,
    /// Chained onto an eager argument — `z.string()` and the like — which the checks' own
    /// `ZodString` methods always apply to directly.
    chained: String,
}

/// What [`zod_default_block`] and [`zod_factory_block`] need beyond the item's own name and
/// parameter list, bundled into one reference because the two already carry as many positional
/// parameters as `clippy::too_many_arguments` admits.
#[cfg(feature = "zod")]
struct ZodDefaultInputs<'defaults> {
    /// Whether `$SchemaDefault`'s annotation is read back off the value it binds rather than
    /// restating the type the item declares — see [`raw_default_block`].
    #[cfg(feature = "typescript")]
    annotated_by_value: bool,
    constrained: Option<(&'defaults str, &'defaults BrandedDefaultChecks)>,
    default_types: &'defaults [(syn::Ident, syn::Type)],
}

/// What [`zod_published_binding`] needs beside the item's own name, parameters and expression.
#[cfg(feature = "zod")]
struct PublishedBinding<'binding> {
    /// The checks written on the type, which its own expression carries.
    checks: &'binding [FunctionName],
    default_types: &'binding [(syn::Ident, syn::Type)],
    /// The functions `preprocess` names on the type, each written around its own expression.
    preprocess: &'binding [FunctionName],
    /// Whether the published expression *is* a sibling's own binding — see
    /// [`republishes_sibling_binding`].
    republished: bool,
}

/// The item a tuple struct's slots are written inside, which a slot may reach back to.
#[cfg(feature = "zod")]
struct SlotOwner<'item> {
    item_name: &'item str,
    #[cfg(feature = "typescript")]
    parameters: &'item [String],
    rust_ident: &'item str,
}

/// What [`default_zod_rendering`] found: a self-contained expression left eager, or a name
/// [`deferred_zod_operand`] still has to wrap — [`zod_default_block`] needs to know which, since a
/// constrained brand's checks chain onto an eager expression but must land inside a deferred thunk.
#[cfg(feature = "zod")]
enum DefaultZodRendering {
    Deferred(String),
    Eager(String),
}

/// What a plain enum's output holds after the enum's own `impl`.
struct PlainEnumAdded<'added> {
    decode_with: &'added DecodeWithParts,
    /// The read the macro writes for an enum that publishes `validate()`.
    judged: &'added proc_macro2::TokenStream,
}

/// The parts [`assemble_schema_output`] joins.
#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
struct SchemaOutputParts<'parts, T> {
    delegate_impl_items: &'parts [proc_macro2::TokenStream],
    generics: &'parts syn::Generics,
    item: &'parts T,
    module_ident: &'parts Ident,
    name: &'parts syn::Ident,
    schema_impl_items: &'parts [proc_macro2::TokenStream],
    /// The type's `validate()`, written beside its delegates so every filling has it.
    validate_method: &'parts Option<proc_macro2::TokenStream>,
    validation_fns: &'parts [proc_macro2::TokenStream],
}

/// What a struct's container says of the members collected under it: whether it answers a missing
/// key with its own `default`, whether it is read back at all, and where the default serde reads
/// for a member's absent key sits among those written as JSON.
struct StructContainer<'container> {
    defaulted: bool,
    read_back: bool,
    serde_default: &'container dyn Fn(&str) -> Option<usize>,
}

#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
impl<'parts, T> SchemaOutputParts<'parts, T> {
    /// These parts around `item`, the item as it is emitted.
    const fn of(&self, item: &'parts T) -> Self {
        Self {
            delegate_impl_items: self.delegate_impl_items,
            generics: self.generics,
            item,
            module_ident: self.module_ident,
            name: self.name,
            schema_impl_items: self.schema_impl_items,
            validate_method: self.validate_method,
            validation_fns: self.validation_fns,
        }
    }
}

#[cfg(feature = "zod")]
impl DefaultZodRendering {
    /// The plain argument an unconstrained default composes: deferred renderings gain their
    /// `z.lazy(...)` wrapper here, at the one place both variants converge back to a single string.
    fn into_argument(self) -> String {
        match self {
            Self::Deferred(schema) => deferred_zod_operand(&schema),
            Self::Eager(schema) => schema,
        }
    }
}

#[cfg(feature = "jsonschema")]
impl MapMemberItem {
    /// The item wrapped for its slot, kept in the form it arrived in.
    fn into_member_schema(self, value: &FieldDef) -> proc_macro2::TokenStream {
        match self {
            Self::Fragment(fragment) => map_member_slot_schema(value, &fragment),
            Self::Value(item_value) => map_member_slot_value(value, item_value),
        }
    }

    /// The item wrapped for its slot as a standalone `serde_json::Value`.
    fn into_member_value(self, value: &FieldDef) -> proc_macro2::TokenStream {
        map_member_slot_value(value, self.into_value())
    }

    /// The item as a standalone `serde_json::Value`, with no slot wrap applied — a fragment lifted
    /// into the `serde_json::json!` a value form already is.
    fn into_value(self) -> proc_macro2::TokenStream {
        match self {
            Self::Fragment(fragment) => quote! { serde_json::json!(#fragment) },
            Self::Value(item_value) => item_value,
        }
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
impl ModelSchemaArgs {
    const fn has_string_constraints(&self) -> bool {
        self.pattern.is_some() || self.min_length.is_some() || self.max_length.is_some()
    }
}

impl RefusedKey<'_> {
    /// What the member holds, after its name.
    fn described(&self) -> String {
        match self {
            Self::Held(held) => format!("which holds `{}` with {}", held.model, held.described),
            Self::Written(key) => format!("a map keyed by `{}`", written_spelling(key)),
        }
    }
}

impl KeyAsked {
    /// The first map the model or the alias `name` holds of its own under the key asked for, as
    /// its expansion recorded it. No map a model keys by its own type parameters is keyed by its
    /// holder's.
    fn held_by(self, name: &str) -> Option<String> {
        match self {
            Self::Parameter(_) => None,
            Self::Refused => bson_refused_holder(name),
        }
    }

    /// Whether serde writes a key of type `key` as the key asked for.
    fn holds(self, key: &syn::Type) -> bool {
        written_key_is(key, &|name| match self {
            Self::Parameter(at) => name == parameter_placeholder(at),
            Self::Refused => {
                BSON_REFUSED_KEY_TYPES.contains(&name) || is_recorded_bson_refused_key(name)
            }
        })
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
impl Surface {
    /// The fixed-arity JSON array a tuple struct of every arity but one writes, which takes no
    /// string check and which no object can be merged with.
    const fn array() -> Self {
        Self::named(Some("container"), Some("array"))
    }

    /// The bare string a plain unit enum writes its member name as.
    #[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
    fn enumerated() -> Self {
        Self {
            shape: PublishedShape::Flat(Some("enumerated")),
            wire: RecordedWire::Leaves(vec![WireLeaf {
                branch: Vec::new(),
                non_object: Some("string"),
                written_as: Some(WrittenAs::PlainEnumName),
            }]),
        }
    }

    /// The same string where no merge reads its leaves.
    #[cfg(not(all(feature = "serde", any(feature = "zod", feature = "typescript"))))]
    const fn enumerated() -> Self {
        Self::named(Some("enumerated"), Some("string"))
    }

    /// The union an externally tagged enum writes, as the leaves a merge descending it reaches.
    #[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
    fn externally_tagged(variants: &Punctuated<syn::Variant, Token![,]>) -> Self {
        Self {
            shape: Self::union().shape,
            wire: RecordedWire::Leaves(external_variant_wire_leaves(variants)),
        }
    }

    /// The same union where no merge reads its leaves. `serde` beside a surface that merges is the
    /// pair that flattens one over the other; without both, nothing asks what the variants write and
    /// the union is the union every other enum shape registers.
    #[cfg(all(feature = "serde", not(any(feature = "zod", feature = "typescript"))))]
    const fn externally_tagged(_variants: &Punctuated<syn::Variant, Token![,]>) -> Self {
        Self::union()
    }

    /// A surface neither answer is read off a written type for.
    const fn named(shape: Option<&'static str>, wire: Option<&'static str>) -> Self {
        #[cfg(not(all(feature = "serde", any(feature = "zod", feature = "typescript"))))]
        let _: Option<&'static str> = wire;
        Self {
            shape: PublishedShape::Flat(shape),
            #[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
            wire: RecordedWire::Named(wire),
        }
    }

    /// The object a struct of named fields writes, which a merge joins its own keys to.
    const fn object() -> Self {
        Self::named(Some("object"), None)
    }

    /// Puts both answers on the entry the name has just registered — taken by value, because a
    /// surface is built for one registration and spent on it.
    fn record(self, rust_ident: &str) {
        record_value_shape(rust_ident, self.shape);
        #[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
        match self.wire {
            RecordedWire::Leaves(leaves) => record_wire_leaves(rust_ident, &leaves),
            RecordedWire::Named(non_object) => record_wire_leaves(
                rust_ident,
                &[WireLeaf {
                    branch: Vec::new(),
                    non_object,
                    written_as: None,
                }],
            ),
        }
    }

    /// The union an enum writes, whose own members the merge multiplies over where it holds them.
    const fn union() -> Self {
        Self::named(Some("union"), None)
    }

    /// The surface a written type publishes, both answers read off the one def. `parameters` is
    /// the item's own type-parameter list; a target that *is* one of them publishes a family
    /// rather than a shape — see [`PublishedShape`].
    fn written(written: &FieldDef, parameters: &[String]) -> Self {
        Self {
            shape: published_value_shape(written, parameters),
            #[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
            wire: RecordedWire::Leaves(published_wire_leaves(written, surfaced_leaves)),
        }
    }
}

impl Parse for DefaultTypeEntry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let ty = input.parse()?;
        Ok(Self { name, ty })
    }
}

/// What the parser cannot read is recorded as [`ModelSchemaArgs::arg_rejection`] rather than
/// dropped, and the item is expanded as though the argument had been left off.
fn parse_model_schema_args(args: proc_macro2::TokenStream) -> ModelSchemaArgs {
    let mut result = ModelSchemaArgs::default();

    if args.is_empty() {
        return result;
    }

    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let read = parser.parse2(args).and_then(|parsed| {
        parsed
            .iter()
            .try_for_each(|meta| apply_arg(&mut result, meta))
    });
    if let Err(rejection) = read {
        result.arg_rejection = Some(rejection);
    }

    result
}

/// Notes the first type-level string check written, and where.
fn note_string_check(result: &mut ModelSchemaArgs, check: &'static str, path: &syn::Path) {
    let written_at = path
        .get_ident()
        .map_or_else(proc_macro2::Span::call_site, proc_macro2::Ident::span);
    result.string_check.get_or_insert((check, written_at));
}

/// The name is read before the shape, so an argument this parser knows, written the wrong way, is
/// answered with what it takes rather than reported as unknown.
fn apply_arg(result: &mut ModelSchemaArgs, meta: &Meta) -> syn::Result<()> {
    let path = meta.path();
    if path.is_ident("name") {
        result.name_override = Some(name_arg_value(str_arg(meta, "name")?)?);
    } else if path.is_ident("pattern") {
        note_string_check(result, "pattern", path);
        record_pattern(result, str_arg(meta, "pattern")?);
    } else if path.is_ident("minLength") {
        note_string_check(result, "minLength", path);
        result.min_length = Some(length_arg(meta, "minLength")?);
    } else if path.is_ident("maxLength") {
        note_string_check(result, "maxLength", path);
        result.max_length = Some(length_arg(meta, "maxLength")?);
    } else if path.is_ident("no_display") {
        result.no_display = flag_arg(meta, "no_display")?;
    } else if path.is_ident("default_types") {
        result.default_types = default_types_arg(meta)?;
    } else if path.is_ident("enable") {
        result.enable = enable_arg(meta)?;
    } else if path.is_ident("decode_with") {
        return Err(syn::Error::new_spanned(
            meta,
            "tixschema: `decode_with` is now an entry of the `enable` list: write \
             `#[model_schema(enable = [\"decode_with\"])]`",
        ));
    } else if path.is_ident("check") {
        result.check = functions_arg(meta, TYPE_CHECK, CHECK_TAKES)?;
    } else if path.is_ident("preprocess") {
        result.preprocess = functions_arg(meta, TYPE_PREPROCESS, PREPROCESS_TAKES)?;
    } else {
        return Err(unknown_arg_rejection(meta));
    }
    Ok(())
}

/// The functions a `check = [...]` or a `preprocess = [...]` argument names, in the order
/// written. `subject` opens a refusal, and `takes` says what the argument is written with.
fn functions_arg(meta: &Meta, subject: &str, takes: &str) -> syn::Result<Vec<FunctionName>> {
    let Meta::NameValue(written) = meta else {
        return Err(syn::Error::new_spanned(meta, format!("{subject} {takes}")));
    };
    function_names(&written.value, subject, takes)
}

/// What an `enable = [...]` argument turns on: each entry once, `"mongodb"` only beside
/// `"decode_with"`, and the list never empty.
fn enable_arg(meta: &Meta) -> syn::Result<Enabled> {
    let Meta::NameValue(written) = meta else {
        return Err(arg_rejection(meta, "enable", ENABLE_TAKES));
    };
    let syn::Expr::Array(list) = &written.value else {
        return Err(arg_rejection(&written.value, "enable", ENABLE_TAKES));
    };
    if list.elems.is_empty() {
        return Err(syn::Error::new_spanned(
            list,
            "tixschema: the list is empty; remove `enable`",
        ));
    }
    let mut enabled = Enabled::default();
    let mut mongodb_entry = None;
    for element in &list.elems {
        let syn::Expr::Lit(expr_lit) = element else {
            return Err(arg_rejection(element, "enable", ENABLE_TAKES));
        };
        let syn::Lit::Str(entry) = &expr_lit.lit else {
            return Err(arg_rejection(element, "enable", ENABLE_TAKES));
        };
        let entry_text = entry.value();
        let turned_on = match entry_text.as_str() {
            "archie" => &mut enabled.archie,
            "decode_with" => &mut enabled.decode_with,
            "mongodb" => {
                mongodb_entry = Some(entry);
                &mut enabled.mongodb
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    entry,
                    format!(
                        "tixschema: unknown entry `{entry_text:?}`; the entries are \
                         `\"archie\"`, `\"decode_with\"` and `\"mongodb\"`"
                    ),
                ));
            }
        };
        if *turned_on {
            return Err(syn::Error::new_spanned(
                entry,
                format!("tixschema: `{entry_text:?}` is written twice"),
            ));
        }
        *turned_on = true;
    }
    if let Some(entry) = mongodb_entry
        && !enabled.decode_with
    {
        return Err(syn::Error::new_spanned(
            entry,
            "tixschema: `\"mongodb\"` requires `\"decode_with\"`: write `enable = \
             [\"decode_with\", \"mongodb\"]`",
        ));
    }
    Ok(enabled)
}

/// The literal an argument's value was written as — every valued `model_schema` argument reaches
/// a surface as a literal, so anything else is one no argument can carry.
fn arg_literal<'meta>(meta: &'meta Meta, name: &str, takes: &str) -> syn::Result<&'meta syn::Lit> {
    let Meta::NameValue(name_value) = meta else {
        return Err(arg_rejection(meta, name, takes));
    };
    if let syn::Expr::Lit(expr_lit) = &name_value.value {
        Ok(&expr_lit.lit)
    } else {
        Err(arg_rejection(&name_value.value, name, takes))
    }
}

/// The string literal a `name = "…"` argument was written as.
fn str_arg<'meta>(meta: &'meta Meta, name: &str) -> syn::Result<&'meta syn::LitStr> {
    let takes = format!("a string literal, written `{name} = \"…\"`");
    let lit = arg_literal(meta, name, &takes)?;
    if let syn::Lit::Str(lit_str) = lit {
        Ok(lit_str)
    } else {
        Err(arg_rejection(lit, name, &takes))
    }
}

/// The name a `name = "…"` argument was written as, once it is one a name can be spelled from.
/// Checked here because `Ident::new` panics on a non-identifier with no span — better caught
/// while the literal the author wrote is still in hand.
fn name_arg_value(lit: &syn::LitStr) -> syn::Result<String> {
    let value = lit.value();
    let mut chars = value.chars();
    let opens = chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_');
    if opens && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Ok(value);
    }
    Err(arg_rejection(
        lit,
        "name",
        "a string literal spelling an identifier: ASCII letters, digits and underscores, not \
         opening with a digit. The value names the type on every surface and, snake-cased, the \
         Rust module its schema is published from, so one an identifier cannot be spelled from is \
         a name no surface can carry",
    ))
}

/// The length a `minLength = N` argument was written as, as a `usize` — the type the generated
/// validator compares it against.
fn length_arg(meta: &Meta, name: &str) -> syn::Result<usize> {
    let takes = format!("an integer literal `usize` can hold, written `{name} = 3`");
    let lit = arg_literal(meta, name, &takes)?;
    if let syn::Lit::Int(lit_int) = lit {
        lit_int.base10_parse()
    } else {
        Err(arg_rejection(lit, name, &takes))
    }
}

/// The pairs a `default_types(…)` argument was written as, in that order.
fn default_types_arg(meta: &Meta) -> syn::Result<Vec<(syn::Ident, syn::Type)>> {
    let takes = "a list of `Parameter = Type` pairs, written `default_types(IdType = String, \
                 DateType = f64)`";
    let Meta::List(list) = meta else {
        return Err(arg_rejection(meta, "default_types", takes));
    };
    let entries =
        list.parse_args_with(Punctuated::<DefaultTypeEntry, Token![,]>::parse_terminated)?;
    if entries.is_empty() {
        return Err(arg_rejection(list, "default_types", takes));
    }
    let mut read: Vec<(syn::Ident, syn::Type)> = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some((_, declared)) = read.iter().find(|(name, _)| *name == entry.name) {
            return Err(repeated_entry_rejection(&entry.name, declared, &entry.ty));
        }
        read.push((entry.name, entry.ty));
    }
    Ok(read)
}

/// The refusal a second `default_types` entry for one parameter earns, spanned on the repeat (the
/// first entry declares what its author meant) and naming both fillings.
fn repeated_entry_rejection(
    name: &syn::Ident,
    declared: &syn::Type,
    repeated: &syn::Type,
) -> syn::Error {
    let first = quote!(#declared);
    let second = quote!(#repeated);
    syn::Error::new(
        name.span(),
        format!(
            "`model_schema` argument `default_types` declares `{name}` twice, first as \
             `{first}` and then as `{second}`. A parameter is described from one filling: the \
             JSON-schema document is generated from it, so a second entry would leave which of the \
             two that document describes to whichever way the list happens to be read, and the \
             other silently dropped. Declare `{name}` once, with the type its document should be \
             generated from."
        ),
    )
}

/// Whether a flag argument is set: it stands alone, or takes the boolean literal that says which
/// way to set it.
fn flag_arg(meta: &Meta, name: &str) -> syn::Result<bool> {
    if matches!(*meta, Meta::Path(_)) {
        return Ok(true);
    }
    let takes =
        format!("no value at all, or a boolean literal, written `{name}` or `{name} = false`");
    let lit = arg_literal(meta, name, &takes)?;
    if let syn::Lit::Bool(lit_bool) = lit {
        Ok(lit_bool.value())
    } else {
        Err(arg_rejection(lit, name, &takes))
    }
}

/// Names what an argument takes, spanned on what was written instead.
fn arg_rejection(tokens: impl quote::ToTokens, name: &str, takes: &str) -> syn::Error {
    syn::Error::new_spanned(
        tokens,
        format!("`model_schema` argument `{name}` takes {takes}"),
    )
}

/// Rejects an argument the parser does not read, spanned on the name as written.
fn unknown_arg_rejection(meta: &Meta) -> syn::Error {
    let path = meta.path();
    syn::Error::new_spanned(
        path,
        format!(
            "unknown `model_schema` argument `{}`. This attribute is this crate's own, so an \
             argument it does not read reaches no emitter: the type would be expanded as though \
             the argument had been left off — a misspelled `name` yields the unrenamed schema on \
             every surface. Valid arguments: {}",
            quote!(#path),
            KNOWN_ARGS.join(", ")
        ),
    )
}

/// Records a `pattern` argument in splice-ready spelling, or, when refused, as written alongside
/// what keeps it off every surface — the guards that answer for it read that one was given, not
/// what it says.
fn record_pattern(result: &mut ModelSchemaArgs, lit_str: &syn::LitStr) {
    match portable_pattern(lit_str)
        .and_then(|portable| constraining_pattern(lit_str, portable))
        .and_then(|constraining| emittable_pattern(lit_str, constraining))
    {
        Ok(pattern) => result.pattern = Some(pattern),
        Err(rejection) => {
            result.pattern_rejection = Some(rejection);
            result.pattern = Some(lit_str.value());
        }
    }
}

pub fn exec_model_schema(args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_args = parse_model_schema_args(args);
    // Written out rather than `parse_macro_input!`, which returns from the enclosing function and
    // so forces its return type to be `proc_macro::TokenStream`. This is the match it expands to.
    let item = match syn::parse2::<Item>(input) {
        Ok(item) => item,
        Err(rejection) => return rejection.to_compile_error(),
    };
    // An argument the parser refused describes a surface no expansion below can honour, so the
    // item is refused before it is dispatched to its shape.
    if let Some(rejection) = parsed_args.arg_rejection.as_ref()
        && let Some(output) =
            refused_item_output(&item, &[attr_guard_error(rejection, &item_label(&item))])
    {
        return output;
    }
    // An item's read holds the hooks of its own members alone.
    #[cfg(feature = "serde")]
    judged_read::forget_member_hooks();
    // Read in every feature combination: a transport, and a type that lists `"mongodb"` for the
    // keys of its maps, read it whether or not a surface is on.
    record_wire_form(&item);
    // A member that holds any JSON value is no shape a target can describe, in any build.
    if let Some(output) = refused_item_output(&item, &any_json_errors(&item, &parsed_args)) {
        return output;
    }
    // A `default_types` declaration is read against the item's own parameters, which the parser
    // never sees, so both directions are answered here, ahead of every shape.
    if let Some(output) =
        refused_item_output(&item, &default_types_guard_errors(&item, &parsed_args))
    {
        return output;
    }
    // A list-form serde rename whose two directions name two keys is refused here, at the one seam
    // a container, a variant and a member are all reachable from, and so is a transparent struct
    // whose two derives take two different fields as its value.
    // A default of a variant's member that no target but Rust reads as serde does is refused
    // beside it, every such member of the item at once.
    if let Some(output) = refused_item_output(&item, &serde_attribute_guard_errors(&item)) {
        return output;
    }
    // A map flattened beside a member that writes a key, or filling a type parameter flattened
    // beside one, shares one level of the object with it, which no TypeScript type describes, and
    // serde reads no entry of a map keyed by a number or a `bool` below a flattened member or in an
    // enum it buffers, so both are refused here, in every build with serde.
    #[cfg(feature = "serde")]
    if let Some(output) = refused_item_output(&item, &flattened_map_errors(&item, &parsed_args)) {
        return output;
    }
    // `Foo()` and `Foo` are two wire shapes to serde and one to every surface, in every feature
    // combination, so it is refused here, ahead of every shape.
    if let Some(output) = refused_item_output(&item, &empty_tuple_variant_errors(&item)) {
        return output;
    }
    // An enum serde writes and reads no variant of has no value to describe, in any build.
    if let Some(output) = refused_item_output(&item, &no_variant_on_the_wire_errors(&item)) {
        return output;
    }
    // A doc example is compiled at one instantiation, and a const parameter takes no filling, so an
    // item writing both is refused here, ahead of every shape.
    if let Some(output) = refused_item_output(&item, &const_parameter_example_errors(&item)) {
        return output;
    }
    // A const handed to a written type as an argument stands where a type belongs, and no surface
    // can spell it: refused here, ahead of every shape.
    if let Some(output) = refused_item_output(&item, &const_parameter_argument_errors(&item)) {
        return output;
    }
    // A name the `indexmap` feature reads is no type this build describes, so it is refused here,
    // ahead of every shape, with the feature named.
    if let Some(output) = refused_item_output(&item, &indexmap_feature_errors(&item)) {
        return output;
    }
    // No key written on a brand's slot reaches a surface, so it is refused here, at the seam every
    // build shares. The item is re-emitted with the attribute taken off.
    if let Some(output) = refused_item_output(&item, &branded_slot_prop_errors(&item)) {
        return output;
    }
    // A `preprocess` the read of the item cannot call is refused here, in every build.
    if let Some(output) = preprocess_refusal_output(&item, &parsed_args) {
        return output;
    }
    // `decode_with` generates methods on the type it is written on, and `"archie"` impls for it, so
    // a shape that cannot carry them is refused here rather than left with plain serde.
    if let Some(output) = refused_item_output(&item, &enable_guard_errors(&item, &parsed_args)) {
        return output;
    }
    // A name another declaration already publishes is refused here, after every guard that refuses
    // the item outright: one that publishes nothing claims nothing.
    if let Some(output) =
        refused_item_output(&item, &published_name_collision_errors(&item, &parsed_args))
    {
        return output;
    }
    // Which Zod binding this item publishes turns on whether it declares type parameters, and a
    // field written at the item's own name reads that answer back the way any other reference does.
    record_item(&item);
    record_item_functions(&item, &parsed_args);
    // Whether a filling satisfies the bounds its parameter declares, and whether a range written by
    // its bare name is the std one, are asked of the compiler: no proc macro can answer either.
    let filling_bound_checks = asked_of_the_compiler(&item, &parsed_args);
    // Held across the dispatch that consumes the item: a constrained brand written above left its
    // consult for whichever expansion registers the name.
    let registering = item_schema_ident(&item).cloned();
    // Independent of the dispatch below: it reads its own borrow of `item` ahead of the move, and
    // needs none of the machinery the other surfaces share.
    let dart_tokens = dart_suffix_tokens(&item, &parsed_args);
    // Same independence as the Dart tokens above; the refusal walks the item's own fields ahead
    // of the move too, since it names each one by its declared type.
    let swift_tokens = swift_suffix_tokens(&item, &parsed_args);
    let swift_refusals = swift_width_refusals(&item);
    // Same independence as the Dart and Swift tokens above.
    let kotlin_tokens = kotlin_suffix_tokens(&item, &parsed_args);
    let expanded = process_item(item, &parsed_args);
    let deferred = with_prefixed_tokens(expanded, &deferred_shape_refusals(registering.as_ref()));
    let bound = with_prefixed_tokens(deferred, &filling_bound_checks);
    let width_checked = with_prefixed_tokens(bound, &swift_refusals);
    quote! { #width_checked #dart_tokens #swift_tokens #kotlin_tokens }
}

/// The expansion of `item` by its shape, or the refusal of a string check its shape cannot carry.
/// The item's expansion, and after it the conversions `enable = ["archie"]` writes beside the type.
fn process_item(item: Item, args: &ModelSchemaArgs) -> TokenStream {
    let archie = archie_conversions(&item, args);
    let expanded = process_shape(item, args);
    quote! { #expanded #archie }
}

/// The expansion of each shape an item can take.
fn process_shape(item: Item, args: &ModelSchemaArgs) -> TokenStream {
    if let Item::Struct(item_struct) = item {
        string_check_refusal_output(&item_struct, args)
            .unwrap_or_else(|| process_struct(item_struct, args))
    } else if let Item::Enum(item_enum) = item {
        enum_string_check_refusal_output(&item_enum, args)
            .unwrap_or_else(|| process_enum(item_enum, args))
    } else if let Item::Type(item_type) = item {
        alias_string_check_refusal_output(&item_type, args)
            .or_else(|| checks::alias_refusal_output(&item_type, &args.check))
            .or_else(|| preprocess::alias_refusal_output(&item_type, &args.preprocess))
            .unwrap_or_else(|| process_type_alias(item_type, args))
    } else {
        syn::Error::new_spanned(
            item,
            prefixed_guard_message("unsupported target for this attribute"),
        )
        .to_compile_error()
    }
}

/// The Dart tokens `item` earns, or nothing without the `dart` feature — always callable, so the
/// tokens that follow the item's own expansion never need a `#[cfg]`-gated `let` of their own.
/// The backend is handed what it reads of `args`: the name, the checks, the `preprocess`, the
/// string check of a brand or of an enum, and each member's keys as [`member_prop_meta`] holds
/// them.
#[cfg(feature = "dart")]
fn dart_suffix_tokens(item: &Item, args: &ModelSchemaArgs) -> TokenStream {
    // An enum's own string check is the one that reaches its members.
    let reaching = matches!(item, Item::Enum(_)).then_some(args);
    dart_schema_dispatch(
        item,
        &DartTypeArgs {
            check: &args.check,
            default_types: &args.default_types,
            max_length: args.max_length,
            member_meta: &|member| member_prop_meta(member, reaching),
            min_length: args.min_length,
            name_override: args.name_override.as_deref(),
            pattern: args.pattern.as_deref(),
            preprocess: &args.preprocess,
        },
    )
}

#[cfg(not(feature = "dart"))]
fn dart_suffix_tokens(_item: &Item, _args: &ModelSchemaArgs) -> TokenStream {
    TokenStream::new()
}

/// The Swift tokens `item` earns, or nothing without the `swift` feature — the same always-
/// callable shape as [`dart_suffix_tokens`]. The emitter is handed the arguments it reads: the
/// name, the checks, the functions a `preprocess` names, a brand's or an enum's string check,
/// and each member's own bounds.
#[cfg(feature = "swift")]
fn swift_suffix_tokens(item: &Item, args: &ModelSchemaArgs) -> TokenStream {
    let enum_args = matches!(item, Item::Enum(_)).then_some(args);
    swift_schema_dispatch(
        item,
        &SwiftTypeArgs {
            check: &args.check,
            default_types: &args.default_types,
            max_length: args.max_length,
            member_meta: &|member| member_prop_meta(member, enum_args),
            min_length: args.min_length,
            name_override: args.name_override.as_deref(),
            pattern: args.pattern.as_deref(),
            preprocess: &args.preprocess,
        },
    )
}

#[cfg(not(feature = "swift"))]
fn swift_suffix_tokens(_item: &Item, _args: &ModelSchemaArgs) -> TokenStream {
    TokenStream::new()
}

/// The Kotlin tokens `item` earns, or nothing without the `kotlin` feature — the same always-
/// callable shape as [`dart_suffix_tokens`]. The emitter is handed the arguments it reads, the
/// type's `preprocess` among them, and a member's bounds as every surface reads them: an enum's
/// own string check is each reached member's as well.
#[cfg(feature = "kotlin")]
fn kotlin_suffix_tokens(item: &Item, args: &ModelSchemaArgs) -> TokenStream {
    let enum_args = matches!(item, Item::Enum(_)).then_some(args);
    kotlin_schema_dispatch(
        item,
        &KotlinArgs {
            check: &args.check,
            default_types: &args.default_types,
            max_length: args.max_length,
            member_meta: &|member| member_prop_meta(member, enum_args),
            min_length: args.min_length,
            name_override: args.name_override.as_deref(),
            pattern: args.pattern.as_deref(),
            preprocess: &args.preprocess,
        },
    )
}

#[cfg(not(feature = "kotlin"))]
fn kotlin_suffix_tokens(_item: &Item, _args: &ModelSchemaArgs) -> TokenStream {
    TokenStream::new()
}

/// Classifies what an alias resolves to, for the registry.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn alias_target_kind(alias_field_def: &FieldDef) -> AliasKind {
    match map_key_path(alias_field_def) {
        MapKeyPath::Open => AliasKind::StringWire,
        MapKeyPath::Unnarrowed => AliasKind::Stringified,
        MapKeyPath::Refused(_) => AliasKind::NoEnumMembers,
        MapKeyPath::Enumerated(target_name) => {
            registered_key_kind(target_name).unwrap_or(AliasKind::Unknown)
        }
    }
}

/// The `compile_error!` tokens an alias whose target reaches a map key with no `enum_members()`
/// earns, or `None` when it reaches none. Spanned on the target, which is where the key was
/// written.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn alias_map_key_guard_error(
    alias: &ItemType,
    alias_field_def: &FieldDef,
) -> Option<proc_macro2::TokenStream> {
    let rejection = map_key_rejection(alias_field_def)?;
    let subject = format!("type alias `{}`", alias.ident);
    let message = prefixed_guard_message(&map_key_rejection_message(&subject, &rejection));
    Some(syn::Error::new_spanned(&alias.ty, message).to_compile_error())
}

/// The `compile_error!` tokens an alias whose target reaches a std type serde has no wire form for
/// earns, or `None` when it reaches none. Spanned on the target, which is where it was written.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn alias_undescribable_std_error(
    alias: &ItemType,
    alias_field_def: &FieldDef,
) -> Option<proc_macro2::TokenStream> {
    let rejection = undescribable_std_rejection(alias_field_def)?;
    let subject = format!("type alias `{}`", alias.ident);
    let message = prefixed_guard_message(&undescribable_std_message(&subject, &rejection));
    Some(syn::Error::new_spanned(&alias.ty, message).to_compile_error())
}

/// Whether serde writes a value of `def` as the bare name of a plain enum's variant, or as an
/// `Option` of one: `def` names a type the registry records as one, itself, under an `Option`, or
/// at what fills a generic type declared above.
#[cfg(feature = "serde")]
fn written_as_a_plain_enum(def: &FieldDef) -> bool {
    let bare = without_its_option(def);
    if let Some(filling) = value_filling(&bare) {
        return written_as_a_plain_enum(filling);
    }
    let FieldDefType::SiblingType(name, _) = &bare.field_type else {
        return false;
    };
    bare.array_depth == 0 && is_recorded_plain_enum(name)
}

/// The alias schema module is referenced by all three schema features — `typescript` and `zod`
/// through the export name, `jsonschema` through `#module_ident::Schema::json_schema()` — so the
/// module and its `register_alias_info` call are gated on the union, not `typescript` alone.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn process_type_alias(item_type: ItemType, args: &ModelSchemaArgs) -> TokenStream {
    let mut alias = item_type;
    alias
        .attrs
        .retain(|attr| !attr.path().is_ident("model_schema"));

    let rust_ident = alias.ident.clone();
    let rust_ident_str = rust_ident.to_string();
    let export_name = compute_alias_export_name(&rust_ident_str, args.name_override.as_deref());
    let module_name = ident_schema_module_name(&rust_ident_str);
    let module_ident = Ident::new(&module_name, rust_ident.span());

    // Registered only once the target has been classified: the alias's own expansion is the only
    // place that still holds the aliased type's tokens.
    let alias_field_def = get_field_def(export_name.as_str(), &alias.ty, "");
    let kind = alias_target_kind(&alias_field_def);
    register_alias_info(&rust_ident_str, &export_name, &module_name, kind);
    // An alias *is* its target, so a key written under its name renders in the target's wire form.
    record_key_wire(&rust_ident_str, alias_field_def.map_key_wire());
    // An alias's schema *is* its target's, so it publishes whatever the target publishes.
    Surface::written(&alias_field_def, &type_parameters_in_scope(&alias.generics))
        .record(&rust_ident_str);
    // An alias *is* the type it names, so a merge that flattens it multiplies over exactly the
    // members the target's own name would have given it.
    #[cfg(feature = "zod")]
    record_zod_union_members(&rust_ident_str, &alias_field_def.zod_union_members());

    // Registered above whatever the outcome, so a type naming a refused alias still resolves to the
    // export name the author wrote and the alias's own diagnostic stays the one they act on.
    let alias_guard_errors: Vec<proc_macro2::TokenStream> =
        alias_undescribable_std_error(&alias, &alias_field_def)
            .into_iter()
            .chain(alias_map_key_guard_error(&alias, &alias_field_def))
            .collect();
    if let Some(output) = guard_failure_output(&alias, Some(&alias.ident), &alias_guard_errors) {
        return output;
    }

    let ts_method = generate_alias_ts_definition_method(&alias, &export_name, &alias_field_def);
    let json_schema_method =
        generate_alias_json_schema_method(&alias, &export_name, &alias_field_def, args);
    #[cfg(feature = "jsonschema")]
    let unit_member = unit_member_record_item(&Item::Type(alias.clone()));
    #[cfg(not(feature = "jsonschema"))]
    let unit_member = proc_macro2::TokenStream::new();
    let zod_method = generate_alias_zod_method(
        &alias,
        &export_name,
        &rust_ident_str,
        &alias_field_def,
        &args.default_types,
    );

    quote! {
        #alias

        pub mod #module_ident {
            use super::*;

            #[non_exhaustive]
            pub struct Schema;

            impl Schema {
                #ts_method
                #json_schema_method
                #unit_member
                #zod_method
            }
        }
    }
}

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
fn process_type_alias(item_type: ItemType, _args: &ModelSchemaArgs) -> TokenStream {
    let mut alias = item_type;
    alias
        .attrs
        .retain(|attr| !attr.path().is_ident("model_schema"));
    quote! { #alias }
}

/// Whether `attrs` carries `#[serde(untagged)]`, read off the raw tokens rather than through
/// [`parse_serde_type_attributes`] — [`record_untagged_enum_item`] feeds a registry that has to
/// answer the same way in every feature combination.
fn has_serde_untagged(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            let mut found = false;
            let _: syn::Result<()> = attr.parse_nested_meta(|nested| {
                if nested.path.is_ident("untagged") {
                    found = true;
                }
                Ok(())
            });
            if found {
                return true;
            }
        }
    }
    false
}

/// Builds the `JSDoc` comment body (lines prefixed with ` * `) for an item, field, or enum
/// variant. The no-docs fallback names what is exported, not as declared in Rust, and a
/// ` ```rust example ` block is dropped as unreadable from a TypeScript comment.
fn build_jsdoc_body(docs_vec: Option<&[String]>, fallback_name: &str) -> String {
    item_jsdoc_body(&item_lines_or_name(docs_vec, fallback_name, |doc_lines| {
        strip_examples_from_docs(doc_lines)
            .iter()
            .flat_map(|v| v.lines().map(ToOwned::to_owned).collect::<Vec<_>>())
            .collect()
    }))
}

/// The `schema_examples()` and `schema_example()` an enum's type publishes, read off its docs. One
/// seam for all five enum shapes, which differ in how their variants are written and not in what
/// the type beside them exposes.
#[cfg(feature = "zod")]
fn enum_schema_example_method(
    attrs: &[syn::Attribute],
    name: &syn::Ident,
    generics: &syn::Generics,
    args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    item_schema_example_method(&extract_examples_tokens(attrs), name, generics, args)
}

#[cfg(all(
    not(feature = "zod"),
    any(feature = "typescript", feature = "jsonschema")
))]
const fn enum_schema_example_method(
    _attrs: &[syn::Attribute],
    _name: &syn::Ident,
    _generics: &syn::Generics,
    _args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    None
}

/// The type a doc example's value is annotated with: the item's own name, with every type parameter
/// it declares instantiated at the type `default_types` declares for it, or at `String` where it
/// declares none.
#[cfg(feature = "zod")]
fn schema_example_value_type(
    name: &syn::Ident,
    generic_params: &[String],
    default_types: &[(syn::Ident, syn::Type)],
) -> proc_macro2::TokenStream {
    if generic_params.is_empty() {
        return quote! { #name };
    }
    let args = generic_params.iter().map(|param| {
        default_types
            .iter()
            .find(|(declared, _)| declared == param.as_str())
            .map_or_else(
                || quote! { ::std::string::String },
                |(_, ty)| quote! { #ty },
            )
    });
    quote! { #name<#(#args),*> }
}

/// The type-level `schema_examples()` a declared item publishes, one value per ` ```rust example `
/// block and each built in a scope of its own, with `schema_example()` answering the first. `None`
/// where there is nothing to build one from — no example written, or no `zod`, the only surface
/// that reads one.
#[cfg(feature = "zod")]
fn item_schema_example_method(
    example_tokens: &[proc_macro2::TokenStream],
    name: &syn::Ident,
    generics: &syn::Generics,
    args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    schema_examples_methods(
        example_tokens,
        &schema_example_value_type(
            name,
            &type_parameters_in_scope(generics),
            &args.default_types,
        ),
    )
}

/// `schema_examples()` and `schema_example()` over the blocks a type writes, each block's value
/// annotated `value_ty`, or `None` where it writes none.
///
/// A later block is held to the type as the first is:
///
/// ```rust,compile_fail
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// /// A sample
/// /// ```rust example
/// /// Sample { value: 1 }
/// /// ```
/// /// ```rust example
/// /// "not a sample"
/// /// ```
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// pub struct Sample {
///     pub value: u32,
/// }
///
/// fn main() {}
/// ```
///
/// Compiled standalone as an ordinary test file, this is the only error it earned:
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:9:1
///    |
///  9 | /// "not a sample"
///    | ^^^^^^^^^^^^^^^^^^ expected `Sample`, found `&str`
/// 10 | /// ```
/// 11 | #[model_schema()]
///    | ----------------- in this attribute macro expansion
/// ```
#[cfg(feature = "zod")]
fn schema_examples_methods(
    example_tokens: &[proc_macro2::TokenStream],
    value_ty: &proc_macro2::TokenStream,
) -> Option<proc_macro2::TokenStream> {
    if example_tokens.is_empty() {
        return None;
    }
    let blocks = example_tokens.iter().map(|code_tokens| {
        quote! {
            {
                let value: #value_ty = {
                    #code_tokens
                };
                serde_json::to_value(&value).unwrap()
            }
        }
    });
    Some(quote! {
        pub fn schema_examples() -> ::std::vec::Vec<serde_json::Value> {
            ::std::vec![#(#blocks),*]
        }

        pub fn schema_example() -> serde_json::Value {
            Self::schema_examples().swap_remove(0)
        }
    })
}

#[cfg(all(
    not(feature = "zod"),
    any(feature = "typescript", feature = "jsonschema")
))]
const fn item_schema_example_method(
    _example_tokens: &[proc_macro2::TokenStream],
    _name: &syn::Ident,
    _generics: &syn::Generics,
    _args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    None
}

/// Builds the delegating impl methods (on the type itself) that forward to its schema module.
#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
fn build_struct_delegate_items(
    module_ident: &Ident,
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    schema_example_method: Option<&proc_macro2::TokenStream>,
) -> Vec<proc_macro2::TokenStream> {
    // The example methods are emitted iff a block was extracted.
    #[cfg(feature = "zod")]
    let examples_json = zod_examples_json(schema_example_method.is_some());
    #[cfg(feature = "zod")]
    let reexport = zod_binding_reexport(rust_ident, item_name, parameters);
    #[cfg(not(feature = "zod"))]
    let _: &_ = &(item_name, rust_ident, parameters);
    #[cfg(not(feature = "zod"))]
    let _: Option<&proc_macro2::TokenStream> = schema_example_method;

    let mut items: Vec<proc_macro2::TokenStream> = Vec::new();

    #[cfg(feature = "jsonschema")]
    items.push(quote! {
        pub fn json_schema() -> serde_json::Value {
            #module_ident::Schema::json_schema()
        }
    });

    #[cfg(feature = "typescript")]
    items.push(quote! {
        pub fn ts_definition() -> ::std::string::String {
            #module_ident::Schema::ts_definition()
        }
    });

    #[cfg(feature = "zod")]
    items.push(zod_schema_with_examples(
        &quote! { #module_ident::Schema::zod_schema() },
        item_name,
        parameters,
        &reexport,
        &examples_json,
    ));

    #[cfg(feature = "zod")]
    items.extend(schema_example_method.cloned());
    items
}

/// Assembles the final macro output for a struct or enum: the item itself, its schema module
/// (with the per-field validation functions and the items `decode_with` adds), and the type's
/// `impl`, which holds its delegates and its `validate()` when it has one.
#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
fn assemble_schema_output<T>(
    parts: &SchemaOutputParts<T>,
    decode_with_items: &TokenStream,
) -> TokenStream
where
    T: quote::ToTokens,
{
    let (impl_generics, type_generics, where_clause) = parts.generics.split_for_impl();
    let item = parts.item;
    let module_ident = parts.module_ident;
    let name = parts.name;
    let schema_impl_items = parts.schema_impl_items;
    let validation_fns = parts.validation_fns;
    let delegate_impl_items = parts.delegate_impl_items;
    let validate_method = parts.validate_method;
    let module_items = schema_module_items(
        quote! {
            #[non_exhaustive]
            pub struct Schema;

            impl Schema {
                #(#schema_impl_items)*
            }

            #(#validation_fns)*
        },
        decode_with_items,
    );

    let output = quote! {
        #item

        pub mod #module_ident {
            use super::*;

            #module_items
        }

        impl #impl_generics #name #type_generics #where_clause {
            #(#delegate_impl_items)*
            #validate_method
        }
    };

    log::trace!("{output}");

    output
}

/// What a schema module holds: `held`, and beside it the items `decode_with` adds. Those items
/// are named `Issue`, `Path` and so on, which `held` then reads from the author's scope outright.
#[cfg(all(
    feature = "serde",
    any(feature = "zod", feature = "typescript", feature = "jsonschema")
))]
fn schema_module_items(held: TokenStream, decode_with_items: &TokenStream) -> TokenStream {
    if decode_with_items.is_empty() {
        return held;
    }
    let scoped = reading_the_authors_scope(held);
    quote! {
        #scoped
        #decode_with_items
    }
}

/// Without `serde` no item carries anything of `decode_with`'s.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "zod", feature = "typescript", feature = "jsonschema")
))]
const fn schema_module_items(held: TokenStream, _decode_with_items: &TokenStream) -> TokenStream {
    held
}

/// The type-level `validate()` a struct publishes: the aggregate of its per-field validators, or
/// `None` where there is nothing to run — no constrained field, or no `serde` to have generated the
/// validators from.
///
/// It is written in the type's own `impl`, so a type that declares a parameter has it at every
/// filling. This builds:
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(default_types(CodeType = String))]
/// #[derive(Deserialize, Serialize)]
/// pub struct Crate<CodeType> {
///     pub codes: Vec<CodeType>,
///     #[model_schema_prop(minLength = 1)]
///     pub label: String,
/// }
///
/// fn main() {
///     let unlabelled = Crate::<u32> { codes: vec![], label: String::new() };
///     assert_eq!(
///         unlabelled.validate().unwrap_err(),
///         ["'label': too short: minimum length is 1, got 0"]
///     );
/// }
/// ```
///
/// The run below adds a `validate` of the consumer's own on one filling:
///
/// ```rust,compile_fail
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(default_types(CodeType = String))]
/// #[derive(Deserialize, Serialize)]
/// pub struct Crate<CodeType> {
///     pub codes: Vec<CodeType>,
///     #[model_schema_prop(minLength = 1)]
///     pub label: String,
/// }
///
/// impl Crate<u32> {
///     pub fn validate(&self) -> Result<(), Vec<String>> {
///         if self.codes.contains(&0) {
///             return Err(vec!["'codes': no code is zero".to_owned()]);
///         }
///         Ok(())
///     }
/// }
///
/// fn main() {}
/// ```
///
/// A `compile_fail` doctest asserts only that some error was raised, so the snippet was compiled
/// standalone as an ordinary test file, and this is the only error it earned:
///
/// ```text
/// error[E0592]: duplicate definitions with name `validate`
///   --> tests/zz_probe.rs:4:1
///    |
///  4 | #[model_schema(default_types(CodeType = String))]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ duplicate definitions for `validate`
/// ...
/// 13 |     pub fn validate(&self) -> Result<(), Vec<String>> {
///    |     ------------------------------------------------- other definition for `validate`
/// ```
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn struct_validate_method(
    validate_bodies: &[proc_macro2::TokenStream],
    module_ident: &Ident,
    checks: &checks::ItemChecks<'_>,
) -> Option<proc_macro2::TokenStream> {
    validate_method(validate_bodies, Some(module_ident), checks)
}

/// Without `serde` no bound is held, and `validate()` runs the checks alone.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn struct_validate_method(
    validate_bodies: &[proc_macro2::TokenStream],
    module_ident: &Ident,
    checks: &checks::ItemChecks<'_>,
) -> Option<proc_macro2::TokenStream> {
    validate_method(validate_bodies, Some(module_ident), checks)
}

/// Builds the type-level `validate()`: what each member adds, in the order declared, then the
/// type's own checks, in the order written. `None` where there is nothing to run. A member's
/// bound calls the `validate_{field}_value` of the schema module, which is `module_ident`.
///
/// A check is called by its name alone, which is read where the type is declared. So where the
/// item names one, no import of the schema module is in scope of the call: each bound brings the
/// module in for itself.
fn validate_method(
    validate_bodies: &[proc_macro2::TokenStream],
    module_ident: Option<&proc_macro2::Ident>,
    checks: &checks::ItemChecks<'_>,
) -> Option<proc_macro2::TokenStream> {
    let own = checks.own_calls(VALIDATE_SINK);
    if validate_bodies.is_empty() && own.is_empty() {
        return None;
    }
    let imported = module_ident
        .filter(|_| !checks.named)
        .map(|module| quote! { use #module::*; });
    Some(quote! {
        /// Validates all constrained fields and returns all validation errors.
        ///
        /// Returns `Ok(())` if all constraints pass, or `Err(Vec<String>)` with all errors.
        pub fn validate(&self) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
            #imported
            let mut errors: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
            #(#validate_bodies)*
            #(#own)*
            if errors.is_empty() { ::core::result::Result::Ok(()) } else { ::core::result::Result::Err(errors) }
        }
    })
}

/// [`validate_method`] in a build with no schema surface, where no bound is held and a type
/// publishes `validate()` only for the checks it names, on itself or on a member.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
fn checks_validate_impl(
    name: &syn::Ident,
    generics: &syn::Generics,
    validate_bodies: &[proc_macro2::TokenStream],
    checks: &checks::ItemChecks<'_>,
) -> proc_macro2::TokenStream {
    let method = checks
        .named
        .then(|| validate_method(validate_bodies, None, checks))
        .flatten();
    let Some(validate) = method else {
        return proc_macro2::TokenStream::new();
    };
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #validate
        }
    }
}

/// The pattern one variant is matched by in the enum-level `validate()`. Only the constrained
/// members are bound, each under the name its check reads; everything else is left unread.
pub fn variant_check_pattern(
    variant_ident: &proc_macro2::Ident,
    kind: &VariantKind,
    total_fields: usize,
    bound: &[BoundMember],
) -> proc_macro2::TokenStream {
    if bound.is_empty() {
        return match *kind {
            VariantKind::Unit => quote! { Self::#variant_ident },
            VariantKind::TupleSingle | VariantKind::TupleMultiple => {
                quote! { Self::#variant_ident(..) }
            }
            VariantKind::Named if total_fields == 0 => quote! { Self::#variant_ident {} },
            VariantKind::Named => quote! { Self::#variant_ident { .. } },
        };
    }
    if matches!(*kind, VariantKind::TupleSingle | VariantKind::TupleMultiple) {
        let slots = (0..total_fields).map(|index| {
            bound
                .iter()
                .find(|member| member.index == index)
                .map_or_else(
                    || quote! { _ },
                    |member| {
                        let binding = &member.binding;
                        quote! { #binding }
                    },
                )
        });
        return quote! { Self::#variant_ident(#(#slots),*) };
    }
    // One iterator, not a name list zipped against a binding list, so the two cannot come out of
    // step: a member with no name is a positional slot.
    let members = bound.iter().filter_map(|member| {
        let field = member.named.as_ref()?;
        let binding = &member.binding;
        Some(quote! { #field: #binding })
    });
    if bound.len() == total_fields {
        quote! { Self::#variant_ident { #(#members),* } }
    } else {
        quote! { Self::#variant_ident { #(#members,)* .. } }
    }
}

/// The match arms the enum-level `validate()` runs — empty when no variant carries a constrained
/// member, which is the parity a constraint-free struct has: it publishes no `validate()` either.
pub fn build_member_check_arms(
    per_variant: Vec<(proc_macro2::TokenStream, Vec<proc_macro2::TokenStream>)>,
) -> Vec<proc_macro2::TokenStream> {
    let mut arms: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut unchecked: Vec<proc_macro2::TokenStream> = Vec::new();
    for (pattern, checks) in per_variant {
        if checks.is_empty() {
            unchecked.push(pattern);
        } else {
            arms.push(quote! { #pattern => { #(#checks)* } });
        }
    }
    if arms.is_empty() {
        return Vec::new();
    }
    if !unchecked.is_empty() {
        arms.push(quote! { #(#unchecked)|* => {} });
    }
    arms
}

/// The `match` an enum's `validate()` runs its members' arms in, as the one thing its members add,
/// or nothing where no variant holds a member to check.
fn matched_arms(arms: &[proc_macro2::TokenStream]) -> Vec<proc_macro2::TokenStream> {
    if arms.is_empty() {
        return Vec::new();
    }
    vec![quote! {
        match self {
            #(#arms),*
        }
    }]
}

/// Builds the type-level `validate()` method for an enum: the checks of whichever variant the
/// value holds, then the enum's own, or `None` where there is nothing to run.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn build_enum_validate_method(
    arms: &[proc_macro2::TokenStream],
    module_ident: &Ident,
    checks: &checks::ItemChecks<'_>,
) -> Option<proc_macro2::TokenStream> {
    validate_method(&matched_arms(arms), Some(module_ident), checks)
}

/// Computes the TypeScript types and Zod schemas contributed by a struct's `#[serde(flatten)]`
/// fields, each beside the answer for whether the object writes those members at all (an empty
/// vector for either disabled output feature).
#[cfg(any(feature = "typescript", feature = "zod"))]
fn compute_flatten_outputs(
    flattened_fields: &[FieldDef],
) -> (Vec<MergedOperand>, Vec<MergedOperand>) {
    #[cfg(feature = "typescript")]
    let ts_types = flattened_fields
        .iter()
        .map(|fld| MergedOperand {
            absence: SourceAbsence::written(fld),
            #[cfg(feature = "zod")]
            branches: Vec::new(),
            #[cfg(feature = "zod")]
            entries: false,
            spelling: flattened_ts_spelling(fld),
        })
        .collect();
    #[cfg(not(feature = "typescript"))]
    let ts_types = Vec::new();

    #[cfg(feature = "zod")]
    let zod_schemas = flattened_fields
        .iter()
        .map(|fld| MergedOperand {
            absence: SourceAbsence::written(fld),
            branches: flattened_zod_branches(fld),
            entries: flattened_as_entries(fld),
            spelling: fld.zod_merged_schema(),
        })
        .collect();
    #[cfg(not(feature = "zod"))]
    let zod_schemas = Vec::new();

    (ts_types, zod_schemas)
}

/// Whether the registration a flattened source names offers its own absence, which is the second
/// key set the merge owes it.
#[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
fn flattened_name_offers_absence(fld: &FieldDef) -> bool {
    if fld.is_array() {
        return false;
    }
    let FieldDefType::SiblingType(name, _) = &fld.field_type else {
        return false;
    };
    lookup_alias_info(name).is_some_and(|info| info.wire.iter().any(WireLeaf::is_published_absence))
}

/// No source offers one where nothing reads `#[serde(flatten)]`: without `serde` no field reaches
/// the merge to begin with, and the registry records no wire for a name to publish an absence in.
#[cfg(all(any(feature = "zod", feature = "typescript"), not(feature = "serde")))]
const fn flattened_name_offers_absence(_fld: &FieldDef) -> bool {
    false
}

/// Whether serde reads a flattened source as the entries of a map, as the refusal of a map beside
/// a member reads it. serde reads an `Option` of it as `None` wherever one entry does not read as
/// the map, and leaves every entry in the object.
#[cfg(all(feature = "serde", any(feature = "zod", feature = "jsonschema")))]
fn flattened_as_entries(fld: &FieldDef) -> bool {
    ranges::def_writes_a_map(fld)
}

/// No source is read as entries where nothing reads `#[serde(flatten)]`.
#[cfg(all(any(feature = "zod", feature = "jsonschema"), not(feature = "serde")))]
const fn flattened_as_entries(_fld: &FieldDef) -> bool {
    false
}

/// The operands an object joins one per branch for a flattened source, and none for a source that
/// contributes one key set and is joined as the one operand it is.
#[cfg(all(feature = "serde", feature = "zod"))]
fn flattened_zod_branches(fld: &FieldDef) -> Vec<String> {
    // The binding of a type that names a `preprocess` is the one schema that calls it, and the
    // one text that replaces the placeholder of a member's default.
    if fld.zod_read_whole() {
        return Vec::new();
    }
    let members = fld.zod_union_members();
    if !members.is_empty() {
        return members.into_iter().map(|member| member.spelling).collect();
    }
    fld.flatten_variants()
        .into_iter()
        .map(|variant| variant.zod)
        .collect()
}

/// No source names branches where nothing reads `#[serde(flatten)]`, for the reason no source offers
/// an absence there.
#[cfg(all(feature = "zod", not(feature = "serde")))]
fn flattened_zod_branches(fld: &FieldDef) -> Vec<String> {
    if fld.zod_read_whole() {
        return Vec::new();
    }
    fld.zod_union_members()
        .into_iter()
        .map(|member| member.spelling)
        .collect()
}

/// What one flattened source is written as where the TypeScript merge names it: the parenthesised
/// union of the key sets the choice it names recorded for this position, and the name itself
/// everywhere else.
#[cfg(all(feature = "serde", feature = "typescript"))]
fn flattened_ts_spelling(fld: &FieldDef) -> String {
    let named = fld.typescript_merged_typename();
    // A function the field names changes no type: its enum's variants are spliced in either way.
    let variants: Vec<String> = fld
        .flatten_variants()
        .into_iter()
        .chain(fld.flatten_variants_written_around())
        .map(|variant| variant.typescript)
        .collect();
    if !variants.is_empty() {
        return if variants.len() > 1 || named_wire_leaves(fld, surfaced_leaves).is_some() {
            format!("({})", variants.join(" | "))
        } else {
            named
        };
    }
    let members = fld.ts_union_members();
    if members.is_empty() {
        named
    } else {
        format!("({})", members.join(" | "))
    }
}

/// No name proves a branch where nothing reads `#[serde(flatten)]`: the registry records no wire for
/// one to prove it in, and no field reaches the merge to begin with.
#[cfg(all(feature = "typescript", not(feature = "serde")))]
fn flattened_ts_spelling(fld: &FieldDef) -> String {
    fld.typescript_merged_typename()
}

/// The schema methods a struct's module publishes — the JSON-schema document, the TypeScript
/// definition, the Zod schema — each built only where its feature is on.
#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
fn struct_schema_impl_items(
    field_defs: Vec<FieldDef>,
    flattened_fields: &[FieldDef],
    item_name: &str,
    rust_ident: &str,
    generics: &syn::Generics,
    args: &ModelSchemaArgs,
    docs: &str,
) -> Vec<proc_macro2::TokenStream> {
    #[cfg(not(feature = "typescript"))]
    let _: &str = docs;
    #[cfg(not(any(feature = "typescript", feature = "zod")))]
    let _: &str = rust_ident;
    #[cfg(not(feature = "jsonschema"))]
    let _: &_ = &args;
    #[cfg(feature = "typescript")]
    let ts_generics = ts_generic_params(generics);
    #[cfg(feature = "zod")]
    let type_parameters = type_parameters_in_scope(generics);
    #[cfg(feature = "typescript")]
    let own_keys: Vec<String> = field_defs.iter().map(|fld| fld.name.clone()).collect();
    // bodies: (type_code, schema_code, json_schema_fields, fields_empty)
    let bodies = render_struct_field_bodies(field_defs, Some(item_name));
    // flatten: (ts_types, zod_schemas)
    #[cfg(any(feature = "typescript", feature = "zod"))]
    let flatten = compute_flatten_outputs(flattened_fields);
    vec![
        #[cfg(feature = "jsonschema")]
        generate_json_schema_method(
            &bodies.2,
            &flatten_merged_sources(flattened_fields),
            item_name,
            &schema_parameters(generics, args),
        ),
        #[cfg(feature = "typescript")]
        generate_ts_definition_method(
            docs,
            item_name,
            rust_ident,
            &ts_generics,
            &bodies.0,
            &own_keys,
            &flatten.0,
        ),
        #[cfg(feature = "zod")]
        generate_zod_schema_method(
            item_name,
            rust_ident,
            &type_parameters,
            args,
            &bodies.1,
            "",
            &flatten.1,
        ),
    ]
}

/// Renders the per-field TypeScript type code, Zod schema code, and JSON-schema fragments for a
/// struct's (non-flattened) fields. Returns the accumulated code and whether the field set is empty.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn render_struct_field_bodies(
    field_defs: Vec<FieldDef>,
    item_name_opt: Option<&str>,
) -> (String, String, Vec<proc_macro2::TokenStream>, bool) {
    let fields_empty = field_defs.is_empty();
    let mut type_code = String::new();
    let mut schema_code = String::new();
    #[cfg(feature = "jsonschema")]
    let mut json_schema_fields: Vec<proc_macro2::TokenStream> = Vec::new();
    #[cfg(not(feature = "jsonschema"))]
    let json_schema_fields: Vec<proc_macro2::TokenStream> = Vec::new();

    for fld in field_defs {
        schema_code.push_str(&write_field_type_and_schema(
            &mut type_code,
            &fld,
            item_name_opt,
        ));
        #[cfg(feature = "jsonschema")]
        json_schema_fields.push(build_field_schema(&fld));
    }

    (type_code, schema_code, json_schema_fields, fields_empty)
}

/// The schema module a refused item publishes in place of the one it has no description for.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn refused_item_schema_module(ident: &syn::Ident) -> proc_macro2::TokenStream {
    let module_ident = Ident::new(&ident_schema_module_name(&ident.to_string()), ident.span());
    let refusal = format!("`{ident}`: refused by `#[model_schema()]`, so it describes nothing");

    #[cfg(feature = "jsonschema")]
    let json_schema_methods = {
        let in_flight_type = in_flight_type();
        quote! {
            pub fn json_schema() -> serde_json::Value {
                panic!(#refusal)
            }

            pub fn json_schema_within(
                _in_flight: &mut #in_flight_type,
                _hoisted_defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            ) -> serde_json::Value {
                panic!(#refusal)
            }
        }
    };
    #[cfg(not(feature = "jsonschema"))]
    let json_schema_methods = quote! {};

    #[cfg(feature = "typescript")]
    let ts_definition_method = quote! {
        pub fn ts_definition() -> ::std::string::String {
            panic!(#refusal)
        }
    };
    #[cfg(not(feature = "typescript"))]
    let ts_definition_method = quote! {};

    #[cfg(feature = "zod")]
    let zod_schema_method = quote! {
        pub fn zod_schema() -> ::std::string::String {
            panic!(#refusal)
        }
    };
    #[cfg(not(feature = "zod"))]
    let zod_schema_method = quote! {};

    quote! {
        pub mod #module_ident {
            #[non_exhaustive]
            pub struct Schema;

            impl Schema {
                #json_schema_methods
                #ts_definition_method
                #zod_schema_method
            }
        }
    }
}

/// Nothing, in a build with no schema surface: no reference to a module is ever emitted there, so
/// a refused item leaves none dangling.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
fn refused_item_schema_module(ident: &syn::Ident) -> proc_macro2::TokenStream {
    let _: &syn::Ident = ident;
    quote! {}
}

/// Emits the original item followed by the `compile_error!` tokens of every violated field guard,
/// or `None` when there are none.
fn guard_failure_output<ItemT>(
    item: &ItemT,
    ident: Option<&syn::Ident>,
    guard_errors: &[proc_macro2::TokenStream],
) -> Option<TokenStream>
where
    ItemT: quote::ToTokens,
{
    if guard_errors.is_empty() {
        return None;
    }
    let absorbing_module = ident.map(refused_item_schema_module);
    let output = quote! {
        #item
        #(#guard_errors)*
        #absorbing_module
    };
    log::trace!("{output}");
    Some(output)
}

/// Turns a `cfg_attr` rejection into `compile_error!` tokens naming the item it was found on,
/// keeping the attribute's span so the diagnostic still points at the offending line.
#[cfg(feature = "serde")]
fn cfg_attr_guard_error(rejection: &syn::Error, item: &str) -> proc_macro2::TokenStream {
    reworded(
        rejection,
        &prefixed_guard_message(&format!("{item}: {rejection}")),
    )
}

/// Turns a rejected `pattern` into `compile_error!` tokens naming what carries it, keeping the
/// literal's span so the diagnostic points at the pattern as written.
fn pattern_guard_error(rejection: &syn::Error, subject: &str) -> proc_macro2::TokenStream {
    reworded(
        rejection,
        &prefixed_guard_message(&format!("{subject}: {rejection}")),
    )
}

/// The macro's own name, in front of every diagnostic it emits — the one thing that separates this
/// crate's refusal from rustc's on a screen full of errors.
fn prefixed_guard_message(message: &str) -> String {
    format!("model_schema: {message}")
}

/// Turns an attribute parser's refusal — of a `model_schema` argument, a `model_schema_prop` key,
/// or a serde renaming that names two keys — into `compile_error!` tokens naming what carries it,
/// keeping the refusal's span so the diagnostic points at the argument, key or value as written.
fn attr_guard_error(rejection: &syn::Error, subject: &str) -> proc_macro2::TokenStream {
    reworded(
        rejection,
        &prefixed_guard_message(&format!("{subject}: {rejection}")),
    )
}

/// `rejection` as `compile_error!` tokens saying `message` instead. `Error::span` joins the two
/// ends of a refusal, which a stable toolchain cannot do, so the tokens syn spans on both ends are
/// kept and only the text they carry is replaced.
fn reworded(rejection: &syn::Error, message: &str) -> proc_macro2::TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();
    for tree in rejection.to_compile_error() {
        let proc_macro2::TokenTree::Group(written) = tree else {
            tokens.extend([tree]);
            continue;
        };
        let mut text = proc_macro2::Literal::string(message);
        text.set_span(written.span());
        let mut body = proc_macro2::Group::new(
            written.delimiter(),
            proc_macro2::TokenTree::Literal(text).into(),
        );
        body.set_span(written.span());
        tokens.extend([proc_macro2::TokenTree::Group(body)]);
        break;
    }
    tokens
}

/// Names a field in a guard message; tuple slots have no ident to name.
fn field_label(raw_field_ident: &str) -> String {
    if raw_field_ident.is_empty() {
        "tuple field".to_owned()
    } else {
        format!("field `{raw_field_ident}`")
    }
}

/// Names the item a type-level guard message is about; a shape this macro does not expand has no
/// ident worth naming.
fn item_label(item: &Item) -> String {
    if let Item::Struct(item_struct) = item {
        format!("type `{}`", item_struct.ident)
    } else if let Item::Enum(item_enum) = item {
        format!("type `{}`", item_enum.ident)
    } else if let Item::Type(item_type) = item {
        format!("type `{}`", item_type.ident)
    } else {
        "item".to_owned()
    }
}

/// The Rust ident a refused item publishes its schema module under — one of the three shapes this
/// macro expands, which are the three a reference can name. Anything else names no module, so a
/// refusal there leaves nothing dangling.
const fn item_schema_ident(item: &Item) -> Option<&syn::Ident> {
    if let Item::Struct(item_struct) = item {
        Some(&item_struct.ident)
    } else if let Item::Enum(item_enum) = item {
        Some(&item_enum.ident)
    } else if let Item::Type(item_type) = item {
        Some(&item_type.ident)
    } else {
        None
    }
}

/// The name an item publishes on every surface: its own ident, or the `name = "..."` override, with
/// an alias taking the `Type` suffix it has no surface name of its own without.
fn item_published_name(item: &Item, override_name: Option<&str>) -> Option<String> {
    let ident = item_schema_ident(item)?.to_string();
    Some(if matches!(*item, Item::Type(_)) {
        compute_alias_export_name(&ident, override_name)
    } else {
        compute_item_export_name(&ident, override_name)
    })
}

/// The `compile_error!` tokens an item earns for publishing a name another declaration has already
/// published. Read at the ungated seam, so the verdict is the same in every feature combination,
/// and spanned on the ident, an override being refusable on either of the two declarations.
fn published_name_collision_errors(
    item: &Item,
    args: &ModelSchemaArgs,
) -> Vec<proc_macro2::TokenStream> {
    let (Some(ident), Some(published)) = (
        item_schema_ident(item),
        item_published_name(item, args.name_override.as_deref()),
    ) else {
        return Vec::new();
    };
    let Some(holder) = claim_published_name(&published, &ident.to_string()) else {
        return Vec::new();
    };
    let message = prefixed_guard_message(&format!(
        "{} publishes as `{published}`, which type `{holder}` already publishes as -- one name \
         cannot carry two declarations, whose types, schemas and definitions would overwrite each \
         other. Give one of them a `#[model_schema(name = \"...\")]` of its own",
        item_label(item)
    ));
    vec![syn::Error::new_spanned(ident, message).to_compile_error()]
}

/// What keeps two functions of the typed paths of an enum under a tag from taking one name.
#[cfg(feature = "serde")]
fn decode_with_arm_refusals(item: &Item) -> Vec<syn::Error> {
    if let Item::Enum(item_enum) = item {
        tag_arm_refusals(item_enum)
    } else {
        Vec::new()
    }
}

/// Without `serde` nothing reads the tag an arm is named by.
#[cfg(not(feature = "serde"))]
const fn decode_with_arm_refusals(_item: &Item) -> Vec<syn::Error> {
    Vec::new()
}

/// The conversions `enable = ["archie"]` writes beside a type, under the `archie` feature.
#[cfg(feature = "archie")]
fn archie_conversions(item: &Item, args: &ModelSchemaArgs) -> TokenStream {
    if args.enable.archie {
        archie::conversions(item)
    } else {
        TokenStream::new()
    }
}

/// Nothing: without the `archie` feature, the entry adds nothing.
#[cfg(not(feature = "archie"))]
fn archie_conversions(_item: &Item, _args: &ModelSchemaArgs) -> TokenStream {
    TokenStream::new()
}

/// The refusal of `enable = ["archie"]` on a type alias, in every build: the conversions are impls
/// on the type the alias names, which the alias cannot be written for.
fn archie_guard_errors(item: &Item, args: &ModelSchemaArgs) -> Vec<proc_macro2::TokenStream> {
    let Item::Type(alias) = item else {
        return Vec::new();
    };
    if !args.enable.archie {
        return Vec::new();
    }
    let name = &alias.ident;
    let target = written_spelling(&alias.ty);
    let refusal = syn::Error::new_spanned(
        &alias.ty,
        format!(
            "`enable = [\"archie\"]` cannot be written on a type alias: `{name}` is another name \
             for `{target}`, and the conversions would have to be written for that type. List it \
             on the model types the alias reaches."
        ),
    );
    vec![attr_guard_error(&refusal, &item_label(item))]
}

/// What the `enable` list refuses on an item: the refusals of `"decode_with"`, then of `"archie"`.
fn enable_guard_errors(item: &Item, args: &ModelSchemaArgs) -> Vec<proc_macro2::TokenStream> {
    let mut errors = decode_with_guard_errors(item, args);
    errors.extend(archie_guard_errors(item, args));
    errors
}

/// The `compile_error!` tokens `decode_with` earns where it cannot be written: on an alias, on a
/// type that borrows, and on an enum under a tag two of whose arms would take one name. With
/// `"mongodb"` listed, also on a map whose key the `bson` library refuses.
fn decode_with_guard_errors(item: &Item, args: &ModelSchemaArgs) -> Vec<proc_macro2::TokenStream> {
    if !args.enable.decode_with {
        return Vec::new();
    }
    let borrowed = item_generics(item).and_then(|generics| generics.lifetimes().next());
    let refusal = if let Item::Type(alias) = item {
        Some(decode_with_alias_refusal(alias))
    } else if let (Some(lifetime), Some(ident)) = (borrowed, item_schema_ident(item)) {
        Some(decode_with_borrow_refusal(ident, lifetime))
    } else if let Some((field_name, declared, reference)) = decode_with_borrowing_field(item) {
        Some(decode_with_reference_refusal(
            &field_name,
            declared,
            reference,
        ))
    } else {
        None
    };
    refusal
        .into_iter()
        .chain(decode_with_arm_refusals(item))
        .chain(
            args.enable
                .mongodb
                .then(|| mongodb_map_key_refusals(item))
                .into_iter()
                .flatten(),
        )
        .map(|rejection| attr_guard_error(&rejection, &item_label(item)))
        .collect()
}

/// One refusal per map a member of `item` holds whose key the `bson` library refuses, or per model
/// or alias declared above that holds one, spanned on the member.
fn mongodb_map_key_refusals(item: &Item) -> Vec<syn::Error> {
    bson_refused_maps(item, KeyAsked::Refused)
        .iter()
        .map(|(variant, position, field, key)| {
            bson_refused_key_refusal(*variant, *position, field, key)
        })
        .collect()
}

/// Each map a member of `item` holds under the key `asked`, beside the member: every member serde
/// writes or reads itself, of a struct and of each variant of an enum serde writes or reads
/// itself. A member that holds the item itself reaches no map of its own the item's own members
/// do not.
fn bson_refused_maps(
    item: &Item,
    asked: KeyAsked,
) -> Vec<(Option<&syn::Ident>, usize, &Field, RefusedKey<'_>)> {
    let members: Vec<(Option<&syn::Ident>, &syn::Fields)> = if let Item::Struct(item_struct) = item
    {
        vec![(None, &item_struct.fields)]
    } else if let Item::Enum(item_enum) = item {
        item_enum
            .variants
            .iter()
            .filter(|variant| {
                !parse_serde_key_omission(&variant.attrs).absent_from_wire()
                    && !written_and_read_by_the_author(&variant.attrs)
            })
            .map(|variant| (Some(&variant.ident), &variant.fields))
            .collect()
    } else {
        Vec::new()
    };
    let own = item_schema_ident(item).map(ToString::to_string);
    members
        .into_iter()
        .flat_map(|(variant, fields)| {
            fields
                .iter()
                .enumerate()
                .filter(|(_, field)| {
                    !parse_serde_key_omission(&field.attrs).absent_from_wire()
                        && !written_and_read_by_the_author(&field.attrs)
                })
                .flat_map(move |(position, field)| {
                    bson_refused_keys(&field.ty, asked)
                        .into_iter()
                        .map(move |key| (variant, position, field, key))
                })
        })
        .filter(
            |(.., key)| !matches!(key, RefusedKey::Held(held) if own.as_ref() == Some(&held.model)),
        )
        .collect()
}

/// The first of `maps`, after the member that holds it.
fn first_map_described(
    maps: &[(Option<&syn::Ident>, usize, &Field, RefusedKey<'_>)],
) -> Option<String> {
    maps.first().map(|(variant, position, field, key)| {
        format!(
            "{}, {}",
            refused_member_named(*variant, *position, field),
            key.described()
        )
    })
}

/// A member named by its ident or its position, behind the variant it is a member of.
fn refused_member_named(variant: Option<&syn::Ident>, position: usize, field: &Field) -> String {
    let member = field.ident.as_ref().map_or_else(
        || format!("field `{position}`"),
        |ident| format!("field `{}`", ident.unraw()),
    );
    variant.map_or_else(
        || member.clone(),
        |held_by| format!("variant `{}`, {member}", held_by.unraw()),
    )
}

/// The refusal of the map `field` reaches through `key`, named by the field's ident or its
/// position, behind the variant it is a member of.
fn bson_refused_key_refusal(
    variant: Option<&syn::Ident>,
    position: usize,
    field: &Field,
    key: &RefusedKey<'_>,
) -> syn::Error {
    let named = refused_member_named(variant, position, field);
    let message = match key {
        RefusedKey::Held(held) => format!(
            "{named}: `{}` holds a map that cannot be stored in MongoDB ({}) — BSON document \
             keys are always text, and the `bson` library refuses to write this key and to read \
             it back. Key that map by a `String`, or do not list \"mongodb\" in `enable`.",
            held.model, held.described
        ),
        RefusedKey::Written(written) => format!(
            "{named}: a map keyed by `{}` cannot be stored in MongoDB — BSON document keys are \
             always text, and the `bson` library refuses to write this key and to read it back. \
             Key the map by a `String`, or do not list \"mongodb\" in `enable`.",
            written_spelling(written)
        ),
    };
    field.ident.as_ref().map_or_else(
        || syn::Error::new_spanned(&field.ty, &message),
        |ident| syn::Error::new(ident.span(), &message),
    )
}

/// The key of each map `ty` holds under the key `asked`, behind `Option`, a list, a tuple, a
/// transparent wrapper, a map's value, either arm of a std `Result` and the bound of a std range,
/// and each model or alias declared above that holds one.
fn bson_refused_keys(ty: &syn::Type, asked: KeyAsked) -> Vec<RefusedKey<'_>> {
    match written_type(ty) {
        syn::Type::Array(array) => bson_refused_keys(&array.elem, asked),
        syn::Type::Slice(slice) => bson_refused_keys(&slice.elem, asked),
        syn::Type::Paren(paren) => bson_refused_keys(&paren.elem, asked),
        syn::Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .flat_map(|elem| bson_refused_keys(elem, asked))
            .collect(),
        syn::Type::Path(named) => {
            let Some(last) = named.path.segments.last() else {
                return Vec::new();
            };
            let held: Vec<&syn::Type> = type_arguments_of(&last.arguments).collect();
            let name = last.ident.to_string();
            if is_map_wrapper(&name) {
                let [key, value, ..] = held.as_slice() else {
                    return Vec::new();
                };
                asked
                    .holds(key)
                    .then_some(RefusedKey::Written(key))
                    .into_iter()
                    .chain(bson_refused_keys(value, asked))
                    .collect()
            } else if name == "Option"
                || is_sequence_wrapper(&name)
                || is_transparent_wrapper(&name)
                || written_std_range(named).is_some()
            {
                held.first()
                    .map_or_else(Vec::new, |inner| bson_refused_keys(inner, asked))
            } else if let Some((ok, err)) = written_std_result(named) {
                bson_refused_keys(ok, asked)
                    .into_iter()
                    .chain(bson_refused_keys(err, asked))
                    .collect()
            } else {
                held_model_maps(named, &name, &held, asked)
                    .into_iter()
                    .map(RefusedKey::Held)
                    .collect()
            }
        }
        // A reference is refused to `"decode_with"` already, and none of the others is stored.
        syn::Type::FnPtr(_)
        | syn::Type::Group(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_)
        | syn::Type::Ptr(_)
        | syn::Type::Reference(_)
        | syn::Type::TraitObject(_)
        | syn::Type::Verbatim(_)
        | _ => Vec::new(),
    }
}

/// The maps the model or the alias `name`, written as `written` with the type arguments `held`,
/// holds under the key `asked`. One of its own: the first its expansion recorded, else the first
/// it keys by its type parameter at a position whose argument is such a key. Then, where it is
/// declared above, each map its arguments hold, every argument being read as one it writes into
/// its row.
fn held_model_maps(
    written: &syn::TypePath,
    name: &str,
    held: &[&syn::Type],
    asked: KeyAsked,
) -> Vec<HeldMap> {
    let own = asked
        .held_by(name)
        .map(|described| HeldMap {
            described,
            model: name.to_owned(),
        })
        .or_else(|| {
            forwarded_key_maps(name)
                .into_iter()
                .find_map(|(at, described)| {
                    asked.holds(held.get(at)?).then(|| HeldMap {
                        described: filled_in(&described, held),
                        model: written_spelling(written),
                    })
                })
        });
    let in_arguments: Vec<HeldMap> = if held.is_empty() || !is_declared_ident(name) {
        Vec::new()
    } else {
        held.iter()
            .flat_map(|argument| bson_refused_keys(argument, asked))
            .map(|key| HeldMap {
                described: key.described(),
                model: written_spelling(written),
            })
            .collect()
    };
    own.into_iter().chain(in_arguments).collect()
}

/// What a struct's, an enum's or an alias's own type parameter at position `at` is spelled as
/// while the maps it keys are recorded. [`filled_in`] writes the argument a holder fills it with in its place.
fn parameter_placeholder(at: usize) -> String {
    format!("{PLACEHOLDER_OPENS}{at}{PLACEHOLDER_CLOSES}")
}

/// `described` with each [`parameter_placeholder`] in it replaced by the argument at its position
/// among `arguments`, spelled as written, and by `_` where none is written there.
fn filled_in(described: &str, arguments: &[&syn::Type]) -> String {
    let mut pieces = described.split(PLACEHOLDER_OPENS);
    let mut filled = pieces.next().unwrap_or_default().to_owned();
    for piece in pieces {
        let (written_at, rest) = piece.split_once(PLACEHOLDER_CLOSES).unwrap_or((piece, ""));
        let argument = written_at
            .parse::<usize>()
            .ok()
            .and_then(|at| arguments.get(at));
        filled.push_str(&argument.map_or_else(|| "_".to_owned(), written_spelling));
        filled.push_str(rest);
    }
    filled
}

/// Whether serde writes a key of type `key` as a number or a `bool`: one of
/// [`BSON_REFUSED_KEY_TYPES`], or a brand or an alias over one declared above, or filled at one
/// where its value reaches its own parameter, under any `Box`, `Rc`, `Arc` and `Cow`.
fn is_bson_refused_key(key: &syn::Type) -> bool {
    KeyAsked::Refused.holds(key)
}

/// Whether the key serde writes for a value of type `key` is one `named` answers for by its last
/// name, under any `Box`, `Rc`, `Arc` and `Cow`, and through a brand or an alias declared above
/// whose value reaches its own parameter, at what fills it.
fn written_key_is(key: &syn::Type, named: &dyn Fn(&str) -> bool) -> bool {
    let mut current = written_type(key);
    loop {
        let syn::Type::Path(path) = current else {
            return false;
        };
        let Some(last) = path.path.segments.last() else {
            return false;
        };
        let name = last.ident.to_string();
        if !is_ownership_wrapper(&name) {
            if let Some(at) = forwarded_key_parameter(&name)
                && let Some(filled) = type_arguments_of(&last.arguments).nth(at)
            {
                return written_key_is(filled, named);
            }
            return named(&name);
        }
        let Some(inner) = type_arguments_of(&last.arguments).next() else {
            return false;
        };
        current = written_type(inner);
    }
}

/// The type arguments a path segment is written with, its lifetimes and consts left out.
fn type_arguments_of(arguments: &syn::PathArguments) -> impl Iterator<Item = &syn::Type> {
    let written = if let syn::PathArguments::AngleBracketed(angled) = arguments {
        Some(angled.args.iter())
    } else {
        None
    };
    written.into_iter().flatten().filter_map(|argument| {
        if let syn::GenericArgument::Type(ty) = argument {
            Some(ty)
        } else {
            None
        }
    })
}

/// What `item` is replaced by where the read of it cannot call a `preprocess` it names: the item
/// with its members' own attribute taken off, and each refusal. `None` where every function it
/// names is one its read calls.
fn preprocess_refusal_output(item: &Item, args: &ModelSchemaArgs) -> Option<TokenStream> {
    let refusals: Vec<proc_macro2::TokenStream> = preprocess::refusals(item, &args.preprocess)
        .iter()
        .map(|rejection| attr_guard_error(rejection, &item_label(item)))
        .collect();
    refused_item_output(item, &refusals)
}

/// Records the functions an item names for the parts of its expansion written apart: its checks,
/// and the `preprocess` of the type and of each member.
fn record_item_functions(item: &Item, args: &ModelSchemaArgs) {
    record_item_checks(item, args);
    preprocess::record(item, &args.preprocess);
    record_zod_binding_preprocess(item, args);
}

fn decode_with_alias_refusal(alias: &ItemType) -> syn::Error {
    let name = &alias.ident;
    let target = written_spelling(&alias.ty);
    syn::Error::new_spanned(
        &alias.ty,
        format!(
            "`enable = [\"decode_with\"]` cannot be written on a type alias: `{name}` is another \
             name for `{target}`, and the methods would have to be added to that type. List it on \
             the model types the alias reaches."
        ),
    )
}

fn decode_with_borrow_refusal(ident: &syn::Ident, lifetime: &syn::LifetimeParam) -> syn::Error {
    let borrowed = &lifetime.lifetime;
    syn::Error::new_spanned(
        lifetime,
        format!(
            "`enable = [\"decode_with\"]` cannot be written on a type that borrows: \
             `from_value_with` owns the value it reads, so the decoded `{ident}<{borrowed}>` cannot \
             borrow from it for `{borrowed}`."
        ),
    )
}

fn decode_with_reference_refusal(
    field_name: &str,
    declared: &syn::Type,
    reference: &syn::TypeReference,
) -> syn::Error {
    let written = written_spelling(declared);
    let lifetime = reference.lifetime.as_ref().map_or_else(
        || "its lifetime".to_owned(),
        |borrowed| format!("`{borrowed}`"),
    );
    syn::Error::new_spanned(
        reference,
        format!(
            "`enable = [\"decode_with\"]` cannot be written on a type with a field that borrows: \
             `{field_name}` is `{written}`, and `from_value_with` owns the value it reads, so \
             nothing decoded from it can be borrowed for {lifetime}. Write the field as an owned \
             type."
        ),
    )
}

/// The first field serde reads whose type holds a reference, with the name the refusal gives it: a
/// tuple position by its number, and an enum's field behind its variant.
fn decode_with_borrowing_field(item: &Item) -> Option<(String, &syn::Type, &syn::TypeReference)> {
    let members: Vec<(String, &syn::Fields)> = if let Item::Struct(item_struct) = item {
        vec![(String::new(), &item_struct.fields)]
    } else if let Item::Enum(item_enum) = item {
        item_enum
            .variants
            .iter()
            .map(|variant| (format!("{}.", variant.ident), &variant.fields))
            .collect()
    } else {
        Vec::new()
    };
    members.into_iter().find_map(|(variant, fields)| {
        fields.iter().enumerate().find_map(|(position, field)| {
            // Nothing is decoded into a field serde never reads, so it borrows from no value.
            if parse_serde_key_omission(&field.attrs).skips_deserializing {
                return None;
            }
            let reference = held_reference(&field.ty)?;
            let member = field
                .ident
                .as_ref()
                .map_or_else(|| position.to_string(), ToString::to_string);
            Some((format!("{variant}{member}"), &field.ty, reference))
        })
    })
}

/// The first reference among the values a type holds, the type itself included.
fn held_reference(written: &syn::Type) -> Option<&syn::TypeReference> {
    match written {
        syn::Type::Reference(reference) => Some(reference),
        syn::Type::Path(type_path) => type_path.path.segments.iter().find_map(|segment| {
            let syn::PathArguments::AngleBracketed(angled) = &segment.arguments else {
                return None;
            };
            angled.args.iter().find_map(|argument| {
                if let syn::GenericArgument::Type(inner) = argument {
                    held_reference(inner)
                } else {
                    None
                }
            })
        }),
        syn::Type::Array(array) => held_reference(&array.elem),
        syn::Type::Slice(slice) => held_reference(&slice.elem),
        syn::Type::Paren(paren) => held_reference(&paren.elem),
        syn::Type::Group(group) => held_reference(&group.elem),
        syn::Type::Tuple(tuple) => tuple.elems.iter().find_map(held_reference),
        // None of these holds a value serde reads out of what it is handed.
        syn::Type::FnPtr(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_)
        | syn::Type::Ptr(_)
        | syn::Type::TraitObject(_)
        | syn::Type::Verbatim(_)
        | _ => None,
    }
}

/// Tokens as their author spells them, without the spaces a token stream prints between them.
pub fn written_spelling<T>(tokens: &T) -> String
where
    T: quote::ToTokens,
{
    quote! { #tokens }
        .to_string()
        .replace(" < ", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
        .replace(" :: ", "::")
        .replace("& ", "&")
}

/// What an item leaves for the expansions after it, ahead of the shape it is dispatched to: which
/// Zod binding it publishes, and what it declares.
#[cfg(any(feature = "serde", feature = "zod"))]
fn record_item(item: &Item) {
    record_own_zod_binding(item);
    record_declaration(item);
    #[cfg(feature = "serde")]
    if let Some(ident) = item_schema_ident(item) {
        let name = ident.to_string();
        record_open_map(&name, ranges::writes_an_open_map(item));
        record_open_map_parameters(&name, ranges::open_map_parameter_positions(item));
        record_text_key_parameters(&name, ranges::text_key_parameters_of(item));
        record_flattened_beside_parameters(&name, ranges::flattened_beside_parameters_of(item));
        record_plain_enum(&name, written_as_a_variant_name(item));
        record_non_object_wire(&name, non_object_wire(item));
        record_value_parameter(&name, value_parameter_position(item));
        record_union_parameters(&name, union_parameters_of(item));
    }
}

/// What serde writes a value of `item` as, in the leaves the flatten refusal reads, read off the
/// item as the leaves a schema surface records are read off what it publishes: a brand's or a
/// struct of one slot's value, any other tuple struct's array, a plain enum's name, each variant
/// serde writes or reads of an externally tagged enum and each such member of an untagged enum,
/// and an alias's value. A brand's, a struct of one slot's and an alias's value is also recorded
/// with the members of the untagged enum it names. A brand's, a struct of one slot's, an alias's
/// value and an untagged member that names a generic type declared above is read at what fills
/// its type parameter.
#[cfg(feature = "serde")]
fn non_object_wire(item: &Item) -> NonObjectWire {
    let parameters = item_generics(item).map_or_else(Vec::new, type_parameters_in_scope);
    let written = |ty: &syn::Type| {
        let mut def = get_field_def("_written", ty, "");
        def.erase_type_parameters(&parameters);
        def
    };
    let mut wire = NonObjectWire::default();
    if let Item::Struct(item_struct) = item {
        if let Some(slot) = written_as_its_slot(item_struct) {
            let value = written(&slot.ty);
            wire.leaves = filled_wire_leaves(&value);
            wire.members = filled_union_members(&value);
            wire.optional = writes_an_optional(value_filling(&value).unwrap_or(&value));
        } else if matches!(item_struct.fields, syn::Fields::Unnamed(_)) {
            wire.leaves = vec![WireLeaf {
                branch: Vec::new(),
                non_object: Some("array"),
                written_as: None,
            }];
        } else {
            // serde writes the members under keys of their own.
        }
    } else if let Item::Enum(item_enum) = item {
        let tagging = Tagging::of(item_enum);
        if matches!(tagging, Tagging::Plain) {
            wire.leaves = vec![WireLeaf {
                branch: Vec::new(),
                non_object: Some("string"),
                written_as: Some(WrittenAs::PlainEnumName),
            }];
        } else if matches!(tagging, Tagging::External) {
            wire.leaves = external_variant_wire_leaves(&item_enum.variants);
        } else if matches!(tagging, Tagging::Untagged) {
            wire.members = item_enum
                .variants
                .iter()
                .filter(|variant| variant_on_the_wire(variant))
                .enumerate()
                .flat_map(|(index, variant)| {
                    let first = variant.fields.iter().next();
                    let leaves = match (variant_wire_kind(variant), first) {
                        (VariantKind::TupleSingle, Some(slot)) => {
                            member_written_leaves(&written(&slot.ty))
                        }
                        _ => vec![WireLeaf {
                            branch: Vec::new(),
                            non_object: None,
                            written_as: None,
                        }],
                    };
                    leaves.into_iter().map(move |mut leaf| {
                        leaf.branch.insert(0, index + 1);
                        leaf
                    })
                })
                .collect();
        } else {
            // serde writes each variant under a tag key, in an object.
        }
    } else if let Item::Type(alias) = item {
        let value = written(&alias.ty);
        wire.leaves = filled_wire_leaves(&value);
        wire.members = filled_union_members(&value);
        wire.optional = writes_an_optional(value_filling(&value).unwrap_or(&value));
    } else {
        // Nothing else is expanded.
    }
    wire
}

/// The field serde writes a value of `item_struct` as: a brand's, or the one slot of a tuple
/// struct.
#[cfg(feature = "serde")]
fn written_as_its_slot(item_struct: &syn::ItemStruct) -> Option<&syn::Field> {
    brand_field(item_struct).or_else(|| match &item_struct.fields {
        syn::Fields::Unnamed(slots) if slots.unnamed.len() == 1 => slots.unnamed.first(),
        syn::Fields::Named(_) | syn::Fields::Unnamed(_) | syn::Fields::Unit => None,
    })
}

/// The position among `item`'s type parameters of the one serde writes a value of `item` as: a
/// brand's, a struct of one slot's or an alias's value that is that parameter under the
/// transparent wrappers, or a type declared above filled with it where [`value_parameter`] holds.
#[cfg(feature = "serde")]
fn value_parameter_position(item: &Item) -> Option<usize> {
    let value = if let Item::Struct(item_struct) = item {
        &written_as_its_slot(item_struct)?.ty
    } else if let Item::Type(alias) = item {
        &*alias.ty
    } else {
        return None;
    };
    let parameters = item_generics(item).map_or_else(Vec::new, type_parameters_in_scope);
    let mut def = get_field_def("_written", value, "");
    def.erase_type_parameters(&parameters);
    let filling = value_filling(&def).unwrap_or(&def);
    if filling.is_optional() || filling.array_depth > 0 {
        return None;
    }
    let FieldDefType::TypeParam(name) = &filling.field_type else {
        return None;
    };
    parameters.iter().position(|parameter| parameter == name)
}

/// What fills the type parameter of the type declared above that `def` names, where serde writes
/// a value of that type as the parameter's filling, read on through each such type the filling
/// names in turn, and `None` for a `def` that names no such type.
#[cfg(feature = "serde")]
fn value_filling(def: &FieldDef) -> Option<&FieldDef> {
    let mut filling = filled_argument(def)?;
    while let Some(deeper) = filled_argument(filling) {
        filling = deeper;
    }
    Some(filling)
}

/// The argument `def` fills the type it names with, at the position [`value_parameter`] records.
#[cfg(feature = "serde")]
fn filled_argument(def: &FieldDef) -> Option<&FieldDef> {
    if def.is_optional() || def.array_depth > 0 {
        return None;
    }
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return None;
    };
    arguments.get(value_parameter(name)?)
}

/// `def` with its outermost `Option` taken off: what serde writes a `Some` of it as.
#[cfg(feature = "serde")]
fn without_its_option(def: &FieldDef) -> FieldDef {
    let mut bare = def.clone();
    bare.nullable_levels
        .retain(|&level| level != bare.array_depth);
    bare
}

/// The leaves of an `Option` of a value whose own leaves are `leaves`: the value's choice at
/// branch 1, and the absence at branch 2.
#[cfg(feature = "serde")]
fn under_an_option(mut leaves: Vec<WireLeaf>) -> Vec<WireLeaf> {
    for leaf in &mut leaves {
        leaf.branch.insert(0, 1);
    }
    leaves.push(WireLeaf {
        branch: vec![2],
        non_object: Some("null"),
        written_as: None,
    });
    leaves
}

/// [`published_wire_leaves`] of `written` off the record every build with serde writes, a generic
/// type declared above that it names, itself or under an `Option`, read at its [`value_filling`].
#[cfg(feature = "serde")]
fn filled_wire_leaves(written: &FieldDef) -> Vec<WireLeaf> {
    if written.is_optional() {
        return under_an_option(filled_wire_leaves(&without_its_option(written)));
    }
    let value = value_filling(written).unwrap_or(written);
    if value.is_optional() {
        filled_wire_leaves(value)
    } else {
        published_wire_leaves(value, written_leaves)
    }
}

/// [`written_union_members`] of `def`, a generic type declared above that it names, itself or
/// under an `Option`, read at its [`value_filling`].
#[cfg(feature = "serde")]
fn filled_union_members(def: &FieldDef) -> Vec<WireLeaf> {
    value_filling(&without_its_option(def))
        .map_or_else(|| written_union_members(def), filled_union_members)
}

/// Whether serde writes a value of `def` as an `Option`: `def` is one, or names a type declared
/// above that `NON_OBJECT_WIRES` records as one.
#[cfg(feature = "serde")]
fn writes_an_optional(def: &FieldDef) -> bool {
    if def.is_optional() {
        return true;
    }
    if def.array_depth > 0 {
        return false;
    }
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return false;
    };
    recorded_non_object_wire(name).is_some_and(|wire| wire.optional)
}

/// What one untagged member written as `fld` contributes below its own position, as the Zod
/// surface's leaves of a member are read: the members of the union it names, the leaves of the
/// name, or the member itself, and under an `Option` both the value's choice and the absence's. A
/// generic type declared above that it names is read at its [`value_filling`].
#[cfg(feature = "serde")]
fn member_written_leaves(fld: &FieldDef) -> Vec<WireLeaf> {
    if fld.is_optional() {
        return under_an_option(member_written_leaves(&without_its_option(fld)));
    }
    if let Some(filling) = value_filling(fld) {
        return member_written_leaves(filling);
    }
    let nested = written_union_members(fld);
    if !nested.is_empty() {
        return nested;
    }
    named_wire_leaves(fld, written_leaves).unwrap_or_else(|| {
        vec![WireLeaf {
            branch: Vec::new(),
            non_object: written_non_object_wire(fld, written_leaves),
            written_as: fld.is_unit().then_some(WrittenAs::Unit),
        }]
    })
}

/// The leaves of the members of the untagged union `item` stands for that serde writes as what
/// fills one of its type parameters, placed as [`non_object_wire`] places each member's leaves:
/// an untagged enum's, and those a brand's, a struct of one slot's or an alias's value hands its
/// own parameters on to. Empty for any other item.
#[cfg(feature = "serde")]
fn union_parameters_of(item: &Item) -> Vec<UnionParameter> {
    let parameters = item_generics(item).map_or_else(Vec::new, type_parameters_in_scope);
    let written = |ty: &syn::Type| {
        let mut def = get_field_def("_written", ty, "");
        def.erase_type_parameters(&parameters);
        def
    };
    if let Item::Struct(item_struct) = item {
        written_as_its_slot(item_struct).map_or_else(Vec::new, |slot| {
            filled_union_parameters(&written(&slot.ty), &parameters)
        })
    } else if let Item::Type(alias) = item {
        filled_union_parameters(&written(&alias.ty), &parameters)
    } else if let Item::Enum(item_enum) = item
        && matches!(Tagging::of(item_enum), Tagging::Untagged)
    {
        item_enum
            .variants
            .iter()
            .filter(|variant| variant_on_the_wire(variant))
            .enumerate()
            .filter_map(|(index, variant)| {
                match (variant_wire_kind(variant), variant.fields.iter().next()) {
                    (VariantKind::TupleSingle, Some(slot)) => Some((index, slot)),
                    _ => None,
                }
            })
            .flat_map(|(index, slot)| {
                member_parameter_leaves(&written(&slot.ty), &parameters)
                    .into_iter()
                    .map(move |mut leaf| {
                        leaf.branch.insert(0, index + 1);
                        leaf
                    })
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// The leaves of the members [`filled_union_members`] reads of `def` that serde writes as what
/// fills one of `parameters`, placed as it places them.
#[cfg(feature = "serde")]
fn filled_union_parameters(def: &FieldDef, parameters: &[String]) -> Vec<UnionParameter> {
    value_filling(&without_its_option(def)).map_or_else(
        || {
            if let FieldDefType::SiblingType(name, arguments) = &def.field_type
                && def.array_depth == 0
            {
                named_union_parameters(name, arguments, parameters)
            } else {
                Vec::new()
            }
        },
        |filling| filled_union_parameters(filling, parameters),
    )
}

/// The leaves of the members of the untagged union `name` stands for, named with `arguments`, that
/// serde writes as what fills one of `parameters`: each leaf [`union_parameters`] records, read at
/// the argument that fills it.
#[cfg(feature = "serde")]
fn named_union_parameters(
    name: &str,
    arguments: &[FieldDef],
    parameters: &[String],
) -> Vec<UnionParameter> {
    union_parameters(name)
        .into_iter()
        .filter_map(|held| Some((arguments.get(held.at)?, held.branch)))
        .flat_map(|(argument, branch)| {
            member_parameter_leaves(argument, parameters)
                .into_iter()
                .map(move |mut leaf| {
                    leaf.branch = [branch.as_slice(), &leaf.branch].concat();
                    leaf
                })
        })
        .collect()
}

/// The leaves of an untagged member written as `fld` that serde writes as what fills one of
/// `parameters`, read as [`member_written_leaves`] reads the member: under an `Option`, at a
/// generic type's [`value_filling`], and through an untagged union declared above that `fld`
/// fills with a parameter.
#[cfg(feature = "serde")]
fn member_parameter_leaves(fld: &FieldDef, parameters: &[String]) -> Vec<UnionParameter> {
    if fld.is_optional() {
        let mut leaves = member_parameter_leaves(&without_its_option(fld), parameters);
        for leaf in &mut leaves {
            leaf.branch.insert(0, 1);
        }
        return leaves;
    }
    if let Some(filling) = value_filling(fld) {
        return member_parameter_leaves(filling, parameters);
    }
    if fld.array_depth > 0 {
        return Vec::new();
    }
    if let FieldDefType::TypeParam(name) = &fld.field_type {
        return parameters
            .iter()
            .position(|parameter| parameter == name)
            .map(|at| UnionParameter {
                at,
                branch: Vec::new(),
            })
            .into_iter()
            .collect();
    }
    let FieldDefType::SiblingType(name, arguments) = &fld.field_type else {
        return Vec::new();
    };
    named_union_parameters(name, arguments, parameters)
}

/// Whether serde writes `item` as the bare name of a plain enum's variant, or as an `Option` of
/// one: a plain enum, and a brand, a struct of one slot or an alias whose value
/// [`written_as_a_plain_enum`] answers for.
#[cfg(feature = "serde")]
fn written_as_a_variant_name(item: &Item) -> bool {
    let value = if let Item::Enum(item_enum) = item {
        return matches!(Tagging::of(item_enum), Tagging::Plain);
    } else if let Item::Struct(item_struct) = item {
        let Some(slot) = written_as_its_slot(item_struct) else {
            return false;
        };
        &slot.ty
    } else if let Item::Type(alias) = item {
        &*alias.ty
    } else {
        return false;
    };
    let parameters = item_generics(item).map_or_else(Vec::new, type_parameters_in_scope);
    let mut def = get_field_def("_written", value, "");
    def.erase_type_parameters(&parameters);
    written_as_a_plain_enum(&def)
}

/// The refusal of each map `item` flattens beside another member that writes a key, or beside the
/// tag serde writes at its level, in a struct and in each variant with named fields, spanned on the
/// map, and of each map that fills a type parameter a type flattens beside a key; then of each map
/// keyed by a number or a `bool` where serde reads every key as text, below a flattened member and
/// in a variant of an untagged enum or of an enum under a tag alone, written there or filling a
/// type parameter read there.
#[cfg(feature = "serde")]
fn flattened_map_errors(item: &Item, args: &ModelSchemaArgs) -> Vec<proc_macro2::TokenStream> {
    let Some(generics) = item_generics(item) else {
        return Vec::new();
    };
    flattening_levels(item)
        .into_iter()
        .flat_map(|(subject, fields, tag)| {
            ranges::flattened_map_beside_errors(&subject, fields, generics, tag.as_deref())
        })
        .chain(ranges::filled_map_errors(item, &args.default_types))
        .chain(ranges::text_read_map_key_errors(item))
        .chain(ranges::filled_text_key_errors(item, &args.default_types))
        .collect()
}

/// Each level of `item` serde writes a flattened member's entries at, with the subject a refusal
/// there opens with and the tag serde writes at that level: a struct, with its own tag, and each
/// variant of an enum serde writes or reads, with the enum's tag where it writes no content key.
#[cfg(feature = "serde")]
fn flattening_levels(item: &Item) -> Vec<(String, &syn::Fields, Option<String>)> {
    let label = item_label(item);
    if let Item::Struct(item_struct) = item {
        vec![(
            format!("{label}: "),
            &item_struct.fields,
            struct_tag(item_struct).map(|(key, _)| key),
        )]
    } else if let Item::Enum(item_enum) = item {
        let tag = match Tagging::of(item_enum) {
            Tagging::Internal { tag } => Some(tag),
            Tagging::Adjacent {
                content: _content,
                tag: _tag,
            } => None,
            Tagging::External | Tagging::Plain | Tagging::Untagged => None,
        };
        item_enum
            .variants
            .iter()
            .filter(|variant| !parse_serde_key_omission(&variant.attrs).absent_from_wire())
            .map(|variant| {
                (
                    format!("{label}: variant `{}`, ", variant.ident),
                    &variant.fields,
                    tag.clone(),
                )
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Nothing, in a build that neither publishes a Zod binding nor reads a value.
#[cfg(not(any(feature = "serde", feature = "zod")))]
const fn record_item(_item: &Item) {}

/// Records the checks an item names for the parts of its expansion written apart from the type's
/// own `validate()`: each bound of it, and its recovering decode.
#[cfg(feature = "serde")]
fn record_item_checks(item: &Item, args: &ModelSchemaArgs) {
    let Some(ident) = item_schema_ident(item) else {
        return;
    };
    let members: Vec<&Field> = if let Item::Struct(item_struct) = item {
        item_struct.fields.iter().collect()
    } else if let Item::Enum(item_enum) = item {
        variant_members(item_enum).collect()
    } else {
        Vec::new()
    };
    record_own_checks(
        &ident_schema_module_name(&ident.to_string()),
        &args.check,
        checks::ItemChecks::of(&args.check, members.into_iter()).named,
    );
}

/// Without `serde` no bound is held and no value is read, so nothing reads an item's checks but
/// its own `validate()`.
#[cfg(not(feature = "serde"))]
const fn record_item_checks(_item: &Item, _args: &ModelSchemaArgs) {}

/// Every member of every variant of `item_enum`, in the order declared.
fn variant_members(item_enum: &syn::ItemEnum) -> impl Iterator<Item = &Field> {
    item_enum
        .variants
        .iter()
        .flat_map(|variant| variant.fields.iter())
}

/// Records which of the two Zod bindings an item publishes, ahead of the shape it is dispatched to.
#[cfg(feature = "zod")]
fn record_own_zod_binding(item: &Item) {
    let Some(generics) = item_generics(item) else {
        return;
    };
    let Some(name) = item_schema_ident(item) else {
        return;
    };
    record_zod_factory(
        &name.to_string(),
        !type_parameters_in_scope(generics).is_empty(),
    );
}

/// Nothing, in a build that reads a value and publishes no Zod binding.
#[cfg(all(feature = "serde", not(feature = "zod")))]
const fn record_own_zod_binding(_item: &Item) {}

/// Records whether the Zod binding `item` publishes calls a `preprocess`: the type names one, or
/// it is another name for a type that does. A type that holds it then reads that binding whole.
#[cfg(feature = "zod")]
fn record_zod_binding_preprocess(item: &Item, args: &ModelSchemaArgs) {
    let Some(name) = item_schema_ident(item) else {
        return;
    };
    let aliased = if let Item::Type(alias) = item {
        get_field_def("", &alias.ty, "").zod_names_a_preprocess()
    } else {
        false
    };
    record_zod_preprocess(&name.to_string(), aliased || !args.preprocess.is_empty());
}

/// Nothing, in a build that publishes no Zod binding.
#[cfg(not(feature = "zod"))]
const fn record_zod_binding_preprocess(_item: &Item, _args: &ModelSchemaArgs) {}

/// The parameters an item declares — the three shapes `model_schema` expands each bind their own;
/// anything else binds none this expansion can read.
const fn item_generics(item: &Item) -> Option<&syn::Generics> {
    if let Item::Struct(item_struct) = item {
        Some(&item_struct.generics)
    } else if let Item::Enum(item_enum) = item {
        Some(&item_enum.generics)
    } else if let Item::Type(item_type) = item {
        Some(&item_type.generics)
    } else {
        None
    }
}

/// The attribute lists an item's own serde renames can be written on, each beside the name a guard
/// message calls it by.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
fn renamable_attribute_lists(item: &Item) -> Vec<(String, &[syn::Attribute])> {
    let mut lists: Vec<(String, &[syn::Attribute])> = vec![(item_label(item), item_attrs(item))];
    if let Item::Struct(item_struct) = item {
        lists.extend(field_attribute_lists(&item_struct.fields));
    } else if let Item::Enum(item_enum) = item {
        for variant in &item_enum.variants {
            lists.push((format!("variant `{}`", variant.ident), &variant.attrs));
            lists.extend(field_attribute_lists(&variant.fields));
        }
    } else {
        // An alias declares no member of its own to rename.
    }
    lists
}

/// The attributes each of `fields` carries, beside the name a guard message calls the field by.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
fn field_attribute_lists(fields: &syn::Fields) -> Vec<(String, &[syn::Attribute])> {
    fields
        .iter()
        .map(|field| {
            let ident = field
                .ident
                .as_ref()
                .map_or_else(String::new, ToString::to_string);
            (field_label(&ident), field.attrs.as_slice())
        })
        .collect()
}

/// The attributes written on the item itself; a shape this macro does not expand carries none it
/// reads.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
fn item_attrs(item: &Item) -> &[syn::Attribute] {
    if let Item::Struct(item_struct) = item {
        &item_struct.attrs
    } else if let Item::Enum(item_enum) = item {
        &item_enum.attrs
    } else if let Item::Type(item_type) = item {
        &item_type.attrs
    } else {
        &[]
    }
}

/// The `compile_error!` tokens for every enum variant written `Foo()` that serde writes or reads,
/// which serde writes with an empty `[]` payload while every surface describes the unit variant
/// `Foo`.
fn empty_tuple_variant_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    let Item::Enum(item_enum) = item else {
        return Vec::new();
    };
    item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
        .filter(|variant| {
            matches!(&variant.fields, syn::Fields::Unnamed(slots) if slots.unnamed.is_empty())
        })
        .map(|variant| {
            let name = &variant.ident;
            syn::Error::new_spanned(
                variant,
                prefixed_guard_message(&format!(
                    "variant `{name}`: `{name}()` is a tuple variant with no field, which serde \
                     writes with an empty `[]` payload, while every surface describes it as the \
                     unit variant `{name}`, so a value of it never crosses. Write `{name}` for a \
                     unit variant, or give it a field."
                )),
            )
            .to_compile_error()
        })
        .collect()
}

/// The `compile_error!` tokens for an enum that declares variants and none serde writes or reads,
/// which leaves no value for a surface to describe.
fn no_variant_on_the_wire_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    let Item::Enum(item_enum) = item else {
        return Vec::new();
    };
    if item_enum.variants.is_empty() || item_enum.variants.iter().any(variant_on_the_wire) {
        return Vec::new();
    }
    let name = &item_enum.ident;
    let message = prefixed_guard_message(&format!(
        "type `{name}`: every variant is under `#[serde(skip)]`, or under `skip_serializing` and \
         `skip_deserializing` together, so serde writes no value of it and reads none, and no \
         surface can describe it. Let serde write or read one of its variants."
    ));
    vec![syn::Error::new_spanned(name, message).to_compile_error()]
}

/// The `compile_error!` tokens every list-form `rename(...)` / `rename_all(...)` the item carries
/// earns whose two directions do not name one key.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
fn rename_direction_guard_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    renamable_attribute_lists(item)
        .into_iter()
        .filter_map(|(subject, attrs)| {
            rename_direction_rejection(attrs)
                .map(|rejection| attr_guard_error(&rejection, &subject))
        })
        .collect()
}

/// Nothing, where no rename is read at all: without the `serde` feature every surface writes the
/// Rust ident, and with no schema feature and no client feature nothing describes the type.
#[cfg(not(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
)))]
const fn rename_direction_guard_errors(_item: &Item) -> Vec<proc_macro2::TokenStream> {
    Vec::new()
}

/// The `compile_error!` tokens of every serde attribute of `item` no target but Rust reads as serde
/// does: a list-form rename whose two directions name two keys, a `#[serde(transparent)]` struct
/// written as the value of one field and read as the value of another, and the `default` of a
/// member of a variant whose JSON no target is handed.
fn serde_attribute_guard_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    let mut refused = rename_direction_guard_errors(item);
    refused.extend(transparent_direction_errors(item));
    refused.extend(variant_default_refusals(item));
    refused
}

/// The `compile_error!` tokens a `#[serde(transparent)]` struct earns where serde writes it as the
/// value of one field and reads it as the value of another, in every build.
fn transparent_direction_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    let Item::Struct(item_struct) = item else {
        return Vec::new();
    };
    transparent_fields_apart(item_struct)
        .map(|[(written_at, written), (read_at, read)]| {
            let message = format!(
                "`#[serde(transparent)]` writes the struct as the value of {} and reads it as the \
                 value of {}, so the payload serde writes is not one serde reads. A schema and an \
                 emitted client describe one value, so neither can be written for the pair. Make \
                 one field the value in both directions, and write `#[serde(skip)]` on every \
                 other field.",
                preprocess::member_label(None, written_at, written),
                preprocess::member_label(None, read_at, read),
            );
            attr_guard_error(
                &syn::Error::new_spanned(&item_struct.ident, message),
                &item_label(item),
            )
        })
        .into_iter()
        .collect()
}

/// The `compile_error!` tokens of every member of a variant of `item` under a `default` of its own
/// that no target but Rust reads as serde does.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
fn variant_default_refusals(item: &Item) -> Vec<proc_macro2::TokenStream> {
    if let Item::Enum(item_enum) = item {
        default_refusals(item_enum)
    } else {
        Vec::new()
    }
}

/// Nothing, where no target but Rust reads a default: without the `serde` feature no attribute is
/// read, and with no schema feature and no client feature nothing describes the type.
#[cfg(not(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
)))]
const fn variant_default_refusals(_item: &Item) -> Vec<proc_macro2::TokenStream> {
    Vec::new()
}

/// The `compile_error!` tokens a `default_types` declaration earns against the item it was written
/// on, checked in both directions: an entry naming nothing the item declares, and — where a JSON
/// document is generated — a parameter the declaration left out.
fn default_types_guard_errors(
    item: &Item,
    args: &ModelSchemaArgs,
) -> Vec<proc_macro2::TokenStream> {
    let Some(generics) = item_generics(item) else {
        return Vec::new();
    };
    let declared: Vec<&syn::Ident> = generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(type_param) => Some(&type_param.ident),
            syn::GenericParam::Const(_) | syn::GenericParam::Lifetime(_) => None,
        })
        .collect();

    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let mut rejections = entries_naming_no_parameter(args, &declared);
    #[cfg(not(any(feature = "zod", feature = "jsonschema")))]
    let rejections = entries_naming_no_parameter(args, &declared);
    #[cfg(feature = "jsonschema")]
    rejections.extend(parameters_left_without_a_default(args, &declared));
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    rejections.extend(fillings_no_document_can_be_built_from(args));
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    rejections.extend(fillings_reaching_an_undescribable_std_type(args));
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    rejections.extend(fillings_reaching_an_unwritable_map_key(args));

    rejections
        .iter()
        .map(|rejection| attr_guard_error(rejection, &item_label(item)))
        .collect()
}

/// The refusal every `default_types` entry that names no type parameter of the item earns, spanned
/// on the entry as written.
fn entries_naming_no_parameter(
    args: &ModelSchemaArgs,
    declared: &[&syn::Ident],
) -> Vec<syn::Error> {
    args.default_types
        .iter()
        .filter(|(name, _)| !declared.contains(&name))
        .map(|(name, _)| syn::Error::new_spanned(name, undeclared_entry_message(name, declared)))
        .collect()
}

/// Why an entry naming nothing is refused, in the two spellings the item earns: one that declares
/// parameters names them back, and one that declares none says so.
fn undeclared_entry_message(name: &syn::Ident, declared: &[&syn::Ident]) -> String {
    if declared.is_empty() {
        return format!(
            "`default_types` entry `{name}` names a type parameter, but this item declares none. A \
             default type is the concrete filling a parameter is described from, so an item with \
             no type parameter has nothing for one to fill."
        );
    }
    let names = declared
        .iter()
        .map(|declared_name| format!("`{declared_name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "`default_types` entry `{name}` names no type parameter of this item. An entry declares \
         the filling for the parameter it names, so one that names none fills nothing while the \
         parameter it was meant for is left with no default at all. Type parameters of this item: \
         {names}."
    )
}

/// The refusal every type parameter the declaration left out earns, spanned on the parameter.
#[cfg(feature = "jsonschema")]
fn parameters_left_without_a_default(
    args: &ModelSchemaArgs,
    declared: &[&syn::Ident],
) -> Vec<syn::Error> {
    declared
        .iter()
        .filter(|name| {
            !args
                .default_types
                .iter()
                .any(|(written, _)| written == **name)
        })
        .map(|name| syn::Error::new_spanned(name, missing_default_message(name, declared)))
        .collect()
}

/// Why a parameter with no default is refused: what the default is for, what the feature has to do
/// with it, and the declaration to write for this item's own parameters.
#[cfg(feature = "jsonschema")]
fn missing_default_message(name: &syn::Ident, declared: &[&syn::Ident]) -> String {
    let sample = declared
        .iter()
        .map(|parameter| format!("{parameter} = String"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "`#[model_schema]` requires a default type for every type parameter, and `{name}` has \
         none. The default type is what the JSON-schema document is generated from: JSON Schema \
         has no type parameters, so without a declared default the macro would have to guess — and \
         a wrong guess produces a document that silently rejects valid payloads. This is required \
         because the `jsonschema` feature is enabled. Declare one for every parameter of this item, \
         each with the type its document should be generated from: \
         `#[model_schema(default_types({sample}))]`."
    )
}

/// The refusal every `default_types` entry earns whose filling names a value neither surface that
/// reads one can build a schema from, spanned on the filling as written.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
fn fillings_no_document_can_be_built_from(args: &ModelSchemaArgs) -> Vec<syn::Error> {
    args.default_types
        .iter()
        .filter_map(|(name, filling)| {
            let written = bare_type_name(filling)?;
            is_undescribable_primitive(&written).then(|| {
                syn::Error::new_spanned(filling, undescribable_filling_message(name, &written))
            })
        })
        .collect()
}

/// The refusal every `default_types` entry earns whose filling reaches a std type serde has no wire
/// form for, at any depth, spanned on the filling as written.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
fn fillings_reaching_an_undescribable_std_type(args: &ModelSchemaArgs) -> Vec<syn::Error> {
    args.default_types
        .iter()
        .filter_map(|(name, filling)| {
            let written = get_field_def("_filling", filling, "");
            let rejection = undescribable_std_rejection(&written)?;
            let message =
                undescribable_std_message(&format!("`default_types` entry `{name}`"), &rejection);
            Some(syn::Error::new_spanned(filling, message))
        })
        .collect()
}

/// The refusal every `default_types` entry earns whose filling reaches a map key no surface can
/// write, at any depth, spanned on the filling as written. Gated with the two surfaces that read a
/// declared filling at all.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
fn fillings_reaching_an_unwritable_map_key(args: &ModelSchemaArgs) -> Vec<syn::Error> {
    args.default_types
        .iter()
        .filter_map(|(name, filling)| {
            let written = get_field_def("_filling", filling, "");
            let rejection = map_key_rejection(&written)?;
            let message =
                map_key_rejection_message(&format!("`default_types` entry `{name}`"), &rejection);
            Some(syn::Error::new_spanned(filling, message))
        })
        .collect()
}

/// The name a type is written as when it is written as a bare name and nothing else — no
/// qualifier, no path, no arguments — or `None` for every other spelling. Only that shape can be
/// checked against reserved names: `some::path::char` names whatever the path resolves to.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
fn bare_type_name(filling: &syn::Type) -> Option<String> {
    let syn::Type::Path(type_path) = filling else {
        return None;
    };
    if type_path.qself.is_some() {
        return None;
    }
    let [segment] = type_path.path.segments.iter().collect::<Vec<_>>()[..] else {
        return None;
    };
    matches!(segment.arguments, syn::PathArguments::None).then(|| segment.ident.to_string())
}

/// Why a filling the dispatch has no arm for is refused: what the filling is read for, what each
/// surface would otherwise name, which features read it, and the way out.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
fn undescribable_filling_message(name: &syn::Ident, written: &str) -> String {
    format!(
        "`default_types` entry `{name}` is filled at `{written}`, which `#[model_schema]` cannot \
         build a schema from. The filling is what the parameter's schema is generated from, and it \
         is rendered through the same dispatch a field's type is — a dispatch that has no arm for \
         `{written}` and so takes it for another `#[model_schema]` item: the JSON document calls \
         into a `{written}_schema` module that nothing publishes, and the zod `$SchemaDefault` \
         names a `{written}$Schema` binding no generated module exports. This is refused wherever \
         a filling is read, which is the `zod` and `jsonschema` features. Fill the parameter at a \
         type the macro describes — a primitive it maps, or a `#[model_schema]` item — or model \
         `{written}` as a newtype over one that carries what the wire holds."
    )
}

/// The compile-time checks a `default_types` declaration earns against the bounds its parameters
/// declare — one per bounded parameter, and one more carrying jointly whatever bound reads a
/// neighbour — emitted beside the expansion rather than in place of it.
fn default_types_bound_checks(
    item: &Item,
    args: &ModelSchemaArgs,
) -> Vec<proc_macro2::TokenStream> {
    if matches!(item, Item::Type(_)) {
        return Vec::new();
    }
    let Some(generics) = item_generics(item) else {
        return Vec::new();
    };
    let mut checks: Vec<proc_macro2::TokenStream> = args
        .default_types
        .iter()
        .filter_map(|(name, filling)| filling_bound_check(generics, name, filling))
        .collect();
    checks.extend(joint_bound_check(generics, args));
    checks
}

/// The checks the compiler answers for `item`: each filling against the bounds its parameter
/// declares, each range written by its bare name against the std range, and each `Value` and
/// `Map` written by its bare name against `serde_json`'s.
fn asked_of_the_compiler(item: &Item, args: &ModelSchemaArgs) -> Vec<proc_macro2::TokenStream> {
    let mut checks = default_types_bound_checks(item, args);
    checks.extend(ranges::identity_checks(item));
    checks.extend(any_json_checks(item, args));
    checks
}

/// The check one filling earns against the bounds its parameter declares alone, or `None` where it
/// declares none such.
fn filling_bound_check(
    generics: &syn::Generics,
    name: &syn::Ident,
    filling: &syn::Type,
) -> Option<proc_macro2::TokenStream> {
    let bounds: Vec<&syn::TypeParamBound> = declared_bounds(generics, name)
        .into_iter()
        .filter(|bound| !mentions_another_parameter(bound, generics, name))
        .collect();
    if bounds.is_empty() {
        return None;
    }
    Some(quote! {
        const _: fn() = || {
            fn default_type_filling<#name: #(#bounds)+*>() {}
            default_type_filling::<#filling>();
        };
    })
}

/// The one check carrying every bound the per-filling pass left behind for reading a neighbour, or
/// `None` where it left none behind or where the item cannot be spelled jointly.
fn joint_bound_check(
    generics: &syn::Generics,
    args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    let consts: Vec<String> = generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Const(const_param) => Some(const_param.ident.to_string()),
            syn::GenericParam::Type(_) | syn::GenericParam::Lifetime(_) => None,
        })
        .collect();
    let mut predicates: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut declared: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut fillings: Vec<&syn::Type> = Vec::new();
    for param in &generics.params {
        match param {
            syn::GenericParam::Lifetime(lifetime_param) => {
                declared.push(quote! { #lifetime_param });
            }
            syn::GenericParam::Const(_) => {}
            syn::GenericParam::Type(type_param) => {
                let name = &type_param.ident;
                let filling = args
                    .default_types
                    .iter()
                    .find(|(declared_name, _)| declared_name == name)
                    .map(|(_, filling)| filling)?;
                declared.push(quote! { #name });
                fillings.push(filling);
                let joint: Vec<&syn::TypeParamBound> = declared_bounds(generics, name)
                    .into_iter()
                    .filter(|bound| {
                        mentions_another_parameter(bound, generics, name)
                            && !reads_any_name(quote::ToTokens::to_token_stream(bound), &consts)
                    })
                    .collect();
                if !joint.is_empty() {
                    predicates.push(quote! { #name: #(#joint)+* });
                }
            }
        }
    }
    if predicates.is_empty() {
        return None;
    }
    Some(quote! {
        const _: fn() = || {
            fn default_type_fillings<#(#declared),*>()
            where
                #(#predicates),*
            {
            }
            default_type_fillings::<#(#fillings),*>();
        };
    })
}

/// Everything a parameter is bounded by, in both places a declaration can put it: beside the
/// parameter, and in the item's `where` clause.
fn declared_bounds<'generics>(
    generics: &'generics syn::Generics,
    name: &syn::Ident,
) -> Vec<&'generics syn::TypeParamBound> {
    let beside_the_parameter = generics.params.iter().filter_map(|param| match param {
        syn::GenericParam::Type(type_param) if type_param.ident == *name => {
            Some(&type_param.bounds)
        }
        syn::GenericParam::Type(_)
        | syn::GenericParam::Const(_)
        | syn::GenericParam::Lifetime(_) => None,
    });
    let in_the_where_clause = generics
        .where_clause
        .iter()
        .flat_map(|where_clause| &where_clause.predicates)
        .filter_map(|predicate| {
            let syn::WherePredicate::Type(predicate_type) = predicate else {
                return None;
            };
            (named_parameter(&predicate_type.bounded_ty) == Some(name))
                .then_some(&predicate_type.bounds)
        });
    beside_the_parameter
        .chain(in_the_where_clause)
        .flatten()
        .collect()
}

/// The parameter a `where` predicate bounds, when it bounds a bare one; a predicate written about
/// `Vec<T>` or `T::Item` bounds that type, not `T`.
fn named_parameter(bounded_ty: &syn::Type) -> Option<&syn::Ident> {
    let syn::Type::Path(type_path) = bounded_ty else {
        return None;
    };
    if type_path.qself.is_some() {
        return None;
    }
    type_path.path.get_ident()
}

/// Whether a bound reads any generic parameter of the item other than the one it bounds. Asked of
/// the tokens rather than the parsed bound: a false positive only withholds a check, costing
/// nothing.
fn mentions_another_parameter(
    bound: &syn::TypeParamBound,
    generics: &syn::Generics,
    name: &syn::Ident,
) -> bool {
    let own = name.to_string();
    let neighbours: Vec<String> = generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Type(type_param) => type_param.ident.to_string(),
            syn::GenericParam::Const(const_param) => const_param.ident.to_string(),
            syn::GenericParam::Lifetime(lifetime_param) => {
                lifetime_param.lifetime.ident.to_string()
            }
        })
        .filter(|declared| *declared != own)
        .collect();
    reads_any_name(quote::ToTokens::to_token_stream(bound), &neighbours)
}

/// Whether a token stream spells any of the given names, at any nesting depth.
fn reads_any_name(tokens: proc_macro2::TokenStream, names: &[String]) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => names.contains(&ident.to_string()),
        proc_macro2::TokenTree::Group(group) => reads_any_name(group.stream(), names),
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

/// Emits the expansion with the tokens it earned beside it standing ahead of it, or the expansion
/// untouched where it earned none.
fn with_prefixed_tokens(expanded: TokenStream, prefix: &[proc_macro2::TokenStream]) -> TokenStream {
    if prefix.is_empty() {
        return expanded;
    }
    quote! {
        #(#prefix)*
        #expanded
    }
}

/// The `compile_error!` tokens an item earns for carrying a ` ```rust example ` block while
/// declaring a const parameter — a doc example is annotated at one type instantiation, which a
/// const cannot fill. Aliases are excluded: they publish no `schema_example()`.
#[cfg(feature = "zod")]
fn const_parameter_example_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    let (generics, attrs) = if let Item::Struct(item_struct) = item {
        (&item_struct.generics, &item_struct.attrs)
    } else if let Item::Enum(item_enum) = item {
        (&item_enum.generics, &item_enum.attrs)
    } else {
        return Vec::new();
    };
    let consts: Vec<&syn::Ident> = generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Const(const_param) => Some(&const_param.ident),
            syn::GenericParam::Type(_) | syn::GenericParam::Lifetime(_) => None,
        })
        .collect();
    let Some(first) = consts.first() else {
        return Vec::new();
    };
    if extract_examples_tokens(attrs).is_empty() {
        return Vec::new();
    }
    vec![attr_guard_error(
        &syn::Error::new_spanned(first, const_parameter_example_message(&consts)),
        &item_label(item),
    )]
}

/// Nothing, in a build that reads no example: the method the refusal is owed for is never built,
/// so the block sits unread the way it does on every other item here.
#[cfg(not(feature = "zod"))]
const fn const_parameter_example_errors(_item: &Item) -> Vec<proc_macro2::TokenStream> {
    Vec::new()
}

/// Why a doc example on a const-declaring item is refused: what the example has to be, why the
/// convention that fills a type parameter reaches no const, what the feature has to do with it, and
/// the two ways out.
#[cfg(feature = "zod")]
fn const_parameter_example_message(consts: &[&syn::Ident]) -> String {
    let names = consts
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "`#[model_schema]` cannot build a `schema_example()` for an item that declares a const \
         parameter, and this item declares {names}. The example is Rust compiled at one \
         instantiation, so its value is annotated with this item's own name and every parameter \
         filled in: a type parameter is filled at the type `default_types` declares for it, or at \
         `String` where it declares none, and a const takes no filling from that convention — both \
         spellings name a type, a const is a value, and no value is the one every \
         const-parameterised example is written at, so one chosen here would render the example at \
         a length this item's author never wrote. This is refused because the `zod` feature is \
         enabled, `zod` being the only surface that reads an example. Remove the ` ```rust example \
         ` block, or the const parameter, whichever this item can do without."
    )
}

/// The `compile_error!` tokens an item earns for handing one of its own const parameters to a
/// generic type it writes, or none where no type written on it does that.
#[cfg(any(feature = "typescript", feature = "jsonschema"))]
fn const_parameter_argument_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    let Some(generics) = item_generics(item) else {
        return Vec::new();
    };
    let consts: Vec<String> = generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Const(const_param) => Some(const_param.ident.to_string()),
            syn::GenericParam::Type(_) | syn::GenericParam::Lifetime(_) => None,
        })
        .collect();
    if consts.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<syn::Ident> = Vec::new();
    for written in item_written_types(item) {
        collect_const_arguments(written, &consts, &mut found);
    }
    found
        .iter()
        .map(|argument| {
            attr_guard_error(
                &syn::Error::new_spanned(
                    argument,
                    const_parameter_argument_message(&argument.to_string()),
                ),
                &item_label(item),
            )
        })
        .collect()
}

/// Every type an item writes out: a struct's field types, an enum's variant field types, an alias's
/// target. The types whose spelling reaches a surface, which is where an argument is written.
#[cfg(any(
    feature = "typescript",
    feature = "jsonschema",
    not(feature = "indexmap")
))]
fn item_written_types(item: &Item) -> Vec<&syn::Type> {
    if let Item::Struct(item_struct) = item {
        item_struct.fields.iter().map(|field| &field.ty).collect()
    } else if let Item::Enum(item_enum) = item {
        item_enum
            .variants
            .iter()
            .flat_map(|variant| variant.fields.iter().map(|field| &field.ty))
            .collect()
    } else if let Item::Type(item_type) = item {
        vec![item_type.ty.as_ref()]
    } else {
        Vec::new()
    }
}

/// Every place one of `consts` is handed to a written type as an argument, collected as the ident
/// it was written at so the refusal points there.
#[cfg(any(feature = "typescript", feature = "jsonschema"))]
fn collect_const_arguments(written: &syn::Type, consts: &[String], found: &mut Vec<syn::Ident>) {
    match written {
        syn::Type::Path(type_path) => {
            for segment in &type_path.path.segments {
                let syn::PathArguments::AngleBracketed(angled) = &segment.arguments else {
                    continue;
                };
                for argument in &angled.args {
                    match argument {
                        syn::GenericArgument::Type(inner) => {
                            if let Some(ident) = bare_ident(inner)
                                && consts.contains(&ident.to_string())
                            {
                                found.push(ident);
                            } else {
                                collect_const_arguments(inner, consts, found);
                            }
                        }
                        syn::GenericArgument::Const(syn::Expr::Path(path)) => {
                            if let Some(ident) = path.path.get_ident()
                                && consts.contains(&ident.to_string())
                            {
                                found.push(ident.clone());
                            }
                        }
                        // A const written as anything but a bare name is an expression; a lifetime
                        // and an associated binding name no const at all.
                        syn::GenericArgument::Const(_)
                        | syn::GenericArgument::Lifetime(_)
                        | syn::GenericArgument::AssocType(_)
                        | syn::GenericArgument::AssocConst(_)
                        | syn::GenericArgument::Constraint(_)
                        | _ => {}
                    }
                }
            }
        }
        syn::Type::Array(array) => collect_const_arguments(&array.elem, consts, found),
        syn::Type::Slice(slice) => collect_const_arguments(&slice.elem, consts, found),
        syn::Type::Reference(reference) => collect_const_arguments(&reference.elem, consts, found),
        syn::Type::Paren(paren) => collect_const_arguments(&paren.elem, consts, found),
        syn::Type::Group(group) => collect_const_arguments(&group.elem, consts, found),
        syn::Type::Tuple(tuple) => {
            for element in &tuple.elems {
                collect_const_arguments(element, consts, found);
            }
        }
        // None of these writes an argument list a const could stand in.
        syn::Type::FnPtr(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_)
        | syn::Type::Ptr(_)
        | syn::Type::TraitObject(_)
        | syn::Type::Verbatim(_)
        | _ => {}
    }
}

/// The ident a type is written as when it is written as a bare name and nothing else, or `None`.
#[cfg(any(feature = "typescript", feature = "jsonschema"))]
fn bare_ident(written: &syn::Type) -> Option<syn::Ident> {
    let syn::Type::Path(type_path) = written else {
        return None;
    };
    if type_path.qself.is_some() {
        return None;
    }
    type_path.path.get_ident().cloned()
}

/// Why a const handed to a written type as an argument is refused: where the const does render,
/// where this one stands instead, what each surface would emit, what the features have to do with
/// it, and the way out.
#[cfg(any(feature = "typescript", feature = "jsonschema"))]
fn const_parameter_argument_message(name: &str) -> String {
    let surfaces = if cfg!(feature = "typescript") && cfg!(feature = "jsonschema") {
        "the `typescript` and `jsonschema` features are enabled, the two surfaces that render an \
         argument list"
    } else if cfg!(feature = "typescript") {
        "the `typescript` feature is enabled, a surface that renders an argument list"
    } else {
        "the `jsonschema` feature is enabled, a surface that renders an argument list"
    };
    format!(
        "`#[model_schema]` cannot render the const parameter `{name}` as a type argument. An \
         argument list is read as a list of types — that is what every surface writes one from — so \
         `{name}` standing in one is taken for a type, and no surface can spell it: the JSON \
         document would call into a `{}_schema` module nothing publishes, and the TypeScript \
         declaration would write `{name}` where nothing binds it, a const being dropped from the \
         declaration it was written on. This is refused because {surfaces}. A const does render as \
         an array length — `[T; {name}]`, which describes as an unbounded array — so hold the value \
         that way, or fill the argument with a type.",
        name.to_lowercase()
    )
}

/// Nothing, in a build that renders no argument list: Zod names the schema the *item* published and
/// a const-declaring item publishes the one schema it has, so the const never stands where a type
/// is read.
#[cfg(not(any(feature = "typescript", feature = "jsonschema")))]
const fn const_parameter_argument_errors(_item: &Item) -> Vec<proc_macro2::TokenStream> {
    Vec::new()
}

/// The `compile_error!` tokens an item earns for each type it writes under a name the `indexmap`
/// feature reads, in a build without the feature. A name the crate declared a type of its own
/// under, the item's among them, is that type.
#[cfg(not(feature = "indexmap"))]
fn indexmap_feature_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    let mut found = Vec::new();
    for written in item_written_types(item) {
        collect_indexmap_names(written, &mut found);
    }
    let own = item_schema_ident(item);
    found
        .into_iter()
        .filter(|(named, _)| {
            own.is_none_or(|ident| ident != *named) && !is_declared_ident(&named.to_string())
        })
        .map(|(named, read_as)| {
            syn::Error::new_spanned(named, indexmap_feature_message(&named.to_string(), read_as))
                .to_compile_error()
        })
        .collect()
}

/// Nothing, in a build that reads the names.
#[cfg(feature = "indexmap")]
const fn indexmap_feature_errors(_item: &Item) -> Vec<proc_macro2::TokenStream> {
    Vec::new()
}

/// The `compile_error!` tokens of [`any_json_rejections`].
fn any_json_errors(item: &Item, args: &ModelSchemaArgs) -> Vec<proc_macro2::TokenStream> {
    any_json_rejections(item, args)
        .iter()
        .map(syn::Error::to_compile_error)
        .collect()
}

/// The refusal an item earns for each member, alias target and `default_types` filling that
/// reaches `serde_json::Value` or `serde_json::Map` written by its full path, each spanned on the
/// type as written. Asked ahead of every shape, so every build refuses it. A bare name is asked of
/// the compiler instead ([`any_json_checks`]).
fn any_json_rejections(item: &Item, args: &ModelSchemaArgs) -> Vec<syn::Error> {
    let label = item_label(item);
    any_json_positions(item, args)
        .into_iter()
        .filter_map(|(subject, ty)| {
            let message = any_json_message(&label, &subject, ranges::any_json_named(ty)?);
            Some(syn::Error::new_spanned(ty, message))
        })
        .collect()
}

/// The check, beside the item, of each `Value` and `Map` a member, an alias target or a
/// `default_types` filling writes by its bare name: the build fails with the refusal of
/// [`any_json_rejections`] where the name is `serde_json`'s type. A name the item declares as a
/// parameter, the item's own and one the crate declared a type under above are that type.
fn any_json_checks(item: &Item, args: &ModelSchemaArgs) -> Vec<proc_macro2::TokenStream> {
    let (Some(own), Some(generics)) = (item_schema_ident(item), item_generics(item)) else {
        return Vec::new();
    };
    let parameters: Vec<String> = generics
        .params
        .iter()
        .map(|parameter| match parameter {
            syn::GenericParam::Type(type_param) => type_param.ident.to_string(),
            syn::GenericParam::Const(const_param) => const_param.ident.to_string(),
            syn::GenericParam::Lifetime(lifetime_param) => {
                lifetime_param.lifetime.ident.to_string()
            }
        })
        .collect();
    let kept = |name: &str| {
        *own == name
            || generics
                .type_params()
                .any(|parameter| parameter.ident == name)
            || is_declared_ident(name)
    };
    let label = item_label(item);
    any_json_positions(item, args)
        .into_iter()
        .flat_map(|(subject, ty)| {
            ranges::any_json_bare(ty, kept, &parameters)
                .into_iter()
                .map(|(name, probed)| {
                    ranges::any_json_check(name, probed, &any_json_message(&label, &subject, name))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Each type an item writes beside the subject a refusal of a JSON value names it by: every
/// member, an alias's target, and each `default_types` filling.
fn any_json_positions<'item>(
    item: &'item Item,
    args: &'item ModelSchemaArgs,
) -> Vec<(String, &'item syn::Type)> {
    let mut written: Vec<(String, &syn::Type)> = Vec::new();
    if let Item::Struct(item_struct) = item {
        written.extend(
            item_struct
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| (preprocess::member_label(None, index, field), &field.ty)),
        );
    } else if let Item::Enum(item_enum) = item {
        for variant in &item_enum.variants {
            written.extend(variant.fields.iter().enumerate().map(|(index, field)| {
                (
                    preprocess::member_label(Some(variant), index, field),
                    &field.ty,
                )
            }));
        }
    } else if let Item::Type(item_type) = item {
        written.push(("the type it names".to_owned(), &item_type.ty));
    } else {
        return Vec::new();
    }
    written.extend(
        args.default_types
            .iter()
            .map(|(name, filling)| (format!("`default_types` entry `{name}`"), filling)),
    );
    written
}

/// The refusal of `name`, `serde_json`'s `Value` or `Map`, reached at `subject` of the item
/// `label` names.
fn any_json_message(label: &str, subject: &str, name: &str) -> String {
    let message = undescribable_std_message(subject, &UndescribableStd::AnyJson(name));
    prefixed_guard_message(&format!("{label}: {message}"))
}

/// Every name of [`INDEXMAP_NAMES`] `written` holds at any depth, written with the type arguments
/// the feature reads it with, beside what the feature reads it as.
#[cfg(not(feature = "indexmap"))]
fn collect_indexmap_names<'ty>(
    written: &'ty syn::Type,
    found: &mut Vec<(&'ty syn::Ident, &'static str)>,
) {
    let current = written_type(written);
    if let syn::Type::Path(type_path) = current {
        let last = type_path.path.segments.len().saturating_sub(1);
        for (position, segment) in type_path.path.segments.iter().enumerate() {
            let syn::PathArguments::AngleBracketed(angled) = &segment.arguments else {
                continue;
            };
            let held: Vec<&syn::Type> = angled
                .args
                .iter()
                .filter_map(|argument| {
                    if let syn::GenericArgument::Type(inner) = argument {
                        Some(inner)
                    } else {
                        None
                    }
                })
                .collect();
            if position == last
                && let Some((_, read_as, _)) = INDEXMAP_NAMES
                    .iter()
                    .find(|(name, _, least)| segment.ident == name && held.len() >= *least)
            {
                found.push((&segment.ident, read_as));
            }
            for inner in held {
                collect_indexmap_names(inner, found);
            }
        }
    } else if let syn::Type::Array(array) = current {
        collect_indexmap_names(&array.elem, found);
    } else if let syn::Type::Slice(slice) = current {
        collect_indexmap_names(&slice.elem, found);
    } else if let syn::Type::Reference(reference) = current {
        collect_indexmap_names(&reference.elem, found);
    } else if let syn::Type::Paren(paren) = current {
        collect_indexmap_names(&paren.elem, found);
    } else if let syn::Type::Tuple(tuple) = current {
        for element in &tuple.elems {
            collect_indexmap_names(element, found);
        }
    } else {
        // No other type is written with a name a feature reads.
    }
}

/// The refusal of a name the `indexmap` feature reads, written where the feature is off.
#[cfg(not(feature = "indexmap"))]
fn indexmap_feature_message(name: &str, read_as: &str) -> String {
    format!(
        "`{name}` is read as {read_as} under tixschema's `indexmap` feature: add `features = \
         [\"indexmap\"]` to the `tixschema` dependency"
    )
}

/// The `compile_error!` tokens an item earns for each `#[model_schema_prop]` written on a field of
/// a `#[serde(transparent)]` brand, its slot or any other, or none where no field carries one.
fn branded_slot_prop_errors(item: &Item) -> Vec<proc_macro2::TokenStream> {
    let Item::Struct(item_struct) = item else {
        return Vec::new();
    };
    let Some(slot) = brand_field(item_struct) else {
        return Vec::new();
    };
    item_struct
        .fields
        .iter()
        .flat_map(|field| {
            let message = if ptr::eq(field, slot) {
                BRANDED_SLOT_PROP_MESSAGE
            } else {
                UNREAD_BRAND_FIELD_PROP_MESSAGE
            };
            field
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("model_schema_prop"))
                .map(move |attr| (attr, message))
        })
        .map(|(attr, message)| {
            attr_guard_error(&syn::Error::new_spanned(attr, message), &item_label(item))
        })
        .collect()
}

/// The `compile_error!` tokens for every `cfg_attr`-wrapped serde attribute an enum carries: on the
/// type (tagging, variant casing) and on each variant (`rename`). Collected before the enum is
/// dispatched, since all three shapes read those same attributes.
#[cfg(feature = "serde")]
fn enum_cfg_attr_guard_errors(
    item_enum: &syn::ItemEnum,
    type_meta: &SerdeTypeMeta,
) -> Vec<proc_macro2::TokenStream> {
    let name = &item_enum.ident;
    type_meta
        .cfg_attr_rejection
        .as_ref()
        .map(|rejection| cfg_attr_guard_error(rejection, &format!("type `{name}`")))
        .into_iter()
        .chain(item_enum.variants.iter().filter_map(|variant| {
            parse_serde_field_attributes(&variant.attrs)
                .cfg_attr_rejection
                .as_ref()
                .map(|rejection| {
                    cfg_attr_guard_error(rejection, &format!("variant `{}`", variant.ident))
                })
        }))
        .collect()
}

/// The `compile_error!` tokens for a branded newtype whose inner type is `Option`, or `None` for
/// every other inner type.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_option_inner_error(
    name: &Ident,
    inner_field: &Field,
) -> Option<proc_macro2::TokenStream> {
    get_field_def("_inner", &inner_field.ty, "")
        .is_optional()
        .then(|| {
            syn::Error::new_spanned(
                inner_field,
                format!(
                    "model_schema: branded newtype `{name}` wraps an `Option`, which has no \
                     representable schema: #[serde(transparent)] writes a `None` to the wire as \
                     `null` (skip_serializing_if cannot suppress it — a transparent newtype has \
                     no key to omit), while the generated brand renders the inner type alone. \
                     Brand the inner type and make the use site optional instead: `Option<{name}>`."
                ),
            )
            .to_compile_error()
        })
}

/// The `compile_error!` tokens for a branded newtype that applies `pattern`, `minLength`, or
/// `maxLength` to an inner type whose schema is not a string, or `None` when the inner can carry
/// them. A list carries the two lengths, which count its items, and no `pattern`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_constraint_inner_error(
    generics: &syn::Generics,
    name: &Ident,
    inner_field: &Field,
    args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    if !args.has_string_constraints() {
        return None;
    }
    if held_as_a_list(&inner_field.ty) {
        return args.pattern.as_ref().map(|_| {
            let message = format!(
                "model_schema: branded newtype `{name}` writes `pattern` on a list, which has no \
                 text for it to match. `minLength` and `maxLength` count the list's items, and \
                 nothing written on the brand reaches them. A rule for each item goes on the \
                 item's own type: declare the item as a `#[serde(transparent)]` brand that \
                 carries the `pattern`."
            );
            syn::Error::new_spanned(inner_field, message).to_compile_error()
        });
    }
    let inner = branded_inner_value_surface(generics, inner_field);
    if let FieldDefType::SiblingType(inner_name, _) = &inner.field_type
        && let Some(parameter) = inner.argument_parameter_name()
    {
        let message = format!(
            "model_schema: branded newtype `{name}` applies string constraints (pattern, \
             minLength, maxLength) to `{inner_name}`, whose schema this crate builds from the \
             type parameter `{parameter}` — so the checks land on whatever the instantiation \
             supplies for it, and one declaration covers every instantiation: Zod appends \
             `.min`/`.max` to a schema the call site decides the shape of, JSON Schema writes \
             them beside the `{{}}` a parameter describes as, where they go inert, and \
             `validate()` measures the inner's `Display` rendering — three surfaces, three \
             answers. Brand a string-typed inner, or drop the constraints."
        );
        return Some(syn::Error::new_spanned(inner_field, message).to_compile_error());
    }
    if !inner.is_array()
        && let FieldDefType::TypeParam(parameter) = &inner.field_type
    {
        let default = declared_default_field(parameter, &args.default_types);
        // `None` here means string-shaped, the `String` fallback for an entry the declaration left
        // out included.
        return non_string_inner_shape(&default).map(|_| {
            let default_ty = declared_default_type_name(parameter, &args.default_types);
            let message = declared_default_constraint_message(name, parameter, &default_ty);
            syn::Error::new_spanned(inner_field, message).to_compile_error()
        });
    }
    let message = match (&inner.field_type, non_string_inner_shape(&inner)) {
        (FieldDefType::SiblingType(inner_name, _), Some(shape)) => {
            named_inner_constraint_message(&name.to_string(), inner_name, shape)
        }
        (_, Some(shape)) => format!(
            "model_schema: branded newtype `{name}` applies string constraints (pattern, \
             minLength, maxLength) to a {shape} inner type, which cannot carry them: Zod reads \
             `.min`/`.max` as bounds on the value itself and has no regex check for a non-string \
             schema, JSON Schema ignores `minLength`/`maxLength`/`pattern` outside `\"type\": \
             \"string\"`, and `validate()` measures the inner's `Display` rendering — three \
             surfaces, three answers. Brand a string-typed inner, or drop the constraints."
        ),
        (_, None) => return None,
    };
    Some(syn::Error::new_spanned(inner_field, message).to_compile_error())
}

/// The Rust spelling of `parameter`'s declared default, for the refusal that names it — the same
/// `String` fallback [`declared_default_field`] resolves a `FieldDef` from. Practically
/// unreachable here, but total anyway rather than leaving a lookup miss unspelled.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn declared_default_type_name(
    parameter: &str,
    default_types: &[(syn::Ident, syn::Type)],
) -> String {
    default_types
        .iter()
        .find(|(declared, _)| declared == parameter)
        .map_or_else(|| "String".to_owned(), |(_, ty)| quote! { #ty }.to_string())
}

/// Why a constrained brand's bare-parameter inner is refused when its *declared default* — not the
/// parameter — turns out not to be string-shaped: the default is what `$SchemaDefault` composes
/// the checks onto, so a default the checks cannot measure is refused in its place.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn declared_default_constraint_message(name: &Ident, parameter: &str, default_ty: &str) -> String {
    format!(
        "model_schema: branded newtype `{name}` applies string constraints (pattern, minLength, \
         maxLength), but its declared default for `{parameter}` is `{default_ty}`, which cannot \
         carry them: Zod reads `.min`/`.max` as bounds on the value itself and has no regex check \
         for a non-string schema, JSON Schema ignores `minLength`/`maxLength`/`pattern` outside \
         `\"type\": \"string\"`, and `validate()` measures the inner's `Display` rendering. \
         Declare a string-typed default, or drop the constraints."
    )
}

/// `parameter`'s declared default as a `syn::Type`, for substituting a concrete Rust type rather
/// than spelling one for a diagnostic. Falls back to `String` for an entry left out — the same
/// fallback [`declared_default_type_name`] and [`declared_default_field`] use, so all three agree.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn declared_default_syn_type(
    parameter: &str,
    default_types: &[(syn::Ident, syn::Type)],
) -> syn::Type {
    default_types
        .iter()
        .find(|(declared, _)| declared == parameter)
        .map_or_else(|| syn::parse_quote!(String), |(_, ty)| ty.clone())
}

/// The consult a constrained brand leaves unanswered, for the expansion that registers the name to
/// answer, or `None` for every brand the registry could answer at its own expansion.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn deferred_shape_question(
    item_struct: &syn::ItemStruct,
    args: &ModelSchemaArgs,
) -> Option<ShapeQuestion> {
    if !args.has_string_constraints() {
        return None;
    }
    let inner = branded_inner_value_surface(&item_struct.generics, brand_field(item_struct)?);
    if inner.is_array() || sequence_wrapper_element(&inner).is_some() {
        return None;
    }
    let FieldDefType::SiblingType(inner_name, arguments) = &inner.field_type else {
        return None;
    };
    if lookup_alias_info(inner_name).is_some() {
        return None;
    }
    Some(ShapeQuestion {
        argument_shapes: arguments.iter().map(non_string_inner_shape).collect(),
        brand: item_struct.ident.to_string(),
        inner: inner_name.clone(),
    })
}

/// What a name a brand asked about publishes, once the registration answering it has landed —
/// `None` where that is a shape a string check lands on, and where the reference wrote no argument
/// at the position the registration published.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn answered_shape(
    published: PublishedShape,
    argument_shapes: &[Option<&'static str>],
) -> Option<&'static str> {
    match published {
        PublishedShape::Flat(shape) => shape,
        PublishedShape::Parameter(position) => *argument_shapes.get(position)?,
    }
}

/// The `compile_error!` tokens an item owes the constrained brands that asked about its name before
/// it existed, spanned on the item's own name — the only tokens this expansion holds.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn deferred_shape_refusals(registering: Option<&Ident>) -> Vec<proc_macro2::TokenStream> {
    let Some(name) = registering else {
        return Vec::new();
    };
    let rust_ident = name.to_string();
    let questions = shape_questions_for(&rust_ident);
    // Asked before the record is read, so the expansion of an item no brand named — which is most
    // of them — never pays for the lookup.
    if questions.is_empty() {
        return Vec::new();
    }
    let Some(info) = lookup_alias_info(&rust_ident) else {
        return Vec::new();
    };
    questions
        .into_iter()
        .filter_map(|question| {
            let shape = answered_shape(info.value_shape, &question.argument_shapes)?;
            let message = named_inner_constraint_message(&question.brand, &rust_ident, shape);
            Some(syn::Error::new_spanned(name, message).to_compile_error())
        })
        .collect()
}

/// The same, where no surface reads a published shape and so no brand ever asked.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
const fn deferred_shape_refusals(
    _registering: Option<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    Vec::new()
}

/// Every named field, tuple slot and (for an alias) target type an item declares, with the label
/// its own refusal message names it by — [`field_label`] for a struct or enum-variant field,
/// [`item_label`] for an alias's own target.
#[cfg(feature = "swift")]
fn swift_scanned_fields(item: &Item) -> Vec<(String, &syn::Type)> {
    if let Item::Struct(item_struct) = item {
        let mut found = Vec::new();
        collect_swift_scanned_fields(&item_struct.fields, &mut found);
        found
    } else if let Item::Enum(item_enum) = item {
        let mut found = Vec::new();
        for variant in &item_enum.variants {
            collect_swift_scanned_fields(&variant.fields, &mut found);
        }
        found
    } else if let Item::Type(item_type) = item {
        vec![(item_label(item), &item_type.ty)]
    } else {
        Vec::new()
    }
}

#[cfg(feature = "swift")]
fn collect_swift_scanned_fields<'item>(
    fields: &'item syn::Fields,
    found: &mut Vec<(String, &'item syn::Type)>,
) {
    match fields {
        syn::Fields::Named(named) => {
            for field in &named.named {
                let raw_name = field
                    .ident
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string);
                found.push((field_label(&raw_name), &field.ty));
            }
        }
        syn::Fields::Unit => {}
        syn::Fields::Unnamed(unnamed) => {
            for field in &unnamed.unnamed {
                found.push((field_label(""), &field.ty));
            }
        }
    }
}

/// The refused primitive `field_def`, or anything it directly contains (a map's key or value, a
/// tuple's slots, a generic argument), names — `None` where nothing under it is a `u64` or
/// `usize`. [`refuses_swift`] answers only for one `FieldDef`'s own type; this walks every one.
#[cfg(feature = "swift")]
fn swift_refused_primitive_name(field_def: &FieldDef) -> Option<&'static str> {
    if refuses_swift(field_def) {
        return Some(if matches!(field_def.field_type, FieldDefType::Usize) {
            "usize"
        } else {
            "u64"
        });
    }
    match &field_def.field_type {
        FieldDefType::Map(key, value) => {
            swift_refused_primitive_name(key).or_else(|| swift_refused_primitive_name(value))
        }
        FieldDefType::SiblingType(_, arguments) => {
            arguments.iter().find_map(swift_refused_primitive_name)
        }
        FieldDefType::Range(_, bound) => swift_refused_primitive_name(bound),
        FieldDefType::Result(ok, err) => {
            swift_refused_primitive_name(ok).or_else(|| swift_refused_primitive_name(err))
        }
        FieldDefType::Tuple(elements) => elements.iter().find_map(swift_refused_primitive_name),
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::I8
        | FieldDefType::Isize
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::U8
        | FieldDefType::Unknown
        | FieldDefType::Usize => None,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => None,
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => None,
    }
}

/// The `compile_error!` tokens an item earns for a `u64` or `usize` field, under `swift` — one
/// per declared field or slot that names one anywhere in its type, spanned on that declared type.
#[cfg(feature = "swift")]
fn swift_width_refusals(item: &Item) -> Vec<proc_macro2::TokenStream> {
    swift_scanned_fields(item)
        .into_iter()
        .filter_map(|(label, ty)| {
            let field_def = get_field_def("", ty, "");
            let refused = swift_refused_primitive_name(&field_def)?;
            let message = format!(
                "{label}: `{refused}` has no Swift mapping; the Swift target refuses unsigned \
                 64-bit and pointer-sized integers"
            );
            Some(syn::Error::new_spanned(ty, prefixed_guard_message(&message)).to_compile_error())
        })
        .collect()
}

#[cfg(not(feature = "swift"))]
const fn swift_width_refusals(_item: &Item) -> Vec<proc_macro2::TokenStream> {
    Vec::new()
}

/// How a constrained brand over a name the registry proves publishes no string is refused, in the
/// one wording both orders of the two declarations reach it by.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn named_inner_constraint_message(brand: &str, inner: &str, shape: &str) -> String {
    format!(
        "model_schema: branded newtype `{brand}` applies string constraints (pattern, minLength, \
         maxLength) to `{inner}`, which this crate writes as the {shape} value the checks are then \
         appended to — and that binding carries no string for them to measure: Zod either reads \
         `.min`/`.max` as a bound on something else or has no such check on it at all, JSON Schema \
         ignores `minLength`/`maxLength`/`pattern` outside `\"type\": \"string\"`, and `validate()` \
         measures the inner's `Display` rendering — three surfaces, three answers. Brand a \
         string-typed inner, or drop the constraints."
    )
}

/// Names the shape an inner type resolves to that has no string for the constraints to measure, or
/// `None` when it writes exactly one and so can carry them.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn non_string_inner_shape(inner: &FieldDef) -> Option<&'static str> {
    if inner.is_array() || sequence_wrapper_element(inner).is_some() {
        return Some("container");
    }
    match &inner.field_type {
        FieldDefType::SiblingType(inner_name, arguments) => {
            instantiated_value_shape(inner_name, arguments)
        }
        FieldDefType::Map(..)
        | FieldDefType::Range(..)
        | FieldDefType::Result(..)
        | FieldDefType::Tuple(..) => Some("container"),
        FieldDefType::Boolean | FieldDefType::BooleanLiteral(_) => Some("boolean"),
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::NumberLiteral(_) => Some("numeric"),
        FieldDefType::TypeParam(_) | FieldDefType::Unknown => Some("opaque"),
        // A `char` writes a one-character string, and `validate()` reaches it as it reaches a
        // numeric or boolean inner: through `Display`.
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => None,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => None,
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => None,
    }
}

/// What a registered name filled with `arguments` publishes as a value, and `None` for a name the
/// registry has no answer for — one declared below the type reading it, or one this crate never
/// expands at all.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn instantiated_value_shape(rust_ident: &str, arguments: &[FieldDef]) -> Option<&'static str> {
    match lookup_alias_info(rust_ident)?.value_shape {
        PublishedShape::Flat(shape) => shape,
        PublishedShape::Parameter(position) => non_string_inner_shape(arguments.get(position)?),
    }
}

/// The JSON type keyword a recorded name's wire describes as, when the leaves `leaves_of` reads
/// prove that wire is no object, and `None` for every name they cannot prove one way or the other.
#[cfg(feature = "serde")]
fn registered_non_object_wire(rust_ident: &str, leaves_of: LeavesOf) -> Option<&'static str> {
    match leaves_of(rust_ident)?.as_slice() {
        [only] => only.non_object,
        _ => None,
    }
}

/// The leaves a schema surface recorded for `rust_ident`.
#[cfg(all(feature = "serde", any(feature = "zod", feature = "typescript")))]
fn surfaced_leaves(rust_ident: &str) -> Option<Vec<WireLeaf>> {
    lookup_alias_info(rust_ident).map(|info| info.wire)
}

/// The leaves every build with serde recorded for `rust_ident`.
#[cfg(feature = "serde")]
fn written_leaves(rust_ident: &str) -> Option<Vec<WireLeaf>> {
    recorded_non_object_wire(rust_ident).map(|wire| wire.leaves)
}

/// The JSON type keyword a value one keyword describes writes, `null` for `()`, and `None` for
/// every type that writes a document no single keyword names — a composite, a name, an
/// `ObjectId`'s `$oid` object, an opaque value.
#[cfg(any(feature = "jsonschema", feature = "serde"))]
const fn scalar_json_type_keyword(field_type: &FieldDefType) -> Option<&'static str> {
    if let FieldDefType::Tuple(elements) = field_type
        && elements.is_empty()
    {
        return Some("null");
    }
    match *field_type {
        FieldDefType::Boolean | FieldDefType::BooleanLiteral(_) => Some("boolean"),
        FieldDefType::F32 | FieldDefType::F64 | FieldDefType::NumberLiteral(_) => Some("number"),
        FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::Usize => Some("integer"),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            Some("string")
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => Some("string"),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => None,
        FieldDefType::Map(..)
        | FieldDefType::Range(..)
        | FieldDefType::Result(..)
        | FieldDefType::SiblingType(..)
        | FieldDefType::Tuple(..)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown => None,
    }
}

/// What the value surface a `#[model_schema()]` item publishes under a name is, as the
/// constrained-brand guard reads shapes — the answer [`crate::utils::record_value_shape`] stores
/// for every name a brand can then reach through.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn published_value_shape(written: &FieldDef, parameters: &[String]) -> PublishedShape {
    let mut target = written.clone();
    target.erase_type_parameters(parameters);
    if target.is_optional() {
        return PublishedShape::Flat(Some("nullable"));
    }
    published_parameter_position(&target, parameters).map_or_else(
        || PublishedShape::Flat(non_string_inner_shape(&target)),
        PublishedShape::Parameter,
    )
}

/// The position of the item's own parameter a written target *is*, and `None` for every target that
/// fixes a shape of its own around one.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn published_parameter_position(target: &FieldDef, parameters: &[String]) -> Option<usize> {
    if target.is_array() || sequence_wrapper_element(target).is_some() {
        return None;
    }
    let name = target.parameter_shape_name()?;
    parameters.iter().position(|declared| declared == name)
}

/// The `compile_error!` tokens for every `cfg_attr`-wrapped serde attribute a branded newtype
/// carries: on the type and on its inner slot, which is positional and so has no name to print.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn branded_cfg_attr_guard_errors(
    item_struct: &syn::ItemStruct,
    inner_field: &Field,
) -> Vec<proc_macro2::TokenStream> {
    let name = &item_struct.ident;
    parse_serde_type_attributes(&item_struct.attrs)
        .cfg_attr_rejection
        .as_ref()
        .map(|rejection| cfg_attr_guard_error(rejection, &format!("type `{name}`")))
        .into_iter()
        .chain(
            parse_serde_field_attributes(&inner_field.attrs)
                .cfg_attr_rejection
                .as_ref()
                .map(|rejection| cfg_attr_guard_error(rejection, &field_label(""))),
        )
        .collect()
}

#[cfg(all(
    not(feature = "serde"),
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
const fn branded_cfg_attr_guard_errors(
    _item_struct: &syn::ItemStruct,
    _inner_field: &Field,
) -> Vec<proc_macro2::TokenStream> {
    Vec::new()
}

/// The `compile_error!` tokens for a brand whose `pattern` argument a guard refuses, or `None`
/// when it carries none or it clears them. An enum answers for its own in
/// [`enum_string_check_refusal_output`]; every other shape is refused the argument before a surface
/// is built.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_pattern_error(name: &Ident, args: &ModelSchemaArgs) -> Option<proc_macro2::TokenStream> {
    args.pattern_rejection
        .as_ref()
        .map(|rejection| pattern_guard_error(rejection, &format!("type `{name}`")))
}

/// The inner type as the two validating surfaces receive it, with the brand's own type parameters
/// already classified as parameters. The guard and the registration both read the inner through
/// this one call, so what a brand is refused for and what it records cannot come apart.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_inner_value_surface(generics: &syn::Generics, inner_field: &Field) -> FieldDef {
    surface_field_def(generics, &get_field_def("_inner", &inner_field.ty, ""))
}

/// What the registry records for a brand.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_alias_kind(inner_field: &Field) -> AliasKind {
    let inner = get_field_def("", &inner_field.ty, "");
    match map_key_path(&inner) {
        MapKeyPath::Open => AliasKind::StringWire,
        MapKeyPath::Unnarrowed => AliasKind::Stringified,
        MapKeyPath::Refused(_) => AliasKind::NoEnumMembers,
        MapKeyPath::Enumerated(inner_name) => match registered_key_kind(inner_name) {
            Some(AliasKind::EnumMembers) => AliasKind::StringWire,
            Some(AliasKind::Stringified) => AliasKind::Stringified,
            _ => AliasKind::NoEnumMembers,
        },
    }
}

/// The form a key written under a brand's name renders in, read off the one field it is written
/// with — the same inner [`branded_alias_kind`] reads.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_key_wire(inner_field: &Field) -> MapKeyWire {
    get_field_def("", &inner_field.ty, "").map_key_wire()
}

/// The `compile_error!` tokens for a branded newtype whose inner reaches a std type serde has no
/// wire form for, or `None` when it reaches none. Spanned on the inner as written.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_undescribable_std_error(
    name: &Ident,
    inner_field: &Field,
) -> Option<proc_macro2::TokenStream> {
    let inner = get_field_def("", &inner_field.ty, "");
    let rejection = undescribable_std_rejection(&inner)?;
    let message = prefixed_guard_message(&undescribable_std_message(
        &format!("type `{name}`"),
        &rejection,
    ));
    Some(syn::Error::new_spanned(&inner_field.ty, message).to_compile_error())
}

/// The `compile_error!` tokens for a branded newtype whose inner reaches a map key no surface can
/// write, or `None` when it reaches none.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_map_key_error(name: &Ident, inner_field: &Field) -> Option<proc_macro2::TokenStream> {
    let inner = get_field_def("", &inner_field.ty, "");
    let rejection = map_key_rejection(&inner)?;
    let message = prefixed_guard_message(&map_key_rejection_message(
        &format!("type `{name}`"),
        &rejection,
    ));
    Some(syn::Error::new_spanned(&inner_field.ty, message).to_compile_error())
}

/// The `compile_error!` tokens for every guard a branded newtype violates: a `cfg_attr`-wrapped
/// serde attribute on the type or on its inner slot, an `Option` inner type, string constraints
/// over an inner type that cannot carry them, a `pattern` that is not a regex, a std type the inner
/// reaches that serde has no wire form for, and a map key the inner reaches that no surface can
/// write.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_guard_errors(
    item_struct: &syn::ItemStruct,
    args: &ModelSchemaArgs,
) -> Vec<proc_macro2::TokenStream> {
    let inner_field = brand_field(item_struct).unwrap();
    branded_cfg_attr_guard_errors(item_struct, inner_field)
        .into_iter()
        .chain(branded_option_inner_error(&item_struct.ident, inner_field))
        .chain(branded_constraint_inner_error(
            &item_struct.generics,
            &item_struct.ident,
            inner_field,
            args,
        ))
        .chain(branded_pattern_error(&item_struct.ident, args))
        .chain(branded_undescribable_std_error(
            &item_struct.ident,
            inner_field,
        ))
        .chain(branded_map_key_error(&item_struct.ident, inner_field))
        .collect()
}

/// The `compile_error!` tokens for a `#[serde(flatten)]` field the registry proves serde writes as
/// a plain enum's variant name, [`written_as_a_plain_enum`], or `None` for every other field. Read
/// from the declaration, not the collected field, so the diagnostic keeps the span of the type it
/// rejects, and names that type with the arguments written for it.
#[cfg(feature = "serde")]
fn flattened_field_guard_error(
    field: &syn::Field,
    type_name: &str,
) -> Option<proc_macro2::TokenStream> {
    let inner = get_field_def("_flattened", &field.ty, "");
    let FieldDefType::SiblingType(name, _) = &inner.field_type else {
        return None;
    };
    if !written_as_a_plain_enum(&inner) {
        return None;
    }
    let inner_name = filled_spelling(&field.ty, name);
    let field_name = field_label(
        &field
            .ident
            .as_ref()
            .map_or_else(String::new, ToString::to_string),
    );
    Some(
        syn::Error::new_spanned(
            &field.ty,
            format!(
                "model_schema: {field_name} of `{type_name}` carries `#[serde(flatten)]` over \
                 `{inner_name}`, which serde does not write as an object: a plain enum writes its \
                 own variant name, so it contributes no members to the object being written — \
                 serde writes that name as a key holding null, which a schema closed around the \
                 remaining fields rejects. Write the field as a named member so the value gets a \
                 key of its own, or flatten a type serde writes as an object. \
                 {FLATTENED_PLAIN_ENUM_SCOPE}"
            ),
        )
        .to_compile_error(),
    )
}

/// The attributes the emitted field keeps: everything but this crate's own `model_schema_prop`,
/// which is inert to every derive — left on the emitted item, rustc reports it as an attribute
/// that does not exist.
fn declaration_attrs(field: &Field) -> Vec<syn::Attribute> {
    field
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("model_schema_prop"))
        .cloned()
        .collect()
}

/// `item` with every `#[model_schema_prop]` taken off its members: what a refused item is
/// emitted as, the attribute being one rustc reports as nonexistent wherever a copy survives.
fn without_member_keys(item: &Item) -> Item {
    let mut stripped = item.clone();
    if let Item::Struct(item_struct) = &mut stripped {
        for field in &mut item_struct.fields {
            field.attrs = declaration_attrs(field);
        }
    } else if let Item::Enum(item_enum) = &mut stripped {
        for variant in &mut item_enum.variants {
            for field in &mut variant.fields {
                field.attrs = declaration_attrs(field);
            }
        }
    } else {
        // An alias has no member.
    }
    stripped
}

/// [`guard_failure_output`] for an item refused before its members are walked: the item is
/// re-emitted [`without_member_keys`].
fn refused_item_output(
    item: &Item,
    guard_errors: &[proc_macro2::TokenStream],
) -> Option<TokenStream> {
    if guard_errors.is_empty() {
        return None;
    }
    guard_failure_output(
        &without_member_keys(item),
        item_schema_ident(item),
        guard_errors,
    )
}

/// Writes the serde attributes generation held back onto the fields they were generated for.
fn apply_deferred_field_attrs<'field>(
    fields: impl Iterator<Item = &'field mut Field>,
    deferred: Vec<Vec<syn::Attribute>>,
) {
    for (field, attrs) in fields.zip(deferred) {
        field.attrs.extend(attrs);
    }
}

/// The `compile_error!` tokens for a `#[serde(flatten)]` field whose recorded wire proves serde
/// does not write an object where the merge needs one, or `None` for every other field and for one
/// [`flattened_field_guard_error`] refuses. A generic type declared above that serde writes as what
/// fills its type parameter is read at that filling, and named with the arguments written for it;
/// any other type is read by its name, and quoted by its [`union_spelling`].
#[cfg(feature = "serde")]
fn flatten_edge_guard_error(
    field: &syn::Field,
    type_name: &str,
) -> Option<proc_macro2::TokenStream> {
    let inner = get_field_def("_flattened", &field.ty, "");
    let (inner_name, non_object, scope) = if let FieldDefType::SiblingType(name, _) =
        &inner.field_type
        && !written_in_place(&inner)
    {
        if written_as_a_plain_enum(&inner) {
            return None;
        }
        let (inner_name, non_object) = value_filling(&without_its_option(&inner)).map_or_else(
            || {
                (
                    union_spelling(&field.ty, name),
                    flattened_name_non_object(&inner),
                )
            },
            |filling| {
                (
                    filled_spelling(&field.ty, name),
                    flattened_filling_non_object(filling),
                )
            },
        );
        (inner_name, non_object, FLATTENED_NON_OBJECT_SCOPE)
    } else {
        let bare = without_its_option(&inner);
        (
            written_spelling(&field.ty),
            written_non_object_wire(&bare, written_leaves).map(NonObject::Whole),
            FLATTENED_IN_PLACE_SCOPE,
        )
    };
    let message = match non_object? {
        NonObject::Branch(branch, named) => format!(
            "model_schema: `{type_name}`: `#[serde(flatten)]` of `{inner_name}` writes a \
             union member that is not an object — its branch {branch} describes a `{named}`, \
             which has no members to merge, and what serde writes for that member does not \
             join the object being written; write the field as a named member so the value \
             gets a key of its own. {scope}"
        ),
        NonObject::Written(branch, written) => {
            let words = WrittenWords::of(written, false);
            format!(
                "model_schema: `{type_name}`: `#[serde(flatten)]` of `{inner_name}` writes a \
                 union member as {} — its branch {branch} describes a `{}`, {}; write the field \
                 as a named member so the value gets a key of its own. {}",
                words.member, words.keyword, words.serde_writes, words.scope
            )
        }
        NonObject::Whole("null") => format!(
            "model_schema: `{type_name}`: `#[serde(flatten)]` of `{inner_name}` is written as \
             `()` — its schema describes a `null`: serde writes no key for it, and reads the \
             object back, while the TypeScript type and the Zod schema intersect the object \
             with `null`, which admits no value; write the field as a named member so the \
             value gets a key of its own. {scope}"
        ),
        NonObject::Whole(named) => format!(
            "model_schema: `{type_name}`: `#[serde(flatten)]` of `{inner_name}` is not written \
             as an object — its schema describes a `{named}`, which has no members to merge, \
             and what serde writes for it does not join the object being written; write the \
             field as a named member so the value gets a key of its own. {scope}"
        ),
    };
    Some(syn::Error::new_spanned(&field.ty, message).to_compile_error())
}

/// Whether a flattened field's type, under its `Option`, is written in place rather than named: a
/// list, a sequence or anything but a name, which [`written_non_object_wire`] answers for.
#[cfg(feature = "serde")]
fn written_in_place(inner: &FieldDef) -> bool {
    let bare = without_its_option(inner);
    if bare.array_depth > 0 {
        return true;
    }
    let FieldDefType::SiblingType(name, _) = &bare.field_type else {
        return true;
    };
    is_sequence_wrapper(name)
}

/// What a flattened field's type writes as no object, read by its name: what every build with serde
/// records, which reads an untagged member at its generic filling, ahead of what a Zod surface
/// recorded, so the branch named is the same in every build.
#[cfg(feature = "serde")]
fn flattened_name_non_object(inner: &FieldDef) -> Option<NonObject> {
    #[cfg(feature = "zod")]
    let surfaced = || surfaced_non_object(inner);
    #[cfg(not(feature = "zod"))]
    let surfaced = || None;
    written_non_object(inner).or_else(surfaced)
}

/// What a flattened field's type writes as no object, as the registry a Zod surface writes proves
/// it.
#[cfg(all(feature = "serde", feature = "zod"))]
fn surfaced_non_object(inner: &FieldDef) -> Option<NonObject> {
    inner
        .zod_union_members()
        .into_iter()
        .find_map(|member| {
            NonObject::member(&member.branch_path(), member.non_object, member.written_as)
        })
        .or_else(|| flattened_name_refused_branch(inner, surfaced_leaves))
        .or_else(|| flattened_name_non_object_wire(inner, surfaced_leaves).map(NonObject::Whole))
}

/// What a flattened field's type writes as no object, as the record every build with serde writes
/// proves it.
#[cfg(feature = "serde")]
fn written_non_object(inner: &FieldDef) -> Option<NonObject> {
    written_union_members(inner)
        .into_iter()
        .find_map(|member| {
            NonObject::member(&member.branch_path(), member.non_object, member.written_as)
        })
        .or_else(|| flattened_name_refused_branch(inner, written_leaves))
        .or_else(|| flattened_name_non_object_wire(inner, written_leaves).map(NonObject::Whole))
}

/// The members of the untagged union `def` names, as every build with serde records them, each
/// leaf the union writes as what fills a type parameter read at the argument `def` fills it with,
/// and nothing for a def that names no such union.
#[cfg(feature = "serde")]
fn written_union_members(def: &FieldDef) -> Vec<WireLeaf> {
    if def.array_depth > 0 {
        return Vec::new();
    }
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return Vec::new();
    };
    let mut members = recorded_non_object_wire(name).map_or_else(Vec::new, |wire| wire.members);
    for parameter in union_parameters(name) {
        let Some(argument) = arguments.get(parameter.at) else {
            continue;
        };
        // The record keeps its leaves in branch order, and drops a union whose every leaf is an
        // object, so the filling's leaves go where the parameter's leaf sorts.
        members.retain(|leaf| leaf.branch != parameter.branch);
        let at = members.partition_point(|leaf| leaf.branch < parameter.branch);
        let filled = member_written_leaves(argument).into_iter().map(|mut leaf| {
            leaf.branch = [parameter.branch.as_slice(), &leaf.branch].concat();
            leaf
        });
        members.splice(at..at, filled);
    }
    members
}

/// The JSON type keyword the item a flattened field names publishes, where the name is the whole of
/// what the field writes and the leaves `leaves_of` reads prove that wire is no object — and `None`
/// everywhere the declaration is left as it stands.
#[cfg(feature = "serde")]
fn flattened_name_non_object_wire(inner: &FieldDef, leaves_of: LeavesOf) -> Option<&'static str> {
    if inner.array_depth > 0 || written_as_a_plain_enum(inner) {
        return None;
    }
    let FieldDefType::SiblingType(name, _) = &inner.field_type else {
        return None;
    };
    registered_non_object_wire(name, leaves_of)
}

/// The branch a flattened field's own name proves is no object, beside the JSON type keyword that
/// branch describes as — and `None` for every name whose leaves prove no such branch.
#[cfg(feature = "serde")]
fn flattened_name_refused_branch(inner: &FieldDef, leaves_of: LeavesOf) -> Option<NonObject> {
    if inner.array_depth > 0 || written_as_a_plain_enum(inner) {
        return None;
    }
    let FieldDefType::SiblingType(name, _) = &inner.field_type else {
        return None;
    };
    refused_branch(&leaves_of(name)?)
}

/// The branch `leaves` prove is no object, beside the JSON type keyword that branch describes as,
/// where they offer one absence and every other leaf is no object — and `None` everywhere else.
#[cfg(feature = "serde")]
fn refused_branch(leaves: &[WireLeaf]) -> Option<NonObject> {
    let (absences, values): (Vec<&WireLeaf>, Vec<&WireLeaf>) =
        leaves.iter().partition(|leaf| leaf.is_published_absence());
    if absences.len() != 1 {
        return None;
    }
    if !values.iter().all(|leaf| leaf.non_object.is_some()) {
        return None;
    }
    let first = values.first()?;
    NonObject::member(&first.branch_path(), first.non_object, first.written_as)
}

/// What a flattened generic type declared above writes as no object, read at `filling`, the
/// argument it writes as its own value: what [`filled_non_object`] answers, and for an `Option`
/// the branch its leaves refuse, as a flattened struct of one slot over that `Option` is refused.
#[cfg(feature = "serde")]
fn flattened_filling_non_object(filling: &FieldDef) -> Option<NonObject> {
    if filling.is_optional() {
        refused_branch(&filled_wire_leaves(filling))
    } else {
        filled_non_object(filling)
    }
}

/// The [`FieldContext::container_read_back`] every walk hands its fields: whether the item they
/// belong to derives `Deserialize`.
#[cfg(feature = "serde")]
fn container_is_read_back(attrs: &[syn::Attribute]) -> bool {
    derives_deserialize(attrs)
}

#[cfg(not(feature = "serde"))]
const fn container_is_read_back(_attrs: &[syn::Attribute]) -> bool {
    false
}

/// Processes every field of a struct, returning the regular field defs, the `#[serde(flatten)]`
/// field defs, the per-field serde validation functions and `validate()` body fragments, and the
/// `compile_error!` tokens for any field-level guard violations.
fn collect_struct_fields(
    fields: &mut syn::Fields,
    rename_all: Option<&str>,
    module_name_opt: Option<&str>,
    type_name: &str,
    generics: &syn::Generics,
    container: &StructContainer<'_>,
) -> StructFieldData {
    let type_parameters = type_parameters_in_scope(generics);
    let mut field_defs = Vec::new();
    let mut flattened_fields: Vec<FieldDef> = Vec::new();
    let mut validation_fns: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut validate_bodies: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut guard_errors: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut deferred_attrs: Vec<Vec<syn::Attribute>> = Vec::new();
    let steps = steps_of_members(fields, &[], rename_all);

    for (index, field) in fields.iter_mut().enumerate() {
        // Read before the field is processed, which strips the attributes the declaration carried.
        let is_flatten = is_flattened_field(field);

        #[cfg(feature = "serde")]
        if is_flatten {
            guard_errors.extend(flattened_field_guard_error(field, type_name));
            guard_errors.extend(flatten_edge_guard_error(field, type_name));
        }

        let ctx = FieldContext {
            container_defaulted: container.defaulted,
            container_read_back: container.read_back,
            enum_args: None,
            member_steps: &steps,
            on_the_wire: true,
            rename_all,
            schema_module_name: module_name_opt,
            type_name,
            type_parameters: &type_parameters,
            variant_ident: None,
        };
        let (mut f_def, validation_fn, validate_body, field_guard_errors) =
            process_field(&ctx, field, index, &mut deferred_attrs);
        let rust_name = field
            .ident
            .as_ref()
            .map_or_default(|ident| ident.unraw().to_string());
        let serde_default = (container.serde_default)(&rust_name);
        #[cfg(any(feature = "zod", feature = "jsonschema"))]
        f_def.serde_default.clone_from(&serde_default);
        #[cfg(not(any(feature = "zod", feature = "jsonschema")))]
        let _: (&mut FieldDef, Option<usize>) = (&mut f_def, serde_default);

        guard_errors.extend(field_guard_errors);

        if is_flatten {
            let _: (&_, &_) = (&validation_fn, &validate_body);
            // The walk is rebuilt here, the body above being discarded with the rest of a flattened
            // field's. The hop writes no key of its own.
            let place = member_place(field, index, false, ctx.steps(index));
            #[cfg(feature = "serde")]
            if let Some(body) = nested_validate_body(field, &f_def, &place) {
                validate_bodies.push(body);
            }
            validate_bodies.extend(held_member_checks(field, &f_def, &place));
            flattened_fields.push(f_def);
            continue;
        }

        if let Some(vfn) = validation_fn {
            validation_fns.push(vfn);
        }
        if let Some(vb) = validate_body {
            validate_bodies.push(vb);
        }
        push_described_field(&mut field_defs, f_def);
    }

    if guard_errors.is_empty() {
        apply_deferred_field_attrs(fields.iter_mut(), deferred_attrs);
    }

    (
        field_defs,
        flattened_fields,
        validation_fns,
        validate_bodies,
        guard_errors,
    )
}

/// The refusal of a type-level string check written where nothing enforces it: on a struct that
/// is no brand, and on a brand in a build with no schema surface, where no reader is generated
/// for it.
fn string_check_refusal_output(
    item_struct: &syn::ItemStruct,
    args: &ModelSchemaArgs,
) -> Option<TokenStream> {
    let (check, written_at) = args.string_check?;
    let brand = is_branded_newtype(item_struct);
    // With a schema surface on, a brand's check is hung on its reader.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    if brand {
        return None;
    }
    let refusal = if brand {
        format!(
            "`{check}` on a brand is enforced by the reader generated with its schema surface, \
             and this build generates none. Turn on `typescript`, `zod` or `jsonschema`, or \
             remove the check."
        )
    } else {
        no_brand_string_check_refusal(check)
    };
    let error = syn::Error::new(
        written_at,
        prefixed_guard_message(&format!("type `{}`: {refusal}", item_struct.ident)),
    )
    .to_compile_error();
    refused_item_output(&Item::Struct(item_struct.clone()), &[error])
}

/// What a type-level string check is told on a declared type that is no brand.
fn no_brand_string_check_refusal(check: &str) -> String {
    format!(
        "`{check}` is supported only on a branded newtype: a `#[serde(transparent)]` struct \
         serde writes as the value of one field. On a field, write it in \
         `#[model_schema_prop(...)]`."
    )
}

/// The refusal of a string check written on an enum that cannot hold as written, or `None` for
/// one that does and for an enum that writes none.
fn enum_string_check_refusal_output(
    item_enum: &syn::ItemEnum,
    args: &ModelSchemaArgs,
) -> Option<TokenStream> {
    refused_item_output(
        &Item::Enum(item_enum.clone()),
        &enum_string_check_errors(item_enum, args),
    )
}

/// What an enum's string check is refused for: a `pattern` no surface reads, a check that reaches
/// nothing, and each name of a unit variant that breaks it.
fn enum_string_check_errors(
    item_enum: &syn::ItemEnum,
    args: &ModelSchemaArgs,
) -> Vec<proc_macro2::TokenStream> {
    let Some((check, written_at)) = args.string_check else {
        return Vec::new();
    };
    let name = &item_enum.ident;
    if let Some(rejection) = &args.pattern_rejection {
        return vec![pattern_guard_error(rejection, &format!("type `{name}`"))];
    }
    if !enum_check_reaches_nothing(item_enum) {
        return unit_variant_name_errors(item_enum, args);
    }
    let refusal = format!(
        "`{check}` reaches nothing here: an enum's check holds each variant written with no data \
         and each field written as a string or a path that carries no `literal`, and `{name}` has \
         neither"
    );
    vec![
        syn::Error::new(
            written_at,
            prefixed_guard_message(&format!("type `{name}`: {refusal}")),
        )
        .to_compile_error(),
    ]
}

/// Whether an enum's string check has nothing to hold: no variant written with no data, and no
/// field the check reaches.
fn enum_check_reaches_nothing(item_enum: &syn::ItemEnum) -> bool {
    !item_enum.variants.iter().any(|variant| {
        written_with_no_data(variant) || variant.fields.iter().any(reached_by_enums_check)
    })
}

/// Whether serde writes or reads `variant` under its name alone, with no data.
fn written_with_no_data(variant: &syn::Variant) -> bool {
    variant_on_the_wire(variant) && matches!(variant_wire_kind(variant), VariantKind::Unit)
}

/// Each name serde writes or reads for `variant`, the written one first, beside the direction it
/// is named in. An untagged enum writes a variant with no data as `null`, under no name.
#[cfg(feature = "serde")]
fn variant_wire_names(
    item_enum: &syn::ItemEnum,
    variant: &syn::Variant,
) -> Vec<(&'static str, String)> {
    if has_serde_untagged(&item_enum.attrs) {
        return Vec::new();
    }
    let ident = variant.ident.to_string();
    let written = get_final_variant_name(
        &ident,
        parse_written_renames(&variant.attrs).rename.as_deref(),
        parse_written_renames(&item_enum.attrs)
            .rename_all
            .as_deref(),
    );
    let read = get_final_variant_name(
        &ident,
        parse_read_renames(&variant.attrs).rename.as_deref(),
        parse_read_renames(&item_enum.attrs).rename_all.as_deref(),
    );
    let mut names = vec![("written", written)];
    let mut read_under = parse_serde_field_attributes(&variant.attrs).aliases;
    read_under.insert(0, read);
    for name in read_under {
        if names.iter().all(|(_, known)| *known != name) {
            names.push(("read", name));
        }
    }
    names
}

/// Without `serde` no attribute is read, and every surface writes the variant's ident.
#[cfg(not(feature = "serde"))]
fn variant_wire_names(
    _item_enum: &syn::ItemEnum,
    variant: &syn::Variant,
) -> Vec<(&'static str, String)> {
    vec![("written", variant.ident.to_string())]
}

/// The checks of `args` that `name` breaks, each as it is written, measured as a field's validator
/// measures a value.
fn checks_broken_by(
    name: &str,
    args: &ModelSchemaArgs,
    pattern: Option<&regex::Regex>,
) -> Vec<String> {
    let too_short = args
        .min_length
        .filter(|min_len| name.len() < *min_len)
        .map(|min_len| format!("`minLength = {min_len}`"));
    let too_long = args
        .max_length
        .filter(|max_len| name.len() > *max_len)
        .map(|max_len| format!("`maxLength = {max_len}`"));
    let unmatched = pattern
        .filter(|built| !built.is_match(name))
        .map(|built| format!("`pattern = {:?}`", built.as_str()));
    too_short
        .into_iter()
        .chain(too_long)
        .chain(unmatched)
        .collect()
}

/// One `compile_error!` per name of a variant written with no data that breaks the enum's string
/// check, spanned on the variant.
fn unit_variant_name_errors(
    item_enum: &syn::ItemEnum,
    args: &ModelSchemaArgs,
) -> Vec<proc_macro2::TokenStream> {
    let type_name = &item_enum.ident;
    let pattern = args
        .pattern
        .as_deref()
        .and_then(|written| regex::Regex::new(written).ok());
    let mut errors: Vec<proc_macro2::TokenStream> = Vec::new();
    for variant in &item_enum.variants {
        if !written_with_no_data(variant) {
            continue;
        }
        for (direction, name) in variant_wire_names(item_enum, variant) {
            let broken = checks_broken_by(&name, args, pattern.as_ref());
            if broken.is_empty() {
                continue;
            }
            let refusal = format!(
                "type `{type_name}`: variant `{}` is {direction} `{name}`, which breaks {} written \
                 on the enum",
                variant.ident,
                broken.join(" and ")
            );
            errors.push(
                syn::Error::new_spanned(variant, prefixed_guard_message(&refusal))
                    .to_compile_error(),
            );
        }
    }
    errors
}

/// The refusal of a type-level string check written on a type alias: the alias is another name
/// for its target, so there is no type of its own for the check, or a reader of it, to belong to.
fn alias_string_check_refusal_output(
    item_type: &ItemType,
    args: &ModelSchemaArgs,
) -> Option<TokenStream> {
    let (check, written_at) = args.string_check?;
    let name = &item_type.ident;
    let target = written_spelling(&item_type.ty);
    let refusal = format!(
        "`{check}` cannot be written on a type alias: `{name}` is another name for `{target}`, \
         so there is no type of its own for the check to belong to. Declare a branded newtype \
         instead — `#[model_schema({check} = …)] #[serde(transparent)] pub struct \
         {name}({target});` — or, on a field, write the check in `#[model_schema_prop(...)]`."
    );
    let error = syn::Error::new(
        written_at,
        prefixed_guard_message(&format!("type `{name}`: {refusal}")),
    )
    .to_compile_error();
    guard_failure_output(item_type, Some(name), &[error])
}

/// Records what serde writes `item` as, for the expansions after it in every build: a bare wire
/// scalar, an untagged enum, a map key the `bson` library refuses, a model or an alias that holds
/// a map under such a key, and a model or an alias that keys a map by its own type parameters.
fn record_wire_form(item: &Item) {
    record_wire_scalar_item(item);
    record_untagged_enum_item(item);
    record_bson_refused_key_item(item);
    record_bson_refused_holder_item(item);
    record_forwarded_key_maps_item(item);
}

/// Records `item` where serde writes it as a bare wire scalar rather than an object: a
/// `#[serde(transparent)]` single-slot tuple struct over one, and an alias of one.
fn record_wire_scalar_item(item: &Item) {
    let Some((name, inner)) = wire_scalar_candidate(item) else {
        return;
    };
    if is_wire_scalar_type(inner) {
        record_wire_scalar(&name.to_string());
    }
}

/// Records `item` where it is an enum declared `#[serde(untagged)]`, or an alias of one, for
/// [`is_recorded_untagged_enum`](crate::utils::is_recorded_untagged_enum) to answer a later
/// item's own refusal. Alias transitivity: `type MyError = SomeUntaggedEnum;` is untagged too.
fn record_untagged_enum_item(item: &Item) {
    if let Item::Enum(item_enum) = item {
        if has_serde_untagged(&item_enum.attrs) {
            record_untagged_enum(&item_enum.ident.to_string());
        }
        return;
    }
    if let Item::Type(item_type) = item
        && is_recorded_untagged_enum(&item_type.ty)
    {
        record_untagged_enum(&item_type.ident.to_string());
    }
}

/// Records `item` where it is a brand or an alias whose value is a key the `bson` library refuses:
/// a number or a `bool`, or another such brand or alias. One whose value reaches one of its own
/// type parameters is recorded with the parameter's position instead.
fn record_bson_refused_key_item(item: &Item) {
    let Some((name, inner)) = wire_scalar_candidate(item) else {
        return;
    };
    if let Some(position) = forwarded_parameter_position(item, inner) {
        record_forwarded_key_parameter(&name.to_string(), position);
        return;
    }
    if is_bson_refused_key(inner) {
        record_bson_refused_key(&name.to_string());
    }
}

/// The position among `item`'s type parameters of the one `inner` reaches: written as that
/// parameter's bare name, under any `Box`, `Rc`, `Arc` and `Cow`, and through a brand or an alias
/// declared above at the position its own value reaches.
fn forwarded_parameter_position(item: &Item, inner: &syn::Type) -> Option<usize> {
    let syn::Type::Path(named) = written_type(inner) else {
        return None;
    };
    if named.qself.is_some() {
        return None;
    }
    if let Some(parameter) = named.path.get_ident() {
        return item_generics(item)?
            .type_params()
            .position(|declared| declared.ident == *parameter);
    }
    let last = named.path.segments.last()?;
    let name = last.ident.to_string();
    let at = if is_ownership_wrapper(&name) {
        0
    } else {
        forwarded_key_parameter(&name)?
    };
    forwarded_parameter_position(item, type_arguments_of(&last.arguments).nth(at)?)
}

/// Records `item` where it is a struct, an enum or an alias that holds a map whose key the `bson`
/// library refuses, with the first such map, whatever its `enable` list: a holder that lists
/// `"mongodb"` is the one refused.
fn record_bson_refused_holder_item(item: &Item) {
    if let (Some(ident), Some(described)) = (
        item_schema_ident(item),
        first_held_map(item, KeyAsked::Refused),
    ) {
        record_bson_refused_holder(&ident.to_string(), &described);
    }
}

/// The first map `item` holds under the key `asked`: in an alias's value, and after the member
/// that holds it in a struct or an enum.
fn first_held_map(item: &Item, asked: KeyAsked) -> Option<String> {
    if let Item::Type(item_type) = item {
        bson_refused_keys(&item_type.ty, asked)
            .first()
            .map(RefusedKey::described)
    } else {
        first_map_described(&bson_refused_maps(item, asked))
    }
}

/// Records `item` where it is a struct, an enum or an alias that keys a map by one of its own type
/// parameters, with the position of each such parameter and the first map it keys, every
/// parameter spelled as [`parameter_placeholder`] writes it.
fn record_forwarded_key_maps_item(item: &Item) {
    let (Some(ident), Some(generics)) = (item_schema_ident(item), item_generics(item)) else {
        return;
    };
    let parameters: Vec<&syn::Ident> = generics
        .type_params()
        .map(|declared| &declared.ident)
        .collect();
    // One name holds one record: a value that reaches a parameter keeps the one
    // `record_bson_refused_key_item` writes.
    if parameters.is_empty()
        || wire_scalar_candidate(item)
            .is_some_and(|(_, inner)| forwarded_parameter_position(item, inner).is_some())
    {
        return;
    }
    let placed = with_parameters_placed(item, &parameters);
    let maps: Vec<(usize, String)> = (0..parameters.len())
        .filter_map(|at| {
            first_held_map(&placed, KeyAsked::Parameter(at)).map(|described| (at, described))
        })
        .collect();
    if !maps.is_empty() {
        record_forwarded_key_maps(&ident.to_string(), maps);
    }
}

/// `item` with each of `parameters` its members' types, or its alias's value, write spelled as
/// the [`parameter_placeholder`] of its position.
fn with_parameters_placed(item: &Item, parameters: &[&syn::Ident]) -> Item {
    let mut placed = item.clone();
    let types: Vec<&mut syn::Type> = if let Item::Struct(item_struct) = &mut placed {
        item_struct
            .fields
            .iter_mut()
            .map(|field| &mut field.ty)
            .collect()
    } else if let Item::Enum(item_enum) = &mut placed {
        item_enum
            .variants
            .iter_mut()
            .flat_map(|variant| variant.fields.iter_mut())
            .map(|field| &mut field.ty)
            .collect()
    } else if let Item::Type(item_type) = &mut placed {
        vec![&mut *item_type.ty]
    } else {
        Vec::new()
    };
    for ty in types {
        if let Ok(written) = syn::parse2(placed_tokens(
            quote::ToTokens::to_token_stream(&*ty),
            parameters,
        )) {
            *ty = written;
        }
    }
    placed
}

/// `tokens` with each ident among `parameters` spelled as the [`parameter_placeholder`] of its
/// position.
fn placed_tokens(tokens: TokenStream, parameters: &[&syn::Ident]) -> TokenStream {
    tokens
        .into_iter()
        .map(|tree| match tree {
            proc_macro2::TokenTree::Ident(ident) => {
                let placeholder = parameters
                    .iter()
                    .position(|parameter| *parameter == &ident)
                    .map(|at| proc_macro2::Ident::new(&parameter_placeholder(at), ident.span()));
                proc_macro2::TokenTree::Ident(placeholder.unwrap_or(ident))
            }
            proc_macro2::TokenTree::Group(group) => {
                let mut placed = proc_macro2::Group::new(
                    group.delimiter(),
                    placed_tokens(group.stream(), parameters),
                );
                placed.set_span(group.span());
                proc_macro2::TokenTree::Group(placed)
            }
            proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => tree,
        })
        .collect()
}

/// The name and inner type of the two shapes that can publish a bare scalar under their own name.
fn wire_scalar_candidate(item: &Item) -> Option<(&syn::Ident, &syn::Type)> {
    if let Item::Type(item_type) = item {
        return Some((&item_type.ident, &item_type.ty));
    }
    let Item::Struct(item_struct) = item else {
        return None;
    };
    Some((&item_struct.ident, &brand_field(item_struct)?.ty))
}

/// How a brand's field is reached on a value of the brand: by its name, or as the slot at
/// `position` among the struct's slots.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn brand_member(position: usize, field: &Field) -> syn::Member {
    field.ident.clone().map_or_else(
        || syn::Member::Unnamed(syn::Index::from(position)),
        syn::Member::Named,
    )
}

/// Whether a struct is a branded newtype: one serde writes as the value of a single field.
fn is_branded_newtype(item_struct: &syn::ItemStruct) -> bool {
    brand_field(item_struct).is_some()
}

/// Computes the TypeScript name, schema-module name, and module ident for a struct, and registers
/// it in the alias registry so other types can resolve references to it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn struct_module_idents(
    name: &syn::Ident,
    name_override: Option<&str>,
    surface: Surface,
) -> (String, String, Ident) {
    let item_name = compute_item_export_name(&name.to_string(), name_override);
    let module_name = ident_schema_module_name(&name.to_string());
    let module_ident = Ident::new(&module_name, name.span());
    register_alias_info(
        &name.to_string(),
        &item_name,
        &module_name,
        AliasKind::NoEnumMembers,
    );
    surface.record(&name.to_string());
    (item_name, module_name, module_ident)
}

/// The enum counterpart of [`struct_module_idents`]. `kind` differs per enum shape: only a plain
/// unit enum gets an `enum_members()` (and is written as `z.enum`); every other shape is a union
/// of its variants' members.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn enum_module_idents(
    name: &syn::Ident,
    item_name: &str,
    kind: AliasKind,
    surface: Surface,
) -> (String, Ident) {
    let module_name = ident_schema_module_name(&name.to_string());
    let module_ident = Ident::new(&module_name, name.span());
    register_alias_info(&name.to_string(), item_name, &module_name, kind);
    // The `z.enum` a plain enum publishes is the one enumeration a key can be written under
    // directly; every other shape reaches one only through a brand or an alias.
    if kind == AliasKind::EnumMembers {
        record_key_wire(&name.to_string(), MapKeyWire::Enumerated);
    }
    surface.record(&name.to_string());
    (module_name, module_ident)
}

/// Extracts a struct's doc lines and each ` ```rust example ` block from them, unconditionally:
/// the example half already answers none where `zod` — the only surface that reads one — is off.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn struct_docs_and_example(
    item_struct: &syn::ItemStruct,
) -> (Option<Vec<String>>, Vec<proc_macro2::TokenStream>) {
    let docs_vec = get_struct_docs(item_struct);
    #[cfg(feature = "zod")]
    let example_tokens = extract_examples_tokens(&item_struct.attrs);
    #[cfg(not(feature = "zod"))]
    let example_tokens = Vec::new();
    (docs_vec, example_tokens)
}

/// The output a struct carrying a `cfg_attr`-wrapped serde attribute on the type is refused with,
/// or `None` when it carries none. A hidden `rename_all` reshapes every field name, so the item is
/// refused before a single field is processed.
#[cfg(feature = "serde")]
fn struct_cfg_attr_guard_output(
    item_struct: &syn::ItemStruct,
    rejection: Option<&syn::Error>,
) -> Option<TokenStream> {
    let ident = &item_struct.ident;
    let error = cfg_attr_guard_error(rejection?, &format!("type `{ident}`"));
    refused_item_output(&Item::Struct(item_struct.clone()), &[error])
}
/// The struct's own `rename_all`, or the `cfg_attr` refusal that pre-empts reading anything.
#[cfg(feature = "serde")]
fn struct_rename_all(item_struct: &syn::ItemStruct) -> Result<Option<String>, TokenStream> {
    let serde_type_meta = parse_serde_type_attributes(&item_struct.attrs);
    if let Some(output) =
        struct_cfg_attr_guard_output(item_struct, serde_type_meta.cfg_attr_rejection.as_ref())
    {
        return Err(output);
    }
    Ok(serde_type_meta.rename_all)
}

/// The struct's own `JSDoc` body: [`build_jsdoc_body`] under `typescript`, empty text otherwise.
#[cfg(feature = "typescript")]
fn struct_docs_body(doc_comment: Option<&[String]>, item_name: &str) -> String {
    build_jsdoc_body(doc_comment, item_name)
}

#[cfg(all(
    not(feature = "typescript"),
    any(feature = "zod", feature = "jsonschema")
))]
const fn struct_docs_body(_doc_comment: Option<&[String]>, _item_name: &str) -> String {
    String::new()
}

/// Puts the key a struct's own `#[serde(tag = "...")]` writes ahead of the struct's members, as
/// one whose only value is the struct's serde name, or refuses a field that writes the same key:
/// serde would write it twice.
#[cfg(feature = "serde")]
fn with_struct_tag(
    item_struct: &syn::ItemStruct,
    mut collected: StructFieldData,
) -> StructFieldData {
    let Some((key, named)) = struct_tag(item_struct) else {
        return collected;
    };
    if collected.0.iter().any(|member| member.name == key) {
        let refusal = format!(
            "type `{}`: `#[serde(tag = \"{key}\")]` writes the struct's name under `{key}`, and \
             a field of the struct writes that key too, so serde would write it twice. Rename \
             one of the two.",
            item_struct.ident
        );
        collected.4.push(
            syn::Error::new_spanned(&item_struct.ident, prefixed_guard_message(&refusal))
                .to_compile_error(),
        );
        return collected;
    }
    let string: syn::Type = syn::parse_quote!(String);
    let mut member = get_field_def(&key, &string, &build_jsdoc_body(None, &key));
    member.field_type = FieldDefType::StringLiteral(named);
    collected.0.insert(0, member);
    collected
}

/// What a struct's fields collect into, with the key its own tag writes ahead of them.
fn struct_members(
    item_struct: &mut syn::ItemStruct,
    rename_all: Option<&str>,
    module_name_opt: Option<&str>,
    container_defaulted: bool,
) -> StructFieldData {
    // Read before the walk: the attributes it leaves on the item are the ones the read is
    // derived from, which this reads as well.
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let serde_defaults = SerdeDefaults::of(item_struct);
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let serde_default = |field: &str| serde_defaults.index(None, field);
    #[cfg(not(any(feature = "zod", feature = "jsonschema")))]
    let serde_default = |_: &str| None;
    let from_fields = collect_struct_fields(
        &mut item_struct.fields,
        rename_all,
        module_name_opt,
        &item_struct.ident.to_string(),
        &item_struct.generics,
        &StructContainer {
            defaulted: container_defaulted,
            read_back: container_is_read_back(&item_struct.attrs),
            serde_default: &serde_default,
        },
    );
    #[cfg(feature = "serde")]
    {
        with_struct_tag(item_struct, from_fields)
    }
    #[cfg(not(feature = "serde"))]
    {
        from_fields
    }
}

fn process_struct(mut item_struct: syn::ItemStruct, args: &ModelSchemaArgs) -> TokenStream {
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    if is_branded_newtype(&item_struct) {
        return process_branded_newtype(item_struct, args);
    }

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let name = item_struct.ident.clone();
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let rust_ident = name.to_string();

    #[cfg(feature = "serde")]
    let rename_all = match struct_rename_all(&item_struct) {
        Ok(rename_all) => rename_all,
        Err(output) => return output,
    };
    #[cfg(not(feature = "serde"))]
    let rename_all: Option<String> = None;

    // A `default` on the container answers a missing key for every field under it, which is one of
    // the things that makes a dropped key readable.
    let container_defaulted = has_serde_default(&item_struct.attrs);

    // A tuple struct's slots have no keys, so the named-field emitters below would render every
    // one of them under the empty ident it carries.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    if is_tuple_struct(&item_struct) {
        return process_tuple_struct(item_struct, rename_all.as_deref(), args);
    }

    // Compute schema-module identifiers and register the struct in the alias registry.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let (item_name, module_name, module_ident) =
        struct_module_idents(&name, args.name_override.as_deref(), Surface::object());

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let docs_and_example = struct_docs_and_example(&item_struct);

    // `Some(..)` selects schema-module-aware field processing; `None` (no schema output feature)
    // skips it so generated code never references a module that won't be emitted.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let module_name_opt = Some(module_name.as_str());
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let module_name_opt: Option<&str> = None;

    // Read before the walk, which takes each member's own attribute off it.
    let item_checks = checks::ItemChecks::of(&args.check, item_struct.fields.iter());

    // `collected`: (field_defs, flattened_fields, validation_fns, validate_bodies, guard_errors).
    let collected = struct_members(
        &mut item_struct,
        rename_all.as_deref(),
        module_name_opt,
        container_defaulted,
    );

    // A violated field guard makes the whole contract unsound, so the schema surface is dropped
    // and only the original item plus the errors are emitted.
    if let Some(output) = guard_failure_output(&item_struct, Some(&item_struct.ident), &collected.4)
    {
        return output;
    }

    // A unit struct holds no member, so it publishes `validate()` for its own checks alone.
    #[cfg(feature = "serde")]
    let unit_struct_impls = unit_struct_impls_for(&mut item_struct, !args.check.is_empty());
    #[cfg(not(feature = "serde"))]
    let unit_struct_impls = unit_struct_impls_for(&item_struct);

    // Read after the fields carry every serde attribute generation hangs on them, so the walker
    // reads each one through the function serde's derive will call for it.
    let decode_with = struct_decode_with(&item_struct, args);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let docs = struct_docs_body(docs_and_example.0.as_deref(), &item_name);

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let schema_impl_items = struct_schema_impl_items(
        collected.0,
        &collected.1,
        &item_name,
        &rust_ident,
        &item_struct.generics,
        args,
        &docs,
    );

    // schema_example must be directly on the type (not in the module) because the example code
    // uses type names that may not be accessible from the nested module.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let schema_example_method =
        item_schema_example_method(&docs_and_example.1, &name, &item_struct.generics, args);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let validate_method = struct_validate_method(&collected.3, &module_ident, &item_checks);

    // Build delegating impl items (schema_example is added directly, not as a delegate).
    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let delegate_impl_items = build_struct_delegate_items(
        &module_ident,
        &item_name,
        &rust_ident,
        &type_parameters_in_scope(&item_struct.generics),
        schema_example_method.as_ref(),
    );

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    {
        struct_output_with_unit_impls(
            &SchemaOutputParts {
                delegate_impl_items: &delegate_impl_items,
                generics: &item_struct.generics,
                item: &item_struct,
                module_ident: &module_ident,
                name: &name,
                schema_impl_items: &schema_impl_items,
                validate_method: &validate_method,
                validation_fns: &collected.2,
            },
            &unit_struct_impls,
            &decode_with,
        )
    }

    #[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
    {
        struct_output_with_unit_impls(
            &item_struct,
            &unit_struct_impls,
            &decode_with,
            &collected.3,
            &item_checks,
        )
    }
}

/// The assembled schema module, with a unit struct's own serde impls and the `impl` `decode_with`
/// adds appended after it.
#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
fn struct_output_with_unit_impls(
    parts: &SchemaOutputParts<syn::ItemStruct>,
    unit_struct_impls: &proc_macro2::TokenStream,
    decode_with: &DecodeWithParts,
) -> TokenStream {
    let mut read = parts.item.clone();
    let judged = judged_read::of_struct(&mut read, parts.validate_method.is_some());
    // The defaults are read off the attributes serde's derive reads, which the walk left on.
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let serde_defaults = SerdeDefaults::of(parts.item);
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let module_items: Vec<proc_macro2::TokenStream> = parts
        .validation_fns
        .iter()
        .cloned()
        .chain([serde_defaults.holder()])
        .collect();
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let values = serde_defaults.values_impl(parts.module_ident);
    #[cfg(not(any(feature = "zod", feature = "jsonschema")))]
    let (module_items, values) = (
        parts.validation_fns.to_vec(),
        proc_macro2::TokenStream::new(),
    );
    let base = assemble_schema_output(
        &SchemaOutputParts {
            validation_fns: &module_items,
            ..parts.of(&read)
        },
        &decode_with.schema_module,
    );
    let decode_with_impl = &decode_with.type_impl;
    quote! {
        #base
        #unit_struct_impls
        #judged
        #decode_with_impl
        #values
    }
}

/// [`struct_output_with_unit_impls`] where no schema surface is on: the item alone, with a unit
/// struct's serde impls, the `validate()` its checks earn it and what `decode_with` adds appended.
#[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
fn struct_output_with_unit_impls(
    item_struct: &syn::ItemStruct,
    unit_struct_impls: &proc_macro2::TokenStream,
    decode_with: &DecodeWithParts,
    validate_bodies: &[proc_macro2::TokenStream],
    item_checks: &checks::ItemChecks<'_>,
) -> TokenStream {
    let validate_impl = checks_validate_impl(
        &item_struct.ident,
        &item_struct.generics,
        validate_bodies,
        item_checks,
    );
    let mut read = item_struct.clone();
    let judged = judged_read::of_struct(&mut read, !validate_impl.is_empty());
    let decode_with_module = &decode_with.schema_module;
    let decode_with_impl = &decode_with.type_impl;
    let output = quote! {
        #read
        #unit_struct_impls
        #validate_impl
        #judged
        #decode_with_module
        #decode_with_impl
    };
    log::trace!("{output}");
    output
}

/// Records what the item declares for the recovering decodes expanded after it, which walk a
/// field typed with a model type differently from one typed with an alias, and whose typed paths
/// reach below a model serde writes under keys and stop at one it writes as one value. An alias
/// that takes a parameter is left out: what it names depends on what fills it.
#[cfg(feature = "serde")]
fn record_declaration(item: &Item) {
    // An item's walker reads the validators and the checks of its own fields, recorded as they
    // are expanded.
    forget_field_validators();
    forget_checks();
    if let Item::Type(alias) = item {
        if alias.generics.params.is_empty() {
            record_declared(
                &alias.ident.to_string(),
                Declared::Alias(quote::ToTokens::to_token_stream(&alias.ty).to_string()),
            );
        }
    } else if let Some(ident) = item_schema_ident(item) {
        let declares = if written_as_one_value(item) {
            Declared::OneValue
        } else {
            Declared::Model
        };
        record_declared(&ident.to_string(), declares);
    } else {
        // `#[model_schema]` is written on nothing else.
    }
}

/// Without `serde` nothing reads a value, so nothing reads what an item declares.
#[cfg(all(feature = "zod", not(feature = "serde")))]
const fn record_declaration(_item: &Item) {}

/// What `decode_with` adds to a struct, read off the struct as it is emitted. Empty without the
/// flag.
#[cfg(feature = "serde")]
fn struct_decode_with(item_struct: &syn::ItemStruct, args: &ModelSchemaArgs) -> DecodeWithParts {
    if !args.enable.decode_with {
        return DecodeWithParts::default();
    }
    let added = struct_recovering_decode(item_struct, args.enable.mongodb);
    let (walkers, checks, probes) = (added.type_impl, added.check_messages, added.probes);
    DecodeWithParts {
        schema_module: added.schema_module,
        type_impl: quote! { #walkers #checks #probes },
    }
}

/// Nothing, where no `serde` feature reads the attributes a walker is written from.
#[cfg(not(feature = "serde"))]
fn struct_decode_with(_item_struct: &syn::ItemStruct, _args: &ModelSchemaArgs) -> DecodeWithParts {
    DecodeWithParts::default()
}

/// What `decode_with` adds to an enum, read off the enum as it is emitted. Empty without the flag.
#[cfg(feature = "serde")]
fn enum_decode_with(item_enum: &syn::ItemEnum, args: &ModelSchemaArgs) -> DecodeWithParts {
    if !args.enable.decode_with {
        return DecodeWithParts::default();
    }
    let added = enum_recovering_decode(item_enum, args.enable.mongodb);
    let (walkers, checks, probes) = (added.type_impl, added.check_messages, added.probes);
    DecodeWithParts {
        schema_module: added.schema_module,
        type_impl: quote! { #walkers #checks #probes },
    }
}

/// Nothing, where no `serde` feature reads the attributes a walker is written from.
#[cfg(not(feature = "serde"))]
fn enum_decode_with(_item_enum: &syn::ItemEnum, _args: &ModelSchemaArgs) -> DecodeWithParts {
    DecodeWithParts::default()
}

/// An enum's assembled schema module, with the `impl` `decode_with` adds appended after it.
#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
fn enum_output_with_decode(
    parts: &SchemaOutputParts<syn::ItemEnum>,
    decode_with: &DecodeWithParts,
) -> TokenStream {
    let mut read = parts.item.clone();
    let judged = judged_read::of_enum(&mut read, parts.validate_method.is_some());
    // The defaults are read off the attributes serde's derive reads, which the walk left on.
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let serde_defaults = SerdeDefaults::of_enum(parts.item);
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let module_items: Vec<proc_macro2::TokenStream> = parts
        .validation_fns
        .iter()
        .cloned()
        .chain([serde_defaults.holder()])
        .collect();
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let values = serde_defaults.values_impl(parts.module_ident);
    #[cfg(not(any(feature = "zod", feature = "jsonschema")))]
    let (module_items, values) = (
        parts.validation_fns.to_vec(),
        proc_macro2::TokenStream::new(),
    );
    let base = assemble_schema_output(
        &SchemaOutputParts {
            validation_fns: &module_items,
            ..parts.of(&read)
        },
        &decode_with.schema_module,
    );
    let decode_with_impl = &decode_with.type_impl;
    quote! {
        #base
        #judged
        #decode_with_impl
        #values
    }
}

/// [`enum_output_with_decode`] where no schema surface is on: the item alone, with the
/// `validate()` its checks earn it and what `decode_with` adds appended. `arms` holds what each
/// variant's members add to that `validate()`.
#[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
fn enum_output_with_decode(
    item_enum: &syn::ItemEnum,
    decode_with: &DecodeWithParts,
    arms: &[proc_macro2::TokenStream],
    item_checks: &checks::ItemChecks<'_>,
) -> TokenStream {
    let validate_impl = checks_validate_impl(
        &item_enum.ident,
        &item_enum.generics,
        &matched_arms(arms),
        item_checks,
    );
    let mut read = item_enum.clone();
    let judged = judged_read::of_enum(&mut read, !validate_impl.is_empty());
    let decode_with_module = &decode_with.schema_module;
    let decode_with_impl = &decode_with.type_impl;
    let output = quote! {
        #read
        #validate_impl
        #judged
        #decode_with_module
        #decode_with_impl
    };
    log::trace!("{output}");
    output
}

/// Records a unit struct in the registry `is_unit_type` reads, then rewrites its derive.
#[cfg(feature = "serde")]
fn unit_struct_impls_for(
    item_struct: &mut syn::ItemStruct,
    publishes_validate: bool,
) -> proc_macro2::TokenStream {
    if matches!(item_struct.fields, syn::Fields::Unit) {
        record_unit_struct(&item_struct.ident.to_string());
        return unit_struct_serde_impls(item_struct, publishes_validate);
    }
    quote! {}
}

/// [`unit_struct_impls_for`] without the `serde` feature: nothing rewrites a derive that cannot be
/// there, and nothing reads the registry it would have recorded into.
#[cfg(not(feature = "serde"))]
fn unit_struct_impls_for(_item_struct: &syn::ItemStruct) -> proc_macro2::TokenStream {
    quote! {}
}

/// Strips `Serialize`/`Deserialize` from a unit struct's own `#[derive(...)]` and answers with
/// impls that write and read `{}` in their place — only the impl whose derive was present. Where
/// the struct publishes `validate()`, its read is `model_schema_read_unjudged` and then
/// `validate()`, as that of every other type that publishes one. Where it names a `preprocess`,
/// `model_schema_read_unjudged` calls each function on the payload first.
#[cfg(feature = "serde")]
fn unit_struct_serde_impls(
    item_struct: &mut syn::ItemStruct,
    publishes_validate: bool,
) -> proc_macro2::TokenStream {
    let name = &item_struct.ident;
    let type_name = name.to_string();
    let mut writes = false;
    let mut reads = false;

    let mut kept_attrs = Vec::with_capacity(item_struct.attrs.len());
    for attr in item_struct.attrs.drain(..) {
        if !attr.path().is_ident("derive") {
            kept_attrs.push(attr);
            continue;
        }
        let Ok(paths) = attr.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
        else {
            kept_attrs.push(attr);
            continue;
        };
        let kept_paths: Vec<syn::Path> = paths
            .into_iter()
            .filter(|path| match path.segments.last() {
                Some(segment) if segment.ident == "Serialize" => {
                    writes = true;
                    false
                }
                Some(segment) if segment.ident == "Deserialize" => {
                    reads = true;
                    false
                }
                _ => true,
            })
            .collect();
        if !kept_paths.is_empty() {
            kept_attrs.push(syn::parse_quote! { #[derive(#(#kept_paths),*)] });
        }
    }
    item_struct.attrs = kept_attrs;

    let serialize_impl = writes.then(|| {
        quote! {
            impl ::serde::Serialize for #name {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error> {
                    use ::serde::ser::SerializeStruct as _;
                    serializer.serialize_struct(#type_name, 0)?.end()
                }
            }
        }
    });

    let deserialize_impl = reads.then(|| unit_struct_read(name, publishes_validate));

    quote! {
        #serialize_impl
        #deserialize_impl
    }
}

/// The `Deserialize` of the unit struct `name`, which reads `{}`: alone, or after
/// `model_schema_read_unjudged` where the struct publishes `validate()` or names a `preprocess`.
#[cfg(feature = "serde")]
fn unit_struct_read(name: &syn::Ident, publishes_validate: bool) -> proc_macro2::TokenStream {
    let type_name = name.to_string();
    let expecting = format!("an empty object for `{type_name}`");
    let read = quote! {
        struct UnitStructVisitor;
        impl<'de> ::serde::de::Visitor<'de> for UnitStructVisitor {
            type Value = #name;
            fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                formatter.write_str(#expecting)
            }
            fn visit_map<A: ::serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> ::core::result::Result<Self::Value, A::Error> {
                while map.next_entry::<::serde::de::IgnoredAny, ::serde::de::IgnoredAny>()?.is_some() {}
                ::core::result::Result::Ok(#name)
            }
        }
        deserializer.deserialize_struct(#type_name, &[], UnitStructVisitor)
    };
    let corrected = judged_read::unit_struct_correction(name);
    if !publishes_validate && corrected.is_none() {
        return quote! {
            impl<'de> ::serde::Deserialize<'de> for #name {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::core::result::Result<Self, D::Error> {
                    #read
                }
            }
        };
    }
    let unjudged = corrected.map_or_else(
        || {
            quote! {
                /// The read alone, for a holder that judges the whole value once.
                pub fn model_schema_read_unjudged<'de, D: ::serde::Deserializer<'de>>(deserializer: D) -> ::core::result::Result<Self, D::Error> {
                    #read
                }
            }
        },
        |(correction, of_corrected)| {
            quote! {
                #correction

                /// The read of a payload the functions `preprocess` names on the type have
                /// corrected.
                pub fn #of_corrected<'de, D: ::serde::Deserializer<'de>>(deserializer: D) -> ::core::result::Result<Self, D::Error> {
                    #read
                }
            }
        },
    );
    let judged = if publishes_validate {
        let refusal = refusal_from_violations();
        quote! {
            let read = Self::model_schema_read_unjudged(deserializer)?;
            read.validate().map_err(#refusal)?;
            ::core::result::Result::Ok(read)
        }
    } else {
        quote! { Self::model_schema_read_unjudged(deserializer) }
    };
    quote! {
        impl #name {
            #unjudged
        }

        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::core::result::Result<Self, D::Error> {
                #judged
            }
        }
    }
}

/// Returns whether a struct's slots are positional. A branded newtype is one too, and is
/// dispatched ahead of this question.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const fn is_tuple_struct(item_struct: &syn::ItemStruct) -> bool {
    matches!(item_struct.fields, syn::Fields::Unnamed(_))
}

/// Whether the slot at `index` of a tuple struct declaring `declared_slots` slots carries serde
/// attributes the wire answers for.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const fn slot_attributes_reach_the_wire(declared_slots: usize) -> bool {
    declared_slots > 1
}

/// Rejects a tuple-struct slot whose serde attributes drop it out of one of serde's two directions
/// and not the other.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn check_slot_wire_is_readable(
    field: &Field,
    index: usize,
    declared_slots: usize,
    type_name: &str,
    omission: SerdeKeyOmission,
) -> Result<(), syn::Error> {
    if !slot_attributes_reach_the_wire(declared_slots) || !omission.drops_one_direction_only() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: slot {index} of tuple struct `{type_name}` carries a serde attribute \
             that drops it from only one of serde's two directions, so the array serde writes is \
             not an array serde reads — one of them carries the slot and the other does not. A \
             slot is written by its place rather than under a key, so there is no optional \
             spelling to describe both and no schema can be written for the pair. Use \
             #[serde(skip)] to take the slot off the wire in both directions, or drop the \
             attribute so the slot is written and read in its place."
        ),
    ))
}

/// Walks a tuple struct's slots.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn collect_tuple_slots(
    fields: &mut syn::Fields,
    rename_all: Option<&str>,
    module_name: &str,
    type_name: &str,
    generics: &syn::Generics,
    container_defaulted: bool,
    container_read_back: bool,
) -> TupleSlots {
    let declared_slots = fields.len();
    let type_parameters = type_parameters_in_scope(generics);
    let mut slots: Vec<FieldDef> = Vec::new();
    let mut validation_fns: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut validate_bodies: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut guard_errors: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut deferred_attrs: Vec<Vec<syn::Attribute>> = Vec::new();
    let steps = steps_of_members(fields, &[], rename_all);
    for (index, field) in fields.iter_mut().enumerate() {
        let omission = parse_serde_key_omission(&field.attrs);
        if let Err(rejection) =
            check_slot_wire_is_readable(field, index, declared_slots, type_name, omission)
        {
            guard_errors.push(rejection.to_compile_error());
        }
        let (slot, validation_fn, validate_body, slot_guard_errors) = process_field(
            &FieldContext {
                container_defaulted,
                container_read_back,
                enum_args: None,
                member_steps: &steps,
                on_the_wire: true,
                rename_all,
                schema_module_name: Some(module_name),
                type_name,
                type_parameters: &type_parameters,
                variant_ident: None,
            },
            field,
            index,
            &mut deferred_attrs,
        );
        guard_errors.extend(slot_guard_errors);
        validation_fns.extend(validation_fn);
        validate_bodies.extend(validate_body);
        if slot_attributes_reach_the_wire(declared_slots) && omission.absent_from_wire() {
            continue;
        }
        slots.push(slot);
    }
    if guard_errors.is_empty() {
        apply_deferred_field_attrs(fields.iter_mut(), deferred_attrs);
    }
    TupleSlots {
        guard_errors,
        shape: tuple_struct_shape(declared_slots, slots),
        validate_bodies,
        validation_fns,
    }
}

/// The shape a tuple struct's declared arity and described slots amount to. Keyed on the
/// *declared* arity, not the described count: a struct declaring two slots with one off the wire
/// still writes a one-element array, not the bare value.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn tuple_struct_shape(declared_slots: usize, mut slots: Vec<FieldDef>) -> TupleStructShape {
    if declared_slots == 1 && slots.len() == 1 {
        return TupleStructShape::BareValue(Box::new(slots.remove(0)));
    }
    TupleStructShape::Array(slots)
}

/// The TypeScript type a tuple struct describes as: the bare value is the slot's own type (serde
/// writes a newtype struct as that value alone), and every array — the empty one included — is the
/// fixed tuple the slots describe as.
#[cfg(feature = "typescript")]
fn tuple_struct_ts_body(shape: &TupleStructShape) -> String {
    match shape {
        TupleStructShape::Array(slots) => format!(
            "[{}]",
            slots
                .iter()
                .map(FieldDef::typescript_slot_typename)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TupleStructShape::BareValue(slot) => slot.typescript_slot_typename(),
    }
}

/// [`tuple_struct_ts_body`] for the Zod surface.
#[cfg(feature = "zod")]
fn tuple_struct_zod_body(shape: &TupleStructShape, owner: &SlotOwner<'_>) -> String {
    match shape {
        TupleStructShape::Array(slots) => format!(
            "z.tuple([{}])",
            slots
                .iter()
                .map(|slot| tuple_slot_zod(slot, owner))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TupleStructShape::BareValue(slot) => tuple_slot_zod(slot, owner),
    }
}

/// One slot as a tuple struct writes it. A slot that reaches the struct itself, or a type declared
/// below it, is written behind `z.lazy`: `z.tuple` takes an array, which has no getter to defer a
/// member behind.
#[cfg(feature = "zod")]
fn tuple_slot_zod(slot: &FieldDef, owner: &SlotOwner<'_>) -> String {
    let written = slot.zod_slot_type();
    let reaches_itself = slot.contains_type_reference(owner.item_name)
        || slot.contains_type_reference(owner.rust_ident);
    #[cfg(feature = "typescript")]
    if reaches_itself && !owner.parameters.is_empty() {
        let own_call = zod_own_factory_call(owner.item_name, owner.parameters);
        if written.contains(&own_call) {
            return written.replace(
                &own_call,
                &deferred_zod_operand(&zod_self_view_call(owner.item_name, owner.parameters)),
            );
        }
    }
    if reaches_itself || slot.reaches_a_type_declared_later() {
        deferred_zod_operand(&written)
    } else {
        written
    }
}

/// The names a factory's arguments are bound under, as a call hands them on.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_factory_argument_names(parameters: &[String]) -> String {
    parameters
        .iter()
        .map(|parameter| zod_factory_argument(parameter))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The call a generic item's builder reaches the item itself through, where reading its own
/// factory's type would be reading its own return type.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_self_view_call(item_name: &str, parameters: &[String]) -> String {
    format!(
        "{item_name}$SchemaSelf({})",
        zod_factory_argument_names(parameters)
    )
}

/// The call a generic item's builder makes to its own factory, with the arguments it was handed.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_own_factory_call(item_name: &str, parameters: &[String]) -> String {
    format!(
        "{item_name}$SchemaFactory({})",
        zod_factory_argument_names(parameters)
    )
}

/// The function [`zod_self_view_call`] names, for a builder that calls it. The overload states
/// what the factory's schema parses, and the implementation beneath it hands that schema over,
/// as the factory's own overload does for its cache.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_self_view(item_name: &str, parameters: &[String], builder: &str) -> String {
    if !builder.contains(&zod_self_view_call(item_name, parameters)) {
        return String::new();
    }
    let outputs = parameters
        .iter()
        .map(|parameter| format!("{parameter}[\"_zod\"][\"output\"]"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "function {item_name}$SchemaSelf{}({}\n): ZodType<{item_name}<{outputs}>>;\n\
         function {item_name}$SchemaSelf({}\n): ZodType {{\n  return \
         {item_name}$SchemaFactory({});\n}}\n\n",
        zod_factory_bounds(parameters),
        zod_factory_arguments(parameters),
        zod_factory_widened_arguments(parameters),
        zod_factory_argument_names(parameters)
    )
}

/// The end of the call whose opening parenthesis is at `open`, one past its closing one. Text
/// between double quotes is passed over.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn call_end(zod: &str, open: usize) -> usize {
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut escaped = false;
    for (at, written) in zod.char_indices().skip_while(|(at, _)| *at < open) {
        if quoted {
            if escaped {
                escaped = false;
            } else if written == '\\' {
                escaped = true;
            } else if written == '"' {
                quoted = false;
            } else {
                // Any other character of the text.
            }
        } else if written == '"' {
            quoted = true;
        } else if written == '(' {
            depth = depth.saturating_add(1);
        } else if written == ')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return at.saturating_add(1);
            }
        } else {
            // Nothing a call's extent is read from.
        }
    }
    zod.len()
}

/// Every map `zod` writes, as the range of its record call.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn record_calls(zod: &str) -> Vec<Range<usize>> {
    ["z.record(", "z.partialRecord("]
        .iter()
        .flat_map(|opening| {
            zod.match_indices(opening).map(|(at, written)| {
                at..call_end(zod, at.saturating_add(written.len()).saturating_sub(1))
            })
        })
        .collect()
}

/// `zod` with each `call` written inside a map replaced by `view`. A call that ends a longer
/// name (`TreeNode$SchemaFactory` for `Node$SchemaFactory`) is another item's.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn inside_maps(zod: &str, call: &str, view: &str) -> String {
    let maps = record_calls(zod);
    let mut written = String::new();
    let mut copied = 0_usize;
    for (at, _) in zod.match_indices(call) {
        let ends_a_name = zod[..at]
            .chars()
            .next_back()
            .is_some_and(|last| last.is_alphanumeric() || last == '_' || last == '$');
        if ends_a_name || !maps.iter().any(|map| map.contains(&at)) {
            continue;
        }
        written.push_str(&zod[copied..at]);
        written.push_str(view);
        copied = at.saturating_add(call.len());
    }
    written.push_str(&zod[copied..]);
    written
}

/// The arguments `handed` lists, split where a comma stands outside every call.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn handed_arguments(handed: &str) -> Vec<&str> {
    let mut arguments = Vec::new();
    let mut depth = 0_usize;
    let mut from = 0_usize;
    for (at, written) in handed.char_indices() {
        if written == '(' {
            depth = depth.saturating_add(1);
        } else if written == ')' {
            depth = depth.saturating_sub(1);
        } else if written == ',' && depth == 0 {
            arguments.push(handed[from..at].trim());
            from = at.saturating_add(1);
        } else {
            // Part of the argument being read.
        }
    }
    arguments.push(handed[from..].trim());
    arguments
}

/// `zod` with each union's arrow handed `view` in place of `call`, where the arrow writes that
/// argument inside a map. The arrow is the one `zod_union_around` writes in `field_type.rs`.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn arrows_over_maps(zod: &str, call: &str, view: &str) -> String {
    let mut written = String::new();
    let mut copied = 0_usize;
    for (at, _) in zod.match_indices(&format!("(<Reached$ extends {ZOD_SCHEMA_BOUND}")) {
        if at < copied {
            continue;
        }
        let arrow_end = call_end(zod, at);
        if !zod[arrow_end..].starts_with('(') {
            continue;
        }
        let handed_end = call_end(zod, arrow_end);
        let handed = &zod[arrow_end.saturating_add(1)..handed_end.saturating_sub(1)];
        let arrow = &zod[at..arrow_end];
        let maps = record_calls(arrow);
        let arguments: Vec<String> = handed_arguments(handed)
            .into_iter()
            .zip(1_usize..)
            .map(|(argument, position)| {
                let name = if position == 1 {
                    "reached$".to_owned()
                } else {
                    format!("reached{position}$")
                };
                let in_a_map = arrow
                    .match_indices(&name)
                    .any(|(named_at, _)| maps.iter().any(|map| map.contains(&named_at)));
                if argument == call && in_a_map {
                    view.to_owned()
                } else {
                    argument.to_owned()
                }
            })
            .collect();
        written.push_str(&zod[copied..=arrow_end]);
        written.push_str(&arguments.join(", "));
        copied = handed_end.saturating_sub(1);
    }
    written.push_str(&zod[copied..]);
    written
}

/// `zod`, a generic item's builder, with each reference the item makes to itself inside a map
/// made through its self view. A map's own type is read off what it holds as the builder's
/// return type is still being inferred, which `tsc --strict` refuses (TS2345) or resolves to
/// `unknown` (TS2322); the view states the type instead.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn maps_of_itself_through_the_self_view(
    item_name: &str,
    parameters: &[String],
    zod: &str,
) -> String {
    let call = zod_own_factory_call(item_name, parameters);
    let view = zod_self_view_call(item_name, parameters);
    inside_maps(&arrows_over_maps(zod, &call, &view), &call, &view)
}

/// [`tuple_struct_ts_body`] for the JSON-schema surface, as a standalone `serde_json::Value`
/// expression, or the diagnostic naming the type when a slot holds a value the dispatch cannot
/// render.
#[cfg(feature = "jsonschema")]
fn tuple_struct_json_body(item_name: &str, shape: &TupleStructShape) -> proc_macro2::TokenStream {
    let described = match shape {
        TupleStructShape::Array(slots) => bounded_slots_json_schema(slots),
        TupleStructShape::BareValue(slot) => bounded_slot_json_schema(slot),
    };
    match described {
        Ok(value) => value,
        Err(rejection) => {
            let message = prefixed_guard_message(&map_member_rejection_message(
                &format!("`{item_name}`"),
                &rejection,
            ));
            syn::Error::new(rejection.span(), message).to_compile_error()
        }
    }
}

/// Builds the `ts_definition()` method for a tuple struct's schema module.
#[cfg(feature = "typescript")]
fn build_tuple_struct_ts_definition_method(
    docs: &str,
    item_name: &str,
    rust_ident: &str,
    ts_generics: &str,
    ts_body: &str,
) -> proc_macro2::TokenStream {
    let reexport = ident_reexport_ts(rust_ident, item_name, ts_generics);
    let type_str = format!(
        "{}\nexport type {item_name}{ts_generics} = {ts_body};{reexport}",
        jsdoc_block(docs, "")
    );
    quote! {
        pub fn ts_definition() -> ::std::string::String {
            #type_str.to_owned()
        }
    }
}

/// Builds the `zod_schema()` method for a tuple struct's schema module, in the same framing every
/// unbranded type publishes: the annotated binding, or the factory when the tuple struct declares
/// parameters.
#[cfg(feature = "zod")]
fn build_tuple_struct_zod_schema_method(
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    published: &PublishedBinding<'_>,
    zod_body: &str,
) -> proc_macro2::TokenStream {
    let reexport = zod_binding_reexport(rust_ident, item_name, parameters);
    let schema_str = zod_published_binding(
        item_name, rust_ident, parameters, published, "", zod_body, &reexport,
    );
    quote! {
        pub fn zod_schema() -> ::std::string::String {
            #schema_str.to_owned()
        }
    }
}

/// What a tuple struct publishes: serde writes one slot as that slot's value alone, so the schema
/// *is* the slot's schema and carries exactly what it carries; every other arity is the fixed array
/// `z.tuple` writes, which takes no string check and which no object can be merged with.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn tuple_struct_surface(fields: &syn::Fields, parameters: &[String]) -> Surface {
    let mut slots = fields.iter();
    match (slots.next(), slots.next()) {
        (Some(slot), None) => Surface::written(&get_field_def("_slot", &slot.ty, ""), parameters),
        _ => Surface::array(),
    }
}

/// Processes a tuple struct that is not a branded newtype.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn process_tuple_struct(
    mut item_struct: syn::ItemStruct,
    rename_all: Option<&str>,
    args: &ModelSchemaArgs,
) -> TokenStream {
    let name = item_struct.ident.clone();
    let (item_name, module_name, module_ident) = struct_module_idents(
        &name,
        args.name_override.as_deref(),
        tuple_struct_surface(
            &item_struct.fields,
            &type_parameters_in_scope(&item_struct.generics),
        ),
    );

    // Read before the walk, which takes each slot's own attribute off it.
    let item_checks = checks::ItemChecks::of(&args.check, item_struct.fields.iter());

    let slots = collect_tuple_slots(
        &mut item_struct.fields,
        rename_all,
        &module_name,
        &name.to_string(),
        &item_struct.generics,
        has_serde_default(&item_struct.attrs),
        container_is_read_back(&item_struct.attrs),
    );

    // A violated slot guard makes the whole contract unsound, so the schema surface is dropped and
    // only the original item plus the errors are emitted.
    if let Some(output) =
        guard_failure_output(&item_struct, Some(&item_struct.ident), &slots.guard_errors)
    {
        return output;
    }

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let docs_and_example = struct_docs_and_example(&item_struct);

    let schema_impl_items: Vec<proc_macro2::TokenStream> = vec![
        #[cfg(feature = "jsonschema")]
        json_schema_methods(
            &item_name,
            &tuple_struct_json_body(&item_name, &slots.shape),
            &schema_parameters(&item_struct.generics, args),
        ),
        #[cfg(feature = "jsonschema")]
        unit_member_record_item(&Item::Struct(item_struct.clone())),
        #[cfg(feature = "typescript")]
        build_tuple_struct_ts_definition_method(
            &build_jsdoc_body(docs_and_example.0.as_deref(), &item_name),
            &item_name,
            &name.to_string(),
            &ts_generic_params(&item_struct.generics),
            &tuple_struct_ts_body(&slots.shape),
        ),
        #[cfg(feature = "zod")]
        build_tuple_struct_zod_schema_method(
            &item_name,
            &name.to_string(),
            &type_parameters_in_scope(&item_struct.generics),
            &PublishedBinding {
                checks: &args.check,
                default_types: &args.default_types,
                preprocess: &args.preprocess,
                republished: tuple_struct_republishes_slot(&slots.shape),
            },
            &tuple_struct_zod_body(
                &slots.shape,
                &SlotOwner {
                    item_name: &item_name,
                    #[cfg(feature = "typescript")]
                    parameters: &type_parameters_in_scope(&item_struct.generics),
                    rust_ident: &name.to_string(),
                },
            ),
        ),
    ];

    // schema_example must be directly on the type (not in the module) because the example code
    // uses type names that may not be accessible from the nested module.
    let schema_example_method =
        item_schema_example_method(&docs_and_example.1, &name, &item_struct.generics, args);

    let delegate_impl_items = build_struct_delegate_items(
        &module_ident,
        &item_name,
        &name.to_string(),
        &type_parameters_in_scope(&item_struct.generics),
        schema_example_method.as_ref(),
    );

    // Read after every slot carries the serde attributes generation hangs on it.
    let decode_with = struct_decode_with(&item_struct, args);

    struct_output_with_unit_impls(
        &SchemaOutputParts {
            delegate_impl_items: &delegate_impl_items,
            generics: &item_struct.generics,
            item: &item_struct,
            module_ident: &module_ident,
            name: &name,
            schema_impl_items: &schema_impl_items,
            validate_method: &struct_validate_method(
                &slots.validate_bodies,
                &module_ident,
                &item_checks,
            ),
            validation_fns: &slots.validation_fns,
        },
        &TokenStream::new(),
        &decode_with,
    )
}

/// Builds the `validate_value` function for a constrained branded newtype, or `None` when it has
/// no string constraints. A path inner is measured by the string serde writes for it; every other
/// inner is reached through `ToString`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn build_branded_validation(
    args: &ModelSchemaArgs,
    generic_params: &[String],
    inner_field: &Field,
    held: &syn::Member,
) -> Option<BrandedValidation> {
    let inner_ty = &inner_field.ty;
    let is_generic = !generic_params.is_empty();
    if held_as_a_list(inner_ty) {
        return branded_list_validation(args, inner_field, held);
    }
    args.has_string_constraints().then(|| {
        let measures_path = branded_inner_measures_path(inner_ty);
        let (checked_param, rendering) = checked_value_parts(measures_path);
        // `resolved_at` keeps the inner field's location while giving the token the macro's own
        // hygiene, so a consumer's lints judge it as generated.
        let to_string_span = inner_ty.span().resolved_at(proc_macro2::Span::call_site());
        // A brand is the value rather than a member of anything, so its report names no field:
        // whatever holds it writes the name, on both languages' side of the wire.
        let measured = quote! { value.len() };
        let mut checks: Vec<proc_macro2::TokenStream> = Vec::new();

        if let Some(min_len) = args.min_length {
            let reported = rust_violation(Bound::MinLength(min_len), None, &measured);
            checks.push(quote! {
                if value.len() < #min_len {
                    errors.push(#reported);
                }
            });
        }
        if let Some(max_len) = args.max_length {
            let reported = rust_violation(Bound::MaxLength(max_len), None, &measured);
            checks.push(quote! {
                if value.len() > #max_len {
                    errors.push(#reported);
                }
            });
        }
        if let Some(pattern) = &args.pattern {
            let reported = rust_violation(Bound::Pattern(pattern), None, &measured);
            checks.push(pattern_check(pattern, &quote! { errors.push(#reported); }));
        }

        let validate_fn = quote! {
            pub fn validate_value(#checked_param) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
                #rendering
                let mut errors: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
                #(#checks)*
                if errors.is_empty() { ::core::result::Result::Ok(()) } else { ::core::result::Result::Err(errors) }
            }
        };

        // A generic brand's checks are its declared default's, so its `validate()` hands the value
        // over as it is and asks nothing of the parameter.
        if is_generic {
            let default_ty =
                substitute_declared_defaults(inner_ty, generic_params, &args.default_types);
            let type_identity = embedded_type_identity();
            let at_the_default = embedded_validate_at_declared_default(
                &default_ty,
                &branded_checked_value(measures_path, to_string_span, &quote! { held }),
            );
            return BrandedValidation {
                checked_inner: quote! { &self.#held },
                validate_fn: quote! {
                    #validate_fn

                    #type_identity

                    #at_the_default
                },
                validator: proc_macro2::Ident::new(
                    "validate_at_declared_default",
                    proc_macro2::Span::call_site(),
                ),
            };
        }

        BrandedValidation {
            checked_inner: branded_checked_value(
                measures_path,
                to_string_span,
                &quote! { self.#held },
            ),
            validate_fn,
            validator: proc_macro2::Ident::new("validate_value", proc_macro2::Span::call_site()),
        }
    })
}

/// [`build_branded_validation`] for a brand over a list, whose two lengths count its items: the
/// validator a list field gets, under the brand's own name.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn branded_list_validation(
    args: &ModelSchemaArgs,
    inner_field: &Field,
    held: &syn::Member,
) -> Option<BrandedValidation> {
    if args.min_length.is_none() && args.max_length.is_none() {
        return None;
    }
    let wraps = constrained_shape(&inner_field.ty)?.wraps;
    let validator = proc_macro2::Ident::new("validate_value", proc_macro2::Span::call_site());
    let through = wraps.iter().map(|_| quote! { * });
    Some(BrandedValidation {
        checked_inner: quote! { &#(#through)* self.#held },
        validate_fn: list_validator(&validator, args.min_length, args.max_length, None),
        validator,
    })
}

// Adapted from `typeid::of` in the typeid crate by David Tolnay (https://github.com/dtolnay/typeid),
// used under the MIT license; see THIRD-PARTY-NOTICES.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn embedded_type_identity() -> proc_macro2::TokenStream {
    quote! {
        fn type_identity<T: ?::core::marker::Sized>() -> ::core::any::TypeId {
            trait NonStaticAny {
                fn get_type_id(&self) -> ::core::any::TypeId
                where
                    Self: 'static;
            }
            impl<T: ?::core::marker::Sized> NonStaticAny for ::core::marker::PhantomData<T> {
                fn get_type_id(&self) -> ::core::any::TypeId
                where
                    Self: 'static,
                {
                    ::core::any::TypeId::of::<T>()
                }
            }
            let phantom_data = ::core::marker::PhantomData::<T>;
            // Only the trait object's lifetime changes, and a TypeId never carries lifetimes.
            NonStaticAny::get_type_id(unsafe {
                ::core::mem::transmute::<&dyn NonStaticAny, &(dyn NonStaticAny + 'static)>(
                    &phantom_data,
                )
            })
        }
    }
}

/// `validate_at_declared_default`, beside `type_identity` in a generic brand's schema module: what
/// `validate_value` answers for `checked`, which reads `held` as `default_ty` at the lifetime of
/// the borrow, where the filling is that one, and `Ok` at any other; a declared default whose
/// `Display` is written for one lifetime alone does not build:
///
/// ```rust,compile_fail
/// use core::fmt;
/// use serde::Serialize;
/// use tixschema::model_schema;
///
/// #[model_schema(no_display)]
/// #[derive(Serialize)]
/// #[serde(transparent)]
/// pub struct Tag<'text>(pub &'text str);
///
/// impl fmt::Display for Tag<'static> {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         f.write_str(self.0)
///     }
/// }
///
/// #[model_schema(minLength = 2, default_types(Text = Tag<'static>))]
/// #[derive(Serialize)]
/// #[serde(transparent)]
/// pub struct Stamp<Text>(pub Text);
///
/// fn main() {}
/// ```
///
/// ```text
/// error[E0521]: borrowed data escapes outside of function
///   --> tests/zz_probe.rs:16:1
///    |
/// 16 |   #[model_schema(minLength = 2, default_types(Text = Tag<'static>))]
///    |   ^-----------------------------------------------------------------
///    |   |
///    |   `held` is a reference that is only valid in the function body
///    |   lifetime `'held` defined here
///    |  _in this attribute macro expansion
///    | |
/// 17 | | #[derive(Serialize)]
/// 18 | | #[serde(transparent)]
/// 19 | | pub struct Stamp<Text>(pub Text);
///    | |                               ^
///    | |                               |
///    | |_______________________________`held` escapes the function body here
///    |                                 argument requires that `'held` must outlive `'static`
/// ```
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn embedded_validate_at_declared_default(
    default_ty: &syn::Type,
    checked: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let read_as = at_one_lifetime(default_ty, &syn::parse_quote!('held));
    quote! {
        /// What the checks answer for a value, which they hold where its filling is the declared
        /// default.
        pub fn validate_at_declared_default<'held, T>(held: &'held T) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
            if type_identity::<T>() == type_identity::<#read_as>() {
                // The two are one type but for a lifetime, and each lifetime the default writes is
                // read as the borrow's own, which the value outlives: none is made longer.
                let held = unsafe { &*::core::ptr::from_ref(held).cast::<#read_as>() };
                validate_value(#checked)
            } else {
                ::core::result::Result::Ok(())
            }
        }
    }
}

/// `ty` with every lifetime it writes replaced by `lifetime`: a named one, `'static`, `'_`, and the
/// one a reference leaves out. A lifetime a `for<...>` binds stays as written, and so does one a
/// function's signature leaves out, which that signature binds.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn at_one_lifetime(ty: &syn::Type, lifetime: &syn::Lifetime) -> syn::Type {
    let mut written = ty.clone();
    type_at_one_lifetime(&mut written, lifetime, &[], false);
    written
}

/// [`at_one_lifetime`] over `ty` in place. `bound` holds what the binders around it declare, and
/// `signature` says whether it stands in a function's signature.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn type_at_one_lifetime(
    ty: &mut syn::Type,
    lifetime: &syn::Lifetime,
    bound: &[syn::Ident],
    signature: bool,
) {
    match ty {
        syn::Type::Reference(reference) => {
            if let Some(written) = &mut reference.lifetime {
                lifetime_at_one(written, lifetime, bound, signature);
            } else if !signature {
                reference.lifetime = Some(lifetime.clone());
            } else {
                // Left out in a signature, which binds it.
            }
            type_at_one_lifetime(&mut reference.elem, lifetime, bound, signature);
        }
        syn::Type::Path(named) => {
            if let Some(qualified) = &mut named.qself {
                type_at_one_lifetime(&mut qualified.ty, lifetime, bound, signature);
            }
            path_at_one_lifetime(&mut named.path, lifetime, bound, signature);
        }
        syn::Type::Array(array) => {
            type_at_one_lifetime(&mut array.elem, lifetime, bound, signature);
        }
        syn::Type::Slice(slice) => {
            type_at_one_lifetime(&mut slice.elem, lifetime, bound, signature);
        }
        syn::Type::Paren(paren) => {
            type_at_one_lifetime(&mut paren.elem, lifetime, bound, signature);
        }
        syn::Type::Group(group) => {
            type_at_one_lifetime(&mut group.elem, lifetime, bound, signature);
        }
        syn::Type::Ptr(pointer) => {
            type_at_one_lifetime(&mut pointer.elem, lifetime, bound, signature);
        }
        syn::Type::Tuple(tuple) => {
            for elem in &mut tuple.elems {
                type_at_one_lifetime(elem, lifetime, bound, signature);
            }
        }
        syn::Type::FnPtr(function) => {
            let within = bound_with(bound, function.lifetimes.as_ref());
            signature_at_one_lifetime(
                &mut function.inputs,
                &mut function.output,
                lifetime,
                &within,
            );
        }
        syn::Type::TraitObject(object) => {
            bounds_at_one_lifetime(&mut object.bounds, lifetime, bound, signature);
        }
        syn::Type::ImplTrait(opaque) => {
            bounds_at_one_lifetime(&mut opaque.bounds, lifetime, bound, signature);
        }
        syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_)
        | syn::Type::Verbatim(_)
        | _ => {}
    }
}

/// [`at_one_lifetime`] over the arguments of each segment of `path`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn path_at_one_lifetime(
    path: &mut syn::Path,
    lifetime: &syn::Lifetime,
    bound: &[syn::Ident],
    signature: bool,
) {
    for segment in &mut path.segments {
        match &mut segment.arguments {
            syn::PathArguments::AngleBracketed(written) => {
                arguments_at_one_lifetime(written, lifetime, bound, signature);
            }
            syn::PathArguments::Parenthesized(taken) => {
                signature_at_one_lifetime(&mut taken.inputs, &mut taken.output, lifetime, bound);
            }
            syn::PathArguments::None => {}
        }
    }
}

/// [`at_one_lifetime`] over each of `arguments`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn arguments_at_one_lifetime(
    arguments: &mut syn::AngleBracketedGenericArguments,
    lifetime: &syn::Lifetime,
    bound: &[syn::Ident],
    signature: bool,
) {
    for argument in &mut arguments.args {
        match argument {
            syn::GenericArgument::Lifetime(written) => {
                lifetime_at_one(written, lifetime, bound, signature);
            }
            syn::GenericArgument::Type(inner) => {
                type_at_one_lifetime(inner, lifetime, bound, signature);
            }
            syn::GenericArgument::AssocType(assigned) => {
                if let Some(own) = &mut assigned.generics {
                    arguments_at_one_lifetime(own, lifetime, bound, signature);
                }
                type_at_one_lifetime(&mut assigned.ty, lifetime, bound, signature);
            }
            syn::GenericArgument::Constraint(constrained) => {
                if let Some(own) = &mut constrained.generics {
                    arguments_at_one_lifetime(own, lifetime, bound, signature);
                }
                bounds_at_one_lifetime(&mut constrained.bounds, lifetime, bound, signature);
            }
            syn::GenericArgument::AssocConst(_) | syn::GenericArgument::Const(_) | _ => {}
        }
    }
}

/// [`at_one_lifetime`] over each of `bounds`, a trait's own `for<...>` binding what it declares.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn bounds_at_one_lifetime(
    bounds: &mut Punctuated<syn::TypeParamBound, Token![+]>,
    lifetime: &syn::Lifetime,
    bound: &[syn::Ident],
    signature: bool,
) {
    for held in bounds {
        match held {
            syn::TypeParamBound::Trait(of_trait) => {
                let within = bound_with(bound, of_trait.lifetimes.as_ref());
                path_at_one_lifetime(&mut of_trait.path, lifetime, &within, signature);
            }
            syn::TypeParamBound::Lifetime(written) => {
                lifetime_at_one(written, lifetime, bound, signature);
            }
            syn::TypeParamBound::PreciseCapture(_) | syn::TypeParamBound::Verbatim(_) | _ => {}
        }
    }
}

/// [`at_one_lifetime`] over what a function takes and answers.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn signature_at_one_lifetime(
    inputs: &mut Punctuated<syn::NamedArg, Token![,]>,
    output: &mut syn::ReturnType,
    lifetime: &syn::Lifetime,
    bound: &[syn::Ident],
) {
    for input in inputs {
        type_at_one_lifetime(&mut input.ty, lifetime, bound, true);
    }
    if let syn::ReturnType::Type(_, returned) = output {
        type_at_one_lifetime(returned, lifetime, bound, true);
    }
}

/// Writes `lifetime` over `written`, unless a `for<...>` binds it or a signature left it out.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn lifetime_at_one(
    written: &mut syn::Lifetime,
    lifetime: &syn::Lifetime,
    bound: &[syn::Ident],
    signature: bool,
) {
    let left_out = written.ident == "_";
    if (left_out && signature) || bound.contains(&written.ident) {
        return;
    }
    *written = lifetime.clone();
}

/// `bound` with the lifetimes `declared` binds.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn bound_with(bound: &[syn::Ident], declared: Option<&syn::BoundLifetimes>) -> Vec<syn::Ident> {
    let mut all = bound.to_vec();
    all.extend(
        declared
            .into_iter()
            .flat_map(|binder| &binder.lifetimes)
            .filter_map(|parameter| match parameter {
                syn::GenericParam::Lifetime(named) => Some(named.lifetime.ident.clone()),
                syn::GenericParam::Type(_) | syn::GenericParam::Const(_) => None,
            }),
    );
    all
}

/// Every one of `parameters` found in `ty`, replaced by its `default_types` filling via
/// [`declared_default_syn_type`]. Recurses through path arguments, references, arrays, slices,
/// tuples, parens and groups; any other shape comes back unchanged.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn substitute_declared_defaults(
    ty: &syn::Type,
    parameters: &[String],
    default_types: &[(syn::Ident, syn::Type)],
) -> syn::Type {
    match ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_none()
                && let Some(ident) = type_path.path.get_ident()
                && parameters.iter().any(|parameter| ident == parameter)
            {
                return declared_default_syn_type(&ident.to_string(), default_types);
            }
            let mut rewritten = type_path.clone();
            for segment in &mut rewritten.path.segments {
                if let syn::PathArguments::AngleBracketed(bracketed) = &mut segment.arguments {
                    for argument in &mut bracketed.args {
                        if let syn::GenericArgument::Type(inner) = argument {
                            *inner = substitute_declared_defaults(inner, parameters, default_types);
                        }
                    }
                }
            }
            syn::Type::Path(rewritten)
        }
        syn::Type::Reference(reference) => {
            let mut rewritten = reference.clone();
            *rewritten.elem =
                substitute_declared_defaults(&reference.elem, parameters, default_types);
            syn::Type::Reference(rewritten)
        }
        syn::Type::Array(array) => {
            let mut rewritten = array.clone();
            *rewritten.elem = substitute_declared_defaults(&array.elem, parameters, default_types);
            syn::Type::Array(rewritten)
        }
        syn::Type::Slice(slice) => {
            let mut rewritten = slice.clone();
            *rewritten.elem = substitute_declared_defaults(&slice.elem, parameters, default_types);
            syn::Type::Slice(rewritten)
        }
        syn::Type::Tuple(tuple) => {
            let mut rewritten = tuple.clone();
            for elem in &mut rewritten.elems {
                *elem = substitute_declared_defaults(elem, parameters, default_types);
            }
            syn::Type::Tuple(rewritten)
        }
        syn::Type::Paren(paren) => {
            let mut rewritten = paren.clone();
            *rewritten.elem = substitute_declared_defaults(&paren.elem, parameters, default_types);
            syn::Type::Paren(rewritten)
        }
        syn::Type::Group(group) => {
            let mut rewritten = group.clone();
            *rewritten.elem = substitute_declared_defaults(&group.elem, parameters, default_types);
            syn::Type::Group(rewritten)
        }
        syn::Type::FnPtr(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_)
        | syn::Type::Ptr(_)
        | syn::Type::TraitObject(_)
        | syn::Type::Verbatim(_)
        | _ => ty.clone(),
    }
}

/// Whether a brand's constrained checks reach its inner value as a path rather than through
/// `Display`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn branded_inner_measures_path(inner_ty: &syn::Type) -> bool {
    constrained_shape(inner_ty).is_some_and(|shape| {
        matches!(shape.leaf, ConstraintLeaf::Path)
            && shape
                .wraps
                .iter()
                .all(|wrap| matches!(wrap, ConstraintWrap::Transparent))
    })
}

/// How a constrained brand hands `receiver` to `validate_value`: a path goes borrowed since the
/// validator renders it itself; every other inner goes through `Display`'s `to_string()`, spanned
/// so a non-`Display` inner's `E0599` lands beside the field rather than on the attribute.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn branded_checked_value(
    measures_path: bool,
    to_string_span: proc_macro2::Span,
    receiver: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if measures_path {
        quote! { &#receiver }
    } else {
        quote_spanned! {to_string_span=> &#receiver.to_string() }
    }
}

/// Builds the schema a branded newtype's constraints are written into: `base_inserts` first — what
/// the inner type describes as before the brand narrows it — then one insert per constraint the
/// brand declares.
#[cfg(feature = "jsonschema")]
fn branded_schema_obj_over(
    args: &ModelSchemaArgs,
    base_inserts: &proc_macro2::TokenStream,
    base_pattern: BasePattern,
) -> proc_macro2::TokenStream {
    let mut constraint_inserts: Vec<proc_macro2::TokenStream> = Vec::new();

    if let Some(min_len) = args.min_length {
        constraint_inserts.push(quote! {
            schema_obj.insert("minLength".to_string(), serde_json::Value::Number(serde_json::Number::from(#min_len as u64)));
        });
    }
    if let Some(max_len) = args.max_length {
        constraint_inserts.push(quote! {
            schema_obj.insert("maxLength".to_string(), serde_json::Value::Number(serde_json::Number::from(#max_len as u64)));
        });
    }
    if let Some(pattern) = &args.pattern {
        constraint_inserts.push(match base_pattern {
            BasePattern::Absent => quote! {
                schema_obj.insert("pattern".to_string(), serde_json::Value::String(#pattern.to_string()));
            },
            #[cfg(feature = "mongodb")]
            BasePattern::Stated => quote! {
                schema_obj.insert("allOf".to_string(), serde_json::json!([{ "pattern": #pattern }]));
            },
        });
    }

    quote! {
        {
            let mut schema_obj = serde_json::Map::new();
            #base_inserts
            #(#constraint_inserts)*
            serde_json::Value::Object(schema_obj)
        }
    }
}

/// Builds the string schema a branded newtype's constraints are written into: `type_name` as the
/// `"type"`, then one insert per constraint the brand declares.
#[cfg(feature = "jsonschema")]
fn branded_constrained_schema_obj(
    args: &ModelSchemaArgs,
    type_name: &str,
) -> proc_macro2::TokenStream {
    branded_schema_obj_over(
        args,
        &quote! {
            schema_obj.insert("type".to_string(), serde_json::Value::String(#type_name.to_string()));
        },
        BasePattern::Absent,
    )
}

/// The string a chrono-typed inner writes, carrying the `"format"` keyword [`chrono_json_schema_format`]
/// gives that type — the one field position carries for it — and narrowed by the brand's own
/// constraints, which sit beside `type` and `format` the way they sit beside `type` alone.
#[cfg(all(feature = "jsonschema", feature = "chrono"))]
fn branded_chrono_schema(args: &ModelSchemaArgs, format: &str) -> proc_macro2::TokenStream {
    branded_schema_obj_over(
        args,
        &quote! {
            schema_obj.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            schema_obj.insert("format".to_string(), serde_json::Value::String(#format.to_string()));
        },
        BasePattern::Absent,
    )
}

/// The `$oid` member an `ObjectId` brand carries: the hex string the type always holds, narrowed
/// by the brand's own constraints. The hex is the base's own `pattern`, so the brand's is layered
/// beside it rather than written over it — see [`BasePattern`].
#[cfg(all(feature = "jsonschema", feature = "mongodb"))]
fn branded_object_id_hex_schema(args: &ModelSchemaArgs) -> proc_macro2::TokenStream {
    branded_schema_obj_over(
        args,
        &quote! {
            schema_obj.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            schema_obj.insert("pattern".to_string(), serde_json::Value::String(#OBJECT_ID_HEX_PATTERN.to_string()));
        },
        BasePattern::Stated,
    )
}

/// The description a brand carries for an inner no `"type"` keyword names: the one the slot
/// dispatch gives that type. An inner the dispatch cannot render replaces the body with the
/// single diagnostic naming the brand, as an unrenderable slot does elsewhere.
#[cfg(feature = "jsonschema")]
fn branded_slot_json_schema(
    args: &ModelSchemaArgs,
    inner: &FieldDef,
    def_name: &str,
) -> proc_macro2::TokenStream {
    match build_tuple_element_json_schema(inner) {
        // A list's count is written on its own array.
        Ok(value) if inner.is_list() => value,
        Ok(value) => branded_layered_over(args, &value),
        Err(rejection) => {
            let message = prefixed_guard_message(&map_member_rejection_message(
                &format!("`{def_name}`"),
                &rejection,
            ));
            syn::Error::new(rejection.span(), message).to_compile_error()
        }
    }
}

/// `described` narrowed by the brand's own constraints from inside an `allOf`, or `described` alone
/// when the brand declares none.
#[cfg(feature = "jsonschema")]
fn branded_layered_over(
    args: &ModelSchemaArgs,
    described: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mut narrowing: Vec<proc_macro2::TokenStream> = Vec::new();
    if let Some(min_len) = args.min_length {
        let len = min_len as u64;
        narrowing.push(quote! { "minLength": #len });
    }
    if let Some(max_len) = args.max_length {
        let len = max_len as u64;
        narrowing.push(quote! { "maxLength": #len });
    }
    if let Some(pattern) = &args.pattern {
        narrowing.push(quote! { "pattern": #pattern });
    }
    if narrowing.is_empty() {
        return described.clone();
    }
    quote! {
        serde_json::json!({ "allOf": [#described, { #(#narrowing),* }] })
    }
}

/// Builds the `json_schema()` method for a branded newtype's schema module.
#[cfg(feature = "jsonschema")]
fn build_branded_json_schema_method(
    args: &ModelSchemaArgs,
    json_inner: &BrandedJsonInner,
    def_name: &str,
    parameters: &[SchemaParameter],
) -> proc_macro2::TokenStream {
    let body = match json_inner {
        #[cfg(feature = "chrono")]
        BrandedJsonInner::Chrono(format) => branded_chrono_schema(args, format),
        BrandedJsonInner::Scalar(type_name) => branded_constrained_schema_obj(args, type_name),
        #[cfg(feature = "mongodb")]
        BrandedJsonInner::ObjectId => {
            object_id_json_schema_value(&branded_object_id_hex_schema(args))
        }
        BrandedJsonInner::Slot(inner) => branded_slot_json_schema(args, inner, def_name),
    };
    json_schema_methods(def_name, &body, parameters)
}

/// The `FieldDef` every surface renders from: the written one with each name that is one of the
/// enclosing item's own type parameters classified as the parameter it is.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn surface_field_def(generics: &syn::Generics, written: &FieldDef) -> FieldDef {
    let mut erased = written.clone();
    erased.erase_type_parameters(&type_parameters_in_scope(generics));
    erased
}

/// The one def both validating surfaces read a branded newtype's inner off, so neither can render
/// a type parameter the other has erased — see [`surface_field_def`]. A list takes the brand's
/// two lengths as the count of its items, which is where a field's own list carries them.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
fn branded_value_inner(
    generics: &syn::Generics,
    inner_ty: &syn::Type,
    args: &ModelSchemaArgs,
) -> FieldDef {
    let mut inner = surface_field_def(generics, &get_field_def("_inner", inner_ty, ""));
    if held_as_a_list(inner_ty) && (args.min_length.is_some() || args.max_length.is_some()) {
        inner.model_schema_prop_meta = Some(ModelSchemaPropMeta {
            max_items: args.max_length,
            min_items: args.min_length,
            ..ModelSchemaPropMeta::default()
        });
    }
    inner
}

/// Resolves the TypeScript inner type name and generic parameter list for a branded newtype.
#[cfg(feature = "typescript")]
fn branded_ts_type_and_generics(
    generic_params: &[String],
    inner_ty: &syn::Type,
) -> (String, String) {
    let inner = get_field_def("_inner", inner_ty, "");
    let written = inner.typescript_typename();
    // The brand is an intersection, which binds tighter than the union a `Result` is written as.
    let ts_inner_type =
        if matches!(inner.field_type, FieldDefType::Result(..)) && inner.array_depth == 0 {
            format!("({written})")
        } else {
            written
        };
    let ts_generics = if generic_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", generic_params.join(", "))
    };
    (ts_inner_type, ts_generics)
}

/// Resolves the JSON schema shape for a branded newtype's inner field, read off the def
/// [`surface_field_def`] has already erased the brand's own type parameters out of.
#[cfg(feature = "jsonschema")]
fn branded_json_inner(inner: &FieldDef) -> BrandedJsonInner {
    #[cfg(feature = "mongodb")]
    if branded_inner_is_object_id(inner) {
        return BrandedJsonInner::ObjectId;
    }
    if branded_inner_is_composite(inner) {
        return BrandedJsonInner::Slot(Box::new(inner.clone()));
    }
    #[cfg(feature = "chrono")]
    if let Some(format) = chrono_json_schema_format(&inner.field_type) {
        return BrandedJsonInner::Chrono(format);
    }
    // A name is not a shape, so it is deferred to the type it names, through the reference every
    // other position defers it through. A `char` is the one-character string it is everywhere.
    if matches!(
        inner.field_type,
        FieldDefType::SiblingType(..) | FieldDefType::Char
    ) {
        return BrandedJsonInner::Slot(Box::new(inner.clone()));
    }
    // Every shape the shared mapping leaves unanswered has been returned above, so the fallback
    // stands for the one thing left over: a spelling that writes a bare string.
    BrandedJsonInner::Scalar(
        scalar_json_type_keyword(&inner.field_type)
            .unwrap_or("string")
            .to_owned(),
    )
}

/// The Zod string checks a branded newtype's `minLength`, `maxLength`, and `pattern` render to,
/// or the empty string when it carries none.
#[cfg(feature = "zod")]
fn branded_zod_string_checks(args: &ModelSchemaArgs) -> String {
    let mut checks = String::new();
    if let Some(min_len) = args.min_length {
        let reported = zod_error_arg(Bound::MinLength(min_len));
        checks = format!("{checks}.min({min_len}, {reported})");
    }
    if let Some(max_len) = args.max_length {
        let reported = zod_error_arg(Bound::MaxLength(max_len));
        checks = format!("{checks}.max({max_len}, {reported})");
    }
    if let Some(pattern) = &args.pattern {
        let literal_body = escape_js_regex_literal(pattern);
        let reported = zod_error_arg(Bound::Pattern(pattern));
        checks = format!("{checks}.check(z.regex(/{literal_body}/, {reported}))");
    }
    checks
}

/// The same three constraints [`branded_zod_string_checks`] renders, spelled instead as one
/// `.check(...)` call over the base functions (`z.minLength`, `z.maxLength`, `z.regex`) — the
/// surface every Zod schema exposes, unlike `.min`/`.max`, which live on `ZodString` alone.
#[cfg(feature = "zod")]
fn branded_zod_base_checks(args: &ModelSchemaArgs) -> String {
    let mut checks = Vec::new();
    if let Some(min_len) = args.min_length {
        let reported = zod_error_arg(Bound::MinLength(min_len));
        checks.push(format!("z.minLength({min_len}, {reported})"));
    }
    if let Some(max_len) = args.max_length {
        let reported = zod_error_arg(Bound::MaxLength(max_len));
        checks.push(format!("z.maxLength({max_len}, {reported})"));
    }
    if let Some(pattern) = &args.pattern {
        let literal_body = escape_js_regex_literal(pattern);
        let reported = zod_error_arg(Bound::Pattern(pattern));
        checks.push(format!("z.regex(/{literal_body}/, {reported})"));
    }
    if checks.is_empty() {
        String::new()
    } else {
        format!(".check({})", checks.join(", "))
    }
}

/// Whether a branded newtype's inner is an `ObjectId` written on its own, reaching the wire as the
/// `$oid` object rather than any string. An arrayed inner is excluded — it writes the array around
/// that object, not this shape.
#[cfg(all(feature = "mongodb", any(feature = "zod", feature = "jsonschema")))]
const fn branded_inner_is_object_id(inner: &FieldDef) -> bool {
    matches!(inner.field_type, FieldDefType::ObjectId) && !inner.is_array()
}

/// Whether a branded newtype's inner writes a composite — an array, a map, a tuple, or a value the
/// parser could not classify at all — rather than a value one `"type"` keyword already names.
#[cfg(feature = "jsonschema")]
fn branded_inner_is_composite(inner: &FieldDef) -> bool {
    if inner.is_array() {
        return true;
    }
    if let FieldDefType::SiblingType(name, args) = &inner.field_type
        && matches!(args.as_slice(), [_])
        && is_sequence_wrapper(name)
    {
        return true;
    }
    match &inner.field_type {
        FieldDefType::Map(..)
        | FieldDefType::Range(..)
        | FieldDefType::Result(..)
        | FieldDefType::Tuple(..)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown => true,
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::SiblingType(..)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::Usize => false,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => false,
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => false,
    }
}

/// Resolves the Zod base schema for a branded newtype's inner type, applying string constraints.
#[cfg(feature = "zod")]
fn branded_zod_inner(args: &ModelSchemaArgs, inner: &FieldDef) -> String {
    // A list's count is written on its own array, and a bare parameter's on its default.
    if inner.is_list() || matches!(inner.field_type, FieldDefType::TypeParam(_)) {
        return inner.zod_type();
    }
    let checks = branded_zod_string_checks(args);
    #[cfg(feature = "mongodb")]
    if branded_inner_is_object_id(inner) {
        return get_object_id_zod_schema_with(&checks);
    }
    format!("{}{checks}", inner.zod_type())
}

/// Builds the `ts_definition()` method for a branded newtype's schema module.
#[cfg(feature = "typescript")]
fn build_branded_ts_definition_method(
    item_name: &str,
    rust_ident: &str,
    ts_generics: &str,
    ts_inner_type: &str,
) -> proc_macro2::TokenStream {
    let reexport = ident_reexport_ts(rust_ident, item_name, ts_generics);
    #[cfg(feature = "zod")]
    {
        let type_str = format!(
            "export type {item_name}{ts_generics} = {ts_inner_type} & $brand<\"{item_name}\">;{reexport}"
        );
        quote! {
            pub fn ts_definition() -> ::std::string::String {
                #type_str.to_string()
            }
        }
    }
    #[cfg(not(feature = "zod"))]
    {
        let unique_symbol = format!("declare const __brand_{item_name}: unique symbol;");
        let type_str = format!(
            "export type {item_name}{ts_generics} = {ts_inner_type} & {{ readonly [__brand_{item_name}]: true }};{reexport}"
        );
        quote! {
            pub fn ts_definition() -> ::std::string::String {
                format!("{}\n{}", #unique_symbol, #type_str)
            }
        }
    }
}

/// The brand marker a build's flavour spells, appended to whatever schema the inner rendered. The
/// name is a *type* argument that only TypeScript reads; Zod's runtime brand takes none, so a
/// build emitting no TypeScript writes the bare call instead.
#[cfg(feature = "zod")]
fn zod_brand_call(item_name: &str) -> String {
    #[cfg(feature = "typescript")]
    {
        format!(".brand<\"{item_name}\">()")
    }
    #[cfg(not(feature = "typescript"))]
    {
        let _: &str = item_name;
        ".brand()".to_owned()
    }
}

/// The value a branded newtype's binding holds: the inner's own schema, the brand, and the
/// description, in the order the receiver's type admits.
#[cfg(feature = "zod")]
fn branded_zod_expression(
    args: &ModelSchemaArgs,
    item_name: &str,
    parameters: &[String],
    inner: &FieldDef,
    plain_description: &str,
) -> String {
    let value = zod_preprocess_wrap(
        zod_check_wrap(branded_zod_inner(args, inner), &args.check),
        &args.preprocess,
    );
    let brand = zod_brand_call(item_name);
    let described = format!(".meta({{\n  description: \"{plain_description}\",\n}})");
    if parameters.is_empty() {
        format!("{value}{brand}{described}")
    } else {
        format!("{value}{described}{brand}")
    }
}

/// Builds the `zod_schema()` method for a branded newtype's schema module.
#[cfg(feature = "zod")]
fn build_branded_zod_schema_method(
    args: &ModelSchemaArgs,
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    inner: &FieldDef,
    plain_description: &str,
) -> proc_macro2::TokenStream {
    let expression = branded_zod_expression(args, item_name, parameters, inner, plain_description);
    let reexport = zod_binding_reexport(rust_ident, item_name, parameters);
    let body = if parameters.is_empty() {
        branded_zod_const_block(item_name, &expression, &reexport)
    } else {
        let checks = branded_zod_string_checks(args);
        let branded_checks = BrandedDefaultChecks {
            base: branded_zod_base_checks(args),
            chained: checks,
        };
        let constrained_default = match (&inner.field_type, branded_checks.chained.is_empty()) {
            (FieldDefType::TypeParam(parameter), false) if !inner.is_array() => {
                Some((parameter.as_str(), &branded_checks))
            }
            _ => None,
        };
        let defaults = ZodDefaultInputs {
            // `.brand()` narrows at the value position, which no restated type can name.
            #[cfg(feature = "typescript")]
            annotated_by_value: true,
            constrained: constrained_default,
            default_types: &args.default_types,
        };
        zod_factory_block(
            item_name,
            rust_ident,
            parameters,
            &defaults,
            "",
            &expression,
            &reexport,
        )
    };
    quote! {
        pub fn zod_schema() -> ::std::string::String {
            #body.to_owned()
        }
    }
}

/// The binding a brand that declares no parameter publishes. A brand always reads its annotation
/// back off the value: `.brand()` narrows at the value position, and no class named here could
/// stay true of whatever the inner rendered to.
#[cfg(feature = "zod")]
fn branded_zod_const_block(item_name: &str, expression: &str, reexport: &str) -> String {
    zod_const_block(item_name, "", expression, reexport, true)
}

/// Builds the delegate methods (on the newtype impl) that forward to its schema module.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn build_branded_delegate_items(
    module_ident: &Ident,
    has_example: bool,
) -> Vec<proc_macro2::TokenStream> {
    #[cfg(not(feature = "zod"))]
    let _: &_ = &has_example;

    #[cfg(feature = "typescript")]
    let delegate_ts = quote! {
        pub fn ts_definition() -> ::std::string::String {
            #module_ident::Schema::ts_definition()
        }
    };

    #[cfg(feature = "zod")]
    let delegate_zod = {
        let examples_json = zod_examples_json(has_example);
        quote! {
            pub fn zod_schema() -> ::std::string::String {
                let base_schema = #module_ident::Schema::zod_schema();
                let examples_json = #examples_json;
                // The one `.meta({` a brand writes closes on its own line, and so does each check
                // written ahead of it, so the anchor is the first such close after the
                // description.
                let described = base_schema.find(".meta({\n  description: ").unwrap_or_default();
                let closed = base_schema[described..].find("\n})").map(|close| described + close);
                if let ::core::option::Option::Some(pos) = closed {
                    let mut result = base_schema[..pos].to_string();
                    result.push_str(&format!("\n  examples: {},", examples_json));
                    result.push_str(&base_schema[pos..]);
                    result
                } else {
                    base_schema
                }
            }
        }
    };

    #[cfg(feature = "jsonschema")]
    let delegate_json_schema = quote! {
        pub fn json_schema() -> serde_json::Value {
            #module_ident::Schema::json_schema()
        }
    };

    vec![
        #[cfg(feature = "jsonschema")]
        delegate_json_schema,
        #[cfg(feature = "typescript")]
        delegate_ts,
        #[cfg(feature = "zod")]
        delegate_zod,
    ]
}

/// The `where` clause bounds the inner field's own type, not just the brand's generic params, so
/// a non-generic brand over a non-`Display` inner raises one `E0277` at the field instead of an
/// unbounded `E0599` inside the generated impl.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn build_branded_display_impl(
    generics: &syn::Generics,
    name: &Ident,
    inner_field: &Field,
    held: &syn::Member,
) -> proc_macro2::TokenStream {
    let (_, type_generics, _) = generics.split_for_impl();
    let mut display_generics = generics.clone();
    let inner_ty = &inner_field.ty;
    let bound_span = inner_field
        .ty
        .span()
        .resolved_at(proc_macro2::Span::call_site());
    display_generics
        .make_where_clause()
        .predicates
        .push(syn::parse_quote_spanned!(bound_span=> #inner_ty: ::std::fmt::Display));
    let (display_impl_generics, _, display_where_clause) = display_generics.split_for_impl();
    // Spanned whole on the field's type, the member with it, which is where a non-`Display`
    // inner is reported.
    let mut spanned = held.clone();
    match &mut spanned {
        syn::Member::Named(field_name) => field_name.set_span(inner_field.ty.span()),
        syn::Member::Unnamed(slot) => slot.span = inner_field.ty.span(),
    }
    let delegate =
        quote_spanned! {inner_field.ty.span()=> ::std::fmt::Display::fmt(&self.#spanned, f) };
    quote! {
        impl #display_impl_generics ::std::fmt::Display for #name #type_generics #display_where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                #delegate
            }
        }
    }
}

/// Builds a branded newtype's `Display` impl together with the static assertion guarding it.
/// `no_display` drops the impl; it drops the assertion only when nothing else needs `Display`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn build_branded_display_tokens(
    generics: &syn::Generics,
    name: &Ident,
    inner_field: &Field,
    held: &syn::Member,
    args: &ModelSchemaArgs,
) -> proc_macro2::TokenStream {
    // Only the serde build emits the validation functions that call `to_string()`.
    #[cfg(feature = "serde")]
    let validation_needs_display = args.has_string_constraints()
        && !branded_inner_measures_path(&inner_field.ty)
        && !held_as_a_list(&inner_field.ty);
    #[cfg(not(feature = "serde"))]
    let validation_needs_display = false;

    if args.no_display && !validation_needs_display {
        return quote! {};
    }
    if args.no_display {
        // No impl is emitted in this branch, so nothing else asserts `Display` on the inner.
        return build_branded_display_assertion(inner_field, generics);
    }
    // The impl's own `where` clause performs this check, spanned on the field; a separate assertion
    // here would double the diagnostic.
    build_branded_display_impl(generics, name, inner_field, held)
}

/// Builds a static assertion that the branded newtype's inner type implements `Display`, spanned
/// on the inner field so a violation surfaces as an `E0277` naming the trait at the field instead
/// of one raised deep inside the generated impl.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn build_branded_display_assertion(
    inner_field: &Field,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    if type_mentions_generic_param(&inner_field.ty, generics) {
        return quote! {};
    }
    let inner_ty = &inner_field.ty;
    // The bound goes in a `where` clause: these tokens carry the user's span, so a consumer's
    // lints judge them as if they were hand-written there.
    quote_spanned! {inner_field.ty.span()=>
        const _: () = {
            const fn assert_display<T>()
            where
                T: ::std::fmt::Display,
            {
            }
            assert_display::<#inner_ty>();
        };
    }
}

/// Reports whether `ty` names any of `generics`' parameters (type, lifetime, or const).
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn type_mentions_generic_param(ty: &syn::Type, generics: &syn::Generics) -> bool {
    let param_names: Vec<String> = generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Type(type_param) => type_param.ident.to_string(),
            syn::GenericParam::Lifetime(lifetime_param) => {
                lifetime_param.lifetime.ident.to_string()
            }
            syn::GenericParam::Const(const_param) => const_param.ident.to_string(),
        })
        .collect();
    !param_names.is_empty() && tokens_name_any(&quote! { #ty }, &param_names)
}

/// Reports whether `tokens` contains an identifier from `names` at any nesting depth.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn tokens_name_any(tokens: &proc_macro2::TokenStream, names: &[String]) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => names.iter().any(|name| ident == name.as_str()),
        proc_macro2::TokenTree::Group(group) => tokens_name_any(&group.stream(), names),
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

/// Builds the `schema_examples()` and `schema_example()` methods for a branded newtype from each
/// extracted example's code.
#[cfg(feature = "zod")]
fn build_branded_schema_example(
    example_tokens: &[proc_macro2::TokenStream],
    name: &Ident,
    generic_params: &[String],
    args: &ModelSchemaArgs,
) -> proc_macro2::TokenStream {
    schema_examples_methods(
        example_tokens,
        &schema_example_value_type(name, generic_params, &args.default_types),
    )
    .unwrap_or_default()
}

/// What a brand's own string checks add to its `validate()`: the validator of its schema module,
/// run on the value the brand holds.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn branded_bound_call(
    validation: &BrandedValidation,
    module_ident: &Ident,
) -> proc_macro2::TokenStream {
    let (validator, checked_inner) = (&validation.validator, &validation.checked_inner);
    quote! {
        if let ::core::result::Result::Err(reported) = #module_ident::#validator(#checked_inner) {
            errors.extend(reported);
        }
    }
}

/// The `validate()` a brand earns: `bound`, what its own string checks run, then the checks it
/// names. Nothing for a brand that carries neither.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn with_branded_checks(
    bound: Option<&proc_macro2::TokenStream>,
    named: &[FunctionName],
) -> proc_macro2::TokenStream {
    let own = checks::ItemChecks {
        named: !named.is_empty(),
        own: named,
    }
    .own_calls(VALIDATE_SINK);
    if bound.is_none() && own.is_empty() {
        return proc_macro2::TokenStream::new();
    }
    branded_validate_method(bound, &own)
}

/// A brand's `validate()`: its own string checks, then the checks it names, each on the brand
/// itself. A brand is the value and no member of anything, so no message here names a path.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_validate_method(
    bound: Option<&proc_macro2::TokenStream>,
    own: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    quote! {
        pub fn validate(&self) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
            let mut errors = ::std::vec::Vec::new();
            #bound
            #(#own)*
            if errors.is_empty() { ::core::result::Result::Ok(()) } else { ::core::result::Result::Err(errors) }
        }
    }
}

/// Assembles the final macro output for a branded newtype: the (possibly attribute-injected)
/// struct, its `Display` impl, the schema module, and the type's `impl`, which holds its
/// delegates and its `validate()` when it has one.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn assemble_branded_output(parts: &BrandedNewtypeOutput) -> TokenStream {
    let item_struct = parts.item_struct;
    let (impl_generics, type_generics, where_clause) = item_struct.generics.split_for_impl();
    let display_tokens = parts.display_tokens;
    let module_ident = parts.module_ident;
    let schema_impl_items = parts.schema_impl_items;
    let validation_tokens = parts.validation_tokens;
    let name = parts.name;
    let delegate_impl_items = parts.delegate_impl_items;
    let schema_example_tokens = parts.schema_example_tokens;
    let validate_method = parts.validate_method;
    let bound_alone = parts.bound_alone;
    let bound_text_method = parts.bound_text_method;

    let module_items = schema_module_items(
        quote! {
            #[non_exhaustive]
            pub struct Schema;

            impl Schema {
                #(#schema_impl_items)*
            }

            #validation_tokens
        },
        &parts.decode_with.schema_module,
    );
    let decode_with_impl = &parts.decode_with.type_impl;

    let output = quote! {
        #item_struct

        #display_tokens

        pub mod #module_ident {
            use super::*;

            #module_items
        }

        impl #impl_generics #name #type_generics #where_clause {
            #(#delegate_impl_items)*
            #schema_example_tokens
            #validate_method
            #bound_alone
            #bound_text_method
        }

        #decode_with_impl
    };

    log::trace!("{output}");

    output
}

/// The output a branded newtype its guards refused is replaced by, or `None` when it earns none.
/// Read before the type is registered: a rejected brand emits no schema, so nothing else should be
/// able to resolve a reference to one.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_guard_failure_output(
    item_struct: &syn::ItemStruct,
    args: &ModelSchemaArgs,
) -> Option<TokenStream> {
    refused_item_output(
        &Item::Struct(item_struct.clone()),
        &branded_guard_errors(item_struct, args),
    )
}

/// Processes a branded newtype — a `#[serde(transparent)]` struct serde writes as the value of one
/// field — into the parts [`assemble_branded_output`] joins: the struct itself, the `Display` impl
/// it earns, its schema module, and the delegate impl forwarding to that module.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn register_branded_newtype(
    item_struct: &syn::ItemStruct,
    rust_ident: &str,
    item_name: &str,
    module_name: &str,
) {
    let inner_field = brand_field(item_struct).unwrap();
    register_alias_info(
        rust_ident,
        item_name,
        module_name,
        branded_alias_kind(inner_field),
    );
    // A brand adds a name to its inner's wire and nothing else, so a key written under it renders
    // in whatever form the inner writes.
    record_key_wire(rust_ident, branded_key_wire(inner_field));
    // A brand is written straight onto its inner's schema — `.brand()` hands back that same
    // instance — so it publishes whatever its inner publishes.
    Surface::written(
        &branded_inner_value_surface(&item_struct.generics, inner_field),
        &type_parameters_in_scope(&item_struct.generics),
    )
    .record(rust_ident);
}

/// Registers the brand, then records the consult question its own registration could not answer —
/// in that order, so a brand naming itself cannot answer its own.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn register_brand_with_questions(
    item_struct: &syn::ItemStruct,
    args: &ModelSchemaArgs,
    rust_ident: &str,
    item_name: &str,
    module_name: &str,
) {
    register_branded_newtype(item_struct, rust_ident, item_name, module_name);
    if let Some(question) = deferred_shape_question(item_struct, args) {
        record_shape_question(question);
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn process_branded_newtype(item_struct: syn::ItemStruct, args: &ModelSchemaArgs) -> TokenStream {
    if let Some(output) = branded_guard_failure_output(&item_struct, args) {
        return output;
    }

    let name = item_struct.ident.clone();
    let rust_ident = name.to_string();
    let item_name = compute_item_export_name(&rust_ident, args.name_override.as_deref());
    let module_name = ident_schema_module_name(&rust_ident);
    let module_ident = Ident::new(&module_name, name.span());

    register_brand_with_questions(&item_struct, args, &rust_ident, &item_name, &module_name);

    #[cfg(feature = "zod")]
    let docs_vec = get_struct_docs(&item_struct);

    #[cfg(feature = "zod")]
    let example_tokens = extract_examples_tokens(&item_struct.attrs);

    #[cfg(feature = "zod")]
    let plain_description = item_description(docs_vec.as_deref(), &item_name);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "serde"))]
    let generic_params = type_parameters_in_scope(&item_struct.generics);

    let (position, inner_field) = brand_slot(&item_struct).unwrap();
    let held = brand_member(position, inner_field);
    let inner_ty = &inner_field.ty;

    // `ts_pair`: (ts_inner_type, ts_generics).
    #[cfg(feature = "typescript")]
    let ts_pair = branded_ts_type_and_generics(&generic_params, inner_ty);

    #[cfg(not(any(feature = "typescript", feature = "zod")))]
    let _: &_ = &inner_ty;

    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    let value_inner = branded_value_inner(&item_struct.generics, inner_ty, args);

    #[cfg(feature = "jsonschema")]
    let json_inner = branded_json_inner(&value_inner);

    #[cfg(feature = "typescript")]
    let ts_definition_method =
        build_branded_ts_definition_method(&item_name, &rust_ident, &ts_pair.1, &ts_pair.0);

    #[cfg(feature = "zod")]
    let zod_schema_method = build_branded_zod_schema_method(
        args,
        &item_name,
        &rust_ident,
        &generic_params,
        &value_inner,
        &plain_description,
    );

    #[cfg(feature = "serde")]
    let bound = BrandBound::of(
        build_branded_validation(args, &generic_params, inner_field, &held).as_ref(),
        &module_ident,
    );
    #[cfg(not(feature = "serde"))]
    let bound = BrandBound::default();

    #[cfg(feature = "jsonschema")]
    let json_schema_method = build_branded_json_schema_method(
        args,
        &json_inner,
        &item_name,
        &schema_parameters(&item_struct.generics, args),
    );

    let schema_impl_items: Vec<proc_macro2::TokenStream> = vec![
        #[cfg(feature = "jsonschema")]
        json_schema_method,
        #[cfg(feature = "jsonschema")]
        unit_member_record_item(&Item::Struct(item_struct.clone())),
        #[cfg(feature = "typescript")]
        ts_definition_method,
        #[cfg(feature = "zod")]
        zod_schema_method,
    ];

    #[cfg(feature = "zod")]
    let has_example = !example_tokens.is_empty();
    #[cfg(not(feature = "zod"))]
    let has_example = false;

    #[cfg(feature = "zod")]
    let schema_example_tokens =
        build_branded_schema_example(&example_tokens, &name, &generic_params, args);
    #[cfg(not(feature = "zod"))]
    let schema_example_tokens = quote! {};

    let delegate_impl_items = build_branded_delegate_items(&module_ident, has_example);

    let display_tokens =
        build_branded_display_tokens(&item_struct.generics, &name, inner_field, &held, args);

    let validate_method = with_branded_checks(bound.held.as_ref(), &args.check);

    // The brand as it is emitted keeps no serde attribute where it derives `Deserialize` alone.
    let bound_text_method = branded_bound_text_method(&item_struct);

    let (decode_with, read_struct) = branded_read(
        item_struct,
        args,
        !validate_method.is_empty(),
        bound.held.is_some(),
    );

    assemble_branded_output(&BrandedNewtypeOutput {
        bound_alone: &bound.alone,
        bound_text_method: &bound_text_method,
        decode_with: &decode_with,
        delegate_impl_items: &delegate_impl_items,
        display_tokens: &display_tokens,
        item_struct: &read_struct,
        module_ident: &module_ident,
        name: &name,
        schema_example_tokens: &schema_example_tokens,
        schema_impl_items: &schema_impl_items,
        validate_method: &validate_method,
        validation_tokens: &bound.module_items,
    })
}

/// What is written beside a brand after its own `impl`, and the brand as it is emitted: what
/// `decode_with` adds, and the read the macro writes for a brand that `publishes_validate` or
/// names a `preprocess`, each function called on the payload, then the brand's own read, then its
/// `validate()`. A brand with neither keeps the read serde's derive writes it. `bounded` says
/// whether the brand carries a bound of its own.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_read(
    mut brand: syn::ItemStruct,
    args: &ModelSchemaArgs,
    publishes_validate: bool,
    bounded: bool,
) -> (DecodeWithParts, syn::ItemStruct) {
    let mut beside = brand_decode_with(&brand, args, bounded);
    let judged = judged_read::of_struct(&mut brand, publishes_validate);
    beside.type_impl.extend(judged);
    (beside, brand)
}

/// [`struct_decode_with`] for a brand. Where the brand is `bounded`, the read its issues are
/// listed from holds the value to the brand's own bound, which no walk lists.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn brand_decode_with(
    brand: &syn::ItemStruct,
    args: &ModelSchemaArgs,
    bounded: bool,
) -> DecodeWithParts {
    if !(bounded && args.enable.decode_with) {
        return struct_decode_with(brand, args);
    }
    let added = brand_recovering_decode(brand, args.enable.mongodb);
    let (walkers, checks, probes) = (added.type_impl, added.check_messages, added.probes);
    DecodeWithParts {
        schema_module: added.schema_module,
        type_impl: quote! { #walkers #checks #probes },
    }
}

/// Nothing, where no `serde` feature reads the attributes a walker is written from.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn brand_decode_with(
    _brand: &syn::ItemStruct,
    _args: &ModelSchemaArgs,
    _bounded: bool,
) -> DecodeWithParts {
    DecodeWithParts::default()
}

/// `model_schema_bound_text`: the text a bound written on a field of this brand's type is measured
/// on. Published where the brand's inner is itself a string or a path, so a brand over anything
/// else has none and such a bound is refused where it was written.
///
/// This builds, and `validate()` holds `latest` to the bound:
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// #[serde(transparent)]
/// pub struct Label(pub String);
///
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// pub struct Record {
///     #[model_schema_prop(minLength = 1)]
///     pub latest: Label,
/// }
///
/// fn main() {
///     let empty = Record { latest: Label(String::new()) };
///     assert!(empty.validate().is_err());
/// }
/// ```
///
/// The run below types the field with a struct instead, and nothing else changed:
///
/// ```rust,compile_fail
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// pub struct Version {
///     pub number: i32,
/// }
///
/// #[model_schema()]
/// #[derive(Deserialize, Serialize)]
/// pub struct Record {
///     #[model_schema_prop(minLength = 1)]
///     pub latest: Version,
/// }
///
/// fn main() {}
/// ```
///
/// A `compile_fail` doctest asserts only that some error was raised, so the snippet was compiled
/// standalone as an ordinary test file, and this is the only error it earned:
///
/// ```text
/// error[E0277]: model_schema: field `latest`: `minLength` cannot apply to `Version`
///   --> tests/zz_probe.rs:14:17
///    |
/// 10 | #[model_schema()]
///    | ----------------- in this attribute macro expansion
/// ...
/// 14 |     pub latest: Version,
///    |                 ^^^^^^^ this type holds no string for the bound to measure
///    |
/// help: the trait `BoundedText` is not implemented for `Version`
///    = note: a length or a pattern is measured on a `String`, on a path, or on a `#[serde(transparent)]` brand over one of them that `#[model_schema]` was written on: declare the field as one of those, or drop the bound
/// ```
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn branded_bound_text_method(item_struct: &syn::ItemStruct) -> proc_macro2::TokenStream {
    let Some((position, inner_field)) = brand_slot(item_struct) else {
        return quote! {};
    };
    let held = brand_member(position, inner_field);
    let Some(shape) = constrained_shape(&inner_field.ty) else {
        return quote! {};
    };
    if !shape
        .wraps
        .iter()
        .all(|wrap| matches!(wrap, ConstraintWrap::Transparent))
    {
        return quote! {};
    }
    let text = match shape.leaf {
        ConstraintLeaf::Path => quote! { self.#held.to_string_lossy() },
        ConstraintLeaf::Str => quote! { ::std::borrow::Cow::Borrowed(&*self.#held) },
        ConstraintLeaf::List | ConstraintLeaf::Named(_) | ConstraintLeaf::Number(_) => {
            return quote! {};
        }
    };
    quote! {
        pub fn model_schema_bound_text(&self) -> ::std::borrow::Cow<'_, str> {
            #text
        }
    }
}

/// Without `serde` no validator is generated, so nothing measures a brand's text.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn branded_bound_text_method(_item_struct: &syn::ItemStruct) -> proc_macro2::TokenStream {
    quote! {}
}

/// Processes an enum item and generates TypeScript and Zod schema definitions for it.
fn process_enum(item_enum: syn::ItemEnum, args: &ModelSchemaArgs) -> TokenStream {
    let name = item_enum.ident.clone();

    #[cfg(feature = "serde")]
    let serde_type_meta = parse_serde_type_attributes(&item_enum.attrs);

    #[cfg(feature = "serde")]
    if let Some(output) = refused_item_output(
        &Item::Enum(item_enum.clone()),
        &enum_cfg_attr_guard_errors(&item_enum, &serde_type_meta),
    ) {
        return output;
    }

    // Every enum shape below is written under this one name, and `enum_module_idents` registers it,
    // so the override reaches the shape's own surfaces and every reference to it alike.
    let item_name = compute_item_export_name(&name.to_string(), args.name_override.as_deref());

    // A tag names a key serde writes whether or not a variant carries a field. Only a declaration
    // serde writes as the bare variant name is the string union the plain-enum path publishes.
    #[cfg(feature = "serde")]
    let writes_bare_variant_names = serde_type_meta.tag.is_none()
        && serde_type_meta.content.is_none()
        && !serde_type_meta.untagged;

    // No attribute is read without the `serde` feature, so no declaration can be told from the
    // untagged default and the bare-name form stands.
    #[cfg(not(feature = "serde"))]
    let writes_bare_variant_names = true;

    if is_plain_enum(&item_enum) && writes_bare_variant_names {
        #[cfg(feature = "serde")]
        let rename_all = serde_type_meta.rename_all.as_deref();

        #[cfg(not(feature = "serde"))]
        let rename_all = None;

        process_plain_enum(item_enum, &name, rename_all, &item_name, args)
    } else {
        #[cfg(feature = "serde")]
        if serde_type_meta.untagged {
            return process_untagged_enum(item_enum, &name, &item_name, args);
        }

        #[cfg(feature = "serde")]
        let casing = EnumCasing {
            variant_fields: serde_type_meta.rename_all_fields.as_deref(),
            variants: serde_type_meta.rename_all.as_deref(),
        };

        #[cfg(not(feature = "serde"))]
        let casing = EnumCasing {
            variant_fields: None,
            variants: None,
        };

        // Neither tagging key named, so serde writes the externally tagged form and that is what
        // the surfaces describe.
        #[cfg(feature = "serde")]
        if serde_type_meta.tag.is_none() && serde_type_meta.content.is_none() {
            return process_externally_tagged_enum(item_enum, &name, casing, &item_name, args);
        }

        // A tag with no content is serde's internally tagged form: what a variant writes joins the
        // tag in one object rather than sitting under a key of its own.
        #[cfg(feature = "serde")]
        if let (Some(tag), None) = (
            serde_type_meta.tag.as_ref(),
            serde_type_meta.content.as_ref(),
        ) {
            return process_internally_tagged_enum(item_enum, &name, tag, casing, &item_name, args);
        }

        #[cfg(feature = "serde")]
        let (tag_name, content_name) = (
            serde_type_meta
                .tag
                .as_ref()
                .map_or_else(|| "type".to_owned(), Clone::clone),
            serde_type_meta
                .content
                .as_ref()
                .map_or_else(|| "value".to_owned(), Clone::clone),
        );

        #[cfg(not(feature = "serde"))]
        let (tag_name, content_name): (String, String) = ("type".to_owned(), "value".to_owned());

        process_discriminated_enum(
            item_enum,
            &name,
            &tag_name,
            &content_name,
            casing,
            &item_name,
            args,
        )
    }
}

/// Flattens an item's doc comments into the plain lines both its `JSDoc` body and its description
/// are spelled from: ` ```rust example ` blocks stripped, leading `*` and surrounding whitespace
/// trimmed, blank lines dropped.
#[cfg(any(feature = "typescript", feature = "zod"))]
fn item_plain_doc_lines(doc_lines: &[String]) -> Vec<String> {
    strip_examples_from_docs(doc_lines)
        .into_iter()
        .flat_map(|v| {
            v.lines()
                .map(|line| {
                    let trimmed = line.trim();
                    trimmed
                        .strip_prefix('*')
                        .unwrap_or(trimmed)
                        .trim()
                        .to_owned()
                })
                .collect::<Vec<_>>()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// An item's doc lines, flattened the way the calling surface reads them, or the exported name
/// (not the declared one) when it says nothing — so a `JSDoc` header never contradicts the
/// `export type` line beneath it.
fn item_lines_or_name(
    docs_vec: Option<&[String]>,
    item_name: &str,
    flatten: impl FnOnce(&[String]) -> Vec<String>,
) -> Vec<String> {
    let lines = docs_vec.map_or_default(flatten);
    if lines.is_empty() {
        vec![item_name.to_owned()]
    } else {
        lines
    }
}

/// The `JSDoc` body an alias's `export type` is emitted under.
#[cfg(feature = "typescript")]
fn alias_jsdoc_body(docs_vec: Option<&[String]>, export_name: &str) -> String {
    item_jsdoc_body(&item_lines_or_name(
        docs_vec,
        export_name,
        strip_examples_from_docs,
    ))
}

/// The `JSDoc` body a set of lines is written as: each prefixed with ` * `, the block closed by a
/// bare ` * ` so it ends on an empty line. Ungated for the reason [`item_lines_or_name`] is.
fn item_jsdoc_body(lines: &[String]) -> String {
    lines
        .iter()
        .map(|l| format!(" * {l}"))
        .chain([" * ".to_owned()])
        .collect::<Vec<_>>()
        .join("\n")
}

/// The complete `JSDoc` block a body is written as, at the indent of whatever it documents: the
/// opener, the body's own lines carried to that indent, and the `*/` that closes it.
fn jsdoc_block(body: &str, indent: &str) -> String {
    let mut block = format!("{indent}/**\n");
    for line in body.lines() {
        let _ = writeln!(block, "{indent}{line}");
    }
    let _ = write!(block, "{indent} */");
    block
}

/// The `JSDoc` block a member is written under: [`jsdoc_block`] at the one indent every emitted
/// member sits at.
fn member_jsdoc_block(body: &str) -> String {
    jsdoc_block(body, MEMBER_INDENT)
}

/// The spelling a member key is written as on the text surfaces.
fn ts_member_key(key: &str) -> String {
    let mut chars = key.chars();
    let heads_an_identifier = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '$');
    if heads_an_identifier
        && chars.all(|char| char.is_ascii_alphanumeric() || char == '_' || char == '$')
    {
        return key.to_owned();
    }
    ts_quoted(key)
}

/// The one-line description a set of lines is published as, escaped for the `description: "…"` it
/// is spliced into. The escape is a no-op on the exported-name fallback: a `name` override must
/// spell an identifier, which cannot contain a quote.
#[cfg(any(feature = "typescript", feature = "zod"))]
fn describe_item_lines(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| escape_js_double_quoted(line))
        .collect::<Vec<_>>()
        .join("\\n")
}

/// The escaped one-line description an item publishes, falling back to the exported name when it
/// carries no docs — spelled from the same lines a `JSDoc` header would use, so the two cannot
/// drift apart.
#[cfg(feature = "zod")]
fn item_description(docs_vec: Option<&[String]>, item_name: &str) -> String {
    describe_item_lines(&item_lines_or_name(
        docs_vec,
        item_name,
        item_plain_doc_lines,
    ))
}

/// Flattens an item's doc comments into a `JSDoc` body and an escaped one-line description, both
/// written from one pass over the same lines (with ` ```rust example ` blocks stripped). Falls
/// back to the exported name when there are no docs, matching every other item path.
#[cfg(any(feature = "typescript", feature = "zod"))]
fn build_item_docs_and_description(
    docs_vec: Option<&[String]>,
    item_name: &str,
) -> (String, String) {
    let plain_lines = item_lines_or_name(docs_vec, item_name, item_plain_doc_lines);

    (
        item_jsdoc_body(&plain_lines),
        describe_item_lines(&plain_lines),
    )
}

/// Collects a plain enum's serialized variant names (respecting serde renames) and per-variant
/// doc strings (the latter only populated when the `typescript` feature is enabled), of each
/// variant serde writes or reads.
fn collect_plain_enum_options(
    item_enum: &mut syn::ItemEnum,
    rename_all: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let mut enum_options = Vec::new();
    #[cfg(feature = "typescript")]
    let mut enum_variant_docs = Vec::new();
    #[cfg(not(feature = "typescript"))]
    let enum_variant_docs: Vec<String> = Vec::new();

    for item in item_enum
        .variants
        .iter_mut()
        .filter(|item| variant_on_the_wire(item))
    {
        #[cfg(feature = "serde")]
        let field_rename = parse_serde_field_attributes(&item.attrs).rename;
        #[cfg(not(feature = "serde"))]
        let field_rename: Option<String> = None;

        let final_name =
            get_final_variant_name(&item.ident.to_string(), field_rename.as_deref(), rename_all);
        enum_options.push(final_name);

        #[cfg(feature = "typescript")]
        {
            // The one member body not written as ` * ` lines: a plain enum's variants are commented
            // inside the union rather than over a property.
            let variant_docs = get_variant_docs(item).map_or_else(String::new, |doc_lines| {
                strip_examples_from_docs(&doc_lines).join("\n")
            });
            enum_variant_docs.push(variant_docs);
        }
    }

    (enum_options, enum_variant_docs)
}

/// Builds the TypeScript union body (`  | "Variant"`, with `JSDoc` per variant) for a plain enum.
#[cfg(feature = "typescript")]
fn build_plain_enum_type_code(enum_options: &[String], enum_variant_docs: &[String]) -> String {
    enum_options
        .iter()
        .enumerate()
        .map(|(idx, v)| {
            let docs = &enum_variant_docs[idx];
            let quoted = ts_quoted(v);
            if docs.is_empty() {
                format!("  | {quoted}")
            } else {
                let formatted_docs = docs
                    .lines()
                    .map(|line| {
                        let trimmed = line.trim();
                        let content = trimmed.strip_prefix('*').unwrap_or(trimmed).trim();
                        if content.is_empty() {
                            "  *".to_owned()
                        } else {
                            format!("  * {content}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("  /*\n{formatted_docs}\n  */\n  | {quoted}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `case "wire": return "Rust";` arms a `{Enum}$Variant` reader switches over, one per
/// variant.
#[cfg(feature = "typescript")]
fn variant_reader_case_arms(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(wire, rust)| format!("    case {}: return \"{rust}\";", ts_quoted(wire)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `{Enum}$Variant` reader for a unit-only enum with no tag: the value itself is the wire
/// name.
#[cfg(feature = "typescript")]
fn plain_enum_variant_reader(item_name: &str, pairs: &[(String, String)]) -> String {
    format!(
        "/** The Rust variant name `value` carries, or \"\" where it carries none. */\n\
         export function {item_name}$Variant(value: unknown): string {{\n  switch (value) {{\n{}\n    default: return \"\";\n  }}\n}}",
        variant_reader_case_arms(pairs)
    )
}

/// The `(wire name, Rust name)` pairs a unit-only enum's own reader switches on, one per variant
/// serde writes or reads.
#[cfg(feature = "typescript")]
fn plain_enum_reader_pairs(
    item_enum: &syn::ItemEnum,
    rename_all: Option<&str>,
) -> Vec<(String, String)> {
    item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
        .map(|variant| {
            #[cfg(feature = "serde")]
            let field_rename = parse_serde_field_attributes(&variant.attrs).rename;
            #[cfg(not(feature = "serde"))]
            let field_rename: Option<String> = None;
            let rust_ident = variant.ident.to_string();
            let wire_name =
                get_final_variant_name(&rust_ident, field_rename.as_deref(), rename_all);
            (wire_name, rust_ident)
        })
        .collect()
}

/// The `{Enum}$Variant` reader for an internally or adjacently tagged enum: the wire name sits
/// under the tag key, whatever else the value carries beside it.
#[cfg(feature = "typescript")]
fn tagged_enum_variant_reader(
    item_name: &str,
    tag_name: &str,
    pairs: &[(String, String)],
) -> String {
    let key = ts_member_key(tag_name);
    let read = if key.starts_with('"') {
        format!("(value as Record<string, unknown>)[{key}]")
    } else {
        format!("(value as {{ {key}?: unknown }}).{key}")
    };
    format!(
        "/** The Rust variant name `value` carries, or \"\" where it carries none. */\n\
         export function {item_name}$Variant(value: unknown): string {{\n  if (typeof value !== \"object\" || value === null) return \"\";\n  switch ({read}) {{\n{}\n    default: return \"\";\n  }}\n}}",
        variant_reader_case_arms(pairs)
    )
}

/// The `{Enum}$Variant` reader for an externally tagged enum: a payload variant writes the wire
/// name as the object's sole key, a unit variant as a bare string.
#[cfg(all(feature = "typescript", feature = "serde"))]
fn externally_tagged_enum_variant_reader(item_name: &str, pairs: &[(String, String)]) -> String {
    format!(
        "/** The Rust variant name `value` carries, or \"\" where it carries none. */\n\
         export function {item_name}$Variant(value: unknown): string {{\n  const key = typeof value === \"string\" ? value\n    : typeof value === \"object\" && value !== null && Object.keys(value).length === 1 ? Object.keys(value)[0]\n    : undefined;\n  switch (key) {{\n{}\n    default: return \"\";\n  }}\n}}",
        variant_reader_case_arms(pairs)
    )
}

/// The `{Enum}$Variant` reader for an untagged enum: no serde form carries the variant's own name,
/// so there is nothing to read.
#[cfg(all(feature = "typescript", feature = "serde"))]
fn untagged_enum_variant_reader(item_name: &str) -> String {
    format!(
        "/** The Rust variant name `value` carries, or \"\" where it carries none. */\n\
         export function {item_name}$Variant(_value: unknown): string {{\n  return \"\";\n}}"
    )
}

/// The `(wire name, Rust name)` pairs a discriminated enum's own reader switches on, one per
/// variant serde writes or reads — computed straight off the declaration, which is all the reader
/// needs.
#[cfg(feature = "typescript")]
fn discriminated_variant_reader_pairs(
    item_enum: &syn::ItemEnum,
    casing: EnumCasing<'_>,
) -> Vec<(String, String)> {
    item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
        .map(|variant| {
            let (field_rename, _) = variant_serde_names(&variant.attrs, casing.variant_fields);
            let rust_ident = variant.ident.to_string();
            let wire_name =
                get_final_variant_name(&rust_ident, field_rename.as_deref(), casing.variants);
            (wire_name, rust_ident)
        })
        .collect()
}

/// The plain-enum `ts_definition()` method, its `{Enum}$Variant` reader computed and appended in
/// one call — kept off `process_plain_enum` so that function stays under the line lint.
#[cfg(feature = "typescript")]
fn plain_enum_ts_definition_method(
    docs: &str,
    item_name: &str,
    type_code: &str,
    item_enum: &syn::ItemEnum,
    rename_all: Option<&str>,
) -> proc_macro2::TokenStream {
    let rust_ident = item_enum.ident.to_string();
    let ts_generics = ts_generic_params(&item_enum.generics);
    let reader =
        plain_enum_variant_reader(item_name, &plain_enum_reader_pairs(item_enum, rename_all));
    generate_plain_enum_ts_definition_method(
        docs,
        item_name,
        &rust_ident,
        &ts_generics,
        type_code,
        &reader,
    )
}

/// The internally- or adjacently-tagged `ts_definition()` method, its reader computed and appended
/// in one call — kept off the two `process_*` functions that call it so they stay under the line
/// lint.
#[cfg(feature = "typescript")]
fn tagged_ts_definition_method(
    docs: &str,
    item_name: &str,
    type_code: &str,
    tag_name: &str,
    item_enum: &syn::ItemEnum,
    casing: EnumCasing<'_>,
) -> proc_macro2::TokenStream {
    let rust_ident = item_enum.ident.to_string();
    let ts_generics = ts_generic_params(&item_enum.generics);
    let reader = tagged_enum_variant_reader(
        item_name,
        tag_name,
        &discriminated_variant_reader_pairs(item_enum, casing),
    );
    generate_discriminated_enum_ts_definition_method(
        docs,
        item_name,
        &rust_ident,
        &ts_generics,
        type_code,
        &reader,
    )
}

/// The externally-tagged `ts_definition()` method, its reader computed and appended in one call —
/// kept off `process_externally_tagged_enum` so that function stays under the line lint.
#[cfg(all(feature = "typescript", feature = "serde"))]
fn externally_tagged_ts_definition_method(
    docs: &str,
    item_name: &str,
    type_code: &str,
    item_enum: &syn::ItemEnum,
    casing: EnumCasing<'_>,
) -> proc_macro2::TokenStream {
    let rust_ident = item_enum.ident.to_string();
    let ts_generics = ts_generic_params(&item_enum.generics);
    let reader = externally_tagged_enum_variant_reader(
        item_name,
        &discriminated_variant_reader_pairs(item_enum, casing),
    );
    generate_discriminated_enum_ts_definition_method(
        docs,
        item_name,
        &rust_ident,
        &ts_generics,
        type_code,
        &reader,
    )
}

/// Processes a plain enum (simple string enum in TypeScript) and generates its definitions.
fn process_plain_enum(
    mut item_enum: syn::ItemEnum,
    name: &syn::Ident,
    rename_all: Option<&str>,
    item_name: &str,
    args: &ModelSchemaArgs,
) -> TokenStream {
    // Compute the schema module name and register the enum so other types can find it.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    // A plain enum is written as a `z.enum` of its variant names, which takes no string check.
    let (_, module_ident) = enum_module_idents(
        name,
        item_name,
        AliasKind::EnumMembers,
        Surface::enumerated(),
    );
    #[cfg(feature = "zod")]
    let rust_ident = name.to_string();

    #[cfg(any(feature = "typescript", feature = "zod"))]
    let docs_vec = get_enum_docs(&item_enum);

    let (enum_options, enum_variant_docs) = collect_plain_enum_options(&mut item_enum, rename_all);
    #[cfg(not(feature = "typescript"))]
    let _: &_ = &&enum_variant_docs;

    #[cfg(feature = "typescript")]
    let type_code = build_plain_enum_type_code(&enum_options, &enum_variant_docs);

    #[cfg(feature = "zod")]
    let schema_code = enum_options
        .iter()
        .map(|v| ts_quoted(v))
        .collect::<Vec<_>>()
        .join(", ");

    let enumerated: Vec<proc_macro2::TokenStream> =
        enum_options.iter().map(|v| quote! { #v }).collect();

    #[cfg(any(feature = "typescript", feature = "zod"))]
    let docs_and_description = build_item_docs_and_description(docs_vec.as_deref(), item_name);

    #[cfg(feature = "jsonschema")]
    // No slot to fill: Rust refuses an all-unit enum that leaves a type parameter unused, so a
    // plain enum reaches here declaring none — whatever consts and lifetimes it also binds.
    let json_schema_method = generate_plain_enum_json_schema_method(&enumerated, item_name, &[]);

    #[cfg(feature = "typescript")]
    let ts_definition_method = plain_enum_ts_definition_method(
        &docs_and_description.0,
        item_name,
        &type_code,
        &item_enum,
        rename_all,
    );

    // Schema module emits zod_schema without examples; example injection happens in the delegating
    // method on the type to avoid `super::` resolution issues.
    #[cfg(feature = "zod")]
    let zod_schema_method = generate_plain_enum_zod_schema_method(
        item_name,
        &rust_ident,
        &schema_code,
        &docs_and_description.1,
        args,
    );

    #[cfg(not(any(feature = "typescript", feature = "zod")))]
    let _: &_ = &item_name;

    // schema_example must be directly on the type (not in the module) because the example code
    // uses type names that may not be accessible from the nested module.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let schema_example_method =
        enum_schema_example_method(&item_enum.attrs, name, &item_enum.generics, args);

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let schema_impl_items: Vec<proc_macro2::TokenStream> = vec![
        #[cfg(feature = "jsonschema")]
        json_schema_method,
        #[cfg(feature = "typescript")]
        ts_definition_method,
        #[cfg(feature = "zod")]
        zod_schema_method,
    ];

    // A plain enum holds no member, so the checks it runs are the ones written on it.
    let item_checks = checks::ItemChecks::of(&args.check, variant_members(&item_enum));

    // Build delegating impl items; the plain-enum delegates match the branded ones, with the
    // `schema_example()` method chained on after them when an example exists, and `validate()`
    // where the enum names a check.
    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let delegate_impl_items: Vec<proc_macro2::TokenStream> =
        build_branded_delegate_items(&module_ident, schema_example_method.is_some())
            .into_iter()
            .chain(schema_example_method)
            .chain(validate_method(&[], Some(&module_ident), &item_checks))
            .collect();

    let decode_with = enum_decode_with(&item_enum, args);

    // A plain enum publishes `validate()` for the checks written on it, and for nothing else.
    let mut read = item_enum.clone();
    let judged = judged_read::of_enum(&mut read, !args.check.is_empty());

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    {
        plain_enum_output(
            &read,
            name,
            &module_ident,
            &schema_impl_items,
            &delegate_impl_items,
            &enumerated,
            &PlainEnumAdded {
                decode_with: &decode_with,
                judged: &judged,
            },
        )
    }

    #[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
    {
        let validate_impl =
            checks_validate_impl(&item_enum.ident, &item_enum.generics, &[], &item_checks);
        plain_enum_output(
            &read,
            name,
            &enumerated,
            &PlainEnumAdded {
                decode_with: &decode_with,
                judged: &judged,
            },
            &validate_impl,
        )
    }
}

/// A plain enum with its schema module, its delegates, `enum_members()`, and what `decode_with`
/// adds.
#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
fn plain_enum_output(
    item_enum: &syn::ItemEnum,
    name: &syn::Ident,
    module_ident: &Ident,
    schema_impl_items: &[proc_macro2::TokenStream],
    delegate_impl_items: &[proc_macro2::TokenStream],
    enum_values: &[proc_macro2::TokenStream],
    added: &PlainEnumAdded<'_>,
) -> TokenStream {
    let (decode_with, judged) = (added.decode_with, added.judged);
    // A plain enum publishes `enum_members()` from an `impl` of its own rather than through
    // `assemble_schema_output`, so it repeats the declaration's parameters itself.
    let (impl_generics, type_generics, where_clause) = item_enum.generics.split_for_impl();
    let module_items = schema_module_items(
        quote! {
            #[non_exhaustive]
            pub struct Schema;

            impl Schema {
                #(#schema_impl_items)*
            }
        },
        &decode_with.schema_module,
    );
    let decode_with_impl = &decode_with.type_impl;
    let output = quote! {
        #item_enum

        pub mod #module_ident {
            use super::*;

            #module_items
        }

        impl #impl_generics #name #type_generics #where_clause {
            #(#delegate_impl_items)*

            pub fn enum_members() -> ::std::vec::Vec<::std::string::String> {
                [
                    #(#enum_values),*
                ].iter().map(|v| v.to_string()).collect::<::std::vec::Vec<_>>()
            }
        }

        #judged
        #decode_with_impl
    };
    log::trace!("{output}");
    output
}

/// [`plain_enum_output`] where no schema surface is on: the item, `enum_members()`, the
/// `validate()` its checks earn it, and what `decode_with` adds.
#[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
fn plain_enum_output(
    item_enum: &syn::ItemEnum,
    name: &syn::Ident,
    enum_values: &[proc_macro2::TokenStream],
    added: &PlainEnumAdded<'_>,
    validate_impl: &proc_macro2::TokenStream,
) -> TokenStream {
    let (decode_with, judged) = (added.decode_with, added.judged);
    let (impl_generics, type_generics, where_clause) = item_enum.generics.split_for_impl();
    let decode_with_module = &decode_with.schema_module;
    let decode_with_impl = &decode_with.type_impl;
    let output = quote! {
        #item_enum

        impl #impl_generics #name #type_generics #where_clause {
            pub fn enum_members() -> ::std::vec::Vec<::std::string::String> {
                [
                    #(#enum_values),*
                ].iter().map(|v| v.to_string()).collect::<::std::vec::Vec<_>>()
            }
        }

        #validate_impl
        #judged
        #decode_with_module
        #decode_with_impl
    };
    log::trace!("{output}");
    output
}

/// The kind a variant publishes, which is not always the kind it declares.
fn variant_wire_kind(variant: &syn::Variant) -> VariantKind {
    if lone_slot_off_wire(variant) {
        VariantKind::Unit
    } else {
        classify_variant(variant)
    }
}

/// Whether the variant declares exactly one slot and takes that slot off the wire in both
/// directions — the collapse [`variant_wire_kind`] publishes a unit for. Held apart from the kind
/// it produces because refusals need the *declared* shape back, not the collapsed one.
fn lone_slot_off_wire(variant: &syn::Variant) -> bool {
    // `classify_variant` names `TupleSingle` for a variant of exactly one unnamed field, so the
    // first field is the lone slot whose omission decides this.
    matches!(classify_variant(variant), VariantKind::TupleSingle)
        && variant
            .fields
            .iter()
            .next()
            .is_some_and(|field| parse_serde_key_omission(&field.attrs).absent_from_wire())
}

/// Rejects a tuple-variant slot whose serde attributes drop it out of one of serde's two directions
/// and not the other.
fn check_variant_slot_wire_is_readable(
    field: &Field,
    index: usize,
    variant_name: &str,
    type_name: &str,
    omission: SerdeKeyOmission,
) -> Result<(), syn::Error> {
    if field.ident.is_some() || !omission.drops_one_direction_only() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: slot {index} of variant `{variant_name}` of enum `{type_name}` carries \
             a serde attribute that drops it from only one of serde's two directions, so what \
             serde writes for the variant is not what serde reads for it — one of them carries the \
             slot and the other does not. A slot is written by its place rather than under a key, \
             so there is no optional spelling to describe both and no schema can be written for \
             the pair. Use #[serde(skip)] to take the slot off the wire in both directions, or \
             drop the attribute so the slot is written and read in its place."
        ),
    ))
}

/// The two name overrides a variant declares: its own `#[serde(rename)]`, and the rule its own
/// fields are read against — its own `rename_all` where it writes one, the container's
/// `rename_all_fields` otherwise. serde treats a struct variant as the container for its fields, so
/// the enum's container-level `rename_all` never reaches them — that one renames variant names only.
#[cfg(feature = "serde")]
fn variant_serde_names(
    attrs: &[syn::Attribute],
    rename_all_fields: Option<&str>,
) -> (Option<String>, Option<String>) {
    (
        parse_serde_field_attributes(attrs).rename,
        parse_serde_type_attributes(attrs)
            .rename_all
            .or_else(|| rename_all_fields.map(ToOwned::to_owned)),
    )
}

/// No declaration is read where nothing parses serde's attributes.
#[cfg(not(feature = "serde"))]
const fn variant_serde_names(
    _attrs: &[syn::Attribute],
    _rename_all_fields: Option<&str>,
) -> (Option<String>, Option<String>) {
    (None, None)
}

/// Whether a field's members join the object being written rather than sitting under a key of their
/// own.
#[cfg(feature = "serde")]
fn is_flattened_field(field: &syn::Field) -> bool {
    parse_serde_field_attributes(&field.attrs).flatten
}

/// No field flattens where nothing parses serde's attributes.
#[cfg(not(feature = "serde"))]
const fn is_flattened_field(_field: &syn::Field) -> bool {
    false
}

/// The refusals of what serde writes or reads that a variant's member earns: a slot dropped in one
/// direction alone, and a `#[serde(flatten)]` member's. None where serde neither writes nor reads
/// the variant.
fn variant_member_wire_errors(
    on_the_wire: bool,
    field: &Field,
    index: usize,
    variant_name: &str,
    enum_type_name: &str,
) -> Vec<proc_macro2::TokenStream> {
    if !on_the_wire {
        return Vec::new();
    }
    let omission = parse_serde_key_omission(&field.attrs);
    let slot =
        check_variant_slot_wire_is_readable(field, index, variant_name, enum_type_name, omission)
            .err()
            .map(|rejection| rejection.to_compile_error());
    let flattened = if is_flattened_field(field) {
        variant_flatten_guard_errors(field, enum_type_name)
    } else {
        Vec::new()
    };
    slot.into_iter().chain(flattened).collect()
}

/// The `compile_error!` tokens a variant's `#[serde(flatten)]` field is refused with, which are the
/// two the struct-level split already reads a flattened field against: a variant's own position
/// multiplies the key sets its sources write exactly as a struct's does.
fn variant_flatten_guard_errors(
    field: &syn::Field,
    enum_type_name: &str,
) -> Vec<proc_macro2::TokenStream> {
    #[cfg(feature = "serde")]
    let refused = [
        flattened_field_guard_error(field, enum_type_name),
        flatten_edge_guard_error(field, enum_type_name),
    ];
    #[cfg(not(feature = "serde"))]
    let refused: [Option<proc_macro2::TokenStream>; 0] = {
        let _: (&_, &_) = (&field, &enum_type_name);
        []
    };

    refused.into_iter().flatten().collect()
}

/// Where the default serde reads for the absent key of a member, by variant and member, sits among
/// those of the variants of `item_enum`, read off its attributes as written.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
fn variant_serde_defaults(item_enum: &syn::ItemEnum) -> Box<VariantDefaultAt> {
    let defaults = SerdeDefaults::of_enum(item_enum);
    Box::new(move |variant, field| defaults.index(Some(variant), field))
}

/// No member is read as its default where no surface writes one.
#[cfg(not(any(feature = "zod", feature = "jsonschema")))]
fn variant_serde_defaults(_item_enum: &syn::ItemEnum) -> Box<VariantDefaultAt> {
    Box::new(|_, _| None)
}

/// Hands `field_def` where the default serde reads for its absent key sits among its type's.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
const fn hand_serde_default(field_def: &mut FieldDef, at: Option<usize>) {
    field_def.serde_default = at;
}

/// Nothing holds a default where no surface writes one.
#[cfg(not(any(feature = "zod", feature = "jsonschema")))]
const fn hand_serde_default(_field_def: &mut FieldDef, _at: Option<usize>) {}

/// Processes each variant of a discriminated enum, returning per-variant field defs, doc strings,
/// and variant kinds in declaration order, for each variant serde writes or reads, plus the
/// collected serde validation functions and the `validate()` arms those functions are run from.
/// A variant serde neither writes nor reads is walked for `validate()` alone, and no guard of the
/// wire reads it.
fn collect_discriminated_variants(
    item_enum: &mut syn::ItemEnum,
    casing: EnumCasing<'_>,
    enum_module_name_opt: Option<&str>,
    args: &ModelSchemaArgs,
) -> DiscriminatedVariantData {
    let type_parameters = type_parameters_in_scope(&item_enum.generics);
    let mut variants: Vec<DiscriminatedVariant> = Vec::new();
    let mut enum_validation_fns: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut guard_errors: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut per_variant_checks = Vec::new();
    let mut deferred_attrs: Vec<Vec<syn::Attribute>> = Vec::new();
    let enum_type_name = item_enum.ident.to_string();
    let enum_read_back = container_is_read_back(&item_enum.attrs);
    let steps_by_variant = steps_of_variants(item_enum, casing.variant_fields);
    // Read before the walk, as a struct's are.
    let default_at = variant_serde_defaults(item_enum);

    for (item, steps) in item_enum.variants.iter_mut().zip(&steps_by_variant) {
        let on_the_wire = variant_on_the_wire(item);
        let (field_rename, variant_rename_all) =
            variant_serde_names(&item.attrs, casing.variant_fields);
        let variant_ident = item.ident.to_string();
        let final_name =
            get_final_variant_name(&variant_ident, field_rename.as_deref(), casing.variants);
        // The Rust shape the `validate()` arm is matched by, beside the wire shape the surfaces
        // describe: a variant that publishes no slot is still declared holding one.
        let declared_kind = classify_variant(item);
        let wire_kind = variant_wire_kind(item);
        // A struct variant is where serde accepts a container-level `default`, so it is the
        // container its own fields' omissions are read against.
        let variant_defaulted = has_serde_default(&item.attrs);

        let member_ctx = FieldContext {
            container_defaulted: variant_defaulted,
            container_read_back: enum_read_back,
            enum_args: Some(args),
            member_steps: steps,
            on_the_wire,
            rename_all: variant_rename_all.as_deref(),
            schema_module_name: enum_module_name_opt,
            type_name: &enum_type_name,
            type_parameters: &type_parameters,
            variant_ident: Some(&variant_ident),
        };
        let mut field_defs: Vec<FieldDef> = Vec::new();
        let mut flattened_fields: Vec<FieldDef> = Vec::new();
        let mut bound: Vec<BoundMember> = Vec::new();
        let mut checks: Vec<proc_macro2::TokenStream> = Vec::new();
        let total_fields = item.fields.len();
        for (index, field) in item.fields.iter_mut().enumerate() {
            let omission = parse_serde_key_omission(&field.attrs);
            let positional = field.ident.is_none();
            // Read before the field is processed, which strips the attributes the declaration
            // carried.
            let is_flatten = is_flattened_field(field);
            guard_errors.extend(variant_member_wire_errors(
                on_the_wire,
                field,
                index,
                &variant_ident,
                &enum_type_name,
            ));

            let serde_default = default_at(&variant_ident, &field_ident_string(field));
            let (mut f_def, validation_fn, validate_body, field_guard_errors) =
                process_field(&member_ctx, field, index, &mut deferred_attrs);
            guard_errors.extend(field_guard_errors);
            hand_serde_default(&mut f_def, serde_default);

            if is_flatten {
                let _: (&_, &_) = (&validation_fn, &validate_body);
                // Rebuilt, as a struct's flattened field is — see there.
                flattened_member_walk(&member_ctx, field, &f_def, index, &mut bound, &mut checks);
                flattened_fields.push(f_def);
                continue;
            }

            enum_validation_fns.extend(validation_fn);
            if let Some(body) = validate_body {
                bound.push(bound_member(field, index));
                checks.push(body);
            }
            if positional && omission.absent_from_wire() {
                continue;
            }
            push_described_field(&mut field_defs, f_def);
        }
        per_variant_checks.push((
            variant_check_pattern(&item.ident, &declared_kind, total_fields, &bound),
            checks,
        ));

        if on_the_wire {
            let discriminator_docs =
                build_jsdoc_body(get_variant_docs(item).as_deref(), &final_name);
            variants.push(DiscriminatedVariant {
                discriminator_value: final_name,
                docs: discriminator_docs,
                field_defs,
                flattened_fields,
                kind: wire_kind,
            });
        }
    }

    if guard_errors.is_empty() {
        apply_deferred_field_attrs(
            item_enum
                .variants
                .iter_mut()
                .flat_map(|item| item.fields.iter_mut()),
            deferred_attrs,
        );
    }

    (
        variants,
        enum_validation_fns,
        guard_errors,
        build_member_check_arms(per_variant_checks),
    )
}

/// Renders the TypeScript type fragments, Zod schema fragments, and JSON-schema fragments for each
/// variant of a discriminated enum.
fn render_discriminated_variants(
    tag_name: &str,
    content_name: &str,
    item_name: &str,
    variants: &[DiscriminatedVariant],
) -> RenderedVariants {
    let mut type_code_items = Vec::new();
    let mut schema_code_items = Vec::new();
    let mut json_schema_variants: Vec<proc_macro2::TokenStream> = Vec::new();

    for variant in variants {
        let (variant_type_code, variant_schema_code, json_schema_variant) =
            generate_variant_code(tag_name, content_name, variant, item_name);
        type_code_items.push(variant_type_code);
        schema_code_items.push(variant_schema_code);
        json_schema_variants.push(json_schema_variant);
    }

    (type_code_items, schema_code_items, json_schema_variants)
}

/// Builds the JSON-schema `oneOf` object expression for a discriminated enum from its per-variant
/// JSON-schema fragments.
#[cfg(feature = "jsonschema")]
fn discriminated_main_schema_code(
    json_schema_variants: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    quote! {
        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert("type".to_string(), serde_json::Value::String("object".to_string()));
        schema_obj.insert("oneOf".to_string(), {
            let result: ::std::vec::Vec<serde_json::Value> = vec![
                #(#json_schema_variants), *
            ];

            serde_json::Value::Array(result)
        });

        serde_json::Value::Object(schema_obj)
    }
}

/// Refuses every variant serde writes or reads of an adjacently tagged enum whose declared lone
/// slot is off the wire in both directions, one error per offender so the expansion names them
/// all.
#[cfg(feature = "serde")]
fn adjacent_collapsed_slot_guard_errors(
    item_enum: &syn::ItemEnum,
    tag_name: &str,
    content_name: &str,
) -> Vec<proc_macro2::TokenStream> {
    let type_name = &item_enum.ident;
    item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant) && lone_slot_off_wire(variant))
        .map(|variant| {
            let variant_name = &variant.ident;
            syn::Error::new_spanned(
                variant,
                format!(
                    "model_schema: variant `{variant_name}` of enum `{type_name}` declares one \
                     slot and takes it off the wire in both directions, so serde writes it as \
                     `{{\"{tag_name}\":\"{variant_name}\"}}` — the payload a unit variant writes — \
                     and then refuses to read that same payload back, asking for the \
                     `{content_name}` key it never wrote. Only \
                     `{{\"{tag_name}\":\"{variant_name}\",\"{content_name}\":null}}` reads, so what \
                     serde writes for the variant and what serde reads for it have no payload in \
                     common and no schema can be written for the pair. Declare `{variant_name}` as \
                     a unit variant: it writes the identical \
                     `{{\"{tag_name}\":\"{variant_name}\"}}` and reads it back. Or drop the \
                     attribute so the slot is written and read in its place."
                ),
            )
            .to_compile_error()
        })
        .collect()
}

/// Builds the Zod `z.discriminatedUnion` expression for a discriminated enum from its per-variant
/// member schemas, beside [`discriminated_main_schema_code`] which answers the same for JSON.
#[cfg(feature = "zod")]
fn discriminated_zod_schema_code(tag_name: &str, members: &[String]) -> String {
    format!(
        "z.discriminatedUnion({}, [{}])",
        ts_quoted(tag_name),
        members
            .iter()
            .map(|member| format!("z.strictObject({member})"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Processes a discriminated enum (tagged union in TypeScript) and generates its definitions.
fn process_discriminated_enum(
    mut item_enum: syn::ItemEnum,
    name: &syn::Ident,
    tag_name: &str,
    content_name: &str,
    casing: EnumCasing<'_>,
    item_name: &str,
    args: &ModelSchemaArgs,
) -> TokenStream {
    // Compute the schema module name and register the enum so other types can find it.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    // Every other enum shape is written as a union of what its variants render as.
    let (module_name, module_ident) =
        enum_module_idents(name, item_name, AliasKind::NoEnumMembers, Surface::union());

    #[cfg(feature = "typescript")]
    let docs_vec = get_enum_docs(&item_enum);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let enum_module_name_opt = Some(module_name.as_str());
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let enum_module_name_opt = None;

    // Read before the walk, which rewrites the very attributes the collapse is read off, and
    // returned before it so no held-back attribute is written onto an item the guard refuses.
    #[cfg(feature = "serde")]
    if let Some(output) = refused_item_output(
        &Item::Enum(item_enum.clone()),
        &adjacent_collapsed_slot_guard_errors(&item_enum, tag_name, content_name),
    ) {
        return output;
    }

    // Bind both result tuples whole so feature-gated field access marks them used (no per-element
    // guards): `variants` = (variants, validation_fns, guard_errors); `rendered` = (ts, zod, json).
    // Read before the walk, which takes each member's own attribute off it.
    let item_checks = checks::ItemChecks::of(&args.check, variant_members(&item_enum));
    let variants =
        collect_discriminated_variants(&mut item_enum, casing, enum_module_name_opt, args);
    if let Some(output) = guard_failure_output(&item_enum, Some(&item_enum.ident), &variants.2) {
        return output;
    }
    let rendered = render_discriminated_variants(tag_name, content_name, item_name, &variants.0);
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let _: &_ = &(name, &rendered);
    let decode_with = enum_decode_with(&item_enum, args);

    #[cfg(feature = "jsonschema")]
    let main_schema_code = discriminated_main_schema_code(&rendered.2);

    #[cfg(feature = "typescript")]
    let type_code = rendered.0.join(" | ");

    #[cfg(feature = "zod")]
    let schema_code = discriminated_zod_schema_code(tag_name, &rendered.1);

    #[cfg(feature = "typescript")]
    let docs = build_jsdoc_body(docs_vec.as_deref(), item_name);

    #[cfg(feature = "jsonschema")]
    let json_schema_method =
        enum_json_schema_methods(&main_schema_code, item_name, &item_enum.generics, args);

    #[cfg(feature = "typescript")]
    let ts_definition_method =
        tagged_ts_definition_method(&docs, item_name, &type_code, tag_name, &item_enum, casing);

    // Schema module emits zod_schema without examples; example injection happens in the delegating
    // method on the type to avoid `super::` resolution issues.
    #[cfg(feature = "zod")]
    let zod_schema_method = generate_discriminated_enum_zod_schema_method(
        item_name,
        &name.to_string(),
        &type_parameters_in_scope(&item_enum.generics),
        args,
        &schema_code,
    );

    #[cfg(not(any(feature = "typescript", feature = "zod")))]
    let _: &_ = &item_name;

    // schema_example must be directly on the type (not in the module) because the example code
    // uses type names that may not be accessible from the nested module.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let schema_example_method =
        enum_schema_example_method(&item_enum.attrs, name, &item_enum.generics, args);

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let schema_impl_items: Vec<proc_macro2::TokenStream> = vec![
        #[cfg(feature = "jsonschema")]
        json_schema_method,
        #[cfg(feature = "typescript")]
        ts_definition_method,
        #[cfg(feature = "zod")]
        zod_schema_method,
    ];

    // Build delegating impl items; the discriminated-enum delegates match the struct ones (the
    // `zod_schema` example injection uses the same `.meta()`-before-`;` form).
    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let delegate_impl_items = build_struct_delegate_items(
        &module_ident,
        item_name,
        &name.to_string(),
        &type_parameters_in_scope(&item_enum.generics),
        schema_example_method.as_ref(),
    );

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    {
        enum_output_with_decode(
            &SchemaOutputParts {
                delegate_impl_items: &delegate_impl_items,
                generics: &item_enum.generics,
                item: &item_enum,
                module_ident: &module_ident,
                name,
                schema_impl_items: &schema_impl_items,
                validate_method: &build_enum_validate_method(
                    &variants.3,
                    &module_ident,
                    &item_checks,
                ),
                validation_fns: &variants.1,
            },
            &decode_with,
        )
    }

    #[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
    {
        enum_output_with_decode(&item_enum, &decode_with, &variants.3, &item_checks)
    }
}

/// The object schema a struct variant's fields sit in when they are written under a key of their
/// own rather than beside a discriminator, with the members of every `#[serde(flatten)]` source the
/// variant carries merged beside them. Used by both the externally tagged form and the adjacently
/// tagged form's own struct variant.
#[cfg(feature = "jsonschema")]
fn named_content_json_value(
    json_fields: &[proc_macro2::TokenStream],
    flattened_fields: &[FieldDef],
    self_type_name: &str,
    variant_name: &str,
) -> proc_macro2::TokenStream {
    let object = quote! {
        {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            let mut properties = serde_json::Map::new();
            let mut required: ::std::vec::Vec<serde_json::Value> = ::std::vec::Vec::new();
            #(#json_fields)*
            schema_obj.insert("properties".to_string(), serde_json::Value::Object(properties));
            schema_obj.insert("required".to_string(), serde_json::Value::Array(required));
            schema_obj.insert("additionalProperties".to_string(), serde_json::Value::Bool(false));
            schema_obj
        }
    };
    variant_merged_json_value(&object, flattened_fields, self_type_name, variant_name)
}

/// A variant's own object with the members of every `#[serde(flatten)]` source it carries merged
/// beside them, and the object exactly as it stands where the variant flattens nothing.
#[cfg(feature = "jsonschema")]
fn variant_merged_json_value(
    base: &proc_macro2::TokenStream,
    flattened_fields: &[FieldDef],
    self_type_name: &str,
    variant_name: &str,
) -> proc_macro2::TokenStream {
    if flattened_fields.is_empty() {
        return quote! { serde_json::Value::Object(#base) };
    }
    merged_object_value(
        base,
        &flatten_merged_sources(flattened_fields),
        &MergeDiagnostic {
            beside_tag: None,
            cycle_remedy: "write the field as a named member so the cycle defers through a reference",
            edge: &format!("`#[serde(flatten)]` in variant `{variant_name}` of"),
            non_object_remedy: "write the field as a named member so the value gets a key of its own",
            subject: self_type_name,
        },
    )
}

/// The diagnostic a variant whose content has no rendering produces, in the value position the
/// content itself would have occupied.
#[cfg(all(feature = "serde", feature = "jsonschema"))]
fn external_content_rejection_value(
    discriminator_value: &str,
    rejection: &MapMemberRejection,
) -> proc_macro2::TokenStream {
    let message = prefixed_guard_message(&map_member_rejection_message(
        &format!("variant `{discriminator_value}`"),
        rejection,
    ));
    syn::Error::new(rejection.span(), message).to_compile_error()
}

/// Renders what one variant of an externally tagged enum writes under its key.
#[cfg(feature = "serde")]
fn render_external_content(
    variant: &DiscriminatedVariant,
    self_type_name: &str,
) -> (String, String, proc_macro2::TokenStream) {
    let field_defs = &variant.field_defs;
    let discriminator_value = &variant.discriminator_value;
    #[cfg(not(feature = "jsonschema"))]
    let _: &str = discriminator_value;

    match &variant.kind {
        VariantKind::Unit => (String::new(), String::new(), quote! {}),
        VariantKind::TupleSingle => {
            // `classify_variant` names this kind only for a variant of exactly one unnamed field.
            let fld = &field_defs[0];

            #[cfg(feature = "zod")]
            let zod = fld.zod_slot_type();
            #[cfg(not(feature = "zod"))]
            let zod = String::new();

            #[cfg(feature = "jsonschema")]
            let json = bounded_slot_json_schema(fld).unwrap_or_else(|rejection| {
                external_content_rejection_value(discriminator_value, &rejection)
            });
            #[cfg(not(feature = "jsonschema"))]
            let json = quote! {};

            (fld.typescript_slot_typename(), zod, json)
        }
        VariantKind::TupleMultiple => {
            let ts = format!(
                "[{}]",
                field_defs
                    .iter()
                    .map(super::field_type::FieldDef::typescript_slot_typename)
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            #[cfg(feature = "zod")]
            let zod = format!(
                "z.tuple([{}])",
                field_defs
                    .iter()
                    .map(super::field_type::FieldDef::zod_slot_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            #[cfg(not(feature = "zod"))]
            let zod = String::new();

            #[cfg(feature = "jsonschema")]
            let json = bounded_slots_json_schema(field_defs).unwrap_or_else(|rejection| {
                external_content_rejection_value(discriminator_value, &rejection)
            });
            #[cfg(not(feature = "jsonschema"))]
            let json = quote! {};

            (ts, zod, json)
        }
        VariantKind::Named => {
            let mut parts = VariantParts {
                json_fields: Vec::new(),
                schema_code: String::new(),
                type_code: String::new(),
            };
            write_named_variant_fields(field_defs, None, self_type_name, &mut parts);

            #[cfg(feature = "zod")]
            let zod = variant_flatten_zod(
                &format!("z.strictObject({{\n{}}})", parts.schema_code),
                &variant.flattened_fields,
            );
            #[cfg(not(feature = "zod"))]
            let zod = String::new();

            #[cfg(feature = "jsonschema")]
            let json = named_content_json_value(
                &parts.json_fields,
                &variant.flattened_fields,
                self_type_name,
                discriminator_value,
            );
            #[cfg(not(feature = "jsonschema"))]
            let json = quote! {};

            (
                format!(
                    "{{\n{}}}{}",
                    parts.type_code,
                    variant_flatten_typescript(
                        &member_names(field_defs),
                        &variant.flattened_fields
                    )
                ),
                zod,
                json,
            )
        }
    }
}

/// Renders one variant of an externally tagged enum as a union member.
#[cfg(feature = "serde")]
fn render_external_variant(
    variant: &DiscriminatedVariant,
    self_type_name: &str,
) -> (String, String, proc_macro2::TokenStream) {
    let key = &variant.discriminator_value;
    let quoted = ts_quoted(key);
    let docs = &variant.docs;

    if matches!(variant.kind, VariantKind::Unit) {
        #[cfg(feature = "jsonschema")]
        let json = quote! { serde_json::json!({ "type": "string", "const": #key }) };
        #[cfg(not(feature = "jsonschema"))]
        let json = quote! {};

        return (
            format!("{}\n  {quoted}", member_jsdoc_block(docs)),
            format!("z.literal({quoted})"),
            json,
        );
    }

    let (content_ts, content_zod, content_json) = render_external_content(variant, self_type_name);

    #[cfg(feature = "zod")]
    let zod = {
        // A `Named` variant defers each recursive field inside the object it renders, so only the
        // kinds with no inner object need the key itself to carry the deferral.
        let defer_key = !matches!(variant.kind, VariantKind::Named)
            && variant
                .field_defs
                .iter()
                .any(|fld| fld.contains_type_reference(self_type_name));
        if defer_key {
            format!("z.strictObject({{\n  get {quoted}() {{ return {content_zod}; }},\n}})")
        } else {
            format!("z.strictObject({{\n  {quoted}: {content_zod},\n}})")
        }
    };
    #[cfg(not(feature = "zod"))]
    let zod = {
        let _: &str = &content_zod;
        String::new()
    };

    // Built key by key rather than through `serde_json::json!`: a struct variant's content is a
    // block of statements, which the macro's value position cannot parse.
    #[cfg(feature = "jsonschema")]
    let json = quote! {
        {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert(
                "type".to_string(),
                serde_json::Value::String("object".to_string()),
            );
            let mut properties = serde_json::Map::new();
            properties.insert(#key.to_string(), #content_json);
            schema_obj.insert(
                "properties".to_string(),
                serde_json::Value::Object(properties),
            );
            schema_obj.insert(
                "required".to_string(),
                serde_json::Value::Array(vec![serde_json::Value::String(#key.to_string())]),
            );
            schema_obj.insert(
                "additionalProperties".to_string(),
                serde_json::Value::Bool(false),
            );
            serde_json::Value::Object(schema_obj)
        }
    };
    #[cfg(not(feature = "jsonschema"))]
    let json = {
        let _: &_ = &content_json;
        quote! {}
    };

    (
        format!(
            "{{\n{}\n  {quoted}: {content_ts};\n}}",
            member_jsdoc_block(docs)
        ),
        zod,
        json,
    )
}

/// Records what an object flattening an externally tagged enum joins for each of its variants, in
/// the order the union writes them. Each Zod operand carries the enum's own `checks`, handed the
/// enum's value: a unit variant's bare name.
#[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
fn record_external_flatten_operands(
    rust_ident: &str,
    variants: &[DiscriminatedVariant],
    members: &[(String, String, proc_macro2::TokenStream)],
    checks: &[FunctionName],
) {
    #[cfg(not(feature = "zod"))]
    let _: &[FunctionName] = checks;
    #[cfg(feature = "typescript")]
    let exclusions = sibling_key_exclusions(
        &variants
            .iter()
            .map(|variant| Some(vec![variant.discriminator_value.clone()]))
            .collect::<Vec<_>>(),
    );
    let operands: Vec<FlattenVariant> = variants
        .iter()
        .zip(members)
        .enumerate()
        .map(|(index, (variant, member))| {
            let unit = matches!(variant.kind, VariantKind::Unit);
            let key = &variant.discriminator_value;
            #[cfg(feature = "typescript")]
            let typescript = {
                let tagged = if unit {
                    format!(
                        "{{\n{}\n  {}: null;\n}}",
                        member_jsdoc_block(&variant.docs),
                        ts_quoted(key)
                    )
                } else {
                    member.0.clone()
                };
                close_tagged_flatten_member(&tagged, &exclusions[index])
            };
            #[cfg(not(feature = "typescript"))]
            let _: usize = index;
            #[cfg(feature = "zod")]
            let zod = if unit {
                zod_check_wrap_handing(
                    format!("z.strictObject({{\n  {}: z.null(),\n}})", ts_quoted(key)),
                    checks,
                    &ts_quoted(key),
                )
            } else {
                zod_check_wrap(member.1.clone(), checks)
            };
            FlattenVariant {
                #[cfg(feature = "typescript")]
                typescript,
                #[cfg(feature = "zod")]
                unit: unit.then(|| key.clone()),
                #[cfg(feature = "zod")]
                zod,
            }
        })
        .collect();
    record_flatten_variants(rust_ident, &operands);
}

/// Which of a flattened choice's keys each member has to say it does not carry, one list per member
/// and in the order the union writes them.
#[cfg(all(feature = "serde", feature = "typescript"))]
fn sibling_key_exclusions(member_keys: &[Option<Vec<String>>]) -> Vec<Vec<String>> {
    member_keys
        .iter()
        .map(|member| {
            let Some(own) = member.as_ref() else {
                return Vec::new();
            };
            let mut excluded: Vec<String> = Vec::new();
            for key in member_keys.iter().flatten().flatten() {
                if !own.contains(key) && !excluded.contains(key) {
                    excluded.push(key.clone());
                }
            }
            excluded
        })
        .collect()
}

/// One externally tagged variant's union member or flatten operand with its siblings' tags marked
/// absent, written in the key-per-line shape [`render_external_variant`] already wrote the variant
/// in. A member that is no object literal keeps its spelling — there is no key list to add a line
/// to.
#[cfg(all(feature = "serde", feature = "typescript"))]
fn close_tagged_flatten_member(member: &str, excluded: &[String]) -> String {
    if excluded.is_empty() {
        return member.to_owned();
    }
    let Some(keys) = member.strip_suffix("\n}") else {
        return member.to_owned();
    };
    let mut closed = keys.to_owned();
    for key in excluded {
        let _ = write!(closed, "\n  {}?: never;", ts_quoted(key));
    }
    closed.push_str("\n}");
    closed
}

/// Joins an externally tagged enum's rendered members into its three union surfaces: the
/// JSON-schema body, the TypeScript union, and the Zod union. Member order is the enum's
/// declaration order, as for the other two enum forms. Each TypeScript object member says it
/// carries no other object member's key, which serde and Zod refuse beside it.
#[cfg(feature = "serde")]
fn join_external_union(
    variants: &[DiscriminatedVariant],
    members: &[(String, String, proc_macro2::TokenStream)],
) -> (proc_macro2::TokenStream, String, String) {
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let _: &_ = &members;
    #[cfg(not(feature = "typescript"))]
    let _: &_ = &variants;

    #[cfg(feature = "jsonschema")]
    let main_schema_code = {
        let json_members = members.iter().map(|(_, _, json)| json);
        quote! {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert("oneOf".to_string(), {
                let result: ::std::vec::Vec<serde_json::Value> = vec![
                    #(#json_members), *
                ];

                serde_json::Value::Array(result)
            });

            serde_json::Value::Object(schema_obj)
        }
    };
    #[cfg(not(feature = "jsonschema"))]
    let main_schema_code = quote! {};

    #[cfg(feature = "typescript")]
    let type_code = {
        let exclusions = sibling_key_exclusions(
            &variants
                .iter()
                .map(|variant| {
                    (!matches!(variant.kind, VariantKind::Unit))
                        .then(|| vec![variant.discriminator_value.clone()])
                })
                .collect::<Vec<_>>(),
        );
        members
            .iter()
            .zip(&exclusions)
            .map(|((ts, _, _), excluded)| close_tagged_flatten_member(ts, excluded))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    #[cfg(not(feature = "typescript"))]
    let type_code = String::new();

    #[cfg(feature = "zod")]
    let schema_code = format!(
        "z.union([{}])",
        members
            .iter()
            .map(|(_, zod, _)| zod.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    #[cfg(not(feature = "zod"))]
    let schema_code = String::new();

    (main_schema_code, type_code, schema_code)
}

/// What each variant of the externally tagged enum `ident` renders as, recorded as well for an
/// object that flattens the enum: the flatten-edge spelling of a variant that holds a value is the
/// member's own, which only this expansion holds.
#[cfg(feature = "serde")]
fn external_members(
    ident: &syn::Ident,
    variants: &[DiscriminatedVariant],
    checks: &[FunctionName],
) -> Vec<(String, String, proc_macro2::TokenStream)> {
    let self_type_name = ident.to_string();
    let members: Vec<(String, String, proc_macro2::TokenStream)> = variants
        .iter()
        .map(|variant| render_external_variant(variant, &self_type_name))
        .collect();
    #[cfg(any(feature = "typescript", feature = "zod"))]
    record_external_flatten_operands(&self_type_name, variants, &members, checks);
    #[cfg(not(any(feature = "typescript", feature = "zod")))]
    let _: &[FunctionName] = checks;
    members
}

/// Processes an enum that carries no serde tagging attributes and generates its definitions.
#[cfg(feature = "serde")]
fn process_externally_tagged_enum(
    mut item_enum: syn::ItemEnum,
    name: &syn::Ident,
    casing: EnumCasing<'_>,
    item_name: &str,
    args: &ModelSchemaArgs,
) -> TokenStream {
    // Compute the schema module name and register the enum so other types can find it.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    // Every other enum shape is written as a union of what its variants render as.
    let (module_name, module_ident) = enum_module_idents(
        name,
        item_name,
        AliasKind::NoEnumMembers,
        Surface::externally_tagged(&item_enum.variants),
    );

    #[cfg(feature = "typescript")]
    let docs_vec = get_enum_docs(&item_enum);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let enum_module_name_opt = Some(module_name.as_str());
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let enum_module_name_opt = None;

    // Read before the walk, which takes each member's own attribute off it.
    let item_checks = checks::ItemChecks::of(&args.check, variant_members(&item_enum));
    let variants =
        collect_discriminated_variants(&mut item_enum, casing, enum_module_name_opt, args);
    if let Some(output) = guard_failure_output(&item_enum, Some(&item_enum.ident), &variants.2) {
        return output;
    }
    let decode_with = enum_decode_with(&item_enum, args);
    let members = external_members(&item_enum.ident, &variants.0, &args.check);

    let (main_schema_code, type_code, schema_code) = join_external_union(&variants.0, &members);
    #[cfg(not(feature = "jsonschema"))]
    let _: &_ = &main_schema_code;
    #[cfg(not(feature = "typescript"))]
    let _: &_ = &type_code;
    #[cfg(not(feature = "zod"))]
    let _: &_ = &schema_code;
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let _: &_ = &name;

    #[cfg(feature = "typescript")]
    let docs = build_jsdoc_body(docs_vec.as_deref(), item_name);

    #[cfg(feature = "jsonschema")]
    let json_schema_method =
        enum_json_schema_methods(&main_schema_code, item_name, &item_enum.generics, args);

    #[cfg(feature = "typescript")]
    let ts_definition_method =
        externally_tagged_ts_definition_method(&docs, item_name, &type_code, &item_enum, casing);

    #[cfg(feature = "zod")]
    let zod_schema_method = generate_discriminated_enum_zod_schema_method(
        item_name,
        &name.to_string(),
        &type_parameters_in_scope(&item_enum.generics),
        args,
        &schema_code,
    );

    #[cfg(not(any(feature = "typescript", feature = "zod")))]
    let _: &_ = &item_name;

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let schema_example_method =
        enum_schema_example_method(&item_enum.attrs, name, &item_enum.generics, args);

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let schema_impl_items: Vec<proc_macro2::TokenStream> = vec![
        #[cfg(feature = "jsonschema")]
        json_schema_method,
        #[cfg(feature = "typescript")]
        ts_definition_method,
        #[cfg(feature = "zod")]
        zod_schema_method,
    ];

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let delegate_impl_items = build_struct_delegate_items(
        &module_ident,
        item_name,
        &name.to_string(),
        &type_parameters_in_scope(&item_enum.generics),
        schema_example_method.as_ref(),
    );

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    {
        enum_output_with_decode(
            &SchemaOutputParts {
                delegate_impl_items: &delegate_impl_items,
                generics: &item_enum.generics,
                item: &item_enum,
                module_ident: &module_ident,
                name,
                schema_impl_items: &schema_impl_items,
                validate_method: &build_enum_validate_method(
                    &variants.3,
                    &module_ident,
                    &item_checks,
                ),
                validation_fns: &variants.1,
            },
            &decode_with,
        )
    }

    #[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
    {
        let _: &_ = &variants.1;
        enum_output_with_decode(&item_enum, &decode_with, &variants.3, &item_checks)
    }
}

/// How an internally tagged newtype variant's inner value fares beside the tag.
#[cfg(feature = "serde")]
fn tagged_content(inner: &FieldDef) -> TaggedContent {
    if inner.is_optional() {
        return TaggedContent::Refused("an optional");
    }
    if inner.array_depth > 0 {
        return TaggedContent::Refused("a sequence");
    }
    if inner.is_unit() {
        return TaggedContent::Unit;
    }
    match inner.field_type {
        FieldDefType::SiblingType(..) => TaggedContent::Flattened,
        FieldDefType::Boolean | FieldDefType::BooleanLiteral(_) => {
            TaggedContent::Refused("a boolean")
        }
        FieldDefType::F32 | FieldDefType::F64 => TaggedContent::Refused("a float"),
        FieldDefType::NumberLiteral(_) => TaggedContent::Refused("a number"),
        FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::Usize => TaggedContent::Refused("an integer"),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            TaggedContent::Refused("a string")
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => TaggedContent::Refused("a string"),
        FieldDefType::Tuple(_) => TaggedContent::Refused("a tuple"),
        FieldDefType::Map(..) => TaggedContent::Unnameable("a map"),
        FieldDefType::Range(..) => TaggedContent::Unnameable("a range"),
        FieldDefType::Result(..) => TaggedContent::Unnameable("a result"),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => TaggedContent::Unnameable("an ObjectId"),
        FieldDefType::TypeParam(_) => TaggedContent::Unnameable("a type parameter"),
        FieldDefType::Unknown => TaggedContent::Unnameable("a type the expansion cannot resolve"),
    }
}

/// The `compile_error!` tokens for every variant an internally tagged enum cannot carry. Collected
/// from the declaration, not the collected variants, so each diagnostic keeps the span of the
/// thing it rejects. A variant serde neither writes nor reads holds nothing beside the tag; a tuple
/// variant is refused all the same, as serde's derive refuses it.
#[cfg(feature = "serde")]
fn internally_tagged_guard_errors(
    item_enum: &syn::ItemEnum,
    tag_name: &str,
) -> Vec<proc_macro2::TokenStream> {
    let enum_name = &item_enum.ident;
    item_enum
        .variants
        .iter()
        .filter_map(|variant| {
            let syn::Fields::Unnamed(unnamed) = &variant.fields else {
                return None;
            };
            let variant_name = &variant.ident;
            if unnamed.unnamed.len() > 1 {
                return Some(
                    syn::Error::new_spanned(
                        variant,
                        format!(
                            "model_schema: variant `{variant_name}` of internally tagged enum \
                             `{enum_name}` is a tuple variant, which `#[serde(tag = \
                             \"{tag_name}\")]` cannot carry: the elements have no names to be \
                             written under beside the tag, and serde's own derive refuses the \
                             declaration outright — `#[serde(tag = \"...\")] cannot be used with \
                             tuple variants`. Name a `content` key so the elements get an array \
                             of their own, or write them as named fields."
                        ),
                    )
                    .to_compile_error(),
                );
            }
            // An empty tuple variant carries nothing: serde writes it as the tag alone, which is
            // what the unit arm already describes.
            let field = unnamed.unnamed.first()?;
            if parse_serde_key_omission(&variant.attrs).absent_from_wire()
                || parse_serde_key_omission(&field.attrs).absent_from_wire()
            {
                return None;
            }
            let inner = get_field_def("_inner", &field.ty, "");
            let content = tagged_content(&inner);
            if matches!(content, TaggedContent::Flattened)
                && let Some(refused) =
                    ranges::tagged_slot_map_error(item_enum, variant, field, tag_name)
            {
                return Some(refused);
            }
            let slot = TaggedSlot {
                inner: &inner,
                written: &field.ty,
            };
            let message = match content {
                TaggedContent::Flattened => match slot.plain_enum() {
                    Some(inner_name) => format!(
                        "model_schema: variant `{variant_name}` of internally tagged enum \
                         `{enum_name}` wraps `{inner_name}`, which serde does not write as an \
                         object: a plain enum writes its own variant name, so nothing of it joins \
                         the tag — serde writes that name as a key holding null, which a schema \
                         closed around the tag rejects. Name a `content` key so the value gets an \
                         object of its own, or wrap it in a struct whose fields can sit beside the \
                         tag. {FLATTENED_PLAIN_ENUM_SCOPE}"
                    ),
                    // serde's tag refuses an `Option` before it reads what the `Option` holds.
                    None => tagged_optional_message(variant_name, enum_name, tag_name, slot)
                        .or_else(|| {
                            tagged_non_object_message(variant_name, enum_name, tag_name, slot)
                        })?,
                },
                TaggedContent::Refused(shape) => format!(
                    "model_schema: variant `{variant_name}` of internally tagged enum \
                     `{enum_name}` wraps {shape}, which cannot sit beside the tag: `#[serde(tag \
                     = \"{tag_name}\")]` with no `content` writes the variant's data as members \
                     of the object the tag is written in, and only a value serde writes as an \
                     object has members to put there. serde refuses this one at run time — \
                     `cannot serialize tagged newtype variant {enum_name}::{variant_name} \
                     containing {shape}`. Name a `content` key so the value gets an object of its \
                     own, or wrap it in a struct whose fields can sit beside the tag."
                ),
                TaggedContent::Unit => format!(
                    "model_schema: variant `{variant_name}` of internally tagged enum \
                     `{enum_name}` wraps `()`, which serde writes as `null`: \
                     {TAGGED_UNIT_WRITTEN} Name a `content` key so the value gets an object of \
                     its own, or write `{variant_name}` as a unit variant, which serde writes as \
                     the tag alone as well."
                ),
                TaggedContent::Unnameable(shape) => format!(
                    "model_schema: variant `{variant_name}` of internally tagged enum \
                     `{enum_name}` wraps {shape}, whose members the expansion cannot name: \
                     `#[serde(tag = \"{tag_name}\")]` with no `content` writes them beside the \
                     tag, and a schema closed around the tag alone rejects every one of them. \
                     Name a `content` key so the value gets an object of its own, or wrap it in a \
                     struct whose fields can sit beside the tag."
                ),
            };
            Some(syn::Error::new_spanned(field, message).to_compile_error())
        })
        .collect()
}

/// The type `written` names under `Option`, its transparent wrappers and references, spelled with
/// the arguments its author fills it with, and `named` where no path is written there.
#[cfg(feature = "serde")]
fn filled_spelling(written: &syn::Type, named: &str) -> String {
    let mut current = written_type(written);
    loop {
        if let syn::Type::Reference(reference) = current {
            current = written_type(&reference.elem);
        } else if let syn::Type::Path(path) = current
            && let Some(last) = path.path.segments.last()
        {
            if (last.ident == "Option" || is_transparent_wrapper(&last.ident.to_string()))
                && let Some(held) = type_arguments_of(&last.arguments).next()
            {
                current = written_type(held);
            } else {
                return written_spelling(last);
            }
        } else {
            return named.to_owned();
        }
    }
}

/// The name a refusal quotes the type `written` names, `named`, by: with the arguments its author
/// fills it with where it is an untagged enum whose members those arguments fill, and by its name
/// alone everywhere else.
#[cfg(feature = "serde")]
fn union_spelling(written: &syn::Type, named: &str) -> String {
    if union_parameters(named).is_empty() {
        named.to_owned()
    } else {
        filled_spelling(written, named)
    }
}

/// How serde writes `filling`, the argument a generic type writes as its own value, where it
/// writes it as no object: a name as [`written_non_object`] reads it, and a string, a number, a
/// `bool`, a list or a tuple by its JSON type keyword. `None` for an `Option`, which
/// [`tagged_optional_message`] answers.
#[cfg(feature = "serde")]
fn filled_non_object(filling: &FieldDef) -> Option<NonObject> {
    if filling.is_optional() {
        return None;
    }
    written_non_object(filling)
        .or_else(|| written_non_object_wire(filling, written_leaves).map(NonObject::Whole))
}

/// The refusal of a newtype variant under a tag alone whose slot names a type declared above that
/// `NON_OBJECT_WIRES` proves serde writes as no object, read as [`flatten_edge_guard_error`] reads
/// it, or a generic type declared above that serde writes as what fills its type parameter,
/// filled there with a value serde writes as no object. `None` for every other slot.
#[cfg(feature = "serde")]
fn tagged_non_object_message(
    variant_name: &syn::Ident,
    enum_name: &syn::Ident,
    tag_name: &str,
    slot: TaggedSlot<'_>,
) -> Option<String> {
    let (inner_name, filling) = slot.read()?;
    let non_object = filling.map_or_else(|| written_non_object(slot.inner), filled_non_object);
    let written = match non_object? {
        NonObject::Branch(branch, named) => format!(
            "writes a union member that is not an object — its branch {branch} describes a \
             `{named}`"
        ),
        NonObject::Written(branch, written) => {
            let words = WrittenWords::of(written, true);
            return Some(format!(
                "model_schema: variant `{variant_name}` of internally tagged enum `{enum_name}` \
                 wraps `{inner_name}`, which writes a union member as {} — its branch {branch} \
                 describes a `{}`, {}. Name a `content` key so the value gets an object of its \
                 own, or wrap it in a struct whose fields can sit beside the tag. {}",
                words.member, words.keyword, words.serde_writes, words.scope
            ));
        }
        NonObject::Whole("null") => {
            return Some(format!(
                "model_schema: variant `{variant_name}` of internally tagged enum `{enum_name}` \
                 wraps `{inner_name}`, which serde writes as `()` — its schema describes a \
                 `null`: {TAGGED_UNIT_WRITTEN} Name a `content` key so the value gets an object \
                 of its own, or write `{variant_name}` as a unit variant, which serde writes as \
                 the tag alone as well. {FLATTENED_NON_OBJECT_SCOPE}"
            ));
        }
        NonObject::Whole(named) => {
            format!("is not written as an object — its schema describes a `{named}`")
        }
    };
    Some(format!(
        "model_schema: variant `{variant_name}` of internally tagged enum `{enum_name}` wraps \
         `{inner_name}`, which {written}, which cannot sit beside the tag: `#[serde(tag = \
         \"{tag_name}\")]` with no `content` writes the variant's data as members of the object \
         the tag is written in, and only a value serde writes as an object has members to put \
         there. Name a `content` key so the value gets an object of its own, or wrap it in a \
         struct whose fields can sit beside the tag. {FLATTENED_NON_OBJECT_SCOPE}"
    ))
}

/// The refusal of a newtype variant under a tag alone whose slot names a type declared above that
/// serde writes as an `Option`, or a generic type declared above that serde writes as what fills
/// its type parameter, filled there with an `Option` or such a type, which the tag refuses for
/// `Some` and `None` alike. `None` for every other slot.
#[cfg(feature = "serde")]
fn tagged_optional_message(
    variant_name: &syn::Ident,
    enum_name: &syn::Ident,
    tag_name: &str,
    slot: TaggedSlot<'_>,
) -> Option<String> {
    let (inner_name, filling) = slot.read()?;
    writes_an_optional(filling.unwrap_or(slot.inner)).then(|| {
        format!(
            "model_schema: variant `{variant_name}` of internally tagged enum `{enum_name}` wraps \
             `{inner_name}`, which serde writes as an optional, which cannot sit beside the tag: \
             `#[serde(tag = \"{tag_name}\")]` with no `content` writes the variant's data as \
             members of the object the tag is written in, and only a value serde writes as an \
             object has members to put there. serde refuses this one at run time — `cannot \
             serialize tagged newtype variant {enum_name}::{variant_name} containing an \
             optional`. Name a `content` key so the value gets an object of its own, or wrap it \
             in a struct whose fields can sit beside the tag. {TAGGED_OPTIONAL_SCOPE}"
        )
    })
}

/// Renders each variant of an internally tagged enum as a union member, beside the variant it was
/// collected from: every variant serde writes or reads is collected, in the order declared.
#[cfg(feature = "serde")]
fn render_internal_variants(
    variants: &[DiscriminatedVariant],
    item_enum: &syn::ItemEnum,
    tag_name: &str,
) -> Vec<(String, String, proc_macro2::TokenStream, bool)> {
    let enum_name = item_enum.ident.to_string();
    variants
        .iter()
        .zip(
            item_enum
                .variants
                .iter()
                .filter(|variant| variant_on_the_wire(variant)),
        )
        .map(|(variant, declared)| {
            render_internal_variant(variant, &declared.ident, tag_name, &enum_name)
        })
        .collect()
}

/// Renders one variant of an internally tagged enum as a union member.
#[cfg(feature = "serde")]
fn render_internal_variant(
    variant: &DiscriminatedVariant,
    variant_ident: &syn::Ident,
    tag_name: &str,
    self_type_name: &str,
) -> (String, String, proc_macro2::TokenStream, bool) {
    // Only the JSON Schema merge names the variant as serde's refusal names it.
    #[cfg(not(feature = "jsonschema"))]
    let _: &syn::Ident = variant_ident;
    let mut parts = tagged_variant_parts(tag_name, &variant.discriminator_value, &variant.docs);
    match variant.kind {
        VariantKind::Named => {
            write_named_variant_fields(
                &variant.field_defs,
                Some(tag_name),
                self_type_name,
                &mut parts,
            );
        }
        // A unit variant writes the tag alone; a tuple variant only reaches here as the flattened
        // newtype the guard admits, whose content joins the finished member below.
        VariantKind::Unit | VariantKind::TupleSingle | VariantKind::TupleMultiple => {}
    }
    parts.type_code.push('}');
    parts.schema_code.push('}');

    #[cfg(feature = "jsonschema")]
    let tag_object =
        tagged_variant_json_object(tag_name, &variant.discriminator_value, &parts.json_fields);

    let flattened = matches!(variant.kind, VariantKind::TupleSingle)
        .then(|| variant.field_defs.first())
        .flatten();

    let Some(inner) = flattened else {
        #[cfg(feature = "jsonschema")]
        let json = variant_merged_json_value(
            &tag_object,
            &variant.flattened_fields,
            self_type_name,
            &variant.discriminator_value,
        );
        #[cfg(not(feature = "jsonschema"))]
        let json = quote! {};

        #[cfg(feature = "zod")]
        let zod = variant_flatten_zod(
            &format!("z.strictObject({})", parts.schema_code),
            &variant.flattened_fields,
        );
        #[cfg(not(feature = "zod"))]
        let zod = String::new();

        let own_keys: Vec<String> = iter::once(tag_name.to_owned())
            .chain(member_names(&variant.field_defs))
            .collect();
        return (
            format!(
                "{}{}",
                parts.type_code,
                variant_flatten_typescript(&own_keys, &variant.flattened_fields)
            ),
            zod,
            json,
            !variant.flattened_fields.is_empty(),
        );
    };

    #[cfg(feature = "jsonschema")]
    let json = {
        let variant_name = &variant.discriminator_value;
        merged_object_value(
            &tag_object,
            // The same reference a `#[serde(flatten)]` base contributes: both name a type whose
            // members join the object being written.
            &[flatten_merged_source(inner)],
            &MergeDiagnostic {
                beside_tag: Some(&format!("{self_type_name}::{variant_ident}")),
                cycle_remedy: "name a `content` key so the content gets an object of its own",
                edge: &format!("the content of variant `{variant_name}`,"),
                non_object_remedy: "name a `content` key so the content gets an object of its own",
                subject: self_type_name,
            },
        )
    };
    #[cfg(not(feature = "jsonschema"))]
    let json = quote! {};

    // The content joins the member through [`deferred_zod_operand`] for the reason a flattened
    // base does: it names a `const` of its own, and one macro invocation sees one type.
    #[cfg(feature = "zod")]
    let zod = format!(
        "z.strictObject({}).and({})",
        parts.schema_code,
        deferred_zod_operand(&inner.zod_type())
    );
    #[cfg(not(feature = "zod"))]
    let zod = String::new();

    (
        format!("{} & {}", parts.type_code, inner.typescript_typename()),
        zod,
        json,
        true,
    )
}

/// Joins an internally tagged enum's rendered members into its three union surfaces. The Zod
/// union is a `discriminatedUnion` only while every member is an object carrying the tag in its
/// own shape; a flattened member (an intersection) forces a plain `z.union` instead.
#[cfg(feature = "serde")]
fn join_internal_union(
    members: &[(String, String, proc_macro2::TokenStream, bool)],
    tag_name: &str,
) -> (proc_macro2::TokenStream, String, String) {
    // Only the Zod surface names the tag: the other two read the members alone.
    #[cfg(not(feature = "zod"))]
    let _: &str = tag_name;
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let _: &_ = &members;

    #[cfg(feature = "jsonschema")]
    let main_schema_code = discriminated_main_schema_code(
        &members
            .iter()
            .map(|(_, _, json, _)| json.clone())
            .collect::<Vec<_>>(),
    );
    #[cfg(not(feature = "jsonschema"))]
    let main_schema_code = quote! {};

    #[cfg(feature = "typescript")]
    let type_code = members
        .iter()
        .map(|(ts, _, _, _)| ts.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    #[cfg(not(feature = "typescript"))]
    let type_code = String::new();

    #[cfg(feature = "zod")]
    let schema_code = {
        let joined = members
            .iter()
            .map(|(_, zod, _, _)| zod.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        if members.iter().any(|&(_, _, _, flattened)| flattened) {
            format!("z.union([{joined}])")
        } else {
            format!("z.discriminatedUnion({}, [{joined}])", ts_quoted(tag_name))
        }
    };
    #[cfg(not(feature = "zod"))]
    let schema_code = String::new();

    (main_schema_code, type_code, schema_code)
}

/// Processes an internally tagged enum (`#[serde(tag = "...")]` with no `content`) and generates
/// its definitions.
#[cfg(feature = "serde")]
fn process_internally_tagged_enum(
    mut item_enum: syn::ItemEnum,
    name: &syn::Ident,
    tag_name: &str,
    casing: EnumCasing<'_>,
    item_name: &str,
    args: &ModelSchemaArgs,
) -> TokenStream {
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    // Every other enum shape is written as a union of what its variants render as.
    let (module_name, module_ident) =
        enum_module_idents(name, item_name, AliasKind::NoEnumMembers, Surface::union());

    #[cfg(feature = "typescript")]
    let docs_vec = get_enum_docs(&item_enum);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let enum_module_name_opt = Some(module_name.as_str());
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let enum_module_name_opt = None;

    if let Some(output) = refused_item_output(
        &Item::Enum(item_enum.clone()),
        &internally_tagged_guard_errors(&item_enum, tag_name),
    ) {
        return output;
    }

    // Read before the walk, which takes each member's own attribute off it.
    let item_checks = checks::ItemChecks::of(&args.check, variant_members(&item_enum));
    let variants =
        collect_discriminated_variants(&mut item_enum, casing, enum_module_name_opt, args);
    if let Some(output) = guard_failure_output(&item_enum, Some(&item_enum.ident), &variants.2) {
        return output;
    }
    let decode_with = enum_decode_with(&item_enum, args);

    let members = render_internal_variants(&variants.0, &item_enum, tag_name);

    let (main_schema_code, type_code, schema_code) = join_internal_union(&members, tag_name);
    #[cfg(not(feature = "jsonschema"))]
    let _: &_ = &main_schema_code;
    #[cfg(not(feature = "typescript"))]
    let _: &_ = &type_code;
    #[cfg(not(feature = "zod"))]
    let _: &_ = &schema_code;
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let _: &_ = &name;

    #[cfg(feature = "typescript")]
    let docs = build_jsdoc_body(docs_vec.as_deref(), item_name);

    #[cfg(feature = "jsonschema")]
    let json_schema_method =
        enum_json_schema_methods(&main_schema_code, item_name, &item_enum.generics, args);

    #[cfg(feature = "typescript")]
    let ts_definition_method =
        tagged_ts_definition_method(&docs, item_name, &type_code, tag_name, &item_enum, casing);

    #[cfg(feature = "zod")]
    let zod_schema_method = generate_discriminated_enum_zod_schema_method(
        item_name,
        &name.to_string(),
        &type_parameters_in_scope(&item_enum.generics),
        args,
        &schema_code,
    );

    #[cfg(not(any(feature = "typescript", feature = "zod")))]
    let _: &_ = &item_name;

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let schema_example_method =
        enum_schema_example_method(&item_enum.attrs, name, &item_enum.generics, args);

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let schema_impl_items: Vec<proc_macro2::TokenStream> = vec![
        #[cfg(feature = "jsonschema")]
        json_schema_method,
        #[cfg(feature = "typescript")]
        ts_definition_method,
        #[cfg(feature = "zod")]
        zod_schema_method,
    ];

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let delegate_impl_items = build_struct_delegate_items(
        &module_ident,
        item_name,
        &name.to_string(),
        &type_parameters_in_scope(&item_enum.generics),
        schema_example_method.as_ref(),
    );

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    {
        enum_output_with_decode(
            &SchemaOutputParts {
                delegate_impl_items: &delegate_impl_items,
                generics: &item_enum.generics,
                item: &item_enum,
                module_ident: &module_ident,
                name,
                schema_impl_items: &schema_impl_items,
                validate_method: &build_enum_validate_method(
                    &variants.3,
                    &module_ident,
                    &item_checks,
                ),
                validation_fns: &variants.1,
            },
            &decode_with,
        )
    }

    #[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
    {
        let _: &_ = &variants.1;
        enum_output_with_decode(&item_enum, &decode_with, &variants.3, &item_checks)
    }
}

/// Renders one variant of an untagged enum as a union member.
#[cfg(feature = "serde")]
fn render_untagged_variant(
    kind: &VariantKind,
    variant: &syn::Variant,
    members: &UntaggedVariantMembers,
    self_type_name: &str,
) -> Result<(String, String, proc_macro2::TokenStream), syn::Error> {
    match (kind, members.field_defs.as_slice()) {
        (VariantKind::TupleSingle, [fld]) => Ok(render_untagged_tuple_single(fld, self_type_name)),
        (VariantKind::Named, _) => Ok(render_untagged_named(
            members,
            self_type_name,
            &variant.ident.to_string(),
        )),
        (VariantKind::Unit, _) if lone_slot_off_wire(variant) => {
            Err(collapsed_untagged_variant_error(variant))
        }
        (VariantKind::Unit, _) => Err(unsupported_untagged_variant_error(
            variant,
            "a unit variant",
        )),
        (VariantKind::TupleSingle | VariantKind::TupleMultiple, _) => {
            Err(unsupported_untagged_variant_error(
                variant,
                &format!("a tuple variant with {} fields", variant.fields.len()),
            ))
        }
    }
}

/// Refuses an untagged variant that reached the unit wire by taking its lone slot off it, spanned on
/// the variant.
#[cfg(feature = "serde")]
fn collapsed_untagged_variant_error(variant: &syn::Variant) -> syn::Error {
    let variant_name = &variant.ident;
    syn::Error::new_spanned(
        variant,
        format!(
            "model_schema: variant `{variant_name}`: the lone slot of `{variant_name}` is off the \
             wire in both directions, so serde writes and reads the variant as the bare `null` a \
             unit variant is written as, whatever the slot holds — and an untagged union has no \
             member spelling for `null`: a member is written as its inner type or as an object of \
             named fields, and the slot this one would have been written as never reaches the wire. \
             Keep the slot on the wire so the member has a value to be written as, or remove \
             `{variant_name}` from the union."
        ),
    )
}

/// Refuses a variant shape the untagged union has no member spelling for, spanned on the variant.
#[cfg(feature = "serde")]
fn unsupported_untagged_variant_error(variant: &syn::Variant, shape: &str) -> syn::Error {
    let variant_name = &variant.ident;
    syn::Error::new_spanned(
        variant,
        format!(
            "model_schema: variant `{variant_name}`: `#[serde(untagged)]` supports newtype \
             (`V(T)`) and struct (`V {{ … }}`) variants only — `{variant_name}` is {shape}, which \
             the union has no member spelling for: a member is written as the inner type or as an \
             object of named fields, and a variant that is neither has nothing to be written as. \
             Give it a single inner type or named fields, or drop `#[serde(untagged)]`."
        ),
    )
}

/// Renders a `TupleSingle` (`S(T)`) untagged variant as a union member (`T` / `T$Schema` / value).
/// The variant carries no key of its own, so a `None` inner reaches the wire as a bare `null`
/// rather than going absent — read through the slot spellings for that reason. That same missing
/// key is why a member naming the union itself is deferred with [`deferred_zod_operand`] rather
/// than the getter [`render_untagged_named`] hangs off a key: read eagerly, the name lands in its
/// own `const`'s initializer and throws on import.
#[cfg(feature = "serde")]
fn render_untagged_tuple_single(
    fld: &FieldDef,
    self_type_name: &str,
) -> (String, String, proc_macro2::TokenStream) {
    #[cfg(feature = "typescript")]
    let ts = fld.typescript_slot_typename_deferring_self(self_type_name);
    #[cfg(not(feature = "typescript"))]
    let ts = fld.typescript_slot_typename();

    #[cfg(feature = "zod")]
    let zod = {
        let slot = fld.zod_slot_type();
        if fld.contains_type_reference(self_type_name) {
            deferred_zod_operand(&slot)
        } else {
            slot
        }
    };
    #[cfg(not(feature = "zod"))]
    let zod = {
        let _: &str = self_type_name;
        String::new()
    };

    #[cfg(feature = "jsonschema")]
    let json_val = nullable_slot_json_schema_value(fld, field_json_schema_value(fld));
    #[cfg(not(feature = "jsonschema"))]
    let json_val = quote! {};

    (ts, zod, json_val)
}

/// Renders a `Named` (`{ a: A }`) untagged variant as a union member (object type / strictObject /
/// object schema), with the members of every `#[serde(flatten)]` source it carries joined onto that
/// object.
#[cfg(feature = "serde")]
fn render_untagged_named(
    members: &UntaggedVariantMembers,
    self_type_name: &str,
    variant_name: &str,
) -> (String, String, proc_macro2::TokenStream) {
    let field_defs = &members.field_defs;
    #[cfg(not(feature = "jsonschema"))]
    let _: &str = variant_name;

    let ts_fields = field_defs
        .iter()
        .map(|fld| {
            format!(
                "{}{}: {}",
                ts_member_key(&fld.name),
                fld.optional_key_marker(),
                fld.typescript_typename()
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let ts = format!(
        "{{ {ts_fields} }}{}",
        variant_flatten_typescript(&member_names(field_defs), &members.flattened_fields)
    );

    #[cfg(feature = "zod")]
    let zod = {
        let mut body = String::from("z.strictObject({ ");
        for fld in field_defs {
            let zod_field_type = fld.zod_type();
            let key = ts_member_key(&fld.name);
            if fld.contains_type_reference(self_type_name) {
                let _ = write!(body, "get {key}() {{ return {zod_field_type}; }}, ");
            } else {
                let _ = write!(body, "{key}: {zod_field_type}, ");
            }
        }
        body.push_str("})");
        variant_flatten_zod(&body, &members.flattened_fields)
    };
    #[cfg(not(feature = "zod"))]
    let zod = {
        let _: &str = self_type_name;
        String::new()
    };

    #[cfg(feature = "jsonschema")]
    let json_val = untagged_named_json_value(
        field_defs,
        &members.flattened_fields,
        self_type_name,
        variant_name,
    );
    #[cfg(not(feature = "jsonschema"))]
    let json_val = quote! {};

    (ts, zod, json_val)
}

/// The keys one untagged member names, and `None` where nothing here proves them. A `Named`
/// variant's keys are the ones it already spells; every other member is written as the type it
/// names, and its key list is not something a registry lookup can answer — nor is a `Named`
/// variant's once it flattens, since the source's keys belong to a type this expansion cannot read.
#[cfg(all(feature = "serde", feature = "typescript"))]
fn untagged_member_keys(
    kind: &VariantKind,
    field_defs: &[FieldDef],
    flattened_fields: &[FieldDef],
) -> Option<Vec<String>> {
    (matches!(kind, VariantKind::Named) && flattened_fields.is_empty())
        .then(|| field_defs.iter().map(|fld| fld.name.clone()).collect())
}

/// One untagged member's flatten operand with its siblings' keys marked absent, written in the
/// one-line shape [`render_untagged_named`] already wrote the member in. A member that is no
/// object literal keeps its spelling.
#[cfg(all(feature = "serde", feature = "typescript"))]
fn close_untagged_flatten_member(member: &str, excluded: &[String]) -> String {
    if excluded.is_empty() {
        return member.to_owned();
    }
    let Some(keys) = member.strip_suffix(" }") else {
        return member.to_owned();
    };
    let mut closed = keys.trim_end().to_owned();
    for key in excluded {
        if !closed.ends_with('{') {
            closed.push(';');
        }
        let _ = write!(closed, " {}?: never", ts_member_key(key));
    }
    closed.push_str(" }");
    closed
}

/// Builds the `{ type: object, properties, required, additionalProperties: false }` JSON-schema
/// value token for a `Named` untagged variant, with the members of every `#[serde(flatten)]` source
/// the variant carries merged beside its own.
#[cfg(all(feature = "serde", feature = "jsonschema"))]
fn untagged_named_json_value(
    field_defs: &[FieldDef],
    flattened_fields: &[FieldDef],
    self_type_name: &str,
    variant_name: &str,
) -> proc_macro2::TokenStream {
    let property_inserts = field_defs.iter().map(|fld| {
        let name_str = fld.name.clone();
        let value = nullable_slot_json_schema_value(fld, field_json_schema_value(fld));
        let required_insert = if fld.key_is_required() {
            quote! {
                required.push(serde_json::Value::String(#name_str.to_string()));
            }
        } else {
            quote! {}
        };
        let default_insert = default_json_insert(fld);
        quote! {
            properties.insert(#name_str.to_string(), #value);
            #required_insert
            #default_insert
        }
    });
    let object = quote! {
        {
            let mut object_schema = serde_json::Map::new();
            object_schema.insert(
                "type".to_string(),
                serde_json::Value::String("object".to_string()),
            );
            let mut properties = serde_json::Map::new();
            let mut required: ::std::vec::Vec<serde_json::Value> = ::std::vec::Vec::new();
            #(#property_inserts)*
            object_schema.insert(
                "properties".to_string(),
                serde_json::Value::Object(properties),
            );
            object_schema.insert(
                "required".to_string(),
                serde_json::Value::Array(required),
            );
            object_schema.insert(
                "additionalProperties".to_string(),
                serde_json::Value::Bool(false),
            );
            object_schema
        }
    };
    variant_merged_json_value(&object, flattened_fields, self_type_name, variant_name)
}

/// The `allOf` a string states two patterns in, where its enum's holds beside its own: one
/// `pattern` keyword states one.
#[cfg(feature = "jsonschema")]
fn both_patterns_json(fld: &FieldDef) -> Option<proc_macro2::TokenStream> {
    let meta = fld.model_schema_prop_meta.as_ref()?;
    let (own, held) = (meta.pattern.as_ref()?, meta.enum_pattern.as_ref()?);
    Some(quote! { serde_json::json!([{ "pattern": #own }, { "pattern": #held }]) })
}

/// Builds the `{ "type": "string", ... }` JSON-schema value token for a `String` field, including
/// any `pattern` / `minLength` / `maxLength` constraints from its `model_schema_prop` metadata.
#[cfg(feature = "jsonschema")]
fn string_field_json_schema_value(fld: &FieldDef) -> proc_macro2::TokenStream {
    let meta = fld.model_schema_prop_meta.as_ref();
    let pattern_insert = both_patterns_json(fld).map_or_else(
        || {
            meta.and_then(|m| m.pattern.clone()).map(|p| {
                quote! {
                    string_schema.insert(
                        "pattern".to_string(),
                        serde_json::Value::String(#p.to_string()),
                    );
                }
            })
        },
        |both| Some(quote! { string_schema.insert("allOf".to_string(), #both); }),
    );
    let min_insert = meta.and_then(|m| m.min_length).map(|n| {
        let len = n as u64;
        quote! {
            string_schema.insert(
                "minLength".to_string(),
                serde_json::Value::Number(serde_json::Number::from(#len)),
            );
        }
    });
    let max_insert = meta.and_then(|m| m.max_length).map(|n| {
        let len = n as u64;
        quote! {
            string_schema.insert(
                "maxLength".to_string(),
                serde_json::Value::Number(serde_json::Number::from(#len)),
            );
        }
    });
    quote! {
        ({
            let mut string_schema = serde_json::Map::new();
            string_schema.insert(
                "type".to_string(),
                serde_json::Value::String("string".to_string()),
            );
            #pattern_insert
            #min_insert
            #max_insert
            serde_json::Value::Object(string_schema)
        })
    }
}

/// Builds a standalone `serde_json::Value` token expression for a single field, with `Vec<T>`
/// array wrapping; used by untagged enum members where the JSON value is consumed directly (not
/// inserted under a property name).
#[cfg(all(feature = "serde", feature = "jsonschema"))]
fn field_json_schema_value(fld: &FieldDef) -> proc_macro2::TokenStream {
    // A covered sequence wrapper writes the JSON array of its element, so the member is dispatched
    // as the arrayed element it stands for.
    if let Some(element_field) = sequence_wrapper_field(fld) {
        return field_json_schema_value(&element_field);
    }

    let inner = match &fld.field_type {
        FieldDefType::SiblingType(name, arguments) => named_field_json_schema_value(
            fld,
            &sibling_json_schema_value(name, arguments, fld.type_span),
        ),
        FieldDefType::String => string_field_json_schema_value(fld),
        FieldDefType::Char => {
            quote! { serde_json::json!({ "type": "string", "minLength": 1, "maxLength": 1 }) }
        }
        FieldDefType::StringLiteral(literal) => {
            quote! { serde_json::json!({ "type": "string", "const": #literal }) }
        }
        FieldDefType::BooleanLiteral(value) => {
            quote! { serde_json::json!({ "type": "boolean", "const": #value }) }
        }
        FieldDefType::NumberLiteral(value) => {
            quote! { serde_json::json!({ "type": "number", "const": #value }) }
        }
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::Boolean => {
            // The arm has matched exactly the types the mapping answers a keyword for.
            let keyword = scalar_json_type_keyword(&fld.field_type).unwrap();
            let bounds = numeric_bound_keywords(fld);
            quote! { serde_json::json!({ "type": #keyword #bounds }) }
        }
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => object_id_json_schema_value(&object_id_hex_json_schema()),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => {
            // The arm has matched exactly the types the mapping answers for.
            let item = chrono_json_schema_item(&fld.field_type).unwrap();
            quote! { serde_json::json!(#item) }
        }
        FieldDefType::Map(key, value) => match map_json_schema_value(key, value) {
            Ok(map_schema) => map_schema,
            Err(rejection) => return member_rejection_value(&fld.name, &rejection),
        },
        FieldDefType::Tuple(elements) => match tuple_json_schema_value(elements) {
            Ok(tuple_schema) => tuple_schema,
            Err(rejection) => return member_rejection_value(&fld.name, &rejection),
        },
        FieldDefType::Range(kind, bound) => match range_json_schema_value(*kind, bound) {
            Ok(range_schema) => range_schema,
            Err(rejection) => return member_rejection_value(&fld.name, &rejection),
        },
        FieldDefType::Result(ok, err) => match result_json_schema_value(ok, err) {
            Ok(result_schema) => result_schema,
            Err(rejection) => return member_rejection_value(&fld.name, &rejection),
        },
        // A parameter is the filling that reached it, and a value the expansion cannot name at all
        // admits any value — the permissive empty schema, as in field position.
        FieldDefType::TypeParam(_) | FieldDefType::Unknown => opaque_json_schema_value(fld),
    };

    arrayed_json_schema_value(fld, inner)
}

/// [`map_member_rejection_error`] for a position that holds a value rather than writing a
/// statement — an untagged variant's member, whose value is consumed straight into the union's
/// `anyOf`.
#[cfg(all(feature = "serde", feature = "jsonschema"))]
fn member_rejection_value(
    field_name: &str,
    rejection: &MapMemberRejection,
) -> proc_macro2::TokenStream {
    let message = prefixed_guard_message(&map_member_rejection_message(
        &field_label(field_name),
        rejection,
    ));
    syn::Error::new(rejection.span(), message).to_compile_error()
}

/// One untagged member's field def in the two readings the guards need: as the surfaces will
/// render it, every reference to one of the enclosing item's own parameters already the opaque
/// value, and as the author spelled it. [`process_field`] reads the same pair off the same erase.
#[cfg(feature = "serde")]
fn untagged_member_field_defs(
    field: &Field,
    field_name: &str,
    type_parameters: &[String],
) -> (FieldDef, FieldDef) {
    let written = get_field_def(field_name, &field.ty, "");
    let mut rendered = written.clone();
    rendered.erase_type_parameters(type_parameters);
    (rendered, written)
}
/// The [`FieldContext`] one untagged variant hands each of its members. `rename_all` is the
/// variant's own, not the enum's — the enum's container-level `rename_all` reaches variant names
/// only, never the fields inside one.
#[cfg(feature = "serde")]
const fn untagged_variant_context<'ctx>(
    variant_defaulted: bool,
    rename_all: Option<&'ctx str>,
    schema_module_name: Option<&'ctx str>,
    enum_type_name: &'ctx str,
    type_parameters: &'ctx [String],
    variant_name: &'ctx str,
    member_steps: &'ctx [Vec<String>],
) -> FieldContext<'ctx> {
    FieldContext {
        container_defaulted: variant_defaulted,
        // An untagged member's own reader is hung by the walk that collects it, where a member's
        // bound decides which variant the payload is; nothing this context reaches hangs one.
        container_read_back: false,
        // That walk holds the member to its enum's check as well, ahead of this context.
        enum_args: None,
        member_steps,
        on_the_wire: true,
        rename_all,
        schema_module_name,
        type_name: enum_type_name,
        type_parameters,
        variant_ident: Some(variant_name),
    }
}

/// Gives a member of an untagged variant the hook that reads the models it holds unjudged. The
/// variant is then chosen as it was before a model judged itself as it was read, and the enum's
/// own read judges what the member holds. A brand the member holds is held to its own bound
/// there, so that bound still chooses the variant. A member its own bound reads, the last one
/// `walked` holds attributes for, is read by that hook alone.
#[cfg(feature = "serde")]
fn read_untagged_member_unjudged(
    ctx: &FieldContext<'_>,
    field: &Field,
    member_def: &FieldDef,
    index: usize,
    walked: &UntaggedVariantMembers,
) {
    if !walked.deferred_attrs.last().is_some_and(Vec::is_empty) {
        return;
    }
    let Some(shape) = member_held_shape(field, member_def) else {
        return;
    };
    judged_read::hook_member(
        field,
        &shape,
        &judged_read::HookPlace {
            choosing: true,
            container_defaulted: ctx.container_defaulted,
            index,
            renamed_under: None,
            variant: ctx.variant_ident,
            written_under_a_key: field.ident.is_some() && !is_flattened_field(field),
        },
    );
}

/// One untagged member's finished def, beside the guard verdicts it earned — the whole of what
/// the collector's loop needs back for the field. `container_defaulted` is the variant's own
/// `#[serde(default)]`, the container this member's omission is read against. `final_field_name`
/// is the key serde writes the member under.
#[cfg(feature = "serde")]
fn untagged_member_field_def(
    ctx: &FieldContext<'_>,
    field: &Field,
    field_name: &str,
    final_field_name: &str,
    prop_meta: ModelSchemaPropMeta,
    serde_field_meta: &SerdeFieldMeta,
    field_validation_guard_error: Option<proc_macro2::TokenStream>,
) -> (FieldDef, Vec<proc_macro2::TokenStream>) {
    let (mut field_def, written_def) =
        untagged_member_field_defs(field, final_field_name, ctx.type_parameters);
    apply_serde_key_omission(&mut field_def, field);
    let serde_guard_errors = field_guard_errors(
        field,
        field_name,
        field_def.is_optional(),
        prop_meta.nullable,
        serde_field_meta,
        ctx,
        field_validation_guard_error,
    );
    let member_guard_errors = collect_field_guard_errors(
        field,
        &field_def,
        &written_def,
        field_name,
        &prop_meta,
        serde_guard_errors,
    );
    field_def.resolve_self_references(ctx.type_name, ctx.type_parameters);
    apply_model_schema_prop_meta(&mut field_def, prop_meta, final_field_name);
    (field_def, member_guard_errors)
}

/// Collects each untagged variant's union-member parts: the TypeScript member type, the Zod
/// member schema, the JSON-schema value token, and the `compile_error!` tokens for any
/// field-level guard violations. The first three are always returned regardless of features.
/// `steps` holds the steps serde writes on the way to each member of the variant.
#[cfg(feature = "serde")]
fn collect_untagged_variant_members(
    variant: &mut syn::Variant,
    &UntaggedEnum {
        args,
        name: enum_type_name,
        rename_all_fields,
        schema_module_name,
        serde_default,
        type_parameters,
    }: &UntaggedEnum<'_>,
    steps: &[Vec<String>],
) -> UntaggedVariantMembers {
    let variant_name = variant.ident.to_string();
    // Read as the discriminated walk reads it: no guard of the wire reads a variant serde neither
    // writes nor reads.
    let on_the_wire = variant_on_the_wire(variant);
    let variant_defaulted = has_serde_default(&variant.attrs);
    // An untagged variant has no name on the wire, so the rename the seam also resolves is unused
    // here.
    let (_, variant_rename_all) = variant_serde_names(&variant.attrs, rename_all_fields);
    let mut walked = UntaggedVariantMembers {
        bound: Vec::new(),
        checks: Vec::new(),
        deferred_attrs: Vec::new(),
        field_defs: Vec::new(),
        flattened_fields: Vec::new(),
        guard_errors: Vec::new(),
        validation_fns: Vec::new(),
    };

    for (index, field) in variant.fields.iter_mut().enumerate() {
        let omission = parse_serde_key_omission(&field.attrs);
        let positional = field.ident.is_none();
        // Read before the field is processed, which strips the attributes the declaration carried.
        let is_flatten = is_flattened_field(field);
        walked.guard_errors.extend(variant_member_wire_errors(
            on_the_wire,
            field,
            index,
            &variant_name,
            enum_type_name,
        ));

        let field_name = field_ident_string(field);
        let prop_meta = member_prop_meta(field, Some(args));
        let serde_field_meta = parse_serde_field_attributes(&field.attrs);
        let member_ctx = FieldContext {
            on_the_wire,
            ..untagged_variant_context(
                variant_defaulted,
                variant_rename_all.as_deref(),
                schema_module_name,
                enum_type_name,
                type_parameters,
                &variant_name,
                steps,
            )
        };
        // The wire key: field-level rename wins outright, otherwise the variant's own rename_all
        // cases the Rust ident — the same resolution `process_field` runs for a struct field.
        let wire_name = get_final_field_name(
            &field_name,
            serde_field_meta.rename.as_deref(),
            member_ctx.rename_all,
        );

        let new_attrs = declaration_attrs(field);
        let mut injected_attrs: Vec<syn::Attribute> = Vec::new();
        let (validation_fn, validate_body, field_validation_guard_error) =
            generate_field_validation(
                &member_ctx,
                field,
                index,
                &prop_meta,
                // The one position where the read is also the gate: an untagged union picks the
                // variant by which one accepts the payload.
                ConstraintGate::Deserializer,
                &mut injected_attrs,
            );
        field.attrs = new_attrs;
        // Pushed for every field, flattened or not: `apply_deferred_field_attrs` zips this against
        // the declaration's own fields, so a skipped push shifts every later field's attributes.
        walked.deferred_attrs.push(injected_attrs);
        if !is_flatten {
            walked.validation_fns.extend(validation_fn);
        }

        let (mut member_def, member_guard_errors) = untagged_member_field_def(
            &member_ctx,
            field,
            &field_name,
            &wire_name,
            prop_meta,
            &serde_field_meta,
            field_validation_guard_error,
        );
        walked.guard_errors.extend(member_guard_errors);
        hand_serde_default(&mut member_def, serde_default(&variant_name, &field_name));
        // Read once the member's def exists, since that is what says whether the member names a
        // type that could publish a validator.
        let place = member_place(field, index, true, member_ctx.steps(index));
        let declared = match (is_flatten, field.ident.is_some()) {
            (true, _) => nested_validate_body(field, &member_def, &place),
            (false, true) => named_member_validate_body(validate_body, field, &member_def, &place),
            (false, false) => slot_validate_body(
                validate_body,
                nested_validate_body(field, &member_def, &place),
            ),
        };
        read_untagged_member_unjudged(&member_ctx, field, &member_def, index, &walked);
        let named = held_member_checks(field, &member_def, &place);
        if let Some(body) = then_checks(declared, named) {
            walked.bound.push(bound_member(field, index));
            walked.checks.push(body);
        }
        if is_flatten {
            walked.flattened_fields.push(member_def);
            continue;
        }
        if positional && omission.absent_from_wire() {
            continue;
        }
        push_described_field(&mut walked.field_defs, member_def);
    }
    walked
}

#[cfg(feature = "serde")]
fn collect_untagged_members(
    item_enum: &mut syn::ItemEnum,
    schema_module_name: Option<&str>,
    args: &ModelSchemaArgs,
) -> UntaggedMemberData {
    let enum_type_name = item_enum.ident.to_string();
    let type_parameters = type_parameters_in_scope(&item_enum.generics);
    let rename_all_fields = parse_serde_type_attributes(&item_enum.attrs).rename_all_fields;
    let mut ts_parts: Vec<String> = Vec::new();
    #[cfg(feature = "typescript")]
    let mut ts_member_keys: Vec<Option<Vec<String>>> = Vec::new();
    let mut zod_parts: Vec<String> = Vec::new();
    #[cfg(feature = "zod")]
    let mut zod_merge_parts: ZodMergeParts = Vec::new();
    #[cfg(not(feature = "zod"))]
    let zod_merge_parts: ZodMergeParts = Vec::new();
    let mut json_parts: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut guard_errors: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut validation_fns: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut per_variant_checks: Vec<(proc_macro2::TokenStream, Vec<proc_macro2::TokenStream>)> =
        Vec::new();
    let mut deferred_attrs: Vec<Vec<syn::Attribute>> = Vec::new();
    let steps_by_variant = steps_of_variants(item_enum, rename_all_fields.as_deref());
    // Read before the walk, as a struct's are.
    let serde_default = variant_serde_defaults(item_enum);
    let walk = UntaggedEnum {
        args,
        name: &enum_type_name,
        rename_all_fields: rename_all_fields.as_deref(),
        schema_module_name,
        serde_default: &*serde_default,
        type_parameters: &type_parameters,
    };

    for (variant, steps) in item_enum.variants.iter_mut().zip(&steps_by_variant) {
        let declared_kind = classify_variant(variant);
        let kind = variant_wire_kind(variant);
        let total_fields = variant.fields.len();
        let mut walked = collect_untagged_variant_members(variant, &walk, steps);
        guard_errors.append(&mut walked.guard_errors);
        validation_fns.append(&mut walked.validation_fns);
        deferred_attrs.append(&mut walked.deferred_attrs);

        let rendered = variant_on_the_wire(variant)
            .then(|| render_untagged_variant(&kind, variant, &walked, &enum_type_name));
        match rendered {
            None => {}
            Some(Ok((ts, zod, json_val))) => {
                #[cfg(feature = "zod")]
                zod_merge_parts.extend(zod_merge_branches(
                    &kind,
                    &walked.field_defs,
                    &zod,
                    ts_parts.len() + 1,
                ));
                #[cfg(feature = "typescript")]
                ts_member_keys.push(untagged_member_keys(
                    &kind,
                    &walked.field_defs,
                    &walked.flattened_fields,
                ));
                ts_parts.push(ts);
                zod_parts.push(zod);
                json_parts.push(json_val);
            }
            Some(Err(err)) => guard_errors.push(err.to_compile_error()),
        }

        per_variant_checks.push((
            variant_check_pattern(&variant.ident, &declared_kind, total_fields, &walked.bound),
            walked.checks,
        ));
    }

    if guard_errors.is_empty() {
        apply_deferred_field_attrs(
            item_enum
                .variants
                .iter_mut()
                .flat_map(|variant| variant.fields.iter_mut()),
            deferred_attrs,
        );
    }

    #[cfg(feature = "typescript")]
    let ts_merge_parts = ts_flatten_operands(&ts_parts, &ts_member_keys);
    #[cfg(not(feature = "typescript"))]
    let ts_merge_parts: Vec<String> = Vec::new();

    (
        ts_parts,
        ts_merge_parts,
        zod_parts,
        zod_merge_parts,
        json_parts,
        guard_errors,
        validation_fns,
        build_member_check_arms(per_variant_checks),
    )
}

/// What an object flattening an untagged enum spells in the enum's name's place, one operand per
/// member — and nothing where the name already spells the same payload set.
#[cfg(all(feature = "serde", feature = "typescript"))]
fn ts_flatten_operands(members: &[String], member_keys: &[Option<Vec<String>>]) -> Vec<String> {
    let operands: Vec<String> = sibling_key_exclusions(member_keys)
        .iter()
        .zip(members)
        .map(|(excluded, member)| close_untagged_flatten_member(member, excluded))
        .collect();
    if operands == members {
        Vec::new()
    } else {
        operands
    }
}

/// What one rendered member contributes to an object that merges the enum: the members of the union
/// it names, when it names one the registry has recorded, and the member as rendered otherwise.
#[cfg(all(feature = "serde", feature = "zod"))]
fn zod_merge_branches(
    kind: &VariantKind,
    field_defs: &[FieldDef],
    rendered: &str,
    branch: usize,
) -> Vec<ZodUnionMember> {
    let leaves = match (kind, field_defs) {
        (VariantKind::TupleSingle, [fld]) => zod_member_leaves(fld, rendered),
        _ => vec![ZodUnionMember {
            branch: Vec::new(),
            non_object: None,
            spelling: rendered.to_owned(),
            written_as: None,
        }],
    };
    leaves
        .into_iter()
        .map(|leaf| {
            let mut trail = vec![branch];
            trail.extend(leaf.branch);
            ZodUnionMember {
                branch: trail,
                non_object: leaf.non_object,
                spelling: leaf.spelling,
                written_as: leaf.written_as,
            }
        })
        .collect()
}

/// What one rendered member contributes below its own position: the member itself, the leaves of
/// any union it names the registry has recorded, and — under an `Option` — both the value's choice
/// and the absence's.
#[cfg(all(feature = "serde", feature = "zod"))]
fn zod_member_leaves(fld: &FieldDef, rendered: &str) -> Vec<ZodUnionMember> {
    // The binding of a type that names a `preprocess` is the one schema that calls it, and the
    // one text that replaces the placeholder of a member's default, so no member of it is written
    // in its place.
    let nested = if fld.zod_read_whole() {
        Vec::new()
    } else {
        fld.zod_union_members()
    };
    let mut leaves = if !nested.is_empty() {
        nested
    } else if let Some(published) = named_wire_leaves(fld, surfaced_leaves) {
        // The name stands for a choice of its own.
        published
            .into_iter()
            .map(|leaf| ZodUnionMember {
                branch: leaf.branch,
                non_object: leaf.non_object,
                spelling: rendered.to_owned(),
                written_as: leaf.written_as,
            })
            .collect()
    } else {
        vec![ZodUnionMember {
            branch: Vec::new(),
            non_object: written_non_object_wire(fld, surfaced_leaves),
            spelling: rendered.to_owned(),
            written_as: fld.is_unit().then_some(WrittenAs::Unit),
        }]
    };
    if fld.is_optional() {
        for leaf in &mut leaves {
            leaf.branch.insert(0, 1);
        }
        leaves.push(ZodUnionMember {
            branch: vec![2],
            non_object: Some("null"),
            spelling: rendered.to_owned(),
            written_as: None,
        });
    }
    leaves
}

/// What serde writes a written type as, when that type proves it is not an object, and `None` when
/// nothing here proves it. Asked of an untagged union's member and of the whole surface an item
/// publishes alike, which is what keeps a member and the name standing for it on one answer.
#[cfg(feature = "serde")]
fn written_non_object_wire(fld: &FieldDef, leaves_of: LeavesOf) -> Option<&'static str> {
    if fld.array_depth > 0 {
        return Some("array");
    }
    if let FieldDefType::Tuple(elements) = &fld.field_type {
        return Some(if elements.is_empty() { "null" } else { "array" });
    }
    if let FieldDefType::SiblingType(name, _) = &fld.field_type {
        return is_sequence_wrapper(name)
            .then_some("array")
            .or_else(|| registered_non_object_wire(name, leaves_of));
    }
    // Everything the two structural questions above leave is a value one keyword describes or
    // nothing here can name, which is exactly what the shared mapping answers.
    scalar_json_type_keyword(&fld.field_type)
}

/// What a `#[model_schema()]` item publishes, as leaves in the merge's vocabulary — the answer
/// [`crate::utils::record_wire_leaves`] stores for every name a flattened member can then reach
/// through.
#[cfg(feature = "serde")]
fn published_wire_leaves(written: &FieldDef, leaves_of: LeavesOf) -> Vec<WireLeaf> {
    if written.is_optional() {
        return under_an_option(published_wire_leaves(
            &without_its_option(written),
            leaves_of,
        ));
    }
    named_wire_leaves(written, leaves_of).unwrap_or_else(|| {
        vec![WireLeaf {
            branch: Vec::new(),
            non_object: written_non_object_wire(written, leaves_of),
            written_as: written.is_unit().then_some(WrittenAs::Unit),
        }]
    })
}

/// The leaves an externally tagged enum's variants write, one per variant serde writes or reads
/// and in the order the `oneOf` it publishes writes them.
#[cfg(feature = "serde")]
fn external_variant_wire_leaves(variants: &Punctuated<syn::Variant, Token![,]>) -> Vec<WireLeaf> {
    variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
        .enumerate()
        .map(|(index, variant)| {
            let unit = matches!(variant_wire_kind(variant), VariantKind::Unit);
            WireLeaf {
                branch: vec![index + 1],
                non_object: unit.then_some("string"),
                written_as: unit.then_some(WrittenAs::UnitVariantName),
            }
        })
        .collect()
}

/// The leaves the item a field names published, where the field's whole wire *is* that name's and
/// one of those leaves proves serde writes no object at its position — and `None` everywhere the
/// one-leaf dispatch already answers.
#[cfg(feature = "serde")]
fn named_wire_leaves(written: &FieldDef, leaves_of: LeavesOf) -> Option<Vec<WireLeaf>> {
    if written.array_depth > 0 {
        return None;
    }
    let FieldDefType::SiblingType(name, _) = &written.field_type else {
        return None;
    };
    let leaves = leaves_of(name)?;
    leaves
        .iter()
        .any(|leaf| leaf.non_object.is_some())
        .then_some(leaves)
}

/// Builds the schema module's impl items for an untagged enum: its JSON schema, its `TypeScript`
/// definition, and its Zod schema, in the order the module publishes them.
#[cfg(all(
    feature = "serde",
    any(feature = "zod", feature = "typescript", feature = "jsonschema")
))]
fn build_untagged_schema_impl_items(
    // The declaration itself rather than the pieces read off it: the name and the parameters are
    // read by different surfaces, and the JSON one reads the parameters beside the attribute.
    item_enum: &syn::ItemEnum,
    item_name: &str,
    docs_vec: Option<&[String]>,
    args: &ModelSchemaArgs,
    ts_parts: &[String],
    zod_parts: &[String],
    json_parts: &[proc_macro2::TokenStream],
) -> Vec<proc_macro2::TokenStream> {
    #[cfg(not(feature = "jsonschema"))]
    let _: &_ = &args;
    #[cfg(not(feature = "typescript"))]
    let _: &_ = &(ts_parts, docs_vec);
    #[cfg(not(feature = "zod"))]
    let _: &_ = &zod_parts;
    #[cfg(not(feature = "jsonschema"))]
    let _: &_ = &json_parts;

    #[cfg(feature = "jsonschema")]
    let main_schema_code = quote! {
        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert("anyOf".to_string(), {
            let result: ::std::vec::Vec<serde_json::Value> = vec![
                #(#json_parts), *
            ];

            serde_json::Value::Array(result)
        });

        serde_json::Value::Object(schema_obj)
    };

    #[cfg(feature = "typescript")]
    let type_code = ts_parts.join(" | ");

    #[cfg(feature = "zod")]
    let schema_code = format!("z.union([{}])", zod_parts.join(", "));

    #[cfg(feature = "typescript")]
    let docs = build_jsdoc_body(docs_vec, item_name);

    #[cfg(feature = "jsonschema")]
    let json_schema_method =
        enum_json_schema_methods(&main_schema_code, item_name, &item_enum.generics, args);

    #[cfg(feature = "typescript")]
    let variant_reader = untagged_enum_variant_reader(item_name);

    #[cfg(feature = "typescript")]
    let ts_definition_method = generate_discriminated_enum_ts_definition_method(
        &docs,
        item_name,
        &item_enum.ident.to_string(),
        &ts_generic_params(&item_enum.generics),
        &type_code,
        &variant_reader,
    );

    #[cfg(feature = "zod")]
    let zod_schema_method = generate_discriminated_enum_zod_schema_method(
        item_name,
        &item_enum.ident.to_string(),
        &type_parameters_in_scope(&item_enum.generics),
        args,
        &schema_code,
    );

    #[cfg(feature = "jsonschema")]
    let unit_member = unit_member_record_item(&Item::Enum(item_enum.clone()));

    vec![
        #[cfg(feature = "jsonschema")]
        json_schema_method,
        #[cfg(feature = "jsonschema")]
        unit_member,
        #[cfg(feature = "typescript")]
        ts_definition_method,
        #[cfg(feature = "zod")]
        zod_schema_method,
    ]
}

/// Processes an untagged enum (`#[serde(untagged)]`), emitting a TypeScript union (`A | B`), a Zod
/// `z.union([...])`, and a JSON-schema `anyOf`. Mirrors [`process_discriminated_enum`]'s
/// setup/assembly so all feature combinations compile.
#[cfg(feature = "serde")]
fn process_untagged_enum(
    mut item_enum: syn::ItemEnum,
    name: &syn::Ident,
    item_name: &str,
    args: &ModelSchemaArgs,
) -> TokenStream {
    // Compute the schema module name and register the enum so other types can find it.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    // Every other enum shape is written as a union of what its variants render as.
    let (module_name, module_ident) =
        enum_module_idents(name, item_name, AliasKind::NoEnumMembers, Surface::union());

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let docs_vec = get_enum_docs(&item_enum);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let enum_module_name_opt = Some(module_name.as_str());
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let enum_module_name_opt = None;

    // Read before the walk, which takes each member's own attribute off it.
    let item_checks = checks::ItemChecks::of(&args.check, variant_members(&item_enum));

    // Render each variant into its union member (TS / Zod / JSON parts).
    let (
        ts_parts,
        ts_merge_parts,
        zod_parts,
        zod_merge_parts,
        json_parts,
        guard_errors,
        enum_validation_fns,
        validate_arms,
    ) = collect_untagged_members(&mut item_enum, enum_module_name_opt, args);

    // A violated field guard makes the whole contract unsound, so the schema surface is dropped
    // and only the original item plus the errors are emitted.
    if let Some(output) = guard_failure_output(&item_enum, Some(&item_enum.ident), &guard_errors) {
        return output;
    }

    // Recorded only past the guards, so nothing merges an enum whose own schema was dropped.
    #[cfg(feature = "zod")]
    record_zod_union_members(&name.to_string(), &zod_merge_parts);
    #[cfg(not(feature = "zod"))]
    let _: &_ = &zod_merge_parts;
    #[cfg(feature = "typescript")]
    record_ts_union_members(&name.to_string(), &ts_merge_parts);
    #[cfg(not(feature = "typescript"))]
    let _: &_ = &ts_merge_parts;

    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let _: &_ = &(name, item_name, &ts_parts, &zod_parts, &json_parts);
    let decode_with = enum_decode_with(&item_enum, args);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let schema_example_method =
        enum_schema_example_method(&item_enum.attrs, name, &item_enum.generics, args);

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let schema_impl_items = build_untagged_schema_impl_items(
        &item_enum,
        item_name,
        docs_vec.as_deref(),
        args,
        &ts_parts,
        &zod_parts,
        &json_parts,
    );

    // The union's own accessor is where a violated member bound is named in Rust: the read hands
    // the bound to serde, which drops the sentence with the candidate it removes.
    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let validate_method = build_enum_validate_method(&validate_arms, &module_ident, &item_checks);
    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    let delegate_impl_items = build_struct_delegate_items(
        &module_ident,
        item_name,
        &name.to_string(),
        &type_parameters_in_scope(&item_enum.generics),
        schema_example_method.as_ref(),
    );

    #[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
    {
        enum_output_with_decode(
            &SchemaOutputParts {
                delegate_impl_items: &delegate_impl_items,
                generics: &item_enum.generics,
                item: &item_enum,
                module_ident: &module_ident,
                name,
                schema_impl_items: &schema_impl_items,
                validate_method: &validate_method,
                validation_fns: &enum_validation_fns,
            },
            &decode_with,
        )
    }

    #[cfg(not(any(feature = "zod", feature = "typescript", feature = "jsonschema")))]
    {
        let _: &_ = &enum_validation_fns;
        enum_output_with_decode(&item_enum, &decode_with, &validate_arms, &item_checks)
    }
}

#[cfg(feature = "jsonschema")]
fn generate_type_schema(
    fld: &FieldDef,
    field_name_str: &str,
    type_json_schema: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(fld, type_json_schema.clone()),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), #schema);
    }
}

/// The open TypeScript and Zod member every variant of a tagged enum starts from: the tag holding
/// the variant's discriminator, with room for whatever the variant writes beside it. Both tagged
/// forms open the same way, so the spelling lives here once.
fn tagged_variant_parts(
    tag_name: &str,
    discriminator_value: &str,
    discriminator_docs: &str,
) -> VariantParts {
    let tag_key = ts_member_key(tag_name);
    let value = ts_quoted(discriminator_value);
    VariantParts {
        json_fields: Vec::new(),
        schema_code: format!("{{\n  {tag_key}: z.literal({value}),\n"),
        type_code: format!(
            "{{\n{}\n  {tag_key}: {value};\n",
            member_jsdoc_block(discriminator_docs)
        ),
    }
}

/// The closed object a tagged variant describes as, as a `serde_json::Map` expression: the tag
/// pinned to the variant's discriminator, plus whatever the variant's own fields contributed. A
/// `Map` rather than a `Value` because the internally tagged form merges further members into it.
#[cfg(feature = "jsonschema")]
fn tagged_variant_json_object(
    tag_name: &str,
    discriminator_value: &str,
    json_fields: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let discriminator_value_str = discriminator_value.to_owned();
    let tag_name_str = tag_name.to_owned();
    quote! {
        {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert(
                "additionalProperties".to_string(),
                serde_json::Value::Bool(false),
            );
            let mut properties = serde_json::Map::new();
            let mut required = ::std::vec::Vec::new();

            properties.insert(
                #tag_name_str.to_string(),
                serde_json::json!({
                    "type": "string",
                    "const": #discriminator_value_str,
                }),
            );
            required.push(serde_json::Value::String(#tag_name_str.to_string()));

            #(#json_fields)*

            schema_obj.insert(
                "properties".to_string(),
                serde_json::Value::Object(properties),
            );

            schema_obj.insert("required".to_string(), serde_json::Value::Array(required));

            schema_obj
        }
    }
}

/// Generates TypeScript and Zod schema code for a discriminated enum variant.
fn generate_variant_code(
    tag_name: &str,
    content_name: &str,
    variant: &DiscriminatedVariant,
    self_type_name: &str,
) -> (String, String, proc_macro2::TokenStream) {
    let discriminator_value = &variant.discriminator_value;
    let field_defs = &variant.field_defs;
    let mut parts = tagged_variant_parts(tag_name, discriminator_value, &variant.docs);

    match &variant.kind {
        VariantKind::Unit => {
            // Unit variant: no additional fields beyond the discriminator TypeScript: { type:
            // "Variant" } Zod: { type: z.literal("Variant") }
        }
        VariantKind::Named => {
            write_adjacent_named_variant_fields(variant, content_name, self_type_name, &mut parts);
        }
        VariantKind::TupleSingle => {
            write_tuple_single_variant_fields(field_defs, content_name, self_type_name, &mut parts);
        }
        VariantKind::TupleMultiple => {
            write_tuple_multiple_variant_fields(
                field_defs,
                content_name,
                self_type_name,
                &mut parts,
            );
        }
    }

    parts.type_code.push('}');
    parts.schema_code.push('}');

    #[cfg(feature = "jsonschema")]
    let json_schema_variant = {
        let object = tagged_variant_json_object(tag_name, discriminator_value, &parts.json_fields);
        quote! { serde_json::Value::Object(#object) }
    };

    #[cfg(not(feature = "jsonschema"))]
    let json_schema_variant = quote! {};

    (parts.type_code, parts.schema_code, json_schema_variant)
}

/// Writes an adjacently tagged struct variant's fields nested under the content key, the shape
/// serde actually writes for it (`{"tag":"Variant","content":{...fields}}`). Reuses
/// [`write_named_variant_fields`]'s own `None`-tag rendering rather than a second nesting mechanism.
fn write_adjacent_named_variant_fields(
    variant: &DiscriminatedVariant,
    content_name: &str,
    self_type_name: &str,
    parts: &mut VariantParts,
) {
    let mut inner = VariantParts {
        json_fields: Vec::new(),
        schema_code: String::new(),
        type_code: String::new(),
    };
    write_named_variant_fields(&variant.field_defs, None, self_type_name, &mut inner);

    let content_key = ts_member_key(content_name);
    let _ = writeln!(
        parts.type_code,
        "  {content_key}: {{\n{}}}{};",
        inner.type_code,
        variant_flatten_typescript(
            &member_names(&variant.field_defs),
            &variant.flattened_fields
        )
    );

    #[cfg(feature = "zod")]
    let _ = writeln!(
        parts.schema_code,
        "  {content_key}: {},",
        variant_flatten_zod(
            &format!("z.strictObject({{\n{}}})", inner.schema_code),
            &variant.flattened_fields
        )
    );

    #[cfg(feature = "jsonschema")]
    push_named_content_json_field(
        &mut parts.json_fields,
        content_name,
        &inner.json_fields,
        variant,
        self_type_name,
    );
}

/// Pushes the JSON-schema property/required entries for an adjacently tagged struct variant's
/// content key, mirroring [`push_single_tuple_json_field`] for the tuple-single case.
#[cfg(feature = "jsonschema")]
fn push_named_content_json_field(
    json_schema_variant_fields: &mut Vec<proc_macro2::TokenStream>,
    content_name: &str,
    inner_json_fields: &[proc_macro2::TokenStream],
    variant: &DiscriminatedVariant,
    self_type_name: &str,
) {
    let content_name_str = content_name.to_owned();
    let inner = named_content_json_value(
        inner_json_fields,
        &variant.flattened_fields,
        self_type_name,
        &variant.discriminator_value,
    );
    json_schema_variant_fields.push(quote! {
        properties.insert(#content_name_str.to_string(), #inner);
        required.push(serde_json::Value::String(#content_name_str.to_string()));
    });
}

/// Writes the named-field portion of an enum variant (TypeScript, Zod, JSON Schema). `tag_name` is
/// the discriminator's key beside these fields (already written by the caller), or `None` where
/// the fields sit in an object of their own.
fn write_named_variant_fields(
    field_defs: &[FieldDef],
    tag_name: Option<&str>,
    self_type_name: &str,
    parts: &mut VariantParts,
) {
    let variant_type_code = &mut parts.type_code;
    let variant_schema_code = &mut parts.schema_code;
    let json_schema_variant_fields = &mut parts.json_fields;
    for fld in field_defs {
        let key = ts_member_key(&fld.name);

        let _ = writeln!(
            variant_type_code,
            "{}\n  {}{}: {};",
            member_jsdoc_block(&fld.docs),
            key,
            fld.optional_key_marker(),
            fld.typescript_typename()
        );

        #[cfg(feature = "zod")]
        {
            let zod_field_type = fld.zod_type();
            let is_recursive = fld.contains_type_reference(self_type_name);

            if is_recursive {
                let _ = writeln!(
                    variant_schema_code,
                    "  get {key}() {{ return {zod_field_type}; }},"
                );
            } else {
                let _ = writeln!(variant_schema_code, "  {key}: {zod_field_type},");
            }
        }

        #[cfg(not(feature = "zod"))]
        {
            let _: &_ = &(&variant_schema_code, self_type_name);
        }

        #[cfg(feature = "jsonschema")]
        if tag_name != Some(fld.name.as_str()) {
            json_schema_variant_fields.push(build_field_schema(fld));
        }
        #[cfg(not(feature = "jsonschema"))]
        let _: &_ = &(tag_name, &json_schema_variant_fields);
    }
}

/// Pushes the JSON-schema property/required entries for a single-element tuple variant value.
#[cfg(feature = "jsonschema")]
fn push_single_tuple_json_field(
    json_schema_variant_fields: &mut Vec<proc_macro2::TokenStream>,
    content_name: &str,
    fld: &FieldDef,
) {
    let content_name_str = content_name.to_owned();
    let field_schema = match bounded_slot_json_schema(fld) {
        Ok(field_schema) => field_schema,
        Err(rejection) => {
            json_schema_variant_fields.push(map_member_rejection_error(content_name, &rejection));
            return;
        }
    };
    json_schema_variant_fields.push(quote! {
        properties.insert(#content_name_str.to_string(), #field_schema);
        required.push(serde_json::Value::String(#content_name_str.to_string()));
    });
}

/// Writes the single-element tuple portion of a discriminated enum variant. The content key is a
/// slot: a `None` there reaches the wire as `null` under the key rather than dropping it, read
/// through the slot spellings for that reason.
fn write_tuple_single_variant_fields(
    field_defs: &[FieldDef],
    content_name: &str,
    self_type_name: &str,
    parts: &mut VariantParts,
) {
    let variant_type_code = &mut parts.type_code;
    let variant_schema_code = &mut parts.schema_code;
    let json_schema_variant_fields = &mut parts.json_fields;
    let Some(fld) = field_defs.first() else {
        let _: (&_, &_, &_) = (
            self_type_name,
            &variant_schema_code,
            &json_schema_variant_fields,
        );
        return;
    };
    let content_key = ts_member_key(content_name);

    let _ = writeln!(
        variant_type_code,
        "  /** Tuple value */\n  {}: {};",
        content_key,
        fld.typescript_slot_typename()
    );

    #[cfg(feature = "zod")]
    {
        let zod_field_type = fld.zod_slot_type();
        let is_recursive = fld.contains_type_reference(self_type_name);

        if is_recursive {
            let _ = writeln!(
                variant_schema_code,
                "  get {content_key}() {{ return {zod_field_type}; }},"
            );
        } else {
            let _ = writeln!(variant_schema_code, "  {content_key}: {zod_field_type},");
        }
    }

    #[cfg(not(feature = "zod"))]
    {
        let _: &_ = &(&variant_schema_code, self_type_name);
    }

    #[cfg(feature = "jsonschema")]
    push_single_tuple_json_field(json_schema_variant_fields, content_name, fld);
    #[cfg(not(feature = "jsonschema"))]
    let _: &_ = &json_schema_variant_fields;
}

/// Writes the multi-element tuple portion of a discriminated enum variant.
fn write_tuple_multiple_variant_fields(
    field_defs: &[FieldDef],
    content_name: &str,
    self_type_name: &str,
    parts: &mut VariantParts,
) {
    let variant_type_code = &mut parts.type_code;
    let variant_schema_code = &mut parts.schema_code;
    let json_schema_variant_fields = &mut parts.json_fields;
    // Multi-element tuple: use TypeScript tuple type `value: [T1, T2, ...]`
    let ts_tuple_types: Vec<String> = field_defs
        .iter()
        .map(super::field_type::FieldDef::typescript_slot_typename)
        .collect();
    let ts_tuple = format!("[{}]", ts_tuple_types.join(", "));

    let tuple_desc: Vec<String> = field_defs
        .iter()
        .enumerate()
        .map(|(i, _)| format!("element {i}"))
        .collect();
    let content_key = ts_member_key(content_name);
    let _ = writeln!(
        variant_type_code,
        "  /** Tuple: [{}] */\n  {}: {};",
        tuple_desc.join(", "),
        content_key,
        ts_tuple
    );

    #[cfg(feature = "zod")]
    {
        let zod_tuple_types: Vec<String> = field_defs
            .iter()
            .map(super::field_type::FieldDef::zod_slot_type)
            .collect();
        let zod_tuple = format!("z.tuple([{}])", zod_tuple_types.join(", "));

        let is_recursive = field_defs
            .iter()
            .any(|fld| fld.contains_type_reference(self_type_name));

        if is_recursive {
            let _ = writeln!(
                variant_schema_code,
                "  get {content_key}() {{ return {zod_tuple}; }},"
            );
        } else {
            let _ = writeln!(variant_schema_code, "  {content_key}: {zod_tuple},");
        }
    }

    #[cfg(not(feature = "zod"))]
    {
        let _: &_ = &(&variant_schema_code, self_type_name);
    }

    #[cfg(feature = "jsonschema")]
    {
        let content_name_str = content_name.to_owned();
        match bounded_slots_json_schema(field_defs) {
            Ok(tuple_schema) => json_schema_variant_fields.push(quote! {
                properties.insert(#content_name_str.to_string(), #tuple_schema);
                required.push(serde_json::Value::String(#content_name_str.to_string()));
            }),
            Err(rejection) => {
                json_schema_variant_fields
                    .push(map_member_rejection_error(content_name, &rejection));
            }
        }
    }
    #[cfg(not(feature = "jsonschema"))]
    let _: &_ = &&json_schema_variant_fields;
}

/// Arrays `item_schema` once per array level the field carries, and hands it back untouched when
/// the field carries none. A level written as a set holds distinct items.
#[cfg(feature = "jsonschema")]
fn arrayed_json_schema_value(
    fld: &FieldDef,
    item_schema: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    (0..fld.array_depth).fold(item_schema, |level_schema, level| {
        let items = if fld.is_nullable_at(level) {
            quote! { serde_json::json!({ "anyOf": [#level_schema, { "type": "null" }] }) }
        } else {
            level_schema
        };
        let distinct = fld
            .is_set_at(level)
            .then(|| quote! { , "uniqueItems": true });
        let bounds = array_length_json_schema_bounds(fld, level);
        quote! { serde_json::json!({ "type": "array", "items": #items #distinct #bounds }) }
    })
}

/// [`arrayed_json_schema_value`] for a caller holding a literal fragment: each wrap nests inside
/// the one `serde_json::json!` the fragment is written into rather than materializing a value.
#[cfg(feature = "jsonschema")]
fn arrayed_json_schema_fragment(
    fld: &FieldDef,
    item_schema: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    (0..fld.array_depth).fold(item_schema.clone(), |level_schema, level| {
        let items = if fld.is_nullable_at(level) {
            quote! { { "anyOf": [#level_schema, { "type": "null" }] } }
        } else {
            level_schema
        };
        let distinct = fld
            .is_set_at(level)
            .then(|| quote! { , "uniqueItems": true });
        let bounds = array_length_json_schema_bounds(fld, level);
        quote! { { "type": "array", "items": #items #distinct #bounds } }
    })
}

/// The `minItems` and `maxItems` of one array level: the arity a fixed-size `[T; N]` pins, as a
/// tuple pins its own, held at the outermost level to the count the field's own `minLength` and
/// `maxLength` wrote. Empty where nothing bounds the level; each keyword carries its own leading
/// comma so emptiness costs the caller nothing.
#[cfg(feature = "jsonschema")]
fn array_length_json_schema_bounds(fld: &FieldDef, level: u8) -> proc_macro2::TokenStream {
    let fixed = fld.fixed_length_at(level);
    let counted = fld
        .model_schema_prop_meta
        .as_ref()
        .filter(|_| level.saturating_add(1) == fld.array_depth);
    let least = fixed.max(counted.and_then(|meta| meta.min_items));
    let most = match (fixed, counted.and_then(|meta| meta.max_items)) {
        (Some(pinned), Some(written)) => Some(pinned.min(written)),
        (pinned, written) => pinned.or(written),
    };
    let min_items = least.map(|count| quote! { , "minItems": #count });
    let max_items = most.map(|count| quote! { , "maxItems": #count });
    quote! { #min_items #max_items }
}

/// Which instant a chrono type's string spells, as the JSON-schema `"format"` keyword that names
/// it, and `None` for every type that writes no such string.
#[cfg(all(feature = "jsonschema", feature = "chrono"))]
const fn chrono_json_schema_format(field_type: &FieldDefType) -> Option<&'static str> {
    match *field_type {
        FieldDefType::NaiveDate => Some("date"),
        FieldDefType::NaiveTime => Some("time"),
        FieldDefType::NaiveDateTime | FieldDefType::DateTime => Some("date-time"),
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::Map(..)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Range(..)
        | FieldDefType::Result(..)
        | FieldDefType::SiblingType(..)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::Tuple(..)
        | FieldDefType::TypeParam(_)
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::Unknown
        | FieldDefType::Usize => None,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => None,
    }
}

/// The JSON schema literal a chrono type describes as — the string it writes, carrying the
/// `"format"` keyword that says which instant it spells — and `None` for every type that writes no
/// such string.
#[cfg(all(feature = "jsonschema", feature = "chrono"))]
fn chrono_json_schema_item(field_type: &FieldDefType) -> Option<proc_macro2::TokenStream> {
    let format = chrono_json_schema_format(field_type)?;
    Some(quote! { { "type": "string", "format": #format } })
}

/// The JSON schema literal for a type that renders inline as a scalar — the object body itself,
/// which a caller writing inside a `serde_json::json!` inlines and one needing a standalone
/// `serde_json::Value` wraps — or `None` for the composite types (sibling references, maps,
/// tuples, unknowns) that have no inline rendering.
#[cfg(feature = "jsonschema")]
fn scalar_field_json_schema_item(fld: &FieldDef) -> Option<proc_macro2::TokenStream> {
    let item_schema = match &fld.field_type {
        FieldDefType::String => quote! { { "type": "string" } },
        // Fixed at 1 rather than read from `model_schema_prop`: a `char` carries none of those
        // constraints, and this is what serde writes for it wherever it stands.
        FieldDefType::Char => quote! { { "type": "string", "minLength": 1, "maxLength": 1 } },
        FieldDefType::StringLiteral(literal) => {
            quote! { { "type": "string", "const": #literal } }
        }
        FieldDefType::BooleanLiteral(value) => {
            quote! { { "type": "boolean", "const": #value } }
        }
        FieldDefType::NumberLiteral(value) => {
            quote! { { "type": "number", "const": #value } }
        }
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::Boolean => {
            // The arm has matched exactly the types the mapping answers a keyword for, so the `?`
            // never fires — it is how the chrono arm beside it reads its own mapping too.
            let keyword = scalar_json_type_keyword(&fld.field_type)?;
            quote! { { "type": #keyword } }
        }
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => chrono_json_schema_item(&fld.field_type)?,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => object_id_json_schema_item(&object_id_hex_json_schema()),
        FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::SiblingType(..)
        | FieldDefType::Map(..)
        | FieldDefType::Range(..)
        | FieldDefType::Result(..)
        | FieldDefType::Tuple(..) => return None,
    };
    Some(item_schema)
}

/// Builds the base JSON schema for a tuple element, ignoring `is_optional`, or `None` when the
/// element holds a value the dispatch cannot render.
#[cfg(feature = "jsonschema")]
fn build_tuple_element_base_json_schema(
    fld: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    let value = normalized_slot_value(fld);
    let item = build_map_member_item(&value)?;
    Ok(arrayed_json_schema_value(&value, item.into_value()))
}

/// The `anyOf [<base>, null]` form for a value that is an `Option`, or `None` when it is not. A
/// slot that cannot be dropped needs it since there's no other way to write `None`; an object key
/// needs it too, since serde reads an explicit `null` into `None` as readily as an absent key.
#[cfg(feature = "jsonschema")]
fn nullable_slot_json_schema(
    fld: &FieldDef,
    base: &proc_macro2::TokenStream,
) -> Option<proc_macro2::TokenStream> {
    fld.is_optional()
        .then(|| quote! { { "anyOf": [#base, { "type": "null" }] } })
}

/// [`nullable_slot_json_schema`] for a caller that needs a standalone `serde_json::Value`: the
/// nullable form is a literal fragment, so it is wrapped, while `base` already is such a value.
#[cfg(feature = "jsonschema")]
fn nullable_slot_json_schema_value(
    fld: &FieldDef,
    base: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    nullable_slot_json_schema(fld, &base)
        .map_or(base, |nullable| quote! { serde_json::json!(#nullable) })
}

/// The schema module a sibling type's `Schema::json_schema()` lives in.
#[cfg(feature = "jsonschema")]
fn sibling_schema_module_ident(name: &str, span: proc_macro2::Span) -> Ident {
    let module_name = match lookup_alias_info(name) {
        Some(alias) => alias.module_name,
        None => ident_schema_module_name(name),
    };
    Ident::new(module_name.as_str(), span)
}

/// `described`, a named type's own schema, narrowed from inside an `allOf` by the length and
/// pattern bounds written on the field holding it, or `described` alone where none was written.
#[cfg(feature = "jsonschema")]
fn named_field_json_schema_value(
    fld: &FieldDef,
    described: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let Some(meta) = &fld.model_schema_prop_meta else {
        return described.clone();
    };
    let mut narrowing: Vec<proc_macro2::TokenStream> = Vec::new();
    if let Some(min_len) = meta.min_length {
        let len = min_len as u64;
        narrowing.push(quote! { "minLength": #len });
    }
    if let Some(max_len) = meta.max_length {
        let len = max_len as u64;
        narrowing.push(quote! { "maxLength": #len });
    }
    if let Some(pattern) = &meta.pattern {
        narrowing.push(quote! { "pattern": #pattern });
    }
    if narrowing.is_empty() {
        return described.clone();
    }
    quote! {
        serde_json::json!({ "allOf": [#described, { #(#narrowing),* }] })
    }
}

/// A sibling's own schema as a standalone `serde_json::Value` expression: the module the reference
/// resolves to, asked for the schema it publishes.
#[cfg(feature = "jsonschema")]
fn sibling_json_schema_value(
    name: &str,
    arguments: &[FieldDef],
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let module_ident = sibling_schema_module_ident(name, span);
    // Named where the run declares the two, not on the field's type: a type handed into a
    // `macro_rules!` body carries its call site's hygiene, under which neither binding is found.
    let run = quote! { in_flight, hoisted_defs };
    if arguments.is_empty() {
        return quote_spanned! {span=> #module_ident::Schema::json_schema_within(#run) };
    }
    let documents = arguments
        .iter()
        .map(|argument| argument_json_schema_value(name, argument));
    // Held in a local: an argument that is itself a reference describes through this same call and
    // borrows the run's two values. Parenthesised for the positions inside a `json!` literal.
    quote_spanned! {span=>
        ({
            let arguments = [#(#documents),*];
            #module_ident::Schema::json_schema_within_with(#run, &arguments)
        })
    }
}

/// One reference-site argument as the document that fills the parameter it stands at.
#[cfg(feature = "jsonschema")]
fn argument_json_schema_value(name: &str, argument: &FieldDef) -> proc_macro2::TokenStream {
    build_tuple_element_json_schema(argument).unwrap_or_else(|rejection| {
        let message = prefixed_guard_message(&map_member_rejection_message(
            &format!("`{name}`"),
            &rejection,
        ));
        syn::Error::new(rejection.span(), message).to_compile_error()
    })
}

/// The local the enclosing item's document reaches one of its own type parameters through, as the
/// `serde_json::Value` expression a position holding that parameter is described by.
#[cfg(feature = "jsonschema")]
fn json_argument_value(parameter: &str) -> proc_macro2::TokenStream {
    let binding = json_argument_ident(parameter);
    quote! { #binding.clone() }
}

/// The local one type parameter's argument document is bound to, as the ident both ends spell it
/// with — the binding the module writes and the reads the body makes of it.
#[cfg(feature = "jsonschema")]
fn json_argument_ident(parameter: &str) -> Ident {
    Ident::new(
        json_argument_binding(parameter).as_str(),
        proc_macro2::Span::call_site(),
    )
}

/// The document a value with no type of its own is described by: the filling of a type parameter,
/// and the permissive empty schema for a value the expansion cannot name at all.
#[cfg(feature = "jsonschema")]
fn opaque_json_schema_value(fld: &FieldDef) -> proc_macro2::TokenStream {
    if let FieldDefType::TypeParam(parameter) = &fld.field_type {
        return json_argument_value(parameter);
    }
    quote! { serde_json::json!({}) }
}

/// The argument slots an item's schema module publishes: one per type parameter it declares, in
/// declaration order, each carrying the document the standalone form fills it with.
#[cfg(feature = "jsonschema")]
fn schema_parameters(generics: &syn::Generics, args: &ModelSchemaArgs) -> Vec<SchemaParameter> {
    type_parameters_in_scope(generics)
        .iter()
        .map(|parameter| SchemaParameter {
            binding: json_argument_ident(parameter),
            default: declared_filling_json_schema_value(parameter, &args.default_types),
        })
        .collect()
}

/// The document a parameter's declared filling describes as, rendered through the dispatch every
/// other type position is rendered through — so the filling describes exactly as a field written
/// with that type describes.
#[cfg(feature = "jsonschema")]
fn declared_filling_json_schema_value(
    parameter: &str,
    default_types: &[(syn::Ident, syn::Type)],
) -> proc_macro2::TokenStream {
    default_types
        .iter()
        .find(|(declared, _)| declared == parameter)
        .map_or_else(
            || quote! { serde_json::json!({}) },
            |(_, filling)| {
                argument_json_schema_value(parameter, &get_field_def(parameter, filling, ""))
            },
        )
}

/// Builds JSON schema for a tuple element (used for tuple fields and for tuple variants), or the
/// rejection when the element holds a value the dispatch cannot render — which the callers turn
/// into the single diagnostic naming the field.
#[cfg(feature = "jsonschema")]
fn build_tuple_element_json_schema(
    fld: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    Ok(nullable_slot_json_schema_value(
        fld,
        build_tuple_element_base_json_schema(fld)?,
    ))
}

/// The `"minimum"` and `"maximum"` a number's own `model_schema_prop` writes, each behind the comma
/// that joins it to the keyword before it, and nothing for any other type.
#[cfg(feature = "jsonschema")]
fn numeric_bound_keywords(fld: &FieldDef) -> proc_macro2::TokenStream {
    let meta = fld
        .model_schema_prop_meta
        .as_ref()
        .filter(|_| fld.field_type.is_numeric());
    let minimum = meta
        .and_then(|bounds| bounds.minimum)
        .map(|bound| quote! { , "minimum": #bound });
    let maximum = meta
        .and_then(|bounds| bounds.maximum)
        .map(|bound| quote! { , "maximum": #bound });
    quote! { #minimum #maximum }
}

/// The value a slot holds under its wrappers, carrying the bounds written on the slot as a named
/// member's schema carries them, or `None` for a slot that writes none.
#[cfg(feature = "jsonschema")]
fn bounded_slot_item(value: &FieldDef) -> Option<proc_macro2::TokenStream> {
    let meta = value.model_schema_prop_meta.as_ref()?;
    let measured = meta.min_length.is_some() || meta.max_length.is_some() || meta.pattern.is_some();
    if let FieldDefType::SiblingType(name, arguments) = &value.field_type {
        let described = sibling_json_schema_value(name, arguments, value.type_span);
        return measured.then(|| named_field_json_schema_value(value, &described));
    }
    if matches!(value.field_type, FieldDefType::String) {
        return measured.then(|| string_field_json_schema_value(value));
    }
    let ranged = meta.minimum.is_some() || meta.maximum.is_some();
    if !(ranged && value.field_type.is_numeric()) {
        return None;
    }
    let keyword = scalar_json_type_keyword(&value.field_type)?;
    let bounds = numeric_bound_keywords(value);
    Some(quote! { serde_json::json!({ "type": #keyword #bounds }) })
}

/// [`build_tuple_element_json_schema`] for a slot of a tuple variant or of a tuple struct, whose
/// own bounds reach its value.
#[cfg(feature = "jsonschema")]
fn bounded_slot_json_schema(
    fld: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    let value = normalized_slot_value(fld);
    let Some(item) = bounded_slot_item(&value) else {
        return build_tuple_element_json_schema(fld);
    };
    Ok(nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(&value, item),
    ))
}

/// The fixed-arity array a tuple describes as — the form serde writes it in — or the rejection when
/// any element holds a value the dispatch cannot render. `()` is the `null` serde writes for it.
#[cfg(feature = "jsonschema")]
fn tuple_json_schema_value(
    elements: &[FieldDef],
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    if elements.is_empty() {
        return Ok(quote! { serde_json::json!({ "type": "null" }) });
    }
    tuple_json_schema_of(elements, build_tuple_element_json_schema)
}

/// The object a std range describes as, the form serde writes it in: each member it writes,
/// required, holding the slot its bound fills. The rejection is the bound's, where the dispatch
/// cannot render it.
#[cfg(feature = "jsonschema")]
fn range_json_schema_value(
    kind: RangeKind,
    bound: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    let held = build_tuple_element_json_schema(bound)?;
    let members = kind.members();
    Ok(quote! {
        serde_json::json!({
            "type": "object",
            "properties": { #(#members: #held),* },
            "required": [#(#members),*],
            "additionalProperties": false
        })
    })
}

/// The objects a `Result` describes as, the form serde writes it in: one per arm, whose one key is
/// the arm's name, required, holding what the arm holds, and `null` for an arm of `()`. The
/// rejection is an arm's, where the dispatch cannot render it.
#[cfg(feature = "jsonschema")]
fn result_json_schema_value(
    ok: &FieldDef,
    err: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    let mut arms = Vec::new();
    for (key, held) in RESULT_ARMS.into_iter().zip([ok, err]) {
        let value = build_tuple_element_json_schema(held)?;
        arms.push(quote! {
            {
                "type": "object",
                "properties": { #key: #value },
                "required": [#key],
                "additionalProperties": false
            }
        });
    }
    Ok(quote! {
        serde_json::json!({ "oneOf": [#(#arms),*] })
    })
}

/// [`tuple_json_schema_value`] for the slots of a tuple variant or of a tuple struct, each
/// carrying its own bounds.
#[cfg(feature = "jsonschema")]
fn bounded_slots_json_schema(
    slots: &[FieldDef],
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    tuple_json_schema_of(slots, bounded_slot_json_schema)
}

/// The fixed-arity array of `elements`, each described by `described`.
#[cfg(feature = "jsonschema")]
fn tuple_json_schema_of(
    elements: &[FieldDef],
    described: fn(&FieldDef) -> Result<proc_macro2::TokenStream, MapMemberRejection>,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    let arity = elements.len();
    let element_schemas = elements
        .iter()
        .map(described)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! {
        serde_json::json!({
            "type": "array",
            "prefixItems": [#(#element_schemas),*],
            "items": false,
            "minItems": #arity,
            "maxItems": #arity
        })
    })
}

/// What a sequence-spelled type holds, and `None` for everything else. A `Vec`/`[T; N]` collapses
/// to array levels; every other sequence wrapper keeps its name, normalized onto its element at
/// render time — both write the same JSON array.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn sequence_wrapper_element(fld: &FieldDef) -> Option<&FieldDef> {
    let FieldDefType::SiblingType(wrapper_name, wrapper_args) = &fld.field_type else {
        return None;
    };
    let [element] = wrapper_args.as_slice() else {
        return None;
    };
    is_sequence_wrapper(wrapper_name).then_some(element)
}

/// The field a sequence-spelled type stands for: its element, carrying the array level the wrapper
/// writes — and `None` for everything else.
#[cfg(feature = "jsonschema")]
fn sequence_wrapper_field(fld: &FieldDef) -> Option<FieldDef> {
    sequence_wrapper_element(fld).map(|element| fld.collection_element_field(element))
}

/// The classification every position reads a map key through.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn map_key_path(key: &FieldDef) -> MapKeyPath<'_> {
    if key.array_depth > 0 || sequence_wrapper_element(key).is_some() {
        return MapKeyPath::Refused(MapKeyRejection::Sequenced(map_key_element_name(key)));
    }
    if key.is_optional() {
        return MapKeyPath::Refused(MapKeyRejection::Optional(map_key_element_name(key)));
    }
    match &key.field_type {
        // A parameter names no type here, so nothing about the key can be enumerated; every
        // instantiation that serializes at all writes its keys as strings.
        FieldDefType::String | FieldDefType::TypeParam(_) => MapKeyPath::Open,
        FieldDefType::SiblingType(key_type_name, args) if args.is_empty() => {
            match registered_key_kind(key_type_name) {
                Some(AliasKind::StringWire) => MapKeyPath::Open,
                Some(AliasKind::Stringified) => MapKeyPath::Unnarrowed,
                _ => MapKeyPath::Enumerated(key_type_name.as_str()),
            }
        }
        FieldDefType::Tuple(elements) if elements.is_empty() => {
            MapKeyPath::Refused(unwritable_key(key, WRITTEN_AS_NULL))
        }
        FieldDefType::Tuple(..) => MapKeyPath::Refused(unwritable_key(key, WRITTEN_AS_ARRAY)),
        FieldDefType::Map(..) | FieldDefType::Range(..) | FieldDefType::Result(..) => {
            MapKeyPath::Refused(unwritable_key(key, WRITTEN_AS_OBJECT))
        }
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => MapKeyPath::Refused(unwritable_key(key, WRITTEN_AS_OBJECT)),
        FieldDefType::SiblingType(..)
        | FieldDefType::Unknown
        | FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::StringLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64 => MapKeyPath::Unnarrowed,
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => MapKeyPath::Unnarrowed,
    }
}

/// The rejection a key earns for its own wire form, naming the key as written and what serde writes
/// for it instead.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn unwritable_key(key: &FieldDef, written_as: &'static str) -> MapKeyRejection {
    MapKeyRejection::Unwritable {
        key_name: map_key_element_name(key),
        written_as,
    }
}

/// Why this key has no rendering, and `None` for every key that may still have one. A key the
/// dispatch already refuses is ruled out by its own spelling; otherwise only a target the registry
/// positively rules out is named — an unregistered or not-yet-expanded name is left alone.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn key_rejection(key: &FieldDef) -> Option<MapKeyRejection> {
    match map_key_path(key) {
        MapKeyPath::Refused(rejection) => Some(rejection),
        MapKeyPath::Enumerated(key_type_name) => proves_no_enum_members(key_type_name)
            .then(|| MapKeyRejection::NoEnumMembers(key_type_name.to_owned())),
        MapKeyPath::Open | MapKeyPath::Unnarrowed => None,
    }
}

/// The Rust spelling of what a map key holds, for the diagnostic a key with no rendering earns.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn map_key_element_name(key: &FieldDef) -> String {
    if let Some(element) = sequence_wrapper_element(key) {
        return map_key_element_name(element);
    }
    match &key.field_type {
        FieldDefType::SiblingType(key_type_name, _) | FieldDefType::TypeParam(key_type_name) => {
            key_type_name.clone()
        }
        FieldDefType::String | FieldDefType::StringLiteral(_) => "String".to_owned(),
        FieldDefType::Boolean | FieldDefType::BooleanLiteral(_) => "bool".to_owned(),
        FieldDefType::NumberLiteral(_) => "f64".to_owned(),
        FieldDefType::Char => "char".to_owned(),
        FieldDefType::U8 => "u8".to_owned(),
        FieldDefType::U16 => "u16".to_owned(),
        FieldDefType::U32 => "u32".to_owned(),
        FieldDefType::U64 => "u64".to_owned(),
        FieldDefType::I8 => "i8".to_owned(),
        FieldDefType::I16 => "i16".to_owned(),
        FieldDefType::I32 => "i32".to_owned(),
        FieldDefType::I64 => "i64".to_owned(),
        FieldDefType::Usize => "usize".to_owned(),
        FieldDefType::Isize => "isize".to_owned(),
        FieldDefType::F32 => "f32".to_owned(),
        FieldDefType::F64 => "f64".to_owned(),
        FieldDefType::Map(..) => "HashMap<_, _>".to_owned(),
        FieldDefType::Range(kind, _) => format!("{}<_>", kind.name()),
        FieldDefType::Result(..) => "Result<_, _>".to_owned(),
        FieldDefType::Tuple(elements) if elements.is_empty() => "()".to_owned(),
        FieldDefType::Tuple(..) => "(_, _)".to_owned(),
        // A type written as no path, a function pointer or a macro among them.
        FieldDefType::Unknown => "_".to_owned(),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => "ObjectId".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => "DateTime".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate => "NaiveDate".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDateTime => "NaiveDateTime".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveTime => "NaiveTime".to_owned(),
    }
}

/// Whether the registry rules the named type out as a source of `enum_members()`. `false` covers
/// both the plain enums that have them and the names this expansion never saw registered.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn proves_no_enum_members(key_type_name: &str) -> bool {
    lookup_alias_info(key_type_name).is_some_and(|key_alias| {
        matches!(
            key_alias.kind,
            AliasKind::NoEnumMembers | AliasKind::StringWire | AliasKind::Stringified
        )
    })
}

/// What the registry says serde writes for a key spelled by this name, and `None` for a name it
/// never saw registered — indistinguishable from a plain enum declared later, which is why the
/// dispatch above leaves such a key on the enumerating path.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn registered_key_kind(key_type_name: &str) -> Option<AliasKind> {
    lookup_alias_info(key_type_name).map(|key_alias| key_alias.kind)
}

/// Why the first map key this field reaches, at any depth, has no rendering.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn map_key_rejection(fld: &FieldDef) -> Option<MapKeyRejection> {
    match &fld.field_type {
        FieldDefType::Map(key, value) => key_rejection(key)
            .or_else(|| map_key_rejection(key))
            .or_else(|| map_key_rejection(value)),
        FieldDefType::SiblingType(_, generics) => generics.iter().find_map(map_key_rejection),
        FieldDefType::Range(_, bound) => map_key_rejection(bound),
        FieldDefType::Result(ok, err) => map_key_rejection(ok).or_else(|| map_key_rejection(err)),
        FieldDefType::Tuple(elements) => elements.iter().find_map(map_key_rejection),
        FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::StringLiteral(_)
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Boolean
        | FieldDefType::Char
        | FieldDefType::String
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64 => None,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => None,
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => None,
    }
}

/// What a key with no members is reported as. The `subject` names where the map was written — a
/// field, an alias — which is what the author can act on and all that differs between them.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn non_enum_map_key_message(subject: &str, key_type_name: &str) -> String {
    format!(
        "{subject}: a map key must be a plain `#[model_schema()]` enum, whose members become the object's keys — `{key_type_name}` resolves to a type with no `enum_members()`"
    )
}

/// What a sequence-wrapped key is reported as, worded like its sibling above and carrying the same
/// `subject`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn sequenced_map_key_message(subject: &str, element_name: &str) -> String {
    format!(
        "{subject}: a map key must be a value serde writes as a string, which is what a JSON object key is — this key is a sequence of `{element_name}`, and serde refuses to serialize a map keyed by one at all"
    )
}

/// What an `Option`-wrapped key is reported as, worded like its siblings above and carrying the same
/// `subject`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn optional_map_key_message(subject: &str, inner_name: &str) -> String {
    format!(
        "{subject}: a map key must be a value serde writes as a string, which is what a JSON object key is — this key is an `Option<{inner_name}>`, whose `Some` serde writes as the bare `{inner_name}` while a `None` has no string form at all and makes serde refuse the whole map; key it by `{inner_name}`"
    )
}

/// What a key serde writes as neither a string nor anything it stringifies for the author is
/// reported as, worded like its siblings above and carrying the same `subject`.
fn unwritable_map_key_message(subject: &str, key_name: &str, written_as: &str) -> String {
    format!(
        "{subject}: a map key must be a value serde writes as a string, which is what a JSON object key is — serde writes `{key_name}` as {written_as}, and refuses to serialize a map keyed by one at all"
    )
}

/// What a map key with no rendering is reported as, whichever reason it has.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn map_key_rejection_message(subject: &str, rejection: &MapKeyRejection) -> String {
    match rejection {
        MapKeyRejection::NoEnumMembers(key_type_name) => {
            non_enum_map_key_message(subject, key_type_name)
        }
        MapKeyRejection::Sequenced(element_name) => {
            sequenced_map_key_message(subject, element_name)
        }
        MapKeyRejection::Optional(inner_name) => optional_map_key_message(subject, inner_name),
        MapKeyRejection::Unwritable {
            key_name,
            written_as,
        } => unwritable_map_key_message(subject, key_name, written_as),
    }
}

/// Rejects a field reaching a map key no surface can write.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn check_map_key(field: &Field, field_def: &FieldDef, label: &str) -> Result<(), syn::Error> {
    let Some(rejection) = map_key_rejection(field_def) else {
        return Ok(());
    };
    Err(syn::Error::new_spanned(
        field,
        prefixed_guard_message(&map_key_rejection_message(label, &rejection)),
    ))
}

/// The object a map whose key serde writes as text of its own describes as, as a standalone
/// `serde_json::Value` expression, or the rejection when the value type has no rendering here: the
/// value's rendering for every member, and the keys by [`text_key_names`] where it states them.
#[cfg(feature = "jsonschema")]
fn text_key_map_json_schema_value(
    key: &FieldDef,
    value: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    let value_schema = build_map_member_schema(value)?;
    Ok(text_key_names(key).map_or_else(
        || {
            quote! {
                serde_json::json!({ "type": "object", "additionalProperties": #value_schema })
            }
        },
        |names| {
            quote! {
                serde_json::json!({
                    "type": "object",
                    "additionalProperties": #value_schema,
                    "propertyNames": (#names)
                })
            }
        },
    ))
}

/// The rendering a map whose value is itself a map carries, dispatched on the inner key exactly as
/// the field position dispatches on the outer one.
#[cfg(feature = "jsonschema")]
fn build_nested_map_member_item(
    inner_key: &FieldDef,
    inner_value: &FieldDef,
) -> Result<MapMemberItem, MapMemberRejection> {
    log::trace!(
        "Map Value is another Map => inner_key: {inner_key:?}, inner_value: {inner_value:?}"
    );

    Ok(match map_key_path(inner_key) {
        MapKeyPath::Enumerated(key_type_name) => MapMemberItem::Value(
            enum_key_map_json_schema_value(key_type_name, inner_key.type_span, inner_value)?,
        ),
        MapKeyPath::Open => {
            let inner_member = build_map_member_schema(inner_value)?;
            MapMemberItem::Fragment(
                quote! { { "type": "object", "additionalProperties": #inner_member } },
            )
        }
        MapKeyPath::Unnarrowed => {
            MapMemberItem::Value(text_key_map_json_schema_value(inner_key, inner_value)?)
        }
        MapKeyPath::Refused(rejection) => {
            return Err(MapMemberRejection::Key(rejection, inner_key.type_span));
        }
    })
}

/// Wraps a member's base schema for the slot it sits in — arrayed once per array level the value
/// carries, nullable when it is an `Option`.
#[cfg(feature = "jsonschema")]
fn map_member_slot_schema(
    value: &FieldDef,
    item_schema: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let arrayed = arrayed_json_schema_fragment(value, item_schema);
    nullable_slot_json_schema(value, &arrayed).unwrap_or(arrayed)
}

/// [`map_member_slot_schema`] for a member already materialized as a `serde_json::Value`: each wrap
/// materializes a value of its own instead of nesting inside the one `serde_json::json!` a literal
/// fragment sits in.
#[cfg(feature = "jsonschema")]
fn map_member_slot_value(
    value: &FieldDef,
    item_value: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    nullable_slot_json_schema_value(value, arrayed_json_schema_value(value, item_value))
}

/// The `FieldDef` a value in a slot — a map member, a tuple element — dispatches as.
#[cfg(feature = "jsonschema")]
fn normalized_slot_value(value: &FieldDef) -> FieldDef {
    let mut normalized = value.clone();
    while let Some(element_field) = sequence_wrapper_field(&normalized) {
        normalized = element_field;
    }
    normalized
}

/// The rendering a value in a slot carries, with the slot wraps left to the caller, or the
/// rejection when the value type has no rendering here.
#[cfg(feature = "jsonschema")]
fn build_map_member_item(value: &FieldDef) -> Result<MapMemberItem, MapMemberRejection> {
    Ok(match &value.field_type {
        FieldDefType::Map(inner_key, inner_value) => {
            build_nested_map_member_item(inner_key, inner_value)?
        }
        // The member is the sibling's own schema, as it is in field position — an expression, not a
        // literal, so it is already the value form.
        FieldDefType::SiblingType(value_type_name, value_args) => {
            log::trace!(
                "Slot SiblingType => value_type_name: {value_type_name}, value_args: {value_args:?}"
            );
            MapMemberItem::Value(sibling_json_schema_value(
                value_type_name,
                value_args,
                value.type_span,
            ))
        }
        // The one `$oid` object every position spells, which a member carries as written.
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => {
            MapMemberItem::Fragment(object_id_json_schema_item(&object_id_hex_json_schema()))
        }
        // A parameter is the filling that reached it, and a value the expansion cannot name at all
        // admits any value — the permissive empty schema, as in field position.
        FieldDefType::TypeParam(parameter) => MapMemberItem::Value(json_argument_value(parameter)),
        FieldDefType::Range(kind, bound) => {
            MapMemberItem::Value(range_json_schema_value(*kind, bound)?)
        }
        FieldDefType::Result(ok, err) => MapMemberItem::Value(result_json_schema_value(ok, err)?),
        // A type written as no path, a function pointer or a macro among them.
        FieldDefType::Unknown => MapMemberItem::Fragment(quote! { {} }),
        // The fixed-arity array its own field position describes, and `null` for `()`.
        FieldDefType::Tuple(elements) => MapMemberItem::Value(tuple_json_schema_value(elements)?),
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::Usize => MapMemberItem::Fragment(scalar_slot_item(value)),
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => MapMemberItem::Fragment(scalar_slot_item(value)),
    })
}

/// [`scalar_field_json_schema_item`] for a slot, which [`build_map_member_item`] hands only the
/// types the scalar mapping renders.
#[cfg(feature = "jsonschema")]
fn scalar_slot_item(fld: &FieldDef) -> proc_macro2::TokenStream {
    scalar_field_json_schema_item(fld).unwrap()
}

/// The `additionalProperties` schema every member of a `String`-keyed map carries, or the rejection
/// when the value type has no rendering here.
#[cfg(feature = "jsonschema")]
fn build_map_member_schema(
    value: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    let normalized = normalized_slot_value(value);
    Ok(build_map_member_item(&normalized)?.into_member_schema(&normalized))
}

/// What a value the mapping cannot render is reported as. The `subject` names where the value was
/// written — a field, an alias — which is what the author can act on and all that differs between
/// those positions, so the reasons are worded once.
#[cfg(feature = "jsonschema")]
fn map_member_rejection_message(subject: &str, rejection: &MapMemberRejection) -> String {
    match rejection {
        MapMemberRejection::Key(key_rejection, _) => {
            map_key_rejection_message(subject, key_rejection)
        }
    }
}

/// The one diagnostic a slot the mapping cannot render produces — on either key path, at any depth,
/// and in a tuple slot the value is reached through.
#[cfg(feature = "jsonschema")]
fn map_member_rejection_error(
    field_name_str: &str,
    rejection: &MapMemberRejection,
) -> proc_macro2::TokenStream {
    let message = prefixed_guard_message(&map_member_rejection_message(
        &format!("field `{field_name_str}`"),
        rejection,
    ));
    syn::Error::new(rejection.span(), message).to_compile_error()
}

/// The object a `String`-keyed map describes as, as a standalone `serde_json::Value` expression, or
/// the rejection when the value type has no rendering here. A `String` key enumerates nothing, so
/// one `additionalProperties` schema — the value's own rendering — stands for every member.
#[cfg(feature = "jsonschema")]
fn string_key_map_json_schema_value(
    value: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    let value_schema = build_map_member_schema(value)?;
    Ok(quote! {
        serde_json::json!({
            "type": "object",
            "additionalProperties": #value_schema
        })
    })
}

/// The object an enum-keyed map describes as, as a standalone `serde_json::Value` expression: one
/// property per member the key enumerates, each carrying the value type's own rendering, and closed
/// to every other key. `Err` when the value has no rendering here, or when the registry proves the
/// key carries no members.
#[cfg(feature = "jsonschema")]
fn enum_key_map_json_schema_value(
    key_type_name: &str,
    key_span: proc_macro2::Span,
    value: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    if proves_no_enum_members(key_type_name) {
        return Err(MapMemberRejection::Key(
            MapKeyRejection::NoEnumMembers(key_type_name.to_owned()),
            key_span,
        ));
    }

    let normalized = normalized_slot_value(value);
    let member_value = build_map_member_item(&normalized)?.into_member_value(&normalized);
    let key_type_name_ident = Ident::new(key_type_name, key_span);
    let key_members = quote_spanned! {key_span=> #key_type_name_ident::enum_members() };
    Ok(quote! {
        serde_json::json!({
            "type": "object",
            "properties": ({
                let value_schema = #member_value;
                let mut map_properties = serde_json::Map::new();
                for enum_key in #key_members {
                    map_properties.insert(enum_key.to_string(), value_schema.clone());
                }
                map_properties
            }),
            "additionalProperties": false
        })
    })
}

/// The object a map describes as, as a standalone `serde_json::Value` expression, dispatched on the
/// classification its key earns — or the rejection when the key has no rendering here, or the value
/// none the member dispatch can write.
#[cfg(feature = "jsonschema")]
fn map_json_schema_value(
    key: &FieldDef,
    value: &FieldDef,
) -> Result<proc_macro2::TokenStream, MapMemberRejection> {
    match map_key_path(key) {
        MapKeyPath::Enumerated(key_type_name) => {
            enum_key_map_json_schema_value(key_type_name, key.type_span, value)
        }
        MapKeyPath::Open => string_key_map_json_schema_value(value),
        MapKeyPath::Unnarrowed => text_key_map_json_schema_value(key, value),
        MapKeyPath::Refused(rejection) => Err(MapMemberRejection::Key(rejection, key.type_span)),
    }
}

/// The `properties` insertion a map-typed field produces.
#[cfg(feature = "jsonschema")]
fn build_map_field_schema(
    fld: &FieldDef,
    key: &FieldDef,
    value: &FieldDef,
    field_name_str: &str,
) -> proc_macro2::TokenStream {
    log::trace!("Map => field_name: {field_name_str}, key: {key:?}, value: {value:?}");

    match map_json_schema_value(key, value) {
        Ok(map_schema) => {
            let field_schema =
                nullable_slot_json_schema_value(fld, arrayed_json_schema_value(fld, map_schema));
            quote! {
                properties.insert(#field_name_str.to_string(), #field_schema);
            }
        }
        Err(rejection) => map_member_rejection_error(field_name_str, &rejection),
    }
}

/// Builds the JSON schema for a `String` field, applying any length/pattern constraints.
#[cfg(feature = "jsonschema")]
fn build_string_field_schema(fld: &FieldDef, field_name_str: &str) -> proc_macro2::TokenStream {
    let min_len_opt = fld
        .model_schema_prop_meta
        .as_ref()
        .and_then(|m| m.min_length);
    let max_len_opt = fld
        .model_schema_prop_meta
        .as_ref()
        .and_then(|m| m.max_length);
    let pattern_opt = fld
        .model_schema_prop_meta
        .as_ref()
        .and_then(|m| m.pattern.as_deref().map(str::to_owned));

    let min_len_insert = min_len_opt.map(|min_len| {
        quote! { schema_obj.insert("minLength".to_string(), serde_json::json!(#min_len)); }
    });
    let max_len_insert = max_len_opt.map(|max_len| {
        quote! { schema_obj.insert("maxLength".to_string(), serde_json::json!(#max_len)); }
    });
    let pattern_insert = both_patterns_json(fld).map_or_else(
        || {
            pattern_opt.as_ref().map(|pattern| {
                quote! { schema_obj.insert("pattern".to_string(), serde_json::json!(#pattern)); }
            })
        },
        |both| Some(quote! { schema_obj.insert("allOf".to_string(), #both); }),
    );

    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(fld, quote! { serde_json::Value::Object(schema_obj) }),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert("type".to_string(), serde_json::json!("string"));
            #min_len_insert
            #max_len_insert
            #pattern_insert
            #schema
        });
    }
}

/// Builds the JSON schema for a string literal field (`const` value).
#[cfg(feature = "jsonschema")]
fn build_string_literal_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    literal: &str,
) -> proc_macro2::TokenStream {
    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(
            fld,
            quote! { serde_json::json!({ "type": "string", "const": #literal }) },
        ),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), { #schema });
    }
}

/// Builds the JSON schema for a boolean literal field (`const` value).
#[cfg(feature = "jsonschema")]
fn build_boolean_literal_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    value: bool,
) -> proc_macro2::TokenStream {
    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(
            fld,
            quote! { serde_json::json!({ "type": "boolean", "const": #value }) },
        ),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), { #schema });
    }
}

/// Builds the JSON schema for a numeric literal field (`const` value).
#[cfg(feature = "jsonschema")]
fn build_number_literal_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    value: f64,
) -> proc_macro2::TokenStream {
    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(
            fld,
            quote! { serde_json::json!({ "type": "number", "const": #value }) },
        ),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), { #schema });
    }
}

/// Builds the JSON schema for a numeric field (`integer` or `number`), applying min/max constraints.
#[cfg(feature = "jsonschema")]
fn build_numeric_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    json_type: &str,
) -> proc_macro2::TokenStream {
    let minimum_opt = fld.model_schema_prop_meta.as_ref().and_then(|m| m.minimum);
    let maximum_opt = fld.model_schema_prop_meta.as_ref().and_then(|m| m.maximum);
    let minimum_insert = minimum_opt.map(|min| {
        quote! { schema_obj.insert("minimum".to_string(), serde_json::json!(#min)); }
    });
    let maximum_insert = maximum_opt.map(|max| {
        quote! { schema_obj.insert("maximum".to_string(), serde_json::json!(#max)); }
    });

    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(fld, quote! { serde_json::Value::Object(schema_obj) }),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert("type".to_string(), serde_json::json!(#json_type));
            #minimum_insert
            #maximum_insert
            #schema
        });
    }
}

/// Builds the JSON schema for a `bool` field.
#[cfg(feature = "jsonschema")]
fn build_boolean_field_schema(fld: &FieldDef, field_name_str: &str) -> proc_macro2::TokenStream {
    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(fld, quote! { serde_json::json!({ "type": "boolean" }) }),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), { #schema });
    }
}

/// Builds the JSON schema for a `char` field: the one-character string serde writes for it, with
/// `minLength`/`maxLength` fixed at 1 rather than read from `model_schema_prop` — a `char` field
/// carries none of those constraints.
#[cfg(feature = "jsonschema")]
fn build_char_field_schema(fld: &FieldDef, field_name_str: &str) -> proc_macro2::TokenStream {
    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(
            fld,
            quote! { serde_json::json!({ "type": "string", "minLength": 1, "maxLength": 1 }) },
        ),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), { #schema });
    }
}

/// Builds the JSON schema for a `SiblingType` field (references to other generated types).
#[cfg(feature = "jsonschema")]
fn build_sibling_type_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    name: &str,
    lst: &[FieldDef],
) -> proc_macro2::TokenStream {
    log::trace!("SiblingType => name: {name}, lst: {lst:?}");
    // The element is dispatched as the arrayed field it stands for, so a sequence wrapper renders
    // exactly as the `Vec` of the same element does — element by element, at every type.
    if let Some(element_field) = sequence_wrapper_field(fld) {
        return build_field_type_schema(&element_field, field_name_str);
    }

    // Every remaining shape is carried by the named type's own schema module. A map is not among
    // them: the parser claims both map idents before the sibling fallback.
    generate_type_schema(
        fld,
        field_name_str,
        &named_field_json_schema_value(fld, &sibling_json_schema_value(name, lst, fld.type_span)),
    )
}

/// The `json!` literal an `ObjectId` describes as — the closed `$oid` object serde writes — with
/// `hex_schema` as the schema of the hex string it holds.
#[cfg(all(feature = "jsonschema", feature = "mongodb"))]
fn object_id_json_schema_item(hex_schema: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! { {
        "type": "object",
        "properties": {
            "$oid": (#hex_schema)
        },
        "required": ["$oid"],
        "additionalProperties": false
    } }
}

/// [`object_id_json_schema_item`] as a standalone `serde_json::Value` expression, for the positions
/// that hold the `$oid` object as a value rather than writing it into a literal.
#[cfg(all(feature = "jsonschema", feature = "mongodb"))]
fn object_id_json_schema_value(hex_schema: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let item = object_id_json_schema_item(hex_schema);
    quote! { serde_json::json!(#item) }
}

/// The hex string an `ObjectId`'s `$oid` member holds, where no brand narrows it further.
#[cfg(all(feature = "jsonschema", feature = "mongodb"))]
fn object_id_hex_json_schema() -> proc_macro2::TokenStream {
    quote! { serde_json::json!({ "type": "string", "pattern": #OBJECT_ID_HEX_PATTERN }) }
}

/// Builds the JSON schema for a `MongoDB` `ObjectId` field (`{ "$oid": string }`).
#[cfg(all(feature = "jsonschema", feature = "mongodb"))]
fn build_object_id_field_schema(fld: &FieldDef, field_name_str: &str) -> proc_macro2::TokenStream {
    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(
            fld,
            object_id_json_schema_value(&object_id_hex_json_schema()),
        ),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), { #schema });
    }
}

/// Builds the JSON schema for a string field with a specific `format` (e.g. date/time/date-time).
#[cfg(all(feature = "jsonschema", feature = "chrono"))]
fn build_string_format_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    format: &str,
) -> proc_macro2::TokenStream {
    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(
            fld,
            quote! { serde_json::json!({ "type": "string", "format": #format }) },
        ),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), { #schema });
    }
}

/// Builds JSON schema for a tuple struct field.
#[cfg(feature = "jsonschema")]
fn build_tuple_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    lst: &[FieldDef],
) -> proc_macro2::TokenStream {
    let tuple_schema = match tuple_json_schema_value(lst) {
        Ok(tuple_schema) => tuple_schema,
        Err(rejection) => return map_member_rejection_error(field_name_str, &rejection),
    };

    let schema = nullable_slot_json_schema_value(fld, arrayed_json_schema_value(fld, tuple_schema));
    quote! {
        properties.insert(#field_name_str.to_string(), #schema);
    }
}

/// Builds the JSON schema for a std range field.
#[cfg(feature = "jsonschema")]
fn build_range_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    kind: RangeKind,
    bound: &FieldDef,
) -> proc_macro2::TokenStream {
    let range_schema = match range_json_schema_value(kind, bound) {
        Ok(range_schema) => range_schema,
        Err(rejection) => return map_member_rejection_error(field_name_str, &rejection),
    };
    let schema = nullable_slot_json_schema_value(fld, arrayed_json_schema_value(fld, range_schema));
    quote! {
        properties.insert(#field_name_str.to_string(), #schema);
    }
}

/// Builds the JSON schema for a std `Result` field.
#[cfg(feature = "jsonschema")]
fn build_result_field_schema(
    fld: &FieldDef,
    field_name_str: &str,
    ok: &FieldDef,
    err: &FieldDef,
) -> proc_macro2::TokenStream {
    let result_schema = match result_json_schema_value(ok, err) {
        Ok(result_schema) => result_schema,
        Err(rejection) => return map_member_rejection_error(field_name_str, &rejection),
    };
    let schema =
        nullable_slot_json_schema_value(fld, arrayed_json_schema_value(fld, result_schema));
    quote! {
        properties.insert(#field_name_str.to_string(), #schema);
    }
}

/// Builds the JSON schema for a field with no type of its own — one of the enclosing item's type
/// parameters, or an opaque value: a function pointer, or any other type written as no path.
#[cfg(feature = "jsonschema")]
fn build_unknown_field_schema(fld: &FieldDef, field_name_str: &str) -> proc_macro2::TokenStream {
    log::trace!("Unknown => field_name: {field_name_str}, fld: {fld:?}");

    let schema = nullable_slot_json_schema_value(
        fld,
        arrayed_json_schema_value(fld, opaque_json_schema_value(fld)),
    );
    quote! {
        properties.insert(#field_name_str.to_string(), #schema);
    }
}

/// The `properties` insertion a field's type produces, without the `required` push. The name is
/// passed rather than read off `fld`: a collection element is dispatched through here standing in
/// for the field it is the element of.
#[cfg(feature = "jsonschema")]
fn build_field_type_schema(fld: &FieldDef, field_name_str: &str) -> proc_macro2::TokenStream {
    let field_type = &fld.field_type;

    match field_type {
        FieldDefType::String => build_string_field_schema(fld, field_name_str),
        FieldDefType::StringLiteral(literal) => {
            build_string_literal_field_schema(fld, field_name_str, literal)
        }
        FieldDefType::BooleanLiteral(value) => {
            build_boolean_literal_field_schema(fld, field_name_str, *value)
        }
        FieldDefType::NumberLiteral(value) => {
            build_number_literal_field_schema(fld, field_name_str, *value)
        }
        FieldDefType::U32
        | FieldDefType::U16
        | FieldDefType::U8
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64 => {
            // The arm has matched exactly the types the mapping answers a keyword for.
            let keyword = scalar_json_type_keyword(field_type).unwrap();
            build_numeric_field_schema(fld, field_name_str, keyword)
        }
        FieldDefType::Boolean => build_boolean_field_schema(fld, field_name_str),
        FieldDefType::Char => build_char_field_schema(fld, field_name_str),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => build_object_id_field_schema(fld, field_name_str),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => {
            // The arm has matched exactly the types the mapping answers for.
            let format = chrono_json_schema_format(&fld.field_type).unwrap();
            build_string_format_field_schema(fld, field_name_str, format)
        }
        FieldDefType::SiblingType(name, lst) => {
            build_sibling_type_field_schema(fld, field_name_str, name, lst)
        }
        FieldDefType::Map(key, value) => build_map_field_schema(fld, key, value, field_name_str),
        FieldDefType::Tuple(lst) => build_tuple_field_schema(fld, field_name_str, lst),
        FieldDefType::Range(kind, bound) => {
            build_range_field_schema(fld, field_name_str, *kind, bound)
        }
        FieldDefType::Result(ok, err) => build_result_field_schema(fld, field_name_str, ok, err),
        // Named exhaustively rather than caught by a wildcard: a new variant must be given a
        // schema here, not silently routed to whatever the last arm happens to emit.
        FieldDefType::TypeParam(_) | FieldDefType::Unknown => {
            build_unknown_field_schema(fld, field_name_str)
        }
    }
}

/// Builds JSON schema for a field.
#[cfg(feature = "jsonschema")]
fn build_field_schema(fld: &FieldDef) -> proc_macro2::TokenStream {
    let field_name_str = fld.name.clone();
    let schema_code = build_field_type_schema(fld, &field_name_str);

    let required_code = if fld.key_is_required() {
        quote! {
            required.push(serde_json::Value::String(#field_name_str.to_string()));
        }
    } else {
        quote! {}
    };
    let default_code = default_json_insert(fld);

    quote! {
        #schema_code
        #required_code
        #default_code
    }
}

/// The statements that state, in the member `fld` the local `properties` holds, the default serde
/// reads for its absent key, and nothing for a member read with no default.
#[cfg(feature = "jsonschema")]
fn default_json_insert(fld: &FieldDef) -> Option<proc_macro2::TokenStream> {
    let field_name_str = &fld.name;
    fld.serde_default.map(|at| {
        let index = proc_macro2::Literal::usize_unsuffixed(at);
        quote! {
            if let ::core::option::Option::Some(serde_json::Value::Object(described)) =
                properties.get_mut(#field_name_str)
            {
                described.insert(
                    "default".to_string(),
                    SerdeDefaults::literals()[#index].1.clone(),
                );
            }
        }
    })
}

/// Writes the TypeScript type and conditionally Zod schema for a field to the provided buffers.
/// `self_type_name` detects recursive type references, which get JavaScript getter syntax to
/// defer them and avoid "use before declaration" errors.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn write_field_type_and_schema(
    type_code: &mut String,
    fld: &FieldDef,
    self_type_name: Option<&str>,
) -> String {
    let key = ts_member_key(&fld.name);

    let _ = writeln!(
        type_code,
        "{}\n  {}{}: {};",
        member_jsdoc_block(&fld.docs),
        key,
        fld.optional_key_marker(),
        fld.typescript_typename()
    );

    #[cfg(feature = "zod")]
    {
        let zod_type = fld.zod_type();

        // A reference back to the item being defined, and one forward to an item declared below it,
        // are the two a value cannot be read for while this literal is being built.
        let defer = self_type_name.is_some_and(|name| fld.contains_type_reference(name))
            || fld.reaches_a_type_declared_later();

        if defer {
            format!("  get {key}() {{ return {zod_type}; }},\n")
        } else {
            format!("  {key}: {zod_type},\n")
        }
    }

    #[cfg(not(feature = "zod"))]
    {
        // When zod feature is disabled, there is no schema fragment to emit
        let _: &_ = &self_type_name;
        let _: &str = &key;
        String::new()
    }
}

/// The binding a walk step introduces, numbered by depth so no two steps of one chain collide.
fn wrap_binding(depth: usize) -> proc_macro2::Ident {
    proc_macro2::Ident::new(&format!("value_{depth}"), proc_macro2::Span::call_site())
}

/// The binding a list level of a walk holds its item's index under, numbered as the item is.
#[cfg(feature = "serde")]
fn index_binding(depth: usize) -> proc_macro2::Ident {
    proc_macro2::Ident::new(&format!("index_{depth}"), proc_macro2::Span::call_site())
}

/// The binding a map level of a walk holds its entry's key under, numbered as the value is.
#[cfg(feature = "serde")]
fn entry_binding(depth: usize) -> proc_macro2::Ident {
    proc_macro2::Ident::new(&format!("entry_{depth}"), proc_macro2::Span::call_site())
}

/// The binding the text serde writes for that key is held under, where a report is written.
#[cfg(feature = "serde")]
fn key_binding(depth: usize) -> proc_macro2::Ident {
    proc_macro2::Ident::new(&format!("key_{depth}"), proc_macro2::Span::call_site())
}

/// The name a variant's constrained member is bound under in the arm that matched it.
fn member_binding(field_ident: &proc_macro2::Ident) -> proc_macro2::Ident {
    proc_macro2::Ident::new(
        &format!("member_{}", field_ident.unraw()),
        proc_macro2::Span::call_site(),
    )
}

/// The same, for a positional slot, numbered by declaration position — the spelling a tuple slot
/// has in place of the field ident a named member is bound under.
fn positional_member_binding(index: usize) -> proc_macro2::Ident {
    proc_macro2::Ident::new(
        &format!("member_slot_{index}"),
        proc_macro2::Span::call_site(),
    )
}

/// The expression a check reads its value from, in whichever position the member was written.
fn member_access_expr(
    access: MemberAccess,
    field_ident_tok: &proc_macro2::Ident,
) -> proc_macro2::TokenStream {
    match access {
        MemberAccess::SelfField => quote! { &self.#field_ident_tok },
        MemberAccess::VariantBinding => {
            let binding = member_binding(field_ident_tok);
            quote! { #binding }
        }
    }
}

/// [`member_access_expr`] for a positional slot, which goes by its place in the declaration.
fn slot_access_expr(access: MemberAccess, index: usize) -> proc_macro2::TokenStream {
    match access {
        MemberAccess::SelfField => {
            let slot = syn::Index::from(index);
            quote! { &self.#slot }
        }
        MemberAccess::VariantBinding => {
            let binding = positional_member_binding(index);
            quote! { #binding }
        }
    }
}

/// Builds the `validate()` contribution for a field, reaching through its wrappers to run the
/// check on the value the constraint actually describes.
#[cfg(feature = "serde")]
fn build_field_validation(
    wraps: &[ConstraintWrap],
    checked: &proc_macro2::TokenStream,
    validate_value_fn_ident: &proc_macro2::Ident,
) -> proc_macro2::TokenStream {
    if wraps.is_empty() {
        return quote! {
            if let ::core::result::Result::Err(reported) = #validate_value_fn_ident(#checked) {
                errors.extend(reported);
            }
        };
    }
    let head = wrap_binding(0);
    let leaf = constraint_leaf(
        CheckSink::Collect,
        &walked_value(wraps),
        validate_value_fn_ident,
    );
    let walk = walk_wraps(wraps, &head, 1, &leaf);
    quote! {
        {
            let #head = #checked;
            #walk
        }
    }
}

/// The binding a walk over `wraps` hands its leaf: the head is depth zero and each wrapper
/// introduces one more, so the last step's binding is named from the count.
#[cfg(feature = "serde")]
fn walked_value(wraps: &[ConstraintWrap]) -> proc_macro2::Ident {
    wrap_binding(wraps.len())
}

/// The constraint check a walk ends on, written for whichever of the two ends takes it.
#[cfg(feature = "serde")]
fn constraint_leaf(
    sink: CheckSink,
    value: &proc_macro2::Ident,
    validate_value_fn_ident: &proc_macro2::Ident,
) -> proc_macro2::TokenStream {
    match sink {
        CheckSink::Collect => quote! {
            if let ::core::result::Result::Err(reported) = #validate_value_fn_ident(#value) {
                errors.extend(reported);
            }
        },
        CheckSink::Fail => quote! {
            #validate_value_fn_ident(#value)?;
        },
    }
}

/// Emits the reach-through for one wrapper and, at the end of the chain, `leaf` — which its caller
/// wrote against [`walked_value`], the binding the last step introduces.
#[cfg(feature = "serde")]
fn walk_wraps(
    wraps: &[ConstraintWrap],
    value: &proc_macro2::Ident,
    depth: usize,
    leaf: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let Some((wrap, rest)) = wraps.split_first() else {
        return leaf.clone();
    };
    let next = wrap_binding(depth);
    let inner = walk_wraps(rest, &next, depth.saturating_add(1), leaf);
    match *wrap {
        // A `None` writes nothing, so there is nothing for the constraint to describe.
        ConstraintWrap::Optional => quote! {
            if let ::core::option::Option::Some(#next) = #value {
                #inner
            }
        },
        // Reached by the nested walk alone, whose leaf writes each index into its report.
        ConstraintWrap::Sequence => {
            let index = index_binding(depth);
            quote! {
                for (#index, #next) in #value.into_iter().enumerate() {
                    #inner
                }
            }
        }
        // Reached by the nested walk alone, whose leaf writes each key into its report.
        ConstraintWrap::Map => {
            let entry = entry_binding(depth);
            quote! {
                for (#entry, #next) in #value {
                    #inner
                }
            }
        }
        // Reached by the nested walk alone, whose leaf writes the key of each bound, bound where
        // a list writes an index.
        ConstraintWrap::Range(kind) => {
            let index = index_binding(depth);
            let bounds: Vec<proc_macro2::TokenStream> = kind
                .members()
                .iter()
                .map(|member| {
                    let field = proc_macro2::Ident::new(member, proc_macro2::Span::call_site());
                    let read = if matches!(kind, RangeKind::Inclusive) {
                        quote! { #value.#field() }
                    } else {
                        quote! { &#value.#field }
                    };
                    quote! { (#member, #read) }
                })
                .collect();
            if let [only] = bounds.as_slice() {
                quote! {
                    {
                        let (#index, #next) = #only;
                        #inner
                    }
                }
            } else {
                quote! {
                    for (#index, #next) in [#(#bounds),*] {
                        #inner
                    }
                }
            }
        }
        ConstraintWrap::Borrowed | ConstraintWrap::Transparent => quote! {
            let #next = &**#value;
            #inner
        },
    }
}

/// Builds the `validate()` contribution for a member the enclosing type declares no bound on, whose
/// own type may publish one, or holds a type that may — a constrained brand, or a nested
/// `#[model_schema()]` type. The member is read where `place` says, and each report is written
/// under the name `place` gives it, then under the place the walk holds its value at.
///
/// The fallback rides inside the block rather than once per validator so that a nested check is the
/// same tokens wherever it lands: a struct's method body, or one arm of an enum's `match`, where a
/// once-per-validator item would sit in the wrong scope.
#[cfg(feature = "serde")]
fn build_nested_validation(shape: &HeldShape, place: &MemberPlace) -> proc_macro2::TokenStream {
    let head = wrap_binding(0);
    let under: Vec<PlaceStep> = place
        .reported
        .iter()
        .map(|key| PlaceStep::Written(key.clone()))
        .collect();
    let walk = held_walk(shape, &head, 1, &under);
    let fallback = unpublished_validate_fallback();
    let keyed = shape.walks_a_map().then(written_key_items);
    let checked = &place.checked;
    quote! {
        {
            #fallback
            #keyed
            let #head = #checked;
            #walk
        }
    }
}

/// The walk of `shape` from `value`, held at `place`, whose first wrapper binds at `depth`.
#[cfg(feature = "serde")]
fn held_walk(
    shape: &HeldShape,
    value: &proc_macro2::Ident,
    depth: usize,
    place: &[PlaceStep],
) -> proc_macro2::TokenStream {
    let below = depth.saturating_add(shape.wraps.len());
    let held = held_at(place, &shape.wraps, depth);
    // What the last wrapper binds, or the value itself where the walk reads through none.
    let reached = below
        .checked_sub(1)
        .filter(|_| !shape.wraps.is_empty())
        .map_or_else(|| value.clone(), wrap_binding);
    let end = match &shape.end {
        HeldEnd::Declared => nested_leaf(&reached, &held),
        HeldEnd::Positions(positions) => {
            let position = wrap_binding(below);
            let walks = positions.iter().map(|(at, of_position)| {
                let slot = syn::Index::from(*at);
                let mut held_under = held.clone();
                held_under.push(PlaceStep::Written(at.to_string()));
                let walk = held_walk(of_position, &position, below.saturating_add(1), &held_under);
                quote! {
                    {
                        let #position = &#reached.#slot;
                        #walk
                    }
                }
            });
            quote! { #(#walks)* }
        }
        HeldEnd::Arms(arms) => {
            let held_by_arm = wrap_binding(below);
            let walks = arms.iter().map(|(arm, of_arm)| {
                let variant = proc_macro2::Ident::new(arm, proc_macro2::Span::call_site());
                let mut held_under = held.clone();
                held_under.push(PlaceStep::Written((*arm).to_owned()));
                let walk = held_walk(of_arm, &held_by_arm, below.saturating_add(1), &held_under);
                quote! {
                    if let ::core::result::Result::#variant(#held_by_arm) = #reached {
                        #walk
                    }
                }
            });
            quote! { #(#walks)* }
        }
    };
    walk_wraps(&shape.wraps, value, depth, &end)
}

/// `place` with what a walk over `wraps` adds to it, where the first wrapper binds at `depth`: the
/// index of each list level and the key of each map level.
#[cfg(feature = "serde")]
fn held_at(place: &[PlaceStep], wraps: &[ConstraintWrap], depth: usize) -> Vec<PlaceStep> {
    let mut held = place.to_vec();
    for (at, wrap) in wraps.iter().enumerate() {
        let bound_at = depth.saturating_add(at);
        match *wrap {
            ConstraintWrap::Map => held.push(PlaceStep::Key(bound_at)),
            ConstraintWrap::Range(_) | ConstraintWrap::Sequence => {
                held.push(PlaceStep::Index(bound_at));
            }
            ConstraintWrap::Borrowed | ConstraintWrap::Optional | ConstraintWrap::Transparent => {}
        }
    }
    held
}

/// What the nested walk reads the key of a map's entry with. `written_key` answers the text
/// serde writes for the key, through serde's serializer for a `Formatter`, and nothing where serde
/// writes no text. The trait for a reference answers nothing for a key whose type has no
/// `Serialize` in reach, a type parameter that declares none among them.
#[cfg(feature = "serde")]
fn written_key_items() -> proc_macro2::TokenStream {
    quote! {
        struct MapKey<'held, K: ?::core::marker::Sized>(&'held K);
        impl<K: ?::core::marker::Sized + serde::Serialize> ::core::fmt::Display for MapKey<'_, K> {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                serde::Serialize::serialize(self.0, formatter)
            }
        }
        trait WrittenKey {
            fn written_key(self) -> ::core::option::Option<::std::string::String>;
        }
        impl<K: ?::core::marker::Sized + serde::Serialize> WrittenKey for MapKey<'_, K> {
            fn written_key(self) -> ::core::option::Option<::std::string::String> {
                let mut written = ::std::string::String::new();
                ::core::fmt::Write::write_fmt(&mut written, format_args!("{self}"))
                    .ok()
                    .map(|()| written)
            }
        }
        trait UnwrittenKey: ::core::marker::Sized {
            fn written_key(self) -> ::core::option::Option<::std::string::String> {
                ::core::option::Option::None
            }
        }
        impl<K: ?::core::marker::Sized> UnwrittenKey for &MapKey<'_, K> {}
    }
}

/// What a field's type answers when it publishes no `validate()` of its own.
#[cfg(feature = "serde")]
fn unpublished_validate_fallback() -> proc_macro2::TokenStream {
    quote! {
        trait UnpublishedValidate {
            fn validate(&self) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
                ::core::result::Result::Ok(())
            }
        }
        impl<T: ?::core::marker::Sized> UnpublishedValidate for &T {}
    }
}

/// The nested check a walk ends on: whatever the type of `value` reported, attributed to `place`,
/// which holds it.
#[cfg(feature = "serde")]
fn nested_leaf(value: &proc_macro2::Ident, place: &[PlaceStep]) -> proc_macro2::TokenStream {
    let Some(named) = named_at(place) else {
        return quote! {
            if let ::core::result::Result::Err(reported) = #value.validate() {
                errors.extend(reported);
            }
        };
    };
    let naming = nested_under_fn();
    // A key is read where a report is written under it, and for no entry that reports nothing.
    let keys: Vec<proc_macro2::TokenStream> = place
        .iter()
        .filter_map(|step| match step {
            PlaceStep::Key(depth) => {
                let (entry, key) = (entry_binding(*depth), key_binding(*depth));
                Some(quote! { let #key = MapKey(#entry).written_key(); })
            }
            PlaceStep::Index(_) | PlaceStep::Written(_) => None,
        })
        .collect();
    let keyed = (!keys.is_empty()).then(keyed_under_fn);
    quote! {
        if let ::core::result::Result::Err(reported) = #value.validate() {
            #naming
            #keyed
            #(#keys)*
            errors.extend(
                reported
                    .iter()
                    .map(|violation| #named),
            );
        }
    }
}

/// `violation` as it is reported from `place`, or `None` for a value held at no place. Each run of
/// keys, positions and indexes is written into it at once, and the key of a map's entry between
/// two runs on its own, there being none where serde writes none.
#[cfg(feature = "serde")]
fn named_at(place: &[PlaceStep]) -> Option<proc_macro2::TokenStream> {
    let mut keys = place.iter().rev().filter_map(|step| match step {
        PlaceStep::Key(depth) => Some(key_binding(*depth)),
        PlaceStep::Index(_) | PlaceStep::Written(_) => None,
    });
    let mut named: Option<proc_macro2::TokenStream> = None;
    let runs = place.rsplit(|step| matches!(step, PlaceStep::Key(_)));
    for (at, run) in runs.enumerate() {
        if at > 0 {
            let key = keys.next()?;
            let reported = reported_so_far(named.as_ref());
            named = Some(quote! { keyed_under(#key.as_deref(), #reported) });
        }
        if let Some(under) = nested_place(run) {
            let reported = reported_so_far(named.as_ref());
            named = Some(quote! { nested_under(#under, #reported) });
        }
    }
    named
}

/// The violation a name is written into next: the one reported, or what `named` made of it.
#[cfg(feature = "serde")]
fn reported_so_far(named: Option<&proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    named.map_or_else(|| quote! { violation }, |written| quote! { &#written })
}

/// A run of keys, positions and indexes as the `&str` a report is written under, joined by dots, or
/// `None` for an empty run.
#[cfg(feature = "serde")]
fn nested_place(run: &[PlaceStep]) -> Option<proc_macro2::TokenStream> {
    let bound = run.iter().any(|step| matches!(step, PlaceStep::Index(_)));
    let steps: Vec<String> = run
        .iter()
        .filter_map(|step| match step {
            PlaceStep::Index(depth) => Some(format!("{{{}}}", index_binding(*depth))),
            PlaceStep::Key(_) => None,
            // A key is text inside a format string, where a brace opens an argument.
            PlaceStep::Written(key) if bound => Some(key.replace('{', "{{").replace('}', "}}")),
            PlaceStep::Written(key) => Some(key.clone()),
        })
        .collect();
    if steps.is_empty() {
        return None;
    }
    let template = steps.join(".");
    Some(if bound {
        quote! { &format!(#template) }
    } else {
        quote! { #template }
    })
}

/// `violation` under the key of a map's entry, or as it is where serde writes no text for the key.
#[cfg(feature = "serde")]
fn keyed_under_fn() -> proc_macro2::TokenStream {
    quote! {
        fn keyed_under(key: ::core::option::Option<&str>, violation: &str) -> ::std::string::String {
            match key {
                ::core::option::Option::Some(written) => nested_under(written, violation),
                ::core::option::Option::None => violation.to_owned(),
            }
        }
    }
}

/// The one spelling rule for writing a holding field's name into a report the value's own type
/// wrote, emitted wherever a name has to be written: the validator's nested walk, a member's
/// check, and the read-time hook that answers for a bound checked before `validate()` ever runs.
pub fn nested_under_fn() -> proc_macro2::TokenStream {
    quote! {
        fn nested_under(field: &str, violation: &str) -> ::std::string::String {
            match violation
                .strip_prefix('\'')
                .and_then(|rest| rest.split_once('\''))
            {
                ::core::option::Option::Some((named, tail)) => format!("'{field}.{named}'{tail}"),
                ::core::option::Option::None => format!("'{field}': {violation}"),
            }
        }
    }
}

/// Whether a refusal is one of this crate's own bound sentences, under whatever field name it
/// already carries.
#[cfg(feature = "serde")]
fn reports_a_bound_fn() -> proc_macro2::TokenStream {
    let stems: Vec<&str> = VIOLATION_STEMS.to_vec();
    quote! {
        fn reports_a_bound(reported: &str) -> bool {
            let said = match reported
                .strip_prefix('\'')
                .and_then(|rest| rest.split_once('\''))
            {
                ::core::option::Option::Some((_, tail)) => tail.strip_prefix(": ").unwrap_or(tail),
                ::core::option::Option::None => reported,
            };
            [#(#stems),*].iter().any(|stem| said.starts_with(stem))
        }
    }
}

/// How a report reaches serde, which carries one sentence and not a list: joined the way the
/// dispatcher joins a fault's detail, so a payload refused as it was read and one refused after it
/// was read say the same thing in the same order.
fn refusal_from_violations() -> proc_macro2::TokenStream {
    quote! {
        |violations: ::std::vec::Vec<::std::string::String>| serde::de::Error::custom(violations.join("; "))
    }
}

/// Builds the serde hook for a field written under wrappers: it deserializes the field's own
/// declared type and then runs the same walk `validate()` runs, so the wire is gated where the
/// constraint lands rather than where the field happens to be spelled.
#[cfg(feature = "serde")]
fn build_wrapped_deserializer(
    deserialize_fn_ident: &proc_macro2::Ident,
    validate_value_fn_ident: &proc_macro2::Ident,
    field_ty: &syn::Type,
    lifetimes: &[syn::Lifetime],
    wraps: &[ConstraintWrap],
) -> proc_macro2::TokenStream {
    let head = wrap_binding(0);
    let leaf = constraint_leaf(
        CheckSink::Fail,
        &walked_value(wraps),
        validate_value_fn_ident,
    );
    let walk = walk_wraps(wraps, &head, 1, &leaf);
    let refusal = refusal_from_violations();
    quote! {
        pub fn #deserialize_fn_ident<'de, #(#lifetimes,)* D>(deserializer: D) -> ::core::result::Result<#field_ty, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            // Nested so that each hook carries its own: a schema module holds one hook per
            // constrained field and a shared name would have to be emitted exactly once.
            fn deserialize_validated<'de, D, T, F>(deserializer: D, check: F) -> ::core::result::Result<T, D::Error>
            where
                D: serde::Deserializer<'de>,
                T: serde::Deserialize<'de>,
                F: ::core::ops::FnOnce(&T) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>>,
            {
                use serde::Deserialize;
                let value = T::deserialize(deserializer)?;
                check(&value).map_err(#refusal)?;
                ::core::result::Result::Ok(value)
            }

            deserialize_validated(deserializer, |#head: &#field_ty| {
                #walk
                ::core::result::Result::Ok(())
            })
        }
    }
}

/// The closure a recovering read lists a field's broken bound with: `validator` run on every
/// value the bound reaches inside one of type `ty`, answering each violation without
/// `field_name`, the name the validator writes, which the issue's path already carries. `None`
/// where the bound reaches nothing of `ty`.
#[cfg(feature = "serde")]
pub fn recovering_bound_check(
    validator: &proc_macro2::TokenStream,
    field_name: Option<&str>,
    ty: &syn::Type,
) -> Option<proc_macro2::TokenStream> {
    let shape = constrained_shape(ty)?;
    let head = wrap_binding(0);
    let value = walked_value(&shape.wraps);
    let leaf = quote! {
        if let ::core::result::Result::Err(reported) = #validator(#value) {
            errors.extend(reported);
        }
    };
    let walk = walk_wraps(&shape.wraps, &head, 1, &leaf);
    // A validator that writes no name answers the sentence as the read lists it.
    let listed = field_name.map_or_else(
        || quote! { errors },
        |written| {
            let named = format!("'{written}': ");
            quote! {
                errors
                    .iter()
                    .map(|violation| violation.strip_prefix(#named).unwrap_or(violation).to_owned())
                    .collect()
            }
        },
    );
    Some(quote! {
        |#head: &#ty| -> ::std::vec::Vec<::std::string::String> {
            let mut errors: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
            #walk
            #listed
        }
    })
}

/// The stem the per-field helpers are named from: `validate_{stem}_value` and `deserialize_{stem}`.
/// A field name is unique only within its variant, while one schema module holds every variant's
/// helpers — so a variant's field carries its variant into the stem to avoid collisions.
#[cfg(feature = "serde")]
pub fn helper_name_stem(field_ident: &str, variant_ident: Option<&str>) -> String {
    variant_ident.map_or_else(
        || field_ident.to_owned(),
        |variant| format!("{}_{field_ident}", to_snake_case(variant)),
    )
}

/// The check a `pattern` constraint holds `value` to, taking `failure` where the value is turned
/// away.
#[cfg(feature = "serde")]
fn pattern_check(pattern: &str, failure: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    trivial_pattern(pattern).map_or_else(
        || {
            quote! {
                {
                    use ::std::sync::LazyLock;
                    static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
                        regex::Regex::new(#pattern).unwrap()
                    });
                    if !RE.is_match(value) {
                        #failure
                    }
                }
            }
        },
        |trivial| {
            let turned_away = pattern_rejects(&trivial);
            quote! {
                if #turned_away {
                    #failure
                }
            }
        },
    )
}

/// The condition under which a trivial pattern turns `value` away — the negation of what it
/// accepts, which is the form the emitted check reads it in.
#[cfg(feature = "serde")]
fn pattern_rejects(trivial: &TrivialPattern) -> proc_macro2::TokenStream {
    match trivial {
        TrivialPattern::IsEmpty => quote! { !value.is_empty() },
        TrivialPattern::Equals(needle) => quote! { value != #needle },
        TrivialPattern::StartsWith(needle) => {
            let sought = needle_pattern(needle);
            quote! { !value.starts_with(#sought) }
        }
        TrivialPattern::EndsWith(needle) => {
            let sought = needle_pattern(needle);
            quote! { !value.ends_with(#sought) }
        }
        TrivialPattern::Contains(needle) => {
            let sought = needle_pattern(needle);
            quote! { !value.contains(#sought) }
        }
    }
}

/// A needle in the spelling the `str` pattern methods want it where the call is written into a
/// crate that denies `clippy::single_char_pattern`: one character as a `char`, anything else as
/// the string it is. Both name the same pattern to the same method.
#[cfg(feature = "serde")]
fn needle_pattern(needle: &str) -> proc_macro2::TokenStream {
    let mut chars = needle.chars();
    if let (Some(only), None) = (chars.next(), chars.next()) {
        return quote! { #only };
    }
    quote! { #needle }
}

/// The parameter a string validator takes and the rendering its checks read `value` from. A path
/// is the one leaf that can't be handed as-is: it arrives borrowed and is rendered once through
/// `to_string_lossy`, the string serde writes for it. Every other leaf already is that string.
#[cfg(feature = "serde")]
fn checked_value_parts(
    measures_path: bool,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    if measures_path {
        (
            quote! { path: &::std::path::Path },
            quote! {
                let rendered = path.to_string_lossy();
                let value: &str = &rendered;
            },
        )
    } else {
        (quote! { value: &str }, quote! {})
    }
}

/// The parameter a field's string validator takes, the rendering its checks read `value` from, and
/// the owned form of the leaf, which is what a bare field of it is declared as.
#[cfg(feature = "serde")]
fn string_leaf_parts(
    leaf: &ConstraintLeaf,
    field_name: &str,
    meta: &ModelSchemaPropMeta,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
) {
    match leaf {
        ConstraintLeaf::Named(named) => (
            quote! { named: &#named },
            named_leaf_rendering(named, field_name, meta),
            quote! { #named },
        ),
        ConstraintLeaf::Path => {
            let (param, rendering) = checked_value_parts(true);
            (param, rendering, quote! { ::std::path::PathBuf })
        }
        ConstraintLeaf::List | ConstraintLeaf::Number(_) | ConstraintLeaf::Str => {
            let (param, rendering) = checked_value_parts(false);
            (param, rendering, quote! { ::std::string::String })
        }
    }
}

/// Reads the text of a named leaf. A brand's own expansion publishes `model_schema_bound_text`,
/// and an inherent method is found ahead of a trait's; the trait declared here answers for an alias
/// of a string or a path, and refuses every other type in the words written on it.
#[cfg(feature = "serde")]
fn named_leaf_rendering(
    named: &syn::Type,
    field_name: &str,
    meta: &ModelSchemaPropMeta,
) -> proc_macro2::TokenStream {
    let keys = [
        ("minLength", meta.min_length.is_some()),
        ("maxLength", meta.max_length.is_some()),
        ("pattern", meta.pattern.is_some()),
    ]
    .into_iter()
    .filter_map(|(key, written)| written.then_some(key))
    .collect::<Vec<_>>()
    .join("`, `");
    let message = format!(
        "model_schema: {}: `{keys}` cannot apply to `{{Self}}`",
        field_label(field_name)
    );
    // Located on the field's type, with the macro's own hygiene: see `build_branded_validation`.
    let read = proc_macro2::Ident::new(
        "model_schema_bound_text",
        named.span().resolved_at(proc_macro2::Span::call_site()),
    );
    quote! {
        #[diagnostic::on_unimplemented(
            message = #message,
            label = "this type holds no string for the bound to measure",
            note = "a length or a pattern is measured on a `String`, on a path, or on a `#[serde(transparent)]` brand over one of them that `#[model_schema]` was written on: declare the field as one of those, or drop the bound"
        )]
        trait BoundedText {
            fn bounded_text(&self) -> ::std::borrow::Cow<'_, str>;
        }
        impl BoundedText for str {
            fn bounded_text(&self) -> ::std::borrow::Cow<'_, str> {
                ::std::borrow::Cow::Borrowed(self)
            }
        }
        impl BoundedText for ::std::string::String {
            fn bounded_text(&self) -> ::std::borrow::Cow<'_, str> {
                ::std::borrow::Cow::Borrowed(self)
            }
        }
        impl BoundedText for ::std::path::Path {
            fn bounded_text(&self) -> ::std::borrow::Cow<'_, str> {
                self.to_string_lossy()
            }
        }
        impl BoundedText for ::std::path::PathBuf {
            fn bounded_text(&self) -> ::std::borrow::Cow<'_, str> {
                self.to_string_lossy()
            }
        }
        trait UnbrandedText {
            fn model_schema_bound_text(&self) -> ::std::borrow::Cow<'_, str>
            where
                Self: BoundedText;
        }
        impl<T: ?::core::marker::Sized> UnbrandedText for T {
            fn model_schema_bound_text(&self) -> ::std::borrow::Cow<'_, str>
            where
                Self: BoundedText,
            {
                BoundedText::bounded_text(self)
            }
        }
        // Keeps both traits in use where the leaf answers with a method of its own.
        let _: ::std::borrow::Cow<'_, str> = UnbrandedText::model_schema_bound_text("");
        let rendered = named.#read();
        let value: &str = &rendered;
    }
}

/// Generates the static validator for a string-shaped field with constraints, plus the serde
/// deserializer — written against the constrained value itself when the field is bare, and against
/// the field's declared type when it is wrapped. A field read through a reference gets the
/// validator alone.
#[cfg(feature = "serde")]
fn generate_string_validation_code(
    member: &CheckedMember,
    meta: &ModelSchemaPropMeta,
    shape: &ConstrainedShape,
    field_ty: &syn::Type,
) -> FieldValidationCode {
    let wraps: &[ConstraintWrap] = &shape.wraps;
    let validate_value_fn_name = format!("validate_{}_value", member.stem);
    let validate_value_fn_ident =
        proc_macro2::Ident::new(&validate_value_fn_name, proc_macro2::Span::call_site());
    let deserialize_fn_name = format!("deserialize_{}", member.stem);
    let deserialize_fn_ident =
        proc_macro2::Ident::new(&deserialize_fn_name, proc_macro2::Span::call_site());

    let named = member.reported.as_deref();
    let (checked_param, rendering, owned) = string_leaf_parts(&shape.leaf, &member.label, meta);

    let measured = quote! { value.len() };
    let mut checks: Vec<proc_macro2::TokenStream> = Vec::new();

    if let Some(min_len) = meta.min_length {
        let reported = rust_violation(Bound::MinLength(min_len), named, &measured);
        checks.push(quote! {
            if value.len() < #min_len {
                errors.push(#reported);
            }
        });
    }

    if let Some(max_len) = meta.max_length {
        let reported = rust_violation(Bound::MaxLength(max_len), named, &measured);
        checks.push(quote! {
            if value.len() > #max_len {
                errors.push(#reported);
            }
        });
    }

    for pattern in meta.patterns() {
        let reported = rust_violation(Bound::Pattern(pattern), named, &measured);
        checks.push(pattern_check(pattern, &quote! { errors.push(#reported); }));
    }

    let deserializer = if ends_on_a_borrow(wraps) {
        quote! {}
    } else if wraps.is_empty() {
        let refusal = refusal_from_violations();
        quote! {
            pub fn #deserialize_fn_ident<'de, D>(deserializer: D) -> ::core::result::Result<#owned, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::Deserialize;
                let s = #owned::deserialize(deserializer)?;
                #validate_value_fn_ident(&s).map_err(#refusal)?;
                ::core::result::Result::Ok(s)
            }
        }
    } else {
        build_wrapped_deserializer(
            &deserialize_fn_ident,
            &validate_value_fn_ident,
            field_ty,
            &shape.lifetimes,
            wraps,
        )
    };

    let module_items = quote! {
        pub fn #validate_value_fn_ident(#checked_param) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
            #rendering
            let mut errors: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
            #(#checks)*
            if errors.is_empty() { ::core::result::Result::Ok(()) } else { ::core::result::Result::Err(errors) }
        }

        #deserializer
    };

    let validate_body = build_field_validation(wraps, &member.checked, &validate_value_fn_ident);

    FieldValidationCode {
        module_items,
        validate_body,
    }
}

/// Generates the static validator for a numeric field with constraints, plus the serde deserializer
/// — see `generate_string_validation_code` for how the two spellings differ.
#[cfg(feature = "serde")]
fn generate_numeric_validation_code(
    member: &CheckedMember,
    rust_type_str: &str,
    meta: &ModelSchemaPropMeta,
    shape: &ConstrainedShape,
    field_ty: &syn::Type,
) -> FieldValidationCode {
    let wraps: &[ConstraintWrap] = &shape.wraps;
    let validate_value_fn_name = format!("validate_{}_value", member.stem);
    let validate_value_fn_ident =
        proc_macro2::Ident::new(&validate_value_fn_name, proc_macro2::Span::call_site());
    let deserialize_fn_name = format!("deserialize_{}", member.stem);
    let deserialize_fn_ident =
        proc_macro2::Ident::new(&deserialize_fn_name, proc_macro2::Span::call_site());

    let rust_type_ident: proc_macro2::TokenStream = rust_type_str.parse().unwrap();
    let named = member.reported.as_deref();

    let measured = quote! { value };
    let mut checks: Vec<proc_macro2::TokenStream> = Vec::new();

    if let Some(minimum) = meta.minimum {
        let min_cast: proc_macro2::TokenStream =
            format!("{minimum} as {rust_type_str}").parse().unwrap();
        let reported = rust_violation(Bound::Minimum(minimum), named, &measured);
        checks.push(quote! {
            if *value < #min_cast {
                errors.push(#reported);
            }
        });
    }

    if let Some(maximum) = meta.maximum {
        let max_cast: proc_macro2::TokenStream =
            format!("{maximum} as {rust_type_str}").parse().unwrap();
        let reported = rust_violation(Bound::Maximum(maximum), named, &measured);
        checks.push(quote! {
            if *value > #max_cast {
                errors.push(#reported);
            }
        });
    }

    let deserializer = if ends_on_a_borrow(wraps) {
        quote! {}
    } else if wraps.is_empty() {
        let refusal = refusal_from_violations();
        quote! {
            pub fn #deserialize_fn_ident<'de, D>(deserializer: D) -> ::core::result::Result<#rust_type_ident, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::Deserialize;
                let v = #rust_type_ident::deserialize(deserializer)?;
                #validate_value_fn_ident(&v).map_err(#refusal)?;
                ::core::result::Result::Ok(v)
            }
        }
    } else {
        build_wrapped_deserializer(
            &deserialize_fn_ident,
            &validate_value_fn_ident,
            field_ty,
            &shape.lifetimes,
            wraps,
        )
    };

    let module_items = quote! {
        pub fn #validate_value_fn_ident(value: &#rust_type_ident) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
            let mut errors: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
            #(#checks)*
            if errors.is_empty() { ::core::result::Result::Ok(()) } else { ::core::result::Result::Err(errors) }
        }

        #deserializer
    };

    let validate_body = build_field_validation(wraps, &member.checked, &validate_value_fn_ident);

    FieldValidationCode {
        module_items,
        validate_body,
    }
}

/// What holds `L` to being a list whose items are counted: every std sequence, an array and a
/// slice are iterated by reference. `unsized_list` is a list reached through a transparent wrapper.
/// A bound on the iterator itself is one the compiler cannot infer `L` under.
#[cfg(feature = "serde")]
fn counted_list_bounds(unsized_list: bool) -> proc_macro2::TokenStream {
    let relaxed = unsized_list.then(|| quote! { L: ?::core::marker::Sized, });
    quote! {
        #relaxed
        for<'held> &'held L: ::core::iter::IntoIterator,
    }
}

/// The validator of a list's item count, under the bounds `minLength` and `maxLength` wrote on it.
/// It takes the list as any type that is one: no type a list holds is written into the schema
/// module, where a type parameter of the item's, `Self` and a borrowed lifetime name nothing.
#[cfg(feature = "serde")]
fn list_validator(
    validator: &proc_macro2::Ident,
    least: Option<usize>,
    most: Option<usize>,
    named: Option<&str>,
) -> proc_macro2::TokenStream {
    let measured = quote! { held };
    let too_few = least.map(|count| {
        let reported = rust_violation(Bound::MinItems(count), named, &measured);
        quote! {
            if held < #count {
                errors.push(#reported);
            }
        }
    });
    let too_many = most.map(|count| {
        let reported = rust_violation(Bound::MaxItems(count), named, &measured);
        quote! {
            if held > #count {
                errors.push(#reported);
            }
        }
    });
    let counted = counted_list_bounds(true);
    quote! {
        pub fn #validator<L>(list: &L) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>>
        where
            #counted
        {
            let held = list.into_iter().count();
            let mut errors: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
            #too_few
            #too_many
            if errors.is_empty() { ::core::result::Result::Ok(()) } else { ::core::result::Result::Err(errors) }
        }
    }
}

/// What the serde hook of a list written under `wraps` is declared with, or `None` for one a
/// reference borrows. It names the list `L` and each transparent wrapper by what it derefs to, so
/// it reads whatever type the member declares.
#[cfg(feature = "serde")]
fn list_hook_header(wraps: &[ConstraintWrap]) -> Option<ListHookHeader> {
    let mut read = quote! { L };
    let mut holders: Vec<proc_macro2::Ident> = Vec::new();
    let mut held_to: Vec<proc_macro2::TokenStream> = Vec::new();
    for (depth, wrap) in wraps.iter().enumerate().rev() {
        match *wrap {
            ConstraintWrap::Borrowed
            | ConstraintWrap::Map
            | ConstraintWrap::Range(_)
            | ConstraintWrap::Sequence => {
                return None;
            }
            ConstraintWrap::Optional => read = quote! { ::core::option::Option<#read> },
            ConstraintWrap::Transparent => {
                let holder =
                    proc_macro2::Ident::new(&format!("H{depth}"), proc_macro2::Span::call_site());
                let relaxed = matches!(
                    depth.checked_sub(1).and_then(|outer| wraps.get(outer)),
                    Some(ConstraintWrap::Transparent)
                )
                .then(|| quote! { ?::core::marker::Sized + });
                held_to.push(quote! { #holder: #relaxed ::core::ops::Deref<Target = #read>, });
                read = quote! { #holder };
                holders.push(holder);
            }
        }
    }
    Some(ListHookHeader {
        counted: counted_list_bounds(matches!(wraps.last(), Some(ConstraintWrap::Transparent))),
        held_to,
        holders,
        read,
    })
}

/// The serde hook of a list written under `wraps`, or nothing for one a reference borrows.
#[cfg(feature = "serde")]
fn list_deserializer(
    deserializer: &proc_macro2::Ident,
    validator: &proc_macro2::Ident,
    wraps: &[ConstraintWrap],
) -> proc_macro2::TokenStream {
    let Some(ListHookHeader {
        counted,
        held_to,
        holders,
        read,
    }) = list_hook_header(wraps)
    else {
        return quote! {};
    };
    let head = wrap_binding(0);
    let leaf = constraint_leaf(CheckSink::Fail, &walked_value(wraps), validator);
    let walk = walk_wraps(wraps, &head, 1, &leaf);
    let refusal = refusal_from_violations();
    quote! {
        pub fn #deserializer<'de, D, #(#holders,)* L>(deserializer: D) -> ::core::result::Result<#read, D::Error>
        where
            D: serde::Deserializer<'de>,
            #read: serde::Deserialize<'de>,
            #(#held_to)*
            #counted
        {
            let read = <#read as serde::Deserialize>::deserialize(deserializer)?;
            let check = |#head: &#read| -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
                #walk
                ::core::result::Result::Ok(())
            };
            check(&read).map_err(#refusal)?;
            ::core::result::Result::Ok(read)
        }
    }
}

/// Generates the static validator of a list field's item count, plus the serde deserializer.
#[cfg(feature = "serde")]
fn generate_list_validation_code(
    member: &CheckedMember,
    meta: &ModelSchemaPropMeta,
    wraps: &[ConstraintWrap],
) -> FieldValidationCode {
    let validator = proc_macro2::Ident::new(
        &format!("validate_{}_value", member.stem),
        proc_macro2::Span::call_site(),
    );
    let deserializer = proc_macro2::Ident::new(
        &format!("deserialize_{}", member.stem),
        proc_macro2::Span::call_site(),
    );
    let validate_fn = list_validator(
        &validator,
        meta.min_length,
        meta.max_length,
        member.reported.as_deref(),
    );
    let deserialize_fn = list_deserializer(&deserializer, &validator, wraps);
    FieldValidationCode {
        module_items: quote! {
            #validate_fn

            #deserialize_fn
        },
        validate_body: build_field_validation(wraps, &member.checked, &validator),
    }
}

/// Reads a field's type down to the value a constraint can land on, collecting the wrappers on the
/// way. A list is that value itself, whatever it holds.
#[cfg(feature = "serde")]
fn constrained_shape(ty: &syn::Type) -> Option<ConstrainedShape> {
    let mut wraps = Vec::new();
    let mut lifetimes: Vec<syn::Lifetime> = Vec::new();
    let mut current = ty;
    loop {
        current = written_type(current);
        if names_a_list(current) {
            return Some(ConstrainedShape {
                leaf: ConstraintLeaf::List,
                lifetimes,
                wraps,
            });
        }
        if let syn::Type::Reference(reference) = current {
            wraps.push(ConstraintWrap::Borrowed);
            current = &reference.elem;
        } else if let syn::Type::Path(path) = current {
            let segment = path.path.segments.last()?;
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                wraps.push(generic_wrap(&segment.ident.to_string())?);
                collect_lifetimes(args, &mut lifetimes);
                current = sole_type_argument(args)?;
            } else if matches!(segment.arguments, syn::PathArguments::None) {
                let leaf = leaf_for_ident(&segment.ident.to_string())
                    .unwrap_or_else(|| ConstraintLeaf::Named(Box::new(current.clone())));
                return Some(ConstrainedShape {
                    leaf,
                    lifetimes,
                    wraps,
                });
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
}

/// Reads a field's type down to the value whose own type would publish the validator, collecting
/// the wrappers it is reached through, each level of a list and each reference among them.
#[cfg(feature = "serde")]
fn nested_shape(ty: &syn::Type) -> Option<Vec<ConstraintWrap>> {
    let mut wraps = Vec::new();
    let mut current = ty;
    loop {
        current = written_type(current);
        if let syn::Type::Reference(reference) = current {
            wraps.push(ConstraintWrap::Borrowed);
            current = &reference.elem;
        } else if let syn::Type::Array(array) = current {
            wraps.push(ConstraintWrap::Sequence);
            current = &array.elem;
        } else if let syn::Type::Slice(slice) = current {
            wraps.push(ConstraintWrap::Sequence);
            current = &slice.elem;
        } else if let syn::Type::Path(path) = current {
            let segment = path.path.segments.last()?;
            let ident = segment.ident.to_string();
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                let Some(wrap) = generic_wrap(&ident) else {
                    return (!reads_through(&ident)).then_some(wraps);
                };
                wraps.push(wrap);
                current = sole_type_argument(args)?;
            } else if matches!(segment.arguments, syn::PathArguments::None) {
                return Some(wraps);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
}

/// Reads a member's type down to every value whose own type would publish the validator, against
/// what `def` says the type holds: through every reference and each level of a list, to a map's
/// values and to each position of a tuple. `None` where it holds no declared type.
#[cfg(feature = "serde")]
fn held_shape(ty: &syn::Type, def: &FieldDef) -> Option<HeldShape> {
    let mut wraps = Vec::new();
    let mut current = ty;
    let mut held = def;
    loop {
        current = written_type(current);
        if let syn::Type::Reference(reference) = current {
            wraps.push(ConstraintWrap::Borrowed);
            current = &reference.elem;
        } else if let syn::Type::Array(array) = current {
            wraps.push(ConstraintWrap::Sequence);
            current = &array.elem;
        } else if let syn::Type::Slice(slice) = current {
            wraps.push(ConstraintWrap::Sequence);
            current = &slice.elem;
        } else if let syn::Type::Tuple(tuple) = current {
            let FieldDefType::Tuple(of_positions) = &held.field_type else {
                return None;
            };
            let positions: Vec<(usize, HeldShape)> = tuple
                .elems
                .iter()
                .zip(of_positions)
                .enumerate()
                .filter_map(|(at, (position, of_position))| {
                    Some((at, held_shape(position, of_position)?))
                })
                .collect();
            return (!positions.is_empty()).then_some(HeldShape {
                end: HeldEnd::Positions(positions),
                wraps,
            });
        } else if let syn::Type::Path(path) = current {
            let segment = path.path.segments.last()?;
            let ident = segment.ident.to_string();
            let declared = matches!(held.field_type, FieldDefType::SiblingType(_, _));
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let FieldDefType::Range(kind, bound) = &held.field_type
                    && RangeKind::named(&ident) == Some(*kind)
                {
                    wraps.push(ConstraintWrap::Range(*kind));
                    current = sole_type_argument(args)?;
                    held = bound;
                    continue;
                }
                if let FieldDefType::Result(ok, err) = &held.field_type
                    && let Some((ok_type, err_type)) = written_std_result(path)
                {
                    let arms: Vec<(&'static str, HeldShape)> = RESULT_ARMS
                        .into_iter()
                        .zip([(ok_type, &**ok), (err_type, &**err)])
                        .filter_map(|(arm, (arm_type, arm_def))| {
                            Some((arm, held_shape(arm_type, arm_def)?))
                        })
                        .collect();
                    return (!arms.is_empty()).then_some(HeldShape {
                        end: HeldEnd::Arms(arms),
                        wraps,
                    });
                }
                if is_map_wrapper(&ident) {
                    let FieldDefType::Map(_, of_values) = &held.field_type else {
                        return None;
                    };
                    wraps.push(ConstraintWrap::Map);
                    current = map_value_argument(args)?;
                    held = of_values;
                    continue;
                }
                let Some(wrap) = generic_wrap(&ident) else {
                    return (declared && !reads_through(&ident)).then_some(HeldShape {
                        end: HeldEnd::Declared,
                        wraps,
                    });
                };
                wraps.push(wrap);
                current = sole_type_argument(args)?;
                held = list_item_def(held, &ident);
            } else if matches!(segment.arguments, syn::PathArguments::None) {
                return declared.then_some(HeldShape {
                    end: HeldEnd::Declared,
                    wraps,
                });
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
}

/// What `def` says a list written `wrapper` holds. A list other than a `Vec` is described as a type
/// of its own over its items, and every other wrapper as what it holds.
#[cfg(feature = "serde")]
fn list_item_def<'def>(def: &'def FieldDef, wrapper: &str) -> &'def FieldDef {
    if let FieldDefType::SiblingType(named, arguments) = &def.field_type
        && named == wrapper
    {
        arguments.first().unwrap_or(def)
    } else {
        def
    }
}

/// The type of a map's values, its second type argument.
#[cfg(feature = "serde")]
fn map_value_argument(args: &syn::AngleBracketedGenericArguments) -> Option<&syn::Type> {
    args.args
        .iter()
        .filter_map(|arg| {
            if let syn::GenericArgument::Type(ty) = arg {
                Some(ty)
            } else {
                None
            }
        })
        .nth(1)
}

/// Whether a generic name stands for a std container the crate describes by what it holds, rather
/// than for a declared type of the author's own.
///
/// Consulted only for the names [`generic_wrap`] found no reach through, so half of what it names
/// is already unreachable from there. Written as the whole set on purpose: narrowing that reach
/// should turn a container into one this walk stops short of, never into a type it walks into.
#[cfg(feature = "serde")]
fn reads_through(ident: &str) -> bool {
    is_transparent_wrapper(ident)
        || is_sequence_wrapper(ident)
        || is_refused_sequence_wrapper(ident)
        || is_map_wrapper(ident)
        || ident == "Option"
}

/// The interior-mutability wrapper on a field's constrained path, when there is one.
/// `constrained_shape` cannot say *why* a shape failed to classify, so this retraces the path to
/// name the blocker for a diagnostic instead of silently dropping the constraint.
#[cfg(feature = "serde")]
fn blocking_interior_mutability_wrapper(ty: &syn::Type) -> Option<String> {
    let mut current = ty;
    loop {
        current = written_type(current);
        if let syn::Type::Array(array) = current {
            current = &array.elem;
        } else if let syn::Type::Slice(slice) = current {
            current = &slice.elem;
        } else if let syn::Type::Reference(reference) = current {
            current = &reference.elem;
        } else if let syn::Type::Path(path) = current {
            let segment = path.path.segments.last()?;
            let ident = segment.ident.to_string();
            if is_interior_mutability_wrapper(&ident) {
                return Some(ident);
            }
            let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                return None;
            };
            current = sole_type_argument(args)?;
        } else {
            return None;
        }
    }
}

/// Whether a type is written as a list: an array, a slice, or one of the std sequences.
#[cfg(any(
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema",
    feature = "serde"
))]
fn names_a_list(ty: &syn::Type) -> bool {
    let written = written_type(ty);
    if matches!(written, syn::Type::Array(_) | syn::Type::Slice(_)) {
        return true;
    }
    let syn::Type::Path(named) = written else {
        return false;
    };
    named.path.segments.last().is_some_and(|segment| {
        matches!(segment.arguments, syn::PathArguments::AngleBracketed(_))
            && is_sequence_wrapper(&segment.ident.to_string())
    })
}

/// Whether a brand's inner is written as a list itself, or as one behind `Box`, `Rc`, `Arc` or
/// `Cow`: the reach `constrained_shape` has through transparent wrappers alone, asked in a build
/// without `serde`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn held_as_a_list(ty: &syn::Type) -> bool {
    let mut current = ty;
    loop {
        if names_a_list(current) {
            return true;
        }
        let syn::Type::Path(named) = written_type(current) else {
            return false;
        };
        let Some(segment) = named.path.segments.last() else {
            return false;
        };
        let syn::PathArguments::AngleBracketed(held) = &segment.arguments else {
            return false;
        };
        if !is_ownership_wrapper(&segment.ident.to_string()) {
            return false;
        }
        let Some(inner) = sole_type_argument(held) else {
            return false;
        };
        current = inner;
    }
}

/// Adds the lifetimes a wrapper spells to the ones already collected, skipping `'static` — which
/// needs no declaration — and any already there, since a lifetime can only be declared once.
#[cfg(feature = "serde")]
fn collect_lifetimes(
    args: &syn::AngleBracketedGenericArguments,
    lifetimes: &mut Vec<syn::Lifetime>,
) {
    for arg in &args.args {
        if let syn::GenericArgument::Lifetime(lifetime) = arg
            && lifetime.ident != "static"
            && !lifetimes.iter().any(|seen| seen.ident == lifetime.ident)
        {
            lifetimes.push(lifetime.clone());
        }
    }
}

/// The wrapper a generic type stands for, or `None` if it is not one the constraint reads through.
/// An interior-mutability wrapper is deliberately absent: it is wire-transparent but does not
/// implement `Deref`, so the walk has no safe way through it.
#[cfg(feature = "serde")]
fn generic_wrap(ident: &str) -> Option<ConstraintWrap> {
    if ident == "Option" {
        Some(ConstraintWrap::Optional)
    } else if is_sequence_wrapper(ident) {
        Some(ConstraintWrap::Sequence)
    } else if is_ownership_wrapper(ident) {
        Some(ConstraintWrap::Transparent)
    } else {
        None
    }
}

/// The one type argument a wrapper holds. A lifetime writes nothing and is not one, which is what
/// lets `Cow<'a, str>` answer here exactly as `Box<str>` does.
fn sole_type_argument(args: &syn::AngleBracketedGenericArguments) -> Option<&syn::Type> {
    let mut types = args.args.iter().filter_map(|arg| {
        if let syn::GenericArgument::Type(ty) = arg {
            Some(ty)
        } else {
            None
        }
    });
    let only = types.next()?;
    types.next().is_none().then_some(only)
}

/// The leaf a bare type name stands for. `str` is `String`'s borrowed form and answers as one, as
/// `Path` does for `PathBuf`; the numerics name themselves, since the validator's parameter is
/// written from the name.
#[cfg(feature = "serde")]
fn leaf_for_ident(ident: &str) -> Option<ConstraintLeaf> {
    match ident {
        "String" | "str" => Some(ConstraintLeaf::Str),
        "PathBuf" | "Path" => Some(ConstraintLeaf::Path),
        "u8" => Some(ConstraintLeaf::Number("u8")),
        "u16" => Some(ConstraintLeaf::Number("u16")),
        "u32" => Some(ConstraintLeaf::Number("u32")),
        "u64" => Some(ConstraintLeaf::Number("u64")),
        "i8" => Some(ConstraintLeaf::Number("i8")),
        "i16" => Some(ConstraintLeaf::Number("i16")),
        "i32" => Some(ConstraintLeaf::Number("i32")),
        "i64" => Some(ConstraintLeaf::Number("i64")),
        "usize" => Some(ConstraintLeaf::Number("usize")),
        "isize" => Some(ConstraintLeaf::Number("isize")),
        "f32" => Some(ConstraintLeaf::Number("f32")),
        "f64" => Some(ConstraintLeaf::Number("f64")),
        _ => None,
    }
}

fn validate_as_number_flag(field_type: &FieldDefType, flag_set: bool) -> Result<(), String> {
    #[cfg(not(feature = "chrono"))]
    let _: &FieldDefType = field_type;

    #[cfg(feature = "chrono")]
    let is_datetime = matches!(field_type, FieldDefType::DateTime);
    #[cfg(not(feature = "chrono"))]
    let is_datetime = false;

    if flag_set && !is_datetime {
        return Err("#[model_schema_prop(as_number)] requires a chrono DateTime<Tz> field".into());
    }
    Ok(())
}

/// `as_number` promises every surface an epoch-milliseconds number; this is the serde hook that
/// makes the Rust side keep that promise. Held back where the author already reads the field
/// through a hook of their own, which serde admits only one of.
#[cfg(all(feature = "serde", feature = "chrono"))]
fn as_number_serde_hook(field: &Field, field_def: &FieldDef) -> Option<syn::Attribute> {
    let as_number = field_def
        .model_schema_prop_meta
        .as_ref()
        .is_some_and(|meta| meta.as_number);
    if !as_number || has_serde_read_hook(&field.attrs) {
        return None;
    }
    let module = if field_def.is_optional() {
        "chrono::serde::ts_milliseconds_option"
    } else {
        "chrono::serde::ts_milliseconds"
    };
    let path_lit = syn::LitStr::new(module, proc_macro2::Span::call_site());
    Some(syn::parse_quote! { #[serde(with = #path_lit)] })
}

#[cfg(all(feature = "serde", not(feature = "chrono")))]
const fn as_number_serde_hook(_field: &Field, _field_def: &FieldDef) -> Option<syn::Attribute> {
    None
}

/// Rejects `ts_optional` where the member has no key for it to make optional. A positional slot
/// writes no key at all, and one a serde attribute takes out of both of serde's directions is
/// described on no surface, so on either the flag asks for a spelling nothing emits.
fn validate_ts_optional_flag(
    field: &Field,
    field_def: &FieldDef,
    flag_set: bool,
) -> Result<(), String> {
    if !flag_set {
        return Ok(());
    }
    if field.ident.is_none() {
        return Err(
            "#[model_schema_prop(ts_optional)] requires a named field: a positional slot writes \
             no key for the flag to make optional"
                .into(),
        );
    }
    if !field_def.is_optional() {
        return Err("#[model_schema_prop(ts_optional)] requires an Option<T> field".into());
    }
    if field_def.absent_from_wire {
        return Err(
            "#[model_schema_prop(ts_optional)] requires a field the wire carries: a serde \
             attribute takes this one out of both directions, so no member is written for the \
             flag to make optional"
                .into(),
        );
    }
    Ok(())
}

fn validate_nullable_flag(field_optional: bool, flag_set: bool) -> Result<(), String> {
    if flag_set && !field_optional {
        return Err("#[model_schema_prop(nullable)] requires an Option<T> field".into());
    }
    Ok(())
}

/// Rejects `nullable` written beside `ts_optional`: the two disagree about the key.
/// `nullable` keeps it and writes `null` for a `None`; `ts_optional` drops it entirely. Together
/// they would spell `field?: T | null`, a third state neither flag models.
fn check_nullable_ts_optional_conflict(flags: &ModelSchemaPropMeta) -> Result<(), String> {
    if flags.nullable && flags.ts_optional {
        return Err(
            "#[model_schema_prop(nullable)] and ts_optional cannot be written together: nullable \
             keeps the key and writes `null` for a `None`, ts_optional drops the key entirely. \
             Pick the one the wire actually carries."
                .into(),
        );
    }
    Ok(())
}

/// The field's name as serde reads it, with no raw prefix; empty for a positional slot.
fn field_ident_string(field: &Field) -> String {
    field
        .ident
        .as_ref()
        .map_or_default(|ident| ident.unraw().to_string())
}

/// Hands the field def the two serde facts the object surfaces spend: whether the field's value
/// leaves the key out of the serialized object rather than writing it, and whether the key is out
/// of both directions at once.
fn apply_serde_key_omission(field_def: &mut FieldDef, field: &Field) {
    let omission = parse_serde_key_omission(&field.attrs);
    field_def.omits_value = field.ident.is_some() && omission.omits_key;
    field_def.absent_from_wire = field.ident.is_some() && omission.absent_from_wire();
}

/// Adds the field to the list every surface is built from, unless the wire carries its key in
/// neither direction.
fn push_described_field(field_defs: &mut Vec<FieldDef>, field_def: FieldDef) {
    if !field_def.absent_from_wire {
        field_defs.push(field_def);
    }
}

/// Rejects a named `Option` field whose serde attributes let a `None` reach the wire as `null`,
/// unless `nullable` already declares that shape on the author's word.
#[cfg(feature = "serde")]
fn check_optional_field_serialization(
    field: &Field,
    is_optional: bool,
    is_nullable: bool,
) -> Result<(), syn::Error> {
    let Some(ident) = field.ident.as_ref() else {
        return Ok(());
    };
    if !is_optional || is_nullable || parse_serde_key_omission(&field.attrs).omits_key {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: field `{ident}` is `Option`, so its declared shape is `{ident}: T | \
             undefined` — the key is dropped for a `None`, not written as `null`. serde writes \
             `null` unless told otherwise. Add #[serde(skip_serializing_if = \"Option::is_none\")] \
             (plus `default` if the type derives Deserialize), or `skip` / `skip_serializing` — or \
             declare #[model_schema_prop(nullable)] if the wire really does carry `null` here."
        ),
    ))
}

/// Rejects a `nullable` field whose serde attributes drop the key the flag says is always
/// written.
#[cfg(feature = "serde")]
fn check_nullable_field_serialization(field: &Field, is_nullable: bool) -> Result<(), syn::Error> {
    let Some(ident) = field.ident.as_ref() else {
        return Ok(());
    };
    if !is_nullable || !parse_serde_key_omission(&field.attrs).omits_key {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: field `{ident}` is declared #[model_schema_prop(nullable)], so its key \
             is always written and a `None` reaches the wire as `null` — but its serde attribute \
             drops the key instead, and the generated schema requires it. Remove the key-dropping \
             attribute, or drop the `nullable` flag."
        ),
    ))
}

/// Rejects a named field whose serde attributes drop its key on the way out while serde still
/// insists on finding that key on the way in.
#[cfg(feature = "serde")]
fn check_omitted_key_is_readable(
    field: &Field,
    is_optional: bool,
    container_defaulted: bool,
) -> Result<(), syn::Error> {
    let Some(ident) = field.ident.as_ref() else {
        return Ok(());
    };
    let omission = parse_serde_key_omission(&field.attrs);
    if is_optional
        || container_defaulted
        || omission.defaulted
        || omission.skips_deserializing
        || !omission.omits_key
    {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: field `{ident}` is left out of the serialized object by its serde \
             attribute, but serde still requires the key when deserializing, so the payload it \
             writes cannot be read back. Add #[serde(default)] to the field (or to the type), or \
             use `skip` if the field should never be read from the payload."
        ),
    ))
}

/// The serde-read guard errors the field violates. A hidden serde attribute leaves every
/// serde-read diagnostic unreliable — the `Option`-null and `nullable`-key guards included — since
/// `field_validation_guard_error` reads no serde attribute and so stands whatever the wrapper hid.
#[cfg(feature = "serde")]
fn field_guard_errors(
    field: &Field,
    raw_field_ident: &str,
    is_optional: bool,
    is_nullable: bool,
    serde_field_meta: &SerdeFieldMeta,
    container: &FieldContext<'_>,
    field_validation_guard_error: Option<proc_macro2::TokenStream>,
) -> Vec<proc_macro2::TokenStream> {
    // A member of a variant serde neither writes nor reads has no key for either check to read.
    let keyed = |check: Result<(), syn::Error>| {
        if container.on_the_wire { check } else { Ok(()) }
    };
    field_validation_guard_error
        .into_iter()
        .chain(serde_field_meta.cfg_attr_rejection.as_ref().map_or_else(
            || {
                keyed(check_optional_field_serialization(
                    field,
                    is_optional,
                    is_nullable,
                ))
                .and_then(|()| check_nullable_field_serialization(field, is_nullable))
                .and_then(|()| {
                    keyed(check_omitted_key_is_readable(
                        field,
                        is_optional,
                        container.container_defaulted,
                    ))
                })
                .err()
                .map(|err| err.to_compile_error())
            },
            |rejection| {
                Some(cfg_attr_guard_error(
                    rejection,
                    &field_label(raw_field_ident),
                ))
            },
        ))
        .collect()
}

/// Every guard error the field violates: the two the type earns — the undescribable-std guard and
/// the map-key guard, neither of which any attribute can hide — then everything the
/// `model_schema_prop` attribute earned, then the serde-side guards when any fired.
fn collect_field_guard_errors(
    field: &Field,
    field_def: &FieldDef,
    written_def: &FieldDef,
    raw_field_ident: &str,
    prop_meta: &ModelSchemaPropMeta,
    serde_guard_errors: Vec<proc_macro2::TokenStream>,
) -> Vec<proc_macro2::TokenStream> {
    let label = field_label(raw_field_ident);

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let map_key_error = check_map_key(field, field_def, &label)
        .err()
        .map(|err| err.to_compile_error());
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let map_key_error = ranges::map_key_error(field, field_def, &label);

    #[cfg(feature = "kotlin")]
    let kotlin_width_error = check_kotlin_width_field(field, field_def, &label)
        .err()
        .map(|err| err.to_compile_error());
    #[cfg(not(feature = "kotlin"))]
    let kotlin_width_error: Option<proc_macro2::TokenStream> = None;

    #[cfg(feature = "serde")]
    let flattened_range_error = ranges::flattened_range_error(field, field_def, &label)
        .or_else(|| ranges::flattened_result_error(field, field_def, &label));
    #[cfg(not(feature = "serde"))]
    let flattened_range_error: Option<proc_macro2::TokenStream> = None;

    check_undescribable_std_field(field, field_def, &label)
        .err()
        .map(|err| err.to_compile_error())
        .into_iter()
        .chain(map_key_error)
        .chain(kotlin_width_error)
        .chain(flattened_range_error)
        .chain(model_schema_prop_guard_errors(
            field,
            field_def,
            written_def,
            &label,
            prop_meta,
        ))
        .chain(serde_guard_errors)
        .collect()
}

/// Rejects a field that reaches `u64` or `usize` at any depth, under the `kotlin` feature. Kotlin
/// has a `ULong` that could carry `u64`; the refusal exists instead for parity with the Swift
/// target, which refuses the same two widths.
#[cfg(feature = "kotlin")]
fn check_kotlin_width_field(
    field: &Field,
    field_def: &FieldDef,
    label: &str,
) -> Result<(), syn::Error> {
    let Some(width) = kotlin_refused_width(field_def) else {
        return Ok(());
    };
    Err(syn::Error::new_spanned(
        field,
        prefixed_guard_message(&format!(
            "{label}: `{width}` has no Kotlin mapping; the Kotlin target refuses unsigned 64-bit \
             and pointer-sized integers"
        )),
    ))
}

/// Every guard the field's `model_schema_prop` attribute earns: what the parser refused, then an
/// unparseable `pattern`, then a bound written where the type renders none, an `as` naming a type
/// the field is not written as, and the misuses the flag validators answer for.
fn model_schema_prop_guard_errors(
    field: &Field,
    field_def: &FieldDef,
    written_def: &FieldDef,
    label: &str,
    prop_meta: &ModelSchemaPropMeta,
) -> Vec<proc_macro2::TokenStream> {
    let refusals = [
        check_fixed_shape_constraints(field, field_def, prop_meta, label).err(),
        check_literal_kind_match(field, field_def, prop_meta, label).err(),
        check_literal_alone(field, prop_meta, label).err(),
        check_as_type_override(field, written_def, prop_meta, label).err(),
        check_as_preprocess_conflict(field, prop_meta, label).err(),
        flag_guard_error(
            field,
            label,
            validate_ts_optional_flag(field, field_def, prop_meta.ts_optional),
        ),
        flag_guard_error(
            field,
            label,
            validate_as_number_flag(&field_def.field_type, prop_meta.as_number),
        ),
        flag_guard_error(
            field,
            label,
            validate_nullable_flag(field_def.is_optional(), prop_meta.nullable),
        ),
        flag_guard_error(field, label, check_nullable_ts_optional_conflict(prop_meta)),
    ];

    prop_meta
        .attr_rejection
        .as_ref()
        .map(|rejection| attr_guard_error(rejection, label))
        .into_iter()
        .chain(
            prop_meta
                .pattern_rejection
                .as_ref()
                .map(|rejection| pattern_guard_error(rejection, label)),
        )
        .chain(
            refusals
                .into_iter()
                .flatten()
                .map(|err| err.to_compile_error()),
        )
        .collect()
}

/// Turns a flag validator's refusal into a spanned error at the field that carries the flag. The
/// validator keeps its message — the one place the misuse is spelled — so a caller doesn't
/// maintain the same sentence twice.
fn flag_guard_error(field: &Field, label: &str, result: Result<(), String>) -> Option<syn::Error> {
    result.err().map(|message| {
        syn::Error::new_spanned(
            field,
            prefixed_guard_message(&format!("{label}: {message}")),
        )
    })
}

/// The bound keys a `model_schema_prop` meta carries, named as they were written, so a guard
/// refusing them can point at the keys to remove rather than at the attribute as a whole.
fn written_constraint_keys(prop_meta: &ModelSchemaPropMeta) -> Vec<&'static str> {
    let mut written = written_keys_of(prop_meta, BoundKind::Text);
    written.extend(written_keys_of(prop_meta, BoundKind::Range));
    written
}

/// The keys of one kind a `model_schema_prop` meta carries, named as they were written.
fn written_keys_of(prop_meta: &ModelSchemaPropMeta, kind: BoundKind) -> Vec<&'static str> {
    let keys = match kind {
        BoundKind::Range => vec![
            ("minimum", prop_meta.minimum.is_some()),
            ("maximum", prop_meta.maximum.is_some()),
        ],
        BoundKind::Scalar => vec![
            ("pattern", prop_meta.pattern.is_some()),
            ("minimum", prop_meta.minimum.is_some()),
            ("maximum", prop_meta.maximum.is_some()),
        ],
        BoundKind::Text => vec![
            ("minLength", prop_meta.min_length.is_some()),
            ("maxLength", prop_meta.max_length.is_some()),
            ("pattern", prop_meta.pattern.is_some()),
        ],
    };
    keys.into_iter()
        .filter_map(|(key, written)| written.then_some(key))
        .collect()
}

/// Where a rule for each item of a list is written, said for the keys a list was refused: a
/// brand carries a `pattern` and no range.
fn item_rule_advice(prop_meta: &ModelSchemaPropMeta) -> String {
    let brand = "A rule for each item goes on the item's own type: declare the item as a \
                 `#[serde(transparent)]` brand that carries the `pattern`.";
    let none = "A brand carries `pattern`, `minLength` and `maxLength` and no range, so a range \
                for each number of a list has no place to be written.";
    let ranged = !written_keys_of(prop_meta, BoundKind::Range).is_empty();
    match (prop_meta.pattern.is_some(), ranged) {
        (true, true) => format!("{brand} {none}"),
        (true, false) => brand.to_owned(),
        (false, _) => none.to_owned(),
    }
}

/// What a field refused a range writes, as the refusal names it, and the keys that bound it where
/// any does.
const fn unranged_value(field_type: &FieldDefType) -> (&'static str, Option<&'static str>) {
    if matches!(field_type, FieldDefType::String) {
        ("a string", Some("`minLength`, `maxLength` or `pattern`"))
    } else if matches!(field_type, FieldDefType::Boolean) {
        ("a boolean", None)
    } else if matches!(field_type, FieldDefType::Char) {
        ("one character", None)
    } else {
        ("a value no range measures", None)
    }
}

/// Rejects a bound of the kind the field's own value is not measured by: a length or a pattern on
/// a number, a range on a string, a boolean, a character or a value no range measures, a pattern
/// or a range on a list.
fn check_bound_kind(
    field: &Field,
    field_def: &FieldDef,
    prop_meta: &ModelSchemaPropMeta,
    label: &str,
) -> Result<(), syn::Error> {
    let Some(kind) = field_def.unmeasured_bound() else {
        return Ok(());
    };
    let misplaced = written_keys_of(prop_meta, kind);
    if misplaced.is_empty() {
        return Ok(());
    }
    let keys = misplaced.join("`, `");
    let declared = written_spelling(&field.ty);
    let (measured, written, kept) = match kind {
        BoundKind::Range => {
            let (written, kept) = unranged_value(&field_def.field_type);
            ("a range is measured on a number", written, kept)
        }
        BoundKind::Scalar => {
            let advice = item_rule_advice(prop_meta);
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "model_schema: {label}: `{keys}` cannot apply to a `{declared}` field — a key \
                     measures the field's own value, and this field writes a list, which has no \
                     text for a pattern and no number for a range. `minLength` and `maxLength` \
                     count the list's items, and no key written here reaches them. {advice}"
                ),
            ));
        }
        BoundKind::Text => (
            "a length or a pattern is measured on a string",
            "a number",
            Some("`minimum` or `maximum`"),
        ),
    };
    let advice = kept.map_or_else(
        || "Drop it.".to_owned(),
        |bounding| format!("Bound {written} with {bounding}, or drop it."),
    );
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: {label}: `{keys}` cannot apply to a `{declared}` field — {measured}, \
             and this field writes {written}: the constraint would reach neither Zod, nor the \
             JSON schema, nor the generated validator. {advice}"
        ),
    ))
}

/// Rejects a length, pattern or range written on a field no surface reads one beside.
fn check_fixed_shape_constraints(
    field: &Field,
    field_def: &FieldDef,
    prop_meta: &ModelSchemaPropMeta,
    label: &str,
) -> Result<(), syn::Error> {
    let written = written_constraint_keys(prop_meta);
    if written.is_empty() {
        return Ok(());
    }
    check_bound_kind(field, field_def, prop_meta, label)?;
    // What is left on a list counts its items, whatever they are.
    if field_def.is_list() {
        return Ok(());
    }
    let keys = written.join("`, `");
    if let Some(name) = field_def.fixed_shape_name() {
        return Err(syn::Error::new_spanned(
            field,
            format!(
                "model_schema: {label}: `{keys}` cannot apply to `{name}` — this crate writes that \
                 type's schema whole, not as the plain string or number a bound is spelled \
                 against, and no surface reads a bound beside it: the constraint would reach \
                 neither Zod, nor the JSON schema, nor the generated validator. Drop it, or carry \
                 the value in a `String` field the bound can measure."
            ),
        ));
    }
    if let Some(shape) = field_def.composite_shape_name() {
        return Err(syn::Error::new_spanned(
            field,
            format!(
                "model_schema: {label}: `{keys}` cannot apply to {shape} — a map renders its keys \
                 and its values, a tuple renders each of its elements, a range renders its start \
                 and its end, a result renders the value of its arm, and every surface builds \
                 those from the members: the bound names no \
                 value here, and `model_schema_prop` has no way to say which member it meant, so \
                 the constraint would reach neither Zod, nor the JSON schema, nor the generated \
                 validator. Constrain the member instead — declare its type as a branded newtype \
                 carrying the bound — or drop it."
            ),
        ));
    }
    if let Some((name, generic)) = field_def.named_shape() {
        let range = written_keys_of(prop_meta, BoundKind::Range);
        if !range.is_empty() {
            let range_keys = range.join("`, `");
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "model_schema: {label}: `{range_keys}` cannot apply to `{name}` — a range is \
                     spelled against a number this crate writes itself, and `{name}` is a type \
                     declared elsewhere, which every surface writes by its name: the constraint \
                     would reach neither Zod, nor the JSON schema, nor the generated validator. \
                     Carry the value in a numeric field the bound can measure, or drop it."
                ),
            ));
        }
        if generic {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "model_schema: {label}: `{keys}` cannot apply to `{name}<…>` — what that type \
                     writes is decided by the arguments it is filled with, so no surface has one \
                     string to measure: the constraint would reach neither Zod, nor the JSON \
                     schema, nor the generated validator. Declare the field as a `String`, or as a \
                     branded newtype over one that declares no type parameter, or drop it."
                ),
            ));
        }
    }
    let Some(parameter) = field_def.parameter_shape_name() else {
        return Ok(());
    };
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: {label}: `{keys}` cannot apply to the type parameter `{parameter}` — \
             the value's type is whatever the instantiation supplies, and one schema is written \
             for every instantiation: Zod and the JSON schema describe the value as the opaque one \
             a bound cannot be spelled against, and neither the generated validator nor serde \
             holds it to anything written here, so the constraint would reach none of them. \
             Constrain the argument instead — declare the type the instantiation supplies as a \
             branded newtype carrying the bound — or drop it."
        ),
    ))
}

/// The `FieldDefType` a `literal` collapses the field to, or `None` when its own kind and the
/// field's declared Rust type disagree — the pair [`check_literal_kind_match`] already refused, with
/// nothing left to render.
fn literal_field_type(literal: &LiteralValue, field_type: &FieldDefType) -> Option<FieldDefType> {
    match literal {
        LiteralValue::Bool(value) => matches!(field_type, FieldDefType::Boolean)
            .then_some(FieldDefType::BooleanLiteral(*value)),
        LiteralValue::Number(value) => field_type
            .is_numeric()
            .then_some(FieldDefType::NumberLiteral(*value)),
        LiteralValue::Str(value) => matches!(field_type, FieldDefType::String)
            .then(|| FieldDefType::StringLiteral(value.clone())),
    }
}

/// The `literal = …` value as the author wrote it, for the mismatch message to name.
fn literal_written(literal: &LiteralValue) -> String {
    match literal {
        LiteralValue::Str(value) => format!("\"{value}\""),
        LiteralValue::Bool(value) => value.to_string(),
        LiteralValue::Number(value) => format_number_literal(*value),
    }
}

/// The adjective a `literal`'s own kind reads as ("a boolean literal…"), and the Rust type that
/// carries it — what [`check_literal_kind_match`] names on either side of the refusal.
const fn literal_kind_words(literal: &LiteralValue) -> (&'static str, &'static str) {
    match literal {
        LiteralValue::Str(_) => ("a string", "a `String`"),
        LiteralValue::Bool(_) => ("a boolean", "a `bool`"),
        LiteralValue::Number(_) => ("a numeric", "a numeric type"),
    }
}

/// Rejects a `literal` whose own kind the field's declared Rust type cannot carry — a boolean
/// literal on a `String` field, a string literal on a `bool` field, and so on: every surface renders
/// the literal in the field's own kind, so a kind neither side agrees on has nothing to render.
fn check_literal_kind_match(
    field: &Field,
    field_def: &FieldDef,
    prop_meta: &ModelSchemaPropMeta,
    label: &str,
) -> Result<(), syn::Error> {
    let Some(literal) = &prop_meta.literal else {
        return Ok(());
    };
    if literal_field_type(literal, &field_def.field_type).is_some() {
        return Ok(());
    }
    let declared_type = &field.ty;
    let declared = quote!(#declared_type).to_string();
    let written = literal_written(literal);
    let (adjective, carrier) = literal_kind_words(literal);
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: {label}: `literal = {written}` cannot apply to a `{declared}` field — \
             {adjective} literal is carried by {carrier}, and the generated TypeScript, Zod and \
             JSON schema all render the literal in the field's own kind. Declare the field as \
             {carrier}, or write the literal as a value the field's own type can carry."
        ),
    ))
}

/// Rejects a `literal` beside any rule: a literal admits one value, so a rule beside it is always
/// met or never met.
fn check_literal_alone(
    field: &Field,
    prop_meta: &ModelSchemaPropMeta,
    label: &str,
) -> Result<(), syn::Error> {
    let Some(literal) = &prop_meta.literal else {
        return Ok(());
    };
    let mut rules = written_constraint_keys(prop_meta);
    if !prop_meta.check.is_empty() {
        rules.push("check");
    }
    if rules.is_empty() {
        return Ok(());
    }
    let written = literal_written(literal);
    let keys = rules.join("`, `");
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: {label}: `literal = {written}` takes no other rule, and `{keys}` is \
             written beside it — a literal admits one value, so the rule is always met or never \
             met. Drop `{keys}`, or drop the literal."
        ),
    ))
}

/// Rejects an `as = Type` naming anything but the type the field already renders.
fn check_as_type_override(
    field: &Field,
    field_def: &FieldDef,
    prop_meta: &ModelSchemaPropMeta,
    label: &str,
) -> Result<(), syn::Error> {
    let Some(as_type) = prop_meta.as_type.as_ref() else {
        return Ok(());
    };
    let target = get_field_def(&field_def.name, as_type, "");
    if *field_def == target || field_def.value_under_wrappers() == target {
        return Ok(());
    }
    let written = quote!(#as_type).to_string();
    let field_type = &field.ty;
    let declared = quote!(#field_type).to_string();
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: {label}: `as = {written}` names a type the field does not render — it is \
             written as `{declared}`. The difference cannot be honored: every surface is written \
             from the declared type, which is the one serde reads and writes, and a \
             `serialize_with` names a function whose output the expansion cannot see, so emitting \
             `{written}` here would describe a payload serde never writes. Declare the field as the \
             type the wire carries, or drop the `as`."
        ),
    ))
}

/// Rejects `as` and `preprocess` written on the same field: `as` names the type the surfaces
/// render and `preprocess` wraps the schema that rendering produced, and this crate defines no
/// order between them.
fn check_as_preprocess_conflict(
    field: &Field,
    prop_meta: &ModelSchemaPropMeta,
    label: &str,
) -> Result<(), syn::Error> {
    if prop_meta.as_type.is_none() || prop_meta.preprocess.is_empty() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        field,
        format!(
            "model_schema: {label}: `as` and `preprocess` cannot be written on the same field — \
             `as` names the type the surfaces render and `preprocess` wraps the schema that \
             rendering produced, and this crate defines no order between the two. Write one or the \
             other."
        ),
    ))
}

/// The std type `written` reaches that this crate describes no schema for, and `None` when it
/// reaches none. The platform string is answered first, so a type reaching both keeps the message
/// that names its actual wire form; the sequence is answered last, its rewrite being the only one
/// of the three that leaves the surrounding type alone.
fn undescribable_std_rejection(written: &FieldDef) -> Option<UndescribableStd<'_>> {
    written
        .os_string_name()
        .map(UndescribableStd::PlatformString)
        .or_else(|| {
            written
                .unsupported_std_wrapper_name()
                .map(UndescribableStd::Wrapper)
        })
        .or_else(|| {
            written
                .refused_sequence_wrapper_name()
                .map(UndescribableStd::Sequence)
        })
}

/// What a written type reaching a std type this crate has no schema for is reported as. The
/// `subject` names where it was written — a field, an alias, a brand, a `default_types` entry, a
/// service operation's message — which is all that differs between the positions.
pub fn undescribable_std_message(subject: &str, rejection: &UndescribableStd<'_>) -> String {
    match *rejection {
        UndescribableStd::AnyJson(name) => format!(
            "{subject} reaches `serde_json::{name}`, which holds any JSON value, so no target can \
             describe it. Declare the shape that value takes as a type of its own, and write that \
             type in its place."
        ),
        UndescribableStd::PlatformString(name) => format!(
            "{subject} reaches `{name}`, which serde writes as an externally tagged enum naming \
             the target platform (`{{\"Unix\":[u8, ...]}}` or `{{\"Windows\":[u16, ...]}}`), not a \
             string, so no schema can describe it portably. Use `String`, or `PathBuf` for a \
             filesystem path."
        ),
        UndescribableStd::Sequence(name) => format!(
            "{subject} reaches `{name}`, which serde writes as the same JSON array `Vec<T>` \
             writes, so nothing on the wire tells the two apart and this crate describes only the \
             one spelling. Use `Vec<T>`, or `VecDeque<T>` where values are pushed at both ends."
        ),
        UndescribableStd::Wrapper(name) => format!(
            "{subject} reaches `{name}`, which serde implements neither `Serialize` nor \
             `Deserialize` for, so there is no wire form for a schema to describe. Store the value \
             `{name}` holds directly, or leave this field out of the serialized shape."
        ),
    }
}

/// Rejects a field that reaches a std type serde has no wire form for, at any depth: left unrefused
/// the name falls through to `FieldDefType::SiblingType` and the expansion references a schema
/// module nothing publishes rather than naming the type the author wrote.
fn check_undescribable_std_field(
    field: &Field,
    field_def: &FieldDef,
    label: &str,
) -> Result<(), syn::Error> {
    let Some(rejection) = undescribable_std_rejection(field_def) else {
        return Ok(());
    };
    Err(syn::Error::new_spanned(
        field,
        prefixed_guard_message(&undescribable_std_message(label, &rejection)),
    ))
}

/// Whether a field is written as a string or a path itself, under `Option`, an ownership wrapper or
/// a reference, which is what an enum's own string check reaches. It is the walk
/// `constrained_shape` makes, held to its two text leaves.
fn written_as_text(ty: &syn::Type) -> bool {
    let mut current = ty;
    loop {
        current = written_type(current);
        if let syn::Type::Reference(reference) = current {
            current = &reference.elem;
            continue;
        }
        let syn::Type::Path(path) = current else {
            return false;
        };
        let Some(segment) = path.path.segments.last() else {
            return false;
        };
        let ident = segment.ident.to_string();
        if matches!(segment.arguments, syn::PathArguments::None) {
            return matches!(ident.as_str(), "String" | "str" | "PathBuf" | "Path");
        }
        let syn::PathArguments::AngleBracketed(held) = &segment.arguments else {
            return false;
        };
        if ident != "Option" && !is_ownership_wrapper(&ident) {
            return false;
        }
        let Some(inner) = sole_type_argument(held) else {
            return false;
        };
        current = inner;
    }
}

/// Whether an enum's own string check reaches `field`: written as a string or a path, and carrying
/// no `literal`, which takes no other rule.
fn reached_by_enums_check(field: &Field) -> bool {
    written_as_text(&field.ty)
        && parse_model_schema_prop_attributes(&field.attrs)
            .literal
            .is_none()
}

/// A member's own `model_schema_prop`, held to the string check of the enum around it where that
/// check reaches the member: the longer `minLength`, the shorter `maxLength`, and both patterns.
fn member_prop_meta(field: &Field, enum_args: Option<&ModelSchemaArgs>) -> ModelSchemaPropMeta {
    let mut meta = parse_model_schema_prop_attributes(&field.attrs);
    let Some(args) = enum_args.filter(|_| reached_by_enums_check(field)) else {
        return meta;
    };
    meta.min_length = meta.min_length.max(args.min_length);
    meta.max_length = match (meta.max_length, args.max_length) {
        (Some(own), Some(held)) => Some(own.min(held)),
        (own, held) => own.or(held),
    };
    if meta.pattern.is_none() {
        meta.pattern.clone_from(&args.pattern);
    }
    #[cfg(any(
        feature = "serde",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift"
    ))]
    if meta.pattern != args.pattern {
        meta.enum_pattern.clone_from(&args.pattern);
    }
    meta
}

/// Processes a field and returns its definition, optional module items (validators/deserializers),
/// optional `validate_body` (contribution to the type-level `validate()` method), and the
/// `compile_error!` tokens for every guard the field violates. `index` is the field's place in
/// its declaration, which is what names a slot.
fn process_field(
    ctx: &FieldContext<'_>,
    field: &mut Field,
    index: usize,
    deferred_attrs: &mut Vec<Vec<syn::Attribute>>,
) -> (
    FieldDef,
    Option<proc_macro2::TokenStream>,
    Option<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
) {
    // Only the serde side has anything to hang on a field; every other build holds back nothing.
    #[cfg(feature = "serde")]
    let mut injected_attrs: Vec<syn::Attribute> = Vec::new();
    #[cfg(not(feature = "serde"))]
    let injected_attrs: Vec<syn::Attribute> = Vec::new();

    #[cfg(feature = "serde")]
    let serde_field_meta = parse_serde_field_attributes(&field.attrs);
    #[cfg(feature = "serde")]
    let field_rename = serde_field_meta.rename.clone();
    #[cfg(not(feature = "serde"))]
    let field_rename: Option<String> = None;

    let raw_field_ident = field_ident_string(field);

    let model_schema_prop_meta = member_prop_meta(field, ctx.enum_args);

    let new_attrs = declaration_attrs(field);

    let final_name =
        get_final_field_name(&raw_field_ident, field_rename.as_deref(), ctx.rename_all);
    let place = member_place(field, index, ctx.variant_ident.is_some(), ctx.steps(index));

    // Generate validation code and hold back the serde attribute it hangs on the field
    #[cfg(feature = "serde")]
    let (mut validation_fn, validate_body, field_validation_guard_error) =
        generate_field_validation(
            ctx,
            field,
            index,
            &model_schema_prop_meta,
            // A struct's field, or a tagged variant's: the shape of the payload says which type it
            // is, so a constraint here decides only whether the value is admissible.
            ConstraintGate::Validator,
            &mut injected_attrs,
        );

    #[cfg(not(feature = "serde"))]
    let (validation_fn, validate_body): (
        Option<proc_macro2::TokenStream>,
        Option<proc_macro2::TokenStream>,
    ) = (None, None);
    #[cfg(not(feature = "serde"))]
    let _: &_ = &ctx.schema_module_name;

    field.attrs = new_attrs;

    let field_type: &syn::Type = &field.ty;

    let field_docs = build_jsdoc_body(get_field_docs(field).as_deref(), &final_name);

    let mut field_def = get_field_def(&final_name, field_type, &field_docs);

    // Resolve `Self` references to the concrete type name so recursive fields (e.g. `Vec<Self>`)
    // are treated exactly like `Vec<EnclosingType>`.
    field_def.resolve_self_references(ctx.type_name, ctx.type_parameters);

    // The field as the author spelled it, kept for the one guard that reads a written name: an `as
    // = Type` may name one of the item's own parameters.
    let written_def = field_def.clone();

    // Every other guard, and the constraint docs below, ask what the surfaces render, so the item's
    // own parameters are erased here.
    field_def.erase_type_parameters(ctx.type_parameters);

    apply_serde_key_omission(&mut field_def, field);

    #[cfg(feature = "serde")]
    let serde_guard_errors = field_guard_errors(
        field,
        &raw_field_ident,
        field_def.is_optional(),
        model_schema_prop_meta.nullable,
        &serde_field_meta,
        ctx,
        field_validation_guard_error,
    );
    #[cfg(not(feature = "serde"))]
    let serde_guard_errors: Vec<proc_macro2::TokenStream> = {
        let _: bool = ctx.container_defaulted;
        // Held alive the same way, and for the reason `container_is_read_back` states: the only
        // reader of this flag is the read-hook writer, which this build does not compile.
        let _: bool = ctx.container_read_back;
        // The key guards, its one reader, are serde's.
        let _: bool = ctx.on_the_wire;
        Vec::new()
    };

    let guard_errors = collect_field_guard_errors(
        field,
        &field_def,
        &written_def,
        &raw_field_ident,
        &model_schema_prop_meta,
        serde_guard_errors,
    );

    let own_checks = checks::member_calls(
        &place,
        &field.ty,
        &model_schema_prop_meta.check,
        VALIDATE_SINK,
    );

    apply_model_schema_prop_meta(&mut field_def, model_schema_prop_meta, &final_name);

    // The read-time half of the reach `validate()` has below: what the field's own type turns
    // away before `validate()` is ever asked.
    #[cfg(feature = "serde")]
    if let Some((hook, attrs)) = member_read_hook(
        ctx,
        field,
        &field_def,
        index,
        validate_body.as_ref(),
        place.reported.as_deref(),
    ) {
        validation_fn = Some(match validation_fn {
            Some(already) => quote! { #already #hook },
            None => hook,
        });
        injected_attrs.extend(attrs);
    }

    #[cfg(feature = "serde")]
    let declared = named_member_validate_body(validate_body, field, &field_def, &place);
    #[cfg(not(feature = "serde"))]
    let declared = validate_body;
    let body = then_checks(declared, own_checks);

    #[cfg(feature = "serde")]
    if let Some(attr) = as_number_serde_hook(field, &field_def) {
        injected_attrs.push(attr);
    }

    deferred_attrs.push(injected_attrs);

    (field_def, validation_fn, body, guard_errors)
}

/// The wrapper chain a field's own type is reached through, where the field is a reference to a
/// declared type under an `Option`, a list, the transparent wrappers and every borrow, and `None`
/// for any other.
///
/// Which types are is the crate's own answer rather than a second list: every leaf a surface
/// renders itself — a primitive, a date, an `ObjectId`, a map, a tuple, one of the item's own
/// parameters — is no reference to a declared type.
#[cfg(feature = "serde")]
fn reachable_nested_shape(field: &Field, field_def: &FieldDef) -> Option<Vec<ConstraintWrap>> {
    matches!(field_def.field_type, FieldDefType::SiblingType(_, _))
        .then(|| nested_shape(&field.ty))
        .flatten()
}

/// [`held_shape`] of a named member, of a flattened one and of an untagged variant's slot. A set, a
/// deque and a heap are described as a declared type is, so such a member was walked to whatever
/// it holds before a map and a tuple were read, and still is.
#[cfg(feature = "serde")]
fn member_held_shape(field: &Field, field_def: &FieldDef) -> Option<HeldShape> {
    held_shape(&field.ty, field_def).or_else(|| {
        reachable_nested_shape(field, field_def).map(|wraps| HeldShape {
            end: HeldEnd::Declared,
            wraps,
        })
    })
}

/// The `validate()` contribution of what a member's *own type* declares — a constrained brand, or
/// a nested `#[model_schema()]` type, held itself, in a list, as a map's value or in a tuple — or
/// `None` where there is nothing below it to run. `place` is where the member is read and the name
/// it is reported under, which is none for a member serde writes as the value of what holds it.
#[cfg(feature = "serde")]
fn nested_validate_body(
    field: &Field,
    field_def: &FieldDef,
    place: &MemberPlace,
) -> Option<proc_macro2::TokenStream> {
    member_held_shape(field, field_def).map(|shape| build_nested_validation(&shape, place))
}

/// [`nested_validate_body`] for a slot of a tuple struct and of a tagged variant, read by what its
/// def holds alone.
#[cfg(feature = "serde")]
fn slot_nested_validate_body(
    field: &Field,
    field_def: &FieldDef,
    place: &MemberPlace,
) -> Option<proc_macro2::TokenStream> {
    held_shape(&field.ty, field_def).map(|shape| build_nested_validation(&shape, place))
}

/// The wrappers a named field's own type is reached through, where a refusal it raises as it is
/// read is answered under the field's key, and `None` for every field it is answered as it is. A
/// field read through a reference has no hook: serde reads no reference to a model.
#[cfg(feature = "serde")]
fn named_below(
    field: &Field,
    field_def: &FieldDef,
    ctx: &FieldContext<'_>,
) -> Option<Vec<ConstraintWrap>> {
    ctx.schema_module_name?;
    field.ident.as_ref()?;
    if !ctx.container_read_back
        || is_flattened_field(field)
        || names_what_the_module_cannot_repeat(&field.ty, ctx.type_parameters)
    {
        return None;
    }
    let serde_meta = parse_serde_field_attributes(&field.attrs);
    if serde_meta.skip || has_serde_read_hook(&field.attrs) {
        return None;
    }
    reachable_nested_shape(field, field_def).filter(|wraps| !ends_on_a_borrow(wraps))
}

/// Gives a member of a struct, or of a tagged variant, the hook that reads the models it holds
/// unjudged, where the item is read back and `validate()` walks what the member holds. Answers
/// whether it was given one. `reported` is the name `validate()` reports the member under.
#[cfg(feature = "serde")]
fn read_unjudged_below(
    ctx: &FieldContext<'_>,
    field: &Field,
    field_def: &FieldDef,
    index: usize,
    reported: Option<&str>,
) -> bool {
    if !ctx.container_read_back {
        return false;
    }
    let walked = if field.ident.is_some() {
        member_held_shape(field, field_def)
    } else {
        held_shape(&field.ty, field_def)
    };
    let Some(shape) = walked else {
        return false;
    };
    judged_read::hook_member(
        field,
        &shape,
        &judged_read::HookPlace {
            choosing: false,
            container_defaulted: ctx.container_defaulted,
            index,
            renamed_under: named_below(field, field_def, ctx).and(reported),
            variant: ctx.variant_ident,
            written_under_a_key: field.ident.is_some() && !is_flattened_field(field),
        },
    )
}

/// The hook a member's own type is read through as the item is read, with the attributes that
/// hang it on the field. What `validate()` walks is read unjudged, through a hook of the item's
/// own read, the whole value being judged once it is read: `None` then, and for every member no
/// hook is hung on.
#[cfg(feature = "serde")]
fn member_read_hook(
    ctx: &FieldContext<'_>,
    field: &Field,
    field_def: &FieldDef,
    index: usize,
    own_bound: Option<&proc_macro2::TokenStream>,
    reported: Option<&str>,
) -> Option<(proc_macro2::TokenStream, Vec<syn::Attribute>)> {
    // A member's own bound stands in for what its type declares, a list's count apart, and a
    // flattened member is walked whatever it writes.
    let walked_below = own_bound.is_none() || field_def.is_list() || is_flattened_field(field);
    if walked_below && read_unjudged_below(ctx, field, field_def, index, reported) {
        return None;
    }
    named_read_hook(field, field_def, ctx, &field_ident_string(field), reported?)
}

/// The read-time hook for a named field whose *own type* carries the bound — a constrained brand,
/// or a nested `#[model_schema()]` type holding one — together with the serde attributes that hang
/// it on the field. `None` for every field no hook may be hung on. `reported` is the name
/// `validate()` reports the field under.
#[cfg(feature = "serde")]
fn named_read_hook(
    field: &Field,
    field_def: &FieldDef,
    ctx: &FieldContext<'_>,
    raw_field_ident: &str,
    reported: &str,
) -> Option<(proc_macro2::TokenStream, Vec<syn::Attribute>)> {
    let module_name = ctx.schema_module_name?;
    let wraps = named_below(field, field_def, ctx)?;

    let stem = helper_name_stem(raw_field_ident, ctx.variant_ident);
    let hook_ident = proc_macro2::Ident::new(
        &format!("{NAMED_READ_HOOK_PREFIX}{stem}"),
        proc_macro2::Span::call_site(),
    );
    let path_lit = syn::LitStr::new(
        &format!("{module_name}::{hook_ident}"),
        proc_macro2::Span::call_site(),
    );
    let mut attrs: Vec<syn::Attribute> = vec![syn::parse_quote! {
        #[serde(deserialize_with = #path_lit)]
    }];
    // A `deserialize_with` turns off serde's reading of a missing `Option` key as `None`, and the
    // `default` puts it back. Only alongside the hook.
    if needs_injected_default(&wraps, has_serde_default(&field.attrs)) {
        attrs.push(syn::parse_quote! { #[serde(default)] });
    }
    Some((
        build_named_read_hook(&hook_ident, &field.ty, reported),
        attrs,
    ))
}

/// The hook itself: read the field's declared type, and where the refusal is one of this crate's
/// own bound sentences, answer it again with `reported`, the holding field's name, written into
/// every violation it carries.
#[cfg(feature = "serde")]
fn build_named_read_hook(
    hook_ident: &proc_macro2::Ident,
    field_ty: &syn::Type,
    reported: &str,
) -> proc_macro2::TokenStream {
    let renaming = judged_read::named_refusal(reported);
    quote! {
        pub fn #hook_ident<'de, D>(deserializer: D) -> ::core::result::Result<#field_ty, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::Deserialize;
            <#field_ty>::deserialize(deserializer).map_err(#renaming)
        }
    }
}

/// Reads a joined report back into the violations it was built from.
#[cfg(feature = "serde")]
fn joined_violations_fn() -> proc_macro2::TokenStream {
    quote! {
        fn joined_violations(reported: &str) -> ::std::vec::Vec<&str> {
            let mut found: ::std::vec::Vec<&str> = ::std::vec::Vec::new();
            let mut rest = reported;
            loop {
                let cut = rest.match_indices("; ").find_map(|(at, _)| {
                    let (head, tail) = rest.split_at(at);
                    let tail = tail.strip_prefix("; ")?;
                    reports_a_bound(tail).then_some((head, tail))
                });
                match cut {
                    ::core::option::Option::Some((head, tail)) => {
                        found.push(head);
                        rest = tail;
                    }
                    ::core::option::Option::None => {
                        found.push(rest);
                        return found;
                    }
                }
            }
        }
    }
}

/// Whether a written type says anything a free function in the schema module cannot repeat: a
/// lifetime it borrows for, `Self`, or one of the item's own type parameters.
#[cfg(feature = "serde")]
fn names_what_the_module_cannot_repeat(ty: &syn::Type, type_parameters: &[String]) -> bool {
    let written = quote! { #ty }.to_string();
    if written.contains('\'') {
        return true;
    }
    written
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .any(|word| word == "Self" || type_parameters.iter().any(|named| named == word))
}

/// What a member contributes to `validate()`: its own check, or the one its type declares where
/// it writes none. A list runs both: its own check counts it, and its items' type holds each item.
/// `place` is where the member is read and the name it is reported under.
#[cfg(feature = "serde")]
fn named_member_validate_body(
    own: Option<proc_macro2::TokenStream>,
    field: &Field,
    field_def: &FieldDef,
    place: &MemberPlace,
) -> Option<proc_macro2::TokenStream> {
    if own.is_some() && !field_def.is_list() {
        return own;
    }
    let below = if field.ident.is_some() {
        nested_validate_body(field, field_def, place)
    } else {
        slot_nested_validate_body(field, field_def, place)
    };
    slot_validate_body(own, below)
}

/// What a member adds to `validate()`: what its bounds and its type declare, then the checks it
/// names.
fn then_checks(
    declared: Option<proc_macro2::TokenStream>,
    checks: Option<proc_macro2::TokenStream>,
) -> Option<proc_macro2::TokenStream> {
    match (declared, checks) {
        (Some(held), Some(named)) => Some(quote! { #held #named }),
        (held, named) => held.or(named),
    }
}

/// What a flattened variant member keeps of `validate()`, bound under the name the arm will match
/// it by: the walk of what its type declares and the checks it names, the hop writing no key of
/// its own.
fn flattened_member_walk(
    ctx: &FieldContext<'_>,
    field: &Field,
    field_def: &FieldDef,
    index: usize,
    bound: &mut Vec<BoundMember>,
    checks: &mut Vec<proc_macro2::TokenStream>,
) {
    let place = member_place(field, index, true, ctx.steps(index));
    #[cfg(feature = "serde")]
    let declared = nested_validate_body(field, field_def, &place);
    #[cfg(not(feature = "serde"))]
    let declared: Option<proc_macro2::TokenStream> = None;
    let named = held_member_checks(field, field_def, &place);
    if let Some(body) = then_checks(declared, named) {
        bound.push(bound_member(field, index));
        checks.push(body);
    }
}

/// What the checks a member's def holds add to `validate()`, read and reported at `place`: those
/// of a flattened member, and of a member of an untagged variant.
fn held_member_checks(
    field: &Field,
    field_def: &FieldDef,
    place: &MemberPlace,
) -> Option<proc_macro2::TokenStream> {
    let named = &field_def.model_schema_prop_meta.as_ref()?.check;
    checks::member_calls(place, &field.ty, named, VALIDATE_SINK)
}

/// What an untagged slot's arm runs: the slot's own check, then the one its type declares.
#[cfg(feature = "serde")]
fn slot_validate_body(
    own: Option<proc_macro2::TokenStream>,
    nested: Option<proc_macro2::TokenStream>,
) -> Option<proc_macro2::TokenStream> {
    match (own, nested) {
        (Some(declared), Some(below)) => Some(quote! { #declared #below }),
        (declared, below) => declared.or(below),
    }
}

/// One named member's entry in the arm's bindings, bound under the name its check already reads.
fn named_bound_member(field_ident: &proc_macro2::Ident, index: usize) -> BoundMember {
    BoundMember {
        binding: member_binding(field_ident),
        index,
        named: Some(field_ident.clone()),
    }
}

/// A member's entry in the arm's bindings: a named member under its ident, a slot by its position.
pub fn bound_member(field: &Field, index: usize) -> BoundMember {
    field.ident.as_ref().map_or_else(
        || BoundMember {
            binding: positional_member_binding(index),
            index,
            named: None,
        },
        |ident| named_bound_member(ident, index),
    )
}

/// Whether the walk reads through a reference anywhere: no hook is emitted for such a field, whose
/// bound is the validator's alone.
#[cfg(feature = "serde")]
fn ends_on_a_borrow(wraps: &[ConstraintWrap]) -> bool {
    wraps
        .iter()
        .any(|wrap| matches!(wrap, ConstraintWrap::Borrowed))
}

/// Whether the field needs a `#[serde(default)]` written for it alongside the `deserialize_with`.
#[cfg(feature = "serde")]
fn needs_injected_default(wraps: &[ConstraintWrap], has_default: bool) -> bool {
    let first_opaque = wraps
        .iter()
        .find(|wrap| !matches!(**wrap, ConstraintWrap::Transparent));
    matches!(first_opaque, Some(ConstraintWrap::Optional)) && !has_default
}

/// The steps serde writes on the way to each member of `fields`, below `prefix`, in the order the
/// members are declared. The only slot of a struct, and of a variant, is the value serde writes
/// for it.
fn steps_of_members(
    fields: &syn::Fields,
    prefix: &[String],
    rename_all: Option<&str>,
) -> Vec<Vec<String>> {
    let lone = matches!(fields, syn::Fields::Unnamed(slots) if slots.unnamed.len() == 1);
    member_steps(fields, prefix, rename_all, lone)
        .into_iter()
        .map(|(_, _, steps)| steps)
        .collect()
}

/// [`steps_of_members`] for each variant of `item_enum`, below what serde writes ahead of the
/// variant's members. `rename_all_fields` cases the members of a variant that names no casing of
/// its own.
fn steps_of_variants(
    item_enum: &syn::ItemEnum,
    rename_all_fields: Option<&str>,
) -> Vec<Vec<Vec<String>>> {
    item_enum
        .variants
        .iter()
        .map(|variant| {
            let (_, rename_all) = variant_serde_names(&variant.attrs, rename_all_fields);
            steps_of_members(
                &variant.fields,
                &variant_steps(item_enum, variant),
                rename_all.as_deref(),
            )
        })
        .collect()
}

/// Where `validate()` reads a member, and the name it reports it under: the one seam a bound and
/// a check of the member are both written from. The name is the steps serde writes on the way to
/// the member, joined by dots, and none where serde writes the value itself.
fn member_place(field: &Field, index: usize, in_variant: bool, steps: &[String]) -> MemberPlace {
    // A variant's member is bound by its arm, a struct's is read off `self`.
    let access = if in_variant {
        MemberAccess::VariantBinding
    } else {
        MemberAccess::SelfField
    };
    let checked = field.ident.as_ref().map_or_else(
        || slot_access_expr(access, index),
        |member| member_access_expr(access, member),
    );
    MemberPlace {
        checked,
        reported: (!steps.is_empty()).then(|| steps.join(".")),
    }
}

/// How a constrained member's helpers name and reach it. It is reported under `steps`, the ones
/// serde writes on the way to it, and its helpers keep its Rust name, or a slot's place among the
/// ones declared.
#[cfg(feature = "serde")]
fn checked_member(
    field: &Field,
    index: usize,
    variant_ident: Option<&str>,
    steps: &[String],
) -> CheckedMember {
    let place = member_place(field, index, variant_ident.is_some(), steps);
    let label = field
        .ident
        .as_ref()
        .map_or_else(String::new, |member| member.unraw().to_string());
    CheckedMember {
        checked: place.checked,
        stem: helper_name_stem(&declared_as(field, index), variant_ident),
        label,
        reported: place.reported,
    }
}

/// What a member's helpers are named from: its Rust name, or a slot's position.
#[cfg(feature = "serde")]
fn declared_as(field: &Field, index: usize) -> String {
    field
        .ident
        .as_ref()
        .map_or_else(|| index.to_string(), |member| member.unraw().to_string())
}

/// The `compile_error!` tokens for a length or range constraint written on a field reached through
/// an interior-mutability wrapper (`RefCell`, `Cell`, `Mutex`, `RwLock`) — unlike `Box`/`Rc`/`Arc`/
/// `Cow`, these don't implement `Deref`, so the validator has no safe way to reach the inner value.
#[cfg(feature = "serde")]
fn interior_mutability_constraint_guard_error(
    field: &Field,
    raw_field_ident: &str,
    wrapper: &str,
) -> proc_macro2::TokenStream {
    syn::Error::new_spanned(
        field,
        format!(
            "model_schema: {}: pattern, minLength, maxLength, minimum and maximum are unsupported \
             on a field reached through `{wrapper}` — unlike `Box`, `Rc`, `Arc` and `Cow`, it does \
             not implement `Deref`, so the generated validator has no safe way to reach its inner \
             value. Drop the constraint, or move the value out from under `{wrapper}`.",
            field_label(raw_field_ident)
        ),
    )
    .to_compile_error()
}

/// Generates per-field serde validation code — the static validator and the `deserialize_with`
/// hook, both published into the schema module either way — and, where `gate` says the read is
/// also a gate, collects the `#[serde(deserialize_with = ...)]` attribute into `injected_attrs`,
/// plus the `#[serde(default)]` that keeps an optional key optional under one. Each violation is
/// reported under the steps serde writes on the way to the field.
#[cfg(feature = "serde")]
fn generate_field_validation(
    ctx: &FieldContext<'_>,
    field: &Field,
    index: usize,
    model_schema_prop_meta: &ModelSchemaPropMeta,
    gate: ConstraintGate,
    injected_attrs: &mut Vec<syn::Attribute>,
) -> (
    Option<proc_macro2::TokenStream>,
    Option<proc_macro2::TokenStream>,
    Option<proc_macro2::TokenStream>,
) {
    let has_string_constraints = model_schema_prop_meta.min_length.is_some()
        || model_schema_prop_meta.max_length.is_some()
        || model_schema_prop_meta.pattern.is_some();
    let has_numeric_constraints =
        model_schema_prop_meta.minimum.is_some() || model_schema_prop_meta.maximum.is_some();

    let member = checked_member(field, index, ctx.variant_ident, ctx.steps(index));
    // Recorded under the stem the member's helpers are named from, for the recovering decode.
    record_member_checks(
        &ident_schema_module_name(ctx.type_name),
        &member.stem,
        &model_schema_prop_meta.check,
    );

    let (Some(module_name), Some(shape)) = (ctx.schema_module_name, constrained_shape(&field.ty))
    else {
        if (has_string_constraints || has_numeric_constraints)
            && let Some(wrapper) = blocking_interior_mutability_wrapper(&field.ty)
        {
            return (
                None,
                None,
                Some(interior_mutability_constraint_guard_error(
                    field,
                    &member.label,
                    &wrapper,
                )),
            );
        }
        return (None, None, None);
    };

    let counts_items =
        model_schema_prop_meta.min_length.is_some() || model_schema_prop_meta.max_length.is_some();
    let generated = match &shape.leaf {
        ConstraintLeaf::List => counts_items
            .then(|| generate_list_validation_code(&member, model_schema_prop_meta, &shape.wraps)),
        ConstraintLeaf::Named(_) | ConstraintLeaf::Path | ConstraintLeaf::Str => {
            has_string_constraints.then(|| {
                generate_string_validation_code(&member, model_schema_prop_meta, &shape, &field.ty)
            })
        }
        ConstraintLeaf::Number(rust_type) => has_numeric_constraints.then(|| {
            generate_numeric_validation_code(
                &member,
                rust_type,
                model_schema_prop_meta,
                &shape,
                &field.ty,
            )
        }),
    };
    let Some(validation_code) = generated else {
        return (None, None, None);
    };

    let helper_stem = &member.stem;
    if gate == ConstraintGate::Validator {
        record_field_validator(module_name, helper_stem, member.reported.as_deref());
    }
    // A borrow publishes no hook, and a member its author reads through a function of their own
    // keeps that one: serde admits one reader per field. Its bound is the validator's alone.
    if gate == ConstraintGate::Deserializer
        && !ends_on_a_borrow(&shape.wraps)
        && !has_serde_read_hook(&field.attrs)
    {
        let deserialize_with_path = format!("{module_name}::deserialize_{helper_stem}");
        let path_lit = syn::LitStr::new(&deserialize_with_path, proc_macro2::Span::call_site());
        injected_attrs.push(syn::parse_quote! {
            #[serde(deserialize_with = #path_lit)]
        });
        // Only alongside the hook.
        if needs_injected_default(&shape.wraps, has_serde_default(&field.attrs)) {
            injected_attrs.push(syn::parse_quote! {
                #[serde(default)]
            });
        }
    }

    (
        Some(validation_code.module_items),
        Some(bound_body(validation_code.validate_body, module_name)),
        None,
    )
}

/// What a member's bound adds to `validate()`. It calls a validator of the schema module by its
/// name alone, which `validate()` imports once for them all. Where the item names a check the
/// bound imports the module for itself instead, so the check is called where no import of the
/// module is in scope, and its name is the function its author wrote.
#[cfg(feature = "serde")]
fn bound_body(body: proc_macro2::TokenStream, module_name: &str) -> proc_macro2::TokenStream {
    if !names_a_check() {
        return body;
    }
    let module = proc_macro2::Ident::new(module_name, proc_macro2::Span::call_site());
    quote! {
        {
            use #module::*;
            #body
        }
    }
}

/// Hands the field the `model_schema_prop` metadata the surfaces read it back off, and applies the
/// one key that names the type rather than constrains it. On a list, `minLength` and `maxLength`
/// become the count of its items, which is where every surface writes them.
fn apply_model_schema_prop_meta(
    field_def: &mut FieldDef,
    mut prop_meta: ModelSchemaPropMeta,
    final_name: &str,
) {
    if field_def.is_list() {
        prop_meta.min_items = prop_meta.min_length.take();
        prop_meta.max_items = prop_meta.max_length.take();
    }
    field_def.model_schema_prop_meta = (prop_meta.as_type.is_some()
        || prop_meta.literal.is_some()
        || prop_meta.min_items.is_some()
        || prop_meta.max_items.is_some()
        || prop_meta.min_length.is_some()
        || prop_meta.max_length.is_some()
        || prop_meta.pattern.is_some()
        || prop_meta.minimum.is_some()
        || prop_meta.maximum.is_some()
        || prop_meta.ts_optional
        || prop_meta.as_number
        || prop_meta.nullable
        || !prop_meta.preprocess.is_empty()
        || !prop_meta.check.is_empty())
    .then_some(prop_meta);

    if let Some(meta) = &field_def.model_schema_prop_meta
        && let Some(literal) = &meta.literal
        && let Some(collapsed) = literal_field_type(literal, &field_def.field_type)
    {
        field_def.field_type = collapsed;
    }

    apply_constraint_docs(field_def, final_name);
}

/// Appends length/range constraint information to a field's generated docs — only where something
/// actually holds the value to the bound. A placement [`check_fixed_shape_constraints`] refuses
/// gets none, since the doc would be the only place the claim appeared.
fn apply_constraint_docs(field_def: &mut FieldDef, final_name: &str) {
    if field_def.constraints_reach_nothing() {
        return;
    }
    let Some(meta) = &field_def.model_schema_prop_meta else {
        return;
    };
    let unmeasured = field_def.unmeasured_bound();
    let mut constraint_docs: Vec<String> = Vec::new();
    if unmeasured != Some(BoundKind::Text) {
        if let Some(min_len) = meta.min_length.or(meta.min_items) {
            constraint_docs.push(format!(" * Minimum length: {min_len}"));
        }
        if let Some(max_len) = meta.max_length.or(meta.max_items) {
            constraint_docs.push(format!(" * Maximum length: {max_len}"));
        }
    }
    if !matches!(unmeasured, Some(BoundKind::Range | BoundKind::Scalar)) {
        if let Some(minimum) = meta.minimum {
            constraint_docs.push(format!(" * Minimum value: {minimum}"));
        }
        if let Some(maximum) = meta.maximum {
            constraint_docs.push(format!(" * Maximum value: {maximum}"));
        }
    }
    if !constraint_docs.is_empty() {
        let extra_docs = constraint_docs.join("\n");
        field_def.docs = if field_def.docs.is_empty() {
            format!(" * {final_name}\n * \n{extra_docs}")
        } else {
            format!("{}\n{}", field_def.docs, extra_docs)
        };
    }
}

/// Gets the serialized name of a struct field. serde cases fields by different rules than enum
/// variants, so the two must not share one entry point.
fn get_final_field_name(
    name: &str,
    field_rename: Option<&str>,
    rename_all: Option<&str>,
) -> String {
    field_rename.map_or_else(
        || resolve_rename_rule(rename_all).apply_to_field(name),
        str::to_owned,
    )
}

fn get_final_variant_name(
    name: &str,
    variant_rename: Option<&str>,
    rename_all: Option<&str>,
) -> String {
    variant_rename.map_or_else(
        || resolve_rename_rule(rename_all).apply_to_variant(name),
        str::to_owned,
    )
}

#[cfg(feature = "jsonschema")]
/// Generates the JSON schema method conditionally based on the jsonschema feature.
fn generate_json_schema_method(
    json_schema_fields: &[proc_macro2::TokenStream],
    flatten_json_schemas: &[MergedSource],
    def_name: &str,
    parameters: &[SchemaParameter],
) -> proc_macro2::TokenStream {
    generate_struct_json_schema_method_impl(
        json_schema_fields,
        flatten_json_schemas,
        def_name,
        parameters,
    )
}

/// What a struct's `#[serde(flatten)]` fields contribute to the object it writes.
#[cfg(feature = "jsonschema")]
fn flatten_merged_sources(flattened_fields: &[FieldDef]) -> Vec<MergedSource> {
    flattened_fields.iter().map(flatten_merged_source).collect()
}

/// What a value whose members join the object being written contributes to it, labelled with the
/// name the author gave it: the type for a named one, the field otherwise — a shape with no name of
/// its own is only ever pointed at through where it was written.
#[cfg(feature = "jsonschema")]
fn flatten_merged_source(fld: &FieldDef) -> MergedSource {
    if let FieldDefType::SiblingType(name, arguments) = &fld.field_type {
        return MergedSource {
            entries: flattened_as_entries(fld),
            label: name.clone(),
            map: false,
            optional: fld.is_optional(),
            unit_member: unit_member_record(name, fld.type_span),
            value: sibling_json_schema_value(name, arguments, fld.type_span),
        };
    }
    // A map joins as its own description, as it is described under a key; a key no surface
    // writes adds no key.
    let (value, map) = if let FieldDefType::TypeParam(parameter) = &fld.field_type {
        (json_argument_value(parameter), false)
    } else if let FieldDefType::Map(key, value) = &fld.field_type
        && fld.array_depth == 0
        && !matches!(map_key_path(key), MapKeyPath::Refused(_))
    {
        let described = map_json_schema_value(key, value)
            .unwrap_or_else(|rejection| map_member_rejection_error(&fld.name, &rejection));
        (described, true)
    } else {
        (quote! { serde_json::json!({ "type": "object" }) }, false)
    };
    MergedSource {
        entries: flattened_as_entries(fld),
        label: fld.name.clone(),
        map,
        optional: fld.is_optional(),
        unit_member: quote! { false },
        value,
    }
}

/// Whether the type `name` stands for publishes the record of an untagged enum with a member
/// serde writes as `()`, read where the merge runs: a type without it answers through the
/// block-local trait, an inherent const being found ahead of a trait's.
#[cfg(feature = "jsonschema")]
fn unit_member_record(name: &str, span: proc_macro2::Span) -> proc_macro2::TokenStream {
    let module_ident = sibling_schema_module_ident(name, span);
    let record = Ident::new(UNIT_MEMBER_RECORD, proc_macro2::Span::call_site());
    let recorded = quote_spanned! {span=> #module_ident::Schema::#record };
    quote! {
        {
            trait WritesNoUnitMember {
                const #record: bool = false;
            }
            impl<T: ?::core::marker::Sized> WritesNoUnitMember for T {}
            <()>::#record || #recorded
        }
    }
}

/// The record [`unit_member_record`] reads, as an item of its schema module's `Schema`: an
/// untagged enum with a member serde writes as `()` says so, and an alias, a brand or a struct of
/// one slot whose value is a name, written bare, answers as that name does. Nothing for any other
/// item.
#[cfg(all(feature = "serde", feature = "jsonschema"))]
fn unit_member_record_item(item: &Item) -> proc_macro2::TokenStream {
    let record = Ident::new(UNIT_MEMBER_RECORD, proc_macro2::Span::call_site());
    let value = if let Item::Enum(item_enum) = item {
        let unit = matches!(Tagging::of(item_enum), Tagging::Untagged)
            && non_object_wire(item)
                .members
                .iter()
                .any(|leaf| leaf.branch.len() == 1 && leaf.written_as == Some(WrittenAs::Unit));
        return if unit {
            quote! { pub const #record: bool = true; }
        } else {
            proc_macro2::TokenStream::new()
        };
    } else if let Item::Struct(item_struct) = item {
        written_as_its_slot(item_struct).map(|slot| &slot.ty)
    } else if let Item::Type(alias) = item {
        Some(&*alias.ty)
    } else {
        None
    };
    // A wrapper is left out: two types each over the other behind one would read each other's
    // record.
    let named = value.filter(|ty| {
        let syn::Type::Path(path) = written_type(ty) else {
            return false;
        };
        path.qself.is_none()
            && path.path.segments.last().is_some_and(|last| {
                let name = last.ident.to_string();
                name != "Option" && !is_transparent_wrapper(&name)
            })
    });
    let parameters = item_generics(item).map_or_else(Vec::new, type_parameters_in_scope);
    let Some(def) = named.map(|ty| {
        let mut def = get_field_def("_written", ty, "");
        def.erase_type_parameters(&parameters);
        def
    }) else {
        return proc_macro2::TokenStream::new();
    };
    if let FieldDefType::SiblingType(name, _) = &def.field_type
        && def.array_depth == 0
        && !def.is_optional()
        && !is_sequence_wrapper(name)
    {
        let forwarded = unit_member_record(name, def.type_span);
        return quote! { pub const #record: bool = #forwarded; };
    }
    proc_macro2::TokenStream::new()
}

/// Nothing, in a build that reads no `#[serde(flatten)]`.
#[cfg(all(not(feature = "serde"), feature = "jsonschema"))]
fn unit_member_record_item(_item: &Item) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

/// The description of the keys of a map whose key serde writes as the text of a value that is no
/// string: a `char` and a chrono date or time by their own descriptions, a number and a `bool` by
/// the pattern of their text, and a type that writes one by its own description, read as that
/// text. `None` for every other key.
#[cfg(feature = "jsonschema")]
fn text_key_names(key: &FieldDef) -> Option<proc_macro2::TokenStream> {
    if !matches!(map_key_path(key), MapKeyPath::Unnarrowed) {
        return None;
    }
    let pattern = match &key.field_type {
        FieldDefType::Char => {
            return scalar_field_json_schema_item(key)
                .map(|item| quote! { serde_json::json!(#item) });
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => {
            return chrono_json_schema_item(&key.field_type)
                .map(|item| quote! { serde_json::json!(#item) });
        }
        FieldDefType::SiblingType(name, arguments) => {
            return Some(named_key_text(&sibling_json_schema_value(
                name,
                arguments,
                key.type_span,
            )));
        }
        FieldDefType::Boolean => BOOLEAN_KEY_PATTERN,
        FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::Usize => INTEGER_KEY_PATTERN,
        FieldDefType::F32 | FieldDefType::F64 => NUMBER_KEY_PATTERN,
        FieldDefType::BooleanLiteral(_)
        | FieldDefType::Map(..)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Range(..)
        | FieldDefType::Result(..)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::Tuple(..)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown => return None,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => return None,
    };
    Some(quote! { serde_json::json!({ "type": "string", "pattern": #pattern }) })
}

/// `described`, the description of a type a map is keyed by, as the text serde writes the key as:
/// a number's and a `bool`'s by the pattern of that text, any other as it stands. Read where the
/// description is built, which states what a brand, an alias or a type filled at its arguments is.
#[cfg(feature = "jsonschema")]
fn named_key_text(described: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let text =
        |pattern: &str| quote! { serde_json::json!({ "type": "string", "pattern": #pattern }) };
    let (integer, number, boolean) = (
        text(INTEGER_KEY_PATTERN),
        text(NUMBER_KEY_PATTERN),
        text(BOOLEAN_KEY_PATTERN),
    );
    quote! {
        ({
            let named_key: serde_json::Value = #described;
            let named_key_type = named_key.get("type").and_then(serde_json::Value::as_str);
            if named_key_type == ::core::option::Option::Some("integer") {
                #integer
            } else if named_key_type == ::core::option::Option::Some("number") {
                #number
            } else if named_key_type == ::core::option::Option::Some("boolean") {
                #boolean
            } else {
                named_key
            }
        })
    }
}

/// The ` & A & B` an object's own block, of the members `own_keys`, is closed with, one operand per
/// merged source.
#[cfg(feature = "typescript")]
fn ts_intersection_suffix(own_keys: &[String], operands: &[MergedOperand]) -> String {
    ts_merged_operands(own_keys, operands)
        .iter()
        .fold(String::new(), |mut acc, operand| {
            let _ = write!(acc, " & {operand}");
            acc
        })
}

/// What each merged source is written as inside the intersection, its absence leaving out the keys
/// the object's own members `own_keys` and the other sources read: Zod's branch without the source
/// reads them.
#[cfg(feature = "typescript")]
fn ts_merged_operands(own_keys: &[String], operands: &[MergedOperand]) -> Vec<String> {
    operands
        .iter()
        .enumerate()
        .map(|(at, operand)| {
            let beside: Vec<String> = own_keys
                .iter()
                .map(|key| ts_quoted(key))
                .chain(
                    operands
                        .iter()
                        .enumerate()
                        .filter(|(other, _)| *other != at)
                        .map(|(_, other)| format!("keyof NonNullable<{}>", other.spelling)),
                )
                .collect();
            ts_merged_operand(operand, &beside.join(" | "))
        })
        .collect()
}

/// The name each of `field_defs` is written under.
fn member_names(field_defs: &[FieldDef]) -> Vec<String> {
    field_defs.iter().map(|fld| fld.name.clone()).collect()
}

/// The ` & A & B` a variant's own object block, of the members `own_keys`, is closed with, empty
/// where the variant flattens nothing.
#[cfg(feature = "typescript")]
fn variant_flatten_typescript(own_keys: &[String], flattened_fields: &[FieldDef]) -> String {
    ts_intersection_suffix(own_keys, &compute_flatten_outputs(flattened_fields).0)
}

/// Nothing closes the block where the surface is off.
#[cfg(not(feature = "typescript"))]
const fn variant_flatten_typescript(
    _own_keys: &[String],
    _flattened_fields: &[FieldDef],
) -> String {
    String::new()
}

/// What a variant's object is written as with its `#[serde(flatten)]` sources merged in: `own`
/// closed with one `.and(...)` chain where the sources write a single key set between them, and one
/// closed copy of `own` per key set inside a `z.union` where they write more.
#[cfg(feature = "zod")]
fn variant_flatten_zod(own: &str, flattened_fields: &[FieldDef]) -> String {
    zod_merged_object(own, &compute_flatten_outputs(flattened_fields).1)
}

/// What one merged source is written as inside the intersection, its absence leaving out the keys
/// `beside` names, a TypeScript union of keys, where it names any.
#[cfg(feature = "typescript")]
fn ts_merged_operand(operand: &MergedOperand, beside: &str) -> String {
    let spelling = &operand.spelling;
    let absent = |keys: String| {
        if beside.is_empty() {
            format!("({spelling} | {{ [K in {keys}]?: never }})")
        } else {
            format!("({spelling} | {{ [K in Exclude<{keys}, {beside}>]?: never }})")
        }
    };
    match operand.absence {
        SourceAbsence::Never => spelling.clone(),
        SourceAbsence::Field => absent(format!("keyof {spelling}")),
        SourceAbsence::Published => absent(format!("keyof NonNullable<{spelling}>")),
    }
}

#[cfg(feature = "typescript")]
/// Generates the TypeScript definition method (TypeScript types only, no Zod schema).
fn generate_ts_definition_method(
    docs: &str,
    item_name: &str,
    rust_ident: &str,
    ts_generics: &str,
    type_code: &str,
    own_keys: &[String],
    flatten_types: &[MergedOperand],
) -> proc_macro2::TokenStream {
    let reexport = ident_reexport_ts(rust_ident, item_name, ts_generics);
    let fields_empty = own_keys.is_empty();
    let has_flatten = !flatten_types.is_empty();
    let intersection_only = ts_merged_operands(own_keys, flatten_types).join(" & ");
    let intersection_suffix = ts_intersection_suffix(own_keys, flatten_types);

    let typescript_type_gen = if fields_empty {
        if has_flatten {
            quote::quote! {
                format!("{}export type {}{} = {};{}", docs, #item_name, #ts_generics, #intersection_only, #reexport)
            }
        } else {
            quote::quote! {
                format!("{}export type {}{} = Record<string, never>;{}", docs, #item_name, #ts_generics, #reexport)
            }
        }
    } else if has_flatten {
        quote::quote! {
            format!("{}export type {}{} = {{\n{}}}{};{}", docs, #item_name, #ts_generics, #type_code, #intersection_suffix, #reexport)
        }
    } else {
        quote::quote! {
            format!("{}export type {}{} = {{\n{}}};{}", docs, #item_name, #ts_generics, #type_code, #reexport)
        }
    };

    let item_docs_gen = bind_item_jsdoc_local(docs);

    quote::quote! {
        pub fn ts_definition() -> ::std::string::String {
            #item_docs_gen
            #typescript_type_gen
        }
    }
}

/// A schema an intersection is built from, written so the name it carries is read when the
/// intersection is used rather than while the `const` holding it is being initialized.
#[cfg(feature = "zod")]
fn deferred_zod_operand(schema: &str) -> String {
    format!("z.lazy(() => {schema})")
}

/// The suffix the binding an item publishes is named with. A generic type publishes a factory —
/// one schema per filling, built on demand — where a type that declares no parameter publishes the
/// one schema it has.
#[cfg(feature = "zod")]
fn zod_binding_suffix(rust_ident: &str, parameters: &[String]) -> &'static str {
    record_zod_factory(rust_ident, !parameters.is_empty());
    if parameters.is_empty() {
        "$Schema"
    } else {
        "$SchemaFactory"
    }
}

/// The zod re-export text an item's module ends with — the factory's own alongside the default's,
/// so a renamed generic item's alias answers to both names exactly as its own declaration does.
/// [`ident_reexport_zod`] writes one binding at a time, so a generic type needs two calls.
#[cfg(feature = "zod")]
fn zod_binding_reexport(rust_ident: &str, export_name: &str, parameters: &[String]) -> String {
    let mut reexport = ident_reexport_zod(
        rust_ident,
        export_name,
        zod_binding_suffix(rust_ident, parameters),
    );
    if !parameters.is_empty() {
        reexport.push_str(&ident_reexport_zod(
            rust_ident,
            export_name,
            "$SchemaDefault",
        ));
    }
    reexport
}

/// The identifier holding the expression a factory's arguments compose into. Bound beside the
/// factory rather than inlined in it, so the factory's return type can be read back off it and
/// neither can drift from the other.
#[cfg(feature = "zod")]
fn zod_factory_builder_name(item_name: &str) -> String {
    format!("build{item_name}$Schema")
}

/// The `TypeScript` parameter list a factory is declared under — `<IdType extends ZodType,
/// DateType extends ZodType>`. A bare `ZodType` annotation would compile but infer nothing:
/// `ZodType` defaults its own parameters, so a field validated by one comes back `unknown`.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_factory_bounds(parameters: &[String]) -> String {
    format!(
        "<{}>",
        parameters
            .iter()
            .map(|parameter| format!("{parameter} extends ZodType"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The argument list a factory and its builder are written with, one line per parameter. Required,
/// every one of them: a default would let a call site say nothing about a filling and still get a
/// schema back, which is the silent mis-validation the factory exists to prevent.
#[cfg(feature = "zod")]
fn zod_factory_arguments(parameters: &[String]) -> String {
    parameters.iter().fold(String::new(), |mut acc, parameter| {
        let argument = zod_factory_argument(parameter);
        #[cfg(feature = "typescript")]
        let _ = write!(acc, "\n  {argument}: {parameter},");
        #[cfg(not(feature = "typescript"))]
        let _ = write!(acc, "\n  {argument},");
        acc
    })
}

/// The private key a factory hangs its memo on, one per item so no two share a slot.
#[cfg(feature = "zod")]
fn zod_cache_name(item_name: &str) -> String {
    format!("{item_name}$SchemaFactoryCache")
}

/// What that key holds: the schema itself where the item declares one parameter, and a `WeakMap`
/// chain keyed by the arguments after the first where it declares more. Every type parameter in it
/// is bound by the factory's own signature — a dependency a module-scope store cannot express.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_cache_type(item_name: &str, parameters: &[String]) -> String {
    let widened = parameters
        .iter()
        .map(|_| "ZodType")
        .collect::<Vec<_>>()
        .join(", ");
    parameters.iter().skip(1).fold(
        format!("WeakMap<ZodType, {item_name}$SchemaOf<{widened}>>"),
        |below, _| format!("WeakMap<ZodType, {below}>"),
    )
}

/// The implementation signature's parameter list — every argument widened to `ZodType`, the type
/// the store is keyed at.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_factory_widened_arguments(parameters: &[String]) -> String {
    parameters.iter().fold(String::new(), |mut acc, parameter| {
        let _ = write!(acc, "\n  {}: ZodType,", zod_factory_argument(parameter));
        acc
    })
}

/// The factory's own parameter list: the arguments the builder takes, with the first carrying the
/// optional memo the factory reads and writes.
#[cfg(feature = "zod")]
fn zod_factory_declaration(item_name: &str, parameters: &[String], bounds: &str) -> String {
    #[cfg(not(feature = "typescript"))]
    let written = {
        let _: (&str, &str) = (item_name, bounds);
        format!(
            "export function {item_name}$SchemaFactory({}\n)",
            zod_factory_arguments(parameters)
        )
    };
    #[cfg(feature = "typescript")]
    let written = {
        let widened = parameters
            .iter()
            .map(|_| "ZodType")
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "export function {item_name}$SchemaFactory{bounds}({}\n): {item_name}$SchemaOf<{}>;\n\
             export function {item_name}$SchemaFactory({}\n): {item_name}$SchemaOf<{widened}>",
            zod_factory_arguments(parameters),
            parameters.join(", "),
            zod_factory_widened_arguments(parameters)
        )
    };
    written
}

/// The factory's body: walk down to the map the last argument keys, hand back what is already
/// there, and otherwise build once, store, and return the very schema that was stored — making two
/// calls with the same arguments the same schema rather than two that merely agree.
#[cfg(feature = "zod")]
fn zod_factory_body(item_name: &str, parameters: &[String]) -> String {
    let arguments: Vec<String> = parameters.iter().map(|p| zod_factory_argument(p)).collect();
    let built = zod_factory_memoized_binding(item_name, parameters);
    let mut body = String::new();
    let mut holder = zod_cache_name(item_name);
    for depth in 1..parameters.len() {
        let below = format!("by{}", parameters[depth]);
        let key = &arguments[depth - 1];
        let _ = write!(
            body,
            "  let {below} = {holder}.get({key});\n  if (!{below}) {{\n    {below} = new \
             WeakMap();\n    {holder}.set({key}, {below});\n  }}\n\n"
        );
        holder = below;
    }
    let last = arguments.last().map_or_else(String::new, Clone::clone);
    let _ = write!(
        body,
        "  const hit = {holder}.get({last});\n  if (hit) return hit;\n\n  {built};\n  \
         {holder}.set({last}, schema);\n  return schema;"
    );
    body
}

/// The statement a factory binds the schema it is about to cache to, without its terminator.
/// Written once and read twice: [`zod_factory_body`] emits it, and the `zod_schema()` injecting
/// the item's examples anchors on it.
#[cfg(feature = "zod")]
fn zod_factory_memoized_binding(item_name: &str, parameters: &[String]) -> String {
    format!(
        "const schema = {}({})",
        zod_factory_builder_name(item_name),
        parameters
            .iter()
            .map(|parameter| zod_factory_argument(parameter))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The `FieldDef` one `default_types` entry parses as: the declared filling for `parameter`, or —
/// absent one — `String`, [`schema_example_value_type`]'s own fallback for the identical gap. Only
/// `jsonschema` requires a declared default; the constrained-brand guard reads this regardless.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn declared_default_field(parameter: &str, default_types: &[(syn::Ident, syn::Type)]) -> FieldDef {
    default_types
        .iter()
        .find(|(declared, _)| declared == parameter)
        .map_or_else(
            || get_field_def(parameter, &syn::parse_quote!(String), ""),
            |(_, ty)| get_field_def(parameter, ty, ""),
        )
}

/// The rendering one declared default composes: the ordinary rendering [`FieldDef::zod_type`]
/// gives every field, deferred wherever that rendering names another item's own module-scope
/// binding — its factory, called with fresh arguments exactly as an ordinary field naming it does,
/// or, where the default names that item at exactly the arguments it calls its own, that item's
/// `$SchemaDefault` directly (the fold).
#[cfg(feature = "zod")]
fn default_zod_rendering(field: &FieldDef) -> DefaultZodRendering {
    if field.array_depth == 0
        && !field.is_optional()
        && let FieldDefType::SiblingType(name, args) = &field.field_type
    {
        if publishes_zod_factory(name)
            && let Some(info) = lookup_alias_info(name)
        {
            let rendered: Vec<String> = args.iter().map(FieldDef::zod_type).collect();
            if zod_default_arguments(name).as_deref() == Some(rendered.as_slice()) {
                return DefaultZodRendering::Deferred(format!(
                    "{}$SchemaDefault",
                    info.export_name
                ));
            }
        }
        return DefaultZodRendering::Deferred(field.zod_type());
    }
    if field.names_a_sibling_binding() {
        return DefaultZodRendering::Deferred(field.zod_type());
    }
    DefaultZodRendering::Eager(field.zod_type())
}

/// What a generic item appends after its factory: the factory called with each parameter's
/// declared default, so `X$SchemaDefault === X$SchemaFactory(<the same arguments>)` by
/// construction, through the very factory a hand-written call would go through.
#[cfg(feature = "zod")]
fn zod_default_block(
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    defaults: &ZodDefaultInputs<'_>,
) -> String {
    let fields: Vec<FieldDef> = parameters
        .iter()
        .map(|parameter| declared_default_field(parameter, defaults.default_types))
        .collect();
    let fold_keys: Vec<String> = fields.iter().map(FieldDef::zod_type).collect();
    record_zod_default_arguments(rust_ident, fold_keys);
    let renderings: Vec<DefaultZodRendering> = fields.iter().map(default_zod_rendering).collect();

    let arguments: Vec<String> = parameters
        .iter()
        .zip(renderings)
        .map(|(parameter, rendering)| match defaults.constrained {
            Some((target, checks)) if target == parameter.as_str() => match rendering {
                DefaultZodRendering::Eager(schema) => {
                    format!("{schema}{}", checks.chained)
                }
                DefaultZodRendering::Deferred(schema) => {
                    deferred_zod_operand(&format!("{schema}{}", checks.base))
                }
            },
            _ => rendering.into_argument(),
        })
        .collect();

    let call = format!("{item_name}$SchemaFactory({})", arguments.join(", "));

    #[cfg(feature = "typescript")]
    if defaults.annotated_by_value {
        return raw_default_block(item_name, &call);
    }

    #[cfg(feature = "typescript")]
    let annotation = format!(
        ": ZodType<{item_name}<{}>>",
        fields
            .iter()
            .map(FieldDef::typescript_typename)
            .collect::<Vec<_>>()
            .join(", ")
    );
    #[cfg(not(feature = "typescript"))]
    let annotation = String::new();

    format!("\n\nexport const {item_name}$SchemaDefault{annotation} = {call};")
}

/// The `$SchemaDefault` of an item whose annotation is read back off the value: the call is bound
/// to a raw `const` first so the export can name its type, the same two lines [`zod_const_block`]
/// writes one level up. A generic item has no `$RawSchema` of its own to name.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn raw_default_block(item_name: &str, call: &str) -> String {
    format!(
        "\n\nconst {item_name}$RawSchemaDefault = {call};\n\nexport const \
         {item_name}$SchemaDefault: typeof {item_name}$RawSchemaDefault = \
         {item_name}$RawSchemaDefault;"
    )
}

/// The name a factory's return type is read back under. A builder that defers a reference may
/// reach its own type through it, and a declaration file writes such a type only by name: an
/// interface is printed as its name, where an alias is printed as the structure it resolves to,
/// with `any` at the point that structure recurs.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_schema_of(
    item_name: &str,
    bounds: &str,
    builder: &str,
    parameters: &[String],
    preamble: &str,
    expression: &str,
) -> String {
    let read_back = format!(
        "ReturnType<\n  typeof {builder}<{}>\n>",
        parameters.join(", ")
    );
    if defers_a_reference(preamble) || defers_a_reference(expression) {
        format!("interface {item_name}$SchemaOf{bounds} extends {read_back} {{}}")
    } else {
        format!("type {item_name}$SchemaOf{bounds} = {read_back};")
    }
}

/// Whether `zod` holds a member written behind a getter or an operand written behind `z.lazy`,
/// the two spellings a reference is deferred in.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn defers_a_reference(zod: &str) -> bool {
    zod.contains("() { return ") || zod.contains("z.lazy(() => ")
}

/// What a generic type's Zod surface is written as, with each map the type holds of itself read
/// through its self view where a TypeScript type is there to declare the view with.
#[cfg(feature = "zod")]
fn zod_factory_block(
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    defaults: &ZodDefaultInputs<'_>,
    preamble: &str,
    expression: &str,
    reexport: &str,
) -> String {
    #[cfg(feature = "typescript")]
    {
        zod_factory_block_around(
            item_name,
            rust_ident,
            parameters,
            defaults,
            &maps_of_itself_through_the_self_view(item_name, parameters, preamble),
            &maps_of_itself_through_the_self_view(item_name, parameters, expression),
            reexport,
        )
    }
    #[cfg(not(feature = "typescript"))]
    {
        zod_factory_block_around(
            item_name, rust_ident, parameters, defaults, preamble, expression, reexport,
        )
    }
}

/// The builder holding the schema a generic type's arguments compose into, the return type read
/// back off it, the cache interfaces, and the exported factory.
#[cfg(feature = "zod")]
fn zod_factory_block_around(
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    defaults: &ZodDefaultInputs<'_>,
    preamble: &str,
    expression: &str,
    reexport: &str,
) -> String {
    let builder = zod_factory_builder_name(item_name);
    let arguments = zod_factory_arguments(parameters);
    let body = zod_factory_body(item_name, parameters);

    #[cfg(feature = "typescript")]
    let bounds = zod_factory_bounds(parameters);
    #[cfg(not(feature = "typescript"))]
    let bounds = String::new();

    // A merged object binds its own keys ahead of the branches that read them, and the keys are
    // composed from the arguments, so the binding sits inside the builder.
    let built = if preamble.is_empty() {
        format!("const {builder} = {bounds}({arguments}\n) =>\n  {expression};")
    } else {
        format!(
            "const {builder} = {bounds}({arguments}\n) => {{\n  {preamble}  return {expression};\n}};"
        )
    };

    let cache = zod_cache_name(item_name);
    #[cfg(feature = "typescript")]
    let declarations = format!(
        "{}\n\nconst {cache} = new {}();\n\n",
        zod_schema_of(
            item_name, &bounds, &builder, parameters, preamble, expression
        ),
        zod_cache_type(item_name, parameters)
    );
    #[cfg(not(feature = "typescript"))]
    let declarations = format!("const {cache} = new WeakMap();\n\n");

    let default_block = zod_default_block(item_name, rust_ident, parameters, defaults);
    let declaration = zod_factory_declaration(item_name, parameters, &bounds);

    #[cfg(feature = "typescript")]
    let self_view = zod_self_view(item_name, parameters, &built);
    #[cfg(not(feature = "typescript"))]
    let self_view = String::new();

    format!(
        "{self_view}{built}\n\n{declarations}{declaration} {{\n{body}\n}}{default_block}{reexport}"
    )
}

/// Whether an item's published Zod expression *is* a sibling's own binding rather than an
/// expression built around one. `array_depth == 0` excludes a sequence, whose element the parser
/// collapses onto without leaving the wrapper under its own name, and `!is_optional()` excludes an
/// `Option`, which renders as a union of its own; either way the schema is newly built and states
/// the item's own type. This is not [`FieldDef::names_a_sibling_binding`], which walks the whole
/// tree and answers for a wrapped sibling too.
#[cfg(feature = "zod")]
fn republishes_sibling_binding(field: &FieldDef) -> bool {
    field.array_depth == 0
        && !field.is_optional()
        && matches!(field.field_type, FieldDefType::SiblingType(..))
}

/// [`republishes_sibling_binding`] for a tuple struct: serde writes a one-slot struct as the slot's
/// value alone, so a bare sibling slot is published verbatim; every other arity builds a `z.tuple`.
#[cfg(feature = "zod")]
fn tuple_struct_republishes_slot(shape: &TupleStructShape) -> bool {
    match shape {
        TupleStructShape::Array(_) => false,
        TupleStructShape::BareValue(slot) => republishes_sibling_binding(slot),
    }
}

/// The binding a type that declares no parameter publishes: the raw schema, then the exported
/// `const` annotated with the type it validates. The annotation is the only place a TypeScript
/// type is named, so a build without `typescript` writes the same value under a bare `const`.
#[cfg(feature = "zod")]
fn zod_const_block(
    item_name: &str,
    preamble: &str,
    expression: &str,
    reexport: &str,
    annotated_by_value: bool,
) -> String {
    #[cfg(feature = "typescript")]
    {
        // `.brand()` narrows at the value position, which restating the item's own type discards,
        // so such a binding reads its type back off what it published.
        let annotation = if annotated_by_value {
            format!("typeof {item_name}$RawSchema")
        } else {
            format!("ZodType<{item_name}>")
        };
        format!(
            "{preamble}const {item_name}$RawSchema = {expression};\n\nexport const \
             {item_name}$Schema: {annotation} = {item_name}$RawSchema;{reexport}"
        )
    }
    #[cfg(not(feature = "typescript"))]
    {
        let _: &_ = &annotated_by_value;
        format!("{preamble}export const {item_name}$Schema = {expression};{reexport}")
    }
}

/// The whole of what a type publishes on the Zod surface: a factory when it declares parameters,
/// and the annotated `const` when it declares none.
#[cfg(feature = "zod")]
fn zod_published_binding(
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    published: &PublishedBinding<'_>,
    preamble: &str,
    expression: &str,
    reexport: &str,
) -> String {
    // On the type's own expression, so every binding of it carries them: the one schema of a
    // type with no parameter, and each schema its builder builds for a type with one. A
    // `preprocess` is written around the checked expression: it corrects what the checks judge.
    if parameters.is_empty() {
        let checked = zod_preprocess_wrap(
            zod_check_wrap(expression.to_owned(), published.checks),
            published.preprocess,
        );
        zod_const_block(
            item_name,
            preamble,
            &checked,
            reexport,
            published.republished,
        )
    } else {
        let defaults = ZodDefaultInputs {
            #[cfg(feature = "typescript")]
            annotated_by_value: published.republished,
            constrained: None,
            default_types: published.default_types,
        };
        let (run_ahead, built) =
            zod_checked_in_a_builder(item_name, parameters, expression, published.checks);
        let factory = zod_factory_block(
            item_name,
            rust_ident,
            parameters,
            &defaults,
            preamble,
            &zod_preprocess_wrap(built, published.preprocess),
            reexport,
        );
        format!("{run_ahead}{factory}")
    }
}

/// The checks of a type that declares parameters, as its builder runs them: the function that
/// runs them, written ahead of the builder, and the builder's expression handed to it. Inside the
/// builder Zod cannot say a member written from a parameter is required, so the value a check is
/// handed there is not the published type. The function's overload hands the schema back as it
/// is, and its implementation takes a schema of the published type at any filling.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_checked_in_a_builder(
    item_name: &str,
    parameters: &[String],
    expression: &str,
    checks: &[FunctionName],
) -> (String, String) {
    if checks.is_empty() {
        return (String::new(), expression.to_owned());
    }
    let any_filling = parameters
        .iter()
        .map(|_| "ZodType[\"_zod\"][\"output\"]")
        .collect::<Vec<_>>()
        .join(", ");
    let run = zod_check_wrap("built".to_owned(), checks);
    (
        format!(
            "function {item_name}$Checked<Built extends ZodType>(built: Built): Built;\nfunction \
             {item_name}$Checked(built: ZodType<{item_name}<{any_filling}>>): ZodType {{\n  \
             return {run};\n}}\n\n"
        ),
        format!("{item_name}$Checked({expression})"),
    )
}

/// Without a TypeScript type to state what a check is handed, the checks are written on the
/// builder's expression itself.
#[cfg(all(feature = "zod", not(feature = "typescript")))]
fn zod_checked_in_a_builder(
    _item_name: &str,
    _parameters: &[String],
    expression: &str,
    checks: &[FunctionName],
) -> (String, String) {
    (String::new(), zod_check_wrap(expression.to_owned(), checks))
}

/// The list a type's `.meta` holds its examples in: each block's value where the type writes one,
/// and `[]` written as text where it writes none, so such a type names no `serde_json`.
#[cfg(feature = "zod")]
fn zod_examples_json(has_example: bool) -> proc_macro2::TokenStream {
    if has_example {
        quote! { serde_json::to_string(&Self::schema_examples()).unwrap() }
    } else {
        quote! { "[]" }
    }
}

/// The `zod_schema()` that publishes what `base` wrote with the item's examples on its binding,
/// ahead of the re-export a renamed item ends with.
#[cfg(feature = "zod")]
fn zod_schema_with_examples(
    base: &proc_macro2::TokenStream,
    item_name: &str,
    parameters: &[String],
    reexport: &str,
    examples_json: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let injected = zod_examples_injection(item_name, parameters);
    quote! {
        pub fn zod_schema() -> ::std::string::String {
            let base_schema = #base;
            let defined = base_schema.strip_suffix(#reexport).unwrap_or(base_schema.as_str());
            let examples_json = #examples_json;
            let mut result = #injected;
            result.push_str(#reexport);
            result
        }
    }
}

/// The expression a `zod_schema()` builds its schema-with-examples from, read off the same decision
/// [`zod_published_binding`] takes: where the examples go depends on what the item published.
#[cfg(feature = "zod")]
fn zod_examples_injection(item_name: &str, parameters: &[String]) -> proc_macro2::TokenStream {
    if parameters.is_empty() {
        return quote! {{
            let examples_part = format!(".meta({{\n  examples: {}\n}})", examples_json);
            if let ::core::option::Option::Some(pos) = defined.rfind(';') {
                let mut injected = defined[..pos].to_string();
                injected.push_str(&examples_part);
                injected.push(';');
                injected
            } else {
                format!("{}{}", defined, examples_part)
            }
        }};
    }
    let binding = zod_factory_memoized_binding(item_name, parameters);
    let anchor = format!("{binding};");
    quote! {
        defined.replace(
            #anchor,
            &format!("{}.meta({{\n    examples: {}\n  }});", #binding, examples_json),
        )
    }
}

/// What one merged source contributes to each branch: one schema per branch of the choice it names,
/// and the source itself when it names none.
#[cfg(feature = "zod")]
fn zod_operand_contributions(operand: &MergedOperand) -> Vec<&str> {
    if operand.branches.is_empty() {
        vec![operand.spelling.as_str()]
    } else {
        operand.branches.iter().map(String::as_str).collect()
    }
}

/// The `.and(...)` chain one per key set the merged sources write between them: one per branch of a
/// source naming a choice, one more without the source wherever it offers its own absence, and the
/// cross product of those across the sources. Always at least one, the empty chain a source-less
/// object closes with. The absence of a source read as entries holds the keys serde drops with it.
#[cfg(feature = "zod")]
fn zod_merged_joins(operands: &[MergedOperand]) -> Vec<String> {
    let mut joins = vec![String::new()];
    for operand in operands {
        joins = joins
            .iter()
            .flat_map(|join| {
                let mut grown: Vec<String> = zod_operand_contributions(operand)
                    .into_iter()
                    .map(|schema| format!("{join}.and({})", deferred_zod_operand(schema)))
                    .collect();
                if operand.absence.offered() {
                    grown.push(if operand.entries {
                        format!("{join}.and({ZOD_DROPPED_ENTRIES})")
                    } else {
                        join.clone()
                    });
                }
                grown
            })
            .collect();
    }
    joins
}

/// What the object's schema is written as: the statements bound ahead of it, and the expression
/// itself.
#[cfg(feature = "zod")]
fn zod_merged_statements(
    item_name: &str,
    own: &str,
    operands: &[MergedOperand],
) -> (String, String) {
    // The root is decided once every combination is counted: one combination writes the object's
    // own keys where they stood, and more bind them to a name.
    let joins = zod_merged_joins(operands);
    if let [only] = joins.as_slice() {
        return (String::new(), format!("{own}{only}"));
    }

    let own_name = format!("{item_name}$OwnSchema");
    let written = joins.iter().fold(String::new(), |mut acc, join| {
        let _ = writeln!(acc, "  {own_name}{join},");
        acc
    });
    (
        format!("const {own_name} = {own};\n\n"),
        format!("z.union([\n{written}])"),
    )
}

/// The same multiplication [`zod_merged_statements`] performs, written where the object stands
/// rather than beside a name for it: one closed copy of `own` per combination, joined in a union
/// once there is more than one.
#[cfg(feature = "zod")]
fn zod_merged_object(own: &str, operands: &[MergedOperand]) -> String {
    let joins = zod_merged_joins(operands);
    if let [only] = joins.as_slice() {
        return format!("{own}{only}");
    }
    let written = joins.iter().fold(String::new(), |mut acc, join| {
        let _ = writeln!(acc, "  {own}{join},");
        acc
    });
    format!("z.union([\n{written}])")
}

#[cfg(feature = "zod")]
/// Each flattened base joins an intersection through [`deferred_zod_operand`]: a base names a
/// `const` of its own, and one macro invocation sees one type, so nothing here can know whether
/// that `const` is declared above this module or below it.
fn generate_zod_schema_method(
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    args: &ModelSchemaArgs,
    schema_code: &str,
    show_opts: &str,
    flatten_schemas: &[MergedOperand],
) -> proc_macro2::TokenStream {
    #[cfg(feature = "zod")]
    {
        let reexport = zod_binding_reexport(rust_ident, item_name, parameters);
        let own = format!("z.strictObject({{\n{schema_code}}}){show_opts}");
        let (preamble, expression) = zod_merged_statements(item_name, &own, flatten_schemas);
        // Note: Example injection is handled by the delegating method on the type itself.
        let body = zod_published_binding(
            item_name,
            rust_ident,
            parameters,
            // A struct always closes an object of its own, so it publishes no sibling's binding.
            &PublishedBinding {
                checks: &args.check,
                default_types: &args.default_types,
                preprocess: &args.preprocess,
                republished: false,
            },
            &preamble,
            &expression,
            &reexport,
        );

        if !holds_a_placeholder(&body) {
            return quote::quote! {
                pub fn zod_schema() -> ::std::string::String {
                    #body.to_owned()
                }
            };
        }
        let written = Ident::new("written", proc_macro2::Span::call_site());
        let replaced = substituted(&quote::quote! { SerdeDefaults }, &written, &[]);
        quote::quote! {
            pub fn zod_schema() -> ::std::string::String {
                let mut #written = ::std::string::String::from(#body);
                #replaced
                #written
            }
        }
    }

    #[cfg(not(feature = "zod"))]
    {
        let _: &_ = &(
            item_name,
            rust_ident,
            parameters,
            args,
            schema_code,
            show_opts,
            flatten_schemas,
        );
        quote::quote! {
            // No method: the `zod` feature is off.
        }
    }
}

/// Binds the `docs` local an item's `ts_definition()` opens with: the item's `JSDoc` block and a
/// trailing newline, so what the block documents is the line straight beneath it.
#[cfg(feature = "typescript")]
fn bind_item_jsdoc_local(docs: &str) -> proc_macro2::TokenStream {
    quote::quote! {
        let docs = format!("/**\n{}\n */\n", #docs);
    }
}

#[cfg(feature = "jsonschema")]
/// Generates the JSON schema method for plain enums conditionally.
fn generate_plain_enum_json_schema_method(
    enumerated: &[proc_macro2::TokenStream],
    def_name: &str,
    parameters: &[SchemaParameter],
) -> proc_macro2::TokenStream {
    #[cfg(feature = "jsonschema")]
    {
        generate_plain_enum_json_schema_method_impl(enumerated, def_name, parameters)
    }

    #[cfg(not(feature = "jsonschema"))]
    {
        let _: &_ = &(enumerated, def_name);
        quote::quote! {
            // No method: the `jsonschema` feature is off.
        }
    }
}

#[cfg(feature = "typescript")]
/// Generates the TypeScript definition method for plain enums (TypeScript types only).
fn generate_plain_enum_ts_definition_method(
    docs: &str,
    item_name: &str,
    rust_ident: &str,
    ts_generics: &str,
    type_code: &str,
    variant_reader: &str,
) -> proc_macro2::TokenStream {
    #[cfg(feature = "typescript")]
    {
        let item_docs_gen = bind_item_jsdoc_local(docs);
        let reexport = ident_reexport_ts(rust_ident, item_name, ts_generics);

        let typescript_type_gen = quote::quote! {
            format!("{}export type {}{} =\n{};{}\n\n{}", docs, #item_name, #ts_generics, #type_code, #reexport, #variant_reader)
        };

        quote::quote! {
            pub fn ts_definition() -> ::std::string::String {
                #item_docs_gen
                #typescript_type_gen
            }
        }
    }

    #[cfg(not(feature = "typescript"))]
    {
        quote::quote! {
            // No method: the `typescript` feature is off.
        }
    }
}

#[cfg(feature = "zod")]
/// A plain enum publishes the one schema it has whatever it declares: Rust refuses a *type*
/// parameter no variant uses, and every variant of a plain enum is a unit, so there is never a
/// parameter for a factory to bind.
fn generate_plain_enum_zod_schema_method(
    item_name: &str,
    rust_ident: &str,
    schema_code: &str,
    description: &str,
    args: &ModelSchemaArgs,
) -> proc_macro2::TokenStream {
    #[cfg(feature = "zod")]
    {
        let reexport = ident_reexport_zod(rust_ident, item_name, "$Schema");
        // The checks the enum names, on the enumeration itself and ahead of its description.
        let checked = zod_check_wrap(String::new(), &args.check);
        // Each function `preprocess` names is written around the checked enumeration: `opened`
        // ahead of it, and `closed` after it.
        let around = zod_preprocess_wrap(String::new(), &args.preprocess);
        let opened = around.trim_end_matches(')');
        let closed = ")".repeat(args.preprocess.len());
        // When typescript feature is enabled, generate TypeScript-style Zod schema
        #[cfg(feature = "typescript")]
        {
            quote::quote! {
                pub fn zod_schema() -> ::std::string::String {
                    format!("const {}$RawSchema = {}z.enum([{}]){}{}.meta({{\n  description: \"{}\",\n}});\n\nexport const {}$Schema: ZodType<{}> = {}$RawSchema;{}", #item_name, #opened, #schema_code, #checked, #closed, #description, #item_name, #item_name, #item_name, #reexport)
                }
            }
        }

        // When typescript feature is disabled, generate JavaScript-style Zod schema
        #[cfg(not(feature = "typescript"))]
        {
            quote::quote! {
                pub fn zod_schema() -> ::std::string::String {
                    format!("export const {}$Schema = {}z.enum([{}]){}{}.meta({{\n  description: \"{}\",\n}});{}", #item_name, #opened, #schema_code, #checked, #closed, #description, #reexport)
                }
            }
        }
    }

    #[cfg(not(feature = "zod"))]
    {
        let _: &_ = &(item_name, rust_ident, schema_code, description, args);
        quote::quote! {
            // No method: the `zod` feature is off.
        }
    }
}

#[cfg(feature = "jsonschema")]
/// Generates the JSON schema method for discriminated enums conditionally.
fn enum_json_schema_methods(
    main_schema_code: &proc_macro2::TokenStream,
    def_name: &str,
    generics: &syn::Generics,
    args: &ModelSchemaArgs,
) -> proc_macro2::TokenStream {
    json_schema_methods(
        def_name,
        &quote::quote! { { #main_schema_code } },
        &schema_parameters(generics, args),
    )
}

#[cfg(feature = "typescript")]
/// Generates the TypeScript definition method for discriminated enums (TypeScript types only).
fn generate_discriminated_enum_ts_definition_method(
    docs: &str,
    item_name: &str,
    rust_ident: &str,
    ts_generics: &str,
    type_code: &str,
    variant_reader: &str,
) -> proc_macro2::TokenStream {
    #[cfg(feature = "typescript")]
    {
        let item_docs_gen = bind_item_jsdoc_local(docs);
        let reexport = ident_reexport_ts(rust_ident, item_name, ts_generics);

        quote::quote! {
            pub fn ts_definition() -> ::std::string::String {
                #item_docs_gen
                let bundled_docs = docs;
                format!(r#"{bundled_docs}export type {}{} = {};{}

{}"#, #item_name, #ts_generics, #type_code, #reexport, #variant_reader)
            }
        }
    }

    #[cfg(not(feature = "typescript"))]
    {
        quote::quote! {
            // No method: the `typescript` feature is off.
        }
    }
}

#[cfg(feature = "zod")]
/// Generates the Zod schema method for discriminated enums (Zod schemas only)
/// Note: Example injection is handled by the delegating method on the type itself.
fn generate_discriminated_enum_zod_schema_method(
    item_name: &str,
    rust_ident: &str,
    parameters: &[String],
    args: &ModelSchemaArgs,
    schema_code: &str,
) -> proc_macro2::TokenStream {
    #[cfg(feature = "zod")]
    {
        let reexport = zod_binding_reexport(rust_ident, item_name, parameters);
        let schema_str = zod_published_binding(
            item_name,
            rust_ident,
            parameters,
            // A union over its variants is an expression of its own, never a sibling's binding.
            &PublishedBinding {
                checks: &args.check,
                default_types: &args.default_types,
                preprocess: &args.preprocess,
                republished: false,
            },
            "",
            schema_code,
            &reexport,
        );
        if !holds_a_placeholder(&schema_str) {
            return quote::quote! {
                pub fn zod_schema() -> ::std::string::String {
                    #schema_str.to_owned()
                }
            };
        }
        let written = Ident::new("written", proc_macro2::Span::call_site());
        let replaced = substituted(&quote::quote! { SerdeDefaults }, &written, &[]);
        quote::quote! {
            pub fn zod_schema() -> ::std::string::String {
                let mut #written = ::std::string::String::from(#schema_str);
                #replaced
                #written
            }
        }
    }

    #[cfg(not(feature = "zod"))]
    {
        let _: &_ = &(item_name, rust_ident, parameters, args, schema_code);
        quote::quote! {
            // No method: the `zod` feature is off.
        }
    }
}

/// Builds the alias module's `ts_definition()`, or nothing when `typescript` is off. The doc
/// block and the generic parameter list are only meaningful to TypeScript, so they are gathered
/// inside the gate rather than by the caller.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn generate_alias_ts_definition_method(
    alias: &ItemType,
    export_name: &str,
    field_def: &FieldDef,
) -> proc_macro2::TokenStream {
    #[cfg(feature = "typescript")]
    {
        let docs_formatted = alias_jsdoc_body(get_item_docs(&alias.attrs).as_deref(), export_name);

        generate_ts_alias_method(
            &docs_formatted,
            export_name,
            &alias.ident.to_string(),
            &ts_generic_params(&alias.generics),
            &surface_field_def(&alias.generics, field_def),
        )
    }
    #[cfg(not(feature = "typescript"))]
    {
        let _: &_ = &(alias, export_name, field_def);
        quote! {}
    }
}

#[cfg(feature = "typescript")]
fn generate_ts_alias_method(
    docs: &str,
    export_name: &str,
    rust_ident: &str,
    ts_generics: &str,
    field_def: &FieldDef,
) -> proc_macro2::TokenStream {
    let alias_name_ts = format!("{export_name}{ts_generics}");
    let target_ts = field_def.typescript_typename();
    let reexport = ident_reexport_ts(rust_ident, export_name, ts_generics);

    let docs_block = jsdoc_block(docs, "");

    quote! {
        pub fn ts_definition() -> ::std::string::String {
            format!(
                "{}\nexport type {} = {};{}",
                #docs_block,
                #alias_name_ts,
                #target_ts,
                #reexport
            )
        }
    }
}

/// The diagnostic an alias whose target the dispatch cannot render emits, in place of the whole
/// `json_schema()` body. Spanned on the target, which is where the unrenderable value was written.
#[cfg(feature = "jsonschema")]
fn alias_json_schema_rejection(
    alias: &ItemType,
    rejection: &MapMemberRejection,
) -> proc_macro2::TokenStream {
    let subject = format!("type alias `{}`", alias.ident);
    let message = prefixed_guard_message(&map_member_rejection_message(&subject, rejection));
    syn::Error::new_spanned(&alias.ty, message).to_compile_error()
}

/// Builds the alias module's `json_schema()`, or nothing when `jsonschema` is off.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn generate_alias_json_schema_method(
    alias: &ItemType,
    export_name: &str,
    field_def: &FieldDef,
    args: &ModelSchemaArgs,
) -> proc_macro2::TokenStream {
    #[cfg(feature = "jsonschema")]
    {
        let body = build_tuple_element_json_schema(&surface_field_def(&alias.generics, field_def))
            .unwrap_or_else(|rejection| alias_json_schema_rejection(alias, &rejection));
        json_schema_methods(
            export_name,
            &body,
            &schema_parameters(&alias.generics, args),
        )
    }
    #[cfg(not(feature = "jsonschema"))]
    {
        // Nothing in this build references an alias module's `json_schema()`; the sibling
        // reference that would (`flatten_field_json_schema_ref`) is itself jsonschema-gated.
        let _: &_ = &(alias, export_name, field_def, args);
        quote! {}
    }
}

/// Builds the alias module's `zod_schema()`, or nothing when `zod` is off.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn generate_alias_zod_method(
    alias: &ItemType,
    export_name: &str,
    rust_ident: &str,
    field_def: &FieldDef,
    default_types: &[(syn::Ident, syn::Type)],
) -> proc_macro2::TokenStream {
    #[cfg(feature = "zod")]
    {
        // The alias's rendered Zod is its `FieldDef` expression: `z.tuple([...])` for a tuple,
        // `z.string()` for a scalar, `Name$Schema` for a sibling.
        let surface = surface_field_def(&alias.generics, field_def);
        let schema_code = surface.zod_type();
        let parameters = type_parameters_in_scope(&alias.generics);
        let reexport = zod_binding_reexport(rust_ident, export_name, &parameters);
        let body = zod_published_binding(
            export_name,
            rust_ident,
            &parameters,
            // A check and a `preprocess` on an alias are refused: it has no `impl` to call a
            // function from, and no read of its own.
            &PublishedBinding {
                checks: &[],
                default_types,
                preprocess: &[],
                republished: republishes_sibling_binding(&surface),
            },
            "",
            &schema_code,
            &reexport,
        );
        // An alias has no `impl` to carry an example on, so its list is the empty one.
        zod_schema_with_examples(
            &quote! { #body.to_owned() },
            export_name,
            &parameters,
            &reexport,
            &zod_examples_json(false),
        )
    }
    #[cfg(not(feature = "zod"))]
    {
        // Without the `zod` feature, `FieldDef::zod_type` does not exist; nothing in
        // this build has zod enabled, so the schema method would be cfg'd out anyway.
        let _: &_ = &(alias, export_name, rust_ident, field_def, default_types);
        quote! {}
    }
}

#[cfg(test)]
mod tests;
