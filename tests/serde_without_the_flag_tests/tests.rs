//! The models of the build with the `serde` feature off, and what each surface writes of them.

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

/// A visibility whose `Hidden` variant carries `skip`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub enum Visibility {
    #[serde(skip)]
    Hidden,
    Shown,
}

/// A status carrying `rename_all`.
#[cfg(any(feature = "typescript", feature = "zod"))]
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Inactive,
    Pending,
    Suspended,
}

/// A note whose `draft` carries `skip_serializing`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Note {
    #[serde(skip_serializing)]
    pub draft: String,
    pub title: String,
}

/// [`Note`] with no `#[serde(...)]` attribute.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct NoteTwin {
    pub draft: String,
    pub title: String,
}

/// A choice carrying `untagged`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Choice {
    Name(String),
    Word(String),
}

/// [`Choice`] with no `#[serde(...)]` attribute.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub enum ChoiceTwin {
    Name(String),
    Word(String),
}

/// A value whose `Unnamed` variant carries `untagged` and `skip`, which the `serde` feature would
/// take off the wire.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub enum Loose {
    Named {
        label: String,
    },
    #[serde(untagged, skip)]
    Unnamed,
}

/// [`Loose`] with no `#[serde(...)]` attribute.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub enum LooseTwin {
    Named { label: String },
    Unnamed,
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
    assert_eq!(
        exported(&Note::ts_definition()).replace("Note", "NoteTwin"),
        exported(&NoteTwin::ts_definition())
    );
    assert_eq!(
        exported(&Choice::ts_definition()).replace("Choice", "ChoiceTwin"),
        exported(&ChoiceTwin::ts_definition())
    );
    assert_eq!(
        exported(&Loose::ts_definition()).replace("Loose", "LooseTwin"),
        exported(&LooseTwin::ts_definition())
    );
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_writes_a_plain_enum_under_its_rust_names() {
    let ts = UserStatus::ts_definition();

    assert!(ts.contains("export type UserStatus"), "Got: {ts}");
    for member in ["\"Active\"", "\"Inactive\"", "\"Pending\"", "\"Suspended\""] {
        assert!(ts.contains(member), "{member} in: {ts}");
    }
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_writes_a_member_under_skip_serializing() {
    let ts = Note::ts_definition();

    assert!(
        ts.lines().any(|line| line == "  draft: string;"),
        "Got: {ts}"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_writes_a_skipped_variant() {
    let ts = Visibility::ts_definition();

    assert!(
        ts.contains("export type Visibility =\n  | \"Hidden\"\n  | \"Shown\";"),
        "Got: {ts}"
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
    assert_eq!(
        Note::zod_schema().replace("Note", "NoteTwin"),
        NoteTwin::zod_schema()
    );
    assert_eq!(
        Choice::zod_schema().replace("Choice", "ChoiceTwin"),
        ChoiceTwin::zod_schema()
    );
    assert_eq!(
        Loose::zod_schema().replace("Loose", "LooseTwin"),
        LooseTwin::zod_schema()
    );
}

#[cfg(feature = "zod")]
#[test]
fn zod_writes_a_plain_enum_under_its_rust_names() {
    let zod = UserStatus::zod_schema();

    assert!(zod.contains("export const UserStatus$Schema"), "Got: {zod}");
    assert!(
        zod.contains("z.enum([\"Active\", \"Inactive\", \"Pending\", \"Suspended\"])"),
        "Got: {zod}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn zod_writes_a_member_under_skip_serializing() {
    let zod = Note::zod_schema();

    assert!(zod.contains("  draft: z.string()"), "Got: {zod}");
    assert!(!zod.contains(".optional()"), "Got: {zod}");
}

#[cfg(feature = "zod")]
#[test]
fn zod_writes_a_skipped_variant() {
    let zod = Visibility::zod_schema();

    assert!(
        zod.contains("z.enum([\"Hidden\", \"Shown\"])"),
        "Got: {zod}"
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
fn the_json_schema_requires_every_member() {
    let schema = Signup::json_schema();

    assert_eq!(
        schema["required"],
        serde_json::json!(["login_count", "team_roles", "user_name"]),
        "Got: {schema}"
    );
}

#[test]
fn the_json_schema_reads_no_serde_attribute() {
    assert_eq!(
        Signup::json_schema()
            .to_string()
            .replace("Signup", "SignupTwin"),
        SignupTwin::json_schema().to_string()
    );
    assert_eq!(
        Event::json_schema()
            .to_string()
            .replace("Event", "EventTwin"),
        EventTwin::json_schema().to_string()
    );
    assert_eq!(
        Note::json_schema().to_string().replace("Note", "NoteTwin"),
        NoteTwin::json_schema().to_string()
    );
    assert_eq!(
        Choice::json_schema()
            .to_string()
            .replace("Choice", "ChoiceTwin"),
        ChoiceTwin::json_schema().to_string()
    );
    assert_eq!(
        Loose::json_schema()
            .to_string()
            .replace("Loose", "LooseTwin"),
        LooseTwin::json_schema().to_string()
    );
}

#[test]
fn the_json_schema_requires_a_member_under_skip_serializing() {
    let schema = Note::json_schema();

    assert_eq!(
        schema["required"],
        serde_json::json!(["draft", "title"]),
        "Got: {schema}"
    );
}

#[cfg(feature = "dart")]
#[test]
fn dart_writes_every_key_it_reads() {
    assert_eq!(
        note_dart::dart_definition().replace("Note", "NoteTwin"),
        note_twin_dart::dart_definition()
    );
    assert_eq!(
        choice_dart::dart_definition().replace("Choice", "ChoiceTwin"),
        choice_twin_dart::dart_definition()
    );
    assert_eq!(
        loose_dart::dart_definition().replace("Loose", "LooseTwin"),
        loose_twin_dart::dart_definition()
    );
}

#[cfg(feature = "swift")]
#[test]
fn swift_writes_every_key_it_reads() {
    assert_eq!(
        note_swift::swift_definition().replace("Note", "NoteTwin"),
        note_twin_swift::swift_definition()
    );
    assert_eq!(
        choice_swift::swift_definition().replace("Choice", "ChoiceTwin"),
        choice_twin_swift::swift_definition()
    );
    assert_eq!(
        loose_swift::swift_definition().replace("Loose", "LooseTwin"),
        loose_twin_swift::swift_definition()
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn kotlin_writes_every_key_it_reads() {
    assert_eq!(
        note_kotlin::kotlin_definition().replace("Note", "NoteTwin"),
        note_twin_kotlin::kotlin_definition()
    );
    assert_eq!(
        choice_kotlin::kotlin_definition().replace("Choice", "ChoiceTwin"),
        choice_twin_kotlin::kotlin_definition()
    );
    assert_eq!(
        loose_kotlin::kotlin_definition().replace("Loose", "LooseTwin"),
        loose_twin_kotlin::kotlin_definition()
    );
}

#[test]
fn the_json_schema_writes_a_skipped_variant() {
    let schema = Visibility::json_schema();

    assert_eq!(
        schema["enum"],
        serde_json::json!(["Hidden", "Shown"]),
        "Got: {schema}"
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

/// The flag is tixschema's: serde's own derive reads its attributes in this build too.
#[test]
fn serde_still_reads_its_own_attributes() {
    let note: Note = serde_json::from_str(r#"{"draft":"plan","title":"Trip"}"#).unwrap();

    assert_eq!(note.draft, "plan");
    assert_eq!(serde_json::to_string(&note).unwrap(), r#"{"title":"Trip"}"#);
    assert_eq!(
        serde_json::to_string(&Visibility::Hidden)
            .unwrap_err()
            .to_string(),
        "the enum variant Visibility::Hidden cannot be serialized"
    );
    assert_eq!(
        serde_json::to_string(&Loose::Unnamed)
            .unwrap_err()
            .to_string(),
        "the enum variant Loose::Unnamed cannot be serialized"
    );
}
