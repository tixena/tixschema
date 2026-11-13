//! One holder of each class of flattened members `#[model_schema]` accepts, written by serde and
//! read back on every client: JSON Schema, Zod under `node`, Dart, Kotlin and Swift each read every
//! payload serde writes, and write back the same object.

/// A value serde writes, whatever its type.
mod erased {
    use serde::Serialize;

    /// What serde writes of a value.
    pub trait Written {
        fn json(&self) -> String;
    }

    impl<T: Serialize> Written for T {
        fn json(&self) -> String {
            serde_json::to_string(self).unwrap()
        }
    }
}

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::flatten_order_models::{
    BareParcel, BareParcelDraft, ClosedParcel, ClosedParcelDraft, Hue, HueDraft, Ink, InkDraft,
    LastParcel, LastParcelDraft, Lid, LidDraft, MapParcel, MapParcelDraft, OpenParcel,
    OpenParcelDraft, Seal, SealDraft, Shape, ShapeDraft, Stamp, StampDraft, TwoOpenParcel,
    TwoOpenParcelDraft,
};
#[cfg(feature = "dart")]
use super::flatten_order_models::{
    bare_parcel_dart, closed_parcel_dart, hue_dart, ink_dart, last_parcel_dart, lid_dart,
    map_parcel_dart, open_parcel_dart, seal_dart, shape_dart, stamp_dart, two_open_parcel_dart,
};
#[cfg(feature = "kotlin")]
use super::flatten_order_models::{
    bare_parcel_kotlin, closed_parcel_kotlin, hue_kotlin, ink_kotlin, last_parcel_kotlin,
    lid_kotlin, map_parcel_kotlin, open_parcel_kotlin, seal_kotlin, shape_kotlin, stamp_kotlin,
    two_open_parcel_kotlin,
};
#[cfg(feature = "swift")]
use super::flatten_order_models::{
    bare_parcel_swift, closed_parcel_swift, hue_swift, ink_swift, last_parcel_swift, lid_swift,
    map_parcel_swift, open_parcel_swift, seal_swift, shape_swift, stamp_swift,
    two_open_parcel_swift,
};
#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

/// Each holder's name and every payload serde writes of it.
fn payloads() -> Vec<(&'static str, String)> {
    let stamp = || StampDraft {
        at: "noon".to_owned(),
    };
    let mut rows = Vec::new();
    let mut add = |name: &'static str, value: &dyn erased::Written| rows.push((name, value.json()));
    for lid in [
        LidDraft::Hinged {
            hinge: "left".to_owned(),
        },
        LidDraft::Loose,
    ] {
        add(
            "ClosedParcel",
            &ClosedParcelDraft {
                label: "n".to_owned(),
                lid: lid.validate().unwrap(),
                stamp: stamp().validate().unwrap(),
            },
        );
    }
    add(
        "BareParcel",
        &BareParcelDraft {
            ink: InkDraft {
                colour: "red".to_owned(),
            }
            .validate()
            .unwrap(),
            stamp: stamp().validate().unwrap(),
        },
    );
    for hue in [HueDraft::Cold, HueDraft::Warm { warmth: 3 }] {
        add(
            "OpenParcel",
            &OpenParcelDraft {
                hue: hue.clone().validate().unwrap(),
                label: "n".to_owned(),
                stamp: stamp().validate().unwrap(),
            },
        );
        for seal in [
            SealDraft::Foil { foil: 2 },
            SealDraft::Wax {
                wax: "red".to_owned(),
            },
        ] {
            add(
                "TwoOpenParcel",
                &TwoOpenParcelDraft {
                    hue: hue.clone().validate().unwrap(),
                    seal: seal.validate().unwrap(),
                },
            );
        }
    }
    for shape in [ShapeDraft::Flat, ShapeDraft::Round { radius: 4 }] {
        add(
            "LastParcel",
            &LastParcelDraft {
                label: "n".to_owned(),
                postmark: stamp().validate().unwrap(),
                shape: shape.validate().unwrap(),
            },
        );
    }
    add(
        "MapParcel",
        &MapParcelDraft {
            counts: BTreeMap::from([("north".to_owned(), 2)]),
        },
    );
    rows
}

/// What every client is held to print: each payload read and written back as the object it was.
fn expected() -> Vec<(usize, Value)> {
    payloads()
        .iter()
        .enumerate()
        .map(|(at, (_, json))| (at, serde_json::from_str(json).unwrap()))
        .collect()
}

/// Each line of `printed`, its object read whatever the order of its keys.
fn objects(printed: &str) -> Vec<(usize, Value)> {
    printed
        .lines()
        .map(|line| {
            let (at, written) = line.split_once(": ").unwrap();
            (
                at.parse().unwrap(),
                serde_json::from_str(written).unwrap_or_else(|_| Value::String(written.to_owned())),
            )
        })
        .collect()
}

#[test]
fn every_payload_serde_writes_reads_back_in_rust_and_holds_to_its_json_schema() {
    for (name, json) in payloads() {
        let schema = match name {
            "ClosedParcel" => ClosedParcel::json_schema(),
            "BareParcel" => BareParcel::json_schema(),
            "OpenParcel" => OpenParcel::json_schema(),
            "LastParcel" => LastParcel::json_schema(),
            "TwoOpenParcel" => TwoOpenParcel::json_schema(),
            _ => MapParcel::json_schema(),
        };
        let instance: Value = serde_json::from_str(&json).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&instance), "{name}: {json}");
        let back = match name {
            "ClosedParcel" => written_back::<ClosedParcel>(&json),
            "BareParcel" => written_back::<BareParcel>(&json),
            "OpenParcel" => written_back::<OpenParcel>(&json),
            "LastParcel" => written_back::<LastParcel>(&json),
            "TwoOpenParcel" => written_back::<TwoOpenParcel>(&json),
            _ => written_back::<MapParcel>(&json),
        };
        assert_eq!(back, json, "{name}");
    }
}

