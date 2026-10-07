//! `from_bson_with` on structs and enums, held to what plain serde says of the same document. Two binaries compile this module, each with the name `bson` bound to one major
//! version of the library, so every case runs against both.
//!
//! An issue is asserted by its kind, its path, its expected type and the value it holds. Its
//! `reason` is the `bson` library's own wording, which its two versions write differently, so a
//! test reads it only where it is about it.

/// The `with` module of [`Hooked::count`]: a number written as text.
mod as_text {
    use core::fmt::Display;

    use serde::Deserialize as _;
    use serde::de::Error as _;

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }

    pub(super) fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: Display,
    {
        serializer.collect_str(value)
    }
}
mod aliased;
mod bounds;
mod flattened;
mod piped;
mod readme;

use core::error::Error;
use core::fmt::Display;
use core::str::FromStr;
use std::collections::HashMap;

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use chrono::{DateTime, Utc};
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// What `$issues` say less each `reason`, whichever type's `$module` declares them.
macro_rules! told {
    ($module:ident, $issues:expr) => {
        $issues
            .iter()
            .map(|issue| match issue {
                $module::Issue::Invalid {
                    path,
                    expected,
                    found,
                    reason: _reason,
                } => (
                    "Invalid",
                    path.to_string(),
                    format!("{expected:?}"),
                    Some(found.clone()),
                ),
                $module::Issue::Missing { path, expected } => {
                    ("Missing", path.to_string(), format!("{expected:?}"), None)
                }
                $module::Issue::Mistyped {
                    path,
                    expected,
                    found,
                } => (
                    "Mistyped",
                    path.to_string(),
                    format!("{expected:?}"),
                    Some(found.clone()),
                ),
                $module::Issue::NoVariant {
                    path,
                    found,
                    variants: _variants,
                } => (
                    "NoVariant",
                    path.to_string(),
                    String::new(),
                    Some(found.clone()),
                ),
                $module::Issue::Undescribed { reason: _reason } => {
                    ("Undescribed", String::new(), String::new(), None)
                }
                $module::Issue::Unknown { path, found } => (
                    "Unknown",
                    path.to_string(),
                    String::new(),
                    Some(found.clone()),
                ),
            })
            .collect::<Vec<Told>>()
    };
}

/// The `reason` of every issue among `$issues` that carries one, in order.
macro_rules! reasons {
    ($module:ident, $issues:expr) => {
        $issues
            .iter()
            .filter_map(|issue| {
                if let $module::Issue::Invalid {
                    path: _path,
                    expected: _expected,
                    found: _found,
                    reason,
                } = issue
                {
                    Some(reason.as_str())
                } else if let $module::Issue::Undescribed { reason } = issue {
                    Some(reason.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<&str>>()
    };
}

/// What each variant's own list says less each `reason`, in the one `NoVariant` among `$issues`.
macro_rules! tried {
    ($module:ident, $issues:expr) => {
        $issues
            .iter()
            .filter_map(|issue| {
                if let $module::Issue::NoVariant {
                    path: _path,
                    found: _found,
                    variants,
                } = issue
                {
                    Some(variants)
                } else {
                    None
                }
            })
            .flatten()
            .map(|(variant, list)| (*variant, told!($module, list)))
            .collect::<Vec<(&str, Vec<Told>)>>()
    };
}

/// One read hook per name: text, refused where it is empty under the hook's own name.
macro_rules! read_hooks {
    ($($name:ident),+) => {$(
        fn $name<'de, D>(deserializer: D) -> Result<String, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let text = String::deserialize(deserializer)?;
            if text.is_empty() {
                return Err(D::Error::custom(concat!(
                    "`",
                    stringify!($name),
                    "` reads no empty text"
                )));
            }
            Ok(text)
        }
    )+};
}

/// One issue less its `reason`: its kind, its path, its expected type, and the value it holds.
type Told = (&'static str, String, String, Option<Bson>);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Version {
    number: i32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    created_at: DateTime<Utc>,
    #[serde(rename = "recordId")]
    id: ObjectId,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    versions: Vec<Version>,
}

/// Ids and numbers held one by one, in a list, a map and an optional field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct CatalogByItem {
    counts: Vec<i32>,
    owners: HashMap<String, ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<ObjectId>,
    tags: Vec<ObjectId>,
}

/// One field per read hook, and one carrying a constraint, which hangs none.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Hooked {
    #[serde(deserialize_with = "upper_only")]
    code: String,
    #[serde(with = "as_text")]
    count: u32,
    #[model_schema_prop(minLength = 3)]
    name: String,
    #[serde(deserialize_with = "parsed", serialize_with = "shown")]
    port: u16,
    #[model_schema_prop(as_number)]
    seen_at: DateTime<Utc>,
}

/// A model type and a plain value, each stored under its name or an alias.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Aliased {
    #[serde(alias = "newest", default, skip_serializing_if = "Option::is_none")]
    latest: Option<Version>,
    #[serde(alias = "fullName", alias = "label")]
    name: String,
}

/// Model types reached through a map, an `Option`, a `Box` and a map of lists.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Bundle {
    boxed: Box<Version>,
    by_name: HashMap<String, Version>,
    by_team: HashMap<String, Vec<Version>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latest: Option<Version>,
}

/// Plain values in a list of lists, an optional list and a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Tally {
    grid: Vec<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    marks: Option<Vec<i32>>,
    scores: HashMap<String, i32>,
}

/// No field serde reads: the walker has no value to read, and declares the one key written.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Derived {
    #[serde(skip_deserializing)]
    total: u32,
}

/// A type whose own `Deserialize` reads nothing, which no walk can see into.
#[model_schema(decode_with)]
#[derive(Debug, PartialEq, Serialize)]
struct Sealed {
    label: String,
}

impl<'de> Deserialize<'de> for Sealed {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(D::Error::custom("sealed: nothing reads this"))
    }
}

/// A field serde writes wherever a predicate lets it be, and never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Kept {
    id: String,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct KeptText {
    id: String,
    #[serde(skip_deserializing, skip_serializing_if = "String::is_empty")]
    note: String,
}

/// Two fields serde neither writes nor reads: one under `skip`, one under its two halves.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Shelved {
    #[serde(skip)]
    cached: u8,
    id: String,
    #[serde(skip_serializing, skip_deserializing)]
    local: u8,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Envelope {
    sealed: Sealed,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Pair(String, u32);

/// A tuple struct, and a tuple as a field and in a list.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Placement {
    pair: Pair,
    spot: (String, u32),
    spots: Vec<(String, u32)>,
}

/// A brand over an id: serde writes it as the id it holds.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct OwnerId(ObjectId);

/// A single-slot tuple struct with no `transparent`: serde writes it as a brand is written.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Wrapper(String);

/// A brand whose own reader refuses text shorter than three characters. Only a schema surface
/// hangs that check, and the declaration is refused in a build with none.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(decode_with, minLength = 3)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Code(String);

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Coded {
    code: Code,
}

/// A brand over a model type.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Pinned(Version);

/// A brand over text.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct RecordId(String);

/// A brand over another brand: serde writes it as the text the inner one holds.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct OwnerRef(RecordId);

/// A brand over a tuple struct: serde writes it as the list the tuple struct is.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct PairRef(Pair);

/// A brand over a plain enum: serde writes it as the variant's name.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct StatusRef(Status);

/// A brand over another brand, over a tuple struct and over a plain enum, each under a key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Referenced {
    owner: OwnerRef,
    pair: PairRef,
    status: StatusRef,
}

/// A flattened brand over text, which serde refuses to flatten at every read. A schema surface
/// refuses the declaration.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Carded {
    #[serde(flatten)]
    id: RecordId,
    name: String,
}

/// A `transparent` struct with a named field over an id: serde writes it as the id it holds.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Owner {
    id: ObjectId,
}

/// A `transparent` struct with a named field over a model type.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Current {
    version: Version,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Assigned {
    current: Current,
    owner: Owner,
}

/// A `transparent` tuple struct beside whose value is a slot serde neither writes nor reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Tags(Vec<String>, #[serde(skip)] u8);

/// A `transparent` tuple struct whose value is its second slot, over an id.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Keeper(#[serde(skip)] u8, ObjectId);

/// A `transparent` tuple struct with a slot serde never reads, over a model type.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Stamped(Version, #[serde(skip)] u8);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Filing {
    keeper: Keeper,
    stamped: Stamped,
    tags: Tags,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Ping;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Labelled {
    owner: OwnerId,
    ping: Ping,
    pinned: Pinned,
    wrapper: Wrapper,
}

#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Page<T> {
    items: Vec<T>,
    total: u32,
}

/// A model type that carries no flag: it only ever fills a parameter.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Plain {
    label: String,
}

/// A generic brand.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Tagged<T>(T);

#[model_schema(decode_with, default_types(A = String, B = i32))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Pairing<A, B> {
    left: A,
    right: B,
}

/// serde's derive asks `Default` of the parameter here, beyond `Deserialize`.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Lenient<T> {
    #[serde(default)]
    extra: T,
    total: u32,
}

/// Parameters named as the methods the flag adds name their own.
#[model_schema(decode_with, default_types(I = String, F = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Keyed<I, F> {
    format: F,
    id: I,
}

/// A type called what the methods the flag adds call a type parameter of their own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct I {
    number: i32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Holder {
    inner: I,
}

/// A type with no parameter, naming generic types filled with a model type and with an id.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Listing {
    page: Page<Version>,
    tagged: Tagged<ObjectId>,
}

#[model_schema()]
type RecordAlias = Record;

/// A field typed with an alias of a flagged model type.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Shelf {
    featured: RecordAlias,
}

/// A plain enum: serde writes the variant's name.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Status {
    Draft,
    Published,
}

/// Externally tagged: `"Empty"`, `{"Label": "x"}`, `{"Circle": {"radius": 1.0}}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Outline {
    Circle { radius: f64 },
    Empty,
    Label(String),
    Owned(ObjectId),
}

/// Internally tagged: `{"kind": "Solid", "color": "red"}`, `{"kind": "Versioned", "number": 3}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Fill {
    Clear,
    Solid { color: String },
    Versioned(Version),
}

/// Adjacently tagged: `{"kind": "Dashed", "data": {"gap": 2}}`, `{"kind": "Hairline"}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Stroke {
    Dashed { gap: u32 },
    Hairline,
    Level(Option<u32>),
    Span(u32, u32),
    Width(u32),
}

/// One enum of each tagged form.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Drawing {
    fill: Fill,
    outline: Outline,
    stroke: Stroke,
}

/// Untagged, with a member whose own check takes its variant out. Only a schema surface hangs it.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Contact {
    Email {
        #[model_schema_prop(minLength = 3)]
        address: String,
    },
    Phone {
        country: i32,
        digits: String,
    },
    Versioned(Version),
}

/// A variant holding two values.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Move {
    Stay,
    To(i32, i32),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Route {
    contact: Contact,
    next: Move,
    status: Status,
}

/// Every renaming serde reads off an enum: of its variants, of one variant, of every variant's
/// fields, and of one variant's fields.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum Shipment {
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    ByAir {
        flight_code: String,
    },
    BySea {
        vessel_name: String,
    },
    #[serde(rename = "pickup")]
    InPerson,
}

/// A tag naming no variant that serde reads all the same, as the variant marked `other`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Signal {
    Go {
        speed: u32,
    },
    #[serde(other)]
    Unrecognized,
}

/// A generic enum: a `T` is read whole where it sits.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Answer<T> {
    Empty,
    Value(T),
}

/// Internally tagged, with a variant serde also reads under an alias and one it never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Coating {
    #[serde(alias = "Blank")]
    Clear,
    #[serde(skip_deserializing)]
    Hidden,
    Solid {
        color: String,
    },
}

/// Externally tagged, each variant also read under an alias.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Contour {
    #[serde(alias = "Round")]
    Circle { radius: f64 },
    #[serde(alias = "Blank")]
    Empty,
}

/// Adjacently tagged, with a variant serde also reads under an alias.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Dash {
    #[serde(alias = "Dotted")]
    Dashed {
        gap: i32,
    },
    Hairline,
}

/// Untagged, with a variant serde never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Reach {
    Email {
        address: String,
    },
    #[serde(skip_deserializing)]
    Pager {
        number: i32,
    },
    Phone {
        country: i32,
        digits: String,
    },
}

/// One enum of each form a variant's name is stored in: under a tag, and as text.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Sketch {
    coating: Coating,
    contour: Contour,
}

/// What [`item`] reads: a count, or text that holds one.
#[derive(Deserialize)]
#[serde(untagged)]
enum Count {
    Number(u32),
    Text(String),
}

/// A read hook named like what the walker binds the value of a key as.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Named {
    #[serde(deserialize_with = "held")]
    name: String,
}

/// One field per name the walker gives a value of its own, each read by the hook of that name.
/// `stored` is bound where a key has an alias, and `read` is what a writer is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Bound {
    #[serde(deserialize_with = "found")]
    found: String,
    #[serde(deserialize_with = "held")]
    held: String,
    #[serde(deserialize_with = "issue")]
    issue: String,
    #[serde(deserialize_with = "item", serialize_with = "read")]
    item: u32,
    #[serde(alias = "id", deserialize_with = "key")]
    key: String,
    #[serde(deserialize_with = "object")]
    object: String,
    #[serde(deserialize_with = "out")]
    out: String,
    #[serde(deserialize_with = "path")]
    path: String,
    #[serde(alias = "saved", deserialize_with = "stored")]
    stored: String,
}

