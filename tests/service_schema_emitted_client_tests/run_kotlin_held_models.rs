//! A holder reads each model it holds unjudged, whether the model is declared below the holder,
//! above it or in another crate: a holder that breaks a bound of its own and holds models that
//! break theirs answers every message, as Rust does. The emitted Kotlin is compiled with `-Werror`
//! and run.

#![cfg(feature = "kotlin")]

#[macro_use]
#[path = "run_kotlin_held_models/macros.rs"]
mod macros;

mod above {
    use super::{Debug, Deserialize, PAYLOAD, Serialize, from_str, model_schema};

    held_types! {
        check: "heldUnluckyAbove",
        crated: HeldCrateAbove,
        pair: HeldPairAbove,
        pick: HeldPickAbove,
        plain: HeldPlainAbove,
        served: HeldServedAbove,
        shape: HeldShapeAbove,
        stuff: HeldStuffAbove,
        tag: HeldTagAbove,
    }

    holder! {
        crated: HeldCrateAbove,
        holder: HolderAbove,
        pair: HeldPairAbove,
        pick: HeldPickAbove,
        plain: HeldPlainAbove,
        served: HeldServedAbove,
        shape: HeldShapeAbove,
        stuff: HeldStuffAbove,
        tag: HeldTagAbove,
    }

    pub fn held_unlucky_above(value: &HeldPickAboveDraft) -> Result<(), Vec<String>> {
        if matches!(value, HeldPickAboveDraft::B) {
            return Err(vec!["a pick is not B".to_owned()]);
        }
        Ok(())
    }

    pub fn answered() -> Vec<String> {
        from_str::<HolderAboveDraft>(PAYLOAD)
            .unwrap()
            .validate()
            .unwrap_err()
    }

    pub fn definitions() -> Vec<String> {
        vec![
            held_crate_above_kotlin::kotlin_definition(),
            held_pair_above_kotlin::kotlin_definition(),
            held_pick_above_kotlin::kotlin_definition(),
            held_plain_above_kotlin::kotlin_definition(),
            held_served_above_kotlin::kotlin_definition(),
            held_shape_above_kotlin::kotlin_definition(),
            held_stuff_above_kotlin::kotlin_definition(),
            held_tag_above_kotlin::kotlin_definition(),
            holder_above_kotlin::kotlin_definition(),
        ]
    }
}

mod aliased_apart {
    use kotlin_members_apart::{
        HeldListApart, HeldMapApart, HeldPairsApart, HeldStuffApart, held_list_apart_kotlin,
        held_map_apart_kotlin, held_pairs_apart_kotlin, held_stuff_apart_kotlin,
    };

    use super::{Debug, Deserialize, Serialize, model_schema};

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct HolderAliasedApart {
        pub listed: HeldListApart,
        pub mapped: HeldMapApart,
        pub pairs: HeldPairsApart<HeldStuffApart>,
    }

    pub fn definitions() -> Vec<String> {
        vec![
            held_list_apart_kotlin::kotlin_definition(),
            held_map_apart_kotlin::kotlin_definition(),
            held_pairs_apart_kotlin::kotlin_definition(),
            held_stuff_apart_kotlin::kotlin_definition(),
            holder_aliased_apart_kotlin::kotlin_definition(),
        ]
    }
}

mod aliased_below {
    use std::collections::HashMap;