/// What serde writes back of `json` read as `T`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::to_string(&serde_json::from_str::<T>(json).unwrap()).unwrap()
}

#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn every_payload_serde_writes_is_read_back_by_the_zod_schema_of_its_holder() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(
            REQUIRED,
            "the Zod schemas of flattened members in each order",
        );
        return;
    };
    let mut source = ["import { z, type ZodType } from \"zod\";".to_owned()];
    let mut parts = source.to_vec();
    for (ts, zod) in [
        (Stamp::ts_definition(), Stamp::zod_schema()),
        (Ink::ts_definition(), Ink::zod_schema()),
        (Lid::ts_definition(), Lid::zod_schema()),
        (Hue::ts_definition(), Hue::zod_schema()),
        (Shape::ts_definition(), Shape::zod_schema()),
        (Seal::ts_definition(), Seal::zod_schema()),
        (ClosedParcel::ts_definition(), ClosedParcel::zod_schema()),
        (BareParcel::ts_definition(), BareParcel::zod_schema()),
        (OpenParcel::ts_definition(), OpenParcel::zod_schema()),
        (LastParcel::ts_definition(), LastParcel::zod_schema()),
        (TwoOpenParcel::ts_definition(), TwoOpenParcel::zod_schema()),
        (MapParcel::ts_definition(), MapParcel::zod_schema()),
    ] {
        parts.push(ts);
        parts.push(zod);
    }
    let mut main = String::new();
    for (at, (name, json)) in payloads().iter().enumerate() {
        let _ = writeln!(
            main,
            "console.log(`{at}: ${{JSON.stringify({name}$Schema.parse({json}))}}`);"
        );
    }
    parts.push(main);
    source[0] = parts.join("\n\n");
    let Some(printed) = ran_with_modules(
        "flatten-order-zod",
        "TIXSCHEMA_NODE",
        "node",
        "flatten.mts",
        &source[0],
        &modules,
    ) else {
        return;
    };
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "dart")]
#[test]
fn every_payload_serde_writes_is_read_back_in_dart() {
    let mut main = String::from("void main() {\n");
    for (at, (name, json)) in payloads().iter().enumerate() {
        let _ = writeln!(
            main,
            "  print('{at}: ${{jsonEncode({name}.fromJson(jsonDecode(r'''{json}''')).toJson())}}');"
        );
    }
    main.push_str("}\n");
    let source = module(
        &[
            stamp_dart::dart_definition(),
            ink_dart::dart_definition(),
            lid_dart::dart_definition(),
            hue_dart::dart_definition(),
            shape_dart::dart_definition(),
            seal_dart::dart_definition(),
            closed_parcel_dart::dart_definition(),
            bare_parcel_dart::dart_definition(),
            open_parcel_dart::dart_definition(),
            last_parcel_dart::dart_definition(),
            two_open_parcel_dart::dart_definition(),
            map_parcel_dart::dart_definition(),
        ],
        &[],
        &main,
    );
    let Some((ran, printed)) = dart("flatten-order", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "kotlin")]
#[test]
fn every_payload_serde_writes_is_read_back_in_kotlin() {
    let mut main = String::from("fun main() {\n");
    for (at, (name, json)) in payloads().iter().enumerate() {
        let _ = writeln!(
            main,
            "    println(\"{at}: \" + Json.encodeToString({name}.serializer(), \
             Json.decodeFromString({name}.serializer(), \"\"\"{json}\"\"\")))"
        );
    }
    main.push_str("}\n");
    let source = [
        "import kotlinx.serialization.*\n\
         import kotlinx.serialization.json.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.builtins.*"
            .to_owned(),
        stamp_kotlin::kotlin_definition(),
        ink_kotlin::kotlin_definition(),
        lid_kotlin::kotlin_definition(),
        hue_kotlin::kotlin_definition(),
        shape_kotlin::kotlin_definition(),
        seal_kotlin::kotlin_definition(),
        closed_parcel_kotlin::kotlin_definition(),
        bare_parcel_kotlin::kotlin_definition(),
        open_parcel_kotlin::kotlin_definition(),
        last_parcel_kotlin::kotlin_definition(),
        two_open_parcel_kotlin::kotlin_definition(),
        map_parcel_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "swift")]
#[test]
fn every_payload_serde_writes_is_read_back_in_swift() {
    let mut main = String::new();
    for (at, (name, json)) in payloads().iter().enumerate() {
        let _ = writeln!(
            main,
            "print(\"{at}: \\(String(data: try! JSONEncoder().encode(try! \
             JSONDecoder().decode({name}.self, from: Data(#\"{json}\"#.utf8))), encoding: \
             .utf8)!)\")"
        );
    }
    let source = [
        "import Foundation".to_owned(),
        stamp_swift::swift_definition(),
        ink_swift::swift_definition(),
        lid_swift::swift_definition(),
        hue_swift::swift_definition(),
        shape_swift::swift_definition(),
        seal_swift::swift_definition(),
        closed_parcel_swift::swift_definition(),
        bare_parcel_swift::swift_definition(),
        open_parcel_swift::swift_definition(),
        last_parcel_swift::swift_definition(),
        two_open_parcel_swift::swift_definition(),
        map_parcel_swift::swift_definition(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-flatten-order",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n{}\n{source}", run.stdout, run.stderr);
    assert_eq!(objects(&run.stdout), expected());
}
