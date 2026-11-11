//! The definitions one document shares: a named type used two or more times is written once under
//! `$defs` and referenced at each use, and one used once is written in place.

/// A revision as a draft holds it.
pub mod drafts {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Revision {
        pub number: u32,
    }
}

/// A revision as a release holds it: published under the same name as a draft's, with another
/// body.
pub mod releases {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Revision {
        pub label: String,
        pub number: u32,
    }
}

/// A probe as a sensor reports it.
pub mod sensors {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Probe {
        pub depth: u32,
    }
}

/// A probe that leads on to the next one: published under the same name as a sensor's, holding
/// one, and reaching itself.
pub mod chains {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    use super::sensors;

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Probe {
        pub held: sensors::Probe,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub next: Option<Box<Self>>,
    }
}

use jsonschema::draft202012;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

const DEFS: &str = "#/$defs/";

/// A probe as a survey holds it: published under the same name as a sensor's, and holding one.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Probe {
    pub held: sensors::Probe,
    pub label: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Survey {
    pub first: Probe,
    pub reading: sensors::Probe,
    pub second: Probe,
    pub spare: sensors::Probe,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Category {
    pub children: Vec<Self>,
    pub label: ShelfLabel,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct CustomerId(pub String);

#[model_schema(default_types(Amount = f64), enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Measured<Amount> {
    pub amount: Amount,
    pub unit: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PhoneNumber {
    pub digits: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostalAddress {
    pub city: String,
    pub street: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Shelf {
    pub label: ShelfLabel,
    pub position: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShelfLabel {
    pub text: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Storefront {
    pub billing: PostalAddress,
    pub category: Category,
    pub current_draft: drafts::Revision,
    pub current_release: releases::Revision,
    pub depth: Measured<f64>,
    pub height: Measured<f64>,
    pub owner: CustomerId,
    pub phone: PhoneNumber,
    pub previous_draft: drafts::Revision,
    pub previous_release: releases::Revision,
    pub reserved: Measured<u32>,
    pub reviewer: CustomerId,
    pub shelf: Shelf,
    pub shipping: PostalAddress,
    pub stock: Measured<u32>,
}

/// Every `$ref` anywhere in `value`.
fn references(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(members) => {
            for (key, member) in members {
                if key == "$ref" {
                    found.push(member.as_str().unwrap().to_owned());
                } else {
                    references(member, found);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                references(item, found);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn storefront() -> Storefront {
    let address = PostalAddress {
        city: "Lisbon".to_owned(),
        street: "Rua Augusta".to_owned(),
    };
    let label = ShelfLabel {
        text: "Tea".to_owned(),
    };
    let measured = |amount: f64| Measured {
        amount,
        unit: "cm".to_owned(),
    };
    let counted = |amount: u32| Measured {
        amount,
        unit: "boxes".to_owned(),
    };
    Storefront {
        billing: address.clone(),
        category: Category {
            children: vec![Category {
                children: Vec::new(),
                label: label.clone(),
            }],
            label: label.clone(),
        },
        current_draft: drafts::Revision { number: 4 },
        current_release: releases::Revision {
            label: "spring".to_owned(),
            number: 3,
        },
        depth: measured(40.5),
        height: measured(180.0),
        owner: CustomerId("customer-1".to_owned()),
        phone: PhoneNumber {
            digits: "213000000".to_owned(),
        },
        previous_draft: drafts::Revision { number: 3 },
        previous_release: releases::Revision {
            label: "winter".to_owned(),
            number: 2,
        },
        reserved: counted(2),
        reviewer: CustomerId("customer-2".to_owned()),
        shelf: Shelf { label, position: 1 },
        shipping: address,
        stock: counted(12),
    }
}

#[test]
fn a_type_used_twice_is_one_definition_and_a_type_used_once_stays_in_place() {
    let object = |properties: Value, required: Value| {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": properties,
            "required": required,
        })
    };
    let reference = |key: &str| json!({ "$ref": format!("{DEFS}{key}") });
    let measured = |amount: &str| {
        object(
            json!({ "amount": { "type": amount }, "unit": { "type": "string" } }),
            json!(["amount", "unit"]),
        )
    };
    assert_eq!(
        Storefront::json_schema(),
        json!({
            "$defs": {
                "PostalAddress": object(
                    json!({ "city": { "type": "string" }, "street": { "type": "string" } }),
                    json!(["city", "street"]),
                ),
                "Category": object(
                    json!({
                        "children": { "type": "array", "items": reference("Category") },
                        "label": reference("ShelfLabel"),
                    }),
                    json!(["children", "label"]),
                ),
                "ShelfLabel": object(json!({ "text": { "type": "string" } }), json!(["text"])),
                "Revision": object(json!({ "number": { "type": "integer" } }), json!(["number"])),
                "Revision.a38e709c45a1e062": object(
                    json!({ "label": { "type": "string" }, "number": { "type": "integer" } }),
                    json!(["label", "number"]),
                ),
                "Measured.number-14eb707856780cce": measured("number"),
                "CustomerId": { "type": "string" },
                "Measured.integer-1db5c4e394db7045": measured("integer"),
            },
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "billing": reference("PostalAddress"),
                "category": reference("Category"),
                "current_draft": reference("Revision"),
                "current_release": reference("Revision.a38e709c45a1e062"),
                "depth": reference("Measured.number-14eb707856780cce"),
                "height": reference("Measured.number-14eb707856780cce"),
                "owner": reference("CustomerId"),
                "phone": object(json!({ "digits": { "type": "string" } }), json!(["digits"])),
                "previous_draft": reference("Revision"),
                "previous_release": reference("Revision.a38e709c45a1e062"),
                "reserved": reference("Measured.integer-1db5c4e394db7045"),
                "reviewer": reference("CustomerId"),
                "shelf": object(
                    json!({ "label": reference("ShelfLabel"), "position": { "type": "integer" } }),
                    json!(["label", "position"]),
                ),
                "shipping": reference("PostalAddress"),
                "stock": reference("Measured.integer-1db5c4e394db7045"),
            },
            "required": [
                "billing", "category", "current_draft", "current_release", "depth", "height",
                "owner", "phone", "previous_draft", "previous_release", "reserved", "reviewer",
                "shelf", "shipping", "stock",
            ],
        })
    );
}

#[test]
fn the_document_is_valid_json_schema_and_every_reference_resolves_within_it() {
    let document = Storefront::json_schema();
    draft202012::meta::validate(&document).unwrap();
    let defs = document["$defs"].as_object().unwrap();
    let mut found = Vec::new();
    references(&document, &mut found);
    assert_eq!(found.len(), 16, "{document:#}");
    for pointer in found {
        let key = pointer.strip_prefix(DEFS).unwrap();
        assert!(
            defs.contains_key(key),
            "`{pointer}` names nothing: {document:#}"
        );
    }
}

#[test]
fn a_written_value_reads_against_the_document_and_a_wrong_one_does_not() {
    let validator = draft202012::new(&Storefront::json_schema()).unwrap();
    let mut written = serde_json::to_value(storefront()).unwrap();
    assert!(validator.is_valid(&written), "{written:#}");
    written["shipping"]["city"] = json!(7_u32);
    assert!(!validator.is_valid(&written));
    written["shipping"]["city"] = json!("Porto");
    written["current_release"] = json!({ "number": 4_u32 });
    assert!(!validator.is_valid(&written));
}

/// `document` is valid JSON Schema 2020-12 and each of its `$ref`s names one of its definitions.
fn assert_valid_and_resolved(document: &Value) {
    draft202012::meta::validate(document).unwrap();
    let mut found = Vec::new();
    references(document, &mut found);
    for pointer in found {
        let key = pointer.strip_prefix(DEFS).unwrap();
        assert!(
            document["$defs"].get(key).is_some(),
            "`{pointer}` names nothing: {document:#}"
        );
    }
}

#[test]
fn a_type_holding_another_type_of_its_name_describes_the_one_it_holds() {
    let document = Probe::json_schema();
    assert_eq!(
        document["properties"]["held"],
        sensors::Probe::json_schema()
    );
    assert_eq!(
        document,
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "held": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": { "depth": { "type": "integer" } },
                    "required": ["depth"],
                },
                "label": { "type": "string" },
            },
            "required": ["held", "label"],
        })
    );
    assert_valid_and_resolved(&document);
}

#[test]
fn two_types_of_one_name_used_twice_each_are_two_definitions() {
    let document = Survey::json_schema();
    let reference = |key: &str| json!({ "$ref": format!("{DEFS}{key}") });
    assert_eq!(
        document,
        json!({
            "$defs": {
                "Probe": sensors::Probe::json_schema(),
                "Probe.8a26ec7e993bf99a": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "held": reference("Probe"),
                        "label": { "type": "string" },
                    },
                    "required": ["held", "label"],
                },
            },
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "first": reference("Probe.8a26ec7e993bf99a"),
                "reading": reference("Probe"),
                "second": reference("Probe.8a26ec7e993bf99a"),
                "spare": reference("Probe"),
            },
            "required": ["first", "reading", "second", "spare"],
        })
    );
    assert_valid_and_resolved(&document);

    let validator = draft202012::new(&document).unwrap();
    let probe = Probe {
        held: sensors::Probe { depth: 3 },
        label: "north".to_owned(),
    };
    let mut written = serde_json::to_value(Survey {
        first: probe.clone(),
        reading: sensors::Probe { depth: 4 },
        second: probe,
        spare: sensors::Probe { depth: 5 },
    })
    .unwrap();
    assert!(validator.is_valid(&written), "{written:#}");
    written["first"]["held"] = written["second"].clone();
    assert!(!validator.is_valid(&written));
}

#[test]
fn a_recursive_type_holding_another_type_of_its_name_reaches_its_own_definition() {
    let document = chains::Probe::json_schema();
    assert_eq!(
        document,
        json!({
            "$defs": {
                "Probe.56a7020ba0740138": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "held": sensors::Probe::json_schema(),
                        "next": {
                            "anyOf": [{ "$ref": "#/$defs/Probe.56a7020ba0740138" }, { "type": "null" }],
                        },
                    },
                    "required": ["held"],
                },
            },
            "$ref": "#/$defs/Probe.56a7020ba0740138",
        })
    );
    assert_valid_and_resolved(&document);

    let validator = draft202012::new(&document).unwrap();
    let mut written = serde_json::to_value(chains::Probe {
        held: sensors::Probe { depth: 1 },
        next: Some(Box::new(chains::Probe {
            held: sensors::Probe { depth: 2 },
            next: None,
        })),
    })
    .unwrap();
    assert!(validator.is_valid(&written), "{written:#}");
    written["next"]["held"] = written["next"].clone();
    assert!(!validator.is_valid(&written));
}
