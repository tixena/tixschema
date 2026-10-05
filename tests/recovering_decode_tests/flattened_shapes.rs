//! What serde reads and leaves when it reads a type flattened, shape by shape, and the walker held
//! to it: a flattened type is handed what serde hands it where it is declared, and the keys it
//! answers as its own are the keys serde reads for it. The tests named `serde_…` run plain serde
//! alone: they are the statement of what serde does.

use alloc::collections::BTreeMap;
use core::fmt::Debug;
use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

// The JSON schema of a type holding a `Version` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use super::version_schema;
use super::{Version, lines};

/// What `from_value_with` lists for `$stored`, read as `$model` by a decider that rejects it.
macro_rules! listed {
    ($model:ty, $module:ident, $stored:expr) => {
        lines(
            &<$model>::from_value_with($stored, |_raw, _found| $module::Verdict::Reject)
                .unwrap_err(),
        )
    };
}

/// What `from_value_with` reads `$stored` as, by a decider that counts each run of its own into
/// `$calls`.
macro_rules! read_counting {
    ($model:ty, $module:ident, $stored:expr, $calls:ident) => {
        <$model>::from_value_with($stored, |_raw, _found| {
            $calls += 1;
            $module::Verdict::Reject
        })
    };
}

/// Implements [`Flagged`] for each `$model`, whose callback types `$module` holds.
macro_rules! flagged {
    ($($model:ty => $module:ident,)+) => {$(
        impl Flagged for $model {
            fn read(stored: Value) -> (Option<Self>, u32) {
                let mut calls = 0_u32;
                let read = read_counting!($model, $module, stored, calls);
                (read.ok(), calls)
            }
        }
    )+};
}

/// A flagged type as [`agrees`] reads it.
trait Flagged: Sized {
    /// What `from_value_with` reads `stored` as under a decider that rejects it, and how many
    /// times that decider ran.
    fn read(stored: Value) -> (Option<Self>, u32);
}

/// Takes every key it is handed, whatever it holds: what serde left for a type flattened where
/// this one is.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Rest {
    #[serde(flatten)]
    entries: BTreeMap<String, Value>,
}

/// Two flattened fields beside a key of the type's own. serde reads them in the order declared.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Flat<A, B> {
    #[serde(flatten)]
    before: A,
    id: String,
    #[serde(flatten)]
    later: B,
}

/// A flattened map of numbers beside a key of text: it takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Counts {
    #[serde(flatten)]
    by_name: HashMap<String, i32>,
    title: String,
}

/// A struct with a named field, held as text so a map of numbers handed its key refuses it.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Rev {
    revision: String,
}

/// A struct that itself flattens a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Notes {
    #[serde(flatten)]
    by_key: HashMap<String, i32>,
    name: String,
}

/// A plain enum: flattened, serde writes the variant's name as a key holding `null`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Mood {
    Calm,
    Tense,
}

/// Externally tagged: `{"Curved": {"radius": 1.5}}`, `{"Straight": null}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Border {
    Curved { radius: f64 },
    Straight,
}

/// Internally tagged: `{"kind": "Solid", "color": "red"}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Tint {
    Clear,
    Solid { color: String },
}

/// Adjacently tagged: `{"rim": "Dotted", "data": {"gap": 2}}`, `{"rim": "Plain"}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "rim", content = "data")]
enum Rim {
    Dotted { gap: i32 },
    Plain,
}

/// Untagged, each variant an object with a key of its own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Via {
    Mail { address: String },
    Pager { number: i32 },
}

/// A single-slot struct over a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct RevRef(Rev);

/// A single-slot struct over a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Extras(HashMap<String, i32>);

/// A single-slot struct over an `Option` of a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Newest(Option<Version>);

/// A generic brand: serde reads it as whatever fills it.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Wrap<T>(T);

/// A tuple struct of two slots, a single-slot struct over text, over a list and over a tuple:
/// serde refuses to flatten each.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Duo(String, u32);

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Word(String);

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Row(Vec<i32>);

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Spot((String, u32));

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

/// A flattened single-slot struct over a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Bag {
    #[serde(flatten)]
    extras: Extras,
    title: String,
}

/// A flattened single-slot struct over an `Option` of a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Paper {
    #[serde(flatten)]
    newest: Newest,
    title: String,
}

/// A flattened generic brand filled with a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Outer {
    #[serde(flatten)]
    body: Wrap<Version>,
    id: String,
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

/// A single-slot struct over an `Option` of a struct whose key holds text.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct NewestRev(Option<Rev>);

// Each shape serde flattens, flattened before `Counts` in one type and after it in another: serde
// reads the two flattened fields in the order declared.

/// `Counts` flattened after a struct with named fields.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct RevFirst {
    #[serde(flatten)]
    before: Rev,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before a struct with named fields.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct RevLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Rev,
}

/// `Counts` flattened after a struct that itself flattens a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct NotesFirst {
    #[serde(flatten)]
    before: Notes,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before a struct that itself flattens a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct NotesLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Notes,
}

/// `Counts` flattened after an externally tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BorderFirst {
    #[serde(flatten)]
    before: Border,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before an externally tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BorderLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Border,
}

