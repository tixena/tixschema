//! What the Dart backend writes for a brand's own string check: in `validate()`, and alone in
//! `validateBound()`, which chooses a variant of an untagged enum that holds the brand.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A brand over a string, held to a length and a pattern.
#[model_schema(minLength = 2, pattern = "^[A-Z]+$")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Slug(String);

/// A brand with a parameter, whose string check is its declared default's.
#[model_schema(minLength = 2, default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Tag<IdType>(IdType);

/// A brand over a list, whose lengths count its items.
#[model_schema(no_display, minLength = 1, maxLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Batch(Vec<String>);

#[test]
fn every_declared_brand_is_constructible() {
    assert_eq!(Slug::new("AB".to_owned()).unwrap().as_inner(), "AB");
    assert_eq!(*Tag::new(7_u32).unwrap().as_inner(), 7);
    assert_eq!(
        Batch::new(vec!["a".to_owned()]).unwrap().as_inner().len(),
        1
    );
}

/// The bounds are measured on the value the brand wraps, and no message names a place: whatever
/// holds the brand writes its own key in front.
#[test]
fn a_brand_over_a_string_measures_its_value_under_no_place() {
    assert_eq!(
        slug_dart::dart_definition(),
        "class Slug { const Slug._(this.value); final String value; factory Slug.of(String value) \
         { final read = Slug._(value); final violations = read.validate(); if \
         (violations.isNotEmpty) throw FormatException(violations.join('; ')); return read; } \
         factory Slug.fromJson(dynamic \
         json) { final read = Slug.fromJsonUnjudged(json); final violations = read.validate(); \
         if (violations.isNotEmpty) throw FormatException(violations.join('; ')); return read; } \
         factory Slug.fromJsonUnjudged(dynamic json) => Slug._(json as String); dynamic toJson() \
         => value; List<String> validateBound() => [ if (value.runes.length < 2) \"too short: \
         minimum length is 2, got ${value.runes.length}\", if \
         (!RegExp(\"^[A-Z]+\\$\").hasMatch(value)) \"does not match pattern '^[A-Z]+\\$'\", ]; \
         List<String> validate() => [ \
         if (value.runes.length < 2) \"too short: minimum length is 2, got \
         ${value.runes.length}\", if (!RegExp(\"^[A-Z]+\\$\").hasMatch(value)) \"does not match \
         pattern '^[A-Z]+\\$'\", ]; }"
    );
}

/// The check holds the declared default of the parameter, tested with a pattern on the value,
/// and no other filling.
#[test]
fn a_brand_with_a_parameter_measures_its_declared_default_alone() {
    assert_eq!(
        tag_dart::dart_definition(),
        "class Tag<IdType> { const Tag._(this.value); final IdType value; factory \
         Tag.of(IdType value) { final read = Tag<IdType>._(value); final violations = \
         read.validate(); if (violations.isNotEmpty) throw FormatException(violations.join('; \
         ')); return read; } factory \
         Tag.fromJson(dynamic json, IdType Function(dynamic) idTypeFromJson) { final read = \
         Tag.fromJsonUnjudged(json, idTypeFromJson); final violations = read.validate(); if \
         (violations.isNotEmpty) throw FormatException(violations.join('; ')); return read; } \
         factory Tag.fromJsonUnjudged(dynamic json, IdType Function(dynamic) idTypeFromJson) => \
         Tag._(idTypeFromJson(json)); dynamic toJson(dynamic Function(IdType) idTypeToJson) => \
         idTypeToJson(value); List<String> validateBound() => [ if (value case final String \
         held) if (held.runes.length < 2) \"too short: minimum length is 2, got \
         ${held.runes.length}\", ]; List<String> validate() => [ if (value case final String \
         held) if (held.runes.length < 2) \"too short: minimum length is 2, got \
         ${held.runes.length}\", ]; }"
    );
}

#[test]
fn a_brand_over_a_list_counts_its_items() {
    assert_eq!(
        batch_dart::dart_definition(),
        "class Batch { const Batch._(this.value); final List<String> value; factory \
         Batch.of(List<String> value) { final read = Batch._(value); final violations = \
         read.validate(); if (violations.isNotEmpty) throw FormatException(violations.join('; \
         ')); return read; } factory \
         Batch.fromJson(dynamic json) { final read = Batch.fromJsonUnjudged(json); final \
         violations = read.validate(); if (violations.isNotEmpty) throw \
         FormatException(violations.join('; ')); return read; } factory \
         Batch.fromJsonUnjudged(dynamic json) => Batch._((json as List<dynamic>).indexed.map((e) => \
         _readAt(null, e.$1, () => e.$2 as String)).toList()); static Read \
         _readAt<Read>(Map<String, dynamic>? object, Object key, Read Function() read) { try { \
         return read(); } catch (refused) { if (object != null && !object.containsKey(key)) throw \
         FormatException('missing field `$key`', <Object>[key]); if (refused is TypeError) throw \
         FormatException('$refused', <Object>[key]); if (refused is FormatException) { final at = \
         refused.source; throw FormatException(refused.message, <Object>[key, if (at is \
         List<Object>) ...at]); } rethrow; } } dynamic toJson() => value.map((e) => e).toList(); \
         List<String> validateBound() => [ if (value.length < 1) \"too short: minimum length is \
         1, got ${value.length}\", if (value.length > 2) \"too long: maximum length is 2, got \
         ${value.length}\", ]; List<String> validate() => [ if (value.length < 1) \"too short: \
         minimum length is 1, got ${value.length}\", if (value.length > 2) \"too long: maximum \
         length is 2, got ${value.length}\", ]; }"
    );
}
