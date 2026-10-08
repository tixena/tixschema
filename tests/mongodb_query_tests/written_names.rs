//! A path is under the name serde writes. A renaming written as a list counts by its `serialize`
//! side, on a field, on a variant and as a casing rule: each path generated for a row is looked
//! up in the document `bson::Serializer::new()` writes for one. Here the two directions of each
//! list agree, which every build takes.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::stored::looked_up;

/// Every renaming of a struct in its list form, the two directions agreeing.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
struct Parcel {
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    reference: String,
    weight_grams: u32,
}

/// Every renaming of an enum in its list form, the two directions agreeing.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "mode",
    rename_all(serialize = "kebab-case", deserialize = "kebab-case"),
    rename_all_fields(serialize = "camelCase", deserialize = "camelCase")
)]
enum Carriage {
    #[serde(rename(serialize = "byAir", deserialize = "byAir"))]
    Air {
        flight_code: String,
    },
    OverLand {
        road_name: String,
    },
    #[serde(rename_all(
        serialize = "SCREAMING_SNAKE_CASE",
        deserialize = "SCREAMING_SNAKE_CASE"
    ))]
    Sea {
        ship_name: String,
    },
}

pub fn stored<T>(row: &T) -> bson::Document
where
    T: Serialize,
{
    let written = row.serialize(bson::Serializer::new()).unwrap();
    written.as_document().unwrap().clone()
}

pub fn pairs<const N: usize>(expected: [(&str, &str); N]) -> [(String, String); N] {
    expected.map(|(key, value)| (key.to_owned(), value.to_owned()))
}

#[test]
fn a_list_form_whose_directions_agree_names_the_key_serde_stores() {
    let parcel = Parcel::MONGO_FIELDS;
    let row = stored(&Parcel {
        reference: "P-1".to_owned(),
        weight_grams: 450_u32,
    });
    assert_eq!(
        [
            looked_up(&row, parcel.reference.eq(String::new()).unwrap()),
            looked_up(&row, parcel.weight_grams.eq(0_u32).unwrap()),
        ],
        pairs([("ref", r#""P-1""#), ("weightGrams", "Int64(450)")])
    );
}

#[test]
fn a_variant_and_its_fields_are_under_the_names_their_list_forms_write() {
    let carriage = Carriage::MONGO_FIELDS;
    let air = stored(&Carriage::Air {
        flight_code: "IB6500".to_owned(),
    });
    assert_eq!(
        [
            looked_up(&air, carriage.is_air()),
            looked_up(&air, carriage.air.flight_code.eq(String::new()).unwrap()),
        ],
        pairs([("mode", r#""byAir""#), ("flightCode", r#""IB6500""#)])
    );
    let land = stored(&Carriage::OverLand {
        road_name: "DR-1".to_owned(),
    });
    assert_eq!(
        [
            looked_up(&land, carriage.is_over_land()),
            looked_up(
                &land,
                carriage.over_land.road_name.eq(String::new()).unwrap()
            ),
        ],
        pairs([("mode", r#""over-land""#), ("roadName", r#""DR-1""#)])
    );
    let sea = stored(&Carriage::Sea {
        ship_name: "Caribe".to_owned(),
    });
    assert_eq!(
        [
            looked_up(&sea, carriage.is_sea()),
            looked_up(&sea, carriage.sea.ship_name.eq(String::new()).unwrap()),
        ],
        pairs([("mode", r#""sea""#), ("SHIP_NAME", r#""Caribe""#)])
    );
}
