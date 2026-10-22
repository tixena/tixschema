//! A key no member of a type reads, read by the serializer the Kotlin emitter writes, compiled with
//! `-Werror`, under the default `Json` and under `Json { ignoreUnknownKeys = true }`, beside what
//! serde reads and what the generated Zod schema answers of the same payload. The default `Json`
//! refuses the key, as Zod and the plugin's own serializer refuse it, and the other reads past it,
//! as the plugin's reads past it. A flattened map, and a flattened type whose keys are not known,
//! take every key they are handed.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::flattened_models::{
    BELTING, Belting, DRESSING, Dressing, GAFFING, Gaffing, Netting, PAYLOAD, STOWING, SWAYING,
    Slinging, Stowing, Swaying, WHELPING, Whelping, belting_kotlin, dressing_kotlin, ensign_kotlin,
    gaffing_kotlin, netting_kotlin, slinging_kotlin, stowing_kotlin, swaying_kotlin,
    tacking_kotlin, whelping_kotlin,
};
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::flattened_models::{Ensign, Tacking};
use super::runtime::ran_kotlin;
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::tuple_variants_models::Labelling;
use super::tuple_variants_models::labelling_kotlin;
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::unwritten_models::Braiding;
use super::unwritten_models::{
    Dousing, Furling, Ratcheting, Reefing, Shackling, Wearing, braiding_kotlin, dousing_kotlin,
    furling_kotlin, ratcheting_kotlin, reefing_kotlin, shackling_kotlin, wearing_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the read under the default `Json`, then under one that ignores unknown keys,
/// each the value written back or the first line of the message of what the read throws.
const READ: &str = r#"
val ignoring = Json { ignoreUnknownKeys = true }

fun <T> unknown(at: String, serializer: KSerializer<T>, payload: String) {
    val told = listOf(Json, ignoring).joinToString(" | ") { json ->
        try {
            "read " + json.encodeToString(serializer, json.decodeFromString(serializer, payload))
        } catch (refusal: SerializationException) {
            "refused: " + (refusal.message ?: "").lines().first()
        }
    }
    println("$at: $told")
}
"#;

/// What the Kotlin read answers of a flattened enum's entries under two variants' names, under
/// either `Json`, as TypeScript and Zod refuse them.
const TWO_VARIANTS: &str =
    "refused: more than one variant of enum Tacking found in flattened data: Bay, Dock";

/// Every payload, beside what serde, Zod and the Kotlin read answer of it: a key no member reads
/// in a struct, in a variant under each tagging and beside a flattened struct, where the struct
/// and the struct it flattens are under their own tag, and beside a flattened enum; the payloads
/// serde writes of those, which hold none; a key a flattened map reads, and one handed to a
/// flattened type whose keys are not known; a key no member reads in a model held one and two
/// levels inside a flattened struct, in the variant of a flattened enum and as a flattened map's
/// value; and a key beside the tag of an enum under a tag alone, of each shape of variant, and
/// beside the tag and the content of one under a tag and a content key.
const ROWS: [Row; 34] = [
    Row {
        ignoring: "read {\"label\":\"north\"}",
        json: r#"{"label":"north","other":1}"#,
        kotlin: UNKNOWN,
        named: "Furling",
        serde: written_back::<Furling>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"Furled\":{\"label\":\"north\"}}",
        json: r#"{"Furled":{"label":"north","other":1}}"#,
        kotlin: UNKNOWN,
        named: "Dousing",
        serde: written_back::<Dousing>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Furled\",\"label\":\"north\"}",
        json: r#"{"kind":"Furled","label":"north","other":1}"#,
        kotlin: UNKNOWN,
        named: "Wearing",
        serde: written_back::<Wearing>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Furled\",\"load\":{\"label\":\"north\"}}",
        json: r#"{"kind":"Furled","load":{"label":"north","other":1}}"#,
        kotlin: UNKNOWN,
        named: "Shackling",
        serde: written_back::<Shackling>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"label\":\"north\"}",
        json: r#"{"label":"north","other":1}"#,
        kotlin: "refused: No variant of Ratcheting matched",
        named: "Ratcheting",
        serde: written_back::<Ratcheting>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"label\":\"north\"}",
        json: r#"{"label":"north","spare":3,"other":1}"#,
        kotlin: UNKNOWN,
        named: "Reefing",
        serde: written_back::<Reefing>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Garnetting\",\"label\":\"north\"}",
        json: r#"{"kind":"Garnetting","label":"north","spare":3}"#,
        kotlin: "read {\"kind\":\"Garnetting\",\"label\":\"north\"}",
        named: "Garnetting",
        serde: written_back::<Garnetting>,
        zod: "read {\"kind\":\"Garnetting\",\"label\":\"north\",\"spare\":3}",
    },
    Row {
        ignoring: "read {\"kind\":\"Garnetting\",\"label\":\"north\"}",
        json: r#"{"kind":"Garnetting","label":"north","other":1}"#,
        kotlin: UNKNOWN,
        named: "Garnetting",
        serde: written_back::<Garnetting>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Ensign\",\"label\":\"n\",\"north\":2}",
        json: DRESSING,
        kotlin: "read {\"kind\":\"Ensign\",\"label\":\"n\",\"north\":2}",
        named: "Dressing",
        serde: written_back::<Dressing>,
        zod: "read {\"north\":2,\"kind\":\"Ensign\",\"label\":\"n\"}",
    },
    Row {
        ignoring: "read {\"kind\":\"Ensign\",\"label\":\"n\",\"north\":2}",
        json: r#"{"north":2,"label":"n","kind":"Ensign","other":1}"#,
        kotlin: UNKNOWN,
        named: "Dressing",
        serde: written_back::<Dressing>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}}",
        json: STOWING[2],
        kotlin: "refused: Encountered an unknown key 'north'",
        named: "Stowing",
        serde: written_back::<Stowing>,
        zod: "refused",
    },
    Row {
        ignoring: TWO_VARIANTS,
        json: STOWING[3],
        kotlin: TWO_VARIANTS,
        named: "Stowing",
        serde: written_back::<Stowing>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"note\":\"a\"}",
        json: BELTING[1],
        kotlin: "refused: Encountered an unknown key 'Other'",
        named: "Belting",
        serde: written_back::<Belting>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"label\":\"n\"}",
        json: PAYLOAD,
        kotlin: "refused: Encountered an unknown key 'north'",
        named: "Slinging",
        serde: written_back::<Slinging>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"Labelled\":{\"label\":\"n\",\"note\":\"a\"}}",
        json: GAFFING[7],
        kotlin: "refused: Encountered an unknown key 'north'",
        named: "Gaffing",
        serde: written_back::<Gaffing>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Hoisted\",\"label\":\"n\"}",
        json: WHELPING[3],
        kotlin: "refused: Encountered an unknown key 'north'",
        named: "Whelping",
        serde: written_back::<Whelping>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"label\":\"n\",\"note\":\"a\"}",
        json: SWAYING[5],
        kotlin: "read {}",
        named: "Swaying",
        serde: written_back::<Swaying>,
        zod: "read {}",
    },
    Row {
        ignoring: "read {\"other\":1}",
        json: r#"{"other":1}"#,
        kotlin: "read {\"other\":1}",
        named: "Netting",
        serde: written_back::<Netting>,
        zod: "read {\"other\":1}",
    },
    Row {
        ignoring: "read {\"label\":\"north\",\"note\":\"a\"}",
        json: r#"{"label":"north","note":"a","other":1}"#,
        kotlin: "read {\"label\":\"north\",\"note\":\"a\"}",
        named: "Parcelling",
        serde: written_back::<Parcelling>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"label\":\"north\"}",
        json: r#"{"label":"north","spare":3}"#,
        kotlin: "read {\"label\":\"north\"}",
        named: "Furling",
        serde: written_back::<Furling>,
        zod: "read {\"label\":\"north\",\"spare\":3}",
    },
    Row {
        ignoring: "read {\"kind\":\"Furled\",\"label\":\"north\"}",
        json: r#"{"kind":"Furled","label":"north","spare":3}"#,
        kotlin: "read {\"kind\":\"Furled\",\"label\":\"north\"}",
        named: "Wearing",
        serde: written_back::<Wearing>,
        zod: "read {\"kind\":\"Furled\",\"label\":\"north\",\"spare\":3}",
    },
    Row {
        ignoring: BITTERED,
        json: concat!(
            r#"{"note":"a","held":{"label":"x","other":1},"label":"n","#,
            r#""rail":{"ringbolt":{"label":"y"}}}"#
        ),
        kotlin: UNKNOWN_AT,
        named: "Bittering",
        serde: written_back::<Bittering>,
        zod: "refused",
    },
    Row {
        ignoring: BITTERED,
        json: concat!(
            r#"{"note":"a","held":{"label":"x"},"label":"n","#,
            r#""rail":{"ringbolt":{"label":"y","other":1}}}"#
        ),
        kotlin: UNKNOWN_AT,
        named: "Bittering",
        serde: written_back::<Bittering>,
        zod: "refused",
    },
    Row {
        ignoring: BITTERED,
        json: r#"{"note":"a","held":{"label":"x"},"label":"n","rail":{"ringbolt":{"label":"y"}}}"#,
        kotlin: BITTERED,
        named: "Bittering",
        serde: written_back::<Bittering>,
        zod: BITTERED,
    },
    Row {
        ignoring: "read {\"note\":\"a\",\"Fast\":{\"label\":\"n\"}}",
        json: r#"{"note":"a","Fast":{"label":"n","other":1}}"#,
        kotlin: UNKNOWN_AT,
        named: "Snatching",
        serde: written_back::<Snatching>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"north\":{\"label\":\"n\"}}",
        json: r#"{"north":{"label":"n","other":1}}"#,
        kotlin: UNKNOWN_AT,
        named: "Halyarding",
        serde: written_back::<Halyarding>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Lull\"}",
        json: r#"{"kind":"Lull","other":1}"#,
        kotlin: UNKNOWN,
        named: "Downhaul",
        serde: written_back::<Downhaul>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Gust\",\"speed\":1}",
        json: r#"{"kind":"Gust","speed":1,"other":1}"#,
        kotlin: UNKNOWN_AT,
        named: "Downhaul",
        serde: written_back::<Downhaul>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Reefed\",\"note\":\"a\",\"label\":\"n\"}",
        json: r#"{"kind":"Reefed","label":"n","note":"a","other":1}"#,
        kotlin: UNKNOWN,
        named: "Downhaul",
        serde: written_back::<Downhaul>,
        zod: "refused",
    },
    Row {
        ignoring: "refused: Field 'value' is required for type with serial name 'Held', but it was \
                   missing at path: $",
        json: r#"{"kind":"Held","label":"x","other":1}"#,
        kotlin: "refused: Encountered an unknown key 'label' at path: $",
        named: "Downhaul",
        serde: written_back::<Downhaul>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Lull\"}",
        json: r#"{"kind":"Lull"}"#,
        kotlin: "read {\"kind\":\"Lull\"}",
        named: "Downhaul",
        serde: written_back::<Downhaul>,
        zod: "read {\"kind\":\"Lull\"}",
    },
    Row {
        ignoring: "read {\"kind\":\"Lull\"}",
        json: r#"{"kind":"Lull","other":1}"#,
        kotlin: UNKNOWN,
        named: "Downhauling",
        serde: written_back::<Downhauling>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Gust\",\"load\":{\"speed\":1}}",
        json: r#"{"kind":"Gust","load":{"speed":1},"other":1}"#,
        kotlin: UNKNOWN,
        named: "Downhauling",
        serde: written_back::<Downhauling>,
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Gust\",\"load\":{\"speed\":1}}",
        json: r#"{"kind":"Gust","load":{"speed":1}}"#,
        kotlin: "read {\"kind\":\"Gust\",\"load\":{\"speed\":1}}",
        named: "Downhauling",
        serde: written_back::<Downhauling>,
        zod: "read {\"kind\":\"Gust\",\"load\":{\"speed\":1}}",
    },
];

