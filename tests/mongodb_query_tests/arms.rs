//! The arms of an enum serde writes under a tag: one function per arm, named after the tag key
//! and the value serde writes for the variant, each in snake case. It answers a filter that holds
//! the arm's tag, and whose `and` takes conditions on the model the arm holds and on no other.
//! Its `update` takes an update of that model, and `tag_on_insert` writes the arm's tag where an
//! upsert inserts the row.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;

/// The rows a courier carries: an arm filter is a value a `const` item holds.
const BY_COURIER: shipping_schema::ArmFilter<Shipping, Courier> =
    Shipping::MONGO_FIELDS.type__by_courier();

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Wagon {
    number: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
enum Shipping {
    #[serde(rename = "by-courier")]
    ByCourier(Courier),
    #[serde(rename = "by-rail")]
    ByRail(Box<Wagon>),
    Collected,
    #[serde(rename_all = "camelCase")]
    Locker {
        bay_number: String,
    },
}

/// Declared below `Shipping`: tixschema has not seen it where the enum is expanded, as it has
/// seen no model of another crate.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Courier {
    name: String,
    region: String,
}

/// Flattens the enum: its tag and what an arm holds sit among the row's own keys.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Waybill {
    depot_code: String,
    #[serde(flatten)]
    shipping: Shipping,
}

/// Holds the enum under a key, and a list of it.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Manifest {
    legs: Vec<Shipping>,
    shipping: Shipping,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Notice {
    code: String,
}

/// Adjacently tagged: what an arm holds is under the content key.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "docType", content = "body", rename_all = "kebab-case")]
enum Paper {
    PastDue(Notice),
    Receipt,
}

/// A model written with an argument where an arm holds it.
#[model_schema(enable = ["decode_with", "mongodb"], default_types(T = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Pallet<T> {
    label: String,
    load: T,
}

/// An arm over a model written with an argument.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
enum Cargo {
    Loose,
    Palletized(Pallet<String>),
}

/// A generic enum: an arm holds a model written with the enum's own parameter.
#[model_schema(enable = ["decode_with", "mongodb"], default_types(T = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
enum Slot<T> {
    Empty,
    Filled(Pallet<T>),
}

/// An arm that holds the enum itself.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "next")]
enum Route {
    End,
    Then(Box<Self>),
}

/// Compiles only where `arm` is the arm filter `A`.
const fn held<A>(_arm: &A) {}

/// The pallets labelled `label`.
fn labelled(label: &str) -> pallet_schema::Filter<Pallet<String>> {
    Pallet::<String>::MONGO_FIELDS
        .label()
        .eq(label.to_owned())
        .unwrap()
}

/// The couriers named `Acme`.
fn named() -> courier_schema::Filter<Courier> {
    Courier::MONGO_FIELDS.name().eq("Acme".to_owned()).unwrap()
}

/// A courier renamed: an update of the rows of `Courier`.
fn renamed() -> courier_schema::Update<Courier> {
    Courier::MONGO_FIELDS
        .name()
        .set("Renamed".to_owned())
        .unwrap()
}

/// A courier renamed through its arm, with its region and the arm's tag written on insert.
fn renamed_with_its_tag() -> shipping_schema::ArmUpdate<Shipping, Courier> {
    let in_the_north = Courier::MONGO_FIELDS
        .region()
        .set_on_insert("north".to_owned())
        .unwrap();
    BY_COURIER
        .update(renamed())
        .and(in_the_north)
        .tag_on_insert()
}

