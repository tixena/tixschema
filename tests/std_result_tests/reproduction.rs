//! The reproduction: a document whose frontmatter reads as a model, or is held as what does not
//! read, under an `Option`.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

use super::documents::admits;

/// A frontmatter that reads.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Read {
    pub title: String,
}

/// A frontmatter that does not read, held as it was written.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Broken {
    pub raw: String,
}

/// A document of an optional frontmatter.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Doc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<Result<Read, Broken>>,
}

#[test]
fn a_frontmatter_of_either_arm_is_written_under_the_arm_and_read_back() {
    for (doc, written) in [
        (
            DocDraft {
                frontmatter: Some(Ok(ReadDraft {
                    title: "Intro".to_owned(),
                }
                .into())),
            },
            json!({ "frontmatter": { "Ok": { "title": "Intro" } } }),
        ),
        (
            DocDraft {
                frontmatter: Some(Err(BrokenDraft {
                    raw: "title: [".to_owned(),
                }
                .into())),
            },
            json!({ "frontmatter": { "Err": { "raw": "title: [" } } }),
        ),
        (DocDraft { frontmatter: None }, json!({})),
    ] {
        let value = serde_json::to_value(&doc).unwrap();
        assert_eq!(value, written);
        assert!(admits(&Doc::json_schema(), &value), "{value}");
        assert_eq!(
            serde_json::from_value::<Doc>(value).unwrap(),
            doc.validate().unwrap()
        );
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
            "z.unknown().check((ctx) => { if (ctx.value !== undefined && typeof ctx.value !== \"string\" && (typeof ctx.value !== \"object\" || ctx.value === null || Array.isArray(ctx.value) || Object.keys(ctx.value).length !== 1)) ctx.issues.push({ code: \"custom\", message: \"expected value\", input: ctx.value, params: { undeserializable: true } }); }).pipe(z.union([z.strictObject({ Ok: Read$Schema }), z.strictObject({ Err: Broken$Schema })]).superRefine((_read, ctx) => { const held = ctx.value; const carrying = [[\"Ok\", \"newtype variant\"], [\"Err\", \"newtype variant\"]]; const keys = typeof held === \"object\" && held !== null && !Array.isArray(held) ? Object.keys(held) : []; const at = ctx.issues.findIndex((issue) => issue.code === \"invalid_union\" && (issue.path ?? []).length === 0); const refused = ctx.issues[at]; const named = typeof held === \"string\" ? carrying.find(([name]) => name === held) : undefined; const arm = keys.length === 1 && refused?.code === \"invalid_union\" ? refused.errors.find((issues) => issues.some((issue) => issue.path[0] === keys[0])) : undefined; if (named !== undefined && refused?.code === \"invalid_union\") ctx.issues.splice(at, 1, { code: \"custom\", message: \"invalid type: unit variant, expected \" + named[1], input: undefined, path: [named[0]], params: { variant: true } }); else if (arm !== undefined) ctx.issues.splice(at, 1, ...arm.map((issue) => ({ ...issue, input: undefined }))); }, { when: () => true }))"
        ),
        "{zod}"
    );
    let both = json!({ "frontmatter": { "Ok": { "title": "a" }, "Err": { "raw": "b" } } });
    assert!(!admits(&Doc::json_schema(), &both));
    serde_json::from_value::<Doc>(both).unwrap_err();
}
