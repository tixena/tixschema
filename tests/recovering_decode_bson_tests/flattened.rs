//! `from_bson_with` on types with `#[serde(flatten)]` fields. A flattened field's keys sit among
//! the keys of the document that flattens it, so it is walked in what the outer type's own fields
//! left of that document, its issues sit at that document's paths, and the keys it declares count
//! as declared. Each case holds the walker's list to what plain serde says of the same document.

use core::fmt::Debug;
use std::collections::HashMap;

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use serde::{Deserialize, Deserializer, Serialize};
use tixschema::model_schema;

// The JSON schema of a type holding a `Version` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use super::version_schema;
use super::{
    Told, Version, invalid, missing, mistyped, oid, serde_reads, string, undescribed, unknown,
    written,
};

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

/// What `from_bson_with` lists for `$stored`, less each `reason`, read as `$model` by a decider
/// that rejects it.
macro_rules! listed {
    ($model:ty, $module:ident, $stored:expr) => {
        told!(
            $module,
            <$model>::from_bson_with($stored, |_raw, _found| $module::Verdict::Reject)
                .unwrap_err()
                .issues
        )
    };
}

/// What `from_bson_with` reads `$stored` as, by a decider that counts each run of its own into
/// `$calls`.
macro_rules! read_counting {
    ($model:ty, $module:ident, $stored:expr, $calls:ident) => {
        <$model>::from_bson_with($stored, |_raw, _found| {
            $calls += 1;
            $module::Verdict::Reject
        })
    };
}

/// Implements [`Flagged`] for each `$model`, whose callback types `$module` holds.
macro_rules! flagged {
    ($($model:ty => $module:ident,)+) => {$(
        impl Flagged for $model {
            fn read(row: Document, json: serde_json::Value) -> (Option<Self>, Option<Self>, u32) {
                let mut calls = 0_u32;
                let from_row = read_counting!($model, $module, row, calls).ok();
                let from_json = <$model>::from_value_with(json, |_raw, _found| {
                    calls += 1;
                    $module::Verdict::Reject
                });
                (from_row, from_json.ok(), calls)
            }
        }
    )+};
}

/// A flagged type as [`reads_what_serde_wrote`] reads it.
trait Flagged: Sized {
    /// What `from_bson_with` reads `row` as and `from_value_with` reads `json` as, each under a
    /// decider that rejects, and how many times the two deciders ran.
    fn read(row: Document, json: serde_json::Value) -> (Option<Self>, Option<Self>, u32);
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Audit {
    created_by: String,
    revision: i32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Extra {
    note: String,
    weight: i32,
}

/// Internally tagged: `{"kind": "Solid", "color": "red"}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Paint {
    Clear,
    Solid { color: String },
}

/// Externally tagged: `{"Curved": {"radius": 1.5}}`, `"Straight"`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Edge {
    Curved { radius: f64 },
    Straight,
}

/// Adjacently tagged: `{"kind": "Dotted", "data": {"gap": 2}}`, `{"kind": "Solid"}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Trim {
    Dotted { gap: i32 },
    Solid,
}

/// An externally tagged enum and an adjacently tagged one, each flattened.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Framed {
    #[serde(flatten)]
    edge: Edge,
    title: String,
    #[serde(flatten)]
    trim: Trim,
}

/// A struct, an optional struct and a tagged enum, each flattened beside a key of the type's own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Sheet {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    extra: Option<Extra>,
    #[serde(flatten)]
    paint: Paint,
    title: String,
}

/// A type that flattens one that flattens others.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Binder {
    #[serde(flatten)]
    sheet: Sheet,
    shelf: String,
}

/// A type that flattens fields, held under a key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Cabinet {
    top: Sheet,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Stamp {
    version: Version,
}

/// An optional flattened type that holds a model type of its own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Proof {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    stamp: Option<Stamp>,
    title: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Origin {
    owner: ObjectId,
    source: String,
}

/// Two flattened structs, one holding an id.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Filed {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten)]
    origin: Origin,
    title: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ByMail {
    address: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ByPhone {
    digits: String,
}

/// Untagged, every variant a document: two model types and a variant with a field of its own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Channel {
    Mail(ByMail),
    Pager { number: i32 },
    Phone(ByPhone),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Notice {
    #[serde(flatten)]
    channel: Channel,
    subject: String,
}

/// A flattened map of plain values.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Counts {
    #[serde(flatten)]
    by_name: HashMap<String, i32>,
    title: String,
}

/// A flattened map of ids.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Owners {
    #[serde(flatten)]
    by_name: HashMap<String, ObjectId>,
    title: String,
}

/// A flattened map of model types, beside a flattened struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Releases {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten)]
    by_name: HashMap<String, Version>,
}

/// A model type that carries no flag: it only ever fills a parameter.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Body {
    text: String,
}

/// A flattened type parameter.
#[model_schema(decode_with, default_types(T = Body))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Letter<T> {
    #[serde(flatten)]
    body: T,
    id: String,
}

/// A flattened type parameter, then a flattened map: two fields that each take the rest.
#[model_schema(decode_with, default_types(T = Body))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Packet<T> {
    #[serde(flatten)]
    body: T,
    #[serde(flatten)]
    counts: HashMap<String, i32>,
    id: String,
}

/// Externally tagged, with a struct variant that flattens a field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Logged {
    Gone,
    Made {
        #[serde(flatten)]
        audit: Audit,
        title: String,
    },
}

/// Internally tagged, with a struct variant that flattens a field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Posted {
    Gone,
    Made {
        #[serde(flatten)]
        audit: Audit,
        title: String,
    },
}

/// Adjacently tagged, with a struct variant that flattens a field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Queued {
    Gone,
    Made {
        #[serde(flatten)]
        audit: Audit,
        title: String,
    },
}

/// Untagged, with a struct variant that flattens a field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Seen {
    Gone {
        at: i32,
    },
    Made {
        #[serde(flatten)]
        audit: Audit,
        title: String,
    },
}

/// An optional flattened map, which no walk reaches.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Spare {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    rest: Option<HashMap<String, i32>>,
    title: String,
}

/// A flattened map of numbers beside a flattened tagged enum, whose text tag serde hands to the
/// map as well.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Mixed {
    #[serde(flatten)]
    counts: HashMap<String, i32>,
    #[serde(flatten)]
    paint: Paint,
}

/// A type that flattens one that flattens a map, which takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Report {
    #[serde(flatten)]
    counts: Counts,
    id: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Captioned {
    name: String,
}

/// Untagged, with a map among its variants, which takes every key the enum is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Scores {
    Captioned(Captioned),
    Tallied(HashMap<String, i32>),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Scoreboard {
    id: String,
    #[serde(flatten)]
    scores: Scores,
}

/// Declares a key the type that flattens it declares too.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Part {
    #[serde(default)]
    title: i32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Whole {
    #[serde(flatten)]
    part: Part,
    title: String,
}

/// Internally tagged, with a struct variant that flattens a type that takes every key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Tabled {
    Gone,
    Made {
        #[serde(flatten)]
        counts: Counts,
        id: String,
    },
}

/// An optional flattened enum, internally tagged.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Painted {
    own: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    paint: Option<Paint>,
}

/// An optional flattened enum, untagged.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Reached {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    channel: Option<Channel>,
    own: String,
}

/// Two optional flattened enums, internally tagged and untagged, beside a flattened struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Layered {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    channel: Option<Channel>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    paint: Option<Paint>,
    title: String,
}

/// Internally tagged, with a variant that holds a model type under a key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Finish {
    Bare,
    Coated { version: Version },
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Finished {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    finish: Option<Finish>,
    own: String,
}

/// An optional flattened enum, externally tagged.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Edged {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    edge: Option<Edge>,
    own: String,
}

/// An optional flattened enum, adjacently tagged.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Trimmed {
    own: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    trim: Option<Trim>,
}

/// A struct with a named field, held as text so a map of numbers handed its key refuses it.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Rev {
    revision: String,
}

/// A flattened struct, then a flattened type that takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Two {
    #[serde(flatten)]
    audit: Rev,
    #[serde(flatten)]
    counts: Counts,
    id: String,
}

/// Internally tagged, with a variant that holds a type that takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Filled {
    Counted(Counts),
}

/// A single-slot struct over a map, flattened.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Extras(HashMap<String, i32>);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Bag {
    #[serde(flatten)]
    extras: Extras,
    title: String,
}

/// A single-slot struct over an `Option` of a struct, flattened.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Newest(Option<Version>);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Paper {
    #[serde(flatten)]
    newest: Newest,
    title: String,
}

