//! `FieldDef` and `FieldDefType`: how a Rust type is read into the description every surface
//! renders from, and the TypeScript and Zod spellings of each.

#[cfg(feature = "serde")]
use core::slice::from_ref;

use syn::{Fields, GenericArgument, Ident, ItemEnum, PathArguments, Type, Variant};

#[cfg(feature = "jsonschema")]
use proc_macro2::Span;
#[cfg(feature = "jsonschema")]
use syn::spanned::Spanned as _;

#[cfg(feature = "serde")]
use syn::Attribute;

#[cfg(feature = "zod")]
use crate::features::model_schema_prop::FunctionName;
use crate::features::model_schema_prop::ModelSchemaPropMeta;
use crate::utils::{MapKeyWire, is_declared_ident, lookup_alias_info, ts_quoted, written_type};

#[cfg(feature = "zod")]
use crate::bound_message::{Bound, zod_error_arg, zod_repeat_check};
#[cfg(feature = "zod")]
use crate::features::serde::defaults::{holds_a_placeholder, placeholder};
#[cfg(feature = "zod")]
use crate::utils::{
    ZodUnionMember, escape_js_regex_literal, names_a_zod_preprocess, publishes_zod_factory,
    zod_factory_argument,
};

#[cfg(any(feature = "zod", all(feature = "serde", feature = "typescript")))]
use crate::utils::placeholders_filled;
#[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
use crate::utils::{FlattenVariant, parameter_placeholder};

#[cfg(feature = "chrono")]
use crate::features::chrono;
#[cfg(feature = "mongodb")]
use crate::features::object_id;

#[cfg(feature = "serde")]
use crate::features::serde::{
    parse_serde_field_attributes as parse_serde_field_attributes_impl,
    parse_serde_type_attributes as parse_serde_type_attributes_impl,
};
// Bring serde metadata types into scope (used by the serde parsing helpers below).
#[cfg(feature = "serde")]
use crate::features::serde::{SerdeFieldMeta, SerdeTypeMeta};

/// The keys serde writes the two arms of a `Result` under, `Ok` first.
pub const RESULT_ARMS: [&str; 2] = ["Ok", "Err"];

/// The two strings serde writes a `bool` key as, in the order both surfaces state them so they
/// cannot drift.
const BOOLEAN_KEY_TYPESCRIPT: &str = "\"true\" | \"false\"";

#[cfg(feature = "zod")]
const BOOLEAN_KEY_ZOD: &str = "z.enum([\"true\", \"false\"])";

/// What `z.array` and `z.union` take. Written bare, as `ZodType` is: a consumer may bind `z` as a
/// constant, which names no type, so nothing emitted writes `z.` where a type is read.
#[cfg(feature = "zod")]
pub const ZOD_SCHEMA_BOUND: &str = "SomeType";

/// A `DateTime<Tz>` key is the RFC 3339 string chrono renders it into, offset always written.
#[cfg(all(feature = "chrono", feature = "zod"))]
const TIMESTAMP_KEY_ZOD: &str = "z.iso.datetime({ offset: true })";

/// The magnitude every `f32` serde reads stays below: a larger one is read as an infinity, which
/// serde then writes as `null`. The midpoint between `f32::MAX` and the next power of two.
#[cfg(any(feature = "zod", feature = "dart"))]
pub const F32_READ_BOUND: &str = "3.4028235677973366e38";

/// The kinds of bound `model_schema_prop` carries, named by what each is measured on: a length or
/// a pattern on a string, a range on a number, and a pattern or a range on one value of either,
/// which a list is not.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundKind {
    Range,
    Scalar,
    Text,
}

/// Which of the four std ranges a [`FieldDefType::Range`] is. serde writes each as the object of
/// the bounds it holds, every bound of the one type the range is written with.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RangeKind {
    /// `RangeFrom<T>`, written `{"start": T}`.
    From,
    /// `Range<T>`, written `{"start": T, "end": T}`.
    HalfOpen,
    /// `RangeInclusive<T>`, written `{"start": T, "end": T}`.
    Inclusive,
    /// `RangeTo<T>`, written `{"end": T}`.
    To,
}

/// The std type a path written with type arguments names, where it names one the field definition
/// reads by its last name.
#[derive(Clone, Copy)]
enum StdType {
    Range(RangeKind),
    Result,
}

/// Classifies how an enum variant stores its data, driving the TypeScript/Zod generation strategy
/// for discriminated union variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantKind {
    /// Named struct fields: `Payment::Card { number: String }`
    /// Generates: `{ type: "Card", number: string }`.
    Named,
    /// Multiple tuple elements: `Value::Complex(String, i64)`
    /// Generates: `{ type: "Complex", value: [string, number] }`.
    TupleMultiple,
    /// Single tuple element: `Value::Text(String)`
    /// Generates: `{ type: "Text", value: string }`.
    TupleSingle,
    /// Unit variant with no fields: `Status::Active`
    /// Generates: `{ type: "Active" }`.
    Unit,
}

/// Enum representing the possible types a field can have in the schema generation system.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldDefType {
    Boolean,
    /// Boolean literal type — added via `model_schema_prop(literal = true)`.
    /// Maps to `true`/`false` in TS, `z.literal(true)`/`z.literal(false)` in Zod.
    BooleanLiteral(bool),
    /// `char`: serde writes a one-character string and reads only that back, so it is described as
    /// one.
    Char,
    #[cfg(feature = "chrono")]
    /// Chrono `DateTime<Tz>` type - requires "`chrono`" feature.
    DateTime,
    F32,
    F64,
    I16,
    I32,
    I64,
    I8,
    Isize,
    Map(Box<FieldDef>, Box<FieldDef>),
    #[cfg(feature = "chrono")]
    /// Chrono `NaiveDate` type - requires "`chrono`" feature.
    NaiveDate,
    #[cfg(feature = "chrono")]
    /// Chrono `NaiveDateTime` type - requires "`chrono`" feature.
    NaiveDateTime,
    #[cfg(feature = "chrono")]
    /// Chrono `NaiveTime` type - requires "`chrono`" feature.
    NaiveTime,
    /// Numeric literal type — added via `model_schema_prop(literal = 214)`.
    NumberLiteral(f64),
    #[cfg(feature = "mongodb")]
    /// `MongoDB` `ObjectId` type - requires "`mongodb`" feature.
    ObjectId,
    /// A std range, and the type each of its bounds is written as.
    Range(RangeKind, Box<FieldDef>),
    /// `Result<T, E>`, written `{"Ok": T}` or `{"Err": E}`.
    Result(Box<FieldDef>, Box<FieldDef>),
    /// A reference to another struct or enum: the Rust ident written at the reference, and its
    /// generic arguments if any.
    SiblingType(String, Vec<FieldDef>),
    String,
    /// String literal type - for fixed string values Added via `model_schema_prop(literal` =
    /// "value") Maps to "value" in TS, z.literal("value") in Zod.
    StringLiteral(String),
    /// Tuple type - generates anonymous object in TS/Zod.
    Tuple(Vec<FieldDef>),
    /// One of the enclosing item's own type parameters — `IdType` in `struct Wrapper<IdType>`.
    TypeParam(String),
    U16,
    U32,
    U64,
    U8,
    /// A type no surface can name, described as any value: one written as no path (a function
    /// pointer, a trait object, a macro), a path with parenthesized arguments, and in Dart an
    /// opaque type parameter.
    Unknown,
    Usize,
}

impl FieldDefType {
    /// The least and the most value of an integer type serde reads, on each side a JavaScript
    /// `number` does not already bound by holding only whole numbers up to 2^53 exactly, which
    /// `.int()` checks; neither side for any other type.
    pub const fn integer_range(&self) -> (Option<f64>, Option<f64>) {
        match self {
            Self::U8 => (Some(0.0), Some(255.0)),
            Self::U16 => (Some(0.0), Some(65_535.0)),
            Self::U32 => (Some(0.0), Some(4_294_967_295.0)),
            Self::U64 | Self::Usize => (Some(0.0), None),
            Self::I8 => (Some(-128.0), Some(127.0)),
            Self::I16 => (Some(-32_768.0), Some(32_767.0)),
            Self::I32 => (Some(-2_147_483_648.0), Some(2_147_483_647.0)),
            Self::I64
            | Self::Isize
            | Self::F32
            | Self::F64
            | Self::Boolean
            | Self::BooleanLiteral(_)
            | Self::Char
            | Self::Map(_, _)
            | Self::NumberLiteral(_)
            | Self::Range(_, _)
            | Self::Result(_, _)
            | Self::SiblingType(_, _)
            | Self::String
            | Self::StringLiteral(_)
            | Self::Tuple(_)
            | Self::TypeParam(_)
            | Self::Unknown => (None, None),
            #[cfg(feature = "mongodb")]
            Self::ObjectId => (None, None),
            #[cfg(feature = "chrono")]
            Self::DateTime | Self::NaiveDate | Self::NaiveDateTime | Self::NaiveTime => {
                (None, None)
            }
        }
    }

    /// Whether this is one of the numeric primitives — every integer and float kind a
    /// `literal = N` may collapse into a [`Self::NumberLiteral`].
    pub const fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::Usize
                | Self::Isize
                | Self::F32
                | Self::F64
        )
    }
}

impl StdType {
    /// The std type `type_path` writes, where it writes a range or a `Result`.
    fn of(type_path: &syn::TypePath) -> Option<Self> {
        written_std_range(type_path)
            .map(Self::Range)
            .or_else(|| written_std_result(type_path).map(|_| Self::Result))
    }
}

impl RangeKind {
    /// The members serde writes, in the order it writes them.
    pub const fn members(self) -> &'static [&'static str] {
        match self {
            Self::From => &["start"],
            Self::HalfOpen | Self::Inclusive => &["start", "end"],
            Self::To => &["end"],
        }
    }

    /// The name the type is declared under in `core::ops`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::From => "RangeFrom",
            Self::HalfOpen => "Range",
            Self::Inclusive => "RangeInclusive",
            Self::To => "RangeTo",
        }
    }

    /// The kind a std range is named by, and `None` for any other name.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "Range" => Some(Self::HalfOpen),
            "RangeFrom" => Some(Self::From),
            "RangeInclusive" => Some(Self::Inclusive),
            "RangeTo" => Some(Self::To),
            _ => None,
        }
    }
}

