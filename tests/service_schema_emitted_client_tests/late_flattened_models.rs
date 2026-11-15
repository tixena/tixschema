//! The holders of `run_late_flattened.rs`, each written three times: with the types it flattens
//! declared above it, below it, and in `flattened_members_apart`, another crate. A holder is
//! written before it can see a type below it or in another crate, so each layout is a different
//! expansion of the same declaration.

use above::{
    late_enum_above_dart, late_enum_above_swift, late_hitch_above_dart, late_hitch_above_swift,
    late_knot_above_dart, late_knot_above_swift, late_loop_above_dart, late_loop_above_swift,
    late_loose_above_dart, late_loose_above_swift, late_loose_enum_above_dart,
    late_loose_enum_above_swift, late_named_above_dart, late_named_above_swift,
    late_option_above_dart, late_option_above_swift, late_option_enum_above_dart,
    late_option_enum_above_swift, late_rope_above_dart, late_rope_above_swift,
    late_structs_above_dart, late_structs_above_swift, late_tail_above_dart, late_tail_above_swift,
};
use apart::{
    late_enum_apart_dart, late_enum_apart_swift, late_loose_apart_dart, late_loose_apart_swift,
    late_loose_enum_apart_dart, late_loose_enum_apart_swift, late_named_apart_dart,
    late_named_apart_swift, late_option_apart_dart, late_option_apart_swift,
    late_option_enum_apart_dart, late_option_enum_apart_swift, late_structs_apart_dart,
    late_structs_apart_swift,
};
use below::{
    late_enum_below_dart, late_enum_below_swift, late_hitch_below_dart, late_hitch_below_swift,
    late_knot_below_dart, late_knot_below_swift, late_loop_below_dart, late_loop_below_swift,
    late_loose_below_dart, late_loose_below_swift, late_loose_enum_below_dart,
    late_loose_enum_below_swift, late_named_below_dart, late_named_below_swift,
    late_option_below_dart, late_option_below_swift, late_option_enum_below_dart,
    late_option_enum_below_swift, late_rope_below_dart, late_rope_below_swift,
    late_structs_below_dart, late_structs_below_swift, late_tail_below_dart, late_tail_below_swift,
};
mod above {
    use super::*;

    /// A struct that refuses no key, flattened beside a struct.
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateLoopAbove {
        pub strand: String,
        pub turns: u32,
    }

    /// A struct that refuses no key, flattened after a struct.
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateTailAbove {
        pub tail: String,
    }