/// Adjacently tagged: the tag is bound as `tag`, what the variant holds as `content`, and its
/// document as `inner`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
enum Adjacent {
    Note {
        #[serde(deserialize_with = "content")]
        content: String,
        #[serde(deserialize_with = "inner")]
        inner: String,
        #[serde(deserialize_with = "tag")]
        tag: String,
    },
}

/// The read hook of [`Hooked::code`]: text that carries no lower-case letter.
fn upper_only<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    if text.chars().any(char::is_lowercase) {
        return Err(D::Error::custom(format!("{text:?} is not upper case")));
    }
    Ok(text)
}

/// A read hook generic over what it reads, which only the field it hangs on pins.
fn parsed<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(D::Error::custom)
}

/// The write hook paired with [`parsed`].
fn shown<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: Display,
{
    serializer.collect_str(value)
}

read_hooks!(
    content, found, held, inner, issue, key, object, out, path, stored, tag
);

/// The read hook of [`Bound::item`]: a count, or text that holds one.
fn item<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Count::deserialize(deserializer)? {
        Count::Number(count) => Ok(count),
        Count::Text(text) => text.parse().map_err(D::Error::custom),
    }
}

/// The write hook of [`Bound::item`], named like the value the walker hands it: text.
fn read<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: Display,
{
    serializer.collect_str(value)
}

/// Whether the name `bson` is bound to version 2 of the library in the package this is built in:
/// tixschema's own binds it to version 2, and `bson3/` to version 3.
fn built_against_version_2() -> bool {
    env!("CARGO_PKG_NAME") == "tixschema"
}

/// What `value` is stored as, written through the serializer both versions of the library have.
fn written<T>(value: &T) -> Document
where
    T: Serialize,
{
    value
        .serialize(bson::Serializer::new())
        .unwrap()
        .as_document()
        .unwrap()
        .clone()
}

/// What plain serde says of `stored_row`, read through the deserializer both versions have.
fn serde_reads<T>(stored_row: &Document) -> bool
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(bson::Deserializer::new(Bson::Document(stored_row.clone()))).is_ok()
}

fn oid(hex: &str) -> ObjectId {
    ObjectId::parse_str(hex).unwrap()
}

fn string(held: &str) -> Bson {
    Bson::String(held.to_owned())
}

fn invalid(path: &str, expected: &str, found: Bson) -> Told {
    ("Invalid", path.to_owned(), expected.to_owned(), Some(found))
}

fn missing(path: &str, expected: &str) -> Told {
    ("Missing", path.to_owned(), expected.to_owned(), None)
}

fn mistyped(path: &str, expected: &str, found: Bson) -> Told {
    (
        "Mistyped",
        path.to_owned(),
        expected.to_owned(),
        Some(found),
    )
}

fn unknown(path: &str, found: Bson) -> Told {
    ("Unknown", path.to_owned(), String::new(), Some(found))
}

fn undescribed() -> Told {
    ("Undescribed", String::new(), String::new(), None)
}

fn drawing() -> Drawing {
    Drawing {
        fill: Fill::Versioned(Version { number: 3_i32 }),
        outline: Outline::Circle { radius: 1.5_f64 },
        stroke: Stroke::Dashed { gap: 2 },
    }
}

fn route() -> Route {
    Route {
        contact: Contact::Phone {
            country: 1_i32,
            digits: "555".to_owned(),
        },
        next: Move::To(1_i32, 2_i32),
        status: Status::Draft,
    }
}

fn record(numbers: &[i32]) -> Record {
    Record {
        created_at: DateTime::from_timestamp_millis(1_759_600_000_000).unwrap(),
        id: oid("6a7cc592ca0574e6efdfe217"),
        name: "Loan".to_owned(),
        note: None,
        versions: numbers.iter().map(|&number| Version { number }).collect(),
    }
}

/// A row as `Record` writes it, holding one version.
fn plain_row() -> Document {
    doc! {
        "recordId": oid("6a7cc592ca0574e6efdfe217"),
        "name": "Loan",
        "createdAt": "2025-10-04T17:46:40Z",
        "versions": [{ "number": 1_i32 }],
    }
}

fn hooked() -> Hooked {
    Hooked {
        code: "AB".to_owned(),
        count: 5,
        name: "Loan".to_owned(),
        port: 80,
        seen_at: DateTime::from_timestamp_millis(1_759_600_000_000).unwrap(),
    }
}

fn bound() -> Bound {
    Bound {
        found: "a".to_owned(),
        held: "b".to_owned(),
        issue: "c".to_owned(),
        item: 5,
        key: "d".to_owned(),
        object: "e".to_owned(),
        out: "f".to_owned(),
        path: "g".to_owned(),
        stored: "h".to_owned(),
    }
}

/// Repairs what older writers left in a `Record` row, inner versions included, and rejects
/// anything else.
fn repair_record(
    row: &mut Document,
    issues: &[record_schema::Issue<Bson>],
) -> record_schema::Verdict {
    use record_schema::{Expected, Issue, Verdict};

    for issue in issues {
        if let Issue::Mistyped {
            path,
            expected: Expected::ObjectId,
            found: Bson::String(hex),
        } = issue
        {
            let Ok(id) = ObjectId::parse_str(hex) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::ObjectId(id));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::DateTime(date),
            reason: _reason,
        } = issue
        {
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(date.timestamp_millis())
            else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::Int64(millis),
            reason: _reason,
        } = issue
        {
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(*millis) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::I32,
            found: Bson::String(text),
            reason: _reason,
        } = issue
        {
            let Ok(number) = text.parse::<i32>() else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::Int32(number));
        } else if let Issue::Unknown {
            path,
            found: _found,
        } = issue
        {
            path.remove_from_document(row);
        } else {
            return Verdict::Reject;
        }
    }
    Verdict::Fixed
}

fn reject(_row: &mut Document, _issues: &[record_schema::Issue<Bson>]) -> record_schema::Verdict {
    record_schema::Verdict::Reject
}

/// Reads `stored_row` with `decide`, and returns the read, what `decide` was handed less each
/// `reason`, and how many times it ran.
fn read_with(
    stored_row: Document,
    decide: fn(&mut Document, &[record_schema::Issue<Bson>]) -> record_schema::Verdict,
) -> (
    Result<Record, record_schema::Unrecovered<Bson>>,
    Vec<Told>,
    u32,
) {
    let mut seen: Vec<Told> = Vec::new();
    let mut calls = 0_u32;
    let read = Record::from_bson_with(stored_row, |raw, found| {
        calls += 1;
        seen = told!(record_schema, found);
        decide(raw, found)
    });
    (read, seen, calls)
}

/// Bson 1: the happy path, written by serde itself.
#[test]
fn bson_1_what_serde_wrote_is_read_and_the_decider_never_runs() {
    let stored_row = written(&record(&[1_i32, 2_i32]));
    assert_eq!(
        stored_row,
        doc! {
            "createdAt": "2025-10-04T17:46:40Z",
            "recordId": oid("6a7cc592ca0574e6efdfe217"),
            "name": "Loan",
            "versions": [{ "number": 1_i32 }, { "number": 2_i32 }],
        }
    );
    let (read, seen, calls) = read_with(stored_row, reject);
    assert_eq!(read, Ok(record(&[1_i32, 2_i32])));
    assert_eq!(seen, Vec::<Told>::new());
    assert_eq!(calls, 0);
}

/// Bson 2: an id stored as its hex string where an `ObjectId` belongs. serde reads it, and a
/// MongoDB query for the id does not match it.
#[test]
fn bson_2_an_id_stored_as_text_is_mistyped_and_fixed() {
    let mut stored_row = plain_row();
    stored_row.insert("recordId", "6a7cc592ca0574e6efdfe217");
    assert!(serde_reads::<Record>(&stored_row));
    let (read, seen, calls) = read_with(stored_row, repair_record);
    assert_eq!(
        seen,
        [mistyped(
            "recordId",
            "ObjectId",
            string("6a7cc592ca0574e6efdfe217")
        )]
    );
    assert_eq!(read, Ok(record(&[1_i32])));
    assert_eq!(calls, 1);
}

/// Bson 3: a date stored as a BSON date, and one stored as epoch milliseconds.
#[test]
fn bson_3_a_date_stored_as_a_bson_date_or_as_an_epoch_is_invalid_and_fixed() {
    for date in [
        Bson::DateTime(bson::DateTime::from_millis(1_759_600_000_000)),
        Bson::Int64(1_759_600_000_000),
    ] {
        let mut stored_row = plain_row();
        stored_row.insert("createdAt", date.clone());
        assert!(!serde_reads::<Record>(&stored_row));
        let (read, seen, calls) = read_with(stored_row, repair_record);
        assert_eq!(seen, [invalid("createdAt", "DateTime", date)]);
        assert_eq!(read, Ok(record(&[1_i32])));
        assert_eq!(calls, 1);
    }
}

/// Bson 4: a malformed id the callback refuses.
#[test]
fn bson_4_a_malformed_id_is_rejected_with_the_list() {
    let mut stored_row = plain_row();
    stored_row.insert("recordId", "not-an-id");
    let (read, seen, calls) = read_with(stored_row, reject);
    let listed = [invalid("recordId", "ObjectId", string("not-an-id"))];
    assert_eq!(seen, listed);
    assert_eq!(told!(record_schema, read.unwrap_err().issues), listed);
    assert_eq!(calls, 1);
}

/// The `reason` of bson 4's issue is the library's own wording of serde's refusal, which version 3
/// writes inside a sentence of its own.
#[test]
fn a_reason_is_the_wording_of_the_bson_version_built_against() {
    let mut stored_row = plain_row();
    stored_row.insert("recordId", "not-an-id");
    let (read, _seen, _calls) = read_with(stored_row, reject);
    let wording = if built_against_version_2() {
        "invalid value: string \"not-an-id\", expected 24-character, big-endian hex string"
    } else {
        "BSON error. Kind: A deserialization-related error occurred. Message: invalid value: \
         string \"not-an-id\", expected 24-character, big-endian hex string."
    };
    assert_eq!(reasons!(record_schema, read.unwrap_err().issues), [wording]);
}

/// Bson 5: a number stored as a 64-bit integer where the field writes a 32-bit one. Numbers are
/// one bracket, so a query for the number matches it.
#[test]
fn bson_5_a_number_stored_as_another_number_type_is_no_issue() {
    let mut stored_row = plain_row();
    stored_row.insert("versions", vec![doc! { "number": 2_i64 }]);
    let (read, seen, calls) = read_with(stored_row, reject);
    assert_eq!(read, Ok(record(&[2_i32])));
    assert_eq!(seen, Vec::<Told>::new());
    assert_eq!(calls, 0);
}

/// A field serde writes wherever its predicate lets it be and never reads has its key in what the
/// type wrote, which is a key the type declares.
#[test]
fn a_key_the_type_writes_under_a_predicate_and_does_not_read_is_declared() {
    let kept_row = written(&Kept {
        id: "i".to_owned(),
        note: Some("n".to_owned()),
    });
    assert_eq!(kept_row, doc! { "id": "i", "note": "n" });
    let mut calls = 0_u32;
    let kept = Kept::from_bson_with(kept_row, |_raw, _found| {
        calls += 1;
        kept_schema::Verdict::Reject
    });
    assert_eq!(
        kept,
        Ok(Kept {
            id: "i".to_owned(),
            note: None,
        })
    );

    let text_row = written(&KeptText {
        id: "i".to_owned(),
        note: "n".to_owned(),
    });
    assert_eq!(text_row, doc! { "id": "i", "note": "n" });
    let text = KeptText::from_bson_with(text_row, |_raw, _found| {
        calls += 1;
        kept_text_schema::Verdict::Reject
    });
    assert_eq!(
        text,
        Ok(KeptText {
            id: "i".to_owned(),
            note: String::new(),
        })
    );
    assert_eq!(calls, 0);
}

/// A field serde neither writes nor reads declares no key, under `skip` or under
/// `skip_serializing` beside `skip_deserializing`: its key in a row is one serde reads past.
#[test]
fn a_key_of_a_field_the_type_neither_writes_nor_reads_is_unknown() {
    let stored_row = doc! { "cached": 1_i32, "id": "i", "local": 9_i32 };
    assert!(serde_reads::<Shelved>(&stored_row));
    let read = Shelved::from_bson_with(stored_row, |_raw, _found| shelved_schema::Verdict::Reject);
    assert_eq!(
        told!(shelved_schema, read.unwrap_err().issues),
        [
            unknown("cached", Bson::Int32(1_i32)),
            unknown("local", Bson::Int32(9_i32))
        ]
    );
}

/// Bson 6: a MongoDB row with the `_id` every row carries, read into a type that does not declare
/// `_id`.
#[test]
fn bson_6_a_rows_own_id_is_a_key_like_any_other() {
    let mut stored_row = plain_row();
    stored_row.insert("_id", oid("6a7cc592ca0574e6efdfe299"));
    assert!(serde_reads::<Record>(&stored_row));
    let (read, seen, calls) = read_with(stored_row, reject);
    assert_eq!(
        seen,
        [unknown(
            "_id",
            Bson::ObjectId(oid("6a7cc592ca0574e6efdfe299"))
        )]
    );
    assert_eq!(
        read,
        Err(record_schema::Unrecovered {
            issues: vec![record_schema::Issue::Unknown {
                path: record_schema::Path(vec![record_schema::Segment::Key("_id".to_owned())]),
                found: Bson::ObjectId(oid("6a7cc592ca0574e6efdfe299")),
            }],
        })
    );
    assert_eq!(calls, 1);
}

