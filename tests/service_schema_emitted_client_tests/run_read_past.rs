//! Each holder of [`read_past_models`](super::read_past_models) read with and without a key no
//! member declares by serde, the JSON Schema, Zod under `node`, and the emitted Dart, Kotlin and
//! Swift types: every surface reads each payload serde writes back as the object it was, reads the
//! `nonstrict` `Customer` past the key as serde does, and refuses it beside the strict `Supplier`,
//! as serde refuses it.

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::read_past_models::{Contact, Customer, Supplier, SupplierContact};
#[cfg(feature = "dart")]
use super::read_past_models::{contact_dart, customer_dart, supplier_contact_dart, supplier_dart};
#[cfg(feature = "kotlin")]
use super::read_past_models::{
    contact_kotlin, customer_kotlin, supplier_contact_kotlin, supplier_kotlin,
};
#[cfg(feature = "swift")]
use super::read_past_models::{
    contact_swift, customer_swift, supplier_contact_swift, supplier_swift,
};
#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

/// What serde writes of `value`, with the name of its type.
fn written<T>(named: &'static str, value: &T) -> (&'static str, String)
where
    T: Serialize,
{
    (named, serde_json::to_string(value).unwrap())
}

/// Each holder's name and a payload serde writes of it, one per variant of what it flattens.
fn payloads() -> Vec<(&'static str, String)> {
    vec![
        written(
            "Customer",
            &Customer {
                contact: Contact::Email {
                    email: "ana@example.com".to_owned(),
                },
                name: "Ana".to_owned(),
            },
        ),
        written(
            "Customer",
            &Customer {
                contact: Contact::Phone {
                    phone: "555".to_owned(),
                },
                name: "Ben".to_owned(),
            },
        ),
        written(
            "Supplier",
            &Supplier {
                contact: SupplierContact::Email {
                    email: "orders@example.com".to_owned(),
                },
                name: "Mill".to_owned(),
            },
        ),
        written(
            "Supplier",
            &Supplier {
                contact: SupplierContact::Phone {
                    phone: "556".to_owned(),
                },
                name: "Forge".to_owned(),
            },
        ),
    ]
}

/// `json` with a key no member of either holder declares.
fn with_stray(json: &str) -> String {
    let mut object: Map<String, Value> = serde_json::from_str(json).unwrap();
    object.insert("stray".to_owned(), json!(1_u8));
    serde_json::to_string(&object).unwrap()
}

/// Every row a surface is handed: each payload, then each payload with a stray key.
fn rows() -> Vec<(&'static str, String)> {
    let payloads = payloads();
    let strays: Vec<(&'static str, String)> = payloads
        .iter()
        .map(|(name, json)| (*name, with_stray(json)))
        .collect();
    payloads.into_iter().chain(strays).collect()
}

/// What every surface is held to print of each row: the object serde wrote, which a stray key
/// beside `Customer` leaves as it was, or `refused` for a stray key beside `Supplier`.
fn expected() -> Vec<(usize, Value)> {
    let payloads = payloads();
    rows()
        .iter()
        .enumerate()
        .map(|(at, (name, _))| {
            let written = &payloads[at % payloads.len()].1;
            let told = if at < payloads.len() || *name == "Customer" {
                serde_json::from_str(written).unwrap()
            } else {
                Value::String("refused".to_owned())
            };
            (at, told)
        })
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

/// What serde reads back of `json` as `T`, or `refused`.
fn serde_told<T>(json: &str) -> Value
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| Value::String("refused".to_owned()),
        |read| serde_json::to_value(read).unwrap(),
    )
}

#[test]
fn serde_reads_past_a_stray_key_beside_customer_and_refuses_it_beside_supplier() {
    let told: Vec<(usize, Value)> = rows()
        .iter()
        .enumerate()
        .map(|(at, (name, json))| {
            let read = if *name == "Customer" {
                serde_told::<Customer>(json)
            } else {
                serde_told::<Supplier>(json)
            };
            (at, read)
        })
        .collect();
    assert_eq!(told, expected());
}

