//! Each holder of [`read_past_models`](super::read_past_models) read with and without a key no
//! member declares by serde, the JSON Schema, Zod under `node`, and the emitted Dart, Kotlin and
//! Swift types: every surface reads each payload serde writes back as the object it was, and reads
//! the key past or refuses it as serde does, each variant of an enum by itself.

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::read_past_models::{
    Agreement, AgreementDraft, Booking, BookingDraft, Contact, ContactDraft, Customer,
    CustomerDraft, Delivery, DeliveryDraft, Event, EventDraft, Order, OrderDraft, Remark,
    RemarkDraft, Signature, SignatureDraft, Supplier, SupplierContact, SupplierContactDraft,
    SupplierDraft, Ticket, TicketDraft,
};
#[cfg(feature = "dart")]
use super::read_past_models::{
    agreement_dart, booking_dart, contact_dart, customer_dart, delivery_dart, event_dart,
    order_dart, remark_dart, signature_dart, supplier_contact_dart, supplier_dart, ticket_dart,
};
#[cfg(feature = "kotlin")]
use super::read_past_models::{
    agreement_kotlin, booking_kotlin, contact_kotlin, customer_kotlin, delivery_kotlin,
    event_kotlin, order_kotlin, remark_kotlin, signature_kotlin, supplier_contact_kotlin,
    supplier_kotlin, ticket_kotlin,
};
#[cfg(feature = "swift")]
use super::read_past_models::{
    agreement_swift, booking_swift, contact_swift, customer_swift, delivery_swift, event_swift,
    order_swift, remark_swift, signature_swift, supplier_contact_swift, supplier_swift,
    ticket_swift,
};
#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

/// One payload serde writes: the name of its type, the payload, the key of the object a stray key
/// is written into, or none for the payload itself, and whether serde reads that stray key past.
struct Payload {
    json: String,
    named: &'static str,
    reads_past: bool,
    stray_at: Option<&'static str>,
}

/// What serde writes of `value`, with the name of its type, the object a stray key goes into and
/// whether serde reads it past there.
fn written<T>(
    named: &'static str,
    value: &T,
    stray_at: Option<&'static str>,
    reads_past: bool,
) -> Payload
where
    T: Serialize,
{
    Payload {
        json: serde_json::to_string(value).unwrap(),
        named,
        reads_past,
        stray_at,
    }
}

/// The two payloads of a holder of [`Contact`] or [`SupplierContact`], one per variant of it.
fn contacts() -> [(Contact, SupplierContact); 2] {
    [
        (
            ContactDraft::Email {
                email: "cy@example.com".to_owned(),
            }
            .validate()
            .unwrap(),
            SupplierContactDraft::Email {
                email: "orders@example.com".to_owned(),
            }
            .validate()
            .unwrap(),
        ),
        (
            ContactDraft::Phone {
                phone: "557".to_owned(),
            }
            .validate()
            .unwrap(),
            SupplierContactDraft::Phone {
                phone: "556".to_owned(),
            }
            .validate()
            .unwrap(),
        ),
    ]
}