/// `Counts` flattened after an internally tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct TintFirst {
    #[serde(flatten)]
    before: Tint,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before an internally tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct TintLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Tint,
}

/// `Counts` flattened after an adjacently tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct RimFirst {
    #[serde(flatten)]
    before: Rim,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before an adjacently tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct RimLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Rim,
}

/// `Counts` flattened after an untagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ViaFirst {
    #[serde(flatten)]
    before: Via,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before an untagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ViaLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Via,
}

/// `Counts` flattened after a single-slot struct over a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct RevRefFirst {
    #[serde(flatten)]
    before: RevRef,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before a single-slot struct over a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct RevRefLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: RevRef,
}

/// `Counts` flattened after a single-slot struct over a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ExtrasFirst {
    #[serde(flatten)]
    before: Extras,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before a single-slot struct over a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ExtrasLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Extras,
}

/// `Counts` flattened after a single-slot struct over an `Option` of a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct NewestRevFirst {
    #[serde(flatten)]
    before: NewestRev,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before a single-slot struct over an `Option` of a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct NewestRevLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: NewestRev,
}

/// `Counts` flattened after a generic brand filled with a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct WrapFirst {
    #[serde(flatten)]
    before: Wrap<Rev>,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before a generic brand filled with a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct WrapLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Wrap<Rev>,
}

/// `Counts` flattened after an `Option` of a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MaybeRevFirst {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    before: Option<Rev>,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before an `Option` of a struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MaybeRevLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    later: Option<Rev>,
}

/// `Counts` flattened after an `Option` of an internally tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MaybeTintFirst {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    before: Option<Tint>,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before an `Option` of an internally tagged enum.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MaybeTintLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    later: Option<Tint>,
}

/// `Counts` flattened after a plain enum, which a schema surface refuses to see flattened.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MoodFirst {
    #[serde(flatten)]
    before: Mood,
    id: String,
    #[serde(flatten)]
    later: Counts,
}

/// `Counts` flattened before a plain enum, which a schema surface refuses to see flattened.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MoodLast {
    #[serde(flatten)]
    before: Counts,
    id: String,
    #[serde(flatten)]
    later: Mood,
}

/// A `#[serde(transparent)]` struct over a type that takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct CountsRef {
    inner: Counts,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Ledgered {
    #[serde(flatten)]
    counts: CountsRef,
    id: String,
}

/// Untagged, with a variant that holds a type that takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Tallied {
    Counted(Counts),
    Named { name: String },
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Scored {
    id: String,
    #[serde(flatten)]
    tallied: Tallied,
}

/// Untagged, with a variant that holds a plain enum. A schema surface refuses the declaration
/// that flattens it.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Feeling {
    Mood(Mood),
    Named { name: String },
}

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Felt {
    #[serde(flatten)]
    feeling: Feeling,
    id: String,
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

/// Externally tagged, with a variant that holds a type that takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Boxed {
    Counted(Counts),
    Empty,
}

/// Adjacently tagged, with a variant that holds a type that takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Crated {
    Counted(Counts),
    Empty,
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

/// Two flattened structs, neither of which takes every key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Signed {
    #[serde(flatten)]
    audit: Rev,
    id: String,
    #[serde(flatten)]
    version: Version,
}

/// A single-slot struct over an `Option` of a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MaybeExtras(Option<HashMap<String, i32>>);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Pouch {
    #[serde(flatten)]
    extras: MaybeExtras,
    title: String,
}

/// A single-slot struct over whatever a JSON value holds.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Anything(Value);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Crate {
    #[serde(flatten)]
    anything: Anything,
    title: String,
}

/// A flattened generic brand filled with a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Basket {
    #[serde(flatten)]
    body: Wrap<HashMap<String, i32>>,
    id: String,
}

/// A model type that carries no flag: it only ever fills a parameter.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Plain {
    text: String,
}

/// A flattened type parameter, then a flattened type that takes every key it is handed.
#[model_schema(decode_with, default_types(T = Plain))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Parcel<T> {
    #[serde(flatten)]
    body: T,
    #[serde(flatten)]
    counts: Counts,
    id: String,
}

/// A flattened generic brand filled with a struct, then a flattened type that takes every key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Wrapping {
    #[serde(flatten)]
    body: Wrap<Plain>,
    #[serde(flatten)]
    counts: Counts,
    id: String,
}

/// An optional flattened type parameter, which no walk reaches, then a flattened map.
#[model_schema(decode_with, default_types(T = Plain))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Bundle<T> {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    body: Option<T>,
    #[serde(flatten)]
    counts: HashMap<String, i32>,
    id: String,
}

/// A single-slot struct over an `Option` of an `Option` of a struct, the inner one in a `Box`,
/// which serde reads through.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Deep(Option<Box<Option<Version>>>);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Deeper {
    #[serde(flatten)]
    deep: Deep,
    name: String,
}

/// A single-slot struct whose slot a hook reads: serde hands the hook what the struct is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Postmark(#[serde(deserialize_with = "as_written")] Version);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Posted {
    id: String,
    #[serde(flatten)]
    postmark: Postmark,
}

