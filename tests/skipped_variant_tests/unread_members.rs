//! A variant serde never reads, under `skip` or `skip_deserializing`, holding a type serde has no
//! read of, under each tagging: each enum builds, as it does under serde's derive alone, and
//! serde reads and writes it as its derive does.

use std::sync::OnceLock;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A model serde writes and never reads.
#[model_schema()]
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Chandlery {
    pub note: String,
}

/// No tag attribute: a variant serde neither writes nor reads holds the model.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum Stevedorage {
    #[serde(skip)]
    Closed {
        cache: Chandlery,
    },
    Open {
        note: String,
    },
}

/// No tag attribute: a variant serde writes and never reads holds the model.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum Wharfinger {
    #[serde(skip_deserializing)]
    Closed {
        cache: Chandlery,
    },
    Open {
        note: String,
    },
}

/// Under a tag alone: a variant serde writes and never reads holds the model.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Ballastage {
    #[serde(skip_deserializing)]
    Closed {
        cache: Chandlery,
    },
    Open {
        note: String,
    },
}

/// Under a tag and a content key: a newtype variant serde writes and never reads holds the model.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum Tallyman {
    #[serde(skip_deserializing)]
    Closed(Chandlery),
    Open {
        note: String,
    },
}

/// Untagged: a variant serde writes and never reads holds the model.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Lockage {
    #[serde(skip_deserializing)]
    Closed {
        cache: Chandlery,
    },
    Open {
        note: String,
    },
}

/// A variant serde neither writes nor reads holds a cell serde has no read or write of.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Lighterman {
    #[serde(skip)]
    Closed {
        cache: OnceLock<String>,
    },
    Open {
        note: String,
    },
}

/// What serde answers for a read of `json` as `T`: the value, or its refusal.
fn read<T>(json: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    serde_json::from_str(json).map_err(|refused| refused.to_string())
}

/// Each enum reads the variant serde reads and refuses the one it never reads, in serde's words.
#[test]
fn a_variant_serde_never_reads_is_refused_as_serdes_derive_refuses_it() {
    let open = || "n".to_owned();
    assert_eq!(
        read::<Stevedorage>(r#"{"Open":{"note":"n"}}"#),
        Ok(Stevedorage::Open { note: open() }),
        "Stevedorage"
    );
    assert_eq!(
        read::<Stevedorage>(r#"{"Closed":{"cache":{"note":"n"}}}"#),
        Err("unknown variant `Closed`, expected `Open` at line 1 column 9".to_owned()),
        "Stevedorage"
    );
    assert_eq!(
        read::<Wharfinger>(r#"{"Open":{"note":"n"}}"#),
        Ok(Wharfinger::Open { note: open() }),
        "Wharfinger"
    );
    assert_eq!(
        read::<Wharfinger>(r#"{"Closed":{"cache":{"note":"n"}}}"#),
        Err("unknown variant `Closed`, expected `Open` at line 1 column 9".to_owned()),
        "Wharfinger"
    );
    assert_eq!(
        read::<Ballastage>(r#"{"kind":"Open","note":"n"}"#),
        Ok(Ballastage::Open { note: open() }),
        "Ballastage"
    );
    assert_eq!(
        read::<Ballastage>(r#"{"kind":"Closed","cache":{"note":"n"}}"#),
        Err("unknown variant `Closed`, expected `Open` at line 1 column 16".to_owned()),
        "Ballastage"
    );
    assert_eq!(
        read::<Tallyman>(r#"{"kind":"Open","body":{"note":"n"}}"#),
        Ok(Tallyman::Open { note: open() }),
        "Tallyman"
    );
    assert_eq!(
        read::<Tallyman>(r#"{"kind":"Closed","body":{"note":"n"}}"#),
        Err("unknown variant `Closed`, expected `Open` at line 1 column 16".to_owned()),
        "Tallyman"
    );
    assert_eq!(
        read::<Lockage>(r#"{"note":"n"}"#),
        Ok(Lockage::Open { note: open() }),
        "Lockage"
    );
    assert_eq!(
        read::<Lockage>(r#"{"cache":{"note":"n"}}"#),
        Err("data did not match any variant of untagged enum Lockage".to_owned()),
        "Lockage"
    );
    assert!(
        matches!(
            read::<Lighterman>(r#"{"Open":{"note":"n"}}"#),
            Ok(Lighterman::Open { note }) if note == "n"
        ),
        "Lighterman"
    );
    assert_eq!(
        read::<Lighterman>(r#"{"Closed":{"cache":"c"}}"#).err(),
        Some("unknown variant `Closed`, expected `Open` at line 1 column 9".to_owned()),
        "Lighterman"
    );
}

/// serde writes the variant it never reads, which every surface describes.
#[test]
fn a_variant_serde_writes_and_never_reads_is_written_and_described() {
    let cache = || Chandlery {
        note: "n".to_owned(),
    };
    for (written, wire, ts) in [
        (
            serde_json::to_string(&Wharfinger::Closed { cache: cache() }),
            r#"{"Closed":{"cache":{"note":"n"}}}"#,
            Wharfinger::ts_definition(),
        ),
        (
            serde_json::to_string(&Ballastage::Closed { cache: cache() }),
            r#"{"kind":"Closed","cache":{"note":"n"}}"#,
            Ballastage::ts_definition(),
        ),
        (
            serde_json::to_string(&Tallyman::Closed(cache())),
            r#"{"kind":"Closed","body":{"note":"n"}}"#,
            Tallyman::ts_definition(),
        ),
        (
            serde_json::to_string(&Lockage::Closed { cache: cache() }),
            r#"{"cache":{"note":"n"}}"#,
            Lockage::ts_definition(),
        ),
    ] {
        assert_eq!(written.unwrap(), wire, "serde's write");
        assert!(ts.contains("Chandlery"), "the TypeScript text:\n{ts}");
    }
}

/// serde writes no variant under `skip`, the one holding a cell among them.
#[test]
fn a_variant_serde_neither_writes_nor_reads_is_not_written() {
    let said = |written: Result<String, serde_json::Error>| {
        written.unwrap_or_else(|refused| refused.to_string())
    };
    assert_eq!(
        said(serde_json::to_string(&Stevedorage::Closed {
            cache: Chandlery {
                note: "n".to_owned()
            }
        })),
        "the enum variant Stevedorage::Closed cannot be serialized",
        "Stevedorage"
    );
    assert_eq!(
        said(serde_json::to_string(&Lighterman::Closed {
            cache: OnceLock::new()
        })),
        "the enum variant Lighterman::Closed cannot be serialized",
        "Lighterman"
    );
}