    use super::{Debug, Deserialize, Serialize, from_str, model_schema};

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct HolderAliasedBelow {
        pub listed: HeldListBelow,
        pub mapped: HeldMapBelow,
        pub pairs: HeldPairsBelow<HeldNoteBelow>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ShelfCfgOff {
        #[model_schema_prop(minLength = 1)]
        pub label: String,
        #[cfg(any())]
        pub pairs: HeldPairsBelow<HeldNoteBelow>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ShelfOfHeld<Held> {
        #[model_schema_prop(minLength = 1)]
        pub label: String,
        pub pairs: HeldPairsBelow<Held>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ShelfUnderCfg {
        #[model_schema_prop(minLength = 1)]
        pub label: String,
        #[cfg(feature = "kotlin")]
        pub pairs: HeldPairsBelow<HeldNoteBelow>,
    }

    #[model_schema()]
    pub type HeldListBelow = Vec<HeldNoteBelow>;

    #[model_schema()]
    pub type HeldMapBelow = HashMap<String, HeldNoteBelow>;

    #[model_schema()]
    pub type HeldPairsBelow<T> = Vec<T>;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct HeldNoteBelow {
        #[model_schema_prop(minLength = 1)]
        pub name: String,
    }

    pub fn definitions() -> Vec<String> {
        vec![
            held_list_below_kotlin::kotlin_definition(),
            held_map_below_kotlin::kotlin_definition(),
            held_note_below_kotlin::kotlin_definition(),
            held_pairs_below_kotlin::kotlin_definition(),
            holder_aliased_below_kotlin::kotlin_definition(),
            shelf_cfg_off_kotlin::kotlin_definition(),
            shelf_of_held_kotlin::kotlin_definition(),
            shelf_under_cfg_kotlin::kotlin_definition(),
        ]
    }

    pub fn of_held(payload: &str) -> Result<ShelfOfHeldDraft<HeldNoteBelow>, serde_json::Error> {
        from_str::<ShelfOfHeldDraft<HeldNoteBelow>>(payload)
    }

    pub fn cfg_off(payload: &str) -> Result<ShelfCfgOffDraft, serde_json::Error> {
        from_str::<ShelfCfgOffDraft>(payload)
    }

    pub fn under_cfg(payload: &str) -> Result<ShelfUnderCfgDraft, serde_json::Error> {
        from_str::<ShelfUnderCfgDraft>(payload)
    }
}

mod apart {
    use kotlin_members_apart::{
        HeldCrateApart, HeldPairApart, HeldPickApart, HeldPlainApart, HeldShapeApart,
        HeldStuffApart, HeldTagApart, ServedApart, held_crate_apart_kotlin, held_pair_apart_kotlin,
        held_pick_apart_kotlin, held_plain_apart_kotlin, held_shape_apart_kotlin,
        held_stuff_apart_kotlin, held_tag_apart_kotlin, served_apart_kotlin,
    };

    use super::{Debug, Deserialize, PAYLOAD, Serialize, from_str, model_schema};

    holder! {
        crated: HeldCrateApart,
        holder: HolderApart,
        pair: HeldPairApart,
        pick: HeldPickApart,
        plain: HeldPlainApart,
        served: ServedApart,
        shape: HeldShapeApart,
        stuff: HeldStuffApart,
        tag: HeldTagApart,
    }

    pub fn answered() -> Vec<String> {
        from_str::<HolderApartDraft>(PAYLOAD)
            .unwrap()
            .validate()
            .unwrap_err()
    }

    pub fn definitions() -> Vec<String> {
        vec![
            held_crate_apart_kotlin::kotlin_definition(),
            held_pair_apart_kotlin::kotlin_definition(),
            held_pick_apart_kotlin::kotlin_definition(),
            held_plain_apart_kotlin::kotlin_definition(),
            held_shape_apart_kotlin::kotlin_definition(),
            held_stuff_apart_kotlin::kotlin_definition(),
            held_tag_apart_kotlin::kotlin_definition(),
            holder_apart_kotlin::kotlin_definition(),
            served_apart_kotlin::kotlin_definition(),
        ]
    }
}

mod below {
    use super::{Debug, Deserialize, PAYLOAD, Serialize, from_str, model_schema};

    holder! {
        crated: HeldCrateBelow,
        holder: HolderBelow,
        pair: HeldPairBelow,
        pick: HeldPickBelow,
        plain: HeldPlainBelow,
        served: HeldServedBelow,
        shape: HeldShapeBelow,
        stuff: HeldStuffBelow,
        tag: HeldTagBelow,
    }

    held_types! {
        check: "heldUnluckyBelow",
        crated: HeldCrateBelow,
        pair: HeldPairBelow,
        pick: HeldPickBelow,
        plain: HeldPlainBelow,
        served: HeldServedBelow,
        shape: HeldShapeBelow,
        stuff: HeldStuffBelow,
        tag: HeldTagBelow,
    }

    pub fn held_unlucky_below(value: &HeldPickBelowDraft) -> Result<(), Vec<String>> {
        if matches!(value, HeldPickBelowDraft::B) {
            return Err(vec!["a pick is not B".to_owned()]);
        }
        Ok(())
    }

    pub fn answered() -> Vec<String> {
        from_str::<HolderBelowDraft>(PAYLOAD)
            .unwrap()
            .validate()
            .unwrap_err()
    }

    pub fn definitions() -> Vec<String> {
        vec![
            held_crate_below_kotlin::kotlin_definition(),
            held_pair_below_kotlin::kotlin_definition(),
            held_pick_below_kotlin::kotlin_definition(),
            held_plain_below_kotlin::kotlin_definition(),
            held_served_below_kotlin::kotlin_definition(),
            held_shape_below_kotlin::kotlin_definition(),
            held_stuff_below_kotlin::kotlin_definition(),
            held_tag_below_kotlin::kotlin_definition(),
            holder_below_kotlin::kotlin_definition(),
        ]
    }
}

mod undrafted_checked {
    use super::{Debug, Deserialize, Serialize, from_str, model_schema};

    #[model_schema(__tixschema_undrafted)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Bolt {
        #[model_schema_prop(minLength = 1)]
        pub name: String,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Lathe {
        #[model_schema_prop(minLength = 1)]
        pub label: String,
        pub turn: Turn,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Mill {
        pub bolt: Bolt,
        #[model_schema_prop(minLength = 1)]
        pub label: String,
    }

    #[model_schema(__tixschema_undrafted, check = ["evenTurns"])]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum Turn {
        Even,
        Odd,
    }

    pub fn answered(payload: &str) -> Vec<String> {
        from_str::<LatheDraft>(payload)
            .unwrap()
            .validate()
            .err()
            .unwrap_or_default()
    }

    pub fn definitions() -> Vec<String> {
        vec![
            lathe_kotlin::kotlin_definition(),
            turn_kotlin::kotlin_definition(),
        ]
    }

    pub fn even_turns(value: &Turn) -> Result<(), Vec<String>> {
        if matches!(value, Turn::Odd) {
            return Err(vec!["a turn is not odd".to_owned()]);
        }
        Ok(())
    }

    pub fn mill_definitions() -> Vec<String> {
        vec![
            bolt_kotlin::kotlin_definition(),
            mill_kotlin::kotlin_definition(),
        ]
    }

    pub fn milled(payload: &str) -> Vec<String> {
        from_str::<MillDraft>(payload)
            .unwrap()
            .validate()
            .err()
            .unwrap_or_default()
    }
}

use core::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tixschema::model_schema;

use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// A payload of the holders of an alias of a list and of a map.
const ALIASED: &str =
    r#"{"listed":[{"name":"a"}],"mapped":{"k":{"name":"b"}},"pairs":[{"name":"c"}]}"#;

/// A payload that breaks a bound of the holder and of every model it holds.
const PAYLOAD: &str = r#"{"crated":{"content":1,"label":""},"label":"","pair":["",1],"pick":"B","plain":"A","served":{"note":"n"},"shape":{"kind":"Named","name":""},"stuff":{"name":""},"tag":"x"}"#;

/// The Kotlin that reads `payload` as the holder of `suffix` and prints each message it throws.
fn checks(suffix: &str, payload: &str) -> String {
    format!(
        r#"
fun heldUnlucky{suffix}(value: HeldPick{suffix}Draft): List<String> =
    if (value == HeldPick{suffix}Draft.B) listOf("a pick is not B") else emptyList()

fun main() {{
    try {{
        Json.decodeFromString<Holder{suffix}>("""{payload}""")
        println("read")
    }} catch (refused: Holder{suffix}Refusal) {{
        refused.issues.forEach {{ println("refused: $it") }}
    }}
}}
"#
    )
}

/// Compiles and runs the Kotlin of `definitions`, and holds what the read of [`PAYLOAD`] throws to
/// `expected`, the messages Rust answers.
fn answers(definitions: &[String], suffix: &str, expected: &[String]) {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions.iter().cloned());
    parts.push(checks(suffix, PAYLOAD));
    let Some(printed) = ran_kotlin(&parts.join("\n\n")) else {
        return;
    };
    let lines: Vec<&str> = printed.lines().collect();
    let wanted: Vec<String> = expected
        .iter()
        .map(|message| format!("refused: {message}"))
        .collect();
    assert_eq!(lines, wanted);
}

#[test]
fn a_holder_above_its_models_answers_every_message() {
    let rust = above::answered();
    assert_eq!(rust.len(), 7, "{rust:?}");
    answers(&above::definitions(), "Above", &rust);
}

#[test]
fn a_holder_apart_from_its_models_answers_every_message() {
    let rust = apart::answered();
    assert_eq!(rust.len(), 7, "{rust:?}");
    answers(&apart::definitions(), "Apart", &rust);
}

#[test]
fn a_holder_below_its_models_answers_every_message() {
    let rust = below::answered();
    assert_eq!(rust.len(), 7, "{rust:?}");
    answers(&below::definitions(), "Below", &rust);
}

#[test]
fn a_holder_of_an_alias_of_a_list_and_of_a_map_compiles_below_and_apart() {
    from_str::<aliased_below::HolderAliasedBelowDraft>(ALIASED).unwrap();
    from_str::<aliased_apart::HolderAliasedApartDraft>(ALIASED).unwrap();
    for (definitions, suffix) in [
        (aliased_below::definitions(), "Below"),
        (aliased_apart::definitions(), "Apart"),
    ] {
        let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
        parts.extend(definitions);
        parts.push(format!(
            "fun main() {{ println(Json.decodeFromString<HolderAliased{suffix}>(\"\"\"{ALIASED}\"\"\").listed.size) }}"
        ));
        if let Some(printed) = ran_kotlin(&parts.join("\n\n")) {
            assert_eq!(printed.trim(), "1");
        }
    }
}

/// The Kotlin that reads `payload` as `decoded`, the type of the holder `refusal` names the
/// refusal of, and prints each message it throws, or `read`.
fn reads(
    definitions: &[String],
    (decoded, refusal): (&str, &str),
    payload: &str,
    functions: &str,
) -> Option<Vec<String>> {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions.iter().cloned());
    parts.push(format!(
        r#"
{functions}

fun main() {{
    try {{
        Json.decodeFromString<{decoded}>("""{payload}""")
        println("read")
    }} catch (refused: {refusal}Refusal) {{
        refused.issues.forEach {{ println("refused: $it") }}
    }} catch (refused: SerializationException) {{
        println("unread: ${{refused.message}}")
    }}
}}
"#
    ));
    ran_kotlin(&parts.join("\n\n")).map(|printed| printed.lines().map(str::to_owned).collect())
}

#[test]
fn a_holder_of_an_undrafted_type_that_has_a_check_answers_the_holders_message() {
    let payload = r#"{"label":"","turn":"Odd"}"#;
    let rust = undrafted_checked::answered(payload);
    assert_eq!(rust.len(), 2, "{rust:?}");
    let functions = "fun evenTurns(value: Turn): List<String> = \
         if (value == Turn.Odd) listOf(\"a turn is not odd\") else emptyList()";
    let kotlin = reads(
        &undrafted_checked::definitions(),
        ("Lathe", "Lathe"),
        payload,
        functions,
    );
    if let Some(lines) = kotlin {
        let wanted: Vec<String> = rust
            .iter()
            .map(|message| format!("refused: {message}"))
            .collect();
        assert_eq!(lines, wanted);
    }
    let even = r#"{"label":"a","turn":"Even"}"#;
    if let Some(lines) = reads(
        &undrafted_checked::definitions(),
        ("Lathe", "Lathe"),
        even,
        functions,
    ) {
        assert_eq!(lines, ["read"]);
    }
}

#[test]
fn a_holder_of_an_undrafted_struct_that_has_a_rule_answers_the_holders_message() {
    let payload = r#"{"bolt":{"name":""},"label":""}"#;
    let rust = undrafted_checked::milled(payload);
    assert_eq!(rust.len(), 2, "{rust:?}");
    let kotlin = reads(
        &undrafted_checked::mill_definitions(),
        ("Mill", "Mill"),
        payload,
        "",
    );
    if let Some(lines) = kotlin {
        let wanted: Vec<String> = rust
            .iter()
            .map(|message| format!("refused: {message}"))
            .collect();
        assert_eq!(lines, wanted);
    }
    let read = r#"{"bolt":{"name":"a"},"label":"b"}"#;
    if let Some(lines) = reads(
        &undrafted_checked::mill_definitions(),
        ("Mill", "Mill"),
        read,
        "",
    ) {
        assert_eq!(lines, ["read"]);
    }
}

#[test]
fn a_holder_of_an_alias_under_a_cfg_or_filled_by_its_parameter_reads_the_alias_below() {
    let broken = r#"{"label":"","pairs":[{"name":""}]}"#;
    let label = r#"{"label":"","pairs":[{"name":"x"}]}"#;
    let intact = r#"{"label":"a","pairs":[{"name":"b"}]}"#;
    let definitions = aliased_below::definitions();
    let holders = [
        ("ShelfUnderCfg", "ShelfUnderCfg"),
        ("ShelfOfHeld<HeldNoteBelow>", "ShelfOfHeld"),
    ];

    // Rust reads the model inside the alias judged, as it is read, so the holder's own `label` is
    // never reached where the model breaks.
    for payload in [broken, label] {
        let rust = [
            aliased_below::under_cfg(payload)
                .err()
                .map(|error| error.to_string()),
            aliased_below::of_held(payload)
                .err()
                .map(|error| error.to_string()),
        ];
        assert!(
            rust.iter().all(|refused| {
                refused.as_deref().is_none_or(|text| {
                    text.starts_with("'name': too short: minimum length is 1, got 0")
                })
            }),
            "{rust:?}"
        );
    }
    // A member under a `cfg` that is off is absent from the Rust type, and the Kotlin of the
    // holder, which asks nothing of it, compiles with the held alias declared below.
    aliased_below::cfg_off(r#"{"label":"a"}"#).unwrap();
    let off = ("ShelfCfgOff", "ShelfCfgOff");
    if let Some(lines) = reads(&definitions, off, intact, "") {
        assert_eq!(lines.len(), 1, "{lines:?}");
    }
    for names in holders {
        if let Some(lines) = reads(&definitions, names, broken, "") {
            assert_eq!(
                lines,
                ["refused: 'pairs.0.name': too short: minimum length is 1, got 0"],
                "{}",
                names.0
            );
        }
        if let Some(lines) = reads(&definitions, names, label, "") {
            assert_eq!(
                lines,
                ["refused: 'label': too short: minimum length is 1, got 0"],
                "{}",
                names.0
            );
        }
        if let Some(lines) = reads(&definitions, names, intact, "") {
            assert_eq!(lines, ["read"], "{}", names.0);
        }
    }
}
