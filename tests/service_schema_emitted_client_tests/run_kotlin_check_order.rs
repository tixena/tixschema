//! A check a member names takes the draft of a member typed with a model that has one, whether the member's
//! type is declared below the holder, above it or in another crate: the emitted Kotlin is
//! compiled with `-Werror` and run.

#![cfg(feature = "kotlin")]

mod below {
    use super::{Debug, Deserialize, Serialize, from_str, model_schema};

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct HolderBelow {
        #[model_schema_prop(check = ["ok"])]
        pub other: ZedBagBelow,
        #[model_schema_prop(check = ["ok"])]
        pub rating: ZedBelow,
        #[model_schema_prop(check = ["ok"])]
        pub served: ServedBelow,
        #[model_schema_prop(check = ["ok"])]
        pub size: u32,
        #[model_schema_prop(check = ["ok"])]
        pub tag: TagBelow,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum ZedBelow {
        A,
        B,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ZedBagBelow {
        pub count: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct TagBelow(String);

    #[model_schema(__tixschema_undrafted)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ServedBelow {
        pub note: String,
    }

    pub fn ok<T>(value: &T) -> Result<(), Vec<String>>
    where
        T: Debug,
    {
        if format!("{value:?}").is_empty() {
            return Err(vec!["empty".to_owned()]);
        }
        Ok(())
    }

    pub fn definitions() -> Vec<String> {
        vec![
            holder_below_kotlin::kotlin_definition(),
            zed_below_kotlin::kotlin_definition(),
            zed_bag_below_kotlin::kotlin_definition(),
            tag_below_kotlin::kotlin_definition(),
            served_below_kotlin::kotlin_definition(),
        ]
    }

    pub fn holder() -> String {
        holder_below_kotlin::kotlin_definition()
    }

    pub fn built() -> bool {
        let rating = ZedBelow::B;
        let other = ZedBagBelowDraft { count: 1 }.into();
        HolderBelowDraft {
            other,
            rating,
            served: ServedBelow {
                note: "n".to_owned(),
            },
            size: 3,
            tag: from_str("\"x\"").unwrap(),
        }
        .validate()
        .is_ok()
    }
}

mod above {
    use super::{Debug, Deserialize, Serialize, from_str, model_schema};

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum ZedAbove {
        A,
        B,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ZedBagAbove {
        pub count: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct TagAbove(String);

    #[model_schema(__tixschema_undrafted)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ServedAbove {
        pub note: String,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct HolderAbove {
        #[model_schema_prop(check = ["ok"])]
        pub other: ZedBagAbove,
        #[model_schema_prop(check = ["ok"])]
        pub rating: ZedAbove,
        #[model_schema_prop(check = ["ok"])]
        pub served: ServedAbove,
        #[model_schema_prop(check = ["ok"])]
        pub size: u32,
        #[model_schema_prop(check = ["ok"])]
        pub tag: TagAbove,
    }

    pub fn ok<T>(value: &T) -> Result<(), Vec<String>>
    where
        T: Debug,
    {
        if format!("{value:?}").is_empty() {
            return Err(vec!["empty".to_owned()]);
        }
        Ok(())
    }

    pub fn definitions() -> Vec<String> {
        vec![
            zed_above_kotlin::kotlin_definition(),
            zed_bag_above_kotlin::kotlin_definition(),
            tag_above_kotlin::kotlin_definition(),
            served_above_kotlin::kotlin_definition(),
            holder_above_kotlin::kotlin_definition(),
        ]
    }

    pub fn holder() -> String {
        holder_above_kotlin::kotlin_definition()
    }

    pub fn built() -> bool {
        let rating = ZedAbove::B;
        let other = ZedBagAboveDraft { count: 1 }.into();
        HolderAboveDraft {
            other,
            rating,
            served: ServedAbove {
                note: "n".to_owned(),
            },
            size: 3,
            tag: from_str("\"x\"").unwrap(),
        }
        .validate()
        .is_ok()
    }
}

mod apart {
    use kotlin_members_apart::{ServedApart, TagApart, ZedApart, ZedBagApart, ZedBagApartDraft};

    use super::{
        Debug, Deserialize, Serialize, from_str, model_schema, served_apart_kotlin,
        tag_apart_kotlin, zed_apart_kotlin, zed_bag_apart_kotlin,
    };

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct HolderApart {
        #[model_schema_prop(check = ["ok"])]
        pub other: ZedBagApart,
        #[model_schema_prop(check = ["ok"])]
        pub rating: ZedApart,
        #[model_schema_prop(check = ["ok"])]
        pub served: ServedApart,
        #[model_schema_prop(check = ["ok"])]
        pub size: u32,
        #[model_schema_prop(check = ["ok"])]
        pub tag: TagApart,
    }

    pub fn ok<T>(value: &T) -> Result<(), Vec<String>>
    where
        T: Debug,
    {
        if format!("{value:?}").is_empty() {
            return Err(vec!["empty".to_owned()]);
        }
        Ok(())
    }

    pub fn definitions() -> Vec<String> {
        vec![
            holder_apart_kotlin::kotlin_definition(),
            zed_apart_kotlin::kotlin_definition(),
            zed_bag_apart_kotlin::kotlin_definition(),
            tag_apart_kotlin::kotlin_definition(),
            served_apart_kotlin::kotlin_definition(),
        ]
    }

    pub fn holder() -> String {
        holder_apart_kotlin::kotlin_definition()
    }

    pub fn built() -> bool {
        let rating = ZedApart::B;
        let other = ZedBagApartDraft { count: 1 }.into();
        HolderApartDraft {
            other,
            rating,
            served: ServedApart {
                note: "n".to_owned(),
            },
            size: 3,
            tag: from_str("\"x\"").unwrap(),
        }
        .validate()
        .is_ok()
    }
}

use core::fmt::Debug;

use kotlin_members_apart::{
    served_apart_kotlin, tag_apart_kotlin, zed_apart_kotlin, zed_bag_apart_kotlin,
};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tixschema::model_schema;

use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const EXPECTED: &str = "\
read: 'rating': rating is B
read: 'other': other is 1
read: 'tag': tag is x
read: 'served': served is n
read: 'size': size is 3
";

/// The Kotlin of each definition, joined with the checks of `suffix`, compiled and run.
fn compiled_and_run(definitions: &[String], suffix: &str) {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions.iter().cloned());
    parts.push(checks(suffix));
    let Some(printed) = ran_kotlin(&parts.join("\n\n")) else {
        return;
    };
    let mut lines: Vec<&str> = printed.lines().collect();
    lines.sort_unstable();
    let mut expected: Vec<&str> = EXPECTED.lines().collect();
    expected.sort_unstable();
    assert_eq!(lines, expected);
}

/// The checks of a holder whose member types carry the suffix `suffix`, as a consumer writes them:
/// one overload for each type a member names.
fn checks(suffix: &str) -> String {
    format!(
        r#"
fun ok(value: Zed{suffix}): List<String> = listOf("rating is ${{value.name}}")
fun ok(value: ZedBag{suffix}Draft): List<String> = listOf("other is ${{value.count}}")
fun ok(value: Tag{suffix}): List<String> = listOf("tag is ${{value.value}}")
fun ok(value: Served{suffix}): List<String> = listOf("served is ${{value.note}}")
fun ok(value: UInt): List<String> = listOf("size is $value")

fun main() {{
    val holder = Json.decodeFromString<Holder{suffix}Draft>("""{{"rating":"B","other":{{"count":1}},"tag":"x","served":{{"note":"n"}},"size":3}}""")
    holder.validationIssues().forEach {{ println("read: $it") }}
}}
"#
    )
}

#[test]
fn a_check_on_a_member_declared_below_the_holder_takes_the_draft() {
    assert!(below::built());
    compiled_and_run(&below::definitions(), "Below");
}

#[test]
fn a_check_on_a_member_declared_above_the_holder_takes_the_draft() {
    assert!(above::built());
    compiled_and_run(&above::definitions(), "Above");
}

#[test]
fn a_check_on_a_member_declared_in_another_crate_takes_the_draft() {
    assert!(apart::built());
    compiled_and_run(&apart::definitions(), "Apart");
}

#[test]
fn the_call_is_the_same_whatever_the_declaration_order() {
    for definition in [below::holder(), above::holder(), apart::holder()] {
        for member in ["rating", "other", "tag", "served"] {
            assert!(
                definition.contains(&format!("ok({member}.kotlinSchemaChecked())")),
                "{member}: {definition}"
            );
        }
        assert!(definition.contains("ok(size)"), "{definition}");
        assert!(!definition.contains("ok(rating.draft)"), "{definition}");
    }
}