/// Bson 7: the decider, on a row with problems in the record, in one of its versions, and in the
/// row's own `_id`, which the decider can read from the row it is handed.
#[test]
fn bson_7_the_decider_repairs_a_row_with_problems_at_three_levels() {
    let stored_row = doc! {
        "_id": oid("6a7cc592ca0574e6efdfe299"),
        "recordId": "6a7cc592ca0574e6efdfe217",
        "name": "Loan",
        "createdAt": Bson::DateTime(bson::DateTime::from_millis(1_759_600_000_000)),
        "versions": [{ "number": 1_i32 }, { "number": "2", "draft": true }],
    };
    let mut seen: Vec<Told> = Vec::new();
    let mut calls = 0_u32;
    let read = Record::from_bson_with(stored_row, |raw, found| {
        calls += 1;
        seen = told!(record_schema, found);
        assert_eq!(
            raw.get_object_id("_id").ok(),
            Some(oid("6a7cc592ca0574e6efdfe299"))
        );
        repair_record(raw, found)
    });
    assert_eq!(
        seen,
        [
            invalid(
                "createdAt",
                "DateTime",
                Bson::DateTime(bson::DateTime::from_millis(1_759_600_000_000))
            ),
            mistyped("recordId", "ObjectId", string("6a7cc592ca0574e6efdfe217")),
            invalid("versions[1].number", "I32", string("2")),
            unknown("versions[1].draft", Bson::Boolean(true)),
            unknown("_id", Bson::ObjectId(oid("6a7cc592ca0574e6efdfe299"))),
        ]
    );
    assert_eq!(read, Ok(record(&[1_i32, 2_i32])));
    assert_eq!(calls, 1);
}

/// One chance: the decider fixes the id and not the number, so the read fails with what the second
/// walk finds, and the decider is not asked again.
#[test]
fn a_fixed_row_gets_one_more_read_and_fails_with_the_second_list() {
    fn ids_only(
        raw: &mut Document,
        found: &[record_schema::Issue<Bson>],
    ) -> record_schema::Verdict {
        for issue in found {
            if let record_schema::Issue::Mistyped {
                path,
                expected: _expected,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            }
        }
        record_schema::Verdict::Fixed
    }

    let mut stored_row = plain_row();
    stored_row.insert("recordId", "6a7cc592ca0574e6efdfe217");
    stored_row.insert("versions", vec![doc! { "number": "two" }]);
    let (read, seen, calls) = read_with(stored_row, ids_only);
    assert_eq!(seen.len(), 2);
    assert_eq!(
        told!(record_schema, read.unwrap_err().issues),
        [invalid("versions[0].number", "I32", string("two"))]
    );
    assert_eq!(calls, 1);
}

/// A row serde reads is not read while the walk still lists an issue in it: `Fixed` over a row
/// left as it was fails the read with the same list.
#[test]
fn a_row_serde_reads_still_fails_after_fixed_while_an_issue_is_left() {
    fn untouched(
        _raw: &mut Document,
        _found: &[record_schema::Issue<Bson>],
    ) -> record_schema::Verdict {
        record_schema::Verdict::Fixed
    }

    let mut stored_row = plain_row();
    stored_row.insert("recordId", "6a7cc592ca0574e6efdfe217");
    assert!(serde_reads::<Record>(&stored_row));
    let (read, seen, calls) = read_with(stored_row, untouched);
    assert_eq!(
        seen,
        [mistyped(
            "recordId",
            "ObjectId",
            string("6a7cc592ca0574e6efdfe217")
        )]
    );
    assert_eq!(told!(record_schema, read.unwrap_err().issues), seen);
    assert_eq!(calls, 1);
}

/// MongoDB's comparison rule: numbers are one bracket, strings and symbols one, and every other
/// type a bracket of its own.
#[test]
fn a_stored_type_is_matched_by_the_types_in_its_own_bracket() {
    use version_schema::same_bracket;

    let numbers = [
        Bson::Int32(1),
        Bson::Int64(1),
        Bson::Double(1.0),
        Bson::Decimal128(bson::Decimal128::from_bytes([0_u8; 16])),
    ];
    let texts = [string("a"), Bson::Symbol("a".to_owned())];
    let alone = [
        Bson::Boolean(true),
        Bson::Null,
        Bson::ObjectId(oid("6a7cc592ca0574e6efdfe217")),
        Bson::DateTime(bson::DateTime::from_millis(0)),
        Bson::Array(Vec::new()),
        Bson::Document(Document::new()),
    ];
    for bracket in [numbers.as_slice(), texts.as_slice()] {
        for stored_as in bracket {
            for written_as in bracket {
                assert!(same_bracket(stored_as, written_as));
            }
        }
    }
    for number in &numbers {
        for held in &texts {
            assert!(!same_bracket(number, held));
            assert!(!same_bracket(held, number));
        }
    }
    for (stored_at, stored_as) in alone.iter().enumerate() {
        for (written_at, written_as) in alone.iter().enumerate() {
            assert_eq!(same_bracket(stored_as, written_as), stored_at == written_at);
        }
        for bracketed in numbers.iter().chain(&texts) {
            assert!(!same_bracket(stored_as, bracketed));
            assert!(!same_bracket(bracketed, stored_as));
        }
    }
}

/// serde reads a struct from a list of its fields in order, which is not the document the struct
/// writes, and refuses one stored as text.
#[test]
fn a_model_stored_as_another_type_is_mistyped_and_one_serde_refuses_is_invalid() {
    let mut stored_row = plain_row();
    stored_row.insert(
        "versions",
        vec![Bson::Array(vec![Bson::Int32(3)]), string("three")],
    );
    let (_read, seen, calls) = read_with(stored_row, reject);
    assert_eq!(
        seen,
        [
            mistyped(
                "versions[0]",
                "Model(\"Version\")",
                Bson::Array(vec![Bson::Int32(3)])
            ),
            invalid("versions[1]", "Model(\"Version\")", string("three")),
        ]
    );
    assert_eq!(calls, 1);
}

/// A key serde needs is `Missing`, and a field stored as something other than the list or the map
/// it is written as is one issue at the field, in this walker's own words.
#[test]
fn a_missing_key_and_a_field_stored_as_neither_its_list_nor_its_map_are_listed_at_the_field() {
    let mut stored_row = plain_row();
    stored_row.remove("name");
    stored_row.insert("versions", doc! { "number": 1_i32 });
    let (read, seen, _calls) = read_with(stored_row, reject);
    assert_eq!(
        seen,
        [
            missing("name", "String"),
            invalid(
                "versions",
                "Array(Model(\"Version\"))",
                Bson::Document(doc! { "number": 1_i32 })
            ),
        ]
    );
    assert_eq!(
        reasons!(record_schema, read.unwrap_err().issues),
        ["not an array"]
    );

    let catalog = CatalogByItem::from_bson_with(
        doc! { "counts": [], "owners": [], "tags": "none" },
        |_raw, _found| catalog_by_item_schema::Verdict::Reject,
    );
    let issues = catalog.unwrap_err().issues;
    assert_eq!(
        told!(catalog_by_item_schema, issues),
        [
            invalid("owners", "Map(ObjectId)", Bson::Array(Vec::new())),
            invalid("tags", "Array(ObjectId)", string("none")),
        ]
    );
    assert_eq!(
        reasons!(catalog_by_item_schema, issues),
        ["not an object", "not an array"]
    );
}

/// Each id and number is read on its own: an issue sits at the item, with the item's own type, and
/// a decider that sets each path repairs the row.
#[test]
fn a_plain_value_in_a_list_a_map_or_an_option_is_listed_and_fixed_at_its_own_path() {
    let stored_row = doc! {
        "tags": [oid("6a7cc592ca0574e6efdfe217"), "6a7cc592ca0574e6efdfe218"],
        "owners": { "alice": "6a7cc592ca0574e6efdfe219" },
        "parent": "6a7cc592ca0574e6efdfe21a",
        "counts": [1_i32, "2"],
    };
    let mut seen: Vec<Told> = Vec::new();
    let read = CatalogByItem::from_bson_with(stored_row, |raw, found| {
        use catalog_by_item_schema::{Issue, Verdict};

        seen = told!(catalog_by_item_schema, found);
        for issue in found {
            if let Issue::Mistyped {
                path,
                expected: _expected,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            } else if let Issue::Invalid {
                path,
                expected: _expected,
                found: Bson::String(written_as),
                reason: _reason,
            } = issue
            {
                let Ok(number) = written_as.parse::<i32>() else {
                    return Verdict::Reject;
                };
                path.set_in_document(raw, Bson::Int32(number));
            } else {
                return Verdict::Reject;
            }
        }
        Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            invalid("counts[1]", "I32", string("2")),
            mistyped(
                "owners.alice",
                "ObjectId",
                string("6a7cc592ca0574e6efdfe219")
            ),
            mistyped(
                "parent",
                "Optional(ObjectId)",
                string("6a7cc592ca0574e6efdfe21a")
            ),
            mistyped("tags[1]", "ObjectId", string("6a7cc592ca0574e6efdfe218")),
        ]
    );
    assert_eq!(
        read,
        Ok(CatalogByItem {
            counts: vec![1_i32, 2_i32],
            owners: HashMap::from([("alice".to_owned(), oid("6a7cc592ca0574e6efdfe219"))]),
            parent: Some(oid("6a7cc592ca0574e6efdfe21a")),
            tags: vec![
                oid("6a7cc592ca0574e6efdfe217"),
                oid("6a7cc592ca0574e6efdfe218")
            ],
        })
    );
}

/// Each read hook is the function serde's derive calls, handed the library's deserializer, so its
/// refusal is the walker's: the author's function, the author's module, a function generic over
/// what it reads, and the module `as_number` hangs.
#[test]
fn a_hooked_field_is_read_through_its_hook() {
    let date = Bson::DateTime(bson::DateTime::from_millis(1_759_600_000_000));
    let stored_row = doc! {
        "code": "ab",
        "count": 5_i32,
        "name": "Loan",
        "port": 80_i32,
        "seenAt": date.clone(),
    };
    let read = Hooked::from_bson_with(stored_row, |_raw, _found| hooked_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(hooked_schema, issues),
        [
            invalid("code", "String", string("ab")),
            invalid("count", "U32", Bson::Int32(5)),
            invalid("port", "U16", Bson::Int32(80)),
            invalid("seenAt", "DateTime", date),
        ]
    );
    let refusals = [
        "\"ab\" is not upper case",
        "invalid type: integer `5`, expected a string",
        "invalid type: integer `80`, expected a string",
        "expected a unix timestamp in milliseconds",
    ];
    let reasons = reasons!(hooked_schema, issues);
    assert_eq!(reasons.len(), refusals.len());
    for (reason, refusal) in reasons.iter().zip(refusals) {
        assert!(reason.contains(refusal), "for {refusal}, got: {reason}");
    }
}

/// A field written back through its own hook is compared in the type that hook writes: text for
/// the two numbers, and a number for the date.
#[test]
fn a_hooked_field_stored_as_its_hook_writes_it_is_no_issue() {
    let stored_row = doc! {
        "code": "AB",
        "count": "5",
        "name": "Loan",
        "port": "80",
        "seenAt": 1_759_600_000_000_i64,
    };
    assert_eq!(written(&hooked()), stored_row);
    let mut calls = 0_u32;
    let read = Hooked::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        hooked_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(hooked()));
    assert_eq!(calls, 0);
}

/// A constraint on a struct's field hangs no read hook: serde reads the value, and the walker
/// holds what it read to the bound, where a schema surface is on to publish its validator.
#[test]
fn a_constraint_on_a_struct_field_is_the_walkers_to_check() {
    let mut stored_row = written(&hooked());
    stored_row.insert("name", "Lo");
    let mut calls = 0_u32;
    let read = Hooked::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        hooked_schema::Verdict::Reject
    });
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        let refused = read.unwrap_err();
        assert_eq!(
            told!(hooked_schema, refused.issues),
            vec![invalid("name", "String", Bson::String("Lo".to_owned()))]
        );
        assert!(
            refused
                .to_string()
                .ends_with("too short: minimum length is 3, got 2"),
            "got: {refused}"
        );
        assert_eq!(calls, 1);
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(read.unwrap().name, "Lo");
        assert_eq!(calls, 0);
    }
}

#[test]
fn a_read_hook_named_like_the_value_a_key_holds_is_the_authors_function() {
    let mut calls = 0_u32;
    let read = Named::from_bson_with(doc! { "name": "x" }, |_raw, _found| {
        calls += 1;
        named_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Named {
            name: "x".to_owned()
        })
    );
    assert_eq!(calls, 0);
}

/// `key` and `stored` are read under their aliases, and `item` as the text its writer writes.
#[test]
fn a_hook_named_like_any_value_a_fields_walker_binds_is_the_authors_function() {
    let stored_row = doc! {
        "found": "a",
        "held": "b",
        "issue": "c",
        "item": "5",
        "id": "d",
        "object": "e",
        "out": "f",
        "path": "g",
        "saved": "h",
    };
    let mut calls = 0_u32;
    let read = Bound::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        bound_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(bound()));
    assert_eq!(calls, 0);
}

/// A `String` reads an empty text, so each refusal here is the hook's own.
#[test]
fn a_value_a_hook_named_like_a_binding_refuses_is_invalid_at_its_field() {
    let stored_row = doc! {
        "found": "",
        "held": "",
        "issue": "",
        "item": "5",
        "id": "",
        "object": "",
        "out": "",
        "path": "",
        "saved": "",
    };
    let read = Bound::from_bson_with(stored_row, |_raw, _found| bound_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(bound_schema, issues),
        [
            "found", "held", "issue", "id", "object", "out", "path", "saved"
        ]
        .map(|at| invalid(at, "String", string("")))
    );
}