/// The hooked slot, then a flattened type that takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct PostedFirst {
    #[serde(flatten)]
    brand: Postmark,
    #[serde(flatten)]
    counts: Counts,
    id: String,
}

/// A single-slot struct over a map a hook reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Chalk(#[serde(deserialize_with = "as_written")] HashMap<String, i32>);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Slate {
    #[serde(flatten)]
    chalk: Chalk,
    id: String,
}

/// A `#[serde(transparent)]` struct whose field a hook reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Wax {
    #[serde(deserialize_with = "as_written")]
    inner: Version,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Waxed {
    id: String,
    #[serde(flatten)]
    wax: Wax,
}

/// A single-slot struct over an `Option` a hook reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Perhaps(#[serde(deserialize_with = "as_written")] Option<Version>);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Guess {
    id: String,
    #[serde(flatten)]
    perhaps: Perhaps,
}

flagged! {
    RevFirst => rev_first_schema,
    RevLast => rev_last_schema,
    NotesFirst => notes_first_schema,
    NotesLast => notes_last_schema,
    BorderFirst => border_first_schema,
    BorderLast => border_last_schema,
    TintFirst => tint_first_schema,
    TintLast => tint_last_schema,
    RimFirst => rim_first_schema,
    RimLast => rim_last_schema,
    ViaFirst => via_first_schema,
    ViaLast => via_last_schema,
    RevRefFirst => rev_ref_first_schema,
    RevRefLast => rev_ref_last_schema,
    ExtrasFirst => extras_first_schema,
    ExtrasLast => extras_last_schema,
    NewestRevFirst => newest_rev_first_schema,
    NewestRevLast => newest_rev_last_schema,
    WrapFirst => wrap_first_schema,
    WrapLast => wrap_last_schema,
    MaybeRevFirst => maybe_rev_first_schema,
    MaybeRevLast => maybe_rev_last_schema,
    MaybeTintFirst => maybe_tint_first_schema,
    MaybeTintLast => maybe_tint_last_schema,
    CountsRef => counts_ref_schema,
    Ledgered => ledgered_schema,
    Tallied => tallied_schema,
    Scored => scored_schema,
    Booked => booked_schema,
    Boxed => boxed_schema,
    Crated => crated_schema,
    Optioned => optioned_schema,
    Pouch => pouch_schema,
    Crate => crate_schema,
    Basket => basket_schema,
    Parcel<Plain> => parcel_schema,
    Wrapping => wrapping_schema,
    Bundle<Plain> => bundle_schema,
    Posted => posted_schema,
    PostedFirst => posted_first_schema,
    Slate => slate_schema,
    Waxed => waxed_schema,
    Guess => guess_schema,
    Deeper => deeper_schema,
}

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
flagged! {
    MoodFirst => mood_first_schema,
    MoodLast => mood_last_schema,
    Felt => felt_schema,
}

/// A read hook that reads what the type's own reader reads, from whatever it is handed.
fn as_written<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer)
}

fn counts() -> Counts {
    Counts {
        by_name: HashMap::from([("a".to_owned(), 1_i32)]),
        title: "t".to_owned(),
    }
}

fn rev() -> Rev {
    Rev {
        revision: "r".to_owned(),
    }
}

/// Holds the walker to serde over `stored`, a record serde writes for a `T`. Where serde reads
/// it, it writes the same record back, and `from_value_with` gives the value serde read and runs
/// no decider; where serde refuses it, `from_value_with` hands the decider its list once.
/// Answers whether serde read it.
fn agrees<T>(stored: &Value) -> bool
where
    T: Flagged + PartialEq + Debug + Serialize + for<'de> Deserialize<'de>,
{
    let (read, calls) = T::read(stored.clone());
    let by_serde = T::deserialize(stored).ok();
    if let Some(value) = &by_serde {
        assert_eq!(&serde_json::to_value(value).unwrap(), stored);
    }
    assert_eq!(read, by_serde, "for {stored}");
    assert_eq!(calls, u32::from(by_serde.is_none()), "for {stored}");
    by_serde.is_some()
}

/// What plain serde says of `stored`, read as `T`.
fn serde_reads<T>(stored: &Value) -> bool
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(stored).is_ok()
}

/// What serde left of `stored` for a type flattened after `S`, and what it handed one flattened
/// before `S`: each time the entries a type that takes every key was read from.
fn serde_left<S>(stored: &Value) -> (BTreeMap<String, Value>, BTreeMap<String, Value>)
where
    S: for<'de> Deserialize<'de>,
{
    let after = Flat::<S, Rest>::deserialize(stored).unwrap();
    let before = Flat::<Rest, S>::deserialize(stored).unwrap();
    (after.later.entries, before.before.entries)
}

/// The entries of `stored`, an object written in a test.
fn entries(stored: &Value) -> BTreeMap<String, Value> {
    stored
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, held)| (key.clone(), held.clone()))
        .collect()
}

/// What the walker is told serde leaves of `stored`, less the key the outer type reads, for a
/// type flattened after `S`, and whether a copy was made for it: what `S`'s own reader asks for
/// says which entries serde takes.
fn walker_left<S>(stored: &Value) -> (BTreeMap<String, Value>, bool)
where
    S: for<'de> Deserialize<'de>,
{
    let mut rest = stored.as_object().unwrap().clone();
    rest.remove("id");
    let left = counts_schema::value_left(S::deserialize, &rest);
    let copied = left.is_some();
    (left.unwrap_or(rest).into_iter().collect(), copied)
}

