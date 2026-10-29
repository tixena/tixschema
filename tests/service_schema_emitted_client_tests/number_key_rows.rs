//! The models a map keyed by a brand over a number or a flag, or by an alias of a number, is held
//! in, and the payloads each client is held to serde on: a key serde refuses is refused wherever
//! the map sits, and a number or a flag written between quotes as a value is refused.

use alloc::collections::BTreeMap;
use core::cmp::Ordering;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The members of `Moorings`, each with the text of a map serde reads and the text of its first
/// key replaced by `x`.
const MOORINGS: [(&str, &str, &str); 13] = [
    ("beacon", r#"{"false":1,"true":2}"#, r#"{"x":1,"true":2}"#),
    ("burgee", r#"{"1":1,"2":2}"#, r#"{"x":1,"2":2}"#),
    ("cutter", r#"{"1":1,"65535":2}"#, r#"{"x":1,"65535":2}"#),
    ("dinghy", r#"{"-1":1,"2":2}"#, r#"{"x":1,"2":2}"#),
    ("hull", r#"{"1":1,"2":2}"#, r#"{"x":1,"2":2}"#),
    ("ketch", r#"{"-1":1,"2":2}"#, r#"{"x":1,"2":2}"#),
    ("lanyard", r#"{"1":1,"2":2}"#, r#"{"x":1,"2":2}"#),
    ("pennant", r#"{"1":1,"2":2}"#, r#"{"x":1,"2":2}"#),
    ("rudder", r#"{"1":1,"2":2}"#, r#"{"x":1,"2":2}"#),
    ("signal", r#"{"false":1,"true":2}"#, r#"{"x":1,"true":2}"#),
    ("skiff", r#"{"-1":1,"2":2}"#, r#"{"x":1,"2":2}"#),
    ("sloop", r#"{"1":1,"255":2}"#, r#"{"x":1,"255":2}"#),
    (
        "yawl",
        r#"{"-9007199254740993":1,"9007199254740993":2}"#,
        r#"{"x":1,"2":2}"#,
    ),
];

const SOUND_BURGEE: &str = r#"{"1":1,"2":2}"#;
const SOUND_SIGNAL: &str = r#"{"false":1,"true":2}"#;
const BROKEN_BURGEE: &str = r#"{"x":1}"#;
const BROKEN_SIGNAL: &str = r#"{"True":1}"#;

/// A brand over an `i16` a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Dinghy(pub i16);

/// A brand over a `u8` a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Sloop(pub u8);

/// A brand over a `u16` a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Cutter(pub u16);

/// A brand over a `u32` a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Burgee(pub u32);

/// A brand over an `i8` a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Skiff(pub i8);

/// A brand over an `i32` a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Ketch(pub i32);

/// A brand over an `i64` a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Yawl(pub i64);

/// A brand over an `f64` a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Gauge(pub f64);

impl Eq for Gauge {}

impl PartialOrd for Gauge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Gauge {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// A brand over a flag a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Signal(pub bool);

/// A brand over a brand over a number a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Lanyard(pub Burgee);

/// An alias of a number a map is keyed by.
#[model_schema()]
pub type Hull = u32;

/// An alias of a flag a map is keyed by.
#[model_schema()]
pub type Beacon = bool;

/// A brand over an alias of a number a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Rudder(pub Hull);

/// An alias of a brand over a number a map is keyed by.
#[model_schema()]
pub type Pennant = Burgee;

/// A map under each brand and alias, as a field.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Moorings {
    pub beacon: BTreeMap<Beacon, u8>,
    pub burgee: BTreeMap<Burgee, u8>,
    pub cutter: BTreeMap<Cutter, u8>,
    pub dinghy: BTreeMap<Dinghy, u8>,
    pub hull: BTreeMap<Hull, u8>,
    pub ketch: BTreeMap<Ketch, u8>,
    pub lanyard: BTreeMap<Lanyard, u8>,
    pub pennant: BTreeMap<Pennant, u8>,
    pub rudder: BTreeMap<Rudder, u8>,
    pub signal: BTreeMap<Signal, u8>,
    pub skiff: BTreeMap<Skiff, u8>,
    pub sloop: BTreeMap<Sloop, u8>,
    pub yawl: BTreeMap<Yawl, u8>,
}

/// A map under a brand over an `f64`, as a field.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Soundings {
    pub gauge: BTreeMap<Gauge, u8>,
}

/// A list, an optional and a map of maps, over a brand over an `f64`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Depths {
    pub listed: Vec<BTreeMap<Gauge, u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<BTreeMap<Gauge, u8>>,
    pub nested: BTreeMap<Hull, BTreeMap<Gauge, u8>>,
}

/// A variant that holds a map under a brand over an `f64`, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Shoal {
    Charted { gauge: BTreeMap<Gauge, u8> },
    Marked(BTreeMap<Gauge, u8>),
}

/// A list, an optional and a map of maps, over a brand and an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Slips {
    pub listed: Vec<BTreeMap<Burgee, u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<BTreeMap<Hull, u8>>,
    pub nested: BTreeMap<Hull, BTreeMap<Signal, u8>>,
}

/// A variant that holds a map under a brand, a flag and an alias, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Pier {
    Docked {
        burgee: BTreeMap<Burgee, u8>,
        signal: BTreeMap<Signal, u8>,
    },
    Moored(BTreeMap<Hull, u8>),
}

/// [`Pier`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentPier {
    Docked {
        burgee: BTreeMap<Burgee, u8>,
        signal: BTreeMap<Signal, u8>,
    },
    Moored(BTreeMap<Hull, u8>),
}

/// The same brands and alias as values, which a quoted number or flag is refused under.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Anchor {
    pub burgee: Burgee,
    pub hull: Hull,
    pub lanyard: Lanyard,
    pub signal: Signal,
}

/// One payload, the type a client run reads it as, and what serde answers of it.
pub struct Row {
    pub json: String,
    pub named: &'static str,
    pub serde: String,
}

/// What serde answers of `json` read as `Held`: the value it reads, written back, or `refused`.
fn serde_told<Held>(json: &str) -> String
where
    Held: DeserializeOwned + Serialize,
{
    serde_json::from_str::<Held>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| format!("read {}", serde_json::to_string(&read).unwrap()),
    )
}

fn row<Held>(named: &'static str, json: &str) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    Row {
        json: json.to_owned(),
        named,
        serde: serde_told::<Held>(json),
    }
}

/// A `Moorings` payload whose member `broken` reads its refused text.
fn moorings_json(broken: &str) -> String {
    let members: Vec<String> = MOORINGS
        .iter()
        .map(|&(name, sound, refused)| {
            format!(
                r#""{name}":{}"#,
                if name == broken { refused } else { sound }
            )
        })
        .collect();
    format!("{{{}}}", members.join(","))
}

fn slips_json(listed: &str, maybe: &str, nested: &str) -> String {
    format!(r#"{{"listed":[{listed}],"maybe":{maybe},"nested":{nested}}}"#)
}

/// Every payload of the number brands and aliases, as a field, in a slot, as a variant's member,
/// and as a value.
pub fn rows() -> Vec<Row> {
    let mut rows = vec![row::<Moorings>("Moorings", &moorings_json(""))];
    for &(name, _, _) in &MOORINGS {
        rows.push(row::<Moorings>("Moorings", &moorings_json(name)));
    }
    rows.extend([
        row::<Moorings>(
            "Moorings",
            &moorings_json("").replace(r#""burgee":{"1":1,"2":2}"#, r#""burgee":{}"#),
        ),
        row::<Moorings>(
            "Moorings",
            &moorings_json("").replace(r#""burgee":{"1":1,"2":2}"#, r#""burgee":{"-1":1}"#),
        ),
        row::<Moorings>(
            "Moorings",
            &moorings_json("").replace(r#""burgee":{"1":1,"2":2}"#, r#""burgee":{"":1}"#),
        ),
        row::<Moorings>(
            "Moorings",
            &moorings_json("").replace(r#""skiff":{"-1":1,"2":2}"#, r#""skiff":{"300":1}"#),
        ),
        row::<Soundings>("Soundings", r#"{"gauge":{"1.5":1,"2.5":2}}"#),
        row::<Soundings>("Soundings", r#"{"gauge":{"x":1}}"#),
        row::<Soundings>("Soundings", r#"{"gauge":{"1.0":1,"2.5":2}}"#),
        row::<Soundings>("Soundings", r#"{"gauge":{"1e2":1}}"#),
        row::<Soundings>("Soundings", r#"{"gauge":{"":1}}"#),
        row::<Soundings>("Soundings", r#"{"gauge":{"-0.5":1,"2":2}}"#),
    ]);
    let gauged = r#"{"1.5":1,"2.5":2}"#;
    let depths = |listed: &str, maybe: &str, nested: &str| {
        format!(r#"{{"listed":[{listed}],"maybe":{maybe},"nested":{nested}}}"#)
    };
    for json in [
        depths(gauged, gauged, &format!(r#"{{"1":{gauged}}}"#)),
        depths("", "null", "{}"),
        depths(r#"{"x":1}"#, gauged, "{}"),
        depths(gauged, r#"{"x":1}"#, "{}"),
        depths(gauged, gauged, r#"{"1":{"x":1}}"#),
    ] {
        rows.push(row::<Depths>("Depths", &json));
    }
    for json in [
        format!(r#"{{"Charted":{{"gauge":{gauged}}}}}"#),
        r#"{"Charted":{"gauge":{"x":1}}}"#.to_owned(),
        format!(r#"{{"Marked":{gauged}}}"#),
        r#"{"Marked":{"x":1}}"#.to_owned(),
    ] {
        rows.push(row::<Shoal>("Shoal", &json));
    }
    let nested = r#"{"1":{"false":1,"true":2}}"#;
    rows.extend([
        row::<Slips>("Slips", &slips_json(SOUND_BURGEE, SOUND_BURGEE, nested)),
        row::<Slips>("Slips", &slips_json("", "null", "{}")),
        row::<Slips>("Slips", &slips_json(BROKEN_BURGEE, SOUND_BURGEE, nested)),
        row::<Slips>("Slips", &slips_json(SOUND_BURGEE, BROKEN_BURGEE, nested)),
        row::<Slips>(
            "Slips",
            &slips_json(SOUND_BURGEE, SOUND_BURGEE, r#"{"x":{"true":2}}"#),
        ),
        row::<Slips>(
            "Slips",
            &slips_json(
                SOUND_BURGEE,
                SOUND_BURGEE,
                &format!(r#"{{"1":{BROKEN_SIGNAL}}}"#),
            ),
        ),
    ]);
    pier_rows::<Pier>(
        &mut rows,
        "Pier",
        |burgee, signal| format!(r#"{{"Docked":{{"burgee":{burgee},"signal":{signal}}}}}"#),
        |hull| format!(r#"{{"Moored":{hull}}}"#),
    );
    pier_rows::<AdjacentPier>(
        &mut rows,
        "AdjacentPier",
        |burgee, signal| {
            format!(r#"{{"kind":"Docked","body":{{"burgee":{burgee},"signal":{signal}}}}}"#)
        },
        |hull| format!(r#"{{"kind":"Moored","body":{hull}}}"#),
    );
    let anchor = |burgee: &str, hull: &str, lanyard: &str, signal: &str| {
        format!(r#"{{"burgee":{burgee},"hull":{hull},"lanyard":{lanyard},"signal":{signal}}}"#)
    };
    for json in [
        anchor("1", "2", "3", "true"),
        anchor(r#""1""#, "2", "3", "true"),
        anchor("1", r#""2""#, "3", "true"),
        anchor("1", "2", r#""3""#, "true"),
        anchor("1", "2", "3", r#""true""#),
    ] {
        rows.push(row::<Anchor>("Anchor", &json));
    }
    rows
}

/// The float keys serde refuses that Swift's own float parse accepts.
#[cfg(feature = "swift")]
pub fn float_texts_serde_refuses() -> Vec<Row> {
    [
        r#"{"gauge":{"+1":1}}"#,
        r#"{"gauge":{".5":1}}"#,
        r#"{"gauge":{"0x10":1}}"#,
        r#"{"gauge":{"1e999":1}}"#,
    ]
    .into_iter()
    .map(|json| row::<Soundings>("Soundings", json))
    .collect()
}

/// The payloads of a variant under one tagging: each member sound, each broken, and the tuple
/// variant sound and broken.
fn pier_rows<Held>(
    rows: &mut Vec<Row>,
    named: &'static str,
    docked: fn(&str, &str) -> String,
    moored: fn(&str) -> String,
) where
    Held: DeserializeOwned + Serialize,
{
    rows.extend([
        row::<Held>(named, &docked(SOUND_BURGEE, SOUND_SIGNAL)),
        row::<Held>(named, &docked(BROKEN_BURGEE, SOUND_SIGNAL)),
        row::<Held>(named, &docked(SOUND_BURGEE, BROKEN_SIGNAL)),
        row::<Held>(named, &moored(SOUND_BURGEE)),
        row::<Held>(named, &moored(BROKEN_BURGEE)),
    ]);
}