/// `item` reads a count stored as a number, which `read` writes back as text.
#[test]
fn a_value_stored_as_another_type_than_a_writer_named_like_a_binding_writes_is_mistyped() {
    let mut stored_row = written(&bound());
    stored_row.insert("item", 5_i32);
    let read = Bound::from_bson_with(stored_row, |_raw, _found| bound_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(bound_schema, issues),
        [mistyped("item", "U32", Bson::Int32(5))]
    );
}

#[test]
fn a_hook_named_like_a_value_an_enum_walker_binds_reads_a_variants_field() {
    let stored_row = doc! {
        "kind": "Note",
        "body": { "content": "a", "inner": "b", "tag": "c" },
    };
    let mut calls = 0_u32;
    let read = Adjacent::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        adjacent_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Adjacent::Note {
            content: "a".to_owned(),
            inner: "b".to_owned(),
            tag: "c".to_owned(),
        })
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_variants_field_a_hook_named_like_a_binding_refuses_is_invalid_at_the_field() {
    let stored_row = doc! {
        "kind": "Note",
        "body": { "content": "", "inner": "", "tag": "" },
    };
    let read =
        Adjacent::from_bson_with(stored_row, |_raw, _found| adjacent_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(adjacent_schema, issues),
        ["body.content", "body.inner", "body.tag"].map(|at| invalid(at, "String", string("")))
    );
}

/// A key stored under an alias is its field, and an issue under it is listed at the alias, so a
/// decider that sets the path fixes the key the row holds.
#[test]
fn a_key_stored_under_an_alias_is_its_field_and_an_issue_is_listed_at_the_alias() {
    for key in ["name", "fullName", "label"] {
        let mut stored_row = Document::new();
        stored_row.insert(key, "Ada");
        let mut calls = 0_u32;
        let read = Aliased::from_bson_with(stored_row, |_raw, _found| {
            calls += 1;
            aliased_schema::Verdict::Reject
        });
        assert_eq!(
            read,
            Ok(Aliased {
                latest: None,
                name: "Ada".to_owned()
            })
        );
        assert_eq!(calls, 0);
    }

    let stored_row = doc! { "fullName": 7_i32, "newest": { "number": "1" } };
    let mut seen: Vec<Told> = Vec::new();
    let read = Aliased::from_bson_with(stored_row, |raw, found| {
        seen = told!(aliased_schema, found);
        for issue in found {
            if let aliased_schema::Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                let fixed = if *expected == aliased_schema::Expected::I32 {
                    Bson::Int32(1)
                } else {
                    string("Ada")
                };
                path.set_in_document(raw, fixed);
            }
        }
        aliased_schema::Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            invalid("newest.number", "I32", string("1")),
            invalid("fullName", "String", Bson::Int32(7)),
        ]
    );
    assert_eq!(
        read,
        Ok(Aliased {
            latest: Some(Version { number: 1 }),
            name: "Ada".to_owned()
        })
    );
}

#[test]
fn a_model_type_is_walked_through_a_map_an_option_and_a_box() {
    let stored_row = doc! {
        "boxed": { "number": "3" },
        "byName": { "first": { "number": "1" } },
        "byTeam": { "core": [{ "number": 1_i32 }, { "number": "2" }] },
        "latest": { "draft": true, "number": 2_i32 },
    };
    let read = Bundle::from_bson_with(stored_row, |_raw, _found| bundle_schema::Verdict::Reject);
    assert_eq!(
        told!(bundle_schema, read.unwrap_err().issues),
        [
            invalid("boxed.number", "I32", string("3")),
            invalid("byName.first.number", "I32", string("1")),
            invalid("byTeam.core[1].number", "I32", string("2")),
            unknown("latest.draft", Bson::Boolean(true)),
        ]
    );
}

/// An optional model type is walked when its key is there and does not hold `null`.
#[test]
fn an_optional_model_type_that_is_absent_or_null_is_no_issue() {
    for latest in [
        None,
        Some(Bson::Null),
        Some(Bson::Document(doc! { "number": 2_i32 })),
    ] {
        let mut stored_row = doc! { "boxed": { "number": 3_i32 }, "byName": {}, "byTeam": {} };
        if let Some(held) = latest {
            stored_row.insert("latest", held);
        }
        let mut calls = 0_u32;
        let read = Bundle::from_bson_with(stored_row, |_raw, _found| {
            calls += 1;
            bundle_schema::Verdict::Reject
        });
        read.unwrap();
        assert_eq!(calls, 0);
    }
}

/// A plain value inside a list of lists, an optional list or a map is read on its own, at its own
/// path, and an optional list stored as neither a list nor `null` names the `Option`.
#[test]
fn a_plain_value_in_a_nested_or_optional_list_or_a_map_is_listed_at_its_own_path() {
    let stored_row = doc! {
        "grid": [[1_i32, "x"], 7_i32],
        "marks": [true],
        "scores": { "ada": "9" },
    };
    let read = Tally::from_bson_with(stored_row, |_raw, _found| tally_schema::Verdict::Reject);
    assert_eq!(
        told!(tally_schema, read.unwrap_err().issues),
        [
            invalid("grid[0][1]", "I32", string("x")),
            invalid("grid[1]", "Array(I32)", Bson::Int32(7)),
            invalid("marks[0]", "I32", Bson::Boolean(true)),
            invalid("scores.ada", "I32", string("9")),
        ]
    );

    let unlisted = Tally::from_bson_with(
        doc! { "grid": [], "marks": "none", "scores": {} },
        |_raw, _found| tally_schema::Verdict::Reject,
    );
    assert_eq!(
        told!(tally_schema, unlisted.unwrap_err().issues),
        [invalid("marks", "Optional(Array(I32))", string("none"))]
    );

    let mut calls = 0_u32;
    let absent = Tally::from_bson_with(
        doc! { "grid": [], "marks": Bson::Null, "scores": {} },
        |_raw, _found| {
            calls += 1;
            tally_schema::Verdict::Reject
        },
    );
    assert_eq!(
        absent,
        Ok(Tally {
            grid: Vec::new(),
            marks: None,
            scores: HashMap::new(),
        })
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_struct_with_no_field_to_read_still_lists_an_undeclared_key() {
    let read = Derived::from_bson_with(written(&Derived { total: 7 }), |_raw, _found| {
        derived_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(Derived { total: 0 }));
    let keyed = Derived::from_bson_with(doc! { "extra": true }, |_raw, _found| {
        derived_schema::Verdict::Reject
    });
    assert_eq!(
        told!(derived_schema, keyed.unwrap_err().issues),
        [unknown("extra", Bson::Boolean(true))]
    );
}

/// serde refuses the row and the walk has nothing to say why: one `Undescribed`, carrying serde's
/// message as the library words it.
#[test]
fn a_refusal_the_walk_cannot_see_into_is_undescribed() {
    let read = Envelope::from_bson_with(doc! { "sealed": { "label": "x" } }, |_raw, _found| {
        envelope_schema::Verdict::Reject
    });
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(envelope_schema, issues),
        [("Undescribed", String::new(), String::new(), None)]
    );
    let reasons = reasons!(envelope_schema, issues);
    assert_eq!(reasons.len(), 1);
    assert!(
        reasons
            .iter()
            .all(|reason| reason.contains("sealed: nothing reads this")),
        "got: {reasons:?}"
    );
}

/// `Path` puts a value at the place an issue names inside a document, and takes one away from it.
#[test]
fn a_path_sets_and_removes_inside_a_bson_document() {
    use record_schema::{Path, Segment};

    let key = |name: &str| Segment::Key(name.to_owned());
    let number = Path(vec![key("versions"), Segment::Index(1), key("number")]);
    let mut raw = doc! { "versions": [{ "number": 1_i32 }, {}] };
    assert!(number.set_in_document(&mut raw, Bson::Int32(2)));
    assert_eq!(
        raw,
        doc! { "versions": [{ "number": 1_i32 }, { "number": 2_i32 }] }
    );
    assert!(number.set_in_document(&mut raw, Bson::Int32(3)));
    assert_eq!(
        raw,
        doc! { "versions": [{ "number": 1_i32 }, { "number": 3_i32 }] }
    );
    assert!(number.remove_from_document(&mut raw));
    assert!(!number.remove_from_document(&mut raw));

    let third = Path(vec![key("versions"), Segment::Index(2)]);
    assert!(!third.set_in_document(&mut raw, Bson::Null));
    assert!(!third.remove_from_document(&mut raw));
    let first = Path(vec![key("versions"), Segment::Index(0)]);
    assert!(first.set_in_document(&mut raw, Bson::Null));
    assert_eq!(raw, doc! { "versions": [Bson::Null, {}] });
    assert!(first.remove_from_document(&mut raw));
    assert_eq!(raw, doc! { "versions": [{}] });

    let name = Path(vec![key("name")]);
    assert!(name.set_in_document(&mut raw, string("Loan")));
    assert!(name.set_in_document(&mut raw, string("Other")));
    assert_eq!(raw, doc! { "versions": [{}], "name": "Other" });

    // A key is looked up in a document and a position in a list, and neither under a plain value.
    for nowhere in [
        Path(vec![key("absent"), key("inner")]),
        Path(vec![key("name"), key("inner")]),
        Path(vec![key("versions"), key("inner")]),
        Path(vec![key("versions"), Segment::Index(0), Segment::Index(0)]),
        Path(vec![Segment::Index(0)]),
        Path::default(),
    ] {
        assert!(!nowhere.set_in_document(&mut raw, Bson::Null));
        assert!(!nowhere.remove_from_document(&mut raw));
    }

    assert!(name.remove_from_document(&mut raw));
    assert!(!name.remove_from_document(&mut raw));
    assert_eq!(raw, doc! { "versions": [{}] });
}

/// `?` carries a failed read into a caller's own error type, and it displays one line per issue.
#[test]
fn unrecovered_is_an_error_a_caller_can_propagate() {
    fn read(stored_row: Document) -> Result<Record, Box<dyn Error>> {
        Ok(Record::from_bson_with(stored_row, reject)?)
    }

    let mut stored_row = plain_row();
    stored_row.remove("name");
    stored_row.insert("legacyField", true);
    let failure = read(stored_row).unwrap_err();
    assert_eq!(
        failure.to_string(),
        "name: missing: expected String\nlegacyField: unknown: found Boolean(true)"
    );
    assert!(failure.source().is_none());
}

/// A tuple struct and a tuple are each the list serde writes them as, walked position by position.
#[test]
fn a_tuple_struct_and_a_tuple_are_walked_by_position() {
    let placement = Placement {
        pair: Pair("p".to_owned(), 3),
        spot: ("s".to_owned(), 4),
        spots: vec![("t".to_owned(), 5)],
    };
    let mut calls = 0_u32;
    let read = Placement::from_bson_with(written(&placement), |_raw, _found| {
        calls += 1;
        placement_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(placement));
    assert_eq!(calls, 0);

    let stored_row = doc! {
        "pair": ["a", "x"],
        "spot": [5_i32, 1_i32],
        "spots": [["t", 5_i32], [6_i32, 7_i32]],
    };
    assert!(!serde_reads::<Placement>(&stored_row));
    let listed =
        Placement::from_bson_with(stored_row, |_raw, _found| placement_schema::Verdict::Reject);
    assert_eq!(
        told!(placement_schema, listed.unwrap_err().issues),
        [
            invalid("pair[1]", "U32", string("x")),
            invalid("spot[0]", "String", Bson::Int32(5)),
            invalid("spots[1][0]", "String", Bson::Int32(6)),
        ]
    );
}

#[test]
fn a_position_that_is_absent_is_missing() {
    let stored_row = doc! { "pair": ["a"], "spot": [], "spots": [["t"]] };
    let read =
        Placement::from_bson_with(stored_row, |_raw, _found| placement_schema::Verdict::Reject);
    assert_eq!(
        told!(placement_schema, read.unwrap_err().issues),
        [
            missing("pair[1]", "U32"),
            missing("spot[0]", "String"),
            missing("spot[1]", "U32"),
            missing("spots[0][1]", "U32"),
        ]
    );
}

/// serde reads past a position the type does not declare in a BSON list, so the position is the
/// only issue, and a decider that removes it repairs the row.
#[test]
fn a_position_the_type_does_not_declare_is_unknown_and_serde_reads_past_it() {
    let stored_row = doc! { "pair": ["a", 1_i32, true], "spot": ["b", 2_i32], "spots": [] };
    assert!(serde_reads::<Placement>(&stored_row));
    let mut seen: Vec<Told> = Vec::new();
    let read = Placement::from_bson_with(stored_row, |raw, found| {
        seen = told!(placement_schema, found);
        for issue in found {
            if let placement_schema::Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_document(raw);
            }
        }
        placement_schema::Verdict::Fixed
    });
    assert_eq!(seen, [unknown("pair[2]", Bson::Boolean(true))]);
    assert_eq!(
        read,
        Ok(Placement {
            pair: Pair("a".to_owned(), 1),
            spot: ("b".to_owned(), 2),
            spots: Vec::new(),
        })
    );
}

/// A tuple stored as no list is one issue at the field in this walker's own words, and a tuple
/// struct stored as one is read whole with its own reader.
#[test]
fn a_tuple_and_a_tuple_struct_stored_as_no_list_are_listed_at_the_value() {
    let stored_row = doc! { "pair": { "0": "a" }, "spot": "none", "spots": [7_i32] };
    let read =
        Placement::from_bson_with(stored_row, |_raw, _found| placement_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(placement_schema, issues),
        [
            invalid("pair", "Model(\"Pair\")", Bson::Document(doc! { "0": "a" })),
            invalid("spot", "Tuple([String, U32])", string("none")),
            invalid("spots[0]", "Tuple([String, U32])", Bson::Int32(7)),
        ]
    );
    assert_eq!(
        reasons!(placement_schema, issues).get(1..),
        Some(["not an array", "not an array"].as_slice())
    );
}

/// A brand and a single-slot tuple struct are each read as the one value they hold, a brand over
/// a model type walks as that type, and a unit struct is a document in which no key is its own.
#[test]
fn a_brand_a_single_slot_struct_and_a_unit_struct_are_walked_as_serde_writes_them() {
    let labelled = Labelled {
        owner: OwnerId(oid("6a7cc592ca0574e6efdfe217")),
        ping: Ping,
        pinned: Pinned(Version { number: 3 }),
        wrapper: Wrapper("w".to_owned()),
    };
    assert_eq!(
        written(&labelled),
        doc! {
            "owner": oid("6a7cc592ca0574e6efdfe217"),
            "ping": {},
            "pinned": { "number": 3_i32 },
            "wrapper": "w",
        }
    );
    let stored_row = doc! {
        "owner": "6a7cc592ca0574e6efdfe217",
        "ping": { "x": 1_i32 },
        "pinned": { "number": "3", "draft": true },
        "wrapper": 5_i32,
    };
    let mut seen: Vec<Told> = Vec::new();
    let read = Labelled::from_bson_with(stored_row, |raw, found| {
        use labelled_schema::{Expected, Issue, Verdict};

        seen = told!(labelled_schema, found);
        for issue in found {
            if let Issue::Mistyped {
                path,
                expected: Expected::ObjectId,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            } else if let Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                let fixed = if *expected == Expected::I32 {
                    Bson::Int32(3)
                } else {
                    string("w")
                };
                path.set_in_document(raw, fixed);
            } else if let Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_document(raw);
            } else {
                return Verdict::Reject;
            }
        }
        Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            mistyped("owner", "ObjectId", string("6a7cc592ca0574e6efdfe217")),
            unknown("ping.x", Bson::Int32(1)),
            invalid("pinned.number", "I32", string("3")),
            unknown("pinned.draft", Bson::Boolean(true)),
            invalid("wrapper", "String", Bson::Int32(5)),
        ]
    );
    assert_eq!(read, Ok(labelled));
}

