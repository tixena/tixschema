//! The paths of each shape serde writes a nested value in: an enum in each of its four forms, a
//! flattened model, a generic model, and the ones the first cases leave out. The models an
//! [`Invoice`] holds are declared here, above it, and each one's module is imported beside it.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::{Address, Customer, Invoice, shown};

/// Flattened into the row that holds it: its keys sit among that row's own.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Audit {
    pub created_by: String,
    pub revision: u32,
}

/// Adjacently tagged: the variant's name under `t`, what it holds under `c`.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "t", content = "c", rename_all = "camelCase")]
pub enum Delivery {
    Courier { carrier: String, tracking: String },
    Locker(u32),
    Pickup,
}

/// Externally tagged: a variant with no data is its name as text, any other is an object with
/// one key, its name.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Rebate {
    Coupon { code: String },
    NoDiscount,
    Percent(f64),
}

/// What an internally tagged variant holds as a model of its own.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WireDetails {
    pub iban: String,
}

/// Internally tagged: the variant's name under `kind`, its fields beside it.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Payment {
    #[serde(rename_all = "camelCase")]
    Card {
        exp_month: u32,
        last4: String,
    },
    Cash,
    Wire(WireDetails),
}

/// Untagged: whatever the variant holds, with nothing naming the variant.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Reference {
    Detailed { code: String, issuer: String },
    Number(u32),
}

/// A generic model: a member typed by the parameter is one whole value of what fills it.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wrapper<T> {
    pub history: Vec<T>,
    pub inner: T,
    pub label: String,
}

/// A model the shapes below hold.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Stop {
    city: String,
    postal_code: String,
}

/// The shapes one path kind does not cover: a list a row may leave out, a list of lists, a tuple,
/// a map of models, and a list of models behind a wrapper.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Assorted {
    by_city: HashMap<String, Stop>,
    grid: Vec<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    labels: Option<Vec<String>>,
    pair: (String, u32),
    stops: Box<[Stop]>,
}

/// A variant holding several values, written as an array under the variant's name: here a name
/// of the variant's own.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Leg {
    #[serde(rename = "byAir")]
    Flight(String, u32),
    Walk,
}

/// A variant holding a model under a key of its own, and one holding several values: adjacently,
/// both at the content key.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "via", content = "at", rename_all = "camelCase")]
enum Drop {
    Door(Stop),
    Span(u32, u32),
}

/// An enum flattened into the row that holds it, and a model flattened in an `Option`.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Journey {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    audit: Option<Audit>,
    code: String,
    drop: Drop,
    leg: Leg,
    #[serde(flatten)]
    payment: Payment,
}

fn moca() -> Stop {
    Stop {
        city: "Moca".to_owned(),
        postal_code: "56000".to_owned(),
    }
}

#[test]
fn a_flattened_models_paths_are_written_at_the_level_of_the_row_that_holds_it() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.audit.created_by.eq("eduardo".to_owned()).unwrap()),
        r#"{ "createdBy": { "$eq": "eduardo" } }"#
    );
    assert_eq!(
        shown(invoice.audit.revision.gte(4_u32).unwrap()),
        r#"{ "revision": { "$gte": Int64(4) } }"#
    );
}

/// The tag is its own key beside the variant's fields, and a variant that holds a model has that
/// model's paths at the enum's own level.
#[test]
fn an_internally_tagged_enum_is_asked_by_its_tag_and_by_each_variants_fields() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.payment.is_card()),
        r#"{ "payment.kind": { "$eq": "card" } }"#
    );
    assert_eq!(
        shown(invoice.payment.card.last4.eq("4242".to_owned()).unwrap()),
        r#"{ "payment.last4": { "$eq": "4242" } }"#
    );
    assert_eq!(
        shown(invoice.payment.card.exp_month.lte(6_u32).unwrap()),
        r#"{ "payment.expMonth": { "$lte": Int64(6) } }"#
    );
    assert_eq!(
        shown(invoice.payment.wire.iban.eq("DO28".to_owned()).unwrap()),
        r#"{ "payment.iban": { "$eq": "DO28" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .payment
                .eq(Payment::Wire(WireDetails {
                    iban: "DO28".to_owned(),
                }))
                .unwrap()
        ),
        r#"{ "payment": { "$eq": { "kind": "wire", "iban": "DO28" } } }"#
    );
    assert_eq!(
        shown(invoice.payment.set(Payment::Cash).unwrap()),
        r#"{ "$set": { "payment": { "kind": "cash" } } }"#
    );
}