#[test]
fn the_json_schema_takes_a_stray_key_beside_customer_and_refuses_it_beside_supplier() {
    let taken: Vec<bool> = rows()
        .iter()
        .map(|(name, json)| {
            let schema = if *name == "Customer" {
                Customer::json_schema()
            } else {
                Supplier::json_schema()
            };
            let instance: Value = serde_json::from_str(json).unwrap();
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&instance)
        })
        .collect();
    let stated: Vec<bool> = expected()
        .iter()
        .map(|(_, told)| told.is_object())
        .collect();
    assert_eq!(taken, stated);
}

#[test]
fn the_zod_schema_reads_past_a_stray_key_beside_customer_and_refuses_it_beside_supplier() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(REQUIRED, "the Zod schemas of holders read past a stray key");
        return;
    };
    let mut parts = vec!["import { z, type ZodType } from \"zod\";".to_owned()];
    for (ts, zod) in [
        (Contact::ts_definition(), Contact::zod_schema()),
        (Customer::ts_definition(), Customer::zod_schema()),
        (
            SupplierContact::ts_definition(),
            SupplierContact::zod_schema(),
        ),
        (Supplier::ts_definition(), Supplier::zod_schema()),
    ] {
        parts.push(ts);
        parts.push(zod);
    }
    let mut main = String::new();
    for (at, (name, json)) in rows().iter().enumerate() {
        let _ = writeln!(
            main,
            "try {{ console.log(`{at}: ${{JSON.stringify({name}$Schema.parse({json}))}}`); }} \
             catch {{ console.log(`{at}: refused`); }}"
        );
    }
    parts.push(main);
    let Some(printed) = ran_with_modules(
        "read-past-zod",
        "TIXSCHEMA_NODE",
        "node",
        "read_past.mts",
        &parts.join("\n\n"),
        &modules,
    ) else {
        return;
    };
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_read_reads_past_a_stray_key_beside_customer_and_refuses_it_beside_supplier() {
    let mut main = String::from("void main() {\n");
    for (at, (name, json)) in rows().iter().enumerate() {
        let _ = writeln!(
            main,
            "  try {{ print('{at}: ${{jsonEncode({name}.fromJson(jsonDecode(r'''{json}''')).toJson())}}'); }} catch (_) {{ print('{at}: refused'); }}"
        );
    }
    main.push_str("}\n");
    let source = module(
        &[
            contact_dart::dart_definition(),
            customer_dart::dart_definition(),
            supplier_contact_dart::dart_definition(),
            supplier_dart::dart_definition(),
        ],
        &[],
        &main,
    );
    let Some((ran, printed)) = dart("read-past", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_read_reads_past_a_stray_key_beside_customer_and_refuses_it_beside_supplier() {
    let mut main = String::from("fun main() {\n");
    for (at, (name, json)) in rows().iter().enumerate() {
        let _ = writeln!(
            main,
            "    try {{ println(\"{at}: \" + Json.encodeToString({name}.serializer(), \
             Json.decodeFromString({name}.serializer(), \"\"\"{json}\"\"\"))) }} catch (_: \
             IllegalArgumentException) {{ println(\"{at}: refused\") }}"
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
        contact_kotlin::kotlin_definition(),
        customer_kotlin::kotlin_definition(),
        supplier_contact_kotlin::kotlin_definition(),
        supplier_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_read_reads_past_a_stray_key_beside_customer_and_refuses_it_beside_supplier() {
    let mut main = String::from(
        "func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {\n  do {\n    \
         let read = try JSONDecoder().decode(type, from: Data(payload.utf8))\n    \
         print(\"\\(at): \\(String(data: try JSONEncoder().encode(read), encoding: .utf8)!)\")\n  \
         } catch {\n    print(\"\\(at): refused\")\n  }\n}\n",
    );
    for (at, (name, json)) in rows().iter().enumerate() {
        let _ = writeln!(main, "told({at}, {name}.self, #\"{json}\"#)");
    }
    let source = [
        "import Foundation".to_owned(),
        contact_swift::swift_definition(),
        customer_swift::swift_definition(),
        supplier_contact_swift::swift_definition(),
        supplier_swift::swift_definition(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-read-past",
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