/// A payload serde writes of each holder of [`Contact`] or [`SupplierContact`], one per variant of
/// what it flattens.
fn contact_payloads() -> Vec<Payload> {
    let mut written_all = Vec::new();
    for (contact, supplier_contact) in contacts() {
        written_all.extend([
            written(
                "Event",
                &EventDraft::Signed {
                    contact: contact.clone(),
                    name: "Cy".to_owned(),
                },
                Some("Signed"),
                true,
            ),
            written(
                "Customer",
                &CustomerDraft {
                    contact: contact.clone(),
                    name: "Ana".to_owned(),
                },
                None,
                true,
            ),
            written(
                "Supplier",
                &SupplierDraft {
                    contact: supplier_contact.clone(),
                    name: "Mill".to_owned(),
                },
                None,
                false,
            ),
            written(
                "Order",
                &OrderDraft::Placed {
                    contact: supplier_contact.clone(),
                    name: "Forge".to_owned(),
                },
                Some("Placed"),
                false,
            ),
            written(
                "Order",
                &OrderDraft::Signed {
                    contact: contact.clone(),
                    name: "Di".to_owned(),
                },
                Some("Signed"),
                true,
            ),
            written(
                "Agreement",
                &AgreementDraft::Signed(
                    SignatureDraft {
                        contact,
                        name: "Eve".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                ),
                Some("Signed"),
                true,
            ),
            written(
                "Booking",
                &BookingDraft::Placed {
                    contact: supplier_contact.clone(),
                    name: "Gil".to_owned(),
                },
                Some("body"),
                false,
            ),
            written(
                "Remark",
                &RemarkDraft::Placed {
                    contact: supplier_contact.clone(),
                    name: "Hal".to_owned(),
                },
                None,
                false,
            ),
            written(
                "Delivery",
                &DeliveryDraft::Placed {
                    contact: supplier_contact,
                    name: "Ida".to_owned(),
                },
                None,
                false,
            ),
        ]);
    }
    written_all
}

/// A payload serde writes of each variant of an enum that holds neither [`Contact`] nor
/// [`SupplierContact`], and of `Booking`'s `Noted` with a stray key beside its tag.
fn variant_payloads() -> Vec<Payload> {
    let ticket = || TicketDraft {
        code: "t1".to_owned(),
    };
    let noted = || BookingDraft::Noted {
        note: "fragile".to_owned(),
    };
    vec![
        written(
            "Agreement",
            &AgreementDraft::Voided {
                reason: "late".to_owned(),
            },
            Some("Voided"),
            false,
        ),
        written(
            "Delivery",
            &DeliveryDraft::Labelled(ticket().validate().unwrap()),
            None,
            false,
        ),
        written(
            "Remark",
            &RemarkDraft::Ticketed(ticket().validate().unwrap()),
            None,
            false,
        ),
        written(
            "Delivery",
            &DeliveryDraft::Loose {
                note: "fragile".to_owned(),
            },
            None,
            true,
        ),
        written("Booking", &noted(), Some("body"), true),
        written("Booking", &noted(), None, true),
        written(
            "Remark",
            &RemarkDraft::Noted {
                note: "fragile".to_owned(),
            },
            None,
            true,
        ),
    ]
}

/// Each holder's name and a payload serde writes of it.
fn payloads() -> Vec<Payload> {
    contact_payloads()
        .into_iter()
        .chain(variant_payloads())
        .collect()
}

/// `payload` with a key no member of its holder declares, in the object `stray_at` names.
fn with_stray(payload: &Payload) -> String {
    let mut object: Map<String, Value> = serde_json::from_str(&payload.json).unwrap();
    let members = match payload.stray_at {
        Some(key) => object.get_mut(key).and_then(Value::as_object_mut).unwrap(),
        None => &mut object,
    };
    members.insert("stray".to_owned(), json!(1_u8));
    serde_json::to_string(&object).unwrap()
}

/// Every row a surface is handed: each payload, then each payload with a stray key.
fn rows() -> Vec<(&'static str, String)> {
    let payloads = payloads();
    let strays: Vec<(&'static str, String)> = payloads
        .iter()
        .map(|payload| (payload.named, with_stray(payload)))
        .collect();
    payloads
        .into_iter()
        .map(|payload| (payload.named, payload.json))
        .chain(strays)
        .collect()
}

/// What every surface is held to print of each row: the object serde wrote, which a stray key
/// serde reads past leaves as it was, or `refused` for a stray key serde refuses.
fn expected() -> Vec<(usize, Value)> {
    let payloads = payloads();
    (0..payloads.len() * 2)
        .map(|at| {
            let payload = &payloads[at % payloads.len()];
            let told = if at < payloads.len() || payload.reads_past {
                serde_json::from_str(&payload.json).unwrap()
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
fn serde_reads_past_a_stray_key_where_the_holder_or_its_variant_reads_past_one() {
    let told: Vec<(usize, Value)> = rows()
        .iter()
        .enumerate()
        .map(|(at, (name, json))| {
            let read = match *name {
                "Agreement" => serde_told::<Agreement>(json),
                "Booking" => serde_told::<Booking>(json),
                "Customer" => serde_told::<Customer>(json),
                "Delivery" => serde_told::<Delivery>(json),
                "Event" => serde_told::<Event>(json),
                "Order" => serde_told::<Order>(json),
                "Remark" => serde_told::<Remark>(json),
                _ => serde_told::<Supplier>(json),
            };
            (at, read)
        })
        .collect();
    assert_eq!(told, expected());
}

#[test]
fn the_json_schema_takes_a_stray_key_where_serde_reads_it_past() {
    let taken: Vec<bool> = rows()
        .iter()
        .map(|(name, json)| {
            let schema = match *name {
                "Agreement" => Agreement::json_schema(),
                "Booking" => Booking::json_schema(),
                "Customer" => Customer::json_schema(),
                "Delivery" => Delivery::json_schema(),
                "Event" => Event::json_schema(),
                "Order" => Order::json_schema(),
                "Remark" => Remark::json_schema(),
                _ => Supplier::json_schema(),
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
fn the_zod_schema_reads_past_a_stray_key_where_serde_reads_it_past() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(REQUIRED, "the Zod schemas of holders read past a stray key");
        return;
    };
    let mut parts = vec!["import { z, type ZodType } from \"zod\";".to_owned()];
    for (ts, zod) in [
        (Contact::ts_definition(), Contact::zod_schema()),
        (Customer::ts_definition(), Customer::zod_schema()),
        (Event::ts_definition(), Event::zod_schema()),
        (
            SupplierContact::ts_definition(),
            SupplierContact::zod_schema(),
        ),
        (Supplier::ts_definition(), Supplier::zod_schema()),
        (Order::ts_definition(), Order::zod_schema()),
        (Signature::ts_definition(), Signature::zod_schema()),
        (Agreement::ts_definition(), Agreement::zod_schema()),
        (Ticket::ts_definition(), Ticket::zod_schema()),
        (Delivery::ts_definition(), Delivery::zod_schema()),
        (Booking::ts_definition(), Booking::zod_schema()),
        (Remark::ts_definition(), Remark::zod_schema()),
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
fn the_dart_read_reads_past_a_stray_key_where_serde_reads_it_past() {
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
            event_dart::dart_definition(),
            supplier_contact_dart::dart_definition(),
            supplier_dart::dart_definition(),
            order_dart::dart_definition(),
            signature_dart::dart_definition(),
            agreement_dart::dart_definition(),
            ticket_dart::dart_definition(),
            delivery_dart::dart_definition(),
            booking_dart::dart_definition(),
            remark_dart::dart_definition(),
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
fn the_kotlin_read_reads_past_a_stray_key_where_serde_reads_it_past() {
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
        event_kotlin::kotlin_definition(),
        supplier_contact_kotlin::kotlin_definition(),
        supplier_kotlin::kotlin_definition(),
        order_kotlin::kotlin_definition(),
        signature_kotlin::kotlin_definition(),
        agreement_kotlin::kotlin_definition(),
        ticket_kotlin::kotlin_definition(),
        delivery_kotlin::kotlin_definition(),
        booking_kotlin::kotlin_definition(),
        remark_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_read_reads_past_a_stray_key_where_serde_reads_it_past() {
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
        event_swift::swift_definition(),
        supplier_contact_swift::swift_definition(),
        supplier_swift::swift_definition(),
        order_swift::swift_definition(),
        signature_swift::swift_definition(),
        agreement_swift::swift_definition(),
        ticket_swift::swift_definition(),
        delivery_swift::swift_definition(),
        booking_swift::swift_definition(),
        remark_swift::swift_definition(),
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
