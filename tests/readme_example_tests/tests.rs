//! The README's `Option` examples, expanded here as the README declares them.

#![cfg(all(feature = "serde", feature = "typescript"))]

mod services;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tixschema::model_schema;

/// The predicate bullet: the key goes missing when the predicate fires, so the member is described
/// under an optional key rather than left untouched.
const PREDICATE_BULLET: &str = "- `#[serde(skip_serializing_if = \"...\")]` -- the key is left out of the payload when the predicate fires: `roles: z.array(z.string()).optional(),` in Zod and no `required` entry in the JSON Schema. On a field that is not an `Option` the TypeScript member takes the optional key too, `roles?: Array<string>;`, there being no second spelling for it; an `Option` field keeps `T | undefined` unless [`ts_optional`](#ts_optional) asks otherwise";

/// The bare-`skip` bullet, whose claim is an absence on every surface.
const SKIP_BULLET: &str = "- `#[serde(skip)]` -- the key is written into no payload and read out of none, so no surface describes the member at all: no TypeScript member, no Zod key, and neither a `properties` nor a `required` entry. On a tuple-struct or tuple-variant slot it takes the slot out of the described tuple, which shortens the arity -- and a variant declaring one slot becomes a unit variant, which is what serde writes for it";

/// The write-half bullet, which lands where the predicate bullet does.
const WRITE_HALF_BULLET: &str = "- `#[serde(skip_serializing)]` -- the write half of `skip`: the key is left out of every payload while a supplied one is still read, and every surface answers as it does for `skip_serializing_if`";

/// The read-half bullet, the one spelling of the three that keeps a required key.
const READ_HALF_BULLET: &str = "- `#[serde(skip_deserializing)]` -- the read half: the key is written into every payload while a supplied one is discarded, so the member keeps a required key";

/// The predicate spelling the bullet names, declared as the bullet describes it.
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct PredicateOmittedKey {
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

/// The bare `skip`, whose member no surface carries.
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct FullySkipped {
    pub id: String,
    #[serde(skip)]
    pub roles: Vec<String>,
}

/// The write half alone.
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct WriteHalfDropped {
    pub id: String,
    #[serde(default, skip_serializing)]
    pub roles: Vec<String>,
}

/// The read half alone.
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct ReadHalfDropped {
    pub id: String,
    #[serde(skip_deserializing)]
    pub roles: Vec<String>,
}

fn readme() -> &'static str {
    include_str!("../../README.md")
}

/// The README declares this, and the generator writes that.
///
/// A member is matched as a whole line on both sides, so a spelling that is merely a prefix of the
/// emitted one cannot pass for it.
fn assert_readme_declares_and_shows(declaration: &str, emission: &str, members: &[&str]) {
    assert!(
        readme().contains(declaration),
        "the README no longer declares this verbatim:\n{declaration}"
    );
    for member in members {
        assert!(
            emission.lines().any(|line| line == *member),
            "the generator no longer writes this member: {member}\nGot: {emission}"
        );
        assert!(
            readme().lines().any(|line| line == *member),
            "the README no longer shows this member verbatim: {member}"
        );
    }
}

/// The "A Struct's Own Tag" example: the key serde writes the struct's name under is a member.
#[test]
fn test_the_struct_tag_example_is_declarable_and_shows_what_it_emits() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind")]
    pub struct Tagged {
        pub name: String,
    }

    let declaration = "#[model_schema()]\n#[derive(Serialize, Deserialize)]\n#[serde(tag = \
                       \"kind\")]\npub struct Tagged {\n    pub name: String,\n}";
    assert_readme_declares_and_shows(
        declaration,
        &Tagged::ts_definition(),
        &[
            "export type Tagged = {",
            "  kind: \"Tagged\";",
            "  name: string;",
        ],
    );
    #[cfg(feature = "zod")]
    assert_readme_declares_and_shows(
        declaration,
        &Tagged::zod_schema(),
        &[
            "const Tagged$RawSchema = z.strictObject({",
            "  kind: z.literal(\"Tagged\"),",
            "  name: z.string(),",
            "});",
        ],
    );
    assert_eq!(
        serde_json::to_value(Tagged {
            name: "x".to_owned()
        })
        .unwrap(),
        serde_json::json!({ "kind": "Tagged", "name": "x" })
    );
}

/// The "Optional Fields" example: three keys the payload may omit, beside two it may not.
#[test]
fn test_the_optional_fields_example_is_declarable_and_shows_what_it_emits() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct UserWithOptionals {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub avatar_url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub email: Option<String>,
        pub id: String,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub phone: Option<String>,
    }

    assert_readme_declares_and_shows(
        "    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub email: Option<String>,\n",
        &UserWithOptionals::ts_definition(),
        &[
            "export type UserWithOptionals = {",
            "  id: string;",
            "  name: string;",
            "  email: string | undefined;",
            "  phone: string | undefined;",
            "  avatar_url: string | undefined;",
            "};",
        ],
    );
}