/// What a lenient `Json` reads of each payload of [`Bittering`], and the default `Json` of the one
/// that holds no key no member reads.
const BITTERED: &str = "read {\"note\":\"a\",\"held\":{\"label\":\"x\"},\"label\":\"n\",\
     \"rail\":{\"ringbolt\":{\"label\":\"y\"}}}";

/// What the default `Json` throws for the key `other`, in the words of the plugin's serializer.
const UNKNOWN: &str = "refused: Encountered an unknown key 'other'";

/// What the plugin's serializer throws for the key `other`, with the path it was read at.
const UNKNOWN_AT: &str = "refused: Encountered an unknown key 'other' at path: $";

/// [`Furling`] under its own tag: a struct the Kotlin emitter writes the serializer of, whose tag
/// is a key it reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Garnetting {
    pub label: String,
    #[serde(default, skip_serializing)]
    pub spare: Option<u32>,
}

/// The member [`Trucking`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Clewline {
    pub label: String,
}

/// A struct with a flattened member of its own, which [`Parcelling`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trucking {
    #[serde(flatten)]
    pub clewline: Clewline,
    pub note: String,
}

/// [`Trucking`] flattened: the Kotlin emitter knows no key it reads, so its holder takes every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parcelling {
    #[serde(flatten)]
    pub trucking: Trucking,
}