/// What a variant holds sits under the content key: its fields below it, and one plain value at
/// it.
#[test]
fn an_adjacently_tagged_enum_is_asked_by_its_tag_and_under_its_content_key() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.delivery.is_courier()),
        r#"{ "delivery.t": { "$eq": "courier" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .delivery
                .courier
                .carrier
                .eq("dhl".to_owned())
                .unwrap()
        ),
        r#"{ "delivery.c.carrier": { "$eq": "dhl" } }"#
    );
    assert_eq!(
        shown(invoice.delivery.locker.gt(10_u32).unwrap()),
        r#"{ "delivery.c": { "$gt": Int64(10) } }"#
    );
}

/// A variant with no data is the enum's own value, its name as text. Any other is a key of the
/// object the enum is written as.
#[test]
fn an_externally_tagged_enum_is_asked_by_the_key_that_names_its_variant() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.discount.is_no_discount()),
        r#"{ "discount": { "$eq": "noDiscount" } }"#
    );
    assert_eq!(
        shown(invoice.discount.is_coupon()),
        r#"{ "discount.coupon": { "$exists": true } }"#
    );
    assert_eq!(
        shown(invoice.discount.coupon.code.eq("FALL".to_owned()).unwrap()),
        r#"{ "discount.coupon.code": { "$eq": "FALL" } }"#
    );
    assert_eq!(
        shown(invoice.discount.percent.gte(10.0_f64).unwrap()),
        r#"{ "discount.percent": { "$gte": Double(10.0) } }"#
    );
}

/// Nothing written names the variant: one plain value is the enum's own key, and a variant's
/// fields are at the enum's own level.
#[test]
fn an_untagged_enums_paths_are_at_its_own_level() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.reference.number.eq(7_u32).unwrap()),
        r#"{ "reference": { "$eq": Int64(7) } }"#
    );
    assert_eq!(
        shown(
            invoice
                .reference
                .detailed
                .issuer
                .eq("bank".to_owned())
                .unwrap()
        ),
        r#"{ "reference.issuer": { "$eq": "bank" } }"#
    );
}

/// A member with a concrete type is a path like any other, and one typed by the parameter is one
/// whole value of the type that fills it.
#[test]
fn a_generic_models_member_typed_by_the_parameter_is_one_whole_value() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.shipping.label.eq("warehouse".to_owned()).unwrap()),
        r#"{ "shipping.label": { "$eq": "warehouse" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .shipping
                .inner
                .eq(Address {
                    city: "Moca".to_owned(),
                    postal_code: "56000".to_owned(),
                })
                .unwrap()
        ),
        r#"{ "shipping.inner": { "$eq": { "city": "Moca", "postalCode": "56000" } } }"#
    );
    let numbered = Wrapper::<u32>::MONGO_FIELDS;
    assert_eq!(
        shown(numbered.inner.gt(5_u32).unwrap()),
        r#"{ "inner": { "$gt": Int64(5) } }"#
    );
    assert_eq!(
        shown(numbered.history.contains(5_u32).unwrap()),
        r#"{ "history": { "$eq": Int64(5) } }"#
    );
}

/// Below a member that is one whole value, the paths of the type that fills it are built by that
/// type's own function, called with the keys the member's path holds.
#[test]
fn the_paths_below_a_whole_value_are_built_by_the_types_own_function() {
    let wrapper = Wrapper::<Customer>::MONGO_FIELDS;
    let inner = Customer::mongo_fields_under::<Wrapper<Customer>>(wrapper.inner.segments());
    assert_eq!(
        shown(
            wrapper
                .label
                .eq("warehouse".to_owned())
                .unwrap()
                .and(inner.address.city.eq("Moca".to_owned()).unwrap())
        ),
        r#"{ "$and": [{ "label": { "$eq": "warehouse" } }, { "inner.address.city": { "$eq": "Moca" } }] }"#
    );
}