/// Holds what the walker is told serde leaves of `stored` after `S` to what serde leaves.
fn told_what_serde_leaves<S>(stored: &Value)
where
    S: for<'de> Deserialize<'de>,
{
    assert_eq!(walker_left::<S>(stored).0, serde_left::<S>(stored).0);
}

/// serde reads a struct with named fields flattened by taking the entries under its keys out of
/// what is left, so a type flattened after it is handed none of them. One flattened before it is
/// handed them all. A single-slot struct, a generic brand, an `Option` and a `Box` over the
/// struct are read as it is.
#[test]
fn serde_takes_the_keys_of_a_flattened_struct_out_of_what_is_left() {
    let stored = json!({ "id": "i", "revision": "r", "x": 1_i32 });
    let left = (
        entries(&json!({ "x": 1_i32 })),
        entries(&json!({ "revision": "r", "x": 1_i32 })),
    );
    assert_eq!(serde_left::<Rev>(&stored), left);
    assert_eq!(serde_left::<RevRef>(&stored), left);
    assert_eq!(serde_left::<NewestRev>(&stored), left);
    assert_eq!(serde_left::<Wrap<Rev>>(&stored), left);
    assert_eq!(serde_left::<Option<Rev>>(&stored), left);
    assert_eq!(serde_left::<Box<Rev>>(&stored), left);
}

/// serde reads a map, a struct that itself flattens a field, an internally tagged enum and an
/// untagged enum from what is left and takes nothing out of it: a type flattened after one is
/// handed the same entries, the keys the first one read among them.
#[test]
fn serde_takes_nothing_for_a_flattened_map_a_struct_that_flattens_and_an_untagged_or_internally_tagged_enum()
 {
    let both = |left: &Value| (entries(left), entries(left));
    assert_eq!(
        serde_left::<Notes>(&json!({ "id": "i", "name": "n", "x": 1_i32 })),
        both(&json!({ "name": "n", "x": 1_i32 }))
    );
    assert_eq!(
        serde_left::<Extras>(&json!({ "id": "i", "x": 1_i32 })),
        both(&json!({ "x": 1_i32 }))
    );
    let solid = json!({ "color": "red", "id": "i", "kind": "Solid", "x": 1_i32 });
    let left = both(&json!({ "color": "red", "kind": "Solid", "x": 1_i32 }));
    assert_eq!(serde_left::<Tint>(&solid), left);
    assert_eq!(serde_left::<Option<Tint>>(&solid), left);
    assert_eq!(serde_left::<Wrap<Tint>>(&solid), left);
    assert_eq!(
        serde_left::<Via>(&json!({ "address": "a@b", "id": "i", "x": 1_i32 })),
        both(&json!({ "address": "a@b", "x": 1_i32 }))
    );
}

/// serde reads a plain enum and an externally tagged one flattened by taking the one entry whose
/// key names a variant, and an adjacently tagged one by taking its tag and its content.
#[test]
fn serde_takes_the_key_naming_a_variant_and_an_adjacent_tag_and_content() {
    let after = entries(&json!({ "x": 1_i32 }));
    let calm = json!({ "Calm": null, "id": "i", "x": 1_i32 });
    let before_calm = entries(&json!({ "Calm": null, "x": 1_i32 }));
    assert_eq!(
        serde_left::<Mood>(&calm),
        (after.clone(), before_calm.clone())
    );
    assert_eq!(
        serde_left::<Option<Mood>>(&calm),
        (after.clone(), before_calm)
    );
    assert_eq!(
        serde_left::<Border>(&json!({ "Curved": { "radius": 1.5_f64 }, "id": "i", "x": 1_i32 })),
        (
            after.clone(),
            entries(&json!({ "Curved": { "radius": 1.5_f64 }, "x": 1_i32 }))
        )
    );
    assert_eq!(
        serde_left::<Rim>(
            &json!({ "data": { "gap": 2_i32 }, "id": "i", "rim": "Dotted", "x": 1_i32 })
        ),
        (
            after,
            entries(&json!({ "data": { "gap": 2_i32 }, "rim": "Dotted", "x": 1_i32 }))
        )
    );
}

/// An `Option` serde reads as absent takes nothing.
#[test]
fn serde_takes_nothing_for_an_absent_flattened_option() {
    let stored = json!({ "id": "i", "x": 1_i32 });
    let left = (
        entries(&json!({ "x": 1_i32 })),
        entries(&json!({ "x": 1_i32 })),
    );
    assert_eq!(serde_left::<Option<Rev>>(&stored), left);
    assert_eq!(serde_left::<Option<Tint>>(&stored), left);
    assert_eq!(serde_left::<Option<Mood>>(&stored), left);
    assert_eq!(serde_left::<NewestRev>(&stored), left);
}

