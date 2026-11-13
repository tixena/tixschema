//! A Kotlin model type is a draft that code builds and a valid class that only `validate()` and a
//! read make: its constructor and `copy()` are private, and it reads through forwarding properties.

#![cfg(feature = "kotlin")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::{ran_kotlin, ran_kotlin_as, refused_by_kotlinc_beside};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What the same types, emitted before a model type was a pair, print for the reads above under
/// `package demo`.
const MASTER_REFUSALS: &str = "missing both: MissingFieldException: Fields [label, parcelCount] are required for type with serial name 'demo.Shipment', but they were missing at path: $\n\
missing one: MissingFieldException: Field 'parcelCount' is required for type with serial name 'demo.Shipment', but it was missing at path: $\n\
unknown key: JsonDecodingException: Unexpected JSON token at offset 37: Encountered an unknown key 'extra' at path: $\n\
Use 'ignoreUnknownKeys = true' in 'Json {}' builder or '@JsonIgnoreUnknownKeys' annotation to ignore unknown keys.\n\
JSON input: {\"label\":\"Shipping\",\"parcelCount\":3,\"extra\":1}\n\
wrong type: SerializationException: invalid type: string \"a\", expected u32 at path: $.parcelCount\n\
wrong label type: JsonDecodingException: Unexpected JSON token at offset 9: Expected quotation mark '\"', but had '5' instead at path: $.label\n\
JSON input: {\"label\":5,\"parcelCount\":3}\n\
broken rule: SerializationException: 'parcelCount': too large: maximum is 100, got 300\n\
nested missing: MissingFieldException: Fields [label, parcelCount] are required for type with serial name 'demo.Shipment', but they were missing at path: $.shipment\n\
crate missing: MissingFieldException: Field 'content' is required for type with serial name 'demo.Crate', but it was missing at path: $\n\
variant missing: SerializationException: Field 'parcelCount' is required for type with serial name 'demo.Shipment', but it was missing at path: $\n\
variant card: SerializationException: Field 'number' is required for type with serial name 'Card', but it was missing at path: $\n\
point: SerializationException: invalid length 1, expected 2 at path: $\n";

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shipment {
    pub label: LabelText,
    #[model_schema_prop(minimum = 1, maximum = 100)]
    pub parcel_count: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Payment {
    Card { number: String },
    Cash,
    Wire(Shipment),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate<T> {
    pub content: T,
    pub shipment: Shipment,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Point(pub u32, pub u32);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Holder {
    pub draft: u32,
}

fn module(main: &str) -> String {
    [
        KOTLIN_IMPORTS.to_owned(),
        label_text_kotlin::kotlin_definition(),
        shipment_kotlin::kotlin_definition(),
        payment_kotlin::kotlin_definition(),
        crate_kotlin::kotlin_definition(),
        point_kotlin::kotlin_definition(),
        holder_kotlin::kotlin_definition(),
        main.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn the_models_are_written_as_the_kotlin_runs_below_write_them() {
    let label = LabelText::new("Shipping".to_owned()).unwrap();
    let shipment = ShipmentDraft {
        label,
        parcel_count: 3,
    }
    .validate()
    .unwrap();
    assert_eq!(
        serde_json::to_string(&shipment).unwrap(),
        r#"{"label":"Shipping","parcelCount":3}"#
    );
    assert_eq!(
        serde_json::to_string(&PaymentDraft::Wire(shipment.clone()).validate().unwrap()).unwrap(),
        r#"{"kind":"Wire","label":"Shipping","parcelCount":3}"#
    );
    let held = CrateDraft {
        content: "beans".to_owned(),
        shipment,
    }
    .validate()
    .unwrap();
    assert_eq!(
        serde_json::to_string(&held).unwrap(),
        r#"{"content":"beans","shipment":{"label":"Shipping","parcelCount":3}}"#
    );
    assert_eq!(
        serde_json::to_string(&PointDraft(1, 2).validate().unwrap()).unwrap(),
        "[1,2]"
    );
    assert_eq!(
        serde_json::to_string(&HolderDraft { draft: 4 }.validate().unwrap()).unwrap(),
        r#"{"draft":4}"#
    );
}

#[test]
fn a_draft_validates_into_the_valid_type_and_throws_what_the_read_throws() {
    let main = r#"
fun main() {
    val shipment = ShipmentDraft(LabelText.of("Shipping"), 3u).validate()
    println(shipment.parcelCount)
    println(shipment.draft)
    println(Json.encodeToString(shipment))
    println(Json.encodeToString(shipment) == Json.encodeToString(shipment.draft))
    try {
        shipment.draft.copy(parcelCount = 200u).validate()
    } catch (refused: SerializationException) {
        println("refused: " + refused.message)
    }
    println(ShipmentDraft(LabelText.of("Shipping"), 200u).validationIssues())
    try {
        Json.decodeFromString<Shipment>("""{"label":"x","parcelCount":200}""")
    } catch (refused: SerializationException) {
        println("read refused: " + refused.message)
    }
    println(Json.decodeFromString<Shipment>("""{"label":"x","parcelCount":2}""").parcelCount)
    println(Json.decodeFromString<ShipmentDraft>("""{"label":"x","parcelCount":200}""").parcelCount)
}
"#;
    let Some(printed) = ran_kotlin(&module(main)) else {
        return;
    };
    assert_eq!(
        printed,
        "3\n\
         ShipmentDraft(label=LabelText(value=Shipping), parcelCount=3)\n\
         {\"label\":\"Shipping\",\"parcelCount\":3}\n\
         true\n\
         refused: 'parcelCount': too large: maximum is 100, got 200\n\
         ['parcelCount': too large: maximum is 100, got 200]\n\
         read refused: 'parcelCount': too large: maximum is 100, got 200\n\
         2\n\
         200\n"
    );
}

#[test]
fn an_enum_a_generic_type_and_a_tuple_have_the_pair() {
    let main = r#"
fun main() {
    val shipment = ShipmentDraft(LabelText.of("Shipping"), 3u).validate()
    val payment = PaymentWire(shipment).validate()
    println(payment.draft)
    println(Json.encodeToString(payment))
    val crate = CrateDraft("beans", shipment).validate()
    println(crate.content)
    println(Json.encodeToString(crate))
    println(PointDraft(1u, 2u).validate().slot1)
}
"#;
    let Some(printed) = ran_kotlin(&module(main)) else {
        return;
    };
    assert_eq!(
        printed,
        "PaymentWire(value=Shipment(draft=ShipmentDraft(label=LabelText(value=Shipping), \
         parcelCount=3)))\n\
         {\"kind\":\"Wire\",\"label\":\"Shipping\",\"parcelCount\":3}\n\
         beans\n\
         {\"content\":\"beans\",\"shipment\":{\"label\":\"Shipping\",\"parcelCount\":3}}\n\
         2\n"
    );
}

#[test]
fn a_valid_type_is_not_built_by_its_constructor_from_outside_it() {
    let beside = "fun built(draft: ShipmentDraft) = Shipment(draft)";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(
        refused.contains("it is private in 'Shipment'"),
        "got:\n{refused}"
    );
}

#[test]
fn a_valid_type_is_not_copied_from_outside_it() {
    let beside = "fun copied(shipment: Shipment) = shipment.copy(draft = shipment.draft)";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(
        refused.contains("cannot access 'fun copy(draft: ShipmentDraft = ...): Shipment'")
            && refused.contains("it is private in 'Shipment'"),
        "got:\n{refused}"
    );
}

#[test]
fn the_unjudged_read_of_a_valid_type_is_private_to_the_emitted_file() {
    let beside = "val read = ShipmentUnjudged";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(
        refused.contains("ShipmentUnjudged") && refused.contains("private"),
        "got:\n{refused}"
    );
}

#[test]
fn a_variant_of_an_enum_is_a_draft_and_the_valid_enum_is_made_by_validate() {
    let beside = "fun made(): Payment = PaymentCard(\"1\")";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(
        refused.contains("return type mismatch: expected 'Payment', actual 'PaymentCard'"),
        "got:\n{refused}"
    );
}

#[test]
fn a_valid_type_is_not_changed_from_outside_it() {
    let beside = "fun changed(shipment: Shipment) { shipment.draft = ShipmentDraft(LabelText.of(\"x\"), 3u) }";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(
        refused.contains("'val' cannot be reassigned"),
        "got:\n{refused}"
    );
}

#[test]
fn a_member_named_draft_is_declared_draft_underscore_and_keeps_its_key() {
    let main = r#"
fun main() {
    val holder = HolderDraft(7u).validate()
    println(holder.draft_)
    println(holder.draft)
    println(Json.encodeToString(holder))
    println(Json.decodeFromString<Holder>("""{"draft":9}""").draft_)
    println(HolderDraft(1u).draft_)
}
"#;
    let Some(printed) = ran_kotlin(&module(main)) else {
        return;
    };
    assert_eq!(printed, "7\nHolderDraft(draft_=7)\n{\"draft\":7}\n9\n1\n");
}

#[test]
fn a_valid_type_answers_validation_issues_and_has_no_validate() {
    let beside = "fun asked(shipment: Shipment): List<String> = shipment.validationIssues()\n\
                  fun judged(shipment: Shipment) = shipment.validate()";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(
        refused.contains("unresolved reference 'validate'"),
        "got:\n{refused}"
    );
    assert!(!refused.contains("validationIssues"), "got:\n{refused}");
}

#[test]
fn every_refusal_names_the_type_as_before_under_a_package() {
    let main = r#"
fun <T> refusal(name: String, serializer: KSerializer<T>, json: String) {
    try {
        Json.decodeFromString(serializer, json)
        println(name + ": read")
    } catch (refused: SerializationException) {
        println(name + ": " + refused.javaClass.simpleName + ": " + refused.message)
    }
}

fun main() {
    refusal("missing both", ShipmentSerializer, "{}")
    refusal("missing one", ShipmentSerializer, """{"label":"Shipping"}""")
    refusal("unknown key", ShipmentSerializer, """{"label":"Shipping","parcelCount":3,"extra":1}""")
    refusal("wrong type", ShipmentSerializer, """{"label":"Shipping","parcelCount":"a"}""")
    refusal("wrong label type", ShipmentSerializer, """{"label":5,"parcelCount":3}""")
    refusal("broken rule", ShipmentSerializer, """{"label":"Shipping","parcelCount":300}""")
    refusal("nested missing", CrateSerializer(String.serializer()), """{"content":"a","shipment":{}}""")
    refusal("crate missing", CrateSerializer(String.serializer()), """{"shipment":{"label":"Shipping","parcelCount":3}}""")
    refusal("variant missing", PaymentSerializer, """{"kind":"Wire","label":"Shipping"}""")
    refusal("variant card", PaymentSerializer, """{"kind":"Card"}""")
    refusal("point", PointSerializer, "[1]")
}
"#;
    let source = format!("package demo\n\n{}", module(main));
    let Some(printed) = ran_kotlin_as(&source, "demo.MainKt") else {
        return;
    };
    assert_eq!(printed, MASTER_REFUSALS);
}
