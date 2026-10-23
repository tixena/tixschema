//! A renaming written as a list whose two directions agree names one key, which every build
//! takes: each client names that key, the one serde writes and reads.

#[cfg(feature = "kotlin")]
use crate::kotlin_numbers::without_number_readers;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

/// Every renaming of a struct in its list form, the two directions agreeing.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
pub struct Parcel {
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub reference: String,
    pub weight_grams: u32,
}

/// Every renaming of an enum in its list form, the two directions agreeing.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "mode",
    rename_all(serialize = "kebab-case", deserialize = "kebab-case"),
    rename_all_fields(serialize = "camelCase", deserialize = "camelCase")
)]
pub enum Carriage {
    #[serde(rename(serialize = "byAir", deserialize = "byAir"))]
    Air {
        flight_code: String,
    },
    OverLand {
        road_name: String,
    },
}

#[test]
fn serde_writes_and_reads_each_agreed_key() {
    let parcel = Parcel {
        reference: "P-1".to_owned(),
        weight_grams: 450,
    };
    let parcel_written = json!({ "ref": "P-1", "weightGrams": 450_u32 });
    assert_eq!(serde_json::to_value(&parcel).unwrap(), parcel_written);
    assert_eq!(
        serde_json::from_value::<Parcel>(parcel_written).unwrap(),
        parcel
    );

    for (carriage, written) in [
        (
            Carriage::Air {
                flight_code: "IB6500".to_owned(),
            },
            json!({ "mode": "byAir", "flightCode": "IB6500" }),
        ),
        (
            Carriage::OverLand {
                road_name: "DR-1".to_owned(),
            },
            json!({ "mode": "over-land", "roadName": "DR-1" }),
        ),
    ] {
        assert_eq!(serde_json::to_value(&carriage).unwrap(), written);
        assert_eq!(
            serde_json::from_value::<Carriage>(written).unwrap(),
            carriage
        );
    }
}

/// Asserts that `definition` holds each of `named`, and none of `unnamed`.
fn assert_names(definition: &str, named: &[&str], unnamed: &[&str]) {
    for fragment in named {
        assert!(
            definition.contains(fragment),
            "missing `{fragment}` in: {definition}"
        );
    }
    for fragment in unnamed {
        assert!(
            !definition.contains(fragment),
            "found `{fragment}` in: {definition}"
        );
    }
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_definition_names_each_agreed_key() {
    assert_names(
        &parcel_dart::dart_definition(),
        &[
            "reference: json['ref'] as String",
            "weight_grams: (switch (json['weightGrams'] as int) { final number when number >= 0 \
             && number <= 4294967295 => number,",
            "'ref': reference",
            "'weightGrams': (switch (weight_grams) {",
        ],
        &["'reference'", "'weight_grams'"],
    );
    assert_names(
        &carriage_dart::dart_definition(),
        &[
            "case 'byAir': return CarriageAir.fromJsonUnjudged(json);",
            "case 'over-land': return CarriageOverLand.fromJsonUnjudged(json);",
            "'mode': 'byAir','flightCode': flight_code",
            "'mode': 'over-land','roadName': road_name",
        ],
        &["'Air'", "'OverLand'", "'flight_code'", "'road_name'"],
    );
}

// A Swift member is written in camel case, and a member that is its own key takes no raw value.
#[cfg(feature = "swift")]
#[test]
fn the_swift_definition_names_each_agreed_key() {
    assert_names(
        &parcel_swift::swift_definition(),
        &["case reference = \"ref\";", "case weightGrams;"],
        &["\"reference\"", "weight_grams"],
    );
    assert_names(
        &carriage_swift::swift_definition(),
        &[
            "case \"byAir\": self = .air(",
            "case \"over-land\": self = .overLand(",
            "try container.encode(\"byAir\", forKey: .swiftSchemaTag)",
            "try container.encode(\"over-land\", forKey: .swiftSchemaTag)",
            "case flightCode;",
            "case roadName;",
        ],
        &["\"Air\"", "\"OverLand\"", "flight_code", "road_name"],
    );
}

// A Kotlin property is written in camel case, and one that is its own key takes no `@SerialName`.
#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_definition_names_each_agreed_key() {
    assert_names(
        &without_number_readers(&parcel_kotlin::kotlin_definition()),
        &[
            "@SerialName(\"ref\") val reference: String",
            "val weightGrams: UInt",
        ],
        &["\"reference\"", "weight_grams"],
    );
    assert_names(
        &without_number_readers(&carriage_kotlin::kotlin_definition()),
        &[
            "@SerialName(\"byAir\") @Serializable data class CarriageAir(val flightCode: String)",
            "@SerialName(\"over-land\") @Serializable data class CarriageOverLand(val roadName: \
             String)",
        ],
        &["\"Air\"", "\"OverLand\"", "flight_code", "road_name"],
    );
}