/// serde refuses to flatten a tuple struct of two slots and a single-slot struct over text, over
/// a list or over a tuple, wherever it is declared.
#[test]
fn serde_refuses_to_flatten_a_tuple_struct_and_a_slot_over_text_a_list_or_a_tuple() {
    let stored = json!({ "id": "i", "x": 1_i32 });
    let refusals = [
        Flat::<Duo, Rest>::deserialize(&stored).unwrap_err(),
        Flat::<Rest, Duo>::deserialize(&stored).unwrap_err(),
        Flat::<Word, Rest>::deserialize(&stored).unwrap_err(),
        Flat::<Rest, Word>::deserialize(&stored).unwrap_err(),
        Flat::<Row, Rest>::deserialize(&stored).unwrap_err(),
        Flat::<Rest, Row>::deserialize(&stored).unwrap_err(),
        Flat::<Spot, Rest>::deserialize(&stored).unwrap_err(),
        Flat::<Rest, Spot>::deserialize(&stored).unwrap_err(),
    ];
    for refusal in refusals {
        assert_eq!(refusal.to_string(), "can only flatten structs and maps");
    }
}

/// serde hands an internally tagged variant's value the object without the tag's entry.
#[test]
fn serde_hands_an_internally_tagged_variants_value_the_object_without_the_tag() {
    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    enum Tagged {
        Open(Rest),
    }

    let read = Tagged::deserialize(json!({ "kind": "Open", "x": 1_i32 })).unwrap();
    assert_eq!(
        read,
        Tagged::Open(Rest {
            entries: entries(&json!({ "x": 1_i32 })),
        })
    );
}

/// What the walker is told serde leaves is what serde leaves, for every shape: each type's own
/// reader says which entries serde takes for it.
#[test]
fn the_walker_is_told_what_serde_leaves_by_each_types_own_reader() {
    let keyed = json!({ "id": "i", "revision": "r", "x": 1_i32 });
    told_what_serde_leaves::<Rev>(&keyed);
    told_what_serde_leaves::<RevRef>(&keyed);
    told_what_serde_leaves::<NewestRev>(&keyed);
    told_what_serde_leaves::<Wrap<Rev>>(&keyed);
    told_what_serde_leaves::<Option<Rev>>(&keyed);
    told_what_serde_leaves::<Box<Rev>>(&keyed);
    told_what_serde_leaves::<Notes>(&json!({ "id": "i", "name": "n", "x": 1_i32 }));
    let open = json!({ "id": "i", "x": 1_i32 });
    told_what_serde_leaves::<Extras>(&open);
    told_what_serde_leaves::<Option<Rev>>(&open);
    told_what_serde_leaves::<NewestRev>(&open);
    told_what_serde_leaves::<HashMap<String, Value>>(&open);
    told_what_serde_leaves::<Value>(&open);
    let calm = json!({ "Calm": null, "id": "i", "x": 1_i32 });
    told_what_serde_leaves::<Mood>(&calm);
    told_what_serde_leaves::<Option<Mood>>(&calm);
    told_what_serde_leaves::<Border>(
        &json!({ "Curved": { "radius": 1.5_f64 }, "id": "i", "x": 1_i32 }),
    );
    let solid = json!({ "color": "red", "id": "i", "kind": "Solid", "x": 1_i32 });
    told_what_serde_leaves::<Tint>(&solid);
    told_what_serde_leaves::<Option<Tint>>(&solid);
    told_what_serde_leaves::<Rim>(
        &json!({ "data": { "gap": 2_i32 }, "id": "i", "rim": "Dotted", "x": 1_i32 }),
    );
    told_what_serde_leaves::<Via>(&json!({ "address": "a@b", "id": "i", "x": 1_i32 }));
}

/// Where serde takes nothing, what is left is the object itself, and no copy is made of it.
#[test]
fn no_copy_is_made_of_what_is_left_where_serde_takes_nothing() {
    let named = json!({ "id": "i", "name": "n", "x": 1_i32 });
    assert!(!walker_left::<Notes>(&named).1);
    assert!(!walker_left::<Extras>(&named).1);
    assert!(!walker_left::<Option<Rev>>(&named).1);
    let solid = json!({ "color": "red", "id": "i", "kind": "Solid", "x": 1_i32 });
    assert!(!walker_left::<Tint>(&solid).1);
    assert!(walker_left::<Rev>(&json!({ "id": "i", "revision": "r", "x": 1_i32 })).1);
}

/// A flattened type declared after a flattened struct is handed none of that struct's keys.
#[test]
fn a_flattened_type_declared_after_a_flattened_struct_is_handed_none_of_its_keys() {
    let two = Two {
        audit: rev(),
        counts: counts(),
        id: "i".to_owned(),
    };
    let written = serde_json::to_value(&two).unwrap();
    assert_eq!(
        written,
        json!({ "a": 1_i32, "id": "i", "revision": "r", "title": "t" })
    );
    let mut calls = 0_u32;
    assert_eq!(read_counting!(Two, two_schema, written, calls), Ok(two));
    assert_eq!(calls, 0);
}