/// A model a model holds, which [`Thimbling`] holds bare and one level down.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ringbolt {
    pub label: String,
}

/// A model that holds [`Ringbolt`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pinrail {
    pub ringbolt: Ringbolt,
}

/// A struct whose keys the Kotlin emitter knows, holding a model one level and two levels down.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Thimbling {
    pub held: Ringbolt,
    pub label: String,
    pub rail: Pinrail,
}

/// [`Thimbling`] flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bittering {
    pub note: String,
    #[serde(flatten)]
    pub thimbling: Thimbling,
}

/// An externally tagged enum of one variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Snatch {
    Fast { label: String },
}

/// [`Snatch`] flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snatching {
    pub note: String,
    #[serde(flatten)]
    pub snatch: Snatch,
}

/// A map of models flattened, the one member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Halyarding {
    #[serde(flatten)]
    pub lines: BTreeMap<String, Ringbolt>,
}

/// An enum under a tag alone: a unit variant, one of members, one of a struct and one of a
/// flattened member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Downhaul {
    Gust {
        speed: u32,
    },
    Held(Ringbolt),
    Lull,
    Reefed {
        note: String,
        #[serde(flatten)]
        ringbolt: Ringbolt,
    },
}

/// An enum under a tag and a content key: a variant of members and a unit variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "load")]
pub enum Downhauling {
    Gust { speed: u32 },
    Lull,
}