/// A generic brand filled with a struct, flattened.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Wrap<T>(T);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Outer {
    #[serde(flatten)]
    body: Wrap<Version>,
    id: String,
}

/// A plain enum: flattened, serde writes the variant's name as a key holding `null`. It is read
/// only where it is flattened, which a schema surface refuses.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Mood {
    Calm,
    Tense,
}

/// A flattened plain enum. A schema surface refuses the declaration.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Moody {
    #[serde(flatten)]
    mood: Mood,
    name: String,
}

/// Internally tagged, with a variant that holds a plain enum. A schema surface refuses the
/// declaration.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Slot {
    Empty,
    Held(Mood),
}

/// Internally tagged, with a struct variant that flattens a struct and then a type that takes
/// every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Booked {
    Gone,
    Made {
        #[serde(flatten)]
        audit: Rev,
        #[serde(flatten)]
        counts: Counts,
        id: String,
    },
}

/// A flattened struct, then a flattened `Option` of a type that takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Optioned {
    #[serde(flatten)]
    audit: Rev,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    counts: Option<Counts>,
    id: String,
}

/// A single-slot struct whose slot a hook reads: serde hands the hook what the struct is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Postmark(#[serde(deserialize_with = "as_written")] Version);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Franked {
    id: String,
    #[serde(flatten)]
    postmark: Postmark,
}

/// A single-slot struct over an id, which serde writes as an object and so flattens.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Marker(ObjectId);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Docket {
    #[serde(flatten)]
    marker: Marker,
    name: String,
}

/// A flattened id, which serde writes as the entry `$oid` of the document that holds it.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct DirectId {
    #[serde(flatten)]
    id: ObjectId,
    name: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MaybeId {
    name: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    oid: Option<ObjectId>,
}

/// A flattened id read through a hook.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct IdThroughHook {
    name: String,
    #[serde(flatten, deserialize_with = "as_written")]
    oid: ObjectId,
}

/// A flattened generic brand filled with an `Option` of a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Pocket {
    #[serde(flatten)]
    body: Wrap<Option<Body>>,
    id: String,
}

/// A flattened `Option` read through a hook.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct DirectMaybe {
    id: String,
    #[serde(
        flatten,
        default,
        deserialize_with = "as_written",
        skip_serializing_if = "Option::is_none"
    )]
    version: Option<Version>,
}

/// A flattened struct read through a hook.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ThroughHook {
    id: String,
    #[serde(flatten, deserialize_with = "as_written")]
    version: Version,
}

/// A flattened map read through a hook.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MapThroughHook {
    #[serde(flatten, deserialize_with = "as_written")]
    entries: HashMap<String, i32>,
    id: String,
}

/// A flattened field read through a hook generic over what it reads, then a flattened type that
/// takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct HookAhead {
    #[serde(flatten, deserialize_with = "as_written")]
    ahead: Version,
    #[serde(flatten)]
    counts: Counts,
    id: String,
}

/// A flattened field serde writes and never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct NeverRead {
    id: String,
    #[serde(flatten, skip_deserializing, skip_serializing_if = "Option::is_none")]
    version: Option<Version>,
}

/// A flagged type with a value of its own to stand where serde reads none.
#[model_schema(decode_with)]
#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
struct Draft {
    pages: i32,
}

/// A flattened field serde writes and never reads, which takes its type's default.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Unpicked {
    #[serde(flatten, skip_deserializing)]
    draft: Draft,
    id: String,
}

/// An optional flattened type parameter.
#[model_schema(decode_with, default_types(T = Body))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Tucked<T> {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    body: Option<T>,
    id: String,
}

/// A flattened JSON value, which holds whatever entries are left.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct OpenEnded {
    id: String,
    #[serde(flatten)]
    rest: serde_json::Value,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MaybeOpenEnded {
    id: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    rest: Option<serde_json::Value>,
}

/// A flattened `Option` of a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MaybeCounted {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    entries: Option<HashMap<String, i32>>,
    id: String,
}

flagged! {
    Counts => counts_schema,
    DirectId => direct_id_schema,
    DirectMaybe => direct_maybe_schema,
    HookAhead => hook_ahead_schema,
    IdThroughHook => id_through_hook_schema,
    Letter<ObjectId> => letter_schema,
    Letter<Option<Body>> => letter_schema,
    Letter<Option<ObjectId>> => letter_schema,
    MapThroughHook => map_through_hook_schema,
    MaybeCounted => maybe_counted_schema,
    MaybeId => maybe_id_schema,
    MaybeOpenEnded => maybe_open_ended_schema,
    NeverRead => never_read_schema,
    OpenEnded => open_ended_schema,
    Pocket => pocket_schema,
    ThroughHook => through_hook_schema,
    Tucked<Body> => tucked_schema,
    Unpicked => unpicked_schema,
}

fn audit() -> Audit {
    Audit {
        created_by: "ada".to_owned(),
        revision: 1_i32,
    }
}

fn sheet(extra: Option<Extra>, paint: Paint) -> Sheet {
    Sheet {
        audit: audit(),
        extra,
        paint,
        title: "t".to_owned(),
    }
}

/// A row as `Sheet` writes it with its optional part absent, and one more key where `legacy`.
fn sheet_row(legacy: bool) -> Document {
    let mut row = doc! { "createdBy": "ada", "revision": 1_i32, "kind": "Clear", "title": "t" };
    if legacy {
        row.insert("legacy", true);
    }
    row
}

fn no_variant(path: &str, found: Bson) -> Told {
    ("NoVariant", path.to_owned(), String::new(), Some(found))
}

/// What serde writes for a type that flattens fields carries no key the walker does not know.
#[test]
fn what_serde_wrote_for_a_type_that_flattens_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let whole = sheet(
        Some(Extra {
            note: "n".to_owned(),
            weight: 2_i32,
        }),
        Paint::Solid {
            color: "red".to_owned(),
        },
    );
    let stored_row = written(&whole);
    assert_eq!(
        stored_row,
        doc! {
            "createdBy": "ada",
            "revision": 1_i32,
            "note": "n",
            "weight": 2_i32,
            "kind": "Solid",
            "color": "red",
            "title": "t",
        }
    );
    assert_eq!(
        read_counting!(Sheet, sheet_schema, stored_row, calls),
        Ok(whole)
    );

    let filed = Filed {
        audit: audit(),
        origin: Origin {
            owner: oid("6a7cc592ca0574e6efdfe217"),
            source: "import".to_owned(),
        },
        title: "t".to_owned(),
    };
    let filed_row = written(&filed);
    assert_eq!(
        filed_row.get("owner"),
        Some(&Bson::ObjectId(oid("6a7cc592ca0574e6efdfe217")))
    );
    assert_eq!(
        read_counting!(Filed, filed_schema, filed_row, calls),
        Ok(filed)
    );

    let notice = Notice {
        channel: Channel::Pager { number: 7_i32 },
        subject: "s".to_owned(),
    };
    assert_eq!(written(&notice), doc! { "number": 7_i32, "subject": "s" });
    assert_eq!(
        read_counting!(Notice, notice_schema, written(&notice), calls),
        Ok(notice)
    );

    let counts = Counts {
        by_name: HashMap::from([("a".to_owned(), 1_i32)]),
        title: "t".to_owned(),
    };
    assert_eq!(written(&counts), doc! { "a": 1_i32, "title": "t" });
    assert_eq!(
        read_counting!(Counts, counts_schema, written(&counts), calls),
        Ok(counts)
    );

    let letter = Letter {
        body: Body {
            text: "x".to_owned(),
        },
        id: "i".to_owned(),
    };
    assert_eq!(written(&letter), doc! { "text": "x", "id": "i" });
    assert_eq!(
        read_counting!(Letter<Body>, letter_schema, written(&letter), calls),
        Ok(letter)
    );
    assert_eq!(calls, 0);
}

/// With none of the optional type's keys in the document, serde reads the field as absent and so
/// does the walker.
#[test]
fn a_flattened_optional_type_that_is_absent_is_no_issue() {
    let mut calls = 0_u32;
    assert_eq!(written(&sheet(None, Paint::Clear)), sheet_row(false));
    assert_eq!(
        read_counting!(Sheet, sheet_schema, sheet_row(false), calls),
        Ok(sheet(None, Paint::Clear))
    );
    assert_eq!(calls, 0);
}