/// The type an internally tagged variant holds is handed the object without the tag's key.
#[test]
fn an_internally_tagged_variants_value_is_handed_the_object_without_the_tag() {
    let filled = Filled::Counted(counts());
    let written = serde_json::to_value(&filled).unwrap();
    assert_eq!(
        written,
        json!({ "a": 1_i32, "kind": "Counted", "title": "t" })
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Filled, filled_schema, written, calls),
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
    let bag_written = serde_json::to_value(&bag).unwrap();
    assert_eq!(bag_written, json!({ "a": 1_i32, "title": "t" }));
    assert_eq!(read_counting!(Bag, bag_schema, bag_written, calls), Ok(bag));

    let paper = Paper {
        newest: Newest(Some(Version { number: 3_i32 })),
        title: "t".to_owned(),
    };
    let paper_written = serde_json::to_value(&paper).unwrap();
    assert_eq!(paper_written, json!({ "number": 3_i32, "title": "t" }));
    assert_eq!(
        read_counting!(Paper, paper_schema, paper_written, calls),
        Ok(paper)
    );

    let outer = Outer {
        body: Wrap(Version { number: 3_i32 }),
        id: "i".to_owned(),
    };
    let outer_written = serde_json::to_value(&outer).unwrap();
    assert_eq!(outer_written, json!({ "id": "i", "number": 3_i32 }));
    assert_eq!(
        read_counting!(Outer, outer_schema, outer_written, calls),
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
    let moody_written = serde_json::to_value(&moody).unwrap();
    assert_eq!(moody_written, json!({ "Calm": null, "name": "a" }));
    assert_eq!(
        read_counting!(Moody, moody_schema, moody_written, calls),
        Ok(moody)
    );

    let slot = Slot::Held(Mood::Calm);
    let slot_written = serde_json::to_value(&slot).unwrap();
    assert_eq!(slot_written, json!({ "Calm": null, "kind": "Held" }));
    assert_eq!(
        read_counting!(Slot, slot_schema, slot_written, calls),
        Ok(slot)
    );
    assert_eq!(calls, 0);
}

/// A shape serde reads by taking its keys, flattened before a type that takes every key, leaves
/// that type none of them: the walker reads what serde reads. Flattened after it, the keys are
/// handed to that type first, which cannot hold them, so serde refuses the record and so does the
/// walker.
#[test]
fn a_shape_that_takes_its_keys_reads_before_a_type_that_takes_every_key_and_not_after_it() {
    let keyed = json!({ "a": 1_i32, "id": "i", "revision": "r", "title": "t" });
    assert!(agrees::<RevFirst>(&keyed));
    assert!(!agrees::<RevLast>(&keyed));
    assert!(agrees::<RevRefFirst>(&keyed));
    assert!(!agrees::<RevRefLast>(&keyed));
    assert!(agrees::<NewestRevFirst>(&keyed));
    assert!(!agrees::<NewestRevLast>(&keyed));
    assert!(agrees::<WrapFirst>(&keyed));
    assert!(!agrees::<WrapLast>(&keyed));
    assert!(agrees::<MaybeRevFirst>(&keyed));
    assert!(!agrees::<MaybeRevLast>(&keyed));

    let curved = json!({ "Curved": { "radius": 1.5_f64 }, "a": 1_i32, "id": "i", "title": "t" });
    assert!(agrees::<BorderFirst>(&curved));
    assert!(!agrees::<BorderLast>(&curved));
    let dotted = json!({
        "a": 1_i32,
        "data": { "gap": 2_i32 },
        "id": "i",
        "rim": "Dotted",
        "title": "t",
    });
    assert!(agrees::<RimFirst>(&dotted));
    assert!(!agrees::<RimLast>(&dotted));
}

/// A shape serde reads without taking anything leaves its keys for a type that takes every key,
/// in either order: serde reads the record where that type can hold them, refuses it where it
/// cannot, and the walker does the same.
#[test]
fn a_shape_that_takes_nothing_shares_its_keys_with_a_type_that_takes_every_key() {
    // A variant's key held as a number is one the map of numbers reads as well.
    let pager = json!({ "a": 1_i32, "id": "i", "number": 7_i32, "title": "t" });
    assert!(agrees::<ViaFirst>(&pager));
    assert!(agrees::<ViaLast>(&pager));
    let mail = json!({ "a": 1_i32, "address": "a@b", "id": "i", "title": "t" });
    assert!(!agrees::<ViaFirst>(&mail));
    assert!(!agrees::<ViaLast>(&mail));

    let solid = json!({ "a": 1_i32, "color": "red", "id": "i", "kind": "Solid", "title": "t" });
    assert!(!agrees::<TintFirst>(&solid));
    assert!(!agrees::<TintLast>(&solid));
    assert!(!agrees::<MaybeTintFirst>(&solid));
    assert!(!agrees::<MaybeTintLast>(&solid));

    // Each of the two takes every key, and neither can hold the other's key of text.
    let named = json!({ "a": 1_i32, "id": "i", "n": 2_i32, "name": "x", "title": "t" });
    assert!(!agrees::<NotesFirst>(&named));
    assert!(!agrees::<NotesLast>(&named));
    let counted = json!({ "a": 1_i32, "id": "i", "title": "t", "x": 2_i32 });
    assert!(!agrees::<ExtrasFirst>(&counted));
    assert!(!agrees::<ExtrasLast>(&counted));
}

/// An `Option` serde reads as absent takes nothing and is handed nothing it reads: the record
/// reads in either order.
#[test]
fn an_absent_optional_shape_reads_before_and_after_a_type_that_takes_every_key() {
    let absent = json!({ "a": 1_i32, "id": "i", "title": "t" });
    assert!(agrees::<MaybeRevFirst>(&absent));
    assert!(agrees::<MaybeRevLast>(&absent));
    assert!(agrees::<MaybeTintFirst>(&absent));
    assert!(agrees::<MaybeTintLast>(&absent));
    assert!(agrees::<NewestRevFirst>(&absent));
    assert!(agrees::<NewestRevLast>(&absent));
}

/// A plain enum flattened before a type that takes every key gives up the key naming its
/// variant, and flattened after it leaves that key, held as `null`, for the first to refuse.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_plain_enum_flattened_before_and_after_a_type_that_takes_every_key_reads_as_serde_reads_it() {
    let calm = json!({ "Calm": null, "a": 1_i32, "id": "i", "title": "t" });
    assert!(agrees::<MoodFirst>(&calm));
    assert!(!agrees::<MoodLast>(&calm));
}

