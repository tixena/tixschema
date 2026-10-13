//! What a member's check takes, and the name it is reported under: the name a bound of that
//! member is reported under.

/// A holder runs the `validate()` of what it holds, and with it that type's checks.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod held_by_another {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    /// A brand held to a bound and to a check: the bound is reported first.
    #[model_schema(minLength = 2, check = ["upperCase"])]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct Tag(pub String);

    #[model_schema(check = ["sealed"])]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Pouch {
        pub open: bool,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Satchel {
        #[model_schema_prop(check = ["notTheSpare"])]
        pub first_pouch: Pouch,
        pub pouch_list: Vec<Pouch>,
        pub pouch_tag: Tag,
    }

    fn upper_case(value: &Tag) -> Result<(), Vec<String>> {
        if value.0.chars().any(char::is_lowercase) {
            return Err(vec!["a tag is upper case".to_owned()]);
        }
        Ok(())
    }

    fn sealed(value: &Pouch) -> Result<(), Vec<String>> {
        if value.open {
            return Err(vec!["a pouch is sealed".to_owned()]);
        }
        Ok(())
    }

    fn not_the_spare(value: &Pouch) -> Result<(), Vec<String>> {
        if value.open {
            return Err(vec!["the first pouch is no spare".to_owned()]);
        }
        Ok(())
    }

    #[test]
    fn a_brands_check_runs_after_its_bound_and_names_no_path() {
        assert_eq!(
            Tag("a".to_owned()).validate().unwrap_err(),
            [
                "too short: minimum length is 2, got 1",
                "a tag is upper case"
            ]
        );
        assert_eq!(Tag("AB".to_owned()).validate(), Ok(()));
    }

    #[test]
    fn a_holder_reports_the_checks_of_what_it_holds_under_the_key_it_holds_it_at() {
        let held = Satchel {
            first_pouch: Pouch { open: true },
            pouch_list: vec![Pouch { open: false }, Pouch { open: true }],
            pouch_tag: Tag("ab".to_owned()),
        };
        assert_eq!(
            held.validate().unwrap_err(),
            [
                "'firstPouch': a pouch is sealed",
                "'firstPouch': the first pouch is no spare",
                "'pouchList.1': a pouch is sealed",
                "'pouchTag': a tag is upper case",
            ]
        );
    }
}

/// A check named after a function the schema module holds for a bound, on the member that bound
/// is written on: the function called is the one written here.
mod named_after_a_validator {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Tagged {
        #[model_schema_prop(minLength = 1, check = ["validateLabelValue"])]
        pub label: String,
    }

    fn validate_label_value(value: &str) -> Result<(), Vec<String>> {
        if value.trim().is_empty() {
            return Err(vec!["a label is not blank".to_owned()]);
        }
        Ok(())
    }

    #[test]
    fn a_check_named_after_a_validator_of_the_schema_module_calls_the_function_written() {
        let blank = Tagged {
            label: " ".to_owned(),
        };
        assert_eq!(
            blank.validate().unwrap_err(),
            ["'label': a label is not blank"]
        );
        let empty = Tagged {
            label: String::new(),
        };
        #[cfg(all(
            feature = "serde",
            any(feature = "typescript", feature = "zod", feature = "jsonschema")
        ))]
        assert_eq!(
            empty.validate().unwrap_err(),
            [
                "'label': too short: minimum length is 1, got 0",
                "'label': a label is not blank",
            ]
        );
        #[cfg(not(all(
            feature = "serde",
            any(feature = "typescript", feature = "zod", feature = "jsonschema")
        )))]
        assert_eq!(
            empty.validate().unwrap_err(),
            ["'label': a label is not blank"]
        );
    }
}

/// The same name on the type that holds the bounded member.
mod named_after_a_validator_on_the_type {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(check = ["validateLabelValue"])]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Labelled {
        #[model_schema_prop(minLength = 1)]
        pub label: String,
    }

    fn validate_label_value(value: &Labelled) -> Result<(), Vec<String>> {
        if value.label == "none" {
            return Err(vec!["'label': a label is not the word none".to_owned()]);
        }
        Ok(())
    }

    #[test]
    fn a_types_check_named_after_a_validator_calls_the_function_written() {
        let held = Labelled {
            label: "none".to_owned(),
        };
        assert_eq!(
            held.validate().unwrap_err(),
            ["'label': a label is not the word none"]
        );
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A member under each wrapper a check is read through, and one that is a list.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Consignment {
    #[model_schema_prop(check = ["knownCarrier"])]
    pub boxed_carrier: Box<str>,
    #[model_schema_prop(check = ["shortList"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_codes: Option<Vec<String>>,
    #[serde(rename = "codes")]
    #[model_schema_prop(check = ["shortList"])]
    pub listed_codes: Vec<String>,
    #[model_schema_prop(check = ["knownCarrier"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_carrier: Option<String>,
    #[model_schema_prop(check = ["knownWeights"])]
    pub weight_by_code: HashMap<String, u32>,
}

/// Two checks on one member and two on the type, which run in the order written.
#[model_schema(check = ["firstOfTheType", "secondOfTheType"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ordered {
    #[model_schema_prop(check = ["firstOfTheMember", "secondOfTheMember"])]
    pub code: String,
    #[model_schema_prop(check = ["firstOfTheMember"])]
    pub r#type: String,
}

