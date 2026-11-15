//! A holder reads what fills a type parameter of a generic model it holds with no judgement and
//! judges the whole value once, for every shape of the generic model and of what fills it: the
//! emitted Swift read of a holder of each shape answers the messages the Rust read answers, each
//! broken value named at its full path.

#![cfg(feature = "swift")]

use super::runtime::judged;
use alloc::collections::BTreeMap;
use core::fmt::Write as _;
use core::ops::Range;
use parameter_models_apart::{
    ApartPage, ApartParcel, ApartSlots, ApartTupled, apart_page_swift, apart_parcel_swift,
    apart_slots_swift, apart_tupled_swift,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// A parcel that keeps every rule.
const SOUND: &str = r#"{"itemLabel":"a"}"#;

/// The functions the models name, in Swift.
const SWIFT_FUNCTIONS: &str = "
func tidied(_ payload: Any) -> Any { payload }
";

/// A parcel that breaks its bound.
const BROKEN: &str = r#"{"itemLabel":""}"#;

/// A model with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShParcel {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

/// A generic struct with a rule of its own.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShPage<Item> {
    pub items: Vec<Item>,
    #[model_schema_prop(minLength = 1)]
    pub page_title: String,
}

/// A member that reaches the parameter through a tuple.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShTupled<Item> {
    pub pair: (Item, u32),
}

/// A member that reaches the parameter through a range.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShRanged<Item> {
    pub span: Range<Item>,
}

/// A member that reaches the parameter through a `Result`.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShResulted<Item> {
    pub outcome: Result<Item, String>,
}

/// A member that reaches the parameter through a map under a key Swift wraps.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShKeyed<Item> {
    pub by_number: BTreeMap<u32, Item>,
}

/// A member that reaches the parameter through a fixed-size array.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShFixed<Item> {
    pub trio: [Item; 2],
}

/// A generic model that names a function on itself.
#[model_schema(default_types(Item = String), preprocess = ["tidied"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShCorrected<Item> {
    pub held: Item,
}

/// A member read as its default where it is absent, over the parameter.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShDefaulted<Item> {
    #[model_schema_prop(nullable)]
    #[serde(default)]
    pub spare: Option<Item>,
}

/// A flattened member that reaches the parameter.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShFlattened<Item> {
    #[serde(flatten)]
    pub inner: ShPage<Item>,
}

/// A member that holds its own type behind a box.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShLinked<Item> {
    pub item: Item,
    #[model_schema_prop(nullable)]
    pub next: Option<Box<Self>>,
}

/// An enum with a tuple variant of several slots.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShSlots<Item> {
    Empty,
    Pair(Item, u32),
    Single(Item),
}

/// An enum under a tag alone, with a newtype variant.
#[model_schema(default_types(Item = ShParcel))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ShTagged<Item> {
    Empty,
    Named { held: Item },
    Wrapped(Item),
}

/// The holder of each shape, every member written with what fills the parameters.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShShelf {
    pub boxed: ShLinked<ShParcel>,
    pub corrected: ShCorrected<ShParcel>,
    pub defaulted: ShDefaulted<Vec<ShParcel>>,
    pub fixed_arg: ShPage<[ShParcel; 2]>,
    pub fixed_held: ShFixed<ShParcel>,
    pub flattened: ShFlattened<ShParcel>,
    pub keyed_arg: ShPage<BTreeMap<u32, ShParcel>>,
    pub keyed_held: ShKeyed<ShParcel>,
    pub ranged_arg: ShPage<Range<ShParcel>>,
    pub ranged_held: ShRanged<ShParcel>,
    pub resulted_arg: ShPage<Result<ShParcel, String>>,
    pub resulted_held: ShResulted<ShParcel>,
    pub slots: ShSlots<ShParcel>,
    pub tagged: ShTagged<ShParcel>,
    pub tupled_arg: ShPage<(ShParcel, u32)>,
    pub tupled_held: ShTupled<ShParcel>,
    pub zlast: ShPage<ShParcel>,
}

/// The holder of each shape, written before the generic types it holds are declared.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShBelowShelf {
    pub page: ShBelowPage<ShBelowParcel>,
    pub slots: ShBelowSlots<ShBelowParcel>,
    pub tupled_arg: ShBelowPage<(ShBelowParcel, u32)>,
    pub tupled_held: ShBelowTupled<ShBelowParcel>,
    pub zlast: ShBelowPage<ShBelowParcel>,
}