/// A key neither the type nor any type it flattens declares is listed once, by the type that
/// holds the document. A row's own `_id` is such a key.
#[test]
fn a_key_no_flattened_type_declares_is_unknown_once() {
    let mut calls = 0_u32;
    let mut stored_row = sheet_row(true);
    stored_row.insert("_id", oid("6a7cc592ca0574e6efdfe299"));
    assert!(serde_reads::<Sheet>(&stored_row));
    let read = read_counting!(Sheet, sheet_schema, stored_row, calls);
    assert_eq!(
        told!(sheet_schema, read.unwrap_err().issues),
        [
            unknown("legacy", Bson::Boolean(true)),
            unknown("_id", Bson::ObjectId(oid("6a7cc592ca0574e6efdfe299"))),
        ]
    );
    assert_eq!(calls, 1);
}

/// An issue inside a flattened type sits at the document's own path plus the type's own key, and
/// never under the name of the field that flattens it.
#[test]
fn an_issue_inside_each_flattened_type_is_listed_at_the_outer_documents_path() {
    let stored_row = doc! { "createdBy": 7_i32, "kind": "Solid", "title": "t" };
    assert!(!serde_reads::<Sheet>(&stored_row));
    assert_eq!(
        listed!(Sheet, sheet_schema, stored_row),
        [
            invalid("createdBy", "String", Bson::Int32(7)),
            missing("revision", "I32"),
            missing("color", "String"),
        ]
    );
}

/// A value a flattened type reads but stores as another type is `Mistyped` at the document's own
/// path, and fixed there.
#[test]
fn an_id_stored_as_text_inside_a_flattened_type_is_mistyped_and_fixed() {
    let stored_row = doc! {
        "createdBy": "ada",
        "revision": 1_i32,
        "owner": "6a7cc592ca0574e6efdfe217",
        "source": "import",
        "title": "t",
    };
    assert!(serde_reads::<Filed>(&stored_row));
    let mut seen: Vec<Told> = Vec::new();
    let read = Filed::from_bson_with(stored_row, |raw, found| {
        seen = told!(filed_schema, found);
        for issue in found {
            if let filed_schema::Issue::Mistyped {
                path,
                expected: filed_schema::Expected::ObjectId,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            } else {
                return filed_schema::Verdict::Reject;
            }
        }
        filed_schema::Verdict::Fixed
    });
    assert_eq!(
        seen,
        [mistyped(
            "owner",
            "ObjectId",
            string("6a7cc592ca0574e6efdfe217")
        )]
    );
    assert_eq!(
        read,
        Ok(Filed {
            audit: audit(),
            origin: Origin {
                owner: oid("6a7cc592ca0574e6efdfe217"),
                source: "import".to_owned(),
            },
            title: "t".to_owned(),
        })
    );
}

/// With some of the optional type's keys in the document and not all it needs, serde reads the
/// field as absent: nothing is `Invalid`, and the document holds what the field would not write.
#[test]
fn a_flattened_optional_type_half_there_is_mistyped_at_the_document() {
    let mut stored_row = sheet_row(false);
    stored_row.insert("note", "n");
    assert!(serde_reads::<Sheet>(&stored_row));
    let read = Sheet::from_bson_with(stored_row.clone(), |raw, found| {
        for issue in found {
            if let sheet_schema::Issue::Mistyped {
                path,
                expected: _expected,
                found: _found,
            } = issue
                && path.0.is_empty()
            {
                raw.remove("note");
            } else {
                return sheet_schema::Verdict::Reject;
            }
        }
        sheet_schema::Verdict::Fixed
    });
    assert_eq!(read, Ok(sheet(None, Paint::Clear)));
    assert_eq!(
        listed!(Sheet, sheet_schema, stored_row.clone()),
        [mistyped(
            "",
            "Optional(Model(\"Extra\"))",
            Bson::Document(stored_row)
        )]
    );
}

/// With its keys in the document and serde reading it, an optional flattened type lists what its
/// own walk finds, at the document's path.
#[test]
fn a_flattened_optional_type_that_is_there_lists_its_own_issues() {
    let stored_row = doc! { "version": { "number": 3_i32, "draft": true }, "title": "t" };
    assert!(serde_reads::<Proof>(&stored_row));
    assert_eq!(
        listed!(Proof, proof_schema, stored_row),
        [unknown("version.draft", Bson::Boolean(true))]
    );
    let refused = doc! { "version": { "number": "3" }, "title": "t" };
    assert!(serde_reads::<Proof>(&refused));
    assert_eq!(
        listed!(Proof, proof_schema, refused.clone()),
        [mistyped(
            "",
            "Optional(Model(\"Stamp\"))",
            Bson::Document(refused)
        )]
    );
}

/// A flattened tagged enum reads its tag in the document that flattens it: a tag naming no variant
/// and an absent one are each the one issue, at the tag's key.
#[test]
fn a_flattened_tagged_enum_reads_its_tag_in_the_outer_document() {
    let mut unknown_tag = sheet_row(false);
    unknown_tag.insert("kind", "Striped");
    assert!(!serde_reads::<Sheet>(&unknown_tag));
    assert_eq!(
        listed!(Sheet, sheet_schema, unknown_tag),
        [invalid(
            "kind",
            "Variants([\"Clear\", \"Solid\"])",
            string("Striped")
        )]
    );
    let mut absent = sheet_row(false);
    absent.remove("kind");
    assert!(!serde_reads::<Sheet>(&absent));
    assert_eq!(
        listed!(Sheet, sheet_schema, absent),
        [missing("kind", "Variants([\"Clear\", \"Solid\"])")]
    );
}

/// A flattened externally tagged enum is the key naming its variant in the outer document, and a
/// flattened adjacently tagged one its tag and content keys there: each is walked under those.
#[test]
fn a_flattened_externally_or_adjacently_tagged_enum_is_walked_under_its_own_keys() {
    let mut calls = 0_u32;
    let framed = Framed {
        edge: Edge::Curved { radius: 1.5_f64 },
        title: "t".to_owned(),
        trim: Trim::Dotted { gap: 2_i32 },
    };
    assert_eq!(
        written(&framed),
        doc! {
            "Curved": { "radius": 1.5_f64 },
            "title": "t",
            "kind": "Dotted",
            "data": { "gap": 2_i32 },
        }
    );
    assert_eq!(
        read_counting!(Framed, framed_schema, written(&framed), calls),
        Ok(framed)
    );
    let units = Framed {
        edge: Edge::Straight,
        title: "t".to_owned(),
        trim: Trim::Solid,
    };
    assert_eq!(
        written(&units),
        doc! { "Straight": Bson::Null, "title": "t", "kind": "Solid" }
    );
    assert_eq!(
        read_counting!(Framed, framed_schema, written(&units), calls),
        Ok(units)
    );
    assert_eq!(calls, 0);

    let stored_row = doc! {
        "Curved": { "radius": "wide" },
        "data": {},
        "kind": "Dotted",
        "legacy": true,
        "title": "t",
    };
    assert!(!serde_reads::<Framed>(&stored_row));
    assert_eq!(
        listed!(Framed, framed_schema, stored_row),
        [
            invalid("Curved.radius", "F64", string("wide")),
            missing("data.gap", "I32"),
            unknown("legacy", Bson::Boolean(true)),
        ]
    );
    let neither = doc! { "kind": "Solid", "title": "t" };
    assert!(!serde_reads::<Framed>(&neither));
    assert_eq!(
        listed!(Framed, framed_schema, neither),
        [missing("", "Variants([\"Curved\", \"Straight\"])")]
    );
}

/// A flattened untagged enum is walked as the variant serde reads from the document, whose keys
/// are then declared, so a key that variant does not declare is `Unknown`.
#[test]
fn a_flattened_untagged_enum_is_walked_as_the_variant_serde_reads() {
    let stored_row = doc! { "address": "a@b", "legacy": true, "subject": "s" };
    assert!(serde_reads::<Notice>(&stored_row));
    assert_eq!(
        listed!(Notice, notice_schema, stored_row),
        [unknown("legacy", Bson::Boolean(true))]
    );
    let own_fields = doc! { "legacy": true, "number": 7_i32, "subject": "s" };
    assert!(serde_reads::<Notice>(&own_fields));
    assert_eq!(
        listed!(Notice, notice_schema, own_fields),
        [unknown("legacy", Bson::Boolean(true))]
    );
}

/// Where serde reads the document as no variant, the one issue is `NoVariant` at the document's own
/// path. Each variant's list holds what its own fields earn, and no key is `Unknown`.
#[test]
fn a_flattened_untagged_enum_no_variant_reads_is_one_no_variant() {
    let stored_row = doc! { "digits": 5_i32, "subject": "s" };
    assert!(!serde_reads::<Notice>(&stored_row));
    let issues = Notice::from_bson_with(stored_row, |_raw, _found| notice_schema::Verdict::Reject)
        .unwrap_err()
        .issues;
    assert_eq!(
        told!(notice_schema, issues),
        [no_variant("", Bson::Document(doc! { "digits": 5_i32 }))]
    );
    assert_eq!(
        tried!(notice_schema, issues),
        [
            ("Mail", vec![missing("address", "String")]),
            ("Pager", vec![missing("number", "I32")]),
            ("Phone", vec![invalid("digits", "String", Bson::Int32(5))]),
        ]
    );
}