/// A unit struct and a brand over a model type read a document of their own, and a unit struct
/// stored as text is read whole with its own reader.
#[test]
fn a_unit_struct_and_a_brand_over_a_model_type_are_read_from_a_document() {
    let mut calls = 0_u32;
    let ping = Ping::from_bson_with(Document::new(), |_raw, _found| {
        calls += 1;
        ping_schema::Verdict::Reject
    });
    assert_eq!(ping, Ok(Ping));
    let pinned = Pinned::from_bson_with(doc! { "number": 3_i32 }, |_raw, _found| {
        calls += 1;
        pinned_schema::Verdict::Reject
    });
    assert_eq!(pinned, Ok(Pinned(Version { number: 3 })));
    assert_eq!(calls, 0);

    let keyed = Ping::from_bson_with(doc! { "x": 1_i32 }, |_raw, _found| {
        ping_schema::Verdict::Reject
    });
    assert_eq!(
        told!(ping_schema, keyed.unwrap_err().issues),
        [unknown("x", Bson::Int32(1))]
    );
    let stored_row = doc! {
        "owner": oid("6a7cc592ca0574e6efdfe217"),
        "ping": "ping",
        "pinned": "three",
        "wrapper": "w",
    };
    let read =
        Labelled::from_bson_with(stored_row, |_raw, _found| labelled_schema::Verdict::Reject);
    assert_eq!(
        told!(labelled_schema, read.unwrap_err().issues),
        [
            invalid("ping", "Model(\"Ping\")", string("ping")),
            invalid("pinned", "Model(\"Version\")", string("three")),
        ]
    );
}

/// What a brand over a model type lists inside a document the caller holds is what that type
/// lists there, under that type's keys, and a unit struct lists nothing and has no key.
#[test]
fn a_brand_over_a_model_type_hands_its_fields_walk_to_that_type() {
    let held = doc! { "legacy": true, "number": "3" };
    let mut out: Vec<pinned_schema::Issue<Bson>> = Vec::new();
    let declared =
        Pinned::decode_with_bson_fields(&held, &[], pinned_schema::issue_from_parts, &mut out);
    assert_eq!(declared, ["number"]);
    assert_eq!(
        told!(pinned_schema, out),
        [invalid("number", "I32", string("3"))]
    );

    let mut unlisted: Vec<ping_schema::Issue<Bson>> = Vec::new();
    let keys =
        Ping::decode_with_bson_fields(&held, &[], ping_schema::issue_from_parts, &mut unlisted);
    assert_eq!(keys, Vec::<&str>::new());
    assert_eq!(unlisted, Vec::new());
}

/// A brand over another brand, over a tuple struct and over a plain enum is the value the type it
/// holds writes: what serde wrote is read without the decider, and an issue inside the held type
/// sits at the path the brand sits at.
#[test]
fn a_brand_over_a_brand_a_tuple_struct_and_a_plain_enum_walks_as_the_type_it_holds() {
    let referenced = Referenced {
        owner: OwnerRef(RecordId("abc".to_owned())),
        pair: PairRef(Pair("a".to_owned(), 1)),
        status: StatusRef(Status::Draft),
    };
    let stored_row = written(&referenced);
    assert_eq!(
        stored_row,
        doc! { "owner": "abc", "pair": ["a", 1_i64], "status": "Draft" }
    );
    let mut calls = 0_u32;
    let read = Referenced::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        referenced_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(referenced));
    assert_eq!(calls, 0);

    let refused_row = doc! { "owner": 5_i32, "pair": ["a", "x"], "status": "Archived" };
    assert!(!serde_reads::<Referenced>(&refused_row));
    let listed = Referenced::from_bson_with(refused_row, |_raw, _found| {
        referenced_schema::Verdict::Reject
    });
    assert_eq!(
        told!(referenced_schema, listed.unwrap_err().issues),
        [
            invalid("owner", "String", Bson::Int32(5)),
            invalid("pair[1]", "U32", string("x")),
            invalid("status", "Model(\"Status\")", string("Archived")),
        ]
    );
}

/// A brand over text, a single-slot struct over text and a tuple struct each list nothing inside
/// a document the caller holds, and no key there is theirs. A brand over one of them answers as
/// it does.
#[test]
fn a_brand_over_text_and_a_tuple_struct_list_nothing_in_a_document_and_return_no_key() {
    let held = doc! { "0": "a", "legacy": true };
    for keyed in [
        RecordId::decode_with_bson_fields::<record_id_schema::Issue<Bson>>,
        Wrapper::decode_with_bson_fields::<record_id_schema::Issue<Bson>>,
        Pair::decode_with_bson_fields::<record_id_schema::Issue<Bson>>,
        OwnerRef::decode_with_bson_fields::<record_id_schema::Issue<Bson>>,
        PairRef::decode_with_bson_fields::<record_id_schema::Issue<Bson>>,
    ] {
        let mut out = Vec::new();
        let declared = keyed(&held, &[], record_id_schema::issue_from_parts, &mut out);
        assert_eq!(declared, Vec::<&str>::new());
        assert_eq!(out, Vec::new());
    }
}

/// serde flattens a struct or a map and nothing else, so it refuses every read of a struct that
/// flattens a brand over text. The brand's fields walker lists nothing for it, and the read
/// carries serde's refusal.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_struct_flattening_a_type_serde_cannot_flatten_lists_nothing_for_it_beside_serdes_refusal() {
    let stored_row = doc! { "name": "n" };
    assert!(!serde_reads::<Carded>(&stored_row));
    let read = Carded::from_bson_with(stored_row, |_raw, _found| carded_schema::Verdict::Reject);
    assert_eq!(
        told!(carded_schema, read.unwrap_err().issues),
        [undescribed()]
    );
}

/// A `transparent` struct with a named field is the value its field holds, at the path the struct
/// sits at: an id stored as text is `Mistyped` there, and over a model type the struct walks as
/// that type. No key is looked up under the field's name.
#[test]
fn a_transparent_struct_with_a_named_field_is_walked_as_the_value_its_field_holds() {
    let assigned = Assigned {
        current: Current {
            version: Version { number: 3 },
        },
        owner: Owner {
            id: oid("6a7cc592ca0574e6efdfe217"),
        },
    };
    assert_eq!(
        written(&assigned),
        doc! {
            "current": { "number": 3_i32 },
            "owner": oid("6a7cc592ca0574e6efdfe217"),
        }
    );
    let mut calls = 0_u32;
    let kept = Assigned::from_bson_with(written(&assigned), |_raw, _found| {
        calls += 1;
        assigned_schema::Verdict::Reject
    });
    assert_eq!(kept.as_ref(), Ok(&assigned));
    assert_eq!(calls, 0);

    let stored_row = doc! {
        "current": { "number": "3", "draft": true },
        "owner": "6a7cc592ca0574e6efdfe217",
    };
    let mut seen: Vec<Told> = Vec::new();
    let read = Assigned::from_bson_with(stored_row, |raw, found| {
        use assigned_schema::{Expected, Issue, Verdict};

        seen = told!(assigned_schema, found);
        for issue in found {
            if let Issue::Mistyped {
                path,
                expected: Expected::ObjectId,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            } else if let Issue::Invalid {
                path,
                expected: Expected::I32,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_document(raw, Bson::Int32(3));
            } else if let Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_document(raw);
            } else {
                return Verdict::Reject;
            }
        }
        Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            invalid("current.number", "I32", string("3")),
            unknown("current.draft", Bson::Boolean(true)),
            mistyped("owner", "ObjectId", string("6a7cc592ca0574e6efdfe217")),
        ]
    );
    assert_eq!(read, Ok(assigned));

    let refused = Assigned::from_bson_with(
        doc! { "current": "three", "owner": { "id": oid("6a7cc592ca0574e6efdfe217") } },
        |_raw, _found| assigned_schema::Verdict::Reject,
    );
    assert_eq!(
        told!(assigned_schema, refused.unwrap_err().issues),
        [
            invalid("current", "Model(\"Version\")", string("three")),
            invalid(
                "owner",
                "ObjectId",
                Bson::Document(doc! { "id": oid("6a7cc592ca0574e6efdfe217") })
            ),
        ]
    );
}

/// Read from a document of its own, a `transparent` struct over a model type lists that type's
/// issues at paths inside that type, and hands its fields walk to that type.
#[test]
fn a_transparent_struct_over_a_model_type_is_read_from_a_document_as_that_type() {
    let mut calls = 0_u32;
    let current = Current::from_bson_with(doc! { "number": 3_i32 }, |_raw, _found| {
        calls += 1;
        current_schema::Verdict::Reject
    });
    assert_eq!(
        current,
        Ok(Current {
            version: Version { number: 3 }
        })
    );
    assert_eq!(calls, 0);

    let refused = Current::from_bson_with(doc! { "number": "3" }, |_raw, _found| {
        current_schema::Verdict::Reject
    });
    assert_eq!(
        told!(current_schema, refused.unwrap_err().issues),
        [invalid("number", "I32", string("3"))]
    );

    let held = doc! { "legacy": true, "number": "3" };
    let mut out: Vec<current_schema::Issue<Bson>> = Vec::new();
    let declared =
        Current::decode_with_bson_fields(&held, &[], current_schema::issue_from_parts, &mut out);
    assert_eq!(declared, ["number"]);
    assert_eq!(
        told!(current_schema, out),
        [invalid("number", "I32", string("3"))]
    );
}

/// A `transparent` tuple struct is the value of the one slot serde reads, whatever other slots it
/// declares, at the path the struct sits at: a list is walked item by item, an id stored as text
/// is `Mistyped` there, and over a model type the struct walks as that type. No position is looked
/// up.
#[test]
fn a_transparent_tuple_struct_is_walked_as_the_value_of_the_slot_serde_reads() {
    let filing = Filing {
        keeper: Keeper(0, oid("6a7cc592ca0574e6efdfe217")),
        stamped: Stamped(Version { number: 3 }, 0),
        tags: Tags(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()], 0),
    };
    let stored = written(&Filing {
        keeper: Keeper(9, oid("6a7cc592ca0574e6efdfe217")),
        stamped: Stamped(Version { number: 3 }, 9),
        tags: Tags(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()], 9),
    });
    assert_eq!(
        stored,
        doc! {
            "keeper": oid("6a7cc592ca0574e6efdfe217"),
            "stamped": { "number": 3_i32 },
            "tags": ["a", "b", "c"],
        }
    );
    assert!(serde_reads::<Filing>(&stored));
    let mut calls = 0_u32;
    let kept = Filing::from_bson_with(stored, |_raw, _found| {
        calls += 1;
        filing_schema::Verdict::Reject
    });
    assert_eq!(kept.as_ref(), Ok(&filing));
    assert_eq!(calls, 0);

    let stored_row = doc! {
        "keeper": "6a7cc592ca0574e6efdfe217",
        "stamped": { "number": "3", "draft": true },
        "tags": ["a", 7_i32, "c"],
    };
    let mut seen: Vec<Told> = Vec::new();
    let read = Filing::from_bson_with(stored_row, |raw, found| {
        use filing_schema::{Expected, Issue, Verdict};

        seen = told!(filing_schema, found);
        for issue in found {
            if let Issue::Mistyped {
                path,
                expected: Expected::ObjectId,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            } else if let Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                let fixed = if *expected == Expected::I32 {
                    Bson::Int32(3)
                } else {
                    string("b")
                };
                path.set_in_document(raw, fixed);
            } else if let Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_document(raw);
            } else {
                return Verdict::Reject;
            }
        }
        Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            mistyped("keeper", "ObjectId", string("6a7cc592ca0574e6efdfe217")),
            invalid("stamped.number", "I32", string("3")),
            unknown("stamped.draft", Bson::Boolean(true)),
            invalid("tags[1]", "String", Bson::Int32(7)),
        ]
    );
    assert_eq!(read, Ok(filing));

    let refused_row = doc! {
        "keeper": [oid("6a7cc592ca0574e6efdfe217")],
        "stamped": "three",
        "tags": 5_i32,
    };
    assert!(!serde_reads::<Filing>(&refused_row));
    let refused =
        Filing::from_bson_with(refused_row, |_raw, _found| filing_schema::Verdict::Reject);
    assert_eq!(
        told!(filing_schema, refused.unwrap_err().issues),
        [
            invalid(
                "keeper",
                "ObjectId",
                Bson::Array(vec![Bson::ObjectId(oid("6a7cc592ca0574e6efdfe217"))])
            ),
            invalid("stamped", "Model(\"Version\")", string("three")),
            invalid("tags", "Array(String)", Bson::Int32(5)),
        ]
    );
}