#[test]
fn an_arm_answers_its_tag_alone() {
    let shipping = Shipping::MONGO_FIELDS;
    assert_eq!(
        shown(shipping.type__by_courier().into_document()),
        r#"{ "type": { "$eq": "by-courier" } }"#
    );
    assert_eq!(shown(BY_COURIER), r#"{ "type": { "$eq": "by-courier" } }"#);
}

#[test]
fn an_arm_takes_conditions_on_the_model_it_holds() {
    assert_eq!(
        shown(BY_COURIER.and(named())),
        r#"{ "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme" } }] }"#
    );
    let in_the_north = Courier::MONGO_FIELDS
        .region()
        .eq("north".to_owned())
        .unwrap();
    assert_eq!(
        shown(BY_COURIER.and(named()).and(in_the_north)),
        r#"{ "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme" } }, { "region": { "$eq": "north" } }] }"#
    );
}

/// `Box<Wagon>` is read through: the arm is over `Wagon`.
#[test]
fn an_arm_over_a_wrapped_model_takes_conditions_on_the_model() {
    let by_rail: shipping_schema::ArmFilter<Shipping, Wagon> =
        Shipping::MONGO_FIELDS.type__by_rail();
    let numbered = Wagon::MONGO_FIELDS.number().eq("W-9".to_owned()).unwrap();
    assert_eq!(
        shown(by_rail.and(numbered)),
        r#"{ "$and": [{ "type": { "$eq": "by-rail" } }, { "number": { "$eq": "W-9" } }] }"#
    );
}

#[test]
fn an_arm_over_a_model_written_with_an_argument_names_it_whole() {
    let palletized: cargo_schema::ArmFilter<Cargo, Pallet<String>> =
        Cargo::MONGO_FIELDS.type__palletized();
    assert_eq!(
        shown(palletized.and(labelled("P-4"))),
        r#"{ "$and": [{ "type": { "$eq": "Palletized" } }, { "label": { "$eq": "P-4" } }] }"#
    );
}

#[test]
fn a_generic_enum_has_its_arms_at_each_filling() {
    let filled: slot_schema::ArmFilter<Slot<String>, Pallet<String>> =
        Slot::<String>::MONGO_FIELDS.kind__filled();
    assert_eq!(
        shown(filled.and(labelled("P-4"))),
        r#"{ "$and": [{ "kind": { "$eq": "Filled" } }, { "label": { "$eq": "P-4" } }] }"#
    );
    assert_eq!(
        shown(Slot::<String>::MONGO_FIELDS.kind__empty()),
        r#"{ "kind": { "$eq": "Empty" } }"#
    );
}

/// The model is the enum, and a condition on it is an arm of the same enum.
#[test]
fn an_arm_that_holds_the_enum_itself_takes_an_arm_of_it() {
    let then: route_schema::ArmFilter<Route, Route> = Route::MONGO_FIELDS.kind__then();
    assert_eq!(
        shown(then.and(Route::MONGO_FIELDS.kind__end())),
        r#"{ "$and": [{ "kind": { "$eq": "Then" } }, { "next.kind": { "$eq": "End" } }] }"#
    );
}

/// A condition on fields written in the variant joins through the plain `and`.
#[test]
fn an_arm_that_holds_no_model_answers_its_tag() {
    let shipping = Shipping::MONGO_FIELDS;
    held::<shipping_schema::ArmFilter<Shipping, ()>>(&shipping.type__collected());
    held::<shipping_schema::ArmFilter<Shipping, ()>>(&shipping.type__locker());
    assert_eq!(
        shown(shipping.type__collected()),
        r#"{ "type": { "$eq": "Collected" } }"#
    );
    let in_the_bay = shipping.locker().bay_number().eq("B-2".to_owned()).unwrap();
    assert_eq!(
        shown(in_the_bay.and(shipping.type__locker())),
        r#"{ "$and": [{ "bayNumber": { "$eq": "B-2" } }, { "type": { "$eq": "Locker" } }] }"#
    );
}

#[test]
fn an_arm_of_an_adjacently_tagged_enum_writes_its_conditions_under_the_content_key() {
    let paper = Paper::MONGO_FIELDS;
    let coded = Notice::MONGO_FIELDS.code().eq("N-7".to_owned()).unwrap();
    assert_eq!(
        shown(paper.doc_type__past_due().and(coded)),
        r#"{ "$and": [{ "docType": { "$eq": "past-due" } }, { "body.code": { "$eq": "N-7" } }] }"#
    );
    assert_eq!(
        shown(paper.doc_type__receipt()),
        r#"{ "docType": { "$eq": "receipt" } }"#
    );
}

/// `nest` writes the tag and each condition under the key that holds the enum, and under none
/// where the row flattens it.
#[test]
fn an_arm_filter_goes_through_what_holds_the_enum() {
    let (waybill, manifest) = (Waybill::MONGO_FIELDS, Manifest::MONGO_FIELDS);
    let at_the_depot = waybill.depot_code().eq("SDQ-1".to_owned()).unwrap();
    assert_eq!(
        shown(
            waybill
                .shipping()
                .nest(BY_COURIER.and(named()))
                .and(at_the_depot)
        ),
        r#"{ "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme" } }, { "depotCode": { "$eq": "SDQ-1" } }] }"#
    );
    assert_eq!(
        shown(manifest.shipping().nest(BY_COURIER.and(named()))),
        r#"{ "$and": [{ "shipping.type": { "$eq": "by-courier" } }, { "shipping.name": { "$eq": "Acme" } }] }"#
    );
    assert_eq!(
        shown(manifest.legs().elem_match(BY_COURIER.and(named()))),
        r#"{ "legs": { "$elemMatch": { "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme" } }] } } }"#
    );
}

#[test]
fn an_arm_filter_joins_a_plain_filter_over_the_enum() {
    let shipping = Shipping::MONGO_FIELDS;
    assert_eq!(
        shown(shipping.is_collected().or(BY_COURIER)),
        r#"{ "$or": [{ "type": { "$eq": "Collected" } }, { "type": { "$eq": "by-courier" } }] }"#
    );
    assert_eq!(
        shown(shipping.is_collected().and(BY_COURIER.and(named()))),
        r#"{ "$and": [{ "type": { "$eq": "Collected" } }, { "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme" } }] }] }"#
    );
    assert_eq!(
        shown(shipping_schema::Filter::from(BY_COURIER).negated()),
        r#"{ "$nor": [{ "type": { "$eq": "by-courier" } }] }"#
    );
}

/// `Courier` is declared below `Shipping`, so no function of the variant takes its update: the
/// arm does, and what flattens the enum nests it among the row's own keys.
#[test]
fn an_arm_takes_an_update_of_the_model_it_holds() {
    let waybill = Waybill::MONGO_FIELDS;
    assert_eq!(
        shown(waybill.shipping().nest(BY_COURIER.update(renamed()))),
        r#"{ "$set": { "name": "Renamed" } }"#
    );
    assert_eq!(
        shown(BY_COURIER.update(renamed()).into_document()),
        r#"{ "$set": { "name": "Renamed" } }"#
    );
}

/// The keys of each operator are joined, and the tag is written at the enum's own level.
#[test]
fn an_arm_update_merges_a_second_update_and_writes_its_tag_on_insert() {
    held::<shipping_schema::ArmUpdate<Shipping, Courier>>(&renamed_with_its_tag());
    assert_eq!(
        shown(renamed_with_its_tag()),
        r#"{ "$set": { "name": "Renamed" }, "$setOnInsert": { "region": "north", "type": "by-courier" } }"#
    );
}

/// `nest` writes every key under the key that holds the enum, the tag included, and the holder's
/// own update joins what it answers.
#[test]
fn an_arm_update_goes_through_what_holds_the_enum() {
    let (waybill, manifest) = (Waybill::MONGO_FIELDS, Manifest::MONGO_FIELDS);
    assert_eq!(
        shown(manifest.shipping().nest(renamed_with_its_tag())),
        r#"{ "$set": { "shipping.name": "Renamed" }, "$setOnInsert": { "shipping.region": "north", "shipping.type": "by-courier" } }"#
    );
    let at_the_depot = waybill
        .depot_code()
        .set_on_insert("SDQ-1".to_owned())
        .unwrap();
    assert_eq!(
        shown(
            waybill
                .shipping()
                .nest(renamed_with_its_tag())
                .and(at_the_depot)
        ),
        r#"{ "$set": { "name": "Renamed" }, "$setOnInsert": { "region": "north", "type": "by-courier", "depotCode": "SDQ-1" } }"#
    );
}

/// The model's keys are under the content key, and the tag is beside it.
#[test]
fn an_arm_update_of_an_adjacently_tagged_enum_writes_its_tag_beside_the_content_key() {
    let coded = Notice::MONGO_FIELDS.code().set("N-8".to_owned()).unwrap();
    assert_eq!(
        shown(
            Paper::MONGO_FIELDS
                .doc_type__past_due()
                .update(coded)
                .tag_on_insert()
        ),
        r#"{ "$set": { "body.code": "N-8" }, "$setOnInsert": { "docType": "past-due" } }"#
    );
}

/// No update is over `()`: the tag is asked for on the arm filter, and joins a plain update of
/// the fields written in the variant.
#[test]
fn an_arm_that_holds_no_model_writes_its_tag_on_insert() {
    let shipping = Shipping::MONGO_FIELDS;
    held::<shipping_schema::ArmUpdate<Shipping, ()>>(&shipping.type__collected().tag_on_insert());
    assert_eq!(
        shown(shipping.type__collected().tag_on_insert()),
        r#"{ "$setOnInsert": { "type": "Collected" } }"#
    );
    let in_the_bay = shipping
        .locker()
        .bay_number()
        .set("B-3".to_owned())
        .unwrap();
    assert_eq!(
        shown(in_the_bay.and(shipping.type__locker().tag_on_insert())),
        r#"{ "$set": { "bayNumber": "B-3" }, "$setOnInsert": { "type": "Locker" } }"#
    );
}

/// `Box<Wagon>` is read through: the arm takes an update of `Wagon`.
#[test]
fn an_arm_over_a_wrapped_model_takes_an_update_of_the_model() {
    let numbered = Wagon::MONGO_FIELDS.number().set("W-1".to_owned()).unwrap();
    assert_eq!(
        shown(Shipping::MONGO_FIELDS.type__by_rail().update(numbered)),
        r#"{ "$set": { "number": "W-1" } }"#
    );
}

/// An update selects no row: the operation's own filter carries the conditions.
#[test]
fn an_arm_update_leaves_the_conditions_of_its_arm_filter_behind() {
    assert_eq!(
        shown(BY_COURIER.and(named()).update(renamed())),
        r#"{ "$set": { "name": "Renamed" } }"#
    );
}

#[test]
fn an_arm_update_turns_into_the_update_of_its_enums_module() {
    let own: shipping_schema::Update<Shipping> =
        shipping_schema::Update::from(BY_COURIER.tag_on_insert());
    assert_eq!(
        shown(own.and(BY_COURIER.update(renamed()))),
        r#"{ "$setOnInsert": { "type": "by-courier" }, "$set": { "name": "Renamed" } }"#
    );
}

/// The arm functions are written beside the enum, where the model is named as its author wrote
/// it.
#[test]
fn an_enum_declared_inside_a_function_has_its_arms() {
    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    enum Signal {
        Go(Wagon),
        Stop,
    }

    let numbered = Wagon::MONGO_FIELDS.number().eq("W-9".to_owned()).unwrap();
    assert_eq!(
        shown(Signal::MONGO_FIELDS.kind__go().and(numbered)),
        r#"{ "$and": [{ "kind": { "$eq": "Go" } }, { "number": { "$eq": "W-9" } }] }"#
    );
    assert_eq!(
        shown(Signal::MONGO_FIELDS.kind__stop()),
        r#"{ "kind": { "$eq": "Stop" } }"#
    );
}

/// The arm enum of a schema module: one unit arm per variant serde writes.
#[test]
fn the_schema_module_of_an_enum_under_a_tag_declares_its_arms() {
    let arms = [
        shipping_schema::Arm::ByCourier,
        shipping_schema::Arm::ByRail,
        shipping_schema::Arm::Collected,
        shipping_schema::Arm::Locker,
    ];
    assert_eq!(
        format!("{arms:?}"),
        "[ByCourier, ByRail, Collected, Locker]"
    );
    assert_eq!(paper_schema::Arm::PastDue, paper_schema::Arm::PastDue);
    assert_ne!(paper_schema::Arm::PastDue, paper_schema::Arm::Receipt);
}
