//! An enum published under a name other than its Rust ident republishes the ident as a
//! `typealias`, with its parameters, under every tagging; one published under its own ident
//! writes none.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(name = "Category")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Genre {
    Poetry,
}

#[model_schema(name = "Passage")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Chapter {
    Opening { line: String },
}

#[model_schema(name = "Staging")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Act {
    Raised { line: String },
}

#[model_schema(name = "Prompt", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Cue<Leaf> {
    Sealed(Leaf),
}

#[model_schema(name = "Parchment", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Scroll<Leaf> {
    Sealed(Leaf),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Season {
    Autumn,
}

#[test]
fn a_renamed_enum_republishes_its_ident_under_every_tagging() {
    for (kotlin, alias) in [
        (
            genre_kotlin::kotlin_definition(),
            "typealias Genre = Category;",
        ),
        (
            chapter_kotlin::kotlin_definition(),
            "typealias Chapter = Passage;",
        ),
        (act_kotlin::kotlin_definition(), "typealias Act = Staging;"),
        (
            cue_kotlin::kotlin_definition(),
            "typealias Cue<Leaf> = Prompt<Leaf>;",
        ),
        (
            scroll_kotlin::kotlin_definition(),
            "typealias Scroll<Leaf> = Parchment<Leaf>;",
        ),
    ] {
        assert!(kotlin.ends_with(alias), "got: {kotlin}");
    }
}

#[test]
fn an_enum_published_under_its_own_ident_writes_no_alias() {
    assert_eq!(
        season_kotlin::kotlin_definition(),
        "@Serializable enum class Season { Autumn; fun validate(): List<String> = emptyList() }"
    );
}

#[test]
fn serde_writes_a_renamed_enum_by_its_variants_alone() {
    assert_eq!(
        [
            serde_json::to_string(&Genre::Poetry).unwrap(),
            serde_json::to_string(&Chapter::Opening {
                line: "a".to_owned()
            })
            .unwrap(),
            serde_json::to_string(&Act::Raised {
                line: "a".to_owned()
            })
            .unwrap(),
            serde_json::to_string(&Cue::Sealed("a".to_owned())).unwrap(),
            serde_json::to_string(&Scroll::Sealed("a".to_owned())).unwrap(),
            serde_json::to_string(&Season::Autumn).unwrap(),
        ],
        [
            r#""Poetry""#,
            r#"{"kind":"Opening","line":"a"}"#,
            r#"{"kind":"Raised","held":{"line":"a"}}"#,
            r#""a""#,
            r#"{"Sealed":"a"}"#,
            r#""Autumn""#,
        ]
    );
}
