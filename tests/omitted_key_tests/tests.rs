//! The wire is the arbiter. serde writes two payloads for a field carrying a
//! `skip_serializing_if` — one with the key, one without — and each surface is held to admitting
//! exactly those two, asserted first here so the surface expectations below are read off them.
//! The attribute is on the declaration in every build, so every build owes the same answer.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// One omitted key beside the members that keep theirs: a plain required field and a sequence,
/// neither of which serde ever drops.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OmittedKeyFields {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    age: Option<u32>,
    id: String,
    roles: Vec<String>,
}

/// The same omission asked of a field that is not an `Option`. A predicate drops the key for a
/// value that is perfectly present in Rust, so the type under the key is unchanged — only whether
/// the key is written at all.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct PredicateOmittedKey {
    id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    roles: Vec<String>,
}

fn with_age() -> OmittedKeyFields {
    OmittedKeyFields {
        age: Some(30),
        id: "1".to_owned(),
        roles: vec![],
    }
}

fn without_age() -> OmittedKeyFields {
    OmittedKeyFields {
        age: None,
        id: "1".to_owned(),
        roles: vec![],
    }
}

fn with_roles() -> PredicateOmittedKey {
    PredicateOmittedKey {
        id: "1".to_owned(),
        roles: vec!["admin".to_owned()],
    }
}

fn without_roles() -> PredicateOmittedKey {
    PredicateOmittedKey {
        id: "1".to_owned(),
        roles: vec![],
    }
}

/// The two payloads every surface below is measured against.
#[test]
fn the_omitted_key_is_absent_from_the_payload_serde_writes() {
    assert_eq!(
        serde_json::to_string(&with_age()).unwrap(),
        r#"{"age":30,"id":"1","roles":[]}"#
    );
    assert_eq!(
        serde_json::to_string(&without_age()).unwrap(),
        r#"{"id":"1","roles":[]}"#
    );

    assert_eq!(
        serde_json::from_str::<OmittedKeyFields>(r#"{"id":"1","roles":[]}"#).unwrap(),
        without_age()
    );
}

#[test]
fn the_predicate_omitted_key_is_absent_from_the_payload_serde_writes() {
    assert_eq!(
        serde_json::to_string(&with_roles()).unwrap(),
        r#"{"id":"1","roles":["admin"]}"#
    );
    assert_eq!(
        serde_json::to_string(&without_roles()).unwrap(),
        r#"{"id":"1"}"#
    );

    assert_eq!(
        serde_json::from_str::<PredicateOmittedKey>(r#"{"id":"1"}"#).unwrap(),
        without_roles()
    );
    assert_eq!(
        serde_json::from_str::<PredicateOmittedKey>(r#"{"id":"1","roles":[]}"#).unwrap(),
        without_roles()
    );
}

/// `age` carries no `ts_optional`, so the attribute that drops its key leaves the spelling alone.
#[test]
#[cfg(feature = "typescript")]
fn typescript_writes_the_option_as_undefined_valued() {
    let ts = OmittedKeyFields::ts_definition();

    assert!(ts.contains("age: number | undefined;"), "Got: {ts}");
    assert!(!ts.contains("age?:"), "Got: {ts}");
    assert!(ts.contains("id: string;"), "Got: {ts}");
    assert!(ts.contains("roles: Array<string>;"), "Got: {ts}");
}

/// The key is optional; the value under it never is.
#[test]
#[cfg(feature = "typescript")]
fn typescript_writes_the_predicate_omitted_key_as_optional() {
    let ts = PredicateOmittedKey::ts_definition();

    assert!(ts.contains("roles?: Array<string>;"), "Got: {ts}");
    assert!(!ts.contains("roles: Array<string>"), "Got: {ts}");
    assert!(ts.contains("id: string;"), "Got: {ts}");
}

#[test]
#[cfg(feature = "zod")]
fn zod_keeps_the_undefined_union_that_already_admits_the_absent_key() {
    let zod = OmittedKeyFields::zod_schema();

    assert!(
        zod.contains(
            "age: z.union([z.null().transform(() => undefined), z.number().int(), z.undefined()]).prefault(undefined),"
        ),
        "Got: {zod}"
    );
    assert!(zod.contains("id: z.string(),"), "Got: {zod}");
    assert!(zod.contains("roles: z.array(z.string()),"), "Got: {zod}");
}

#[test]
#[cfg(feature = "zod")]
fn zod_marks_the_predicate_omitted_key_optional() {
    let zod = PredicateOmittedKey::zod_schema();

    assert!(
        zod.contains("roles: z.array(z.string()).optional(),"),
        "Got: {zod}"
    );
    assert!(zod.contains("id: z.string(),"), "Got: {zod}");
}

/// The JSON surface says it by leaving the field out of `required` while still describing it.
#[test]
#[cfg(feature = "jsonschema")]
fn the_json_schema_describes_the_omitted_key_without_requiring_it() {
    let schema = OmittedKeyFields::json_schema();

    assert!(schema["properties"]["age"].is_object());
    assert_eq!(
        schema["required"],
        serde_json::json!(["id", "roles"]),
        "Got: {schema}"
    );
}

/// Same for the predicate-dropped key: described, not required.
#[test]
#[cfg(feature = "jsonschema")]
fn the_json_schema_describes_the_predicate_omitted_key_without_requiring_it() {
    let schema = PredicateOmittedKey::json_schema();

    assert_eq!(
        schema["properties"]["roles"]["type"],
        serde_json::json!("array"),
        "Got: {schema}"
    );
    assert_eq!(
        schema["required"],
        serde_json::json!(["id"]),
        "Got: {schema}"
    );
}

#[test]
#[cfg(feature = "dart")]
fn dart_reads_the_predicate_omitted_key_through_a_null_guard() {
    let dart = predicate_omitted_key_dart::dart_definition();

    assert!(dart.contains("final List<String>? roles;"), "Got: {dart}");
    assert!(
        dart.contains("this.roles,") && !dart.contains("required this.roles,"),
        "Got: {dart}"
    );
    assert!(
        dart.contains("] == null ? null : (json['"),
        "casting the key serde left out is a `TypeError` on the payload without it. Got: {dart}"
    );
    assert!(dart.contains("if (roles != null)"), "Got: {dart}");
    assert!(dart.contains("required this.id,"), "Got: {dart}");
}

/// The `Option` carrying the same omission was already nullable, and is pinned here as undisturbed.
#[test]
#[cfg(feature = "dart")]
fn dart_leaves_the_optional_field_carrying_the_same_omission_alone() {
    let dart = omitted_key_fields_dart::dart_definition();

    assert!(dart.contains("final int? age;"), "Got: {dart}");
    assert!(
        dart.contains("this.age,") && !dart.contains("required this.age,"),
        "Got: {dart}"
    );
    assert!(dart.contains("if (age != null)"), "Got: {dart}");
    assert!(
        dart.contains("required this.roles,") && dart.contains("final List<String> roles;"),
        "`roles` here carries no omission at all, so its key is written every time. Got: {dart}"
    );
}
