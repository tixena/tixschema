//! A Kotlin brand is built through `of` alone, and its unjudged read is not reachable outside the
//! file the emitted types are joined into, while a holder reads it as it did.

#![cfg(feature = "kotlin")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::{ran_kotlin, refused_by_kotlinc_beside};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Label {
    pub text: LabelText,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Point(pub u32, pub u32);

fn module(main: &str) -> String {
    [
        KOTLIN_IMPORTS.to_owned(),
        label_text_kotlin::kotlin_definition(),
        label_kotlin::kotlin_definition(),
        point_kotlin::kotlin_definition(),
        main.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn of_builds_a_brand_that_keeps_its_rules_and_throws_for_one_that_breaks_them() {
    let main = r#"
fun main() {
    println(LabelText.of("Shipping").value)
    try {
        LabelText.of("")
    } catch (refused: SerializationException) {
        println("refused: " + refused.message)
    }
    println(Json.encodeToString(Label(text = LabelText.of("Shipping"))))
    try {
        Json.decodeFromString<Label>("""{"text":""}""")
    } catch (refused: SerializationException) {
        println("read refused: " + refused.message)
    }
    println(Point(1u, 2u))
}
"#;
    let label = Label {
        text: LabelText::new("Shipping".to_owned()).unwrap(),
    };
    assert_eq!(
        serde_json::to_string(&label).unwrap(),
        "{\"text\":\"Shipping\"}"
    );
    assert_eq!(serde_json::to_string(&Point(1, 2)).unwrap(), "[1,2]");
    let Some(printed) = ran_kotlin(&module(main)) else {
        return;
    };
    assert_eq!(
        printed,
        "Shipping\n\
         refused: too short: minimum length is 1, got 0\n\
         {\"text\":\"Shipping\"}\n\
         read refused: 'text': too short: minimum length is 1, got 0\n\
         Point(slot0=1, slot1=2)\n"
    );
}

#[test]
fn a_brand_is_not_built_by_its_constructor_from_outside_it() {
    let beside = "fun built() = LabelText(\"\")";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(
        refused.contains("it is private in 'LabelText'"),
        "got:\n{refused}"
    );
}

#[test]
fn the_unjudged_read_of_a_brand_is_private_to_the_emitted_file() {
    let beside = "val read = LabelTextUnjudged";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(
        refused.contains("LabelTextUnjudged") && refused.contains("private"),
        "got:\n{refused}"
    );
}
