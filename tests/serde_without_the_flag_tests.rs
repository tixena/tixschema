//! Models that derive serde and carry `#[serde(...)]` attributes, built with the `serde` feature
//! off.
//!
//! Without the feature TypeScript and Zod read no `#[serde(...)]` attribute: each member is its
//! Rust name in camelCase, no key is optional because of `default` or `skip_serializing_if`, and an
//! enum is written as it is with no attribute. `validate()` and the JSON Schema document name each
//! member by its Rust name.

#![cfg(all(not(feature = "serde"), any(feature = "typescript", feature = "zod")))]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A sign-up carrying `rename_all`, `rename`, `default` and `skip_serializing_if`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Signup {
    #[serde(default)]
    pub login_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub team_roles: Vec<String>,
    #[serde(rename = "handle")]
    #[model_schema_prop(check = ["knownName"])]
    pub user_name: String,
}

/// [`Signup`] with no `#[serde(...)]` attribute.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct SignupTwin {
    pub login_count: u32,
    pub team_roles: Vec<String>,
    #[model_schema_prop(check = ["knownName"])]
    pub user_name: String,
}

/// An event carrying `tag` and `rename_all`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Event {
    Closed,
    Opened { opened_at: u32 },
}

/// [`Event`] with no `#[serde(...)]` attribute.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub enum EventTwin {
    Closed,
    Opened { opened_at: u32 },
}

fn known_name(value: &str) -> Result<(), Vec<String>> {
    if value == "nobody" {
        return Err(vec!["a name is known".to_owned()]);
    }
    Ok(())
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_writes_each_member_in_camel_case_and_none_optional() {
    let ts = Signup::ts_definition();

    for member in [
        "  loginCount: number;",
        "  teamRoles: Array<string>;",
        "  userName: string;",
    ] {
        assert!(ts.lines().any(|line| line == member), "{member} in: {ts}");
    }
    for key in ["login-count", "team-roles", "handle", "?:"] {
        assert!(!ts.contains(key), "{key} in: {ts}");
    }
}

/// `ts` from its `export` on, past the header that carries the type's own doc.
#[cfg(feature = "typescript")]
fn exported(ts: &str) -> &str {
    &ts[ts.find("export ").unwrap()..]
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_reads_no_serde_attribute() {
    assert_eq!(
        exported(&Signup::ts_definition()).replace("Signup", "SignupTwin"),
        exported(&SignupTwin::ts_definition())
    );
    assert_eq!(
        exported(&Event::ts_definition()).replace("Event", "EventTwin"),
        exported(&EventTwin::ts_definition())
    );
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_writes_a_tagged_enum_as_it_is_with_no_attribute() {
    let ts = Event::ts_definition();

    assert!(ts.contains("  type: \"Opened\";"), "Got: {ts}");
    assert!(ts.contains("  openedAt: number;"), "Got: {ts}");
    for spelling in ["kind", "opened-at", "\"opened\""] {
        assert!(!ts.contains(spelling), "{spelling} in: {ts}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn zod_writes_each_key_in_camel_case_and_none_optional() {
    let zod = Signup::zod_schema();

    for key in [
        "  loginCount: z.number().int().min(0).max(4294967295),",
        "  teamRoles: z.array(z.string()),",
        "  userName: z.string()",
    ] {
        assert!(zod.contains(key), "{key} in: {zod}");
    }
    for spelling in [
        "login-count",
        "team-roles",
        "handle",
        ".prefault(",
        ".optional()",
    ] {
        assert!(!zod.contains(spelling), "{spelling} in: {zod}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn zod_reads_no_serde_attribute() {
    assert_eq!(
        Signup::zod_schema().replace("Signup", "SignupTwin"),
        SignupTwin::zod_schema()
    );
    assert_eq!(
        Event::zod_schema().replace("Event", "EventTwin"),
        EventTwin::zod_schema()
    );
}

#[cfg(feature = "zod")]
#[test]
fn zod_writes_a_tagged_enum_as_it_is_with_no_attribute() {
    let zod = Event::zod_schema();

    assert!(zod.contains("z.discriminatedUnion(\"type\""), "Got: {zod}");
    assert!(zod.contains("  openedAt: z.number()"), "Got: {zod}");
    for spelling in ["kind", "opened-at", "\"opened\""] {
        assert!(!zod.contains(spelling), "{spelling} in: {zod}");
    }
}

#[test]
fn the_json_schema_names_each_member_by_its_rust_name() {
    let schema = Signup::json_schema();
    let properties = schema["properties"].as_object().unwrap();

    assert_eq!(
        properties.keys().collect::<Vec<_>>(),
        ["login_count", "team_roles", "user_name"],
        "Got: {schema}"
    );
    for (member, described) in properties {
        assert!(described.get("default").is_none(), "{member} in: {schema}");
    }

    let event = Event::json_schema();
    assert_eq!(
        event["oneOf"][1]["properties"]["value"]["properties"]["opened_at"]["type"], "integer",
        "Got: {event}"
    );
}

#[test]
fn validate_names_a_member_by_its_rust_name() {
    let unknown = Signup {
        login_count: 0,
        team_roles: Vec::new(),
        user_name: "nobody".to_owned(),
    };

    assert_eq!(
        unknown.validate().unwrap_err(),
        ["'user_name': a name is known"]
    );
}