/// Every other place a type is handed an object its caller reads keys from hands it what serde
/// hands it there: a type that takes every key reads what serde wrote, with no call.
#[test]
fn every_place_a_type_is_handed_an_object_hands_it_what_serde_hands_it() {
    let held = json!({ "a": 1_i32, "title": "t" });
    assert!(agrees::<CountsRef>(&held));
    assert!(agrees::<Tallied>(&held));
    assert!(agrees::<Boxed>(&json!({ "Counted": held })));
    assert!(agrees::<Crated>(
        &json!({ "data": held, "kind": "Counted" })
    ));
    let keyed = json!({ "a": 1_i32, "id": "i", "title": "t" });
    assert!(agrees::<Ledgered>(&keyed));
    assert!(agrees::<Scored>(&keyed));

    let made = json!({ "a": 1_i32, "id": "i", "kind": "Made", "revision": "r", "title": "t" });
    assert!(agrees::<Booked>(&made));
    let there = json!({ "a": 1_i32, "id": "i", "revision": "r", "title": "t" });
    assert!(agrees::<Optioned>(&there));
    assert!(agrees::<Optioned>(&json!({ "id": "i", "revision": "r" })));
}

/// An untagged variant that holds a plain enum, flattened: the key naming the plain enum's
/// variant is its own.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_flattened_untagged_variant_over_a_plain_enum_claims_the_key_naming_its_variant() {
    let felt = Felt {
        feeling: Feeling::Mood(Mood::Calm),
        id: "i".to_owned(),
    };
    let written = serde_json::to_value(&felt).unwrap();
    assert_eq!(written, json!({ "Calm": null, "id": "i" }));
    assert!(agrees::<Felt>(&written));
}

/// An issue in a type handed what serde hands it is listed once, where it sits: a count held as
/// text at its key, and a key of the struct declared first held as the wrong type at that key.
#[test]
fn an_issue_in_a_type_handed_what_serde_hands_it_is_listed_once_where_it_sits() {
    let count_as_text = json!({ "a": "x", "id": "i", "revision": "r", "title": "t" });
    assert!(!serde_reads::<Two>(&count_as_text));
    assert_eq!(
        listed!(Two, two_schema, count_as_text),
        ["a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"]
    );
    let revision_as_number = json!({ "a": 1_i32, "id": "i", "revision": 5_i32, "title": "t" });
    assert!(!serde_reads::<Two>(&revision_as_number));
    assert_eq!(
        listed!(Two, two_schema, revision_as_number),
        [
            "revision: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string"
        ]
    );

    let filled_count = json!({ "a": "x", "kind": "Counted", "title": "t" });
    assert!(!serde_reads::<Filled>(&filled_count));
    assert_eq!(
        listed!(Filled, filled_schema, filled_count),
        ["a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"]
    );
    let unknown_tag = json!({ "a": 1_i32, "kind": "Emptied", "title": "t" });
    assert!(!serde_reads::<Filled>(&unknown_tag));
    assert_eq!(
        listed!(Filled, filled_schema, unknown_tag),
        [
            "kind: invalid: expected Variants([\"Counted\"]), found String(\"Emptied\"): unknown variant `Emptied`, expected `Counted`"
        ]
    );
}

/// An issue in what a flattened single-slot struct holds is what the type that flattens that
/// value itself lists for it. The issue of an `Option` serde reads as absent holds what the
/// single-slot struct was handed, which is all it has.
#[test]
fn an_issue_in_a_flattened_single_slot_struct_is_what_a_flattened_field_of_its_value_lists() {
    let bag_count = json!({ "a": "x", "title": "t" });
    assert!(!serde_reads::<Bag>(&bag_count));
    assert_eq!(
        listed!(Bag, bag_schema, bag_count),
        ["a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"]
    );

    let number_as_text = json!({ "number": "3", "title": "t" });
    assert_eq!(
        Paper::deserialize(&number_as_text).unwrap(),
        Paper {
            newest: Newest(None),
            title: "t".to_owned(),
        }
    );
    assert_eq!(
        listed!(Paper, paper_schema, number_as_text),
        [
            "the value itself: mistyped: expected Optional(Model(\"Version\")), found Object {\"number\": String(\"3\")}"
        ]
    );

    let outer_number = json!({ "id": "i", "number": "3" });
    assert!(!serde_reads::<Outer>(&outer_number));
    assert_eq!(
        listed!(Outer, outer_schema, outer_number),
        [
            "the value itself: invalid: expected TypeParam(\"T\"), found Object {\"number\": String(\"3\")}: invalid type: string \"3\", expected i32"
        ]
    );
}