/// An untagged enum's fields walker returns the keys of the variant serde reads, and every key of
/// the document where serde reads none.
#[test]
fn an_untagged_enums_fields_walker_returns_the_keys_of_the_variant_serde_reads() {
    let mut out: Vec<channel_schema::Issue<Bson>> = Vec::new();
    assert_eq!(
        Channel::decode_with_bson_fields(
            &doc! { "address": "a@b", "legacy": true },
            &[],
            channel_schema::issue_from_parts,
            &mut out,
        ),
        ["address"]
    );
    assert_eq!(
        Channel::decode_with_bson_fields(
            &doc! { "legacy": true, "number": 7_i32 },
            &[],
            channel_schema::issue_from_parts,
            &mut out,
        ),
        ["number"]
    );
    assert_eq!(out, Vec::new());
    let neither = doc! { "digits": 5_i32, "legacy": true };
    assert_eq!(
        Channel::decode_with_bson_fields(
            &neither,
            &[Ok("channel".to_owned())],
            channel_schema::issue_from_parts,
            &mut out,
        ),
        ["digits", "legacy"]
    );
    assert_eq!(
        told!(channel_schema, out),
        [no_variant("channel", Bson::Document(neither))]
    );
}

/// Every key nothing else declares belongs to a flattened map: none is `Unknown`, and each such
/// value is read as the map's own, at its key.
#[test]
fn a_flattened_map_takes_every_key_nothing_else_declares() {
    let stored_row = doc! { "a": "x", "b": 2_i32, "title": "t" };
    assert!(!serde_reads::<Counts>(&stored_row));
    assert_eq!(
        listed!(Counts, counts_schema, stored_row),
        [invalid("a", "I32", string("x"))]
    );
    let owners = doc! { "alice": "6a7cc592ca0574e6efdfe219", "title": "t" };
    assert!(serde_reads::<Owners>(&owners));
    assert_eq!(
        listed!(Owners, owners_schema, owners),
        [mistyped(
            "alice",
            "ObjectId",
            string("6a7cc592ca0574e6efdfe219")
        )]
    );
}

/// A flattened map of model types walks each value at its key, past the keys a flattened struct
/// beside it declares.
#[test]
fn a_flattened_map_of_model_types_walks_each_value_at_its_key() {
    let mut calls = 0_u32;
    let releases = Releases {
        audit: audit(),
        by_name: HashMap::from([("first".to_owned(), Version { number: 1_i32 })]),
    };
    assert_eq!(
        written(&releases),
        doc! { "createdBy": "ada", "revision": 1_i32, "first": { "number": 1_i32 } }
    );
    assert_eq!(
        read_counting!(Releases, releases_schema, written(&releases), calls),
        Ok(releases)
    );
    assert_eq!(calls, 0);

    let stored_row = doc! {
        "createdBy": "ada",
        "revision": 1_i32,
        "first": { "number": "1", "draft": true },
    };
    assert!(!serde_reads::<Releases>(&stored_row));
    assert_eq!(
        listed!(Releases, releases_schema, stored_row),
        [
            invalid("first.number", "I32", string("1")),
            unknown("first.draft", Bson::Boolean(true)),
        ]
    );
}

/// A flattened type parameter is read whole, with its own reader, from the keys nothing else
/// declares. What fills it is invisible to the walker, so none of those keys is `Unknown`.
#[test]
fn a_flattened_type_parameter_is_read_whole_from_the_keys_nothing_else_declares() {
    let stored_row = doc! { "id": "i", "more": 1_i32, "text": 5_i32 };
    assert!(!serde_reads::<Letter<Body>>(&stored_row));
    assert_eq!(
        listed!(Letter<Body>, letter_schema, stored_row),
        [invalid(
            "",
            "TypeParam(\"T\")",
            Bson::Document(doc! { "more": 1_i32, "text": 5_i32 })
        )]
    );

    let mut calls = 0_u32;
    let extra_key = doc! { "id": "i", "more": 1_i32, "text": "x" };
    assert_eq!(
        read_counting!(Letter<Body>, letter_schema, extra_key, calls),
        Ok(Letter {
            body: Body {
                text: "x".to_owned(),
            },
            id: "i".to_owned(),
        })
    );
    // Filled with a map, the parameter is read from the keys the type itself does not declare.
    let counted = doc! { "id": "i", "x": 1_i32 };
    assert!(serde_reads::<Letter<HashMap<String, i32>>>(&counted));
    assert_eq!(
        read_counting!(Letter<HashMap<String, i32>>, letter_schema, counted, calls),
        Ok(Letter {
            body: HashMap::from([("x".to_owned(), 1_i32)]),
            id: "i".to_owned(),
        })
    );
    assert_eq!(calls, 0);
}

/// What the first field that takes the rest leaves for the next is nothing the walker can see: the
/// struct filling the parameter reads its own key, and serde hands the map only what is left. The
/// second field is walked by nothing, so a value serde refuses in it is serde's refusal alone.
#[test]
fn a_second_flattened_field_that_takes_the_rest_is_walked_by_nothing() {
    let mut calls = 0_u32;
    let stored_row = doc! { "a": 1_i32, "id": "i", "text": "x" };
    assert_eq!(
        read_counting!(Packet<Body>, packet_schema, stored_row, calls),
        Ok(Packet {
            body: Body {
                text: "x".to_owned(),
            },
            counts: HashMap::from([("a".to_owned(), 1_i32)]),
            id: "i".to_owned(),
        })
    );
    assert_eq!(calls, 0);
    let refused = doc! { "a": "bad", "id": "i", "text": "x" };
    assert!(!serde_reads::<Packet<Body>>(&refused));
    assert_eq!(
        listed!(Packet<Body>, packet_schema, refused),
        [undescribed()]
    );
}

/// Each of two flattened types contributes its keys, and one undeclared key is listed once.
#[test]
fn two_flattened_types_each_declare_their_keys() {
    let stored_row = doc! {
        "createdBy": "ada",
        "legacy": true,
        "owner": oid("6a7cc592ca0574e6efdfe217"),
        "revision": 1_i32,
        "source": "import",
        "title": "t",
    };
    assert!(serde_reads::<Filed>(&stored_row));
    let mut out: Vec<filed_schema::Issue<Bson>> = Vec::new();
    assert_eq!(
        Filed::decode_with_bson_fields(&stored_row, &[], filed_schema::issue_from_parts, &mut out),
        ["title", "createdBy", "revision", "owner", "source"]
    );
    assert_eq!(out, Vec::new());
    assert_eq!(
        listed!(Filed, filed_schema, stored_row),
        [unknown("legacy", Bson::Boolean(true))]
    );
}

/// A type that flattens one that flattens others declares every key down the chain, and a type
/// that flattens fields is walked at the path of the key it is held under.
#[test]
fn a_flattened_type_that_flattens_others_is_walked_in_the_same_document() {
    let mut stored_row = sheet_row(true);
    stored_row.insert("createdBy", 7_i32);
    stored_row.insert("shelf", "s");
    assert!(!serde_reads::<Binder>(&stored_row));
    assert_eq!(
        listed!(Binder, binder_schema, stored_row),
        [
            invalid("createdBy", "String", Bson::Int32(7)),
            unknown("legacy", Bson::Boolean(true)),
        ]
    );
    let mut top = sheet_row(true);
    top.insert("createdBy", 7_i32);
    let held = doc! { "top": top };
    assert!(!serde_reads::<Cabinet>(&held));
    assert_eq!(
        listed!(Cabinet, cabinet_schema, held),
        [
            invalid("top.createdBy", "String", Bson::Int32(7)),
            unknown("top.legacy", Bson::Boolean(true)),
        ]
    );
}