/// One payload, the type it is read as, and what each read answers of it.
struct Row {
    /// What the Kotlin read under `Json { ignoreUnknownKeys = true }` answers.
    ignoring: &'static str,
    json: &'static str,
    /// What the Kotlin read under the default `Json` answers.
    kotlin: &'static str,
    /// The name the type is published under, in Kotlin and in Zod.
    named: &'static str,
    /// What serde answers of the payload.
    serde: fn(&str) -> String,
    /// What the generated Zod schema answers of the payload: what it reads, or `refused`.
    zod: &'static str,
}

/// What serde writes back of `json` read as `T`, or `refused`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| format!("read {}", serde_json::to_string(&read).unwrap()),
    )
}

/// serde reads every payload, past the keys no member reads.
#[test]
fn serde_reads_past_a_key_no_member_reads() {
    let serde_told: Vec<String> = ROWS.iter().map(|row| (row.serde)(row.json)).collect();
    assert_eq!(
        serde_told,
        [
            "read {\"label\":\"north\"}",
            "read {\"Furled\":{\"label\":\"north\"}}",
            "read {\"kind\":\"Furled\",\"label\":\"north\"}",
            "read {\"kind\":\"Furled\",\"load\":{\"label\":\"north\"}}",
            "read {\"label\":\"north\"}",
            "read {\"label\":\"north\"}",
            "read {\"kind\":\"Garnetting\",\"label\":\"north\"}",
            "read {\"kind\":\"Garnetting\",\"label\":\"north\"}",
            "read {\"kind\":\"Ensign\",\"label\":\"n\",\"north\":2}",
            "read {\"kind\":\"Ensign\",\"label\":\"n\",\"north\":2}",
            "read {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}}",
            "read {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}}",
            "read {\"note\":\"a\"}",
            "read {\"label\":\"n\"}",
            "read {\"Labelled\":{\"label\":\"n\",\"note\":\"a\"}}",
            "read {\"kind\":\"Hoisted\",\"label\":\"n\"}",
            "read {\"label\":\"n\",\"note\":\"a\"}",
            "read {\"other\":1}",
            "read {\"label\":\"north\",\"note\":\"a\"}",
            "read {\"label\":\"north\"}",
            "read {\"kind\":\"Furled\",\"label\":\"north\"}",
            BITTERED,
            BITTERED,
            BITTERED,
            "read {\"note\":\"a\",\"Fast\":{\"label\":\"n\"}}",
            "read {\"north\":{\"label\":\"n\"}}",
            "read {\"kind\":\"Lull\"}",
            "read {\"kind\":\"Gust\",\"speed\":1}",
            "read {\"kind\":\"Reefed\",\"note\":\"a\",\"label\":\"n\"}",
            "read {\"kind\":\"Held\",\"label\":\"x\"}",
            "read {\"kind\":\"Lull\"}",
            "read {\"kind\":\"Lull\"}",
            "read {\"kind\":\"Gust\",\"load\":{\"speed\":1}}",
            "read {\"kind\":\"Gust\",\"load\":{\"speed\":1}}",
        ],
        "serde no longer reads each payload as it did"
    );
}