/// The holder of each shape, over generic types declared in another crate.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShApartShelf {
    pub page: ApartPage<ApartParcel>,
    pub slots: ApartSlots<ApartParcel>,
    pub tupled_arg: ApartPage<(ApartParcel, u32)>,
    pub tupled_held: ApartTupled<ApartParcel>,
    pub zlast: ApartPage<ApartParcel>,
}

/// A model with a bound, declared below the holder that holds it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShBelowParcel {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

/// A generic struct with a rule of its own, declared below its holder.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShBelowPage<Item> {
    pub items: Vec<Item>,
    #[model_schema_prop(minLength = 1)]
    pub page_title: String,
}

/// A member that reaches the parameter through a tuple, declared below its holder.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShBelowTupled<Item> {
    pub pair: (Item, u32),
}

/// A tuple variant of several slots, declared below its holder.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShBelowSlots<Item> {
    Empty,
    Pair(Item, u32),
}

/// The payload as it is.
pub fn tidied(payload: Value) -> Value {
    payload
}

fn page(items: &[&str]) -> String {
    format!(r#"{{"items":[{}],"pageTitle":"t"}}"#, items.join(","))
}

/// A shelf every member of which keeps every rule, but the one `changed` writes.
fn shelf(changed: &[(&str, &str)]) -> String {
    let members: Vec<(&str, String)> = vec![
        ("boxed", format!(r#"{{"item":{SOUND},"next":null}}"#)),
        ("corrected", format!(r#"{{"held":{SOUND}}}"#)),
        ("defaulted", r#"{"spare":null}"#.to_owned()),
        ("fixedArg", page(&[&format!("[{SOUND},{SOUND}]")])),
        ("fixedHeld", format!(r#"{{"trio":[{SOUND},{SOUND}]}}"#)),
        ("flattened", page(&[SOUND])),
        ("keyedArg", page(&[&format!(r#"{{"1":{SOUND}}}"#)])),
        ("keyedHeld", format!(r#"{{"byNumber":{{"1":{SOUND}}}}}"#)),
        (
            "rangedArg",
            page(&[&format!(r#"{{"start":{SOUND},"end":{SOUND}}}"#)]),
        ),
        (
            "rangedHeld",
            format!(r#"{{"span":{{"start":{SOUND},"end":{SOUND}}}}}"#),
        ),
        ("resultedArg", page(&[&format!(r#"{{"Ok":{SOUND}}}"#)])),
        ("resultedHeld", format!(r#"{{"outcome":{{"Ok":{SOUND}}}}}"#)),
        ("slots", r#""empty""#.to_owned()),
        ("tagged", r#"{"kind":"empty"}"#.to_owned()),
        ("tupledArg", page(&[&format!("[{SOUND},1]")])),
        ("tupledHeld", format!(r#"{{"pair":[{SOUND},1]}}"#)),
        ("zlast", page(&[SOUND])),
    ];
    let written: Vec<String> = members
        .iter()
        .map(|(key, value)| {
            let held = changed
                .iter()
                .find(|(at, _)| at == key)
                .map_or(value.as_str(), |(_, written)| written);
            format!(r#""{key}":{held}"#)
        })
        .collect();
    format!("{{{}}}", written.join(","))
}

/// Each case, its name and the payload of a shelf broken in one member.
fn cases() -> Vec<(&'static str, String)> {
    let broken_at = |member: &'static str, held: String| (member, Some((member, held)));
    [
        ("sound", None),
        broken_at(
            "boxed",
            format!(r#"{{"item":{SOUND},"next":{{"item":{BROKEN},"next":null}}}}"#),
        ),
        broken_at("corrected", format!(r#"{{"held":{BROKEN}}}"#)),
        broken_at("defaulted", format!(r#"{{"spare":[{SOUND},{BROKEN}]}}"#)),
        broken_at("fixedArg", page(&[&format!("[{SOUND},{BROKEN}]")])),
        broken_at("fixedHeld", format!(r#"{{"trio":[{BROKEN},{SOUND}]}}"#)),
        broken_at(
            "flattened",
            format!(r#"{{"items":[{BROKEN}],"pageTitle":""}}"#),
        ),
        broken_at("keyedArg", page(&[&format!(r#"{{"1":{BROKEN}}}"#)])),
        broken_at(
            "keyedHeld",
            format!(r#"{{"byNumber":{{"1":{SOUND},"2":{BROKEN}}}}}"#),
        ),
        broken_at(
            "rangedArg",
            page(&[&format!(r#"{{"start":{SOUND},"end":{BROKEN}}}"#)]),
        ),
        broken_at(
            "rangedHeld",
            format!(r#"{{"span":{{"start":{SOUND},"end":{BROKEN}}}}}"#),
        ),
        broken_at("resultedArg", page(&[&format!(r#"{{"Ok":{BROKEN}}}"#)])),
        broken_at(
            "resultedHeld",
            format!(r#"{{"outcome":{{"Ok":{BROKEN}}}}}"#),
        ),
        broken_at("slots", format!(r#"{{"pair":[{BROKEN},1]}}"#)),
        broken_at("tagged", r#"{"kind":"wrapped","itemLabel":""}"#.to_owned()),
        broken_at("tupledArg", page(&[&format!("[{BROKEN},1]")])),
        broken_at("tupledHeld", format!(r#"{{"pair":[{BROKEN},1]}}"#)),
    ]
    .into_iter()
    .map(|(name, changed)| {
        let broken_last = page(&[BROKEN]);
        let payload = shelf(&changed.as_ref().map_or_else(Vec::new, |(at, written)| {
            vec![(*at, written.as_str()), ("zlast", broken_last.as_str())]
        }));
        (name, payload)
    })
    .collect()
}

/// What the Rust read answers of each case, one line per message.
fn expected() -> String {
    cases()
        .iter()
        .fold(String::new(), |mut lines, (name, payload)| {
            match serde_json::from_str::<ShShelf>(payload) {
                Ok(_) => {
                    let _ = writeln!(lines, "{name}: read");
                }
                Err(refused) => {
                    for message in refused.to_string().split("; ") {
                        let _ = writeln!(lines, "{name}: refused: {message}");
                    }
                }
            }
            lines
        })
}

#[test]
fn the_holder_answers_in_rust_every_broken_value_at_its_full_path() {
    let answered = expected();
    for line in [
        "sound: read",
        "tupledArg: refused: 'tupledArg.items.0.0.itemLabel': too short: minimum length is 1, got 0",
        "tupledHeld: refused: 'tupledHeld.pair.0.itemLabel': too short: minimum length is 1, got 0",
        "fixedArg: refused: 'fixedArg.items.0.1.itemLabel': too short: minimum length is 1, got 0",
    ] {
        assert!(answered.contains(line), "no `{line}` in:\n{answered}");
    }
}

#[test]
fn the_swift_read_of_a_holder_answers_what_the_rust_read_answers() {
    let definitions = [
        sh_parcel_swift::swift_definition(),
        sh_page_swift::swift_definition(),
        sh_tupled_swift::swift_definition(),
        sh_ranged_swift::swift_definition(),
        sh_resulted_swift::swift_definition(),
        sh_keyed_swift::swift_definition(),
        sh_fixed_swift::swift_definition(),
        sh_corrected_swift::swift_definition(),
        sh_defaulted_swift::swift_definition(),
        sh_flattened_swift::swift_definition(),
        sh_linked_swift::swift_definition(),
        sh_slots_swift::swift_definition(),
        sh_tagged_swift::swift_definition(),
        sh_shelf_swift::swift_definition(),
    ];
    let calls = cases()
        .iter()
        .map(|(name, payload)| format!("row(\"{name}\", #\"{payload}\"#)"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "import Foundation\n\n{}\n{SWIFT_FUNCTIONS}\nfunc row(_ name: String, _ payload: String) {{\n  do {{\n    _ = try JSONDecoder().decode(ShShelf.self, from: Data(payload.utf8))\n    print(\"\\(name): read\")\n  }} catch let refused as ShShelfRefusal {{\n    for message in refused.issues {{\n      print(\"\\(name): refused: \\(message)\")\n    }}\n  }} catch DecodingError.dataCorrupted(let context) {{\n    for message in context.debugDescription.components(separatedBy: \"; \") {{\n      print(\"\\(name): refused: \\(message)\")\n    }}\n  }} catch {{\n    print(\"\\(name): refused: \\(error)\")\n  }}\n}}\n\n{calls}\n",
        definitions.join("\n\n"),
    );
    let Some(run) = judged(
        "parameter-shapes",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n--- source ---\n{source}", run.stderr);
    assert_eq!(run.stdout, expected());
}

/// Each case of a shelf of the late layouts, its name and the payload of a shelf broken in one
/// member and in the last.
fn late_cases() -> Vec<(&'static str, String)> {
    let sound: Vec<(&str, String)> = vec![
        ("page", page(&[SOUND])),
        ("slots", r#""empty""#.to_owned()),
        ("tupledArg", page(&[&format!("[{SOUND},1]")])),
        ("tupledHeld", format!(r#"{{"pair":[{SOUND},1]}}"#)),
        ("zlast", page(&[SOUND])),
    ];
    let shelf = |changed: &[(&str, String)]| {
        let written: Vec<String> = sound
            .iter()
            .map(|(key, value)| {
                let held = changed
                    .iter()
                    .find(|(at, _)| at == key)
                    .map_or(value.as_str(), |(_, written)| written.as_str());
                format!(r#""{key}":{held}"#)
            })
            .collect();
        format!("{{{}}}", written.join(","))
    };
    let broken_last = page(&[BROKEN]);
    let broken = [
        ("page", page(&[BROKEN, SOUND, BROKEN])),
        ("slots", format!(r#"{{"pair":[{BROKEN},1]}}"#)),
        ("tupledArg", page(&[&format!("[{BROKEN},1]")])),
        ("tupledHeld", format!(r#"{{"pair":[{BROKEN},1]}}"#)),
    ];
    let mut cases = vec![("sound", shelf(&[]))];
    for (name, payload) in broken {
        cases.push((
            name,
            shelf(&[(name, payload), ("zlast", broken_last.clone())]),
        ));
    }
    cases
}

/// What the Rust read of `Shelf` answers of each of `cases`, one line per message.
fn expected_of<Shelf>(cases: &[(&str, String)]) -> String
where
    Shelf: DeserializeOwned,
{
    cases
        .iter()
        .fold(String::new(), |mut lines, (name, payload)| {
            match serde_json::from_str::<Shelf>(payload) {
                Ok(_) => {
                    let _ = writeln!(lines, "{name}: read");
                }
                Err(refused) => {
                    for message in refused.to_string().split("; ") {
                        let _ = writeln!(lines, "{name}: refused: {message}");
                    }
                }
            }
            lines
        })
}

/// Runs `definitions` in Swift, reading each of `cases` as `shelf`, and answers what it printed.
fn run_swift_shelf(
    named: &str,
    shelf: &str,
    definitions: &[String],
    cases: &[(&str, String)],
) -> String {
    let calls = cases
        .iter()
        .map(|(name, payload)| format!("row(\"{name}\", #\"{payload}\"#)"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "import Foundation\n\n{}\nfunc row(_ name: String, _ payload: String) {{\n  do {{\n    _ = try JSONDecoder().decode({shelf}.self, from: Data(payload.utf8))\n    print(\"\\(name): read\")\n  }} catch let refused as {shelf}Refusal {{\n    for message in refused.issues {{\n      print(\"\\(name): refused: \\(message)\")\n    }}\n  }} catch DecodingError.dataCorrupted(let context) {{\n    for message in context.debugDescription.components(separatedBy: \"; \") {{\n      print(\"\\(name): refused: \\(message)\")\n    }}\n  }} catch {{\n    print(\"\\(name): refused: \\(error)\")\n  }}\n}}\n\n{calls}\n",
        definitions.join("\n\n"),
    );
    let Some(run) = judged(
        named,
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return String::new();
    };
    assert!(run.succeeded, "{}\n--- source ---\n{source}", run.stderr);
    run.stdout
}

#[test]
fn the_swift_read_of_a_holder_whose_generic_types_are_declared_below_it_answers_what_rust_does() {
    let cases = late_cases();
    let answered = run_swift_shelf(
        "parameter-shapes-below",
        "ShBelowShelf",
        &[
            sh_below_shelf_swift::swift_definition(),
            sh_below_page_swift::swift_definition(),
            sh_below_tupled_swift::swift_definition(),
            sh_below_slots_swift::swift_definition(),
            sh_below_parcel_swift::swift_definition(),
        ],
        &cases,
    );
    if !answered.is_empty() {
        assert_eq!(answered, expected_of::<ShBelowShelf>(&cases));
    }
}

#[test]
fn the_swift_read_of_a_holder_whose_generic_types_are_declared_in_another_crate_answers_what_rust_does()
 {
    let cases = late_cases();
    let answered = run_swift_shelf(
        "parameter-shapes-apart",
        "ShApartShelf",
        &[
            sh_apart_shelf_swift::swift_definition(),
            apart_page_swift::swift_definition(),
            apart_tupled_swift::swift_definition(),
            apart_slots_swift::swift_definition(),
            apart_parcel_swift::swift_definition(),
        ],
        &cases,
    );
    if !answered.is_empty() {
        assert_eq!(answered, expected_of::<ShApartShelf>(&cases));
    }
}
