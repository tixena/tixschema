//! The `of` function of a valid Kotlin type, which builds the draft from its members and throws
//! what `validate()` throws, held to what kotlinc accepts and what the run prints.

#![cfg(feature = "kotlin")]

use alloc::borrow::Cow;
use alloc::rc::Rc;
use alloc::sync::Arc;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockTime {
    #[model_schema_prop(maximum = 23)]
    pub hour: u8,
    #[model_schema_prop(maximum = 59)]
    pub minute: u8,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockFlag {
    pub advanced: bool,
    #[model_schema_prop(literal = true)]
    pub is_calculated: bool,
    #[model_schema_prop(literal = "dock")]
    pub kind: String,
    #[model_schema_prop(literal = 3)]
    pub version: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockOf {
    pub of: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockStamp;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockWrapped(pub String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockHeld {
    #[model_schema_prop(literal = "dock")]
    pub boxed: Box<str>,
    #[model_schema_prop(literal = 3)]
    pub counted: Rc<u32>,
    #[model_schema_prop(literal = "dock")]
    pub cow: Cow<'static, str>,
    pub label: String,
    #[model_schema_prop(literal = true)]
    pub shared: Arc<bool>,
}

#[model_schema()]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DockStatic {
    #[model_schema_prop(literal = true)]
    pub flag: &'static bool,
    pub label: String,
    #[model_schema_prop(literal = 3)]
    pub number: &'static u32,
    #[model_schema_prop(literal = "dock")]
    pub text: &'static str,
}

fn module(main: &str) -> String {
    [
        KOTLIN_IMPORTS.to_owned(),
        dock_time_kotlin::kotlin_definition(),
        dock_flag_kotlin::kotlin_definition(),
        dock_of_kotlin::kotlin_definition(),
        dock_stamp_kotlin::kotlin_definition(),
        dock_wrapped_kotlin::kotlin_definition(),
        dock_held_kotlin::kotlin_definition(),
        dock_static_kotlin::kotlin_definition(),
        main.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn of_builds_the_valid_value_and_throws_what_validate_throws() {
    let main = r#"
fun main() {
    println(Json.encodeToString(DockTime.of(12u, 0u)))
    println(Json.encodeToString(DockFlag.of(advanced = false)))
    println(Json.encodeToString(DockOf.of(of = "x")))
    println(Json.encodeToString(DockStamp.of()))
    println(Json.encodeToString(DockWrapped.of("w")))
    println(Json.encodeToString(DockHeld.of(label = "x")))
    println(Json.encodeToString(DockStatic.of(label = "x")))
    try {
        DockTime.of(24u, 0u)
        println("built")
    } catch (error: SerializationException) {
        println(error.message)
    }
}
"#;
    let Some(printed) = ran_kotlin(&module(main)) else {
        return;
    };
    let refused = DockTimeDraft {
        hour: 24,
        minute: 0,
    }
    .validate()
    .unwrap_err();
    assert_eq!(
        printed,
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            serde_json::to_string(&DockTimeDraft {
                hour: 12,
                minute: 0
            })
            .unwrap(),
            serde_json::to_string(&DockFlagDraft {
                advanced: false,
                is_calculated: true,
                kind: "dock".to_owned(),
                version: 3
            })
            .unwrap(),
            serde_json::to_string(&DockOfDraft { of: "x".to_owned() }).unwrap(),
            serde_json::to_string(&DockStampDraft).unwrap(),
            serde_json::to_string(&DockWrappedDraft("w".to_owned())).unwrap(),
            serde_json::to_string(&DockHeldDraft {
                boxed: "dock".into(),
                counted: Rc::new(3),
                cow: Cow::Borrowed("dock"),
                label: "x".to_owned(),
                shared: Arc::new(true),
            })
            .unwrap(),
            serde_json::to_string(&DockStaticDraft {
                flag: &true,
                label: "x".to_owned(),
                number: &3,
                text: "dock",
            })
            .unwrap(),
            refused.join("; "),
        )
    );
}