/// What serde writes for a struct variant that flattens a field is read under every tagging.
#[test]
fn what_serde_wrote_for_a_variant_that_flattens_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let logged = written(&Logged::Made {
        audit: audit(),
        title: "t".to_owned(),
    });
    assert_eq!(
        logged,
        doc! { "Made": { "createdBy": "ada", "revision": 1_i32, "title": "t" } }
    );
    read_counting!(Logged, logged_schema, logged, calls).unwrap();
    let posted = written(&Posted::Made {
        audit: audit(),
        title: "t".to_owned(),
    });
    assert_eq!(
        posted,
        doc! { "kind": "Made", "createdBy": "ada", "revision": 1_i32, "title": "t" }
    );
    read_counting!(Posted, posted_schema, posted, calls).unwrap();
    let queued = written(&Queued::Made {
        audit: audit(),
        title: "t".to_owned(),
    });
    assert_eq!(
        queued,
        doc! { "kind": "Made", "data": { "createdBy": "ada", "revision": 1_i32, "title": "t" } }
    );
    read_counting!(Queued, queued_schema, queued, calls).unwrap();
    let seen = written(&Seen::Made {
        audit: audit(),
        title: "t".to_owned(),
    });
    assert_eq!(
        seen,
        doc! { "createdBy": "ada", "revision": 1_i32, "title": "t" }
    );
    read_counting!(Seen, seen_schema, seen, calls).unwrap();
    assert_eq!(calls, 0);
}

/// A variant's flattened field is walked in the document the variant's fields sit in, wherever the
/// tagging puts that document, and its keys count as the variant's own.
#[test]
fn a_variants_flattened_field_is_walked_in_the_variants_document_under_every_tagging() {
    let content = doc! { "createdBy": 7_i32, "legacy": 1_i32, "title": "t" };
    let mut internal = content.clone();
    internal.insert("kind", "Made");
    assert!(!serde_reads::<Posted>(&internal));
    assert_eq!(
        listed!(Posted, posted_schema, internal),
        [
            invalid("createdBy", "String", Bson::Int32(7)),
            missing("revision", "I32"),
            unknown("legacy", Bson::Int32(1)),
        ]
    );
    let external = doc! { "Made": content.clone() };
    assert!(!serde_reads::<Logged>(&external));
    assert_eq!(
        listed!(Logged, logged_schema, external),
        [
            invalid("Made.createdBy", "String", Bson::Int32(7)),
            missing("Made.revision", "I32"),
            unknown("Made.legacy", Bson::Int32(1)),
        ]
    );
    let adjacent = doc! { "data": content, "kind": "Made" };
    assert!(!serde_reads::<Queued>(&adjacent));
    assert_eq!(
        listed!(Queued, queued_schema, adjacent),
        [
            invalid("data.createdBy", "String", Bson::Int32(7)),
            missing("data.revision", "I32"),
            unknown("data.legacy", Bson::Int32(1)),
        ]
    );
    let untagged = doc! { "createdBy": "ada", "legacy": 1_i32, "revision": 1_i32, "title": "t" };
    assert!(serde_reads::<Seen>(&untagged));
    assert_eq!(
        listed!(Seen, seen_schema, untagged),
        [unknown("legacy", Bson::Int32(1))]
    );
}

/// An untagged enum no variant of which reads the document lists, for the variant that flattens a
/// field, what that field's type finds beside the variant's own.
#[test]
fn an_untagged_variant_that_flattens_lists_the_flattened_types_issues_as_its_own() {
    let stored_row = doc! { "createdBy": 7_i32, "title": "t" };
    assert!(!serde_reads::<Seen>(&stored_row));
    let issues = Seen::from_bson_with(stored_row, |_raw, _found| seen_schema::Verdict::Reject)
        .unwrap_err()
        .issues;
    assert_eq!(
        tried!(seen_schema, issues),
        [
            (
                "Gone",
                vec![
                    missing("at", "I32"),
                    unknown("createdBy", Bson::Int32(7)),
                    unknown("title", string("t")),
                ]
            ),
            (
                "Made",
                vec![
                    invalid("createdBy", "String", Bson::Int32(7)),
                    missing("revision", "I32"),
                ]
            ),
        ]
    );
}

/// No walk reaches an optional flattened map: serde reads it as absent where a value in it is not
/// the map's own, so nothing is listed and no key is `Unknown`.
#[test]
fn an_optional_flattened_map_is_walked_by_nothing_and_takes_every_key() {
    let mut calls = 0_u32;
    let stored_row = doc! { "a": "x", "title": "t" };
    assert_eq!(
        read_counting!(Spare, spare_schema, stored_row, calls),
        Ok(Spare {
            rest: None,
            title: "t".to_owned(),
        })
    );
    assert_eq!(calls, 0);
}

/// serde hands a flattened map the keys of a tagged enum flattened beside it, which the walker
/// counts as the enum's alone. serde refuses the text tag as a number and the walk finds nothing,
/// so the read carries serde's refusal alone.
#[test]
fn a_flattened_map_that_cannot_hold_the_tag_of_an_enum_beside_it_is_undescribed() {
    let stored_row = doc! { "kind": "Clear" };
    assert!(!serde_reads::<Mixed>(&stored_row));
    assert_eq!(listed!(Mixed, mixed_schema, stored_row), [undescribed()]);
}

/// serde hands a flattened type the keys the outer type's own fields did not take. A flattened
/// type that flattens a map is handed no key of the outer type's, so the map reads none.
#[test]
fn a_flattened_type_that_flattens_a_map_is_handed_no_key_of_the_outer_type() {
    let report = Report {
        counts: Counts {
            by_name: HashMap::from([("a".to_owned(), 1_i32)]),
            title: "t".to_owned(),
        },
        id: "i".to_owned(),
    };
    assert_eq!(
        written(&report),
        doc! { "a": 1_i32, "title": "t", "id": "i" }
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Report, report_schema, written(&report), calls),
        Ok(report)
    );
    assert_eq!(calls, 0);
}

/// A flattened untagged enum is read from what the outer type's own fields left, so a map among
/// its variants holds none of the outer type's keys.
#[test]
fn a_flattened_untagged_enum_with_a_map_variant_is_handed_no_key_of_the_outer_type() {
    let scoreboard = Scoreboard {
        id: "i".to_owned(),
        scores: Scores::Tallied(HashMap::from([("a".to_owned(), 1_i32)])),
    };
    assert_eq!(written(&scoreboard), doc! { "id": "i", "a": 1_i32 });
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Scoreboard, scoreboard_schema, written(&scoreboard), calls),
        Ok(scoreboard)
    );
    assert_eq!(calls, 0);
}

/// A key both the outer type and a type it flattens declare is the outer type's alone: serde
/// hands the flattened type a document without it, and so does the walker.
#[test]
fn a_key_the_outer_type_and_a_flattened_type_both_declare_is_the_outer_types_alone() {
    let stored_row = doc! { "title": "t" };
    assert!(serde_reads::<Whole>(&stored_row));
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Whole, whole_schema, stored_row, calls),
        Ok(Whole {
            part: Part { title: 0_i32 },
            title: "t".to_owned(),
        })
    );
    assert_eq!(calls, 0);

    let numbered = doc! { "title": 5_i32 };
    assert!(!serde_reads::<Whole>(&numbered));
    assert_eq!(
        listed!(Whole, whole_schema, numbered),
        [invalid("title", "String", Bson::Int32(5))]
    );
    let undeclared = doc! { "legacy": true, "title": "t" };
    assert!(serde_reads::<Whole>(&undeclared));
    assert_eq!(
        listed!(Whole, whole_schema, undeclared),
        [unknown("legacy", Bson::Boolean(true))]
    );
}

/// An issue inside a flattened type that takes every key it is handed is listed where it sits: a
/// value the map refuses at its key, and none at a key of the outer type's.
#[test]
fn an_issue_in_a_flattened_type_that_flattens_a_map_is_listed_at_its_key() {
    let stored_row = doc! { "a": "x", "id": "i", "title": "t" };
    assert!(!serde_reads::<Report>(&stored_row));
    assert_eq!(
        listed!(Report, report_schema, stored_row),
        [invalid("a", "I32", string("x"))]
    );
    // The map takes a key nothing declares, so serde refuses a value in it that is no count.
    let undeclared = doc! { "a": 1_i32, "id": "i", "legacy": [1_i32], "title": "t" };
    assert!(!serde_reads::<Report>(&undeclared));
    assert_eq!(
        listed!(Report, report_schema, undeclared),
        [invalid("legacy", "I32", Bson::Array(vec![Bson::Int32(1)]))]
    );
}

/// Where no variant of a flattened untagged enum reads what it is handed, the one issue is
/// `NoVariant` at the outer document's path, holding what the enum was handed.
#[test]
fn a_flattened_untagged_enum_no_variant_reads_holds_what_the_enum_was_handed() {
    let stored_row = doc! { "a": "x", "id": "i" };
    assert!(!serde_reads::<Scoreboard>(&stored_row));
    let issues = Scoreboard::from_bson_with(stored_row, |_raw, _found| {
        scoreboard_schema::Verdict::Reject
    })
    .unwrap_err()
    .issues;
    assert_eq!(
        told!(scoreboard_schema, issues),
        [no_variant("", Bson::Document(doc! { "a": "x" }))]
    );
    assert_eq!(
        tried!(scoreboard_schema, issues),
        [
            ("Captioned", vec![missing("name", "String")]),
            ("Tallied", vec![invalid("a", "I32", string("x"))]),
        ]
    );
}