/// Read from a document of its own, a `transparent` tuple struct over a model type lists that
/// type's issues at paths inside that type, and hands its fields walk to that type.
#[test]
fn a_transparent_tuple_struct_over_a_model_type_is_read_from_a_document_as_that_type() {
    let mut calls = 0_u32;
    let stamped = Stamped::from_bson_with(doc! { "number": 3_i32 }, |_raw, _found| {
        calls += 1;
        stamped_schema::Verdict::Reject
    });
    assert_eq!(stamped, Ok(Stamped(Version { number: 3 }, 0)));
    assert_eq!(calls, 0);

    let refused = Stamped::from_bson_with(doc! { "number": "3" }, |_raw, _found| {
        stamped_schema::Verdict::Reject
    });
    assert_eq!(
        told!(stamped_schema, refused.unwrap_err().issues),
        [invalid("number", "I32", string("3"))]
    );

    let held = doc! { "legacy": true, "number": "3" };
    let mut out: Vec<stamped_schema::Issue<Bson>> = Vec::new();
    let declared =
        Stamped::decode_with_bson_fields(&held, &[], stamped_schema::issue_from_parts, &mut out);
    assert_eq!(declared, ["number"]);
    assert_eq!(
        told!(stamped_schema, out),
        [invalid("number", "I32", string("3"))]
    );
}

/// The brand is read with its own reader, so the check written on it runs and its message is the
/// issue's.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_constrained_brand_is_refused_with_its_own_message() {
    let read = Coded::from_bson_with(doc! { "code": "ab" }, |_raw, _found| {
        coded_schema::Verdict::Reject
    });
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(coded_schema, issues),
        [invalid("code", "String", string("ab"))]
    );
    let reasons = reasons!(coded_schema, issues);
    assert!(
        reasons
            .iter()
            .all(|reason| reason.contains("too short: minimum length is 3, got 2")),
        "got: {reasons:?}"
    );
    assert_eq!(
        Coded::from_bson_with(doc! { "code": "abc" }, |_raw, _found| {
            coded_schema::Verdict::Reject
        }),
        Ok(Coded {
            code: Code("abc".to_owned())
        })
    );
}

/// An issue inside what fills the parameter is one issue at the field, with no path into the
/// item, and a model type filling the parameter needs no flag.
#[test]
fn a_value_of_a_parameters_type_is_read_whole_at_its_field() {
    let items = vec![
        Bson::Document(doc! { "number": 1_i32 }),
        Bson::Document(doc! { "number": "2" }),
    ];
    let stored_row = doc! { "items": items.clone(), "total": "two" };
    let read =
        Page::<Version>::from_bson_with(stored_row, |_raw, _found| page_schema::Verdict::Reject);
    assert_eq!(
        told!(page_schema, read.unwrap_err().issues),
        [
            invalid("items", "Array(TypeParam(\"T\"))", Bson::Array(items)),
            invalid("total", "U32", string("two")),
        ]
    );

    let page = Page {
        items: vec![Plain {
            label: "a".to_owned(),
        }],
        total: 1,
    };
    let mut calls = 0_u32;
    let unflagged = Page::<Plain>::from_bson_with(written(&page), |_raw, _found| {
        calls += 1;
        page_schema::Verdict::Reject
    });
    assert_eq!(unflagged, Ok(page));
    assert_eq!(calls, 0);
}

/// A BSON walker is bound to write a parameter's value back, so an id filling one and stored as
/// text is `Mistyped`, as it is under a brand that names the id outright.
#[test]
fn a_value_of_a_parameters_type_stored_as_another_type_is_mistyped() {
    let stored_row = doc! { "left": "6a7cc592ca0574e6efdfe217", "right": 2_i64 };
    assert!(serde_reads::<Pairing<ObjectId, i32>>(&stored_row));
    let mut seen: Vec<Told> = Vec::new();
    let read = Pairing::<ObjectId, i32>::from_bson_with(stored_row, |raw, found| {
        seen = told!(pairing_schema, found);
        for issue in found {
            if let pairing_schema::Issue::Mistyped {
                path,
                expected: _expected,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            }
        }
        pairing_schema::Verdict::Fixed
    });
    assert_eq!(
        seen,
        [mistyped(
            "left",
            "TypeParam(\"A\")",
            string("6a7cc592ca0574e6efdfe217")
        )]
    );
    assert_eq!(
        read,
        Ok(Pairing {
            left: oid("6a7cc592ca0574e6efdfe217"),
            right: 2_i32,
        })
    );
}

/// What the derive asks of the parameter beyond `Deserialize` is carried by the bound on the type
/// itself, so the methods are there wherever serde reads and writes the type.
#[test]
fn a_generic_struct_with_a_defaulted_field_of_the_parameters_type_builds_and_reads() {
    let mut calls = 0_u32;
    let read = Lenient::<String>::from_bson_with(doc! { "total": 1_i32 }, |_raw, _found| {
        calls += 1;
        lenient_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Lenient {
            extra: String::new(),
            total: 1,
        })
    );
    assert_eq!(calls, 0);

    let refused = Lenient::<String>::from_bson_with(
        doc! { "extra": 7_i32, "total": 1_i32 },
        |_raw, _found| lenient_schema::Verdict::Reject,
    );
    assert_eq!(
        told!(lenient_schema, refused.unwrap_err().issues),
        [invalid("extra", "TypeParam(\"T\")", Bson::Int32(7))]
    );
}

/// A generic flagged type reached through a field is walked by its own walker at the field's
/// path, and a generic brand is read as the value it holds.
#[test]
fn a_generic_type_reached_through_a_field_is_walked_at_the_fields_path() {
    let listing = Listing {
        page: Page {
            items: vec![Version { number: 1 }],
            total: 1,
        },
        tagged: Tagged(oid("6a7cc592ca0574e6efdfe217")),
    };
    let mut calls = 0_u32;
    let read = Listing::from_bson_with(written(&listing), |_raw, _found| {
        calls += 1;
        listing_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(listing));
    assert_eq!(calls, 0);

    let items = vec![Bson::Document(doc! { "number": "1" })];
    let stored_row = doc! {
        "page": { "draft": true, "items": items.clone(), "total": "one" },
        "tagged": "6a7cc592ca0574e6efdfe217",
    };
    let listed =
        Listing::from_bson_with(stored_row, |_raw, _found| listing_schema::Verdict::Reject);
    assert_eq!(
        told!(listing_schema, listed.unwrap_err().issues),
        [
            invalid("page.items", "Array(TypeParam(\"T\"))", Bson::Array(items)),
            invalid("page.total", "U32", string("one")),
            unknown("page.draft", Bson::Boolean(true)),
            mistyped(
                "tagged",
                "TypeParam(\"T\")",
                string("6a7cc592ca0574e6efdfe217")
            ),
        ]
    );
}

/// An alias of a flagged model type is that type, so the field is walked into.
#[test]
fn a_field_typed_with_an_alias_of_a_model_type_is_walked_through_the_alias() {
    let mut featured = plain_row();
    featured.insert("name", 7_i32);
    featured.insert("versions", vec![doc! { "number": "1" }]);
    let read = Shelf::from_bson_with(doc! { "featured": featured }, |_raw, _found| {
        shelf_schema::Verdict::Reject
    });
    assert_eq!(
        told!(shelf_schema, read.unwrap_err().issues),
        [
            invalid("featured.name", "String", Bson::Int32(7)),
            invalid("featured.versions[0].number", "I32", string("1")),
        ]
    );
    let absent = Shelf::from_bson_with(Document::new(), |_raw, _found| {
        shelf_schema::Verdict::Reject
    });
    assert_eq!(
        told!(shelf_schema, absent.unwrap_err().issues),
        [missing("featured", "Model(\"RecordAlias\")")]
    );
}

/// The methods are generic over a decider and an issue type of their own, which take other names
/// where the type's parameters are called what they would be.
#[test]
fn a_type_whose_parameters_are_named_as_the_methods_own_builds_and_reads() {
    let stored_row = doc! { "format": "csv", "id": "6a7cc592ca0574e6efdfe217" };
    let read = Keyed::<ObjectId, String>::from_bson_with(stored_row, |_raw, _found| {
        keyed_schema::Verdict::Reject
    });
    assert_eq!(
        told!(keyed_schema, read.unwrap_err().issues),
        [mistyped(
            "id",
            "TypeParam(\"I\")",
            string("6a7cc592ca0574e6efdfe217")
        )]
    );
}

#[test]
fn a_field_typed_as_the_walkers_own_parameter_is_called_reads_as_the_authors_type() {
    let mut calls = 0_u32;
    let read = Holder::from_bson_with(doc! { "inner": { "number": 1_i32 } }, |_raw, _found| {
        calls += 1;
        holder_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Holder {
            inner: I { number: 1 }
        })
    );
    assert_eq!(calls, 0);

    let stored_row = doc! { "inner": { "number": "one" } };
    let refused = Holder::from_bson_with(stored_row, |_raw, _found| holder_schema::Verdict::Reject);
    assert_eq!(
        told!(holder_schema, refused.unwrap_err().issues),
        [invalid("inner.number", "I32", string("one"))]
    );
}

#[test]
fn what_serde_wrote_for_each_enum_form_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let stored_row = written(&drawing());
    assert_eq!(
        stored_row,
        doc! {
            "fill": { "kind": "Versioned", "number": 3_i32 },
            "outline": { "Circle": { "radius": 1.5_f64 } },
            "stroke": { "kind": "Dashed", "data": { "gap": 2_i64 } },
        }
    );
    let read = Drawing::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        drawing_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(drawing()));

    let units = doc! {
        "fill": { "kind": "Clear" },
        "outline": "Empty",
        "stroke": { "kind": "Hairline" },
    };
    let unit_read = Drawing::from_bson_with(units, |_raw, _found| {
        calls += 1;
        drawing_schema::Verdict::Reject
    });
    assert_eq!(
        unit_read,
        Ok(Drawing {
            fill: Fill::Clear,
            outline: Outline::Empty,
            stroke: Stroke::Hairline,
        })
    );

    let single = Drawing {
        fill: Fill::Solid {
            color: "red".to_owned(),
        },
        outline: Outline::Owned(oid("6a7cc592ca0574e6efdfe217")),
        stroke: Stroke::Span(1, 2),
    };
    let single_read = Drawing::from_bson_with(written(&single), |_raw, _found| {
        calls += 1;
        drawing_schema::Verdict::Reject
    });
    assert_eq!(single_read, Ok(single));

    let routed = written(&route());
    assert_eq!(
        routed,
        doc! {
            "contact": { "country": 1_i32, "digits": "555" },
            "next": { "To": [1_i32, 2_i32] },
            "status": "Draft",
        }
    );
    let route_read = Route::from_bson_with(routed, |_raw, _found| {
        calls += 1;
        route_schema::Verdict::Reject
    });
    assert_eq!(route_read, Ok(route()));
    assert_eq!(calls, 0);
}

/// A plain enum is one value, read with its own reader. A name it does not declare
/// is serde's refusal, and its name as the one key of a document is a form serde reads and the
/// enum does not write.
#[test]
fn a_plain_enum_is_one_value_read_with_its_own_reader() {
    let mut stored_row = written(&route());
    stored_row.insert("status", "Archived");
    assert!(!serde_reads::<Route>(&stored_row));
    let read = Route::from_bson_with(stored_row, |_raw, _found| route_schema::Verdict::Reject);
    assert_eq!(
        told!(route_schema, read.unwrap_err().issues),
        [invalid("status", "Model(\"Status\")", string("Archived"))]
    );

    let keyed = Bson::Document(doc! { "Draft": Bson::Null });
    let mut keyed_row = written(&route());
    keyed_row.insert("status", keyed.clone());
    assert!(serde_reads::<Route>(&keyed_row));
    let listed = Route::from_bson_with(keyed_row, |_raw, _found| route_schema::Verdict::Reject);
    assert_eq!(
        told!(route_schema, listed.unwrap_err().issues),
        [mistyped("status", "Model(\"Status\")", keyed)]
    );
}

/// A document is never the bare name a plain enum writes, so every call reports:
/// `Invalid` where serde refuses the document, and `Mistyped` where it reads it.
#[test]
fn from_bson_with_on_a_plain_enum_reports_on_every_call() {
    let mut calls = 0_u32;
    let read = Status::from_bson_with(Document::new(), |_raw, found| {
        calls += 1;
        assert_eq!(
            told!(status_schema, found),
            [invalid(
                "",
                "Model(\"Status\")",
                Bson::Document(Document::new())
            )]
        );
        status_schema::Verdict::Reject
    });
    read.unwrap_err();

    let keyed = doc! { "Draft": Bson::Null };
    assert!(serde_reads::<Status>(&keyed));
    let keyed_read = Status::from_bson_with(keyed.clone(), |_raw, found| {
        calls += 1;
        assert_eq!(
            told!(status_schema, found),
            [mistyped(
                "",
                "Model(\"Status\")",
                Bson::Document(keyed.clone())
            )]
        );
        status_schema::Verdict::Reject
    });
    keyed_read.unwrap_err();
    assert_eq!(calls, 2);
}