/// One member of a recovering decode's `Expected`: its name, the names it carries, and how many
/// members follow under it.
#[cfg(feature = "serde")]
pub type ExpectedMember = (&'static str, Vec<String>, usize);

/// Struct representing a field's definition for schema generation.
#[derive(Clone, Debug)]
pub struct FieldDef {
    pub absent_from_wire: bool,
    pub array_depth: u8,
    pub array_lengths: Vec<(u8, usize)>,
    pub docs: String,
    pub field_type: FieldDefType,
    pub model_schema_prop_meta: Option<ModelSchemaPropMeta>,
    pub name: String,
    pub nullable_levels: Vec<u8>,
    pub omits_value: bool,
    /// Where the default serde reads for an absent key sits among its struct's, for a member
    /// whose default is written as JSON.
    #[cfg(any(feature = "zod", feature = "jsonschema"))]
    pub serde_default: Option<usize>,
    /// The array levels written as a set, numbered as `array_lengths` numbers them.
    pub set_levels: Vec<u8>,
    #[cfg(feature = "jsonschema")]
    pub type_span: Span,
}

/// Two field defs are equal when they describe the same value on every surface: the same type, the
/// same array levels, the same fixed lengths, the same nullable levels and the same set levels.
impl PartialEq for FieldDef {
    fn eq(&self, other: &Self) -> bool {
        self.array_depth == other.array_depth
            && self.array_lengths == other.array_lengths
            && self.nullable_levels == other.nullable_levels
            && self.set_levels == other.set_levels
            && self.field_type == other.field_type
    }
}

impl FieldDef {
    /// The enclosing item's own type parameter this field reaches *below* its own position — the
    /// `T` a `Later<T>` hands to the type it names, and the same `T` inside a tuple or a map.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    pub fn argument_parameter_name(&self) -> Option<&str> {
        match &self.field_type {
            FieldDefType::SiblingType(_, arguments) => arguments
                .iter()
                .find_map(|argument| argument.parameter_or_argument_name()),
            FieldDefType::Map(key, value) => key
                .parameter_or_argument_name()
                .or_else(|| value.parameter_or_argument_name()),
            FieldDefType::Tuple(elements) => {
                elements.iter().find_map(Self::parameter_or_argument_name)
            }
            FieldDefType::Range(_, bound) => bound.parameter_or_argument_name(),
            FieldDefType::Result(ok, err) => ok
                .parameter_or_argument_name()
                .or_else(|| err.parameter_or_argument_name()),
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

    /// The functions the field's `check` names.
    #[cfg(feature = "zod")]
    fn checks(&self) -> &[FunctionName] {
        match &self.model_schema_prop_meta {
            Some(meta) => &meta.check,
            None => &[],
        }
    }

    /// The element of a collection wrapper, as the field the wrapper's own serialization makes it.
    pub fn collection_element_field(&self, element: &Self) -> Self {
        let mut arrayed = element.clone();
        arrayed.name.clone_from(&self.name);
        let own_level = element.array_depth;
        arrayed.array_depth = element.array_depth.saturating_add(1);
        if matches!(&self.field_type, FieldDefType::SiblingType(name, _) if is_set_wrapper(name)) {
            arrayed.mark_set_at(own_level);
        }
        for level in &self.nullable_levels {
            arrayed.mark_nullable_at(level.saturating_add(arrayed.array_depth));
        }
        for &(level, length) in &self.array_lengths {
            arrayed.mark_fixed_length_at(level.saturating_add(arrayed.array_depth), length);
        }
        for level in &self.set_levels {
            arrayed.mark_set_at(level.saturating_add(arrayed.array_depth));
        }
        arrayed.array_depth = arrayed.array_depth.saturating_add(self.array_depth);
        arrayed
            .model_schema_prop_meta
            .clone_from(&self.model_schema_prop_meta);
        arrayed.omits_value = self.omits_value;
        arrayed.absent_from_wire = self.absent_from_wire;
        arrayed
    }

    /// The shape this field renders when the values a bound could be spelled against are its
    /// members rather than the field itself.
    pub const fn composite_shape_name(&self) -> Option<&'static str> {
        match &self.field_type {
            FieldDefType::Map(_, _) => Some("a map"),
            FieldDefType::Range(_, _) => Some("a range"),
            FieldDefType::Result(_, _) => Some("a result"),
            FieldDefType::Tuple(_) => Some("a tuple"),
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => None,
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate
            | FieldDefType::NaiveTime
            | FieldDefType::NaiveDateTime
            | FieldDefType::DateTime => None,
            FieldDefType::SiblingType(_, _)
            | FieldDefType::TypeParam(_)
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
        }
    }

    /// Whether a length, a pattern or a range written on this field reaches no surface at all —
    /// the one question both the refusal and the docs are written from, so neither can come to
    /// answer it differently from the other. A list is measured itself, whatever it holds.
    pub fn constraints_reach_nothing(&self) -> bool {
        !self.is_list()
            && (self.fixed_shape_name().is_some()
                || self.composite_shape_name().is_some()
                || self.parameter_shape_name().is_some())
    }

    #[cfg(any(feature = "zod", all(feature = "serde", feature = "typescript")))]
    /// Checks if this field contains a reference to the given type name.
    pub fn contains_type_reference(&self, type_name: &str) -> bool {
        match &self.field_type {
            FieldDefType::SiblingType(name, generics) => {
                if name == type_name {
                    return true;
                }
                generics
                    .iter()
                    .any(|g| g.contains_type_reference(type_name))
            }
            FieldDefType::Map(k, v) => {
                k.contains_type_reference(type_name) || v.contains_type_reference(type_name)
            }
            FieldDefType::Range(_, bound) => bound.contains_type_reference(type_name),
            FieldDefType::Result(ok, err) => {
                ok.contains_type_reference(type_name) || err.contains_type_reference(type_name)
            }
            FieldDefType::Tuple(elements) => elements
                .iter()
                .any(|e| e.contains_type_reference(type_name)),
            // Primitive and leaf types can't contain recursive references
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
            | FieldDefType::F64 => false,
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => false,
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate
            | FieldDefType::NaiveTime
            | FieldDefType::NaiveDateTime
            | FieldDefType::DateTime => false,
        }
    }

    /// Rewrites every name that is one of the enclosing item's own type parameters into
    /// [`FieldDefType::TypeParam`], so the surfaces stop reading it as a reference to another
    /// generated type.
    pub fn erase_type_parameters(&mut self, parameters: &[String]) {
        if let FieldDefType::SiblingType(name, _) = &self.field_type
            && parameters.iter().any(|parameter| parameter == name)
        {
            self.field_type = FieldDefType::TypeParam(name.clone());
            return;
        }
        for nested in self.nested_type_positions() {
            nested.erase_type_parameters(parameters);
        }
    }

    /// The field's type as a recovering decode's `Expected` describes it, each member written
    /// before the members under it: `Optional` and `Array` per level, outermost first, then the
    /// category.
    #[cfg(feature = "serde")]
    pub fn expected_members(&self) -> Vec<ExpectedMember> {
        let mut members: Vec<ExpectedMember> = Vec::new();
        for level in (0..=self.array_depth).rev() {
            if self.is_nullable_at(level) {
                members.push(("Optional", Vec::new(), 1));
            }
            if level > 0 {
                members.push(("Array", Vec::new(), 1));
            }
        }
        let (member, names, under): (&'static str, Vec<String>, &[Self]) = match &self.field_type {
            FieldDefType::Boolean => ("Boolean", Vec::new(), &[]),
            FieldDefType::BooleanLiteral(value) => ("BooleanLiteral", vec![value.to_string()], &[]),
            FieldDefType::Char => ("Char", Vec::new(), &[]),
            #[cfg(feature = "chrono")]
            FieldDefType::DateTime => ("DateTime", Vec::new(), &[]),
            FieldDefType::F32 => ("F32", Vec::new(), &[]),
            FieldDefType::F64 => ("F64", Vec::new(), &[]),
            FieldDefType::I16 => ("I16", Vec::new(), &[]),
            FieldDefType::I32 => ("I32", Vec::new(), &[]),
            FieldDefType::I64 => ("I64", Vec::new(), &[]),
            FieldDefType::I8 => ("I8", Vec::new(), &[]),
            FieldDefType::Isize => ("Isize", Vec::new(), &[]),
            FieldDefType::Map(_, value) => ("Map", Vec::new(), from_ref(&**value)),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate => ("NaiveDate", Vec::new(), &[]),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDateTime => ("NaiveDateTime", Vec::new(), &[]),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveTime => ("NaiveTime", Vec::new(), &[]),
            FieldDefType::NumberLiteral(value) => {
                ("NumberLiteral", vec![format_number_literal(*value)], &[])
            }
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => ("ObjectId", Vec::new(), &[]),
            FieldDefType::Range(kind, bound) => {
                ("Range", vec![kind.name().to_owned()], from_ref(&**bound))
            }
            // An object naming neither arm is told as an externally tagged enum's tag naming none
            // of its variants.
            FieldDefType::Result(_, _) => {
                ("Variants", RESULT_ARMS.map(str::to_owned).to_vec(), &[])
            }
            // serde writes every std sequence as the array a `Vec` writes.
            FieldDefType::SiblingType(name, items)
                if items.len() == 1
                    && (is_sequence_wrapper(name) || is_refused_sequence_wrapper(name)) =>
            {
                ("Array", Vec::new(), items)
            }
            FieldDefType::SiblingType(name, _) => ("Model", vec![name.clone()], &[]),
            FieldDefType::String => ("String", Vec::new(), &[]),
            FieldDefType::StringLiteral(text) => ("StringLiteral", vec![text.clone()], &[]),
            FieldDefType::Tuple(elements) => ("Tuple", Vec::new(), elements),
            FieldDefType::TypeParam(name) => ("TypeParam", vec![name.clone()], &[]),
            FieldDefType::U16 => ("U16", Vec::new(), &[]),
            FieldDefType::U32 => ("U32", Vec::new(), &[]),
            FieldDefType::U64 => ("U64", Vec::new(), &[]),
            FieldDefType::U8 => ("U8", Vec::new(), &[]),
            FieldDefType::Unknown => ("Unknown", Vec::new(), &[]),
            FieldDefType::Usize => ("Usize", Vec::new(), &[]),
        };
        members.push((member, names, under.len()));
        members.extend(under.iter().flat_map(Self::expected_members));
        members
    }

    /// The element count the array at `level` was written with, for a level written as a `[T; N]`
    /// whose `N` the expansion could read. `None` is every other level: serde writes as many items
    /// as it holds there, so nothing bounds it.
    #[cfg(any(
        feature = "jsonschema",
        feature = "zod",
        feature = "dart",
        feature = "kotlin",
        feature = "swift"
    ))]
    pub fn fixed_length_at(&self, level: u8) -> Option<usize> {
        self.array_lengths
            .iter()
            .find(|&&(at, _)| at == level)
            .map(|&(_, length)| length)
    }

    /// The name of the type this field renders as, when that type's schema is one the crate writes
    /// whole and a `model_schema_prop` bound has no place in.
    pub const fn fixed_shape_name(&self) -> Option<&'static str> {
        match &self.field_type {
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => Some("ObjectId"),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate => Some("chrono::NaiveDate"),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveTime => Some("chrono::NaiveTime"),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDateTime => Some("chrono::NaiveDateTime"),
            #[cfg(feature = "chrono")]
            FieldDefType::DateTime => Some("chrono::DateTime"),
            FieldDefType::SiblingType(_, _)
            | FieldDefType::Map(_, _)
            | FieldDefType::Range(_, _)
            | FieldDefType::Result(_, _)
            | FieldDefType::Tuple(_)
            | FieldDefType::TypeParam(_)
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
        }
    }

    /// What an object flattening this field joins for each variant of the externally tagged enum it
    /// names, as that enum recorded them, and nothing for a field that names no such enum. Answered
    /// under the same bound [`Self::zod_union_members`] is, and for the same reason: what is
    /// spliced in the name's place has to be the whole of what the operand validates.
    #[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
    pub fn flatten_variants(&self) -> Vec<FlattenVariant> {
        if self.is_written_around() {
            return Vec::new();
        }
        self.recorded_flatten_variants()
    }

    /// [`Self::flatten_variants`] of a field that names a function its schema is written around,
    /// which an operand writes around the variants in place of the name: the enum's own binding
    /// takes a unit variant as its bare name, which serde writes flattened as an entry over `null`.
    #[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
    pub fn flatten_variants_written_around(&self) -> Vec<FlattenVariant> {
        if !self.is_written_around() {
            return Vec::new();
        }
        self.recorded_flatten_variants()
    }

    /// Whether `#[model_schema_prop(as_number)]` was written on this field: a `DateTime<Tz>` serde
    /// writes as the integer of its epoch milliseconds.
    #[cfg(feature = "chrono")]
    pub fn has_as_number(&self) -> bool {
        self.model_schema_prop_meta
            .as_ref()
            .is_some_and(|m| m.as_number)
    }

    /// Whether `#[model_schema_prop(nullable)]` was written on this field — an `Option<T>` at
    /// object-key position rendering `T | null` with the key required, instead of the coercing
    /// default. Validated elsewhere to sit only on an `Option<T>` field, so a caller reading it
    /// under [`Self::is_optional`] never needs to ask twice.
    fn has_nullable(&self) -> bool {
        self.model_schema_prop_meta
            .as_ref()
            .is_some_and(|m| m.nullable)
    }

    /// Whether the field describes an array at all — the question every surface asked of the
    /// boolean this depth replaced. Asked only where a schema is generated.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    pub const fn is_array(&self) -> bool {
        self.array_depth > 0
    }

    /// Whether the field's own value is a list, under any `Option` and transparent wrapper: a key
    /// written on it measures the list and never its items.
    pub fn is_list(&self) -> bool {
        self.array_depth > 0
            || matches!(
                &self.field_type,
                FieldDefType::SiblingType(name, arguments)
                    if arguments.len() == 1 && is_sequence_wrapper(name)
            )
    }

    /// Whether the value sitting at array level `level` was written as an `Option`. Levels count
    /// from the innermost value outward: level 0 is what `field_type` names, level `array_depth`
    /// is the field as a whole. Below the outermost, a `None` reaches the wire as a `null` among
    /// the items of the array one level up — the array itself is always written.
    pub fn is_nullable_at(&self, level: u8) -> bool {
        self.nullable_levels.contains(&level)
    }

    /// Whether the field as a whole is an `Option` — the outermost level, and the only one whose
    /// `None` is not written inside an array. What it costs is the position's to say: an absent key
    /// in struct-field position, a `null` in a slot that cannot be dropped.
    pub fn is_optional(&self) -> bool {
        self.is_nullable_at(self.array_depth)
    }

    /// Whether the field's own value, under its own `Option`, is a set.
    #[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
    pub fn is_set(&self) -> bool {
        self.own_level()
            .is_some_and(|leveled| leveled.is_set_at(leveled.array_depth.saturating_sub(1)))
    }

    /// Whether the array at level `level` was written as a set, levels numbered as
    /// `array_lengths` numbers them: level 0 is the array that holds what `field_type` names.
    #[cfg(any(
        feature = "jsonschema",
        feature = "zod",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    pub fn is_set_at(&self, level: u8) -> bool {
        self.set_levels.contains(&level)
    }

    /// Whether the field is `()` itself, which serde writes as `null`, and which every surface
    /// describes as `null`.
    #[cfg(any(
        feature = "serde",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    pub const fn is_unit(&self) -> bool {
        self.array_depth == 0
            && self.nullable_levels.is_empty()
            && matches!(&self.field_type, FieldDefType::Tuple(elements) if elements.is_empty())
    }

    /// Whether the field names a function its schema is written around, a `preprocess` or a
    /// `check`: what the type it names splices in its own place is then no longer the whole of
    /// what the field validates.
    #[cfg(any(feature = "zod", all(feature = "serde", feature = "typescript")))]
    fn is_written_around(&self) -> bool {
        self.model_schema_prop_meta
            .as_ref()
            .is_some_and(|meta| !meta.preprocess.is_empty() || !meta.check.is_empty())
    }

    /// Whether every payload serde writes carries a key for this field — what the JSON surface's
    /// `required` names, and the one question there that a field's `Option`-ness alone cannot
    /// answer. A `nullable` `Option<T>` is the one case where both hold at once: the key is always
    /// written and `null` is its empty value. A key serde reads as a default it can write as JSON
    /// is not required either.
    #[cfg(feature = "jsonschema")]
    pub fn key_is_required(&self) -> bool {
        if self.is_optional() {
            self.has_nullable()
        } else {
            !self.omits_value && self.serde_default.is_none()
        }
    }

    /// Whether the object key this field writes may be absent — `field?: T` admits the payload with
    /// no such key, `field: T | undefined` demands it. The key-dropping serde attribute is
    /// deliberately not read for an `Option`, since the `Option`-null guard already requires one on
    /// every named `Option` field.
    fn key_may_be_absent(&self) -> bool {
        if self.is_optional() {
            self.model_schema_prop_meta
                .as_ref()
                .is_some_and(|m| m.ts_optional)
        } else {
            self.omits_value
        }
    }

    /// The form serde writes this key in, wherever that form is not what the key's own type spells
    /// on the two nominal surfaces. Read through the registry for a name, so a brand or alias
    /// chain forwards its target's answer; a key under an array or an `Option` writes no key at all
    /// and keeps its name, its own guard having refused it already.
    pub fn map_key_wire(&self) -> MapKeyWire {
        if self.array_depth > 0 || self.is_optional() {
            return MapKeyWire::Named;
        }
        if matches!(self.field_type, FieldDefType::Boolean) {
            return MapKeyWire::Boolean;
        }
        #[cfg(feature = "chrono")]
        if matches!(self.field_type, FieldDefType::DateTime) {
            return MapKeyWire::Timestamp;
        }
        let FieldDefType::SiblingType(name, arguments) = &self.field_type else {
            return MapKeyWire::Named;
        };
        if arguments.is_empty() {
            // A name the registry has no entry for is one declared below this map, which
            // `map_key_path` already reads as an enumeration.
            lookup_alias_info(name).map_or(MapKeyWire::Enumerated, |info| info.key_wire)
        } else {
            MapKeyWire::Named
        }
    }

    fn mark_fixed_length_at(&mut self, level: u8, length: usize) {
        if !self.array_lengths.iter().any(|&(at, _)| at == level) {
            self.array_lengths.push((level, length));
        }
    }

    fn mark_nullable_at(&mut self, level: u8) {
        if !self.nullable_levels.contains(&level) {
            self.nullable_levels.push(level);
        }
    }

    fn mark_set_at(&mut self, level: u8) {
        if !self.set_levels.contains(&level) {
            self.set_levels.push(level);
        }
    }

    /// The type declared elsewhere a bound written on this field lands on, and whether it is
    /// written with arguments. A list is not asked: a bound written on one measures the list.
    pub fn named_shape(&self) -> Option<(&str, bool)> {
        let FieldDefType::SiblingType(name, arguments) = &self.field_type else {
            return None;
        };
        Some((name, !arguments.is_empty()))
    }

    /// Whether this field's rendering reads any other item's own module-scope Zod binding — a
    /// factory call or a bare `$Schema` const — at any depth, wrapped or named directly. Deferral
    /// asks a wider question than the direct-sibling fold gate does:
    /// `z.array(Tagged$SchemaFactory(z.string()))` reads a sibling `const` exactly as much as a
    /// bare `Tagged$SchemaFactory(z.string())` does.
    #[cfg(feature = "zod")]
    pub fn names_a_sibling_binding(&self) -> bool {
        match &self.field_type {
            FieldDefType::SiblingType(name, generics) => {
                !is_sequence_wrapper(name) || generics.iter().any(Self::names_a_sibling_binding)
            }
            FieldDefType::Map(_, value) => value.names_a_sibling_binding(),
            FieldDefType::Range(_, bound) => bound.names_a_sibling_binding(),
            FieldDefType::Result(ok, err) => {
                ok.names_a_sibling_binding() || err.names_a_sibling_binding()
            }
            FieldDefType::Tuple(elements) => elements.iter().any(Self::names_a_sibling_binding),
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
            | FieldDefType::F64 => false,
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => false,
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate
            | FieldDefType::NaiveTime
            | FieldDefType::NaiveDateTime
            | FieldDefType::DateTime => false,
        }
    }

    /// The defs written inside this one, which is every position a type parameter can be reached at
    /// below the top: a `SiblingType`'s generic arguments, a `Map`'s key and value, a `Tuple`'s
    /// elements — every other variant names a type outright and holds no def. Listed exhaustively
    /// so a variant that grows a nested position cannot silently escape a walk that reads a
    /// parameter.
    fn nested_type_positions(&mut self) -> Vec<&mut Self> {
        match &mut self.field_type {
            FieldDefType::SiblingType(_, generics) => generics.iter_mut().collect(),
            FieldDefType::Map(key, value) => vec![key, value],
            FieldDefType::Range(_, bound) => vec![bound],
            FieldDefType::Result(ok, err) => vec![ok, err],
            FieldDefType::Tuple(elements) => elements.iter_mut().collect(),
            // Leaf types hold no def.
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
            | FieldDefType::F64 => Vec::new(),
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => Vec::new(),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate
            | FieldDefType::NaiveTime
            | FieldDefType::NaiveDateTime
            | FieldDefType::DateTime => Vec::new(),
        }
    }

    /// The std range or the std `Result` the key of the first map this field reaches is written
    /// as, at any depth, by the name a refusal writes it under.
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    pub fn object_map_key(&self) -> Option<String> {
        if let FieldDefType::Map(key, _) = &self.field_type {
            if let FieldDefType::Range(kind, _) = key.field_type {
                return Some(format!("{}<_>", kind.name()));
            }
            if matches!(key.field_type, FieldDefType::Result(_, _)) {
                return Some("Result<_, _>".to_owned());
            }
        }
        self.clone()
            .nested_type_positions()
            .into_iter()
            .find_map(|nested| nested.object_map_key())
    }

    /// Reads each of the enclosing item's own type parameters this field reaches, at any depth, as
    /// a value no surface describes: what such a value writes is then its own, read with no
    /// converter of the codec's.
    #[cfg(feature = "dart")]
    pub fn opaque_parameters(&mut self) {
        if matches!(self.field_type, FieldDefType::TypeParam(_)) {
            self.field_type = FieldDefType::Unknown;
            return;
        }
        for nested in self.nested_type_positions() {
            nested.opaque_parameters();
        }
    }

    /// The `?` a member written with an absent-able key carries, and nothing for one whose key is
    /// always written.
    pub fn optional_key_marker(&self) -> &'static str {
        if self.key_may_be_absent() { "?" } else { "" }
    }

    /// The name of the first `OsString`/`OsStr` this field reaches, at any depth.
    pub fn os_string_name(&self) -> Option<&str> {
        self.reached_name(|name| matches!(name, "OsString" | "OsStr"))
    }

    /// The field written with its own value as an array level of its own, a sequence wrapper's
    /// level among its element's, or `None` where its own value is no array.
    #[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
    fn own_level(&self) -> Option<Self> {
        if self.array_depth > 0 {
            return Some(self.clone());
        }
        if let FieldDefType::SiblingType(name, arguments) = &self.field_type
            && let [element] = arguments.as_slice()
            && is_sequence_wrapper(name)
        {
            return Some(self.collection_element_field(element));
        }
        None
    }

    /// The parameter this field is or reaches, which is what a nested position is asked for: below
    /// the top level the two questions have one answer.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    fn parameter_or_argument_name(&self) -> Option<&str> {
        self.parameter_shape_name()
            .or_else(|| self.argument_parameter_name())
    }

    /// The enclosing item's own type parameter this field renders as, when a bound spelled against
    /// it names a value whose type the expansion never sees.
    pub fn parameter_shape_name(&self) -> Option<&str> {
        match &self.field_type {
            FieldDefType::TypeParam(name) => Some(name),
            FieldDefType::SiblingType(_, _)
            | FieldDefType::Map(_, _)
            | FieldDefType::Range(_, _)
            | FieldDefType::Result(_, _)
            | FieldDefType::Tuple(_)
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

    /// Rewrites each of the enclosing item's own type parameters, `parameters`, into the
    /// [`parameter_placeholder`] of its position, which a holder fills with its own argument.
    #[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
    pub fn place_type_parameters(&mut self, parameters: &[String]) {
        if let FieldDefType::TypeParam(name) = &self.field_type
            && let Some(at) = parameters.iter().position(|parameter| parameter == name)
        {
            self.field_type = FieldDefType::TypeParam(parameter_placeholder(at));
            return;
        }
        for nested in self.nested_type_positions() {
            nested.place_type_parameters(parameters);
        }
    }

    /// The first name this field reaches, at any depth, that `refused` answers for. A name is only
    /// ever written where a type is, so the walk descends the positions holding written types and
    /// stops at every value the parser resolved to something of its own.
    fn reached_name<F>(&self, refused: F) -> Option<&str>
    where
        F: Fn(&str) -> bool + Copy,
    {
        match &self.field_type {
            FieldDefType::SiblingType(name, generics) => {
                if refused(name.as_str()) {
                    return Some(name);
                }
                generics
                    .iter()
                    .find_map(|argument| argument.reached_name(refused))
            }
            FieldDefType::Map(key, value) => key
                .reached_name(refused)
                .or_else(|| value.reached_name(refused)),
            FieldDefType::Range(_, bound) => bound.reached_name(refused),
            FieldDefType::Result(ok, err) => ok
                .reached_name(refused)
                .or_else(|| err.reached_name(refused)),
            FieldDefType::Tuple(elements) => elements
                .iter()
                .find_map(|element| element.reached_name(refused)),
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

    /// Whether this field reaches a type the registry does not hold yet — a `#[model_schema]` item
    /// declared below the one being expanded, whether or not that item declares a parameter of its
    /// own.
    #[cfg(feature = "zod")]
    pub fn reaches_a_type_declared_later(&self) -> bool {
        match &self.field_type {
            FieldDefType::SiblingType(name, generics) => {
                (!is_sequence_wrapper(name) && lookup_alias_info(name).is_none())
                    || generics.iter().any(Self::reaches_a_type_declared_later)
            }
            FieldDefType::Map(key, value) => {
                key.reaches_a_type_declared_later() || value.reaches_a_type_declared_later()
            }
            FieldDefType::Range(_, bound) => bound.reaches_a_type_declared_later(),
            FieldDefType::Result(ok, err) => {
                ok.reaches_a_type_declared_later() || err.reaches_a_type_declared_later()
            }
            FieldDefType::Tuple(elements) => {
                elements.iter().any(Self::reaches_a_type_declared_later)
            }
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
            | FieldDefType::F64 => false,
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => false,
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate
            | FieldDefType::NaiveTime
            | FieldDefType::NaiveDateTime
            | FieldDefType::DateTime => false,
        }
    }

    /// What the externally tagged enum this field names recorded for each of its variants, each of
    /// the enum's own type parameters filled with the argument written at its position as a
    /// reference to the enum writes it. Nothing where a position the variants name has no argument.
    #[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
    fn recorded_flatten_variants(&self) -> Vec<FlattenVariant> {
        if self.array_depth > 0 {
            return Vec::new();
        }
        let FieldDefType::SiblingType(name, arguments) = &self.field_type else {
            return Vec::new();
        };
        let recorded = lookup_alias_info(name).map_or_else(Vec::new, |info| info.flatten_variants);
        recorded
            .into_iter()
            .map(|variant| {
                Some(FlattenVariant {
                    #[cfg(feature = "typescript")]
                    typescript: placeholders_filled(&variant.typescript, |at| {
                        arguments.get(at).map(Self::typescript_typename)
                    })?,
                    #[cfg(feature = "zod")]
                    unit: variant.unit,
                    #[cfg(feature = "zod")]
                    zod: placeholders_filled(&variant.zod, |at| {
                        arguments.get(at).map(Self::zod_type)
                    })?,
                })
            })
            .collect::<Option<Vec<FlattenVariant>>>()
            .unwrap_or_default()
    }

    /// The name of the first `is_refused_sequence_wrapper` this field reaches, at any depth.
    pub fn refused_sequence_wrapper_name(&self) -> Option<&str> {
        self.reached_name(is_refused_sequence_wrapper)
    }

    /// Rewrites any `Self` type reference to the concrete enclosing type name, at the parameters
    /// the enclosing item declares.
    pub fn resolve_self_references(&mut self, type_name: &str, parameters: &[String]) {
        match &mut self.field_type {
            FieldDefType::SiblingType(name, generics) => {
                if name == "Self" {
                    type_name.clone_into(name);
                    generics.extend(parameters.iter().map(|parameter| Self {
                        name: parameter.clone(),
                        field_type: FieldDefType::TypeParam(parameter.clone()),
                        array_depth: 0,
                        array_lengths: Vec::new(),
                        docs: String::new(),
                        model_schema_prop_meta: None,
                        nullable_levels: Vec::new(),
                        absent_from_wire: false,
                        omits_value: false,
                        #[cfg(any(feature = "zod", feature = "jsonschema"))]
                        serde_default: None,
                        set_levels: Vec::new(),
                        #[cfg(feature = "jsonschema")]
                        type_span: Span::call_site(),
                    }));
                }
                for generic in generics.iter_mut() {
                    generic.resolve_self_references(type_name, parameters);
                }
            }
            FieldDefType::Map(key, value) => {
                key.resolve_self_references(type_name, parameters);
                value.resolve_self_references(type_name, parameters);
            }
            FieldDefType::Range(_, bound) => bound.resolve_self_references(type_name, parameters),
            FieldDefType::Result(ok, err) => {
                ok.resolve_self_references(type_name, parameters);
                err.resolve_self_references(type_name, parameters);
            }
            FieldDefType::Tuple(elements) => {
                for element in elements.iter_mut() {
                    element.resolve_self_references(type_name, parameters);
                }
            }
            // Leaf types cannot contain nested references.
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
            | FieldDefType::F64 => {}
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => {}
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate
            | FieldDefType::NaiveTime
            | FieldDefType::NaiveDateTime
            | FieldDefType::DateTime => {}
        }
    }

    /// The members of the untagged union this field names as an object flattening it spells them,
    /// as the registry recorded them, each of the union's own type parameters filled with the
    /// argument written at its position — and nothing for a field that names no such union, for
    /// one whose members carry no exclusion the union's own name does not already describe, and for
    /// one that writes no argument at a position a member names.
    #[cfg(all(feature = "serde", feature = "typescript"))]
    pub fn ts_union_members(&self) -> Vec<String> {
        if self.array_depth > 0 || self.is_written_around() {
            return Vec::new();
        }
        let FieldDefType::SiblingType(name, arguments) = &self.field_type else {
            return Vec::new();
        };
        lookup_alias_info(name)
            .map_or_else(Vec::new, |info| info.ts_union_members)
            .iter()
            .map(|member| {
                placeholders_filled(member, |at| {
                    arguments.get(at).map(Self::typescript_typename)
                })
            })
            .collect::<Option<Vec<String>>>()
            .unwrap_or_default()
    }

    /// Builds the TypeScript type before the outermost optional wrap: the type match plus one
    /// `Array<…>` per array level, each carrying the `| null` of the level it wraps. The outermost
    /// level's wrap lives in `typescript_typename` and `typescript_slot_typename`, which is where
    /// the position it sits in decides between `| undefined` and `| null`.
    fn typescript_base(&self) -> String {
        let result = match &self.field_type {
            // A type written as no path, a function pointer or a macro among them.
            FieldDefType::Unknown => "unknown".to_owned(),
            // The declaration binds this name, so the field is written under it.
            FieldDefType::TypeParam(name) => name.clone(),
            FieldDefType::Range(kind, bound) => {
                let held = bound.typescript_slot_typename();
                range_object(*kind, |member| format!("{member}: {held}"), "; ")
            }
            FieldDefType::Result(ok, err) => typescript_result(ok, err),
            FieldDefType::Tuple(lst) if lst.is_empty() => "null".to_owned(),
            FieldDefType::Tuple(lst) => {
                let elements = lst
                    .iter()
                    .map(Self::typescript_slot_typename)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{elements}]")
            }
            FieldDefType::SiblingType(name, lst) => {
                if let [element] = lst.as_slice()
                    && is_sequence_wrapper(name)
                {
                    // The element re-enters the whole per-type rendering as the arrayed field it
                    // stands for, so a set renders exactly as the `Vec` of that element does.
                    return self.collection_element_field(element).typescript_base();
                } else if let Some(info) = lookup_alias_info(name) {
                    if lst.is_empty() {
                        info.export_name
                    } else {
                        format!(
                            "{}<{}>",
                            info.export_name,
                            lst.iter()
                                .map(Self::typescript_typename)
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                } else if lst.is_empty() {
                    name.clone()
                } else {
                    format!(
                        "{name}<{}>",
                        lst.iter()
                            .map(Self::typescript_typename)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            FieldDefType::Map(k, v) => {
                format!(
                    "Partial<Record<{}, {}>>",
                    k.typescript_map_key_typename(),
                    v.typescript_slot_typename()
                )
            }
            FieldDefType::Boolean => "boolean".to_owned(),
            FieldDefType::Char | FieldDefType::String => "string".to_owned(),
            FieldDefType::StringLiteral(literal) => ts_quoted(literal),
            FieldDefType::BooleanLiteral(value) => value.to_string(),
            FieldDefType::NumberLiteral(value) => format_number_literal(*value),
            FieldDefType::U8
            | FieldDefType::U16
            | FieldDefType::U32
            | FieldDefType::U64
            | FieldDefType::I8
            | FieldDefType::I16
            | FieldDefType::I32
            | FieldDefType::I64
            | FieldDefType::Usize
            | FieldDefType::Isize => "number".to_owned(),
            FieldDefType::F32 | FieldDefType::F64 => "number".to_owned(),
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => object_id::get_object_id_typescript_type(),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate => chrono::get_naive_date_typescript_type(),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveTime => chrono::get_naive_time_typescript_type(),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDateTime => chrono::get_naive_datetime_typescript_type(),
            #[cfg(feature = "chrono")]
            FieldDefType::DateTime => {
                if self.has_as_number() {
                    chrono::get_datetime_number_typescript_type()
                } else {
                    chrono::get_datetime_typescript_type()
                }
            }
        };
        (0..self.array_depth).fold(result, |wrapped, level| {
            let item = if self.is_nullable_at(level) {
                format!("{wrapped} | null")
            } else {
                wrapped
            };
            format!("Array<{item}>")
        })
    }

    /// The key a `Partial<Record<…>>` is written with: the key's own type, except where the key is
    /// one of the enclosing item's type parameters, which states `string`, and where serde writes
    /// the key in a form its own type does not spell — the same answer
    /// [`Self::zod_map_record_call`] gives, for the same reason.
    fn typescript_map_key_typename(&self) -> String {
        if self.parameter_shape_name().is_some() {
            return "string".to_owned();
        }
        match self.map_key_wire() {
            MapKeyWire::Boolean => BOOLEAN_KEY_TYPESCRIPT.to_owned(),
            #[cfg(feature = "chrono")]
            MapKeyWire::Timestamp => "string".to_owned(),
            MapKeyWire::Enumerated | MapKeyWire::Named => self.typescript_typename(),
        }
    }

    /// What this field contributes to an object that writes its members beside its own, on the
    /// TypeScript surface: the value itself, with no answer for the outermost `Option`.
    #[cfg(feature = "typescript")]
    pub fn typescript_merged_typename(&self) -> String {
        self.typescript_base()
    }

    /// The TypeScript type for a value in a slot that cannot be dropped — a tuple element, a map
    /// entry, or the content key of a single-element tuple variant, which serde always writes. An
    /// `Option` there is null-flavored (`{base} | null`) rather than undefined-flavored: none of
    /// those positions can be omitted the way an object key can, so serde emits `null` for a
    /// `None` in each of them.
    pub fn typescript_slot_typename(&self) -> String {
        let base = self.typescript_base();
        if self.is_optional() {
            format!("{base} | null")
        } else {
            base
        }
    }

    /// The slot spelling of a member of the type named by `self_type_name`, with a map whose values
    /// name that type written so the alias declaring the union stays resolvable: `Partial<Record<K,
    /// V>>` would make the alias circular (TS2456). A `string` or `number` key is written as the
    /// index-signature object it is equal to, and an enumerated key as the mapped type.
    #[cfg(all(feature = "serde", feature = "typescript"))]
    pub fn typescript_slot_typename_deferring_self(&self, self_type_name: &str) -> String {
        let FieldDefType::Map(key, value) = &self.field_type else {
            return self.typescript_slot_typename();
        };
        if self.array_depth > 0 || !value.contains_type_reference(self_type_name) {
            return self.typescript_slot_typename();
        }
        let key_type = key.typescript_map_key_typename();
        let value_type = value.typescript_slot_typename();
        let base = if matches!(key_type.as_str(), "number" | "string") {
            format!("{{ [key: {key_type}]: {value_type} | undefined }}")
        } else {
            format!("{{ [key in {key_type}]?: {value_type} }}")
        };
        if self.is_optional() {
            format!("{base} | null")
        } else {
            base
        }
    }

    /// Generates the TypeScript type name for this field.
    pub fn typescript_typename(&self) -> String {
        let pre_result = self.typescript_base();
        if self.is_optional() {
            if self.has_nullable() {
                format!("{pre_result} | null")
            } else if self.key_may_be_absent() {
                // An absent-able key renders as `field?: T`, so the `| undefined` is redundant, and
                // under `exactOptionalPropertyTypes` it would claim an explicit `undefined`.
                pre_result
            } else {
                format!("{pre_result} | undefined")
            }
        } else {
            pre_result
        }
    }

    /// The kind of bound no surface reads beside this field, its own value being what the other
    /// kind is measured on: a number has no length or pattern; a string, a boolean, a character
    /// and a value this crate has no description for have no range; and a list has neither a
    /// pattern nor a range. The refusal and the docs are both written from it, as from
    /// [`Self::constraints_reach_nothing`].
    pub fn unmeasured_bound(&self) -> Option<BoundKind> {
        if self.is_list() {
            Some(BoundKind::Scalar)
        } else if self.field_type.is_numeric() {
            Some(BoundKind::Text)
        } else if matches!(
            self.field_type,
            FieldDefType::String
                | FieldDefType::Boolean
                | FieldDefType::Char
                | FieldDefType::Unknown
        ) {
            Some(BoundKind::Range)
        } else {
            None
        }
    }

    /// The name of the first `is_unsupported_std_wrapper` this field reaches, at any depth.
    pub fn unsupported_std_wrapper_name(&self) -> Option<&str> {
        self.reached_name(is_unsupported_std_wrapper)
    }

    /// The value this field's wrappers hold, as a field of its own: the same type with the
    /// `Option`s and the array levels dropped. The wrappers are the field's to declare and the
    /// value under them is what a type name stands for, so this is what an `as = Type` is compared
    /// against when the target names a bare type — the spelling `as = String` on a `Vec<String>`
    /// uses.
    pub fn value_under_wrappers(&self) -> Self {
        let mut value = self.clone();
        value.array_depth = 0;
        value.array_lengths.clear();
        value.nullable_levels.clear();
        value.set_levels.clear();
        value
    }

    /// The field with its own set read as a plain list: what a walk below the set reads, the set's
    /// own repeat being asked ahead of its count. The field as it is where its own value is no
    /// set.
    #[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
    pub fn without_own_set(&self) -> Self {
        match self.own_level() {
            Some(mut leveled) if leveled.is_set_at(leveled.array_depth.saturating_sub(1)) => {
                let own = leveled.array_depth.saturating_sub(1);
                leveled.set_levels.retain(|&level| level != own);
                leveled
            }
            Some(_) | None => self.clone(),
        }
    }

    /// The type match plus one `z.array(…)` per array level, each carrying the `z.nullable(…)` of
    /// the level it wraps and the `.length(N)` of a level written as a fixed-size `[T; N]`, before
    /// the preprocess wrap. The outermost level carries the count the field's own keys hold it to,
    /// and a level written as a set the check that refuses a repeat, after the count.
    #[cfg(feature = "zod")]
    fn zod_array_base(&self) -> String {
        let result = match &self.field_type {
            // A type written as no path, a function pointer or a macro among them.
            FieldDefType::Unknown => "z.unknown()".to_owned(),
            // A `const` cannot be parameterised, so a parameter composes the argument the factory
            // binds for it.
            FieldDefType::TypeParam(name) => zod_factory_argument(name),
            FieldDefType::Range(kind, bound) => {
                let held = bound.zod_slot_type();
                let members = range_object(*kind, |member| format!("{member}: {held}"), ", ");
                format!("z.strictObject({members})")
            }
            FieldDefType::Result(ok, err) => {
                let arms = result_union(ok, err, |arm, held| {
                    format!("z.strictObject({{ {arm}: {} }})", held.zod_slot_type())
                });
                format!("z.union([{}])", arms.join(", "))
            }
            FieldDefType::Tuple(lst) if lst.is_empty() => "z.null()".to_owned(),
            FieldDefType::Tuple(lst) => {
                let elements = lst
                    .iter()
                    .map(Self::zod_slot_type)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("z.tuple([{elements}])")
            }
            FieldDefType::SiblingType(name, lst) => {
                if let [element] = lst.as_slice()
                    && is_sequence_wrapper(name)
                {
                    // The element carries this field's own array levels, so it applies the wrap.
                    return self.collection_element_field(element).zod_array_base();
                } else if let Some(info) = lookup_alias_info(name) {
                    // What the named type published is what this can name: a factory where the type
                    // declares parameters, and the one schema it has where it declares none.
                    if publishes_zod_factory(name) {
                        zod_factory_call(&info.export_name, lst)
                    } else {
                        format!("{}$Schema{}", info.export_name, self.zod_named_checks())
                    }
                } else if lst.is_empty() {
                    format!("{name}$Schema{}", self.zod_named_checks())
                } else {
                    // A name the registry does not hold yet, written with arguments, is a generic
                    // type expanded after this one: only a factory can take them.
                    zod_factory_call(name, lst)
                }
            }
            FieldDefType::Map(k, v) => k.zod_map_record_call(&v.zod_slot_type()),
            FieldDefType::Boolean => "z.boolean()".to_owned(),
            // serde writes a `char` as a one-character string and reads only that back, so the
            // length is fixed, not read from `model_schema_prop`.
            FieldDefType::Char => "z.string().length(1)".to_owned(),
            FieldDefType::String => self.zod_string_type(),
            FieldDefType::StringLiteral(literal) => format!("z.literal({})", ts_quoted(literal)),
            FieldDefType::BooleanLiteral(value) => format!("z.literal({value})"),
            FieldDefType::NumberLiteral(value) => {
                format!("z.literal({})", format_number_literal(*value))
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
            | FieldDefType::Isize => self.zod_number_type("z.number().int()"),
            FieldDefType::F32 => self.zod_number_type(&format!(
                "z.number().gt(-{F32_READ_BOUND}).lt({F32_READ_BOUND})"
            )),
            FieldDefType::F64 => self.zod_number_type("z.number()"),
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => object_id::get_object_id_zod_schema(),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate => chrono::get_naive_date_zod_schema(),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveTime => chrono::get_naive_time_zod_schema(),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDateTime => chrono::get_naive_datetime_zod_schema(),
            #[cfg(feature = "chrono")]
            FieldDefType::DateTime => {
                if self.has_as_number() {
                    chrono::get_datetime_number_zod_schema()
                } else {
                    chrono::get_datetime_native_zod_schema()
                }
            }
        };
        (0..self.array_depth).fold(result, |wrapped, level| {
            self.zod_array_level(wrapped, level)
        })
    }

    /// The `z.array(…)` of array level `level` around `wrapped`, the schema of what it holds.
    #[cfg(feature = "zod")]
    fn zod_array_level(&self, wrapped: String, level: u8) -> String {
        let item = if self.is_nullable_at(level) {
            format!("z.nullable({wrapped})")
        } else {
            wrapped
        };
        let bound = self
            .fixed_length_at(level)
            .map_or_else(String::new, |length| format!(".length({length})"));
        let counted = if level.saturating_add(1) == self.array_depth {
            self.zod_item_count_checks()
        } else {
            String::new()
        };
        let distinct = if self.is_set_at(level) {
            zod_repeat_check()
        } else {
            String::new()
        };
        format!("z.array({item}){bound}{counted}{distinct}")
    }

    /// Builds the Zod schema before the struct-field optional wrap: the type match, the
    /// array wraps, the checks, and the preprocess wrap. The
    /// `z.union([…, z.undefined()]).prefault(undefined)` wrap lives in `zod_type`.
    #[cfg(feature = "zod")]
    fn zod_base(&self) -> String {
        self.zod_preprocess_wrap(self.zod_checked_base())
    }

    /// [`Self::zod_array_base`] with the checks the field names written on it: the schema of the
    /// value a check takes, under any `Option` and inside any `preprocess`.
    #[cfg(feature = "zod")]
    fn zod_checked_base(&self) -> String {
        zod_check_wrap(self.zod_array_base(), self.checks())
    }

    /// Every factory call this field's schema is written with, outermost only and once each, in
    /// the text [`Self::zod_array_base`] writes it as.
    #[cfg(feature = "zod")]
    fn zod_factory_calls(&self, calls: &mut Vec<String>) {
        if let FieldDefType::Map(key, value) = &self.field_type {
            key.zod_factory_calls(calls);
            value.zod_factory_calls(calls);
        } else if let FieldDefType::Tuple(elements) = &self.field_type {
            for element in elements {
                element.zod_factory_calls(calls);
            }
        } else if let FieldDefType::Range(_, bound) = &self.field_type {
            bound.zod_factory_calls(calls);
        } else if let FieldDefType::Result(ok, err) = &self.field_type {
            ok.zod_factory_calls(calls);
            err.zod_factory_calls(calls);
        } else if let FieldDefType::SiblingType(name, arguments) = &self.field_type {
            let call = if let [element] = arguments.as_slice()
                && is_sequence_wrapper(name)
            {
                return element.zod_factory_calls(calls);
            } else if let Some(info) = lookup_alias_info(name) {
                publishes_zod_factory(name).then(|| zod_factory_call(&info.export_name, arguments))
            } else {
                (!arguments.is_empty()).then(|| zod_factory_call(name, arguments))
            };
            if let Some(written) = call
                && !calls.contains(&written)
            {
                calls.push(written);
            }
        } else {
            // Every other type is a leaf, which names no item.
        }
    }

    /// Every factory call a union is handed in place of, in the text [`Self::zod_checked_base`]
    /// writes it as. A field that is one call and names a check hands the call in with the checks
    /// on it: the parameter that stands for the call is no schema a check can be written on.
    #[cfg(feature = "zod")]
    fn zod_handed_calls(&self) -> Vec<String> {
        let mut calls = Vec::new();
        self.zod_factory_calls(&mut calls);
        let base = self.zod_array_base();
        if !self.checks().is_empty() && calls.as_slice() == [base] {
            return vec![self.zod_checked_base()];
        }
        calls
    }

    /// The `.min(…)` and `.max(…)` a list's own `minLength` and `maxLength` write on its array.
    #[cfg(feature = "zod")]
    fn zod_item_count_checks(&self) -> String {
        let Some(meta) = &self.model_schema_prop_meta else {
            return String::new();
        };
        let least = meta.min_items.map(|count| {
            let reported = zod_error_arg(Bound::MinItems(count));
            format!(".min({count}, {reported})")
        });
        let most = meta.max_items.map(|count| {
            let reported = zod_error_arg(Bound::MaxItems(count));
            format!(".max({count}, {reported})")
        });
        least.into_iter().chain(most).collect()
    }

    /// The whole record call a map is written as, read off its key: the constructor moves with the
    /// key schema, because `z.record` over an enumerated key demands every member — the two strings
    /// a `bool` writes and the members of a plain enum are both such an enumeration — and a map
    /// holding one of them has to parse.
    #[cfg(feature = "zod")]
    fn zod_map_record_call(&self, value_schema: &str) -> String {
        if self.parameter_shape_name().is_some() {
            return format!("z.record(z.string(), {value_schema})");
        }
        match self.map_key_wire() {
            MapKeyWire::Boolean => {
                format!("z.partialRecord({BOOLEAN_KEY_ZOD}, {value_schema})")
            }
            MapKeyWire::Enumerated => {
                format!("z.partialRecord({}, {value_schema})", self.zod_type())
            }
            #[cfg(feature = "chrono")]
            MapKeyWire::Timestamp => format!("z.record({TIMESTAMP_KEY_ZOD}, {value_schema})"),
            MapKeyWire::Named => format!("z.record({}, {value_schema})", self.zod_type()),
        }
    }

    /// The same value on the Zod surface, for the same reason: what a merged source validates, with
    /// the outermost `Option` left to whatever assembles the merge. See
    /// [`Self::typescript_merged_typename`].
    /// A field written around a function whose type is an externally tagged enum joins the
    /// function around its variants, each as a flattened enum is joined, where the enum's own
    /// binding is not the one schema that calls a function of its own or replaces a default.
    #[cfg(feature = "zod")]
    pub fn zod_merged_schema(&self) -> String {
        #[cfg(feature = "serde")]
        {
            let variants = self.flatten_variants_written_around();
            if !variants.is_empty()
                && !self.zod_names_a_preprocess()
                && !variants
                    .iter()
                    .any(|variant| holds_a_placeholder(&variant.zod))
            {
                let choice = format!(
                    "z.union([{}])",
                    variants
                        .iter()
                        .map(|variant| variant.zod.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                // A check takes the enum's value, where a unit variant is its bare name.
                let value = variants
                    .iter()
                    .rev()
                    .filter_map(|variant| variant.unit.as_deref())
                    .fold("ctx.value".to_owned(), |rest, key| {
                        let quoted = ts_quoted(key);
                        format!("{quoted} in ctx.value ? {quoted} : {rest}")
                    });
                return self.zod_preprocess_wrap(zod_check_wrap_handing(
                    choice,
                    self.checks(),
                    &value,
                ));
            }
        }
        self.zod_base()
    }

    /// The field's length and pattern bounds as one `.check(…)` written onto a named type's
    /// schema, or nothing where none was written. `.check` is declared on every schema and returns
    /// the schema's own type, so a brand keeps its brand and a binding annotated `ZodType` takes it.
    #[cfg(feature = "zod")]
    fn zod_named_checks(&self) -> String {
        let Some(meta) = &self.model_schema_prop_meta else {
            return String::new();
        };
        let mut checks: Vec<String> = Vec::new();
        if let Some(min_len) = meta.min_length {
            let reported = zod_error_arg(Bound::MinLength(min_len));
            checks.push(format!("z.minLength({min_len}, {reported})"));
        }
        if let Some(max_len) = meta.max_length {
            let reported = zod_error_arg(Bound::MaxLength(max_len));
            checks.push(format!("z.maxLength({max_len}, {reported})"));
        }
        if let Some(pattern) = &meta.pattern {
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

    /// Whether the type this field names calls a `preprocess` of its own in its Zod binding, which
    /// is then the one schema that calls it: no member of it is written in its place.
    #[cfg(feature = "zod")]
    pub fn zod_names_a_preprocess(&self) -> bool {
        matches!(&self.field_type, FieldDefType::SiblingType(name, _) if names_a_zod_preprocess(name))
    }

    /// Builds the Zod schema string for a numeric field: the range of its integer type, then any
    /// min/max constraints. A side of the range a declared bound narrows is left to the bound.
    #[cfg(feature = "zod")]
    fn zod_number_type(&self, base: &str) -> String {
        let mut result = base.to_owned();
        let declared = self.model_schema_prop_meta.as_ref();
        let minimum = declared.and_then(|meta| meta.minimum);
        let maximum = declared.and_then(|meta| meta.maximum);
        let (least, most) = self.field_type.integer_range();
        if let Some(width) = least
            && minimum.is_none_or(|bound| bound < width)
        {
            result = format!("{result}.min({width})");
        }
        if let Some(width) = most
            && maximum.is_none_or(|bound| bound > width)
        {
            result = format!("{result}.max({width})");
        }
        if let Some(min) = minimum {
            let reported = zod_error_arg(Bound::Minimum(min));
            result = format!("{result}.min({min}, {reported})");
        }
        if let Some(max) = maximum {
            let reported = zod_error_arg(Bound::Maximum(max));
            result = format!("{result}.max({max}, {reported})");
        }
        result
    }

    /// Wraps `schema` in one `z.preprocess(fn, …)` per function named by `preprocess`, innermost
    /// function first, or returns it unwrapped where none was written. Held apart from
    /// [`Self::zod_base`] so [`Self::zod_type`] can also wrap the whole `nullable` union rather
    /// than only the type match beneath it — coerce, then validate.
    #[cfg(feature = "zod")]
    fn zod_preprocess_wrap(&self, schema: String) -> String {
        let Some(meta) = &self.model_schema_prop_meta else {
            return schema;
        };
        zod_preprocess_wrap(schema, &meta.preprocess)
    }

    /// Whether the type this field names is written by its own Zod binding alone, with no member
    /// of it written in its place: the binding calls a `preprocess` of the type's own, or a
    /// member's text holds the default of a member, which only the binding's own text replaces.
    #[cfg(feature = "zod")]
    pub fn zod_read_whole(&self) -> bool {
        #[cfg(feature = "serde")]
        let flattened = self
            .flatten_variants()
            .iter()
            .any(|variant| holds_a_placeholder(&variant.zod));
        #[cfg(not(feature = "serde"))]
        let flattened = false;
        flattened
            || self.zod_names_a_preprocess()
            || self
                .zod_union_members()
                .iter()
                .any(|member| holds_a_placeholder(&member.spelling))
    }

    /// The Zod schema for a value in a slot that cannot be dropped — a tuple element, a map entry,
    /// or the content key of a single-element tuple variant, which serde always writes. An
    /// `Option` there is null-flavored (`z.nullable({base})`) rather than undefined-flavored: none
    /// of those positions can be omitted the way an object key can, so serde emits `null` for a
    /// `None` in each of them.
    #[cfg(feature = "zod")]
    pub fn zod_slot_type(&self) -> String {
        let base = self.zod_base();
        if self.is_optional() {
            format!("z.nullable({base})")
        } else {
            base
        }
    }

    /// Builds the Zod schema string for a string field, applying any length/pattern constraints.
    #[cfg(feature = "zod")]
    fn zod_string_type(&self) -> String {
        let mut result = "z.string()".to_owned();
        if let Some(meta) = &self.model_schema_prop_meta
            && let Some(min_len) = meta.min_length
        {
            let reported = zod_error_arg(Bound::MinLength(min_len));
            result = format!("{result}.min({min_len}, {reported})");
        }
        if let Some(meta) = &self.model_schema_prop_meta
            && let Some(max_len) = meta.max_length
        {
            let reported = zod_error_arg(Bound::MaxLength(max_len));
            result = format!("{result}.max({max_len}, {reported})");
        }
        if let Some(meta) = &self.model_schema_prop_meta {
            for pattern in meta.patterns() {
                let literal_body = escape_js_regex_literal(pattern);
                let reported = zod_error_arg(Bound::Pattern(pattern));
                result = format!("{result}.check(z.regex(/{literal_body}/, {reported}))");
            }
        }
        result
    }

    #[cfg(feature = "zod")]
    /// Generates the Zod schema string for this field (requires "zod" feature).
    pub fn zod_type(&self) -> String {
        if self.is_optional() {
            let calls = self.zod_handed_calls();
            if self.has_nullable() {
                let union = zod_union_around(&self.zod_checked_base(), &calls, |held| {
                    format!("z.union([{held}, z.null()])")
                });
                self.zod_preprocess_wrap(union)
            } else {
                zod_union_around(&self.zod_base(), &calls, |held| {
                    format!(
                        "z.union([z.null().transform(() => undefined), {held}, z.undefined()]).prefault(undefined)"
                    )
                })
            }
        } else {
            let pre_result = self.zod_base();
            if self.key_may_be_absent() {
                format!("{pre_result}.optional()")
            } else if let Some(index) = self.serde_default {
                // Parsed as a payload is, so the default is held to the member's checks.
                format!("{pre_result}.prefault({})", placeholder(index))
            } else {
                pre_result
            }
        }
    }

    /// The members of the untagged union this field names, as the registry recorded them, each of
    /// the union's own type parameters filled with the argument written at its position, and
    /// nothing for a field that names no such union or writes no argument at a position a member
    /// names.
    #[cfg(feature = "zod")]
    pub fn zod_union_members(&self) -> Vec<ZodUnionMember> {
        if self.array_depth > 0 || self.is_written_around() {
            return Vec::new();
        }
        let FieldDefType::SiblingType(name, arguments) = &self.field_type else {
            return Vec::new();
        };
        lookup_alias_info(name)
            .map_or_else(Vec::new, |info| info.zod_union_members)
            .into_iter()
            .map(|mut member| {
                member.spelling = placeholders_filled(&member.spelling, |at| {
                    arguments.get(at).map(Self::zod_type)
                })?;
                Some(member)
            })
            .collect::<Option<Vec<ZodUnionMember>>>()
            .unwrap_or_default()
    }
}

/// The TypeScript union of the two objects a `Result` of the arms `ok` and `err` is written as.
fn typescript_result(ok: &FieldDef, err: &FieldDef) -> String {
    result_union(ok, err, |arm, held| {
        format!("{{ {arm}: {} }}", held.typescript_slot_typename())
    })
    .join(" | ")
}

/// The two objects a `Result` is written as, each as `arm` spells it from the key serde writes it
/// under and the value it holds.
fn result_union(
    ok: &FieldDef,
    err: &FieldDef,
    arm: impl Fn(&str, &FieldDef) -> String,
) -> [String; 2] {
    let [ok_key, err_key] = RESULT_ARMS;
    [arm(ok_key, ok), arm(err_key, err)]
}

/// The object literal `{ … }` of the members a range of `kind` writes, each as `member` spells it,
/// joined by `separator`.
fn range_object(kind: RangeKind, member: impl Fn(&str) -> String, separator: &str) -> String {
    let members: Vec<String> = kind.members().iter().map(|name| member(name)).collect();
    format!("{{ {} }}", members.join(separator))
}

/// `schema` inside one `z.preprocess(fn, …)` per function named, the first one written outermost,
/// so each is handed the answer of the one before it. A field's schema and a type's own
/// expression are both wrapped here.
#[cfg(feature = "zod")]
pub fn zod_preprocess_wrap(schema: String, functions: &[FunctionName]) -> String {
    functions.iter().rev().fold(schema, |wrapped, function| {
        format!("z.preprocess({}, {wrapped})", function.name)
    })
}

/// `schema` with one `.check(…)` per function named, in the order written. Each hands its
/// function the value and a payload of its own, and copies what the function pushed with
/// `continue: true`: Zod skips a later check where an earlier issue carries no such flag, and an
/// issue's own flag cannot be set where it was pushed. No type is named, so a build without
/// `typescript` writes the same text.
#[cfg(feature = "zod")]
pub fn zod_check_wrap(schema: String, checks: &[FunctionName]) -> String {
    zod_check_wrap_handing(schema, checks, "ctx.value")
}

/// [`zod_check_wrap`], handing each function `value`, an expression over `ctx.value`, as the
/// value and as the payload's. It is written in both places so the function's own parameter
/// types it, and a variant's bare name stays the literal the type names.
#[cfg(feature = "zod")]
pub fn zod_check_wrap_handing(schema: String, checks: &[FunctionName], value: &str) -> String {
    checks.iter().fold(schema, |wrapped, check| {
        if value == "ctx.value" {
            format!(
                "{wrapped}.check((ctx) => {{\n  const own = {{ value: ctx.value, issues: \
                 ctx.issues.slice(0, 0) }};\n  {}(own.value, own);\n  for (const issue of \
                 own.issues) ctx.issues.push({{ ...issue, continue: true }});\n}})",
                check.name
            )
        } else {
            format!(
                "{wrapped}.check((ctx) => {{\n  const own = {{ issues: ctx.issues.slice(0, 0) \
                 }};\n  {}({value}, {{ ...own, value: {value} }});\n  for (const issue of \
                 own.issues) ctx.issues.push({{ ...issue, continue: true }});\n}})",
                check.name
            )
        }
    })
}

/// The identifier a generic type parameter's `fromJson` converter argument binds to — `itemTypeFromJson`
/// for `ItemType`, mirroring [`zod_factory_argument`]'s lower-camel naming for the same reason: a
/// Dart `const` field position cannot itself be parameterised, so a generic type's codec takes one
/// converter function per parameter instead.
#[cfg(feature = "dart")]
pub fn dart_from_json_argument(parameter: &str) -> String {
    format!("{}FromJson", dart_lower_camel(parameter))
}

/// The identifier a generic type parameter's `toJson` converter argument binds to — the encode
/// counterpart of [`dart_from_json_argument`].
#[cfg(feature = "dart")]
pub fn dart_to_json_argument(parameter: &str) -> String {
    format!("{}ToJson", dart_lower_camel(parameter))
}

/// `parameter`, lower-camel-cased: `IdType` -> `idType`. `pub` (rather than private, like every
/// other helper on this page) because the plain-enum builder in `features::dart` also needs it, to
/// turn a Rust variant ident into a valid Dart enhanced-enum member name.
#[cfg(feature = "dart")]
pub fn dart_lower_camel(parameter: &str) -> String {
    let mut characters = parameter.chars();
    characters.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_lowercase(), characters.as_str())
    })
}

/// The `f64` a `literal = N` was written with, formatted the way TypeScript and Zod read a numeric
/// literal type: a whole value renders without the trailing `.0` `f64`'s own `Display` carries.
pub fn format_number_literal(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

/// The union `union` writes around `held`, with every factory call in `held` handed in through a
/// generic arrow. Checking a union reads what each member takes and answers, and a factory's own
/// schema is the type still being inferred there, so a type parameter stands in for it. The names
/// end in `$`, which no argument a factory binds can.
#[cfg(feature = "zod")]
fn zod_union_around(held: &str, calls: &[String], union: impl Fn(&str) -> String) -> String {
    if calls.is_empty() {
        return union(held);
    }
    let mut body = held.to_owned();
    let mut types = Vec::new();
    let mut values = Vec::new();
    for (call, position) in calls.iter().zip(1_usize..) {
        let numbered = if position == 1 {
            String::new()
        } else {
            position.to_string()
        };
        body = replace_call(&body, call, &format!("reached{numbered}$"));
        types.push(format!("Reached{numbered}$ extends {ZOD_SCHEMA_BOUND}"));
        values.push(format!("reached{numbered}$: Reached{numbered}$"));
    }
    format!(
        "(<{}>({}) => {})({})",
        types.join(", "),
        values.join(", "),
        union(&body),
        calls.join(", ")
    )
}

/// `schema` with `call` replaced by `name` wherever it starts a name of its own, so that a call to
/// `Node$SchemaFactory` is not found inside one to `TreeNode$SchemaFactory`.
#[cfg(feature = "zod")]
fn replace_call(schema: &str, call: &str, name: &str) -> String {
    let mut pieces = schema.split(call);
    let mut replaced = pieces.next().unwrap_or_default().to_owned();
    for piece in pieces {
        let inside_a_name = replaced
            .chars()
            .next_back()
            .is_some_and(|last| last.is_alphanumeric() || last == '_' || last == '$');
        replaced.push_str(if inside_a_name { call } else { name });
        replaced.push_str(piece);
    }
    replaced
}

/// A reference to a type that publishes a factory, as the call it has to be. Each argument is
/// rendered by the renderer that renders the reference itself, so an argument that is a forwarded
/// parameter, a primitive, a date, or another generic reference all reach the call the same way —
/// and one that is itself generic composes at whatever depth it was written at.
#[cfg(feature = "zod")]
fn zod_factory_call(name: &str, arguments: &[FieldDef]) -> String {
    format!(
        "{name}$SchemaFactory({})",
        arguments
            .iter()
            .map(FieldDef::zod_type)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The one list of wrappers the crate renders as arrays, shared by every surface.
pub fn is_sequence_wrapper(name: &str) -> bool {
    match name {
        "BTreeSet" | "BinaryHeap" | "HashSet" | "Vec" | "VecDeque" => true,
        #[cfg(feature = "indexmap")]
        "IndexSet" => !is_declared_ident(name),
        _ => false,
    }
}

/// The sequences whose items serde reads as distinct: a repeat in the payload is kept once.
pub fn is_set_wrapper(name: &str) -> bool {
    match name {
        "BTreeSet" | "HashSet" => true,
        #[cfg(feature = "indexmap")]
        "IndexSet" => !is_declared_ident(name),
        _ => false,
    }
}

/// The one list of maps the crate reads as a map: each is an object on the wire, written from
/// its two leading type arguments.
pub fn is_map_wrapper(name: &str) -> bool {
    match name {
        "BTreeMap" | "HashMap" => true,
        // A name of another crate's, unless this crate declared a type of its own under it.
        #[cfg(feature = "indexmap")]
        "IndexMap" => !is_declared_ident(name),
        _ => false,
    }
}

/// The std sequence the crate refuses rather than renders. serde writes it as the array a `Vec`
/// writes, so the covered spelling already describes every wire form it has.
pub fn is_refused_sequence_wrapper(name: &str) -> bool {
    name == "LinkedList"
}

/// The number of leading type arguments a container's wire form is written from, or `None` for a
/// name that is not one of them.
fn container_wire_arity(name: &str) -> Option<usize> {
    if is_map_wrapper(name) {
        Some(2)
    } else if is_sequence_wrapper(name) {
        Some(1)
    } else {
        None
    }
}

/// The one list of std wrappers the crate reads straight through to what they hold: everything
/// serde writes as the value alone, with no wrapper of its own on the wire.
pub fn is_transparent_wrapper(name: &str) -> bool {
    is_ownership_wrapper(name) || is_interior_mutability_wrapper(name)
}

/// The ownership and borrow wrappers: each implements `Deref`, so a constraint's generated
/// validator reaches the inner value with a plain `&**value`.
pub fn is_ownership_wrapper(name: &str) -> bool {
    matches!(name, "Arc" | "Box" | "Cow" | "Rc")
}

/// The interior-mutability wrappers: serde writes each as the value it guards, with `RefCell` and
/// `Mutex`/`RwLock` returning a serialization error — not a panic — when the guard cannot be taken
/// (an already-mutably-borrowed `RefCell`, a poisoned `Mutex`/`RwLock`), the same fallible path a
/// schema does not describe for any other type. None of the four implements `Deref`, which is why a
/// constraint's generated validator cannot reach through one the way it reaches through an
/// ownership wrapper — see `generic_wrap` in `model_schema.rs`.
pub fn is_interior_mutability_wrapper(name: &str) -> bool {
    matches!(name, "Cell" | "Mutex" | "RefCell" | "RwLock")
}

/// The std cell/lock/lazy-init types and borrow guards serde implements neither `Serialize` nor
/// `Deserialize` for: unlike `is_transparent_wrapper`'s members there is no wire form to describe.
/// Matched on the bare name, as `os_string_name` matches its two — a user item named `Ref` or
/// `RefMut` is refused with them.
pub fn is_unsupported_std_wrapper(name: &str) -> bool {
    matches!(
        name,
        "OnceLock"
            | "OnceCell"
            | "LazyLock"
            | "LazyCell"
            | "Ref"
            | "RefMut"
            | "MutexGuard"
            | "RwLockReadGuard"
            | "RwLockWriteGuard"
    )
}

/// Classifies a `syn::Variant` into its `VariantKind`.
pub fn classify_variant(variant: &Variant) -> VariantKind {
    match &variant.fields {
        Fields::Unit => VariantKind::Unit,
        Fields::Named(_) => VariantKind::Named,
        Fields::Unnamed(fields) => {
            if fields.unnamed.is_empty() {
                // `Foo()` is refused at expansion before any shape reads it.
                VariantKind::Unit
            } else if fields.unnamed.len() == 1 {
                VariantKind::TupleSingle
            } else {
                VariantKind::TupleMultiple
            }
        }
    }
}

/// Main function to create `FieldDef` from `syn::Type`.
fn get_field_def_from_type_path(
    type_path: &syn::TypePath,
    field_name: String,
    field_docs: &str,
) -> FieldDef {
    let Some(segment) = type_path.path.segments.last() else {
        return FieldDef {
            name: field_name,
            field_type: FieldDefType::Unknown,
            array_depth: 0,
            array_lengths: Vec::new(),
            docs: field_docs.to_owned(),
            model_schema_prop_meta: None,
            nullable_levels: Vec::new(),
            absent_from_wire: false,
            omits_value: false,
            #[cfg(any(feature = "zod", feature = "jsonschema"))]
            serde_default: None,
            set_levels: Vec::new(),
            #[cfg(feature = "jsonschema")]
            type_span: type_path.span(),
        };
    };
    let ident = segment.ident.to_string();
    match &segment.arguments {
        PathArguments::None => FieldDef {
            name: field_name,
            field_type: get_field_def_type_or_sibling(&ident),
            array_depth: 0,
            array_lengths: Vec::new(),
            docs: field_docs.to_owned(),
            model_schema_prop_meta: None,
            nullable_levels: Vec::new(),
            absent_from_wire: false,
            omits_value: false,
            #[cfg(any(feature = "zod", feature = "jsonschema"))]
            serde_default: None,
            set_levels: Vec::new(),
            // The name segment, not the whole path: it is what a generated module is named after,
            // so a reference the module cannot resolve is blamed on the name it was built from.
            #[cfg(feature = "jsonschema")]
            type_span: segment.ident.span(),
        },
        PathArguments::AngleBracketed(args) => get_field_def_from_generic_type(
            &segment.ident,
            args,
            StdType::of(type_path),
            field_name,
            field_docs,
        ),
        // Function pointer types are unsupported; fall back to `unknown`.
        PathArguments::Parenthesized(_) => FieldDef {
            name: field_name,
            field_type: FieldDefType::Unknown,
            array_depth: 0,
            array_lengths: Vec::new(),
            docs: field_docs.to_owned(),
            model_schema_prop_meta: None,
            nullable_levels: Vec::new(),
            absent_from_wire: false,
            omits_value: false,
            #[cfg(any(feature = "zod", feature = "jsonschema"))]
            serde_default: None,
            set_levels: Vec::new(),
            #[cfg(feature = "jsonschema")]
            type_span: segment.ident.span(),
        },
    }
}

/// Builds a `FieldDef` for a named type written with generic arguments. `std` is the std range or
/// the std `Result` the path writes, where it writes one.
fn get_field_def_from_generic_type(
    type_ident: &Ident,
    args: &syn::AngleBracketedGenericArguments,
    std: Option<StdType>,
    field_name: String,
    field_docs: &str,
) -> FieldDef {
    let ident_name = type_ident.to_string();
    let ident = ident_name.as_str();
    let mut arg_types: Vec<FieldDef> = args
        .args
        .iter()
        .filter_map(|arg| {
            if let GenericArgument::Type(inner_ty) = arg {
                Some(get_field_def("", inner_ty, ""))
            } else {
                None
            }
        })
        .collect();
    // A container is claimed by its own name, so what it is written with past its wire form is
    // dropped before the arms below count arguments.
    if let Some(arity) = container_wire_arity(ident)
        && arg_types.len() > arity
    {
        arg_types.truncate(arity);
    }
    if arg_types.is_empty() {
        written_def(
            FieldDefType::SiblingType(ident.to_owned(), vec![]),
            field_name,
            field_docs,
            type_ident,
        )
    } else if let [element] = arg_types.as_slice()
        && let Some(collapsed) = collapsed_wrapper_def(ident, element, &field_name, field_docs)
    {
        collapsed
    } else if arg_types.len() == 2 && is_map_wrapper(ident) {
        log::trace!(
            "Creating HashMap Map type - key: {:?}, value: {:?}",
            arg_types[0],
            arg_types[1]
        );
        FieldDef {
            array_depth: 0,
            array_lengths: Vec::new(),
            name: field_name,
            field_type: FieldDefType::Map(
                Box::new(arg_types[0].clone()),
                Box::new(arg_types[1].clone()),
            ),
            docs: field_docs.to_owned(),
            model_schema_prop_meta: None,
            nullable_levels: Vec::new(),
            absent_from_wire: false,
            omits_value: false,
            #[cfg(any(feature = "zod", feature = "jsonschema"))]
            serde_default: None,
            set_levels: Vec::new(),
            #[cfg(feature = "jsonschema")]
            type_span: type_ident.span(),
        }
    } else if arg_types.len() == 1 && is_datetime_generic_type(ident) {
        // The timezone type parameter says nothing about what is written.
        written_def(datetime_field_type(), field_name, field_docs, type_ident)
    } else if let Some(StdType::Range(kind)) = std
        && let [bound] = arg_types.as_slice()
    {
        let field_type = FieldDefType::Range(kind, Box::new(bound.clone()));
        written_def(field_type, field_name, field_docs, type_ident)
    } else if matches!(std, Some(StdType::Result))
        && let [ok, err] = arg_types.as_slice()
    {
        let field_type = FieldDefType::Result(Box::new(ok.clone()), Box::new(err.clone()));
        written_def(field_type, field_name, field_docs, type_ident)
    } else {
        log::trace!("Creating SiblingType - name: {ident}, arg_types: {arg_types:?}");
        let field_type = FieldDefType::SiblingType(ident.to_owned(), arg_types);
        written_def(field_type, field_name, field_docs, type_ident)
    }
}

/// A def of `field_type` under no wrapper, its type written at `type_ident`.
fn written_def(
    field_type: FieldDefType,
    field_name: String,
    field_docs: &str,
    type_ident: &Ident,
) -> FieldDef {
    #[cfg(not(feature = "jsonschema"))]
    let _: &Ident = type_ident;
    FieldDef {
        name: field_name,
        field_type,
        array_depth: 0,
        array_lengths: Vec::new(),
        docs: field_docs.to_owned(),
        model_schema_prop_meta: None,
        nullable_levels: Vec::new(),
        absent_from_wire: false,
        omits_value: false,
        #[cfg(any(feature = "zod", feature = "jsonschema"))]
        serde_default: None,
        set_levels: Vec::new(),
        #[cfg(feature = "jsonschema")]
        type_span: type_ident.span(),
    }
}

/// The std range `type_path` writes: one of the four names with one type argument, written in full
/// under `core::ops` or `std::ops`, or under any other path but a name a `#[model_schema]` type of
/// the crate's own was declared under above.
pub fn written_std_range(type_path: &syn::TypePath) -> Option<RangeKind> {
    if type_path.qself.is_some() {
        return None;
    }
    let segment = type_path.path.segments.last()?;
    let kind = RangeKind::named(&segment.ident.to_string())?;
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let written = arguments
        .args
        .iter()
        .filter(|argument| matches!(argument, GenericArgument::Type(_)))
        .count();
    (written == 1 && (names_std_ops(&type_path.path) || !is_declared_ident(kind.name())))
        .then_some(kind)
}

/// The two arms of the std `Result` `type_path` writes: `Result` with two type arguments, written
/// in full under `core::result` or `std::result`, or under any other path but a name a
/// `#[model_schema]` type of the crate's own was declared under above.
pub fn written_std_result(type_path: &syn::TypePath) -> Option<(&Type, &Type)> {
    if type_path.qself.is_some() {
        return None;
    }
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let mut written = arguments.args.iter().filter_map(|argument| {
        if let GenericArgument::Type(ty) = argument {
            Some(ty)
        } else {
            None
        }
    });
    let (Some(ok), Some(err), None) = (written.next(), written.next(), written.next()) else {
        return None;
    };
    (names_std_result(&type_path.path) || !is_declared_ident("Result")).then_some((ok, err))
}

/// Whether `path` is written in full under `core::ops` or `std::ops`, where it names the std type
/// whatever the crate declares.
pub fn names_std_ops(path: &syn::Path) -> bool {
    written_in_std(path, "ops")
}

/// Whether `path` is written in full under `core::result` or `std::result`, where it names the
/// std type whatever the crate declares.
pub fn names_std_result(path: &syn::Path) -> bool {
    written_in_std(path, "result")
}

/// Whether `path` is one name under `module` of `core` or `std`.
fn written_in_std(path: &syn::Path, module: &str) -> bool {
    let mut segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string());
    let (Some(root), Some(written), Some(_), None) = (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) else {
        return false;
    };
    matches!(root.as_str(), "core" | "std") && written == module
}

/// The def a single-argument wrapper collapses onto, for the wrappers that collapse.
fn collapsed_wrapper_def(
    ident: &str,
    element: &FieldDef,
    field_name: &str,
    field_docs: &str,
) -> Option<FieldDef> {
    let mut result = element.clone();
    match ident {
        "Option" => result.mark_nullable_at(result.array_depth),
        "Vec" => result.array_depth = result.array_depth.saturating_add(1),
        _ if is_transparent_wrapper(ident) => {}
        _ => return None,
    }
    field_name.clone_into(&mut result.name);
    field_docs.clone_into(&mut result.docs);
    Some(result)
}

/// The element count a fixed-size array was written with, when the expansion can read it — a
/// literal is the whole of that. A const generic parameter, a `const` item and any computed length
/// each name a value only the compiler has, and the macro runs before there is one to ask for, so
/// each describes as the unbounded array every other sequence spelling describes as.
fn literal_array_length(len: &syn::Expr) -> Option<usize> {
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Int(literal),
        attrs: _attrs,
    }) = len
    else {
        return None;
    };
    literal.base10_parse::<usize>().ok()
}

/// Debug logging: Set `RUST_LOG=trace` to see HashMap/SiblingType creation.
pub fn get_field_def(name: &str, ty: &Type, field_docs: &str) -> FieldDef {
    let field_name = name.to_owned();
    let written = written_type(ty);
    if let Type::Path(type_path) = written {
        get_field_def_from_type_path(type_path, field_name, field_docs)
    } else if let Type::Reference(type_ref) = written {
        // let lifetime = type_ref .lifetime .as_ref() .map_or("".to_string(), |l| format!("'{}",
        // l.ident));
        get_field_def(name, type_ref.elem.as_ref(), field_docs)
    } else if let Type::Array(type_array) = written {
        let mut def = get_field_def(name, &type_array.elem, field_docs);
        // The array this spelling adds is the level the element's own depth counts up to, and the
        // length is that level's — not the field's, which may sit under further wrappers.
        let level = def.array_depth;
        def.array_depth = def.array_depth.saturating_add(1);
        if let Some(length) = literal_array_length(&type_array.len) {
            def.mark_fixed_length_at(level, length);
        }
        def
    } else if let Type::Slice(type_slice) = written {
        let mut def = get_field_def(name, &type_slice.elem, field_docs);
        def.array_depth = def.array_depth.saturating_add(1);
        def
    } else if let Type::Tuple(type_tuple) = written {
        let elements: Vec<FieldDef> = type_tuple
            .elems
            .iter()
            .enumerate()
            .map(|(idx, v)| get_field_def(&format!("element_{idx}"), v, field_docs))
            .collect();
        FieldDef {
            name: field_name,
            field_type: FieldDefType::Tuple(elements),
            array_depth: 0,
            array_lengths: Vec::new(),
            docs: field_docs.to_owned(),
            model_schema_prop_meta: None,
            nullable_levels: Vec::new(),
            absent_from_wire: false,
            omits_value: false,
            #[cfg(any(feature = "zod", feature = "jsonschema"))]
            serde_default: None,
            set_levels: Vec::new(),
            #[cfg(feature = "jsonschema")]
            type_span: written.span(),
        }
    } else {
        FieldDef {
            name: field_name,
            field_type: FieldDefType::Unknown,
            array_depth: 0,
            array_lengths: Vec::new(),
            docs: field_docs.to_owned(),
            model_schema_prop_meta: None,
            nullable_levels: Vec::new(),
            absent_from_wire: false,
            omits_value: false,
            #[cfg(any(feature = "zod", feature = "jsonschema"))]
            serde_default: None,
            set_levels: Vec::new(),
            #[cfg(feature = "jsonschema")]
            type_span: written.span(),
        }
    }
}

/// Helper to map Rust type name strings to `FieldDefType`.
fn get_field_def_type_or_sibling(t_name: &str) -> FieldDefType {
    if lookup_alias_info(t_name).is_some() {
        return FieldDefType::SiblingType(t_name.to_owned(), vec![]);
    }
    match t_name {
        "bool" => FieldDefType::Boolean,
        "char" => FieldDefType::Char,
        // `str` and `Path` are the borrowed forms of `String` and `PathBuf`, and each writes the
        // same JSON string its owned form does.
        "String" | "PathBuf" | "str" | "Path" => FieldDefType::String,
        "u8" => FieldDefType::U8,
        "u16" => FieldDefType::U16,
        "u32" => FieldDefType::U32,
        "u64" => FieldDefType::U64,
        "i8" => FieldDefType::I8,
        "i16" => FieldDefType::I16,
        "i32" => FieldDefType::I32,
        "i64" => FieldDefType::I64,
        "usize" => FieldDefType::Usize,
        "isize" => FieldDefType::Isize,
        "f32" => FieldDefType::F32,
        "f64" => FieldDefType::F64,
        #[cfg(feature = "mongodb")]
        "ObjectId" => {
            if object_id::should_handle_as_object_id(t_name) {
                FieldDefType::ObjectId
            } else {
                FieldDefType::SiblingType(t_name.to_owned(), vec![])
            }
        }
        #[cfg(not(feature = "mongodb"))]
        "ObjectId" => {
            eprintln!("warning: ObjectId type detected but the 'mongodb' feature is not enabled");
            eprintln!(
                "         ObjectId will be treated as a custom type (may cause compilation errors)"
            );
            eprintln!("         Enable the mongodb feature: features = [\"mongodb\"]");
            eprintln!("         Or add the required ObjectId type definition to your code");
            FieldDefType::SiblingType(t_name.to_owned(), vec![])
        }
        #[cfg(feature = "chrono")]
        "NaiveDate" => FieldDefType::NaiveDate,
        #[cfg(feature = "chrono")]
        "NaiveTime" => FieldDefType::NaiveTime,
        #[cfg(feature = "chrono")]
        "NaiveDateTime" => FieldDefType::NaiveDateTime,
        type_name => FieldDefType::SiblingType(type_name.to_owned(), vec![]),
    }
}

/// Whether a name is one the language reserves for a primitive type that
/// [`get_field_def_type_or_sibling`] has no arm for.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
pub fn is_undescribable_primitive(name: &str) -> bool {
    matches!(name, "i128" | "u128" | "f16" | "f128")
}

/// Delegates to [`crate::features::serde::parse_serde_type_attributes`] for the type-level serde
/// metadata (`rename_all`, `tag`, …) `model_schema.rs` reads.
#[cfg(feature = "serde")]
pub fn parse_serde_type_attributes(attrs: &[Attribute]) -> SerdeTypeMeta {
    parse_serde_type_attributes_impl(attrs)
}

/// Delegates to [`crate::features::serde::parse_serde_field_attributes`] for a field's rename and
/// `flatten`.
#[cfg(feature = "serde")]
pub fn parse_serde_field_attributes(attrs: &[Attribute]) -> SerdeFieldMeta {
    parse_serde_field_attributes_impl(attrs)
}

/// Utility to check if an enum is a plain unit enum (no fields in variants).
pub fn is_plain_enum(item_enum: &ItemEnum) -> bool {
    item_enum
        .variants
        .iter()
        .all(|variant| matches!(variant.fields, Fields::Unit))
}

/// Check if a type name is a `DateTime` generic type (chrono feature).
/// Returns true only when chrono feature is enabled and type is `DateTime`.
#[cfg(feature = "chrono")]
fn is_datetime_generic_type(type_name: &str) -> bool {
    type_name == "DateTime"
}

#[cfg(not(feature = "chrono"))]
const fn is_datetime_generic_type(_type_name: &str) -> bool {
    false
}

/// What a `DateTime<Tz>` field carries: the chrono type, the timezone parameter saying nothing
/// about what is written.
#[cfg(feature = "chrono")]
const fn datetime_field_type() -> FieldDefType {
    FieldDefType::DateTime
}

/// Unreachable without chrono — `is_datetime_generic_type` answers `false` there — and named as
/// any other unknown type would be.
#[cfg(not(feature = "chrono"))]
fn datetime_field_type() -> FieldDefType {
    FieldDefType::SiblingType("DateTime".to_owned(), vec![])
}

#[cfg(test)]
mod tests;
