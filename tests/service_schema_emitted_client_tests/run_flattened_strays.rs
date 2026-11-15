//! Each holder of [`flattened_stray_models`](super::flattened_stray_models) read by the emitted
//! Dart, Kotlin and Swift types beside serde: each payload serde writes reads back as the object it
//! was, and the same payload with a key no member declares is refused, as serde refuses it.

#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::flattened_stray_models::{
    BillingAddressDraft, BillingContactDraft, BillingCustomer, BillingCustomerDraft, BoxSizeDraft,
    CheckoutOrder, CheckoutOrderDraft, CheckoutPaymentDraft, Consignment, ConsignmentDraft,
    ConsignmentLabelDraft, CourierDeliveryDraft, CourierShipment, CourierShipmentDraft,
    DoorFinishDraft, GiftBox, GiftBoxDraft, GiftOrder, GiftOrderDraft, GiftWrapDraft,
    GuestCustomer, GuestCustomerDraft, PackageWeightDraft, PaintedDoor, PaintedDoorDraft,
    ShopEvent, ShopEventDraft, SizedCarton, SizedCartonDraft, SizedPallet, SizedPalletDraft,
    WeighedPackageDraft, WeighedParcel, WeighedParcelDraft,
};
#[cfg(feature = "dart")]
use super::flattened_stray_models::{
    billing_address_dart, billing_contact_dart, billing_customer_dart, box_size_dart,
    checkout_order_dart, checkout_payment_dart, consignment_dart, consignment_label_dart,
    courier_delivery_dart, courier_shipment_dart, door_finish_dart, gift_box_dart, gift_order_dart,
    gift_wrap_dart, guest_customer_dart, package_weight_dart, painted_door_dart, shop_event_dart,
    sized_carton_dart, sized_pallet_dart, weighed_package_dart, weighed_parcel_dart,
};
#[cfg(feature = "kotlin")]
use super::flattened_stray_models::{
    billing_address_kotlin, billing_contact_kotlin, billing_customer_kotlin, box_size_kotlin,
    checkout_order_kotlin, checkout_payment_kotlin, consignment_kotlin, consignment_label_kotlin,
    courier_delivery_kotlin, courier_shipment_kotlin, door_finish_kotlin, gift_box_kotlin,
    gift_order_kotlin, gift_wrap_kotlin, guest_customer_kotlin, package_weight_kotlin,
    painted_door_kotlin, shop_event_kotlin, sized_carton_kotlin, sized_pallet_kotlin,
    weighed_package_kotlin, weighed_parcel_kotlin,
};
#[cfg(feature = "swift")]
use super::flattened_stray_models::{
    billing_address_swift, billing_contact_swift, billing_customer_swift, box_size_swift,
    checkout_order_swift, checkout_payment_swift, consignment_label_swift, consignment_swift,
    courier_delivery_swift, courier_shipment_swift, door_finish_swift, gift_box_swift,
    gift_order_swift, gift_wrap_swift, guest_customer_swift, package_weight_swift,
    painted_door_swift, shop_event_swift, sized_carton_swift, sized_pallet_swift,
    weighed_package_swift, weighed_parcel_swift,
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

/// Each holder's name and a payload serde writes of it, for the holders of a struct.
fn struct_payloads() -> Vec<(&'static str, String)> {
    let address = || BillingAddressDraft {
        city: "Lima".to_owned(),
    };
    vec![
        written(
            "BillingCustomer",
            &BillingCustomerDraft {
                name: "Ana".to_owned(),
                contact: BillingContactDraft {
                    phone: "555".to_owned(),
                    address: address().validate().unwrap(),
                }
                .into(),
            },
        ),
        written(
            "Consignment",
            &ConsignmentDraft::Labelled(
                ConsignmentLabelDraft {
                    code: "f1".to_owned(),
                }
                .into(),
            ),
        ),
        written(
            "GiftOrder",
            &GiftOrderDraft {
                label: "g1".to_owned(),
                wrap: GiftWrapDraft {
                    paper: "red".to_owned(),
                }
                .into(),
            },
        ),
        written(
            "GiftBox",
            &GiftBoxDraft {
                label: "g2".to_owned(),
                wrap: Some(
                    GiftWrapDraft {
                        paper: "blue".to_owned(),
                    }
                    .into(),
                ),
            },
        ),
        written(
            "GiftBox",
            &GiftBoxDraft {
                label: "g3".to_owned(),
                wrap: None,
            },
        ),
        written(
            "GuestCustomer",
            &GuestCustomerDraft {
                name: "Ben".to_owned(),
                contact: BillingContactDraft {
                    phone: "556".to_owned(),
                    address: address().validate().unwrap(),
                }
                .into(),
            },
        ),
    ]
    .into_iter()
    .chain(delivery_payloads())
    .collect()
}

/// Each holder's name and a payload serde writes of it, for the holders of a struct that hold a
/// payment, a delivery or a weight.
fn delivery_payloads() -> Vec<(&'static str, String)> {
    vec![
        written(
            "CheckoutOrder",
            &CheckoutOrderDraft {
                label: "o1".to_owned(),
                payment: CheckoutPaymentDraft::Card {
                    last4: "4242".to_owned(),
                }
                .into(),
            },
        ),
        written(
            "CheckoutOrder",
            &CheckoutOrderDraft {
                label: "o2".to_owned(),
                payment: CheckoutPaymentDraft::Cash.validate().unwrap(),
            },
        ),
        written(
            "CourierShipment",
            &CourierShipmentDraft {
                label: "s1".to_owned(),
                delivery: CourierDeliveryDraft::Courier {
                    courier: "DHL".to_owned(),
                }
                .into(),
            },
        ),
        written(
            "CourierShipment",
            &CourierShipmentDraft {
                label: "s2".to_owned(),
                delivery: CourierDeliveryDraft::Pickup {
                    store: "Centro".to_owned(),
                }
                .into(),
            },
        ),
        written(
            "WeighedParcel",
            &WeighedParcelDraft {
                label: "p1".to_owned(),
                weight: WeighedPackageDraft(PackageWeightDraft { grams: 500 }.validate().unwrap())
                    .into(),
            },
        ),
    ]
}

/// Each holder's name and a payload serde writes of it, for the holders of an enum.
fn enum_payloads() -> Vec<(&'static str, String)> {
    let address = || BillingAddressDraft {
        city: "Lima".to_owned(),
    };
    vec![
        written(
            "SizedCarton",
            &SizedCartonDraft {
                label: "c1".to_owned(),
                size: BoxSizeDraft::Small { width: 3 }.into(),
            },
        ),
        written(
            "SizedCarton",
            &SizedCartonDraft {
                label: "c2".to_owned(),
                size: BoxSizeDraft::Large(PackageWeightDraft { grams: 2 }.into()).into(),
            },
        ),
        written(
            "SizedPallet",
            &SizedPalletDraft {
                label: "l1".to_owned(),
                size: Some(BoxSizeDraft::Small { width: 3 }.into()),
            },
        ),
        written(
            "SizedPallet",
            &SizedPalletDraft {
                label: "l2".to_owned(),
                size: None,
            },
        ),
        written(
            "ShopEvent",
            &ShopEventDraft::Moved {
                label: "e1".to_owned(),
                address: address().validate().unwrap(),
            },
        ),
        written("ShopEvent", &ShopEventDraft::Counted { count: 2 }),
        written(
            "PaintedDoor",
            &PaintedDoorDraft {
                finish: DoorFinishDraft::Bare { gloss: Some(2) }.into(),
                label: "f1".to_owned(),
            },
        ),
        written(
            "PaintedDoor",
            &PaintedDoorDraft {
                finish: DoorFinishDraft::Painted {
                    colour: "red".to_owned(),
                }
                .into(),
                label: "f2".to_owned(),
            },
        ),
    ]
}

/// Each holder's name and a payload serde writes of it, one per shape of what it flattens.
fn payloads() -> Vec<(&'static str, String)> {
    struct_payloads()
        .into_iter()
        .chain(enum_payloads())
        .collect()
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
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
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
                "BillingCustomer" => serde_told::<BillingCustomer>(json),
                "CheckoutOrder" => serde_told::<CheckoutOrder>(json),
                "GuestCustomer" => serde_told::<GuestCustomer>(json),
                "GiftOrder" => serde_told::<GiftOrder>(json),
                "Consignment" => serde_told::<Consignment>(json),
                "GiftBox" => serde_told::<GiftBox>(json),
                "CourierShipment" => serde_told::<CourierShipment>(json),
                "WeighedParcel" => serde_told::<WeighedParcel>(json),
                "SizedCarton" => serde_told::<SizedCarton>(json),
                "SizedPallet" => serde_told::<SizedPallet>(json),
                "ShopEvent" => serde_told::<ShopEvent>(json),
                _ => serde_told::<PaintedDoor>(json),
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
            billing_address_dart::dart_definition(),
            billing_contact_dart::dart_definition(),
            billing_customer_dart::dart_definition(),
            guest_customer_dart::dart_definition(),
            gift_order_dart::dart_definition(),
            gift_box_dart::dart_definition(),
            gift_wrap_dart::dart_definition(),
            consignment_label_dart::dart_definition(),
            consignment_dart::dart_definition(),
            checkout_payment_dart::dart_definition(),
            checkout_order_dart::dart_definition(),
            courier_delivery_dart::dart_definition(),
            courier_shipment_dart::dart_definition(),
            package_weight_dart::dart_definition(),
            weighed_package_dart::dart_definition(),
            weighed_parcel_dart::dart_definition(),
            box_size_dart::dart_definition(),
            sized_carton_dart::dart_definition(),
            sized_pallet_dart::dart_definition(),
            shop_event_dart::dart_definition(),
            door_finish_dart::dart_definition(),
            painted_door_dart::dart_definition(),
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
        billing_address_kotlin::kotlin_definition(),
        billing_contact_kotlin::kotlin_definition(),
        billing_customer_kotlin::kotlin_definition(),
        guest_customer_kotlin::kotlin_definition(),
        gift_order_kotlin::kotlin_definition(),
        gift_box_kotlin::kotlin_definition(),
        gift_wrap_kotlin::kotlin_definition(),
        consignment_label_kotlin::kotlin_definition(),
        consignment_kotlin::kotlin_definition(),
        checkout_payment_kotlin::kotlin_definition(),
        checkout_order_kotlin::kotlin_definition(),
        courier_delivery_kotlin::kotlin_definition(),
        courier_shipment_kotlin::kotlin_definition(),
        package_weight_kotlin::kotlin_definition(),
        weighed_package_kotlin::kotlin_definition(),
        weighed_parcel_kotlin::kotlin_definition(),
        box_size_kotlin::kotlin_definition(),
        sized_carton_kotlin::kotlin_definition(),
        sized_pallet_kotlin::kotlin_definition(),
        shop_event_kotlin::kotlin_definition(),
        door_finish_kotlin::kotlin_definition(),
        painted_door_kotlin::kotlin_definition(),
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
        billing_address_swift::swift_definition(),
        billing_contact_swift::swift_definition(),
        billing_customer_swift::swift_definition(),
        guest_customer_swift::swift_definition(),
        gift_order_swift::swift_definition(),
        gift_box_swift::swift_definition(),
        gift_wrap_swift::swift_definition(),
        consignment_label_swift::swift_definition(),
        consignment_swift::swift_definition(),
        checkout_payment_swift::swift_definition(),
        checkout_order_swift::swift_definition(),
        courier_delivery_swift::swift_definition(),
        courier_shipment_swift::swift_definition(),
        package_weight_swift::swift_definition(),
        weighed_package_swift::swift_definition(),
        weighed_parcel_swift::swift_definition(),
        box_size_swift::swift_definition(),
        sized_carton_swift::swift_definition(),
        sized_pallet_swift::swift_definition(),
        shop_event_swift::swift_definition(),
        door_finish_swift::swift_definition(),
        painted_door_swift::swift_definition(),
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