/// A member's check in each position a variant holds one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Dispatch {
    ByRoad {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
    BySea(
        #[model_schema_prop(check = ["knownCarrier"])] String,
        #[model_schema_prop(check = ["shortList"])] Vec<String>,
    ),
    Carried(#[model_schema_prop(check = ["knownCarrier"])] String),
    Held,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]
pub enum DispatchAdjacent {
    ByRoad {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
    Carried(#[model_schema_prop(check = ["knownCarrier"])] String),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum DispatchUntagged {
    ByRoad {
        #[model_schema_prop(check = ["shortList"])]
        road_stops: Vec<String>,
    },
    Carried(#[model_schema_prop(check = ["knownCarrier"])] String),
}

/// A struct of one slot, which serde writes as the slot's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sticker(#[model_schema_prop(check = ["knownCarrier"])] pub String);

/// The members a holder writes into its own object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Carriage {
    pub carrier: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Waybill {
    #[serde(flatten)]
    #[model_schema_prop(check = ["carriedBySomeone"])]
    pub carriage: Carriage,
    pub number: u32,
}

fn known_carrier(value: &str) -> Result<(), Vec<String>> {
    if value == "unknown" {
        return Err(vec!["a carrier is known".to_owned()]);
    }
    Ok(())
}

/// A message that names a place of its own, which the member's name is written in front of.
fn short_list(value: &[String]) -> Result<(), Vec<String>> {
    value.get(2).map_or(Ok(()), |third| {
        Err(vec![format!("'2': {third} is one code too many")])
    })
}

fn known_weights(value: &HashMap<String, u32>) -> Result<(), Vec<String>> {
    if value.values().any(|&weight| weight == 0) {
        return Err(vec!["a weight is not zero".to_owned()]);
    }
    Ok(())
}

fn first_of_the_type(value: &Ordered) -> Result<(), Vec<String>> {
    Err(vec![format!("first of the type, on {}", value.code)])
}

fn second_of_the_type(value: &Ordered) -> Result<(), Vec<String>> {
    Err(vec![format!("second of the type, on {}", value.code)])
}

fn first_of_the_member(value: &str) -> Result<(), Vec<String>> {
    Err(vec![format!("first of the member, on {value}")])
}

fn second_of_the_member(value: &str) -> Result<(), Vec<String>> {
    Err(vec![format!("second of the member, on {value}")])
}

fn carried_by_someone(value: &Carriage) -> Result<(), Vec<String>> {
    if value.carrier.is_empty() {
        return Err(vec!["'carrier': a waybill names its carrier".to_owned()]);
    }
    Ok(())
}

fn texts(written: &[&str]) -> Vec<String> {
    written.iter().map(|&text| text.to_owned()).collect()
}

fn consignment() -> Consignment {
    Consignment {
        boxed_carrier: Box::from("north"),
        item_codes: None,
        listed_codes: Vec::new(),
        spare_carrier: None,
        weight_by_code: HashMap::new(),
    }
}

#[test]
fn a_check_on_an_option_runs_only_where_it_holds_a_value() {
    assert_eq!(consignment().validate(), Ok(()));
    let held = Consignment {
        item_codes: Some(texts(&["a", "b", "c"])),
        spare_carrier: Some("unknown".to_owned()),
        ..consignment()
    };
    #[cfg(feature = "serde")]
    let named = [
        "'itemCodes.2': c is one code too many",
        "'spareCarrier': a carrier is known",
    ];
    #[cfg(not(feature = "serde"))]
    let named = [
        "'item_codes.2': c is one code too many",
        "'spare_carrier': a carrier is known",
    ];
    assert_eq!(held.validate().unwrap_err(), named);
}

#[test]
fn a_check_takes_a_list_whole_and_the_value_under_a_wrapper() {
    let held = Consignment {
        boxed_carrier: Box::from("unknown"),
        listed_codes: texts(&["a", "b", "c"]),
        weight_by_code: HashMap::from([("a".to_owned(), 0)]),
        ..consignment()
    };
    #[cfg(feature = "serde")]
    let named = [
        "'boxedCarrier': a carrier is known",
        "'codes.2': c is one code too many",
        "'weightByCode': a weight is not zero",
    ];
    #[cfg(not(feature = "serde"))]
    let named = [
        "'boxed_carrier': a carrier is known",
        "'listed_codes.2': c is one code too many",
        "'weight_by_code': a weight is not zero",
    ];
    assert_eq!(held.validate().unwrap_err(), named);
}

#[test]
fn several_checks_run_in_the_order_written_and_none_stops_another() {
    let held = Ordered {
        code: "c".to_owned(),
        r#type: "t".to_owned(),
    };
    assert_eq!(
        held.validate().unwrap_err(),
        [
            "'code': first of the member, on c",
            "'code': second of the member, on c",
            "'type': first of the member, on t",
            "first of the type, on c",
            "second of the type, on c",
        ]
    );
}

/// serde writes a variant under its name, and under the content key where the enum names one.
/// Without `serde` no attribute is read, and every variant is under its own name.
#[test]
fn a_member_of_a_variant_is_reported_as_a_bound_of_it_is() {
    #[cfg(feature = "serde")]
    let (stops, held_stops, held) = (
        "'ByRoad.roadStops.2': c is one code too many",
        "'detail.roadStops.2': c is one code too many",
        "'detail': a carrier is known",
    );
    #[cfg(not(feature = "serde"))]
    let (stops, held_stops, held) = (
        "'ByRoad.road_stops.2': c is one code too many",
        "'ByRoad.road_stops.2': c is one code too many",
        "'Carried': a carrier is known",
    );
    let by_road = Dispatch::ByRoad {
        road_stops: texts(&["a", "b", "c"]),
    };
    assert_eq!(by_road.validate().unwrap_err(), [stops]);
    assert_eq!(
        Dispatch::BySea("unknown".to_owned(), texts(&["a", "b", "c"]))
            .validate()
            .unwrap_err(),
        [
            "'BySea.0': a carrier is known",
            "'BySea.1.2': c is one code too many"
        ]
    );
    assert_eq!(
        Dispatch::Carried("unknown".to_owned())
            .validate()
            .unwrap_err(),
        ["'Carried': a carrier is known"]
    );
    assert_eq!(Dispatch::Held.validate(), Ok(()));
    assert_eq!(Dispatch::Carried("north".to_owned()).validate(), Ok(()));

    let adjacent = DispatchAdjacent::ByRoad {
        road_stops: texts(&["a", "b", "c"]),
    };
    assert_eq!(adjacent.validate().unwrap_err(), [held_stops]);
    assert_eq!(
        DispatchAdjacent::Carried("unknown".to_owned())
            .validate()
            .unwrap_err(),
        [held]
    );
}

/// The one slot of a struct is the value serde writes for the struct, in every build.
#[test]
fn the_one_slot_of_a_struct_is_reported_under_no_name() {
    assert_eq!(
        Sticker("unknown".to_owned()).validate().unwrap_err(),
        ["a carrier is known"]
    );
    assert_eq!(Sticker("north".to_owned()).validate(), Ok(()));
}

/// The lone slot of an untagged variant is the value serde writes, so it has no name.
#[cfg(feature = "serde")]
#[test]
fn the_lone_slot_of_an_untagged_variant_is_reported_under_no_name() {
    assert_eq!(
        DispatchUntagged::Carried("unknown".to_owned())
            .validate()
            .unwrap_err(),
        ["a carrier is known"]
    );
    let by_road = DispatchUntagged::ByRoad {
        road_stops: texts(&["a", "b", "c"]),
    };
    assert_eq!(
        by_road.validate().unwrap_err(),
        ["'roadStops.2': c is one code too many"]
    );
}

/// Without `serde` the attribute that leaves the tag out is not read, and the variant has a name.
#[cfg(not(feature = "serde"))]
#[test]
fn a_variants_slot_is_reported_under_the_variants_name_without_serde() {
    assert_eq!(
        DispatchUntagged::Carried("unknown".to_owned())
            .validate()
            .unwrap_err(),
        ["'Carried': a carrier is known"]
    );
    let by_road = DispatchUntagged::ByRoad {
        road_stops: texts(&["a", "b", "c"]),
    };
    assert_eq!(
        by_road.validate().unwrap_err(),
        ["'ByRoad.road_stops.2': c is one code too many"]
    );
}

/// A flattened member writes no key of its own. Without `serde` the attribute is not read, and
/// the member is a field like any other.
#[test]
fn a_flattened_member_is_reported_under_no_name() {
    let held = Waybill {
        carriage: Carriage {
            carrier: String::new(),
        },
        number: 7,
    };
    #[cfg(feature = "serde")]
    let named = ["'carrier': a waybill names its carrier"];
    #[cfg(not(feature = "serde"))]
    let named = ["'carriage.carrier': a waybill names its carrier"];
    assert_eq!(held.validate().unwrap_err(), named);
    assert_eq!(held.number, 7);
}

/// A type declared inside a function body, whose check is declared there too.
#[test]
fn a_check_is_called_where_the_type_is_declared() {
    #[model_schema(check = ["sealed"])]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Envelope {
        #[model_schema_prop(check = ["addressed"])]
        pub address: String,
        pub open: bool,
    }

    fn sealed(value: &Envelope) -> Result<(), Vec<String>> {
        if value.open {
            return Err(vec!["an envelope is sealed".to_owned()]);
        }
        Ok(())
    }

    fn addressed(value: &str) -> Result<(), Vec<String>> {
        if value.is_empty() {
            return Err(vec!["an envelope is addressed".to_owned()]);
        }
        Ok(())
    }

    let held = Envelope {
        address: String::new(),
        open: true,
    };
    assert_eq!(
        held.validate().unwrap_err(),
        [
            "'address': an envelope is addressed",
            "an envelope is sealed"
        ]
    );
}