/// A bullet from the "Supported Serde attributes" list, held to what it claims the generator
/// writes.
fn assert_readme_bullet_shows(bullet: &str, emission: &str, spellings: &[&str]) {
    assert!(
        readme().lines().any(|line| line == bullet),
        "the README no longer carries this bullet verbatim:\n{bullet}"
    );
    for spelling in spellings {
        assert!(
            bullet.contains(spelling),
            "the bullet no longer quotes this spelling: {spelling}"
        );
        assert!(
            emission.contains(spelling),
            "the generator no longer writes this spelling: {spelling}\nGot: {emission}"
        );
    }
}

/// The predicate bullet quotes two spellings, and both are read off a real emission.
#[test]
fn test_the_predicate_bullet_shows_the_optional_key_it_claims() {
    assert_readme_bullet_shows(
        PREDICATE_BULLET,
        &PredicateOmittedKey::ts_definition(),
        &["roles?: Array<string>;"],
    );

    #[cfg(feature = "zod")]
    assert_readme_bullet_shows(
        PREDICATE_BULLET,
        &PredicateOmittedKey::zod_schema(),
        &["roles: z.array(z.string()).optional(),"],
    );
}

#[test]
fn test_the_skip_bullets_show_the_three_answers_they_separate() {
    assert_readme_bullet_shows(SKIP_BULLET, &FullySkipped::ts_definition(), &[]);
    let written = serde_json::to_string(&FullySkipped {
        id: "1".to_owned(),
        roles: vec!["x".to_owned()],
    })
    .unwrap();
    assert_eq!(written, r#"{"id":"1"}"#);
    let read_back = serde_json::from_str::<FullySkipped>(r#"{"id":"1","roles":["x"]}"#).unwrap();
    assert_eq!(read_back.roles, Vec::<String>::new());
    let skipped = FullySkipped::ts_definition();
    assert!(!skipped.contains("roles"), "Got: {skipped}");

    assert_readme_bullet_shows(WRITE_HALF_BULLET, &WriteHalfDropped::ts_definition(), &[]);
    let supplied = serde_json::from_str::<WriteHalfDropped>(r#"{"id":"1","roles":["x"]}"#).unwrap();
    assert_eq!(supplied.roles, vec!["x".to_owned()]);
    let write_half = WriteHalfDropped::ts_definition();
    assert!(
        write_half.contains("roles?: Array<string>;"),
        "Got: {write_half}"
    );

    assert_readme_bullet_shows(READ_HALF_BULLET, &ReadHalfDropped::ts_definition(), &[]);
    let read_half = ReadHalfDropped::ts_definition();
    assert!(
        read_half.contains("roles: Array<string>;"),
        "Got: {read_half}"
    );
    assert!(!read_half.contains("roles?"), "Got: {read_half}");
}

#[test]
fn test_the_optional_fields_section_shows_the_slot_tuple_it_claims() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct Pair(#[serde(skip)] pub Option<String>, pub String);

    assert_readme_declares_and_shows(
        "`struct Pair(#[serde(skip)] Option<String>, String)`",
        &Pair::ts_definition(),
        &[],
    );

    let written = serde_json::to_string(&Pair(Some("s".to_owned()), "x".to_owned())).unwrap();
    assert_eq!(written, r#"["x"]"#);
    assert_eq!(serde_json::from_str::<Pair>(r#"["x"]"#).unwrap().0, None);
    assert!(readme().contains(r#"writes `["x"]`, reads that back, and refuses `["s","x"]`"#));

    let ts = Pair::ts_definition();
    assert!(ts.contains("= [string];"), "Got: {ts}");
    assert!(readme().contains("`[string]` in TypeScript"));

    #[cfg(feature = "zod")]
    {
        let zod = Pair::zod_schema();
        assert!(zod.contains("= z.tuple([z.string()]);"), "Got: {zod}");
        assert!(readme().contains("`z.tuple([z.string()])` in Zod"));
    }
}

/// The "Collections and Maps" example.
#[test]
fn test_the_collections_example_is_declarable() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct UserWithCollections {
        pub id: String,
        pub metadata: HashMap<String, String>,
        pub scores: Vec<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub settings: Option<HashMap<String, String>>,
        pub tags: Vec<String>,
    }

    assert_readme_declares_and_shows(
        "    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub settings: Option<HashMap<String, String>>,\n",
        &UserWithCollections::ts_definition(),
        &[],
    );
}

#[test]
#[cfg(feature = "mongodb")]
fn test_the_object_id_example_is_declarable_and_shows_what_it_emits() {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ObjectId(pub String);

    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct Document {
        pub author_id: ObjectId,
        pub id: ObjectId,
        pub metadata: HashMap<String, ObjectId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parent_id: Option<ObjectId>,
        pub related_docs: HashMap<String, Vec<ObjectId>>,
        pub tags: Vec<ObjectId>,
        pub title: String,
    }

    assert_readme_declares_and_shows(
        "    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub parent_id: Option<ObjectId>,\n",
        &Document::ts_definition(),
        &[
            "export type Document = {",
            "  id: ObjectId;",
            "  title: string;",
            "  author_id: ObjectId;",
            "  tags: Array<ObjectId>;",
            "  metadata: Partial<Record<string, ObjectId>>;",
            "  parent_id: ObjectId | undefined;",
            "  related_docs: Partial<Record<string, Array<ObjectId>>>;",
            "};",
        ],
    );

    #[cfg(feature = "zod")]
    assert_readme_declares_and_shows(
        "pub struct Document {",
        &Document::zod_schema(),
        &[
            "  parent_id: z.union([z.null().transform(() => undefined), z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: \"Invalid ObjectId\" }) }), z.undefined()]).prefault(undefined),",
        ],
    );
}

#[test]
#[cfg(feature = "chrono")]
fn test_the_chrono_example_is_declarable_and_shows_what_it_emits() {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Event {
        pub created_at: DateTime<Utc>,
        pub date: NaiveDate,
        #[model_schema_prop(as_number)]
        pub epoch_ms: DateTime<Utc>,
        pub local_datetime: NaiveDateTime,
        pub name: String,
        pub time: NaiveTime,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub updated_at: Option<DateTime<Utc>>,
    }

    assert_readme_declares_and_shows(
        "    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub updated_at: Option<DateTime<Utc>>,\n",
        &Event::ts_definition(),
        &[
            "export type Event = {",
            "  name: string;",
            "  date: string;",
            "  time: string;",
            "  local_datetime: string;",
            "  created_at: Date;",
            "  epoch_ms: number;",
            "  updated_at: Date | undefined;",
            "};",
        ],
    );

    #[cfg(feature = "zod")]
    assert_readme_declares_and_shows(
        "pub struct Event {",
        &Event::zod_schema(),
        &[
            "  created_at: z.coerce.date(),",
            "  updated_at: z.union([z.null().transform(() => undefined), z.coerce.date(), z.undefined()]).prefault(undefined),",
        ],
    );
}

/// The README declares this, and the block beside it is the emission whole — held as one block
/// rather than line by line, since a truncated paste (matching only the first several lines)
/// would still leave the reader without the factory the rest declares.
fn assert_readme_declares_and_shows_whole(declaration: &str, emission: &str) {
    assert!(
        readme().contains(declaration),
        "the README no longer declares this verbatim:\n{declaration}"
    );
    assert!(
        readme().contains(emission.trim_end()),
        "the README no longer shows this emission whole:\n{emission}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_the_branded_newtype_example_is_declarable_and_shows_what_it_emits() {
    #[model_schema(default_types(IdType = String))]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct UserId<IdType>(pub IdType);

    #[model_schema()]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct CorrelationId(pub String);

    assert_readme_declares_and_shows_whole(
        "pub struct UserId<IdType>(pub IdType);",
        &UserId::<String>::ts_definition(),
    );
    assert_readme_declares_and_shows_whole(
        "pub struct UserId<IdType>(pub IdType);",
        &UserId::<String>::zod_schema(),
    );
    assert_readme_declares_and_shows_whole(
        "pub struct CorrelationId(pub String);",
        &CorrelationId::ts_definition(),
    );
    assert_readme_declares_and_shows_whole(
        "pub struct CorrelationId(pub String);",
        &CorrelationId::zod_schema(),
    );
}

#[test]
#[cfg(not(feature = "zod"))]
fn test_the_branded_newtype_example_shows_what_a_build_without_zod_emits() {
    #[model_schema(default_types(IdType = String))]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct UserId<IdType>(pub IdType);

    #[model_schema()]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct CorrelationId(pub String);

    assert_readme_declares_and_shows_whole(
        "pub struct UserId<IdType>(pub IdType);",
        &UserId::<String>::ts_definition(),
    );
    assert_readme_declares_and_shows_whole(
        "pub struct CorrelationId(pub String);",
        &CorrelationId::ts_definition(),
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_the_documented_branded_newtype_example_is_declarable_and_shows_what_it_emits() {
    /// Generic document identifier.
    ///
    /// - `DocumentId<String>` for API/HTTP layer
    /// - `DocumentId<ObjectId>` for `MongoDB` layer
    ///
    /// ```rust example
    /// DocumentId("64de3d95ff45b119e5b53a7e".to_string())
    /// ```
    #[model_schema(default_types(IdType = String))]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct DocumentId<IdType>(pub IdType);

    assert_readme_declares_and_shows_whole(
        "pub struct DocumentId<IdType>(pub IdType);",
        &DocumentId::<String>::zod_schema(),
    );
}

/// The rejected half of the constrained-brand rule for an inner written over a parameter.
#[test]
fn test_the_constrained_brand_over_a_parameterised_inner_example_is_still_shown() {
    assert!(
        readme().contains("pub struct TaggedSlug<T>(pub Tagged<T>);"),
        "the README no longer shows the rejected declaration"
    );
    assert!(
        readme().contains("pub struct Tagged<T>(pub T);"),
        "the README no longer shows the inner the rejected declaration is written over"
    );
}