    /// A struct that refuses a key it does not declare.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateRopeAbove {
        pub strand: String,
        pub turns: u32,
    }

    /// An externally tagged enum.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum LateKnotAbove {
        Bowline { loops: u32 },
        Sheet { label: String },
    }

    /// An externally tagged enum with a rename and an alias.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum LateHitchAbove {
        Bowline {
            loops: u32,
        },
        #[serde(alias = "hitch")]
        Hitch {
            turns: u32,
        },
        #[serde(rename = "sheet")]
        Sheet {
            label: String,
        },
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateStructsAbove {
        #[serde(flatten)]
        pub rope: LateLoopAbove,
        #[serde(flatten)]
        pub tail: LateTailAbove,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateEnumAbove {
        #[serde(flatten)]
        pub knot: LateKnotAbove,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateNamedAbove {
        #[serde(flatten)]
        pub hitch: LateHitchAbove,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateOptionAbove {
        pub note: String,
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub rope: Option<LateRopeAbove>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateOptionEnumAbove {
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub hitch: Option<LateHitchAbove>,
        pub note: String,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateLooseAbove {
        pub note: String,
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub rope: Option<LateRopeAbove>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateLooseEnumAbove {
        #[serde(flatten)]
        pub hitch: LateHitchAbove,
        pub note: String,
    }
}

mod below {
    use super::*;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateStructsBelow {
        #[serde(flatten)]
        pub rope: LateLoopBelow,
        #[serde(flatten)]
        pub tail: LateTailBelow,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateEnumBelow {
        #[serde(flatten)]
        pub knot: LateKnotBelow,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateNamedBelow {
        #[serde(flatten)]
        pub hitch: LateHitchBelow,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateOptionBelow {
        pub note: String,
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub rope: Option<LateRopeBelow>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateOptionEnumBelow {
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub hitch: Option<LateHitchBelow>,
        pub note: String,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateLooseBelow {
        pub note: String,
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub rope: Option<LateRopeBelow>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateLooseEnumBelow {
        #[serde(flatten)]
        pub hitch: LateHitchBelow,
        pub note: String,
    }

    /// A struct that refuses no key, flattened beside a struct.
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateLoopBelow {
        pub strand: String,
        pub turns: u32,
    }

    /// A struct that refuses no key, flattened after a struct.
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateTailBelow {
        pub tail: String,
    }

    /// A struct that refuses a key it does not declare.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateRopeBelow {
        pub strand: String,
        pub turns: u32,
    }

    /// An externally tagged enum.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum LateKnotBelow {
        Bowline { loops: u32 },
        Sheet { label: String },
    }

    /// An externally tagged enum with a rename and an alias.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum LateHitchBelow {
        Bowline {
            loops: u32,
        },
        #[serde(alias = "hitch")]
        Hitch {
            turns: u32,
        },
        #[serde(rename = "sheet")]
        Sheet {
            label: String,
        },
    }
}

mod apart {
    use super::*;
    use flattened_members_apart::{
        LateHitchApart, LateKnotApart, LateLoopApart, LateRopeApart, LateTailApart,
    };

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateStructsApart {
        #[serde(flatten)]
        pub rope: LateLoopApart,
        #[serde(flatten)]
        pub tail: LateTailApart,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateEnumApart {
        #[serde(flatten)]
        pub knot: LateKnotApart,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateNamedApart {
        #[serde(flatten)]
        pub hitch: LateHitchApart,
        pub width: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateOptionApart {
        pub note: String,
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub rope: Option<LateRopeApart>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateOptionEnumApart {
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub hitch: Option<LateHitchApart>,
        pub note: String,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateLooseApart {
        pub note: String,
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub rope: Option<LateRopeApart>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LateLooseEnumApart {
        #[serde(flatten)]
        pub hitch: LateHitchApart,
        pub note: String,
    }
}

use flattened_members_apart::{
    late_hitch_apart_dart, late_hitch_apart_swift, late_knot_apart_dart, late_knot_apart_swift,
    late_loop_apart_dart, late_loop_apart_swift, late_rope_apart_dart, late_rope_apart_swift,
    late_tail_apart_dart, late_tail_apart_swift,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The layouts a holder is written in.
pub const LAYOUTS: [&str; 3] = ["Above", "Below", "Apart"];

/// The emitted Dart of every holder and type of `layout`, holders first where the types are below them.
pub fn dart_definitions(layout: &str) -> Vec<String> {
    match layout {
        "Above" => vec![
            late_loop_above_dart::dart_definition(),
            late_tail_above_dart::dart_definition(),
            late_rope_above_dart::dart_definition(),
            late_knot_above_dart::dart_definition(),
            late_hitch_above_dart::dart_definition(),
            late_structs_above_dart::dart_definition(),
            late_enum_above_dart::dart_definition(),
            late_named_above_dart::dart_definition(),
            late_option_above_dart::dart_definition(),
            late_option_enum_above_dart::dart_definition(),
            late_loose_above_dart::dart_definition(),
            late_loose_enum_above_dart::dart_definition(),
        ],
        "Below" => vec![
            late_structs_below_dart::dart_definition(),
            late_enum_below_dart::dart_definition(),
            late_named_below_dart::dart_definition(),
            late_option_below_dart::dart_definition(),
            late_option_enum_below_dart::dart_definition(),
            late_loose_below_dart::dart_definition(),
            late_loose_enum_below_dart::dart_definition(),
            late_loop_below_dart::dart_definition(),
            late_tail_below_dart::dart_definition(),
            late_rope_below_dart::dart_definition(),
            late_knot_below_dart::dart_definition(),
            late_hitch_below_dart::dart_definition(),
        ],
        "Apart" => vec![
            late_structs_apart_dart::dart_definition(),
            late_enum_apart_dart::dart_definition(),
            late_named_apart_dart::dart_definition(),
            late_option_apart_dart::dart_definition(),
            late_option_enum_apart_dart::dart_definition(),
            late_loose_apart_dart::dart_definition(),
            late_loose_enum_apart_dart::dart_definition(),
            late_loop_apart_dart::dart_definition(),
            late_tail_apart_dart::dart_definition(),
            late_rope_apart_dart::dart_definition(),
            late_knot_apart_dart::dart_definition(),
            late_hitch_apart_dart::dart_definition(),
        ],
        _ => Vec::new(),
    }
}

/// The emitted Swift of every holder and type of `layout`, holders first where the types are below them.
pub fn swift_definitions(layout: &str) -> Vec<String> {
    match layout {
        "Above" => vec![
            late_loop_above_swift::swift_definition(),
            late_tail_above_swift::swift_definition(),
            late_rope_above_swift::swift_definition(),
            late_knot_above_swift::swift_definition(),
            late_hitch_above_swift::swift_definition(),
            late_structs_above_swift::swift_definition(),
            late_enum_above_swift::swift_definition(),
            late_named_above_swift::swift_definition(),
            late_option_above_swift::swift_definition(),
            late_option_enum_above_swift::swift_definition(),
            late_loose_above_swift::swift_definition(),
            late_loose_enum_above_swift::swift_definition(),
        ],
        "Below" => vec![
            late_structs_below_swift::swift_definition(),
            late_enum_below_swift::swift_definition(),
            late_named_below_swift::swift_definition(),
            late_option_below_swift::swift_definition(),
            late_option_enum_below_swift::swift_definition(),
            late_loose_below_swift::swift_definition(),
            late_loose_enum_below_swift::swift_definition(),
            late_loop_below_swift::swift_definition(),
            late_tail_below_swift::swift_definition(),
            late_rope_below_swift::swift_definition(),
            late_knot_below_swift::swift_definition(),
            late_hitch_below_swift::swift_definition(),
        ],
        "Apart" => vec![
            late_structs_apart_swift::swift_definition(),
            late_enum_apart_swift::swift_definition(),
            late_named_apart_swift::swift_definition(),
            late_option_apart_swift::swift_definition(),
            late_option_enum_apart_swift::swift_definition(),
            late_loose_apart_swift::swift_definition(),
            late_loose_enum_apart_swift::swift_definition(),
            late_loop_apart_swift::swift_definition(),
            late_tail_apart_swift::swift_definition(),
            late_rope_apart_swift::swift_definition(),
            late_knot_apart_swift::swift_definition(),
            late_hitch_apart_swift::swift_definition(),
        ],
        _ => Vec::new(),
    }
}

/// What serde reads `payload` as for the holder `holder` of `layout`, written back, or `None` where it refuses it.
pub fn serde_written(holder: &str, layout: &str, payload: &str) -> Option<serde_json::Value> {
    fn back<Held>(payload: &str) -> Option<serde_json::Value>
    where
        Held: DeserializeOwned + Serialize,
    {
        serde_json::to_value(serde_json::from_str::<Held>(payload).ok()?).ok()
    }
    match (holder, layout) {
        ("LateStructs", "Above") => back::<above::LateStructsAbove>(payload),
        ("LateEnum", "Above") => back::<above::LateEnumAbove>(payload),
        ("LateNamed", "Above") => back::<above::LateNamedAbove>(payload),
        ("LateOption", "Above") => back::<above::LateOptionAbove>(payload),
        ("LateOptionEnum", "Above") => back::<above::LateOptionEnumAbove>(payload),
        ("LateLoose", "Above") => back::<above::LateLooseAbove>(payload),
        ("LateLooseEnum", "Above") => back::<above::LateLooseEnumAbove>(payload),
        ("LateStructs", "Below") => back::<below::LateStructsBelow>(payload),
        ("LateEnum", "Below") => back::<below::LateEnumBelow>(payload),
        ("LateNamed", "Below") => back::<below::LateNamedBelow>(payload),
        ("LateOption", "Below") => back::<below::LateOptionBelow>(payload),
        ("LateOptionEnum", "Below") => back::<below::LateOptionEnumBelow>(payload),
        ("LateLoose", "Below") => back::<below::LateLooseBelow>(payload),
        ("LateLooseEnum", "Below") => back::<below::LateLooseEnumBelow>(payload),
        ("LateStructs", "Apart") => back::<apart::LateStructsApart>(payload),
        ("LateEnum", "Apart") => back::<apart::LateEnumApart>(payload),
        ("LateNamed", "Apart") => back::<apart::LateNamedApart>(payload),
        ("LateOption", "Apart") => back::<apart::LateOptionApart>(payload),
        ("LateOptionEnum", "Apart") => back::<apart::LateOptionEnumApart>(payload),
        ("LateLoose", "Apart") => back::<apart::LateLooseApart>(payload),
        ("LateLooseEnum", "Apart") => back::<apart::LateLooseEnumApart>(payload),
        _ => None,
    }
}
