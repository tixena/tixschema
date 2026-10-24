//! Untagged enums whose variants are told apart by a `literal` on one shared key, the enums that
//! look like them and are not, and [`ROWS`]: the payloads, the model each is read as, whether serde
//! reads it, and the Zod text each enum is written as.

#![cfg(feature = "serde")]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Every payload Zod is held to serde on. Each is read by serde as the column says, and by the
/// Zod schema of the same model alike.
pub const ROWS: [Row; 33] = [
    row(
        "Element",
        r#"{"label":"X","dataType":"Currency","isCalculated":true,"path":"p","isAdvanced":false,"decimals":2}"#,
        true,
    ),
    row(
        "Element",
        r#"{"label":"X","dataType":"Currency","decimals":2}"#,
        true,
    ),
    row(
        "Element",
        r#"{"label":"X","dataType":"Currency","isCalculated":false,"decimals":2}"#,
        true,
    ),
    row(
        "Element",
        r#"{"label":"X","dataType":"Currency","isCalculated":null,"decimals":2}"#,
        true,
    ),
    row(
        "Element",
        r#"{"label":"X","dataType":"Currency","isCalculated":"yes","decimals":2}"#,
        false,
    ),
    row(
        "Element",
        r#"{"label":"X","dataType":"Currency","isCalculated":true,"decimals":2}"#,
        false,
    ),
    row(
        "Calculation",
        r#"{"isCalculated":true,"path":"p","isAdvanced":true}"#,
        true,
    ),
    row("Calculation", "{}", true),
    row("Calculation", r#"{"isCalculated":false}"#, true),
    row("Calculation", r#"{"isCalculated":"yes"}"#, false),
    row(
        "CalculationHolder",
        r#"{"calculation":{"isCalculated":true,"path":"p","isAdvanced":false}}"#,
        true,
    ),
    row("CalculationHolder", r#"{"calculation":{}}"#, true),
    row(
        "CalculationHolder",
        r#"{"calculation":{"isCalculated":false}}"#,
        true,
    ),
    row(
        "CalculationHolder",
        r#"{"calculation":{"isCalculated":true}}"#,
        false,
    ),
    row(
        "OmitsLast",
        r#"{"isCalculated":true,"path":"p","isAdvanced":false}"#,
        true,
    ),
    row("OmitsLast", r#"{"isCalculated":false}"#, true),
    row("OmitsLast", r#"{"label":"x"}"#, true),
    row("OmitsLast", r#"{"isCalculated":"yes"}"#, false),
    row("OmitsLast", r#"{"isCalculated":true}"#, false),
    row("OmitsFirst", r#"{"label":"x"}"#, true),
    row("OmitsFirst", r#"{"isCalculated":false}"#, true),
    row(
        "OmitsFirst",
        r#"{"isCalculated":true,"path":"p","isAdvanced":false}"#,
        true,
    ),
    row("OmitsBesideOption", r#"{"label":"x"}"#, true),
    row("OmitsBesideOption", r#"{"isCalculated":false}"#, true),
    row("OmitsTwice", r#"{"note":"n"}"#, true),
    row("OmitsTwice", r#"{"label":"x"}"#, true),
    row("OmitsAsModel", r#"{"label":"x"}"#, true),
    row("OmitsAsModel", r#"{"isCalculated":false}"#, true),
    row(
        "OmitsAsModel",
        r#"{"isCalculated":true,"path":"p","isAdvanced":false}"#,
        true,
    ),
    row("OmitsAsModel", r#"{"isCalculated":true}"#, false),
    row(
        "AliasedPanel",
        r#"{"label":"X","isCalculated":true,"path":"p","isAdvanced":false}"#,
        true,
    ),
    row("AliasedPanel", r#"{"label":"X"}"#, true),
    row(
        "AliasedPanel",
        r#"{"label":"X","isCalculated":"yes"}"#,
        false,
    ),
];

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Formula {
    pub is_advanced: bool,
    #[model_schema_prop(literal = true)]
    pub is_calculated: bool,
    pub path: String,
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Calculation {
    Calculated(Formula),
    NotCalculated {
        #[serde(rename = "isCalculated", skip_serializing_if = "Option::is_none")]
        #[model_schema_prop(literal = false, ts_optional)]
        is_calculated: Option<bool>,
    },
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    pub decimals: u8,
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "dataType")]
pub enum Kind {
    Currency {
        #[serde(flatten)]
        amount: Amount,
        #[serde(flatten)]
        calculated: Calculation,
    },
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
pub struct Element {
    #[serde(flatten)]
    pub header: Header,
    #[serde(flatten)]
    pub kind: Kind,
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
pub struct CalculationHolder {
    pub calculation: Calculation,
}

/// Two variants hold the literal in an `Option`: an absent key matches both.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BothOptional {
    Off {
        #[serde(rename = "isCalculated", skip_serializing_if = "Option::is_none")]
        #[model_schema_prop(literal = false, ts_optional)]
        is_calculated: Option<bool>,
        label: String,
    },
    On {
        #[serde(rename = "isCalculated", skip_serializing_if = "Option::is_none")]
        #[model_schema_prop(literal = true, ts_optional)]
        is_calculated: Option<bool>,
        path: String,
    },
}

/// A variant holds the shared key under no literal.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnmarkedKey {
    Calculated(Formula),
    Unmarked {
        #[serde(rename = "isCalculated")]
        is_calculated: bool,
    },
}

/// The model the newtype variant holds is declared below the enum.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelBelow {
    Calculated(FormulaBelow),
    Off {
        #[serde(rename = "isCalculated")]
        #[model_schema_prop(literal = false)]
        is_calculated: bool,
    },
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaBelow {
    #[model_schema_prop(literal = true)]
    pub is_calculated: bool,
    pub path: String,
}

/// Two variants hold the same literal.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SameLiteral {
    First {
        #[serde(rename = "isCalculated")]
        #[model_schema_prop(literal = true)]
        is_calculated: bool,
        path: String,
    },
    Second {
        #[serde(rename = "isCalculated")]
        #[model_schema_prop(literal = true)]
        is_calculated: bool,
        label: String,
    },
}

/// One variant omits the key and is declared last, and no variant holds the key in an `Option`.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OmitsLast {
    Calculated(Formula),
    Off {
        #[serde(rename = "isCalculated")]
        #[model_schema_prop(literal = false)]
        is_calculated: bool,
    },
    Plain {
        label: String,
    },
}

/// The variant that omits the key is declared before another variant.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OmitsFirst {
    Absent {
        label: String,
    },
    Calculated(Formula),
    Off {
        #[serde(rename = "isCalculated")]
        #[model_schema_prop(literal = false)]
        is_calculated: bool,
    },
}

/// The variant that omits the key is last, and another variant holds the key in an `Option`.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OmitsBesideOption {
    Calculated(Formula),
    NotCalculated {
        #[serde(rename = "isCalculated", skip_serializing_if = "Option::is_none")]
        #[model_schema_prop(literal = false, ts_optional)]
        is_calculated: Option<bool>,
    },
    Plain {
        label: String,
    },
}

/// A model declared above holds no literal and no key, and its variant is declared last.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
pub struct PlainBody {
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OmitsAsModel {
    Calculated(Formula),
    Off {
        #[serde(rename = "isCalculated")]
        #[model_schema_prop(literal = false)]
        is_calculated: bool,
    },
    Plain(PlainBody),
}

/// Two variants omit the key.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OmitsTwice {
    Calculated(Formula),
    Plain { label: String },
    Quiet { note: String },
}

/// One payload: the model it is read as, and whether serde reads it.
pub struct Row {
    pub json: &'static str,
    pub read_as: &'static str,
    pub serde_reads: bool,
}

#[model_schema()]
pub type CalculationAlias = Calculation;

/// The enum flattened through an alias of it.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
pub struct AliasedPanel {
    #[serde(flatten)]
    pub calculated: CalculationAlias,
    #[serde(flatten)]
    pub header: Header,
}

/// An enum that declares a type parameter.
#[model_schema(default_types(ValueType = String))]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Valued<ValueType> {
    Off {
        #[serde(rename = "isCalculated")]
        #[model_schema_prop(literal = false)]
        is_calculated: bool,
    },
    On {
        #[serde(rename = "isCalculated")]
        #[model_schema_prop(literal = true)]
        is_calculated: bool,
        value: ValueType,
    },
}

const fn row(read_as: &'static str, json: &'static str, serde_reads: bool) -> Row {
    Row {
        json,
        read_as,
        serde_reads,
    }
}

/// Whether serde reads `json` as `read_as`, or the name of a model there is none of.
pub fn serde_reads(read_as: &str, json: &str) -> Result<bool, String> {
    /// The read of `json` as `Read`.
    fn of<Read>(json: &str) -> bool
    where
        Read: DeserializeOwned,
    {
        serde_json::from_str::<Read>(json).is_ok()
    }
    match read_as {
        "Calculation" => Ok(of::<Calculation>(json)),
        "AliasedPanel" => Ok(of::<AliasedPanel>(json)),
        "CalculationHolder" => Ok(of::<CalculationHolder>(json)),
        "Element" => Ok(of::<Element>(json)),
        "OmitsAsModel" => Ok(of::<OmitsAsModel>(json)),
        "OmitsBesideOption" => Ok(of::<OmitsBesideOption>(json)),
        "OmitsFirst" => Ok(of::<OmitsFirst>(json)),
        "OmitsLast" => Ok(of::<OmitsLast>(json)),
        "OmitsTwice" => Ok(of::<OmitsTwice>(json)),
        other => Err(format!("no model {other}")),
    }
}
