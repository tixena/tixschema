//! Each holder of [`flattened_stray_models`](super::flattened_stray_models) read by the emitted
//! Dart, Kotlin and Swift types beside serde: each payload serde writes reads back as the object it
//! was, and the same payload with a key no member declares is refused, as serde refuses it.

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::flattened_stray_models::{
    Address, Carton, Contact, Customer, Delivery, Event, Measured, Order, Pallet, Parcel, Payment,
    Shipment, Size, Weight,
};
#[cfg(feature = "dart")]
use super::flattened_stray_models::{
    address_dart, carton_dart, contact_dart, customer_dart, delivery_dart, event_dart,
    measured_dart, order_dart, pallet_dart, parcel_dart, payment_dart, shipment_dart, size_dart,
    weight_dart,
};
#[cfg(feature = "kotlin")]
use super::flattened_stray_models::{
    address_kotlin, carton_kotlin, contact_kotlin, customer_kotlin, delivery_kotlin, event_kotlin,
    measured_kotlin, order_kotlin, pallet_kotlin, parcel_kotlin, payment_kotlin, shipment_kotlin,
    size_kotlin, weight_kotlin,
};
#[cfg(feature = "swift")]
use super::flattened_stray_models::{
    address_swift, carton_swift, contact_swift, customer_swift, delivery_swift, event_swift,
    measured_swift, order_swift, pallet_swift, parcel_swift, payment_swift, shipment_swift,
    size_swift, weight_swift,
};
#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;

/// What serde writes of `value`, with the name of its type.
fn written<T>(named: &'static str, value: &T) -> (&'static str, String)
where
    T: Serialize,
{
    (named, serde_json::to_string(value).unwrap())
}

/// Each holder's name and a payload serde writes of it, one per shape of what it flattens.
fn payloads() -> Vec<(&'static str, String)> {
    let address = || Address {
        city: "Lima".to_owned(),
    };
    vec![
        written(
            "Customer",
            &Customer {
                name: "Ana".to_owned(),
                contact: Contact {
                    phone: "555".to_owned(),
                    address: address(),
                },
            },
        ),
        written(
            "Order",
            &Order {
                label: "o1".to_owned(),
                payment: Payment::Card {
                    last4: "4242".to_owned(),
                },
            },
        ),
        written(
            "Order",
            &Order {
                label: "o2".to_owned(),
                payment: Payment::Cash,
            },
        ),
        written(
            "Shipment",
            &Shipment {
                label: "s1".to_owned(),
                delivery: Delivery::Courier {
                    courier: "DHL".to_owned(),
                },
            },
        ),
        written(
            "Shipment",
            &Shipment {
                label: "s2".to_owned(),
                delivery: Delivery::Pickup {
                    store: "Centro".to_owned(),
                },
            },
        ),
        written(
            "Parcel",
            &Parcel {
                label: "p1".to_owned(),
                weight: Measured(Weight { grams: 500 }),
            },
        ),
        written(
            "Carton",
            &Carton {
                label: "c1".to_owned(),
                size: Size::Small { width: 3 },
            },
        ),
        written(
            "Carton",
            &Carton {
                label: "c2".to_owned(),
                size: Size::Large(Weight { grams: 2 }),
            },
        ),
        written(
            "Pallet",
            &Pallet {
                label: "l1".to_owned(),
                size: Some(Size::Small { width: 3 }),
            },
        ),
        written(
            "Pallet",
            &Pallet {
                label: "l2".to_owned(),
                size: None,
            },
        ),
        written(
            "Event",
            &Event::Moved {
                label: "e1".to_owned(),
                address: address(),
            },
        ),
        written("Event", &Event::Counted { count: 2 }),
    ]
}

/// `json` with a key no member of any holder declares.
fn with_stray(json: &str) -> String {
    let mut object: Map<String, Value> = serde_json::from_str(json).unwrap();
    object.insert("stray".to_owned(), json!(1_u8));
    serde_json::to_string(&object).unwrap()
}

/// Every row a client is handed: each payload, then each payload with a stray key.
fn rows() -> Vec<(&'static str, String)> {
    let payloads = payloads();
    let strays: Vec<(&'static str, String)> = payloads
        .iter()
        .map(|(name, json)| (*name, with_stray(json)))
        .collect();
    payloads.into_iter().chain(strays).collect()
}

/// What every client is held to print of each row: the object it read, or `refused`.
fn expected() -> Vec<(usize, Value)> {
    let count = payloads().len();
    rows()
        .iter()
        .enumerate()
        .map(|(at, (_, json))| {
            let told = if at < count {
                serde_json::from_str(json).unwrap()
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
fn serde_reads_back_each_payload_and_refuses_each_stray_key() {
    let told: Vec<(usize, Value)> = rows()
        .iter()
        .enumerate()
        .map(|(at, (name, json))| {
            let read = match *name {
                "Customer" => serde_told::<Customer>(json),
                "Order" => serde_told::<Order>(json),
                "Shipment" => serde_told::<Shipment>(json),
                "Parcel" => serde_told::<Parcel>(json),
                "Carton" => serde_told::<Carton>(json),
                "Pallet" => serde_told::<Pallet>(json),
                _ => serde_told::<Event>(json),
            };
            (at, read)
        })
        .collect();
    assert_eq!(told, expected());
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_read_reads_back_each_payload_and_refuses_each_stray_key() {
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
            address_dart::dart_definition(),
            contact_dart::dart_definition(),
            customer_dart::dart_definition(),
            payment_dart::dart_definition(),
            order_dart::dart_definition(),
            delivery_dart::dart_definition(),
            shipment_dart::dart_definition(),
            weight_dart::dart_definition(),
            measured_dart::dart_definition(),
            parcel_dart::dart_definition(),
            size_dart::dart_definition(),
            carton_dart::dart_definition(),
            pallet_dart::dart_definition(),
            event_dart::dart_definition(),
        ],
        &[],
        &main,
    );
    let Some((ran, printed)) = dart("flattened-strays", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_read_reads_back_each_payload_and_refuses_each_stray_key() {
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
        address_kotlin::kotlin_definition(),
        contact_kotlin::kotlin_definition(),
        customer_kotlin::kotlin_definition(),
        payment_kotlin::kotlin_definition(),
        order_kotlin::kotlin_definition(),
        delivery_kotlin::kotlin_definition(),
        shipment_kotlin::kotlin_definition(),
        weight_kotlin::kotlin_definition(),
        measured_kotlin::kotlin_definition(),
        parcel_kotlin::kotlin_definition(),
        size_kotlin::kotlin_definition(),
        carton_kotlin::kotlin_definition(),
        pallet_kotlin::kotlin_definition(),
        event_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_read_reads_back_each_payload_and_refuses_each_stray_key() {
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
        address_swift::swift_definition(),
        contact_swift::swift_definition(),
        customer_swift::swift_definition(),
        payment_swift::swift_definition(),
        order_swift::swift_definition(),
        delivery_swift::swift_definition(),
        shipment_swift::swift_definition(),
        weight_swift::swift_definition(),
        measured_swift::swift_definition(),
        parcel_swift::swift_definition(),
        size_swift::swift_definition(),
        carton_swift::swift_definition(),
        pallet_swift::swift_definition(),
        event_swift::swift_definition(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-flattened-strays",
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