/// A tag naming no variant. It sits at the tag's key and holds the tag where the enum has
/// one, and at the enum's own path, holding the whole value, where the tag is the document's key.
#[test]
fn a_tag_naming_no_variant_is_invalid_with_the_variants_the_enum_accepts() {
    let stored_row = doc! {
        "fill": { "color": "red", "kind": "Striped" },
        "outline": { "Hexagon": {} },
        "stroke": { "data": 1_i32, "kind": "Dotted" },
    };
    assert!(!serde_reads::<Drawing>(&stored_row));
    let read = Drawing::from_bson_with(stored_row, |_raw, _found| drawing_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(drawing_schema, issues),
        [
            invalid(
                "fill.kind",
                "Variants([\"Clear\", \"Solid\", \"Versioned\"])",
                string("Striped")
            ),
            invalid(
                "outline",
                "Variants([\"Circle\", \"Empty\", \"Label\", \"Owned\"])",
                Bson::Document(doc! { "Hexagon": {} })
            ),
            invalid(
                "stroke.kind",
                "Variants([\"Dashed\", \"Hairline\", \"Level\", \"Span\", \"Width\"])",
                string("Dotted")
            ),
        ]
    );
    let reasons = reasons!(drawing_schema, issues);
    for (reason, tag) in reasons.iter().zip(["Striped", "Hexagon", "Dotted"]) {
        assert!(
            reason.contains(&format!("unknown variant `{tag}`")),
            "got: {reason}"
        );
    }
}

/// A problem inside the variant the tag names reaches the callback at its full path, and a
/// decider that works on each path repairs the row. An id stored as text inside a variant is
/// `Mistyped` there.
#[test]
fn an_issue_inside_a_variant_is_listed_and_fixed_at_its_full_path() {
    let stored_row = doc! {
        "fill": { "draft": true, "kind": "Versioned", "number": "3" },
        "outline": { "Owned": "6a7cc592ca0574e6efdfe217" },
        "stroke": { "data": {}, "kind": "Dashed" },
    };
    assert!(!serde_reads::<Drawing>(&stored_row));
    let mut seen: Vec<Told> = Vec::new();
    let read = Drawing::from_bson_with(stored_row, |raw, found| {
        use drawing_schema::{Expected, Issue, Verdict};

        seen = told!(drawing_schema, found);
        for issue in found {
            if let Issue::Invalid {
                path,
                expected: Expected::I32,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_document(raw, Bson::Int32(3));
            } else if let Issue::Mistyped {
                path,
                expected: Expected::ObjectId,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            } else if let Issue::Missing {
                path,
                expected: _expected,
            } = issue
            {
                path.set_in_document(raw, Bson::Int32(2));
            } else if let Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_document(raw);
            } else {
                return Verdict::Reject;
            }
        }
        Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            invalid("fill.number", "I32", string("3")),
            unknown("fill.draft", Bson::Boolean(true)),
            mistyped(
                "outline.Owned",
                "ObjectId",
                string("6a7cc592ca0574e6efdfe217")
            ),
            missing("stroke.data.gap", "U32"),
        ]
    );
    assert_eq!(
        read,
        Ok(Drawing {
            fill: Fill::Versioned(Version { number: 3_i32 }),
            outline: Outline::Owned(oid("6a7cc592ca0574e6efdfe217")),
            stroke: Stroke::Dashed { gap: 2 },
        })
    );

    let circle = doc! { "Circle": { "extra": 1_i32, "radius": "wide" } };
    assert!(!serde_reads::<Outline>(&circle));
    let listed = Outline::from_bson_with(circle, |_raw, _found| outline_schema::Verdict::Reject);
    assert_eq!(
        told!(outline_schema, listed.unwrap_err().issues),
        [
            invalid("Circle.radius", "F64", string("wide")),
            unknown("Circle.extra", Bson::Int32(1)),
        ]
    );
}

/// The content of a struct variant stored as a number is the one place the walker lists
/// nothing, so the read carries serde's refusal alone.
#[test]
fn a_struct_variants_content_stored_as_no_document_is_undescribed() {
    let external = doc! { "Circle": 5_i32 };
    assert!(!serde_reads::<Outline>(&external));
    let read = Outline::from_bson_with(external, |_raw, _found| outline_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(told!(outline_schema, issues), [undescribed()]);
    let reasons = reasons!(outline_schema, issues);
    assert!(
        reasons
            .iter()
            .all(|reason| reason.contains("invalid type: integer `5`")),
        "got: {reasons:?}"
    );

    let adjacent = doc! { "data": 5_i32, "kind": "Dashed" };
    assert!(!serde_reads::<Stroke>(&adjacent));
    let listed = Stroke::from_bson_with(adjacent, |_raw, _found| stroke_schema::Verdict::Reject);
    assert_eq!(
        told!(stroke_schema, listed.unwrap_err().issues),
        [undescribed()]
    );
}

/// An absent tag is the one issue, at the tag's key. Every key of an internally tagged
/// document then counts as declared, and so do the two keys an adjacently tagged one writes.
#[test]
fn an_absent_tag_is_missing_at_the_tags_key() {
    let internal = doc! { "color": "red" };
    assert!(!serde_reads::<Fill>(&internal));
    let read = Fill::from_bson_with(internal, |_raw, _found| fill_schema::Verdict::Reject);
    assert_eq!(
        told!(fill_schema, read.unwrap_err().issues),
        [missing(
            "kind",
            "Variants([\"Clear\", \"Solid\", \"Versioned\"])"
        )]
    );
    let adjacent = doc! { "data": { "gap": 2_i32 } };
    assert!(!serde_reads::<Stroke>(&adjacent));
    let listed = Stroke::from_bson_with(adjacent, |_raw, _found| stroke_schema::Verdict::Reject);
    assert_eq!(
        told!(stroke_schema, listed.unwrap_err().issues),
        [missing(
            "kind",
            "Variants([\"Dashed\", \"Hairline\", \"Level\", \"Span\", \"Width\"])"
        )]
    );
}

/// A tag naming no variant is `Mistyped` where serde reads the whole document all the same.
#[test]
fn a_tag_naming_no_variant_that_serde_reads_is_mistyped() {
    let stored_row = doc! { "kind": "Warp", "speed": 9_i32 };
    assert!(serde_reads::<Signal>(&stored_row));
    let read = Signal::from_bson_with(stored_row, |_raw, _found| signal_schema::Verdict::Reject);
    assert_eq!(
        told!(signal_schema, read.unwrap_err().issues),
        [mistyped(
            "kind",
            "Variants([\"Go\", \"Unrecognized\"])",
            string("Warp")
        )]
    );
}

/// A key beside the tag that the variant does not declare is `Unknown`, and what an adjacently
/// tagged variant holds is walked under the content key by its kind.
#[test]
fn a_tagged_enum_is_a_document_whose_other_keys_are_the_variants() {
    let internal = doc! { "extra": 1_i32, "kind": "Clear" };
    assert!(serde_reads::<Fill>(&internal));
    let read = Fill::from_bson_with(internal, |_raw, _found| fill_schema::Verdict::Reject);
    assert_eq!(
        told!(fill_schema, read.unwrap_err().issues),
        [unknown("extra", Bson::Int32(1))]
    );

    for (stored_row, told) in [
        (
            doc! { "extra": 1_i32, "kind": "Hairline" },
            vec![unknown("extra", Bson::Int32(1))],
        ),
        (
            doc! { "kind": "Dashed" },
            vec![missing("data", "Model(\"Stroke\")")],
        ),
        (doc! { "kind": "Width" }, vec![missing("data", "U32")]),
        (
            doc! { "data": "wide", "kind": "Width" },
            vec![invalid("data", "U32", string("wide"))],
        ),
        (
            doc! { "data": [1_i32, "2"], "kind": "Span" },
            vec![invalid("data[1]", "U32", string("2"))],
        ),
        (
            doc! { "data": 7_i32, "kind": "Span" },
            vec![invalid("data", "Tuple([U32, U32])", Bson::Int32(7))],
        ),
        (
            doc! { "data": "high", "kind": "Level" },
            vec![invalid("data", "Optional(U32)", string("high"))],
        ),
    ] {
        let listed = Stroke::from_bson_with(stored_row.clone(), |_raw, _found| {
            stroke_schema::Verdict::Reject
        });
        assert_eq!(
            told!(stroke_schema, listed.unwrap_err().issues),
            told,
            "for {stored_row}"
        );
    }

    let mut calls = 0_u32;
    for stored_row in [
        doc! { "kind": "Level" },
        doc! { "data": Bson::Null, "kind": "Level" },
    ] {
        let level = Stroke::from_bson_with(stored_row, |_raw, _found| {
            calls += 1;
            stroke_schema::Verdict::Reject
        });
        assert_eq!(level, Ok(Stroke::Level(None)));
    }
    assert_eq!(calls, 0);
}

/// A variant holding two values is walked by position under its key.
#[test]
fn a_variant_holding_two_values_is_walked_by_position() {
    let mut stored_row = written(&route());
    stored_row.insert("next", doc! { "To": [1_i32, "2", 3_i32] });
    assert!(!serde_reads::<Route>(&stored_row));
    let read = Route::from_bson_with(stored_row, |_raw, _found| route_schema::Verdict::Reject);
    assert_eq!(
        told!(route_schema, read.unwrap_err().issues),
        [
            invalid("next.To[1]", "I32", string("2")),
            unknown("next.To[2]", Bson::Int32(3)),
        ]
    );
    let short = Move::from_bson_with(doc! { "To": [1_i32] }, |_raw, _found| {
        move_schema::Verdict::Reject
    });
    assert_eq!(
        told!(move_schema, short.unwrap_err().issues),
        [missing("To[1]", "I32")]
    );
}

/// An untagged value no variant reads is one `NoVariant` at the enum's path, holding each
/// variant's own list in the order declared.
#[test]
fn an_untagged_value_no_variant_reads_is_one_no_variant_with_each_variants_list() {
    let stored_row = doc! { "country": "one", "digits": "555" };
    assert!(!serde_reads::<Contact>(&stored_row));
    let read = Contact::from_bson_with(stored_row.clone(), |_raw, _found| {
        contact_schema::Verdict::Reject
    });
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(contact_schema, issues),
        [(
            "NoVariant",
            String::new(),
            String::new(),
            Some(Bson::Document(stored_row))
        )]
    );
    assert_eq!(
        tried!(contact_schema, issues),
        [
            (
                "Email",
                vec![
                    missing("address", "String"),
                    unknown("country", string("one")),
                    unknown("digits", string("555")),
                ]
            ),
            ("Phone", vec![invalid("country", "I32", string("one"))]),
            (
                "Versioned",
                vec![
                    missing("number", "I32"),
                    unknown("country", string("one")),
                    unknown("digits", string("555")),
                ]
            ),
        ]
    );

    let mut held = written(&route());
    held.insert("contact", 7_i32);
    let listed = Route::from_bson_with(held, |_raw, _found| route_schema::Verdict::Reject);
    assert_eq!(
        told!(route_schema, listed.unwrap_err().issues),
        [(
            "NoVariant",
            "contact".to_owned(),
            String::new(),
            Some(Bson::Int32(7))
        )]
    );
}

/// The hook tixschema hangs on a constrained member is the one serde reads it through, so
/// a value the constraint refuses takes that variant out.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_untagged_enums_constrained_member_is_read_through_its_hook() {
    let stored_row = doc! { "address": "ab" };
    assert!(!serde_reads::<Contact>(&stored_row));
    let read = Contact::from_bson_with(stored_row, |_raw, _found| contact_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    let tried = tried!(contact_schema, issues);
    assert_eq!(
        tried,
        [
            ("Email", vec![invalid("address", "String", string("ab"))]),
            (
                "Phone",
                vec![
                    missing("country", "I32"),
                    missing("digits", "String"),
                    unknown("address", string("ab")),
                ]
            ),
            (
                "Versioned",
                vec![missing("number", "I32"), unknown("address", string("ab"))]
            ),
        ]
    );
    let refusals: Vec<&str> = issues
        .iter()
        .filter_map(|issue| {
            if let contact_schema::Issue::NoVariant {
                path: _path,
                found: _found,
                variants,
            } = issue
            {
                variants.first()
            } else {
                None
            }
        })
        .flat_map(|(_variant, list)| reasons!(contact_schema, list))
        .collect();
    assert_eq!(refusals.len(), 1);
    assert!(
        refusals
            .iter()
            .all(|reason| reason.contains("too short: minimum length is 3, got 2")),
        "got: {refusals:?}"
    );
}

/// serde says which variant reads the document, and that variant alone is walked.
#[test]
fn an_untagged_value_is_walked_as_the_variant_serde_reads_it_as() {
    let email = doc! { "address": "ann@example.org", "legacy": true };
    assert!(serde_reads::<Contact>(&email));
    let read = Contact::from_bson_with(email, |_raw, _found| contact_schema::Verdict::Reject);
    assert_eq!(
        told!(contact_schema, read.unwrap_err().issues),
        [unknown("legacy", Bson::Boolean(true))]
    );
    let versioned = doc! { "draft": true, "number": 3_i32 };
    assert!(serde_reads::<Contact>(&versioned));
    let listed = Contact::from_bson_with(versioned, |_raw, _found| contact_schema::Verdict::Reject);
    assert_eq!(
        told!(contact_schema, listed.unwrap_err().issues),
        [unknown("draft", Bson::Boolean(true))]
    );
}