/// A list a row may leave out is one whole list that may be absent, a list of lists is a list
/// whose elements are lists, a tuple is one whole value, and a list behind a wrapper is the list.
/// A map of models has no path.
#[test]
fn a_shape_no_path_kind_covers_is_one_whole_value() {
    let assorted = Assorted::MONGO_FIELDS;
    assert_eq!(
        shown(assorted.labels.exists(false)),
        r#"{ "labels": { "$exists": false } }"#
    );
    assert_eq!(
        shown(assorted.labels.set(vec!["a".to_owned()]).unwrap()),
        r#"{ "$set": { "labels": ["a"] } }"#
    );
    assert_eq!(
        shown(assorted.grid.contains(vec![1_u32, 2_u32]).unwrap()),
        r#"{ "grid": { "$eq": [Int64(1), Int64(2)] } }"#
    );
    assert_eq!(
        shown(assorted.pair.eq(("a".to_owned(), 1_u32)).unwrap()),
        r#"{ "pair": { "$eq": ["a", Int64(1)] } }"#
    );
    assert_eq!(
        shown(assorted.stops.city.eq("Moca".to_owned()).unwrap()),
        r#"{ "stops.city": { "$eq": "Moca" } }"#
    );
    assert_eq!(
        shown(assorted.stops.push(moca()).unwrap()),
        r#"{ "$push": { "stops": { "city": "Moca", "postalCode": "56000" } } }"#
    );
    let stored = Assorted {
        by_city: HashMap::from([("Moca".to_owned(), moca())]),
        grid: vec![vec![1_u32, 2_u32]],
        labels: None,
        pair: ("a".to_owned(), 1_u32),
        stops: Box::new([moca()]),
    }
    .serialize(bson::Serializer::new())
    .unwrap();
    assert_eq!(
        shown(stored.as_document().unwrap().clone()),
        r#"{ "byCity": { "Moca": { "city": "Moca", "postalCode": "56000" } }, "grid": [[Int64(1), Int64(2)]], "pair": ["a", Int64(1)], "stops": [{ "city": "Moca", "postalCode": "56000" }] }"#
    );
}

/// Several values in one variant are an array: each slot's path is its position under the key
/// the variant is written at, which is the name serde writes the variant as. A model held under
/// a key of its own has its paths below it.
#[test]
fn a_variant_of_several_values_and_one_of_a_model_are_typed_under_their_key() {
    let journey = Journey::MONGO_FIELDS;
    assert_eq!(
        shown(journey.leg.flight.0.eq("IB-1".to_owned()).unwrap()),
        r#"{ "leg.byAir.0": { "$eq": "IB-1" } }"#
    );
    assert_eq!(
        shown(journey.leg.flight.1.gt(3_u32).unwrap()),
        r#"{ "leg.byAir.1": { "$gt": Int64(3) } }"#
    );
    assert_eq!(
        shown(journey.leg.is_flight()),
        r#"{ "leg.byAir": { "$exists": true } }"#
    );
    assert_eq!(
        shown(journey.leg.is_walk()),
        r#"{ "leg": { "$eq": "walk" } }"#
    );
    assert_eq!(
        shown(journey.drop.door.city.eq("Moca".to_owned()).unwrap()),
        r#"{ "drop.at.city": { "$eq": "Moca" } }"#
    );
    assert_eq!(
        shown(journey.drop.door.set(moca()).unwrap()),
        r#"{ "$set": { "drop.at": { "city": "Moca", "postalCode": "56000" } } }"#
    );
    assert_eq!(
        shown(journey.drop.span.1.lt(9_u32).unwrap()),
        r#"{ "drop.at.1": { "$lt": Int64(9) } }"#
    );
    assert_eq!(
        shown(journey.drop.is_span()),
        r#"{ "drop.via": { "$eq": "span" } }"#
    );
}

/// A flattened enum's paths, and those of a model flattened in an `Option`, are at the level of
/// the row that flattens them, as serde writes their keys.
#[test]
fn a_flattened_enum_and_a_flattened_optional_model_are_at_the_rows_own_level() {
    let journey = Journey::MONGO_FIELDS;
    assert_eq!(
        shown(journey.payment.is_wire()),
        r#"{ "kind": { "$eq": "wire" } }"#
    );
    assert_eq!(
        shown(journey.payment.card.last4.eq("4242".to_owned()).unwrap()),
        r#"{ "last4": { "$eq": "4242" } }"#
    );
    assert_eq!(
        shown(journey.audit.revision.gte(2_u32).unwrap()),
        r#"{ "revision": { "$gte": Int64(2) } }"#
    );
    let stored = Journey {
        audit: Some(Audit {
            created_by: "eduardo".to_owned(),
            revision: 2_u32,
        }),
        code: "J-1".to_owned(),
        drop: Drop::Door(moca()),
        leg: Leg::Flight("IB-1".to_owned(), 4_u32),
        payment: Payment::Wire(WireDetails {
            iban: "DO28".to_owned(),
        }),
    }
    .serialize(bson::Serializer::new())
    .unwrap();
    assert_eq!(
        shown(stored.as_document().unwrap().clone()),
        r#"{ "createdBy": "eduardo", "revision": Int64(2), "code": "J-1", "drop": { "via": "door", "at": { "city": "Moca", "postalCode": "56000" } }, "leg": { "byAir": ["IB-1", Int64(4)] }, "kind": "wire", "iban": "DO28" }"#
    );
}
