//! A renaming written as a list that names one key for writing and another, or none, for
//! reading: each path is under the `serialize` side, looked up in the document
//! `bson::Serializer::new()` writes for a row.
//!
//! A build that describes a type refuses such a list, so this module is compiled where nothing
//! describes one.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;
use super::stored::looked_up;
use super::written_names::{pairs, stored};

/// A field renamed for both directions apart, one for writing alone, one for reading alone,
/// and one the rule cases, which is written for writing alone.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct Carton {
    #[serde(rename(serialize = "written", deserialize = "read"))]
    both_ways: u32,
    cased_by_rule: u32,
    #[serde(rename(deserialize = "only_read"))]
    read_way: u32,
    #[serde(rename(serialize = "only_written"))]
    write_way: u32,
}

/// Named by a tag: a variant renamed for writing, the rule of the variants and of their
/// fields written for writing alone, and a variant with a rule of its own.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "mode",
    rename_all(serialize = "kebab-case"),
    rename_all_fields(serialize = "camelCase")
)]
enum Haulage {
    #[serde(rename(serialize = "byAir", deserialize = "air"))]
    Air {
        flight_code: String,
    },
    OverLand {
        road_name: String,
    },
    #[serde(rename_all(serialize = "SCREAMING_SNAKE_CASE"))]
    Sea {
        ship_name: String,
    },
}

/// Named by the key each variant is under.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
enum Relay {
    #[serde(rename(serialize = "byHand"))]
    Courier {
        badge_number: u32,
    },
    DropBox(u32),
    NotSent,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Dispatch {
    relay: Relay,
}

#[test]
fn a_field_is_under_the_serialize_side_of_its_rename() {
    let paths = Carton::MONGO_FIELDS;
    let row = stored(&Carton {
        both_ways: 1_u32,
        cased_by_rule: 2_u32,
        read_way: 3_u32,
        write_way: 4_u32,
    });
    assert_eq!(
        shown(row.clone()),
        r#"{ "written": Int64(1), "casedByRule": Int64(2), "readWay": Int64(3), "only_written": Int64(4) }"#
    );
    assert_eq!(
        [
            looked_up(&row, paths.both_ways.eq(0_u32).unwrap()),
            looked_up(&row, paths.cased_by_rule.eq(0_u32).unwrap()),
            looked_up(&row, paths.read_way.eq(0_u32).unwrap()),
            looked_up(&row, paths.write_way.eq(0_u32).unwrap()),
        ],
        pairs([
            ("written", "Int64(1)"),
            ("casedByRule", "Int64(2)"),
            ("readWay", "Int64(3)"),
            ("only_written", "Int64(4)"),
        ])
    );
}

#[test]
fn a_tag_asks_for_the_name_a_variant_is_written_under() {
    let haulage = Haulage::MONGO_FIELDS;
    let air = stored(&Haulage::Air {
        flight_code: "IB6500".to_owned(),
    });
    assert_eq!(shown(haulage.is_air()), r#"{ "mode": { "$eq": "byAir" } }"#);
    assert_eq!(
        [
            looked_up(&air, haulage.is_air()),
            looked_up(&air, haulage.air.flight_code.eq(String::new()).unwrap()),
        ],
        pairs([("mode", r#""byAir""#), ("flightCode", r#""IB6500""#)])
    );
    let land = stored(&Haulage::OverLand {
        road_name: "DR-1".to_owned(),
    });
    assert_eq!(
        shown(haulage.is_over_land()),
        r#"{ "mode": { "$eq": "over-land" } }"#
    );
    assert_eq!(
        [
            looked_up(&land, haulage.is_over_land()),
            looked_up(
                &land,
                haulage.over_land.road_name.eq(String::new()).unwrap()
            ),
        ],
        pairs([("mode", r#""over-land""#), ("roadName", r#""DR-1""#)])
    );
    let sea = stored(&Haulage::Sea {
        ship_name: "Caribe".to_owned(),
    });
    assert_eq!(
        [
            looked_up(&sea, haulage.is_sea()),
            looked_up(&sea, haulage.sea.ship_name.eq(String::new()).unwrap()),
        ],
        pairs([("mode", r#""sea""#), ("SHIP_NAME", r#""Caribe""#)])
    );
}

#[test]
fn a_variant_is_under_the_key_it_is_written_as() {
    let relay = &Dispatch::MONGO_FIELDS.relay;
    let courier = stored(&Dispatch {
        relay: Relay::Courier {
            badge_number: 7_u32,
        },
    });
    assert_eq!(
        [
            looked_up(&courier, relay.is_courier()),
            looked_up(&courier, relay.courier.badge_number.eq(0_u32).unwrap()),
        ],
        pairs([
            ("relay.byHand", r#"{ "badge_number": Int64(7) }"#),
            ("relay.byHand.badge_number", "Int64(7)"),
        ])
    );
    let drop_box = stored(&Dispatch {
        relay: Relay::DropBox(12_u32),
    });
    assert_eq!(
        [
            looked_up(&drop_box, relay.is_drop_box()),
            looked_up(&drop_box, relay.drop_box.eq(0_u32).unwrap()),
        ],
        pairs([
            ("relay.drop-box", "Int64(12)"),
            ("relay.drop-box", "Int64(12)"),
        ])
    );
    let not_sent = stored(&Dispatch {
        relay: Relay::NotSent,
    });
    assert_eq!(
        shown(relay.is_not_sent()),
        r#"{ "relay": { "$eq": "not-sent" } }"#
    );
    assert_eq!(
        looked_up(&not_sent, relay.is_not_sent()),
        pairs([("relay", r#""not-sent""#)])[0]
    );
}
