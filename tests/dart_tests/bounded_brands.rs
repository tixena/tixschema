//! What the Dart backend writes for a brand's own string check. A build with no schema surface
//! refuses such a check, so these are declared where one is on.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A brand over a string, held to a length and a pattern.
#[model_schema(minLength = 2, pattern = "^[A-Z]+$")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Slug(pub String);

/// A brand with a parameter, whose string check is its declared default's.
#[model_schema(minLength = 2, default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Tag<IdType>(pub IdType);

/// A brand over a list, whose lengths count its items.
#[model_schema(no_display, minLength = 1, maxLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Batch(pub Vec<String>);

#[test]
fn every_declared_brand_is_constructible() {
    assert_eq!(Slug("AB".to_owned()).0, "AB");
    assert_eq!(Tag(7_u32).0, 7);
    assert_eq!(Batch(vec!["a".to_owned()]).0.len(), 1);
}

/// The bounds are measured on the value the brand wraps, and no message names a place: whatever
/// holds the brand writes its own key in front.
#[test]
fn a_brand_over_a_string_measures_its_value_under_no_place() {
    assert_eq!(
        slug_dart::dart_definition(),
        "class Slug { const Slug(this.value); final String value; factory Slug.fromJson(dynamic \
         json) => Slug(json as String); dynamic toJson() => value; List<String> validate() => [ \
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
        "class Tag<IdType> { const Tag(this.value); final IdType value; factory \
         Tag.fromJson(dynamic json, IdType Function(dynamic) idTypeFromJson) => \
         Tag(idTypeFromJson(json)); dynamic toJson(dynamic Function(IdType) idTypeToJson) => \
         idTypeToJson(value); List<String> validate() => [ if (value case final String held) if \
         (held.runes.length < 2) \"too short: minimum length is 2, got ${held.runes.length}\", ]; }"
    );
}

#[test]
fn a_brand_over_a_list_counts_its_items() {
    assert_eq!(
        batch_dart::dart_definition(),
        "class Batch { const Batch(this.value); final List<String> value; factory \
         Batch.fromJson(dynamic json) => Batch((json as List<dynamic>).map((e) => e as \
         String).toList()); dynamic toJson() => value.map((e) => e).toList(); List<String> \
         validate() => [ if (value.length < 1) \"too short: minimum length is 1, got \
         ${value.length}\", if (value.length > 2) \"too long: maximum length is 2, got \
         ${value.length}\", ]; }"
    );
}