/// The Kotlin read refuses a key no member reads under the default `Json`, where Zod refuses it,
/// and reads past it under `Json { ignoreUnknownKeys = true }`. A payload serde writes holds no
/// such key, and is read under both.
#[test]
fn a_key_no_member_reads_is_refused_in_kotlin_under_the_default_json() {
    let mut expected = String::new();
    let mut main = String::from(READ);
    main.push_str("fun main() {\n");
    for (at, row) in ROWS.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {} | {}", row.kotlin, row.ignoring);
        let _ = writeln!(
            main,
            "    unknown(\"{at}\", {}.serializer(), \"\"\"{}\"\"\")",
            row.named, row.json
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        braiding_kotlin::kotlin_definition(),
        furling_kotlin::kotlin_definition(),
        reefing_kotlin::kotlin_definition(),
        dousing_kotlin::kotlin_definition(),
        wearing_kotlin::kotlin_definition(),
        shackling_kotlin::kotlin_definition(),
        ratcheting_kotlin::kotlin_definition(),
        garnetting_kotlin::kotlin_definition(),
        clewline_kotlin::kotlin_definition(),
        trucking_kotlin::kotlin_definition(),
        parcelling_kotlin::kotlin_definition(),
        labelling_kotlin::kotlin_definition(),
        gaffing_kotlin::kotlin_definition(),
        swaying_kotlin::kotlin_definition(),
        whelping_kotlin::kotlin_definition(),
        slinging_kotlin::kotlin_definition(),
        tacking_kotlin::kotlin_definition(),
        stowing_kotlin::kotlin_definition(),
        belting_kotlin::kotlin_definition(),
        ensign_kotlin::kotlin_definition(),
        dressing_kotlin::kotlin_definition(),
        netting_kotlin::kotlin_definition(),
        ringbolt_kotlin::kotlin_definition(),
        pinrail_kotlin::kotlin_definition(),
        thimbling_kotlin::kotlin_definition(),
        bittering_kotlin::kotlin_definition(),
        snatch_kotlin::kotlin_definition(),
        snatching_kotlin::kotlin_definition(),
        halyarding_kotlin::kotlin_definition(),
        downhaul_kotlin::kotlin_definition(),
        downhauling_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// The generated Zod schemas, and what prints each row's verdict.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_module() -> String {
    let mut module = [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        Braiding::ts_definition(),
        Braiding::zod_schema(),
        Furling::ts_definition(),
        Furling::zod_schema(),
        Reefing::ts_definition(),
        Reefing::zod_schema(),
        Dousing::ts_definition(),
        Dousing::zod_schema(),
        Wearing::ts_definition(),
        Wearing::zod_schema(),
        Shackling::ts_definition(),
        Shackling::zod_schema(),
        Ratcheting::ts_definition(),
        Ratcheting::zod_schema(),
        Garnetting::ts_definition(),
        Garnetting::zod_schema(),
        Clewline::ts_definition(),
        Clewline::zod_schema(),
        Trucking::ts_definition(),
        Trucking::zod_schema(),
        Parcelling::ts_definition(),
        Parcelling::zod_schema(),
        Labelling::ts_definition(),
        Labelling::zod_schema(),
        Gaffing::ts_definition(),
        Gaffing::zod_schema(),
        Swaying::ts_definition(),
        Swaying::zod_schema(),
        Whelping::ts_definition(),
        Whelping::zod_schema(),
        Slinging::ts_definition(),
        Slinging::zod_schema(),
        Tacking::ts_definition(),
        Tacking::zod_schema(),
        Stowing::ts_definition(),
        Stowing::zod_schema(),
        Belting::ts_definition(),
        Belting::zod_schema(),
        Ensign::ts_definition(),
        Ensign::zod_schema(),
        Dressing::ts_definition(),
        Dressing::zod_schema(),
        Netting::ts_definition(),
        Netting::zod_schema(),
        Ringbolt::ts_definition(),
        Ringbolt::zod_schema(),
        Pinrail::ts_definition(),
        Pinrail::zod_schema(),
        Thimbling::ts_definition(),
        Thimbling::zod_schema(),
        Bittering::ts_definition(),
        Bittering::zod_schema(),
        Snatch::ts_definition(),
        Snatch::zod_schema(),
        Snatching::ts_definition(),
        Snatching::zod_schema(),
        Halyarding::ts_definition(),
        Halyarding::zod_schema(),
        Downhaul::ts_definition(),
        Downhaul::zod_schema(),
        Downhauling::ts_definition(),
        Downhauling::zod_schema(),
    ]
    .join("\n\n");
    module.push_str("\n\nconst told = [];\n");
    for row in &ROWS {
        let _ = writeln!(
            module,
            "told.push(((parsed) => parsed.success ? \"read \" + JSON.stringify(parsed.data) : \
             \"refused\")({}$Schema.safeParse(JSON.parse({}))));",
            row.named,
            serde_json::to_string(row.json).unwrap()
        );
    }
    module.push_str("console.log(JSON.stringify(told));\n");
    module
}

/// The generated Zod schema refuses each payload that holds a key no member reads, a flattened
/// type whose keys the Kotlin emitter does not know among them, and reads the others.
#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn zod_refuses_a_key_no_member_reads() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(
            REQUIRED,
            "the generated Zod schemas of a key no member reads",
        );
        return;
    };
    let Some(printed) = ran_with_modules(
        "unknown-keys",
        "TIXSCHEMA_NODE",
        "node",
        "unknown.mts",
        &zod_module(),
        &modules,
    ) else {
        return;
    };
    let told: Vec<String> = serde_json::from_str(printed.trim()).unwrap();
    let pinned: Vec<&str> = ROWS.iter().map(|row| row.zod).collect();
    assert_eq!(
        told, pinned,
        "the Zod schema no longer answers each payload as it did"
    );
}