/// A flattened plain enum lists what an externally tagged enum lists for a variant that holds
/// nothing: nothing where a variant's key holds something other than `null`, which serde refuses,
/// and `Missing` where no key names a variant.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_flattened_plain_enum_lists_what_an_externally_tagged_enum_lists_for_a_unit_variant() {
    let not_null = json!({ "Calm": 5_i32, "name": "a" });
    let refusal = Moody::deserialize(&not_null).unwrap_err().to_string();
    assert_eq!(
        listed!(Moody, moody_schema, not_null),
        [format!("undescribed: {refusal}")]
    );
    let no_variant = json!({ "name": "a" });
    assert!(!serde_reads::<Moody>(&no_variant));
    assert_eq!(
        listed!(Moody, moody_schema, no_variant),
        ["the value itself: missing: expected Variants([\"Calm\", \"Tense\"])"]
    );
}

/// A key nothing declares, beside two flattened structs neither of which takes every key, is
/// `Unknown` once, at the outer object's path.
#[test]
fn a_key_no_flattened_struct_declares_is_unknown_once_beside_two_of_them() {
    let stored = json!({ "id": "i", "legacy": true, "number": 3_i32, "revision": "r" });
    assert!(serde_reads::<Signed>(&stored));
    assert_eq!(
        listed!(Signed, signed_schema, stored),
        ["legacy: unknown: found Bool(true)"]
    );
}

/// More single-slot structs serde flattens claim what serde reads for them: one over an `Option`
/// of a map, one over a JSON value, a generic brand filled with a map, and one over an `Option`
/// of an `Option` of a struct.
#[test]
fn a_flattened_single_slot_struct_over_an_optional_map_or_a_json_value_claims_what_serde_reads() {
    let counted = json!({ "a": 1_i32, "title": "t" });
    assert!(agrees::<Pouch>(&counted));
    assert!(agrees::<Pouch>(&json!({ "title": "t" })));
    assert!(agrees::<Crate>(
        &json!({ "a": 1_i32, "b": "x", "title": "t" })
    ));
    assert!(agrees::<Basket>(&json!({ "a": 1_i32, "id": "i" })));
    assert!(agrees::<Deeper>(&json!({ "name": "n", "number": 3_i32 })));
    assert!(agrees::<Deeper>(&json!({ "name": "n" })));
}

/// A type parameter, flattened as a field or inside a generic brand, gives up to the flattened
/// type declared after it the keys serde takes for whatever fills it. A flattened map declared
/// after an optional one is walked, and handed none of its keys.
#[test]
fn a_flattened_type_declared_after_a_flattened_type_parameter_is_handed_what_its_filling_left() {
    let texted = json!({ "a": 1_i32, "id": "i", "text": "x", "title": "t" });
    assert!(agrees::<Parcel<Plain>>(&texted));
    assert!(agrees::<Wrapping>(&texted));

    let bundled = json!({ "a": 1_i32, "id": "i", "text": "x" });
    assert!(agrees::<Bundle<Plain>>(&bundled));
    assert!(agrees::<Bundle<Plain>>(&json!({ "a": 1_i32, "id": "i" })));
    let count_as_text = json!({ "a": "x", "id": "i", "text": "x" });
    assert!(!serde_reads::<Bundle<Plain>>(&count_as_text));
    assert_eq!(
        listed!(Bundle<Plain>, bundle_schema, count_as_text),
        ["a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"]
    );
}

/// A single-slot struct whose slot a hook reads is handed to the hook as serde hands it: it is
/// read whole, and every key is its own. Over an `Option`, serde reads it as absent where the hook
/// does, and nothing is listed for it.
#[test]
fn a_flattened_single_slot_struct_with_a_hooked_slot_claims_what_its_hook_reads() {
    let numbered = json!({ "id": "i", "number": 3_i32 });
    assert!(agrees::<Posted>(&numbered));
    assert!(agrees::<Waxed>(&numbered));
    assert!(agrees::<Guess>(&numbered));
    assert!(agrees::<Guess>(&json!({ "id": "i" })));
    assert!(agrees::<Slate>(&json!({ "a": 1_i32, "id": "i" })));
    assert!(agrees::<PostedFirst>(
        &json!({ "a": 1_i32, "id": "i", "number": 3_i32, "title": "t" })
    ));

    let number_as_text = json!({ "id": "i", "number": "3" });
    assert!(!serde_reads::<Posted>(&number_as_text));
    assert_eq!(
        listed!(Posted, posted_schema, number_as_text),
        [
            "the value itself: invalid: expected Model(\"Version\"), found Object {\"number\": String(\"3\")}: invalid type: string \"3\", expected i32"
        ]
    );
}