/// A variant's flattened type is handed what the variant's own fields left of the document they
/// sit in, and none of the tag an internally tagged enum reads there.
#[test]
fn a_variants_flattened_type_that_flattens_a_map_is_handed_no_key_of_the_variant() {
    let made = Tabled::Made {
        counts: Counts {
            by_name: HashMap::from([("a".to_owned(), 1_i32)]),
            title: "t".to_owned(),
        },
        id: "i".to_owned(),
    };
    assert_eq!(
        written(&made),
        doc! { "kind": "Made", "a": 1_i32, "title": "t", "id": "i" }
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Tabled, tabled_schema, written(&made), calls),
        Ok(made)
    );
    assert_eq!(calls, 0);
    let stored_row = doc! { "a": "x", "id": "i", "kind": "Made", "title": "t" };
    assert!(!serde_reads::<Tabled>(&stored_row));
    assert_eq!(
        listed!(Tabled, tabled_schema, stored_row),
        [invalid("a", "I32", string("x"))]
    );
}

/// A flattened optional enum is absent when what names its variant is absent: the tag of a tagged
/// one, and any variant serde reads of an untagged one. serde wrote each row, and reads it.
#[test]
fn a_flattened_optional_enum_that_is_absent_is_no_issue() {
    let mut calls = 0_u32;
    let painted = Painted {
        own: "x".to_owned(),
        paint: None,
    };
    assert_eq!(written(&painted), doc! { "own": "x" });
    assert_eq!(
        read_counting!(Painted, painted_schema, written(&painted), calls),
        Ok(painted)
    );
    let reached = Reached {
        channel: None,
        own: "x".to_owned(),
    };
    assert_eq!(written(&reached), doc! { "own": "x" });
    assert_eq!(
        read_counting!(Reached, reached_schema, written(&reached), calls),
        Ok(reached)
    );
    assert_eq!(calls, 0);
}

/// With the keys of a struct flattened beside it in the document, an absent optional enum is
/// still absent: none of those keys names a variant of it.
#[test]
fn an_absent_flattened_optional_enum_beside_another_flattened_type_is_no_issue() {
    let layered = Layered {
        audit: audit(),
        channel: None,
        paint: None,
        title: "t".to_owned(),
    };
    assert_eq!(
        written(&layered),
        doc! { "createdBy": "ada", "revision": 1_i32, "title": "t" }
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Layered, layered_schema, written(&layered), calls),
        Ok(layered)
    );
    assert_eq!(calls, 0);
}

/// With the optional enum absent, the document's keys are the outer type's to judge: one nothing
/// declares is `Unknown`, a variant's key that serde reads no variant from among them.
#[test]
fn an_absent_flattened_optional_enum_leaves_the_documents_keys_to_the_outer_type() {
    let tagged = doc! { "legacy": true, "own": "x" };
    assert!(serde_reads::<Painted>(&tagged));
    assert_eq!(
        listed!(Painted, painted_schema, tagged),
        [unknown("legacy", Bson::Boolean(true))]
    );
    let untagged = doc! { "legacy": true, "own": "x" };
    assert!(serde_reads::<Reached>(&untagged));
    assert_eq!(
        listed!(Reached, reached_schema, untagged),
        [unknown("legacy", Bson::Boolean(true))]
    );
    let no_variant = doc! { "address": 5_i32, "own": "x" };
    assert!(serde_reads::<Reached>(&no_variant));
    assert_eq!(
        listed!(Reached, reached_schema, no_variant),
        [unknown("address", Bson::Int32(5))]
    );
}

/// With what names its variant in the document and serde reading it, an optional flattened enum
/// is walked as that variant, and lists what the walk finds at the document's path.
#[test]
fn a_flattened_optional_enum_that_is_there_is_walked_as_the_variant_it_names() {
    let mut calls = 0_u32;
    let painted = Painted {
        own: "x".to_owned(),
        paint: Some(Paint::Solid {
            color: "red".to_owned(),
        }),
    };
    assert_eq!(
        written(&painted),
        doc! { "own": "x", "kind": "Solid", "color": "red" }
    );
    assert_eq!(
        read_counting!(Painted, painted_schema, written(&painted), calls),
        Ok(painted)
    );
    let reached = Reached {
        channel: Some(Channel::Mail(ByMail {
            address: "a@b".to_owned(),
        })),
        own: "x".to_owned(),
    };
    assert_eq!(written(&reached), doc! { "address": "a@b", "own": "x" });
    assert_eq!(
        read_counting!(Reached, reached_schema, written(&reached), calls),
        Ok(reached)
    );
    assert_eq!(calls, 0);

    let inside = doc! {
        "kind": "Coated",
        "own": "x",
        "version": { "draft": true, "number": 3_i32 },
    };
    assert!(serde_reads::<Finished>(&inside));
    assert_eq!(
        listed!(Finished, finished_schema, inside),
        [unknown("version.draft", Bson::Boolean(true))]
    );
    let beside = doc! { "address": "a@b", "legacy": true, "own": "x" };
    assert!(serde_reads::<Reached>(&beside));
    assert_eq!(
        listed!(Reached, reached_schema, beside),
        [unknown("legacy", Bson::Boolean(true))]
    );
}

/// With its tag in the document and serde not reading the enum, the document holds what the field
/// would not write: the one issue is `Mistyped` at the document's own path.
#[test]
fn a_flattened_optional_enum_named_and_not_read_is_mistyped_at_the_document() {
    for stored_row in [
        doc! { "kind": "Striped", "own": "x" },
        doc! { "kind": "Solid", "own": "x" },
    ] {
        assert!(serde_reads::<Painted>(&stored_row));
        assert_eq!(
            listed!(Painted, painted_schema, stored_row.clone()),
            [mistyped(
                "",
                "Optional(Model(\"Paint\"))",
                Bson::Document(stored_row)
            )]
        );
    }
}

/// An optional externally tagged enum is absent when no key names a variant of it, as before.
#[test]
fn an_absent_flattened_optional_externally_tagged_enum_lists_nothing() {
    let mut calls = 0_u32;
    let absent = Edged {
        edge: None,
        own: "x".to_owned(),
    };
    assert_eq!(written(&absent), doc! { "own": "x" });
    assert_eq!(
        read_counting!(Edged, edged_schema, written(&absent), calls),
        Ok(absent)
    );
    let there = Edged {
        edge: Some(Edge::Curved { radius: 1.5_f64 }),
        own: "x".to_owned(),
    };
    assert_eq!(
        written(&there),
        doc! { "Curved": { "radius": 1.5_f64 }, "own": "x" }
    );
    assert_eq!(
        read_counting!(Edged, edged_schema, written(&there), calls),
        Ok(there)
    );
    assert_eq!(calls, 0);
    let undeclared = doc! { "legacy": true, "own": "x" };
    assert!(serde_reads::<Edged>(&undeclared));
    assert_eq!(
        listed!(Edged, edged_schema, undeclared),
        [unknown("legacy", Bson::Boolean(true))]
    );
}

/// An optional adjacently tagged enum is absent when its tag's key is: its content key alone
/// names no variant, and is the outer type's to judge.
#[test]
fn an_absent_flattened_optional_adjacently_tagged_enum_lists_nothing() {
    let mut calls = 0_u32;
    let absent = Trimmed {
        own: "x".to_owned(),
        trim: None,
    };
    assert_eq!(written(&absent), doc! { "own": "x" });
    assert_eq!(
        read_counting!(Trimmed, trimmed_schema, written(&absent), calls),
        Ok(absent)
    );
    let there = Trimmed {
        own: "x".to_owned(),
        trim: Some(Trim::Dotted { gap: 2_i32 }),
    };
    assert_eq!(
        written(&there),
        doc! { "own": "x", "kind": "Dotted", "data": { "gap": 2_i32 } }
    );
    assert_eq!(
        read_counting!(Trimmed, trimmed_schema, written(&there), calls),
        Ok(there)
    );
    assert_eq!(calls, 0);
    let content_alone = doc! { "data": { "gap": 2_i32 }, "own": "x" };
    assert!(serde_reads::<Trimmed>(&content_alone));
    assert_eq!(
        listed!(Trimmed, trimmed_schema, content_alone),
        [unknown("data", Bson::Document(doc! { "gap": 2_i32 }))]
    );
}