/// The README's sentence on a key no member reads shows the payload and the message of the first
/// row the run above measures.
#[test]
fn the_readme_shows_the_refusal_the_run_measures() {
    let readme = include_str!("../../README.md");
    let thrown = ROWS[0].kotlin.strip_prefix("refused: ").unwrap();
    let shown = format!(
        "`{}` is refused with `SerializationException: {thrown}`",
        ROWS[0].json
    );
    assert!(
        readme.contains(&shown),
        "the README no longer shows {shown}"
    );
    assert!(
        readme.contains(
            "`struct Furling { label: String, #[serde(default, skip_serializing)] spare: \
             Option<u32> }`"
        ),
        "the README no longer declares the struct the row reads"
    );
}

/// The README's sentences on a model held inside a flattened struct and on an enum under a tag key
/// show payloads the run measures, refused under the default `Json`.
#[test]
fn the_readme_shows_the_nested_and_tagged_refusals_the_run_measures() {
    let readme = include_str!("../../README.md");
    for (at, json) in [
        (22, ROWS[22].json),
        (32, r#"{"kind":"Gust","load":{"speed":1},"other":1}"#),
        (31, r#"{"kind":"Lull","other":1}"#),
        (26, r#"{"kind":"Lull","other":1}"#),
    ] {
        assert!(
            ROWS[at].json == json && ROWS[at].kotlin.starts_with("refused"),
            "row {at} is not the refusal the README shows"
        );
        let shown = format!("`{json}`");
        assert!(readme.contains(&shown), "not in the README: {shown}");
    }
}
