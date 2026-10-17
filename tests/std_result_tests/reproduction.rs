//! The reproduction: a document whose frontmatter reads as a model, or is held as what does not
//! read, under an `Option`.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

use super::documents::admits;

/// A frontmatter that reads.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Read {
    pub title: String,
}

/// A frontmatter that does not read, held as it was written.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Broken {
    pub raw: String,
}

/// A document of an optional frontmatter.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Doc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<Result<Read, Broken>>,
}

#[test]
fn a_frontmatter_of_either_arm_is_written_under_the_arm_and_read_back() {
    for (doc, written) in [
        (
            Doc {
                frontmatter: Some(Ok(Read {
                    title: "Intro".to_owned(),
                })),
            },
            json!({ "frontmatter": { "Ok": { "title": "Intro" } } }),
        ),
        (
            Doc {
                frontmatter: Some(Err(Broken {
                    raw: "title: [".to_owned(),
                })),
            },
            json!({ "frontmatter": { "Err": { "raw": "title: [" } } }),
        ),
        (Doc { frontmatter: None }, json!({})),
    ] {
        let value = serde_json::to_value(&doc).unwrap();
        assert_eq!(value, written);
        assert!(admits(&Doc::json_schema(), &value), "{value}");
        assert_eq!(serde_json::from_value::<Doc>(value).unwrap(), doc);
    }
}

#[test]
fn a_frontmatter_is_described_as_the_object_serde_writes() {
    let typescript = Doc::ts_definition();
    assert!(
        typescript.contains("frontmatter: { Ok: Read } | { Err: Broken } | undefined;"),
        "{typescript}"
    );
    let zod = Doc::zod_schema();
    assert!(
        zod.contains(
            "z.union([z.strictObject({ Ok: Read$Schema }), z.strictObject({ Err: Broken$Schema })])"
        ),
        "{zod}"
    );
    let both = json!({ "frontmatter": { "Ok": { "title": "a" }, "Err": { "raw": "b" } } });
    assert!(!admits(&Doc::json_schema(), &both));
    serde_json::from_value::<Doc>(both).unwrap_err();
}
