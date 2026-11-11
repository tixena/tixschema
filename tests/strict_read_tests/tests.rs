//! Each case of what serde does with `deny_unknown_fields`, read through `#[model_schema]` types.

use alloc::collections::BTreeMap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "load")]
pub enum Adjacent {
    Load { weight: u32 },
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Base {
    pub id: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Cached {
    #[serde(skip)]
    pub cache: u32,
    pub name: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Compass {
    #[serde(flatten)]
    pub directions: BTreeMap<String, u32>,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Declared {
    pub name: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub enum External {
    Square { side: u32 },
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Holder {
    #[serde(flatten)]
    pub base: Base,
    pub name: String,
}

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
pub struct LenientHolder {
    #[serde(flatten)]
    pub base: Base,
    pub name: String,
}

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "side")]
pub enum LenientSide {
    Load { weight: u32 },
}

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
pub struct LenientPing;

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
pub struct LenientRole {
    pub name: String,
}

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum LenientShape {
    Square { side: u32 },
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Opaque {
    #[serde(flatten)]
    pub below: Below,
    pub name: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Below {
    pub id: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Ping;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Report {
    pub entities: Vec<Role>,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Role {
    #[serde(default, rename = "minOccurs", skip_serializing_if = "Option::is_none")]
    pub min_occurs: Option<u32>,
    pub name: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "side")]
pub enum Side {
    Load { weight: u32 },
    Unload { reason: String },
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum SideHolder {
    Load {
        #[serde(flatten)]
        base: Base,
        weight: u32,
    },
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Titled {
    #[serde(alias = "label")]
    pub title: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Nothing {
    #[serde(skip)]
    pub held: u8,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Counter {
    #[serde(skip, default = "seven")]
    pub start: u32,
    #[serde(skip)]
    pub total: u32,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    #[serde(skip)]
    pub label: String,
    #[serde(skip)]
    pub retries: u32,
    #[serde(skip, default = "seven")]
    pub start: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            label: "main".to_owned(),
            retries: 3,
            start: 0,
        }
    }
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Memo<T: Default> {
    #[serde(skip)]
    pub held: T,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(bound(deserialize = ""))]
pub struct Shelf<T> {
    #[serde(skip)]
    pub items: Vec<T>,
}

/// A type with no `Default`, which `Shelf` holds under the bound its author wrote.
pub struct Unmade;

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug)]
pub struct LenientNothing {
    #[serde(skip)]
    pub held: u8,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct Unread {
    #[serde(skip_deserializing)]
    pub computed: u32,
    pub name: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Shape {
    Circle { radius: u32 },
    Square { side: u32 },
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum Tide {
    Ebbing { height: u32 },
    Slack,
}

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum LenientTide {
    Ebbing { height: u32 },
    Slack,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Mooring {
    pub berth: String,
    #[serde(flatten)]
    pub tide: Tide,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum Gauge {
    Sounded { depth: u32 },
}

/// A variant that flattens a member reading every key keeps the whole enum open: `nonstrict`, as
/// the build asks, it reads past a key beside its tag and content key on every level.
#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "form", content = "body")]
pub enum Reading {
    Gauged {
        station: String,
        #[serde(flatten)]
        gauge: Gauge,
    },
    Missing,
}

fn seven() -> u32 {
    7
}

fn read<T>(json: &str) -> Result<(), String>
where
    T: DeserializeOwned,
{
    let mut deserializer = serde_json::Deserializer::from_str(json);
    serde_path_to_error::deserialize::<_, T>(&mut deserializer)
        .map(drop)
        .map_err(|failure| format!("{}|{}", failure.path(), failure.inner()))
}

#[test]
fn a_flattened_map_owns_every_other_key() {
    assert_eq!(read::<Compass>(r#"{"north":1,"south":2}"#), Ok(()));
}

#[test]
fn a_key_under_a_flattened_struct_is_refused_and_a_good_payload_reads() {
    let refused = read::<Holder>(r#"{"name":"a","id":"b","stray":1}"#).unwrap_err();
    assert!(refused.starts_with(".|unknown field `stray`"), "{refused}");
    assert_eq!(read::<Holder>(r#"{"name":"a","id":"b"}"#), Ok(()));
}

#[test]
fn a_key_under_a_tag_alone_is_refused_beside_a_flattened_struct_too() {
    let refused = read::<Side>(r#"{"side":"Load","weight":1,"stray":1}"#).unwrap_err();
    assert!(refused.starts_with(".|unknown field `stray`"), "{refused}");
    let flattened =
        read::<SideHolder>(r#"{"kind":"Load","id":"b","weight":1,"stray":1}"#).unwrap_err();
    assert!(
        flattened.starts_with(".|unknown field `stray`"),
        "{flattened}"
    );
    assert_eq!(read::<Side>(r#"{"side":"Unload","reason":"a"}"#), Ok(()));
}

#[test]
fn a_key_under_an_adjacent_tag_is_refused_inside_the_content() {
    let refused = read::<Adjacent>(r#"{"kind":"Load","load":{"weight":1,"stray":1}}"#).unwrap_err();
    assert!(
        refused.starts_with("load.stray|unknown field `stray`"),
        "{refused}"
    );
}

#[test]
fn a_key_under_an_external_variant_is_refused_at_its_path() {
    let refused = read::<External>(r#"{"Square":{"side":1,"stray":1}}"#).unwrap_err();
    assert!(
        refused.starts_with("Square.stray|unknown field `stray`"),
        "{refused}"
    );
}

#[test]
fn a_key_no_variant_of_an_untagged_enum_reads_is_refused() {
    let refused = read::<Shape>(r#"{"side":1,"stray":1}"#).unwrap_err();
    assert_eq!(
        refused,
        ".|data did not match any variant of untagged enum Shape"
    );
    assert_eq!(read::<Shape>(r#"{"radius":1}"#), Ok(()));
}

#[test]
fn a_key_an_alias_names_reads_and_the_key_of_a_skipped_member_is_refused() {
    assert_eq!(read::<Titled>(r#"{"label":"a"}"#), Ok(()));
    let refused = read::<Cached>(r#"{"name":"a","cache":1}"#).unwrap_err();
    assert!(
        refused.starts_with("cache|unknown field `cache`"),
        "{refused}"
    );
    let kept: Cached = serde_json::from_str(r#"{"name":"a"}"#).unwrap();
    assert_eq!((kept.name.as_str(), kept.cache), ("a", 0));
}

#[test]
fn a_misspelled_key_in_a_list_element_is_refused_at_its_path() {
    let refused =
        read::<Report>(r#"{"entities":[{"name":"a"},{"name":"b","minOccur":1}]}"#).unwrap_err();
    assert!(
        refused.starts_with("entities[1].minOccur|unknown field `minOccur`"),
        "{refused}"
    );
    assert_eq!(
        read::<Report>(r#"{"entities":[{"name":"a","minOccurs":2}]}"#),
        Ok(())
    );
}

#[test]
fn a_nonstrict_type_reads_past_a_key_no_member_declares() {
    assert_eq!(read::<LenientRole>(r#"{"name":"a","stray":1}"#), Ok(()));
    assert_eq!(
        read::<LenientHolder>(r#"{"name":"a","id":"b","stray":1}"#),
        Ok(())
    );
    assert_eq!(
        read::<LenientSide>(r#"{"side":"Load","weight":1,"stray":1}"#),
        Ok(())
    );
    assert_eq!(read::<LenientShape>(r#"{"side":1,"stray":1}"#), Ok(()));
}

/// A struct declared below its holder takes its own keys, so the holder refuses a stray key.
#[test]
fn a_type_that_flattens_a_struct_declared_below_it_refuses_a_key_neither_declares() {
    assert_eq!(read::<Opaque>(r#"{"name":"a","id":"b"}"#), Ok(()));
    let refused = read::<Opaque>(r#"{"name":"a","id":"b","stray":1}"#).unwrap_err();
    assert!(refused.contains("unknown field `stray`"), "{refused}");
}

#[test]
fn a_type_that_wrote_the_attribute_itself_reads_as_it_did() {
    let refused = read::<Declared>(r#"{"name":"a","stray":1}"#).unwrap_err();
    assert!(
        refused.starts_with("stray|unknown field `stray`"),
        "{refused}"
    );
    assert_eq!(read::<Declared>(r#"{"name":"a"}"#), Ok(()));
}

#[test]
fn a_unit_struct_refuses_a_key_and_a_nonstrict_one_reads_past_it() {
    assert_eq!(read::<Ping>("{}"), Ok(()));
    let refused = read::<Ping>(r#"{"a":1}"#).unwrap_err();
    assert!(refused.contains("unknown field `a`"), "{refused}");
    assert_eq!(read::<LenientPing>("{}"), Ok(()));
    assert_eq!(read::<LenientPing>(r#"{"a":1}"#), Ok(()));
}

#[test]
fn a_member_serde_writes_and_never_reads_keeps_its_own_writes_readable() {
    let written = serde_json::to_string(&Unread {
        computed: 4,
        name: "a".to_owned(),
    })
    .unwrap();
    assert_eq!(written, r#"{"computed":4,"name":"a"}"#);
    let read: Unread = serde_json::from_str(&written).unwrap();
    assert_eq!((read.computed, read.name.as_str()), (0, "a"));
}

#[test]
fn a_struct_with_no_member_serde_reads_refuses_a_key_and_reads_an_empty_object() {
    let refused = read::<Nothing>(r#"{"stray":1}"#).unwrap_err();
    assert!(
        refused.contains("unknown field `stray`, there are no fields"),
        "{refused}"
    );
    let empty = serde_json::from_str::<Nothing>("{}").unwrap();
    assert_eq!(empty.held, 0);
    let lenient = serde_json::from_str::<LenientNothing>(r#"{"stray":1}"#).unwrap();
    assert_eq!(lenient.held, 0);
}

#[test]
fn a_struct_with_no_member_serde_reads_builds_each_member_from_its_default() {
    let counter = serde_json::from_str::<Counter>("{}").unwrap();
    assert_eq!((counter.start, counter.total), (7, 0));
    let settings = serde_json::from_str::<Settings>("{}").unwrap();
    assert_eq!(
        (settings.label.as_str(), settings.retries, settings.start),
        ("main", 3, 7)
    );
    for refused in [
        read::<Counter>(r#"{"start":1}"#),
        read::<Settings>(r#"{"stray":1}"#),
    ] {
        assert!(
            refused
                .as_ref()
                .is_err_and(|told| told.contains("unknown field")),
            "{refused:?}"
        );
    }
}

#[test]
fn a_generic_struct_with_no_member_serde_reads_refuses_a_key_under_the_bounds_serde_asks() {
    let memo = serde_json::from_str::<Memo<String>>("{}").unwrap();
    assert_eq!(memo.held, "");
    let shelf = serde_json::from_str::<Shelf<Unmade>>("{}").unwrap();
    assert!(shelf.items.is_empty());
    for refused in [
        read::<Memo<u32>>(r#"{"stray":1}"#),
        read::<Shelf<Unmade>>(r#"{"stray":1}"#),
    ] {
        assert!(
            refused
                .as_ref()
                .is_err_and(|told| told.contains("unknown field `stray`")),
            "{refused:?}"
        );
    }
}

#[test]
fn the_unit_variant_of_a_strict_enum_under_a_tag_alone_refuses_a_key_beside_the_tag() {
    assert_eq!(
        serde_json::from_str::<Tide>(r#"{"kind":"Slack"}"#).unwrap(),
        Tide::Slack
    );
    let refused = serde_json::from_str::<Tide>(r#"{"kind":"Slack","stray":1}"#)
        .unwrap_err()
        .to_string();
    assert!(
        refused.starts_with("unknown field `stray`, there are no fields"),
        "{refused}"
    );
    assert_eq!(
        serde_json::from_str::<LenientTide>(r#"{"kind":"Slack","stray":1}"#).unwrap(),
        LenientTide::Slack
    );
}

#[test]
fn the_unit_variant_flattened_last_refuses_a_key_its_holder_leaves_and_reads_its_own_back() {
    let moored = Mooring {
        berth: "north".to_owned(),
        tide: Tide::Slack,
    };
    let written = serde_json::to_string(&moored).unwrap();
    assert_eq!(written, r#"{"berth":"north","kind":"Slack"}"#);
    assert_eq!(serde_json::from_str::<Mooring>(&written).unwrap(), moored);
    let refused = serde_json::from_str::<Mooring>(r#"{"berth":"north","kind":"Slack","stray":1}"#)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("unknown field `stray`"), "{refused}");
}

#[test]
fn a_nonstrict_enum_whose_variant_flattens_a_tagged_enum_reads_back_and_past_a_key() {
    for reading in [
        Reading::Gauged {
            station: "pier".to_owned(),
            gauge: Gauge::Sounded { depth: 4 },
        },
        Reading::Missing,
    ] {
        let written = serde_json::to_string(&reading).unwrap();
        assert_eq!(serde_json::from_str::<Reading>(&written).unwrap(), reading);
    }
    assert_eq!(
        serde_json::from_str::<Reading>(r#"{"form":"Missing","stray":1}"#).unwrap(),
        Reading::Missing
    );
}