/// Each flagged type answers whether a document holds what names a value of it: a key of its own
/// or of a type it flattens for a struct, any key for one that flattens a map, the tag's key or a
/// variant's for a tagged enum, and a variant serde reads for an untagged one.
#[test]
fn each_flagged_type_answers_whether_a_document_names_a_value_of_it() {
    assert!(Audit::decode_with_bson_named(&doc! { "createdBy": 7_i32 }));
    assert!(!Audit::decode_with_bson_named(&doc! { "legacy": true }));
    assert!(Sheet::decode_with_bson_named(&doc! { "kind": "Clear" }));
    assert!(Sheet::decode_with_bson_named(&doc! { "note": "n" }));
    assert!(!Sheet::decode_with_bson_named(&doc! { "legacy": true }));
    assert!(Counts::decode_with_bson_named(&doc! { "legacy": true }));
    assert!(!Counts::decode_with_bson_named(&doc! {}));
    assert!(Paint::decode_with_bson_named(&doc! { "kind": 5_i32 }));
    assert!(!Paint::decode_with_bson_named(&doc! { "color": "red" }));
    assert!(Edge::decode_with_bson_named(
        &doc! { "Straight": Bson::Null }
    ));
    assert!(!Edge::decode_with_bson_named(&doc! { "radius": 1.5_f64 }));
    assert!(Trim::decode_with_bson_named(&doc! { "kind": "Solid" }));
    assert!(!Trim::decode_with_bson_named(&doc! { "data": {} }));
    assert!(Channel::decode_with_bson_named(&doc! { "address": "a@b" }));
    assert!(!Channel::decode_with_bson_named(&doc! { "address": 5_i32 }));
    assert!(!Channel::decode_with_bson_named(&doc! {}));
}

fn rev() -> Rev {
    Rev {
        revision: "r".to_owned(),
    }
}

fn counts() -> Counts {
    Counts {
        by_name: HashMap::from([("a".to_owned(), 1_i32)]),
        title: "t".to_owned(),
    }
}

fn mark() -> ObjectId {
    oid("6a7cc592ca0574e6efdfe217")
}

/// Holds both reads to serde over what serde writes for `value`, as a document and as a JSON
/// value: plain serde reads each, and `from_bson_with` and `from_value_with` give what it reads
/// and run no decider.
fn reads_what_serde_wrote<T>(value: &T)
where
    T: Flagged + PartialEq + Debug + Serialize + for<'de> Deserialize<'de>,
{
    let (row, json) = (written(value), serde_json::to_value(value).unwrap());
    let by_serde = (
        T::deserialize(bson::Deserializer::new(Bson::Document(row.clone()))).ok(),
        T::deserialize(&json).ok(),
    );
    assert!(by_serde.0.is_some() && by_serde.1.is_some(), "for {row}");
    let (from_row, from_json, calls) = T::read(row, json);
    assert_eq!((from_row, from_json), by_serde);
    assert_eq!(calls, 0);
}

/// A read hook that reads what the type's own reader reads, from whatever it is handed.
fn as_written<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer)
}

/// A flattened type declared after a flattened struct is handed none of that struct's keys, in a
/// struct, in a variant's fields, and where it is an `Option`.
#[test]
fn a_flattened_type_declared_after_a_flattened_struct_is_handed_none_of_its_keys() {
    let mut calls = 0_u32;
    let two = Two {
        audit: rev(),
        counts: counts(),
        id: "i".to_owned(),
    };
    assert_eq!(
        written(&two),
        doc! { "revision": "r", "a": 1_i32, "title": "t", "id": "i" }
    );
    assert_eq!(
        read_counting!(Two, two_schema, written(&two), calls),
        Ok(two)
    );

    let made = Booked::Made {
        audit: rev(),
        counts: counts(),
        id: "i".to_owned(),
    };
    assert_eq!(
        read_counting!(Booked, booked_schema, written(&made), calls),
        Ok(made)
    );

    for held in [Some(counts()), None] {
        let optioned = Optioned {
            audit: rev(),
            counts: held,
            id: "i".to_owned(),
        };
        assert_eq!(
            read_counting!(Optioned, optioned_schema, written(&optioned), calls),
            Ok(optioned)
        );
    }
    assert_eq!(calls, 0);
}

/// The type an internally tagged variant holds is handed the document without the tag's key.
#[test]
fn an_internally_tagged_variants_value_is_handed_the_document_without_the_tag() {
    let filled = Filled::Counted(counts());
    assert_eq!(
        written(&filled),
        doc! { "kind": "Counted", "a": 1_i32, "title": "t" }
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Filled, filled_schema, written(&filled), calls),
        Ok(filled)
    );
    assert_eq!(calls, 0);
}

/// A flattened single-slot struct over a map, over an `Option` of a struct and a flattened
/// generic brand filled with a struct each claim the keys serde reads for them.
#[test]
fn a_flattened_single_slot_struct_claims_the_keys_serde_reads_for_it() {
    let mut calls = 0_u32;
    let bag = Bag {
        extras: Extras(HashMap::from([("a".to_owned(), 1_i32)])),
        title: "t".to_owned(),
    };
    assert_eq!(written(&bag), doc! { "a": 1_i32, "title": "t" });
    assert_eq!(
        read_counting!(Bag, bag_schema, written(&bag), calls),
        Ok(bag)
    );

    let paper = Paper {
        newest: Newest(Some(Version { number: 3_i32 })),
        title: "t".to_owned(),
    };
    assert_eq!(written(&paper), doc! { "number": 3_i32, "title": "t" });
    assert_eq!(
        read_counting!(Paper, paper_schema, written(&paper), calls),
        Ok(paper)
    );

    let outer = Outer {
        body: Wrap(Version { number: 3_i32 }),
        id: "i".to_owned(),
    };
    assert_eq!(written(&outer), doc! { "number": 3_i32, "id": "i" });
    assert_eq!(
        read_counting!(Outer, outer_schema, written(&outer), calls),
        Ok(outer)
    );
    assert_eq!(calls, 0);
}

/// A flattened plain enum is the key naming its variant, holding `null`, and that key is its own:
/// flattened in a struct, and held by an internally tagged variant.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_flattened_plain_enum_claims_the_key_naming_its_variant() {
    let mut calls = 0_u32;
    let moody = Moody {
        mood: Mood::Calm,
        name: "a".to_owned(),
    };
    assert_eq!(written(&moody), doc! { "Calm": Bson::Null, "name": "a" });
    assert_eq!(
        read_counting!(Moody, moody_schema, written(&moody), calls),
        Ok(moody)
    );

    let slot = Slot::Held(Mood::Calm);
    assert_eq!(written(&slot), doc! { "kind": "Held", "Calm": Bson::Null });
    assert_eq!(
        read_counting!(Slot, slot_schema, written(&slot), calls),
        Ok(slot)
    );
    assert_eq!(calls, 0);
}

/// An issue that is there is still listed once, where it sits, in each of the shapes above.
#[test]
fn an_issue_in_a_type_handed_what_serde_hands_it_is_listed_once_where_it_sits() {
    let count_as_text = doc! { "a": "x", "id": "i", "revision": "r", "title": "t" };
    assert!(!serde_reads::<Two>(&count_as_text));
    assert_eq!(
        listed!(Two, two_schema, count_as_text),
        [invalid("a", "I32", string("x"))]
    );
    let revision_as_number = doc! { "a": 1_i32, "id": "i", "revision": 5_i32, "title": "t" };
    assert!(!serde_reads::<Two>(&revision_as_number));
    assert_eq!(
        listed!(Two, two_schema, revision_as_number),
        [invalid("revision", "String", Bson::Int32(5))]
    );

    let filled_count = doc! { "a": "x", "kind": "Counted", "title": "t" };
    assert!(!serde_reads::<Filled>(&filled_count));
    assert_eq!(
        listed!(Filled, filled_schema, filled_count),
        [invalid("a", "I32", string("x"))]
    );
    let unknown_tag = doc! { "a": 1_i32, "kind": "Emptied", "title": "t" };
    assert!(!serde_reads::<Filled>(&unknown_tag));
    assert_eq!(
        listed!(Filled, filled_schema, unknown_tag),
        [invalid(
            "kind",
            "Variants([\"Counted\"])",
            string("Emptied")
        )]
    );

    let bag_count = doc! { "a": "x", "title": "t" };
    assert!(!serde_reads::<Bag>(&bag_count));
    assert_eq!(
        listed!(Bag, bag_schema, bag_count),
        [invalid("a", "I32", string("x"))]
    );

    // serde reads the `Option` as absent, and the document holds what it would not write. The
    // issue holds what the single-slot struct was handed, which is all it has.
    let number_as_text = doc! { "number": "3", "title": "t" };
    assert!(serde_reads::<Paper>(&number_as_text));
    assert_eq!(
        listed!(Paper, paper_schema, number_as_text),
        [mistyped(
            "",
            "Optional(Model(\"Version\"))",
            Bson::Document(doc! { "number": "3" })
        )]
    );

    let outer_number = doc! { "id": "i", "number": "3" };
    assert!(!serde_reads::<Outer>(&outer_number));
    assert_eq!(
        listed!(Outer, outer_schema, outer_number),
        [invalid(
            "",
            "TypeParam(\"T\")",
            Bson::Document(doc! { "number": "3" })
        )]
    );
}