/// Tags and keys are walked under the names serde writes, and `Variants` lists them.
#[test]
fn variants_and_their_fields_are_walked_under_their_wire_names() {
    let by_air = Shipment::ByAir {
        flight_code: "TX1".to_owned(),
    };
    let stored_row = written(&by_air);
    assert_eq!(stored_row, doc! { "type": "by_air", "FLIGHT_CODE": "TX1" });
    let mut calls = 0_u32;
    let read = Shipment::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        shipment_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(by_air));
    assert_eq!(calls, 0);

    for (renamed_row, told) in [
        (
            doc! { "flight_code": "TX1", "type": "by_air" },
            vec![
                missing("FLIGHT_CODE", "String"),
                unknown("flight_code", string("TX1")),
            ],
        ),
        (
            doc! { "type": "by_sea", "vesselName": 7_i32 },
            vec![invalid("vesselName", "String", Bson::Int32(7))],
        ),
        (
            doc! { "type": "InPerson" },
            vec![invalid(
                "type",
                "Variants([\"by_air\", \"by_sea\", \"pickup\"])",
                string("InPerson"),
            )],
        ),
    ] {
        let listed = Shipment::from_bson_with(renamed_row.clone(), |_raw, _found| {
            shipment_schema::Verdict::Reject
        });
        assert_eq!(
            told!(shipment_schema, listed.unwrap_err().issues),
            told,
            "for {renamed_row}"
        );
    }
}

/// An internally tagged variant holding a model type hands the document to that type's fields
/// walker, and the tag's key joins the keys it returns.
#[test]
fn a_tagged_enums_fields_walker_returns_the_keys_that_are_its_own() {
    let versioned = doc! { "draft": true, "kind": "Versioned", "number": "3" };
    let mut out: Vec<fill_schema::Issue<Bson>> = Vec::new();
    let declared = Fill::decode_with_bson_fields(
        &versioned,
        &[Ok("fill".to_owned())],
        fill_schema::issue_from_parts,
        &mut out,
    );
    assert_eq!(declared, ["number", "kind"]);
    assert_eq!(
        told!(fill_schema, out),
        [invalid("fill.number", "I32", string("3"))]
    );

    let mut none: Vec<fill_schema::Issue<Bson>> = Vec::new();
    assert_eq!(
        Fill::decode_with_bson_fields(
            &doc! { "color": "red", "kind": "Solid" },
            &[],
            fill_schema::issue_from_parts,
            &mut none,
        ),
        ["kind", "color"]
    );
    assert_eq!(
        Stroke::decode_with_bson_fields(
            &doc! { "data": { "gap": 2_i32 }, "kind": "Dashed", "legacy": true },
            &[],
            fill_schema::issue_from_parts,
            &mut none,
        ),
        ["kind", "data"]
    );
    assert_eq!(
        Outline::decode_with_bson_fields(
            &doc! { "Circle": { "radius": 1.5_f64 }, "legacy": true },
            &[],
            fill_schema::issue_from_parts,
            &mut none,
        ),
        ["Circle"]
    );
    assert_eq!(none, Vec::new());
}

/// A generic enum gets its methods under the bounds a generic struct does. A value of
/// the parameter's type is read whole where it sits, and written back to compare what it is
/// stored as.
#[test]
fn a_generic_enum_reads_a_parameters_value_whole() {
    let held = doc! { "number": "x" };
    let read = Answer::<Version>::from_bson_with(doc! { "Value": held.clone() }, |_raw, _found| {
        answer_schema::Verdict::Reject
    });
    assert_eq!(
        told!(answer_schema, read.unwrap_err().issues),
        [invalid("Value", "TypeParam(\"T\")", Bson::Document(held))]
    );

    let mut calls = 0_u32;
    let kept =
        Answer::<Version>::from_bson_with(doc! { "Value": { "number": 3_i32 } }, |_raw, _found| {
            calls += 1;
            answer_schema::Verdict::Reject
        });
    assert_eq!(kept, Ok(Answer::Value(Version { number: 3_i32 })));
    assert_eq!(calls, 0);

    let stored_row = doc! { "Value": "6a7cc592ca0574e6efdfe217" };
    assert!(serde_reads::<Answer<ObjectId>>(&stored_row));
    let listed = Answer::<ObjectId>::from_bson_with(stored_row, |_raw, _found| {
        answer_schema::Verdict::Reject
    });
    assert_eq!(
        told!(answer_schema, listed.unwrap_err().issues),
        [mistyped(
            "Value",
            "TypeParam(\"T\")",
            string("6a7cc592ca0574e6efdfe217")
        )]
    );
}

/// serde reads a variant under each alias it carries, so a tag or a key stored as one names that
/// variant: the read is serde's, and nothing is listed for the alias.
#[test]
fn a_tag_stored_as_an_alias_names_its_variant_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let stored_row = doc! { "coating": { "kind": "Blank" }, "contour": "Blank" };
    let read = Sketch::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        sketch_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Sketch {
            coating: Coating::Clear,
            contour: Contour::Empty,
        })
    );
    let keyed_row = doc! {
        "coating": { "color": "red", "kind": "Solid" },
        "contour": { "Round": { "radius": 1.5_f64 } },
    };
    let keyed = Sketch::from_bson_with(keyed_row, |_raw, _found| {
        calls += 1;
        sketch_schema::Verdict::Reject
    });
    assert_eq!(
        keyed,
        Ok(Sketch {
            coating: Coating::Solid {
                color: "red".to_owned(),
            },
            contour: Contour::Circle { radius: 1.5_f64 },
        })
    );
    for stored_dash in [
        doc! { "data": { "gap": 2_i32 }, "kind": "Dotted" },
        written(&Dash::Dashed { gap: 2_i32 }),
    ] {
        let dash = Dash::from_bson_with(stored_dash, |_raw, _found| {
            calls += 1;
            dash_schema::Verdict::Reject
        });
        assert_eq!(dash, Ok(Dash::Dashed { gap: 2_i32 }));
    }
    let hairline = Dash::from_bson_with(doc! { "kind": "Hairline" }, |_raw, _found| {
        calls += 1;
        dash_schema::Verdict::Reject
    });
    assert_eq!(hairline, Ok(Dash::Hairline));
    assert_eq!(calls, 0);
}

/// A variant stored under an alias is walked as that variant, and the key it is stored under is
/// the one its issues are pathed under, so a decider that works on each path repairs the row.
#[test]
fn an_issue_inside_a_variant_stored_under_an_alias_is_listed_and_fixed_under_the_stored_key() {
    let undeclared = doc! { "Round": { "extra": 1_i32, "radius": 1.5_f64 } };
    assert!(serde_reads::<Contour>(&undeclared));
    let read = Contour::from_bson_with(undeclared, |_raw, _found| contour_schema::Verdict::Reject);
    assert_eq!(
        told!(contour_schema, read.unwrap_err().issues),
        [unknown("Round.extra", Bson::Int32(1))]
    );

    let refused = doc! { "Round": { "radius": "wide" } };
    assert!(!serde_reads::<Contour>(&refused));
    let mut seen: Vec<Told> = Vec::new();
    let fixed = Contour::from_bson_with(refused, |raw, found| {
        seen = told!(contour_schema, found);
        for issue in found {
            if let contour_schema::Issue::Invalid {
                path,
                expected: contour_schema::Expected::F64,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_document(raw, Bson::Double(1.5_f64));
            } else {
                return contour_schema::Verdict::Reject;
            }
        }
        contour_schema::Verdict::Fixed
    });
    assert_eq!(seen, [invalid("Round.radius", "F64", string("wide"))]);
    assert_eq!(fixed, Ok(Contour::Circle { radius: 1.5_f64 }));

    let mut held = written(&Sketch {
        coating: Coating::Clear,
        contour: Contour::Empty,
    });
    held.insert("contour", doc! { "Round": {} });
    assert!(!serde_reads::<Sketch>(&held));
    let listed = Sketch::from_bson_with(held, |_raw, _found| sketch_schema::Verdict::Reject);
    assert_eq!(
        told!(sketch_schema, listed.unwrap_err().issues),
        [missing("contour.Round.radius", "F64")]
    );
}

/// An adjacently tagged variant whose tag is stored as an alias is walked under the content key,
/// which no alias renames.
#[test]
fn an_adjacent_tag_stored_as_an_alias_walks_its_content_under_the_content_key() {
    for (stored_row, reads, told) in [
        (
            doc! { "data": { "gap": "wide", "legacy": true }, "kind": "Dotted" },
            false,
            vec![
                invalid("data.gap", "I32", string("wide")),
                unknown("data.legacy", Bson::Boolean(true)),
            ],
        ),
        (
            doc! { "data": { "gap": 2_i32, "legacy": true }, "kind": "Dotted" },
            true,
            vec![unknown("data.legacy", Bson::Boolean(true))],
        ),
        (
            doc! { "kind": "Dotted" },
            false,
            vec![missing("data", "Model(\"Dash\")")],
        ),
    ] {
        assert_eq!(serde_reads::<Dash>(&stored_row), reads, "for {stored_row}");
        let listed = Dash::from_bson_with(stored_row.clone(), |_raw, _found| {
            dash_schema::Verdict::Reject
        });
        assert_eq!(
            told!(dash_schema, listed.unwrap_err().issues),
            told,
            "for {stored_row}"
        );
    }
}

/// serde reads no tag as a variant under `skip_deserializing`, so a tag naming one names no
/// variant: `Invalid` where the tag is. `Variants` lists what serde does read, each variant's
/// name and then its aliases, and nothing for that variant.
#[test]
fn a_tag_naming_a_variant_serde_never_reads_names_no_variant() {
    let stored_row = written(&Coating::Hidden);
    assert_eq!(stored_row, doc! { "kind": "Hidden" });
    assert!(!serde_reads::<Coating>(&stored_row));
    let read = Coating::from_bson_with(stored_row, |_raw, _found| coating_schema::Verdict::Reject);
    assert_eq!(
        told!(coating_schema, read.unwrap_err().issues),
        [invalid(
            "kind",
            "Variants([\"Clear\", \"Blank\", \"Solid\"])",
            string("Hidden")
        )]
    );
    let absent = Coating::from_bson_with(Document::new(), |_raw, _found| {
        coating_schema::Verdict::Reject
    });
    assert_eq!(
        told!(coating_schema, absent.unwrap_err().issues),
        [missing(
            "kind",
            "Variants([\"Clear\", \"Blank\", \"Solid\"])"
        )]
    );

    let unnamed = doc! { "Oval": {} };
    assert!(!serde_reads::<Contour>(&unnamed));
    let listed = Contour::from_bson_with(unnamed.clone(), |_raw, _found| {
        contour_schema::Verdict::Reject
    });
    assert_eq!(
        told!(contour_schema, listed.unwrap_err().issues),
        [invalid(
            "",
            "Variants([\"Circle\", \"Round\", \"Empty\", \"Blank\"])",
            Bson::Document(unnamed)
        )]
    );
}

/// serde reads no document as an untagged variant under `skip_deserializing`, so what it wrote for
/// one is read by no variant, and the one `NoVariant` holds a list for every other variant alone.
#[test]
fn an_untagged_variant_serde_never_reads_gets_no_list_of_its_own() {
    let stored_row = written(&Reach::Pager { number: 7_i32 });
    assert_eq!(stored_row, doc! { "number": 7_i32 });
    assert!(!serde_reads::<Reach>(&stored_row));
    let read = Reach::from_bson_with(stored_row.clone(), |_raw, _found| {
        reach_schema::Verdict::Reject
    });
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(reach_schema, issues),
        [(
            "NoVariant",
            String::new(),
            String::new(),
            Some(Bson::Document(stored_row))
        )]
    );
    assert_eq!(
        tried!(reach_schema, issues),
        [
            (
                "Email",
                vec![
                    missing("address", "String"),
                    unknown("number", Bson::Int32(7)),
                ]
            ),
            (
                "Phone",
                vec![
                    missing("country", "I32"),
                    missing("digits", "String"),
                    unknown("number", Bson::Int32(7)),
                ]
            ),
        ]
    );

    let email = doc! { "address": "ann@example.org", "legacy": true };
    assert!(serde_reads::<Reach>(&email));
    let listed = Reach::from_bson_with(email, |_raw, _found| reach_schema::Verdict::Reject);
    assert_eq!(
        told!(reach_schema, listed.unwrap_err().issues),
        [unknown("legacy", Bson::Boolean(true))]
    );
}

/// The key an externally tagged variant is stored under is the enum's own, alias or name.
#[test]
fn an_externally_tagged_enums_fields_walker_returns_the_key_the_variant_is_stored_under() {
    let mut none: Vec<contour_schema::Issue<Bson>> = Vec::new();
    for (stored_row, own) in [
        (
            doc! { "Round": { "radius": 1.5_f64 }, "legacy": true },
            "Round",
        ),
        (
            doc! { "Circle": { "radius": 1.5_f64 }, "legacy": true },
            "Circle",
        ),
        (doc! { "Blank": Bson::Null, "legacy": true }, "Blank"),
        (doc! { "Empty": Bson::Null, "legacy": true }, "Empty"),
    ] {
        assert_eq!(
            Contour::decode_with_bson_fields(
                &stored_row,
                &[],
                contour_schema::issue_from_parts,
                &mut none,
            ),
            [own]
        );
    }
    assert_eq!(none, Vec::new());
}