/// A flattened plain enum lists what an externally tagged enum lists for a variant that holds
/// nothing: nothing where a variant's key holds something other than `null`, which serde refuses,
/// and `Missing` where no key names a variant.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_flattened_plain_enum_lists_what_an_externally_tagged_enum_lists_for_a_unit_variant() {
    let not_null = doc! { "Calm": 5_i32, "name": "a" };
    assert!(!serde_reads::<Moody>(&not_null));
    assert_eq!(listed!(Moody, moody_schema, not_null), [undescribed()]);
    let no_variant_key = doc! { "name": "a" };
    assert!(!serde_reads::<Moody>(&no_variant_key));
    assert_eq!(
        listed!(Moody, moody_schema, no_variant_key),
        [missing("", "Variants([\"Calm\", \"Tense\"])")]
    );
}

/// A single-slot struct whose slot a hook reads is handed to the hook as serde hands it: it is
/// read whole, and every key is its own.
#[test]
fn a_flattened_single_slot_struct_with_a_hooked_slot_claims_what_its_hook_reads() {
    let mut calls = 0_u32;
    let franked = Franked {
        id: "i".to_owned(),
        postmark: Postmark(Version { number: 3_i32 }),
    };
    assert_eq!(written(&franked), doc! { "id": "i", "number": 3_i32 });
    assert_eq!(
        read_counting!(Franked, franked_schema, written(&franked), calls),
        Ok(franked)
    );
    assert_eq!(calls, 0);

    let number_as_text = doc! { "id": "i", "number": "3" };
    assert!(!serde_reads::<Franked>(&number_as_text));
    assert_eq!(
        listed!(Franked, franked_schema, number_as_text),
        [invalid(
            "",
            "Model(\"Version\")",
            Bson::Document(doc! { "number": "3" })
        )]
    );
}

/// A single-slot struct over an id is flattened as the object serde writes for an id, and the key
/// of that object is the struct's own, from a document and from a JSON value.
#[test]
fn a_flattened_single_slot_struct_over_an_id_claims_the_key_serde_reads_for_it() {
    let docket = Docket {
        marker: Marker(oid("6a7cc592ca0574e6efdfe217")),
        name: "n".to_owned(),
    };
    let row = written(&docket);
    assert_eq!(
        row,
        doc! { "$oid": "6a7cc592ca0574e6efdfe217", "name": "n" }
    );
    assert!(serde_reads::<Docket>(&row));
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Docket, docket_schema, row, calls).as_ref(),
        Ok(&docket)
    );
    let as_json = serde_json::to_value(&docket).unwrap();
    let from_json = Docket::from_value_with(as_json, |_raw, _found| {
        calls += 1;
        docket_schema::Verdict::Reject
    });
    assert_eq!(from_json, Ok(docket));
    assert_eq!(calls, 0);
}

/// A flattened id is read from the entry serde writes for it, from a document and from a JSON
/// value: alone, as an `Option` there and absent, through a hook, and where it fills a parameter.
#[test]
fn a_flattened_id_reads_what_serde_wrote_with_no_call() {
    let direct = DirectId {
        id: mark(),
        name: "n".to_owned(),
    };
    assert_eq!(
        written(&direct),
        doc! { "$oid": "6a7cc592ca0574e6efdfe217", "name": "n" }
    );
    reads_what_serde_wrote(&direct);
    reads_what_serde_wrote(&MaybeId {
        name: "n".to_owned(),
        oid: Some(mark()),
    });
    reads_what_serde_wrote(&MaybeId {
        name: "n".to_owned(),
        oid: None,
    });
    reads_what_serde_wrote(&IdThroughHook {
        name: "n".to_owned(),
        oid: mark(),
    });
    reads_what_serde_wrote(&Letter::<ObjectId> {
        body: mark(),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&Letter::<Option<ObjectId>> {
        body: Some(mark()),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&Letter::<Option<ObjectId>> {
        body: None,
        id: "i".to_owned(),
    });
}

/// A flattened `Option` in what fills a parameter, or read through a hook, is absent to serde
/// wherever the value does not read, so a record written with it absent is no issue. With it
/// there, it is read.
#[test]
fn a_flattened_option_not_seen_that_is_absent_is_no_issue() {
    reads_what_serde_wrote(&Letter::<Option<Body>> {
        body: None,
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&Letter::<Option<Body>> {
        body: Some(Body {
            text: "t".to_owned(),
        }),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&Pocket {
        body: Wrap(None),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&Pocket {
        body: Wrap(Some(Body {
            text: "t".to_owned(),
        })),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&DirectMaybe {
        id: "i".to_owned(),
        version: None,
    });
    reads_what_serde_wrote(&DirectMaybe {
        id: "i".to_owned(),
        version: Some(Version { number: 3_i32 }),
    });
}

/// serde reads an `Option` behind a hook or in what fills a parameter as absent where what it
/// holds is refused, and reads the document. Nothing here sees that `Option`, so nothing is listed.
#[test]
fn a_value_refused_inside_a_flattened_option_not_seen_is_read_as_absent() {
    let mut calls = 0_u32;
    let number_as_text = doc! { "id": "i", "number": "3" };
    assert!(serde_reads::<DirectMaybe>(&number_as_text));
    assert_eq!(
        read_counting!(DirectMaybe, direct_maybe_schema, number_as_text, calls),
        Ok(DirectMaybe {
            id: "i".to_owned(),
            version: None,
        })
    );
    let text_as_number = doc! { "id": "i", "text": 5_i32 };
    assert!(serde_reads::<Letter<Option<Body>>>(&text_as_number));
    assert_eq!(
        read_counting!(Letter<Option<Body>>, letter_schema, text_as_number, calls),
        Ok(Letter {
            body: None,
            id: "i".to_owned(),
        })
    );
    assert_eq!(calls, 0);
}

/// A value read whole that is no `Option` and that its reader refuses is listed at the document,
/// as serde refuses the document.
#[test]
fn a_flattened_value_read_whole_that_its_reader_refuses_is_invalid_at_the_document() {
    let number_as_text = doc! { "id": "i", "number": "3" };
    assert!(!serde_reads::<ThroughHook>(&number_as_text));
    assert_eq!(
        listed!(ThroughHook, through_hook_schema, number_as_text),
        [invalid(
            "",
            "Model(\"Version\")",
            Bson::Document(doc! { "number": "3" })
        )]
    );
}

/// Every other kind of flattened field reads what serde wrote for it with no call, from a
/// document and from a JSON value: a map and an `Option` of one, a value read whole through a
/// hook, a JSON value and an `Option` of one, a field serde writes and never reads, and an
/// `Option` of a parameter's type, there and absent.
#[test]
fn every_kind_of_flattened_field_reads_what_serde_wrote_with_no_call() {
    let entries = || HashMap::from([("a".to_owned(), 1_i32)]);
    reads_what_serde_wrote(&counts());
    reads_what_serde_wrote(&MaybeCounted {
        entries: Some(entries()),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&MaybeCounted {
        entries: None,
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&ThroughHook {
        id: "i".to_owned(),
        version: Version { number: 3_i32 },
    });
    reads_what_serde_wrote(&MapThroughHook {
        entries: entries(),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&HookAhead {
        ahead: Version { number: 3_i32 },
        counts: counts(),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&OpenEnded {
        id: "i".to_owned(),
        rest: serde_json::json!({ "a": 1_i32, "b": "x" }),
    });
    reads_what_serde_wrote(&MaybeOpenEnded {
        id: "i".to_owned(),
        rest: Some(serde_json::json!({ "a": 1_i32 })),
    });
    reads_what_serde_wrote(&MaybeOpenEnded {
        id: "i".to_owned(),
        rest: None,
    });
    reads_what_serde_wrote(&NeverRead {
        id: "i".to_owned(),
        version: Some(Version { number: 3_i32 }),
    });
    reads_what_serde_wrote(&Unpicked {
        draft: Draft { pages: 3_i32 },
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&Tucked::<Body> {
        body: Some(Body {
            text: "t".to_owned(),
        }),
        id: "i".to_owned(),
    });
    reads_what_serde_wrote(&Tucked::<Body> {
        body: None,
        id: "i".to_owned(),
    });
}
