//! What TypeScript, Zod and the JSON Schema write for a `Result`: the object whose one key names
//! its arm, each arm rendered as its type renders anywhere else and `()` as `null`.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::documents::{admits, refusal};
use super::models::{
    Attempted, Bureau, Errand, Errata, Masthead, Missive, Notebook, Postbox, Reply, Ruling,
    Summary, Turn, Unparsed, judged_payloads, read_payloads, refused_payloads, settlement_schema,
};

/// `()` outside an arm of a `Result`, beside one inside an arm.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bare {
    pub armed: Result<(), u32>,
    pub unit: (),
}

/// Each model a payload is read as: its name, its JSON Schema, and whether serde reads a JSON as
/// it.
type Described = (&'static str, fn() -> Value, fn(&str) -> bool);

/// The JSON Schema of a `Result` whose `Ok` and `Err` hold `ok` and `err`.
fn result_document(ok: &Value, err: &Value) -> Value {
    json!({
        "oneOf": [
            {
                "type": "object",
                "properties": { "Ok": ok },
                "required": ["Ok"],
                "additionalProperties": false,
            },
            {
                "type": "object",
                "properties": { "Err": err },
                "required": ["Err"],
                "additionalProperties": false,
            },
        ]
    })
}

/// The JSON Schema of a `Result<u32, String>`.
fn number_or_text() -> Value {
    result_document(&json!({ "type": "integer" }), &json!({ "type": "string" }))
}

/// Whether serde reads `json` as a `T`.
fn reads<T>(json: &str) -> bool
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).is_ok()
}

/// The models the payloads are read as.
fn described() -> Vec<Described> {
    vec![
        (
            "Attempted",
            Attempted::<u32>::json_schema,
            reads::<Attempted<u32>>,
        ),
        ("Bureau", Bureau::json_schema, reads::<Bureau>),
        ("Errand", Errand::json_schema, reads::<Errand>),
        ("Errata", Errata::json_schema, reads::<Errata>),
        ("Missive", Missive::json_schema, reads::<Missive>),
        ("Notebook", Notebook::json_schema, reads::<Notebook>),
        ("Postbox", Postbox::json_schema, reads::<Postbox>),
        ("Reply", Reply::json_schema, reads::<Reply>),
        ("Summary", Summary::json_schema, reads::<Summary>),
        ("Turn", Turn::<u32>::json_schema, reads::<Turn<u32>>),
    ]
}

/// The model `read_as` names.
fn model(read_as: &str) -> Described {
    described()
        .into_iter()
        .find(|(named, _, _)| *named == read_as)
        .unwrap()
}

#[test]
fn a_result_is_described_as_the_object_serde_writes() {
    let typescript = Reply::ts_definition();
    for member in [
        "outcome: { Ok: number } | { Err: string };",
        "stored: { Ok: null } | { Err: string };",
    ] {
        assert!(typescript.contains(member), "{member} in: {typescript}");
    }
    let zod = Reply::zod_schema();
    for member in [
        "outcome: z.union([z.strictObject({ Ok: z.number().int() }), z.strictObject({ Err: z.string() })]),",
        "stored: z.union([z.strictObject({ Ok: z.null() }), z.strictObject({ Err: z.string() })]),",
    ] {
        assert!(zod.contains(member), "{member} in: {zod}");
    }
    let document = Reply::json_schema();
    assert_eq!(document["properties"]["outcome"], number_or_text());
    assert_eq!(
        document["properties"]["stored"],
        result_document(&json!({ "type": "null" }), &json!({ "type": "string" }))
    );
    assert_eq!(
        serde_json::to_value(super::models::reply()).unwrap(),
        json!({ "outcome": { "Ok": 7_u32 }, "stored": { "Ok": null } })
    );
}

#[test]
fn an_arm_of_unit_is_null_on_every_surface() {
    let typescript = Summary::ts_definition();
    for member in [
        "both: { Ok: null } | { Err: null };",
        "err: { Ok: number } | { Err: null };",
        "ok: { Ok: null } | { Err: string };",
    ] {
        assert!(typescript.contains(member), "{member} in: {typescript}");
    }
    let zod = Summary::zod_schema();
    for member in [
        "both: z.union([z.strictObject({ Ok: z.null() }), z.strictObject({ Err: z.null() })]),",
        "err: z.union([z.strictObject({ Ok: z.number().int() }), z.strictObject({ Err: z.null() })]),",
    ] {
        assert!(zod.contains(member), "{member} in: {zod}");
    }
    let null = json!({ "type": "null" });
    assert_eq!(
        Summary::json_schema()["properties"]["both"],
        result_document(&null, &null)
    );
}

#[test]
fn unit_outside_an_arm_is_described_as_the_null_an_arm_is() {
    let typescript = Bare::ts_definition();
    assert!(
        typescript.contains("armed: { Ok: null } | { Err: number };"),
        "{typescript}"
    );
    assert!(typescript.contains("unit: null;"), "{typescript}");
    let zod = Bare::zod_schema();
    assert!(zod.contains("unit: z.null(),"), "{zod}");
    assert!(
        zod.contains("armed: z.union([z.strictObject({ Ok: z.null() }), z.strictObject({ Err: z.number().int() })]),"),
        "{zod}"
    );
    let document = Bare::json_schema();
    assert_eq!(
        document["properties"]["armed"]["oneOf"][0]["properties"]["Ok"],
        json!({ "type": "null" })
    );
    assert_eq!(document["properties"]["unit"], json!({ "type": "null" }));
}

#[test]
fn a_result_is_the_object_in_every_position_that_holds_a_value() {
    let object = "{ Ok: number } | { Err: string }";
    let typescript = Postbox::ts_definition();
    for member in [
        format!("filled: Mailbag<{object}>;"),
        format!("keyed: Partial<Record<string, {object}>>;"),
        format!("listed: Array<{object}>;"),
        format!("maybe: {object} | undefined;"),
        format!("nested: {{ Ok: {object} }} | {{ Err: string }};"),
        format!("paired: [{object}, string];"),
    ] {
        assert!(typescript.contains(&member), "{member} in: {typescript}");
    }
    let schema =
        "z.union([z.strictObject({ Ok: z.number().int() }), z.strictObject({ Err: z.string() })])";
    let zod = Postbox::zod_schema();
    for member in [
        format!("filled: Mailbag$SchemaFactory({schema}),"),
        format!("keyed: z.record(z.string(), {schema}),"),
        format!("listed: z.array({schema}),"),
        format!(
            "maybe: z.union([z.null().transform(() => undefined), {schema}, z.undefined()]).prefault(undefined),"
        ),
        format!(
            "nested: z.union([z.strictObject({{ Ok: {schema} }}), z.strictObject({{ Err: z.string() }})]),"
        ),
        format!("paired: z.tuple([{schema}, z.string()]),"),
    ] {
        assert!(zod.contains(&member), "{member} in: {zod}");
    }
    let result = number_or_text();
    let document = Postbox::json_schema();
    assert_eq!(
        document["properties"]["filled"]["properties"]["held"],
        result
    );
    assert_eq!(
        document["properties"]["keyed"]["additionalProperties"],
        result
    );
    assert_eq!(document["properties"]["listed"]["items"], result);
    assert_eq!(
        document["properties"]["maybe"],
        json!({ "anyOf": [result, { "type": "null" }] })
    );
    assert_eq!(
        document["properties"]["nested"],
        result_document(&number_or_text(), &json!({ "type": "string" }))
    );
    assert_eq!(document["properties"]["paired"]["prefixItems"][0], result);
}

#[test]
fn a_result_is_the_object_as_a_brand_an_alias_and_a_variant_slot() {
    let object = "{ Ok: number } | { Err: string }";
    let brand = Ruling::ts_definition();
    assert!(
        brand.contains(&format!(
            "export type Ruling = ({object}) & $brand<\"Ruling\">;"
        )),
        "{brand}"
    );
    assert!(
        Ruling::zod_schema().contains(
            "z.union([z.strictObject({ Ok: z.number().int() }), z.strictObject({ Err: z.string() })]).brand<\"Ruling\">()"
        ),
        "{}",
        Ruling::zod_schema()
    );
    assert_eq!(Ruling::json_schema(), number_or_text());
    let alias = settlement_schema::Schema::ts_definition();
    assert!(
        alias.contains(&format!("export type SettlementType = {object};")),
        "{alias}"
    );
    assert_eq!(settlement_schema::Schema::json_schema(), number_or_text());
    let turn = Turn::<u32>::ts_definition();
    for member in [
        "\"Slot\": { Ok: Held } | { Err: string };",
        "outcome: { Ok: Held } | { Err: string };",
    ] {
        assert!(turn.contains(member), "{member} in: {turn}");
    }
    let zod = Turn::<u32>::zod_schema();
    assert!(
        zod.contains("\"Slot\": z.union([z.strictObject({ Ok: held }), z.strictObject({ Err: z.string() })]),"),
        "{zod}"
    );
    assert_eq!(
        Turn::<u32>::json_schema()["oneOf"][1]["properties"]["Slot"],
        number_or_text()
    );
    assert_eq!(
        Attempted::<u32>::json_schema()["properties"]["outcome"],
        number_or_text()
    );
}

#[test]
fn an_arm_renders_as_its_type_renders_anywhere_else() {
    let typescript = Missive::ts_definition();
    assert!(
        typescript.contains("frontmatter: { Ok: Masthead } | { Err: Unparsed } | undefined;"),
        "{typescript}"
    );
    let zod = Missive::zod_schema();
    assert!(
        zod.contains("z.union([z.strictObject({ Ok: Masthead$Schema }), z.strictObject({ Err: Unparsed$Schema })])"),
        "{zod}"
    );
    let document = Missive::json_schema();
    assert_eq!(
        document["properties"]["frontmatter"],
        json!({
            "anyOf": [
                result_document(&Masthead::json_schema(), &Unparsed::json_schema()),
                { "type": "null" },
            ]
        })
    );
    let required = document["required"].as_array().cloned().unwrap_or_default();
    assert!(!required.contains(&json!("frontmatter")), "{document}");
}

#[test]
fn the_json_schema_admits_what_serde_writes_and_refuses_what_serde_refuses() {
    for payload in read_payloads() {
        let (_, document, serde_reads) = model(payload.read_as);
        let value: Value = serde_json::from_str(&payload.json).unwrap();
        assert!(serde_reads(&payload.json), "{}", payload.name);
        assert!(
            admits(&document(), &value),
            "{}: {}",
            payload.name,
            payload.json
        );
    }
    for payload in refused_payloads() {
        let (_, document, serde_reads) = model(payload.read_as);
        let value: Value = serde_json::from_str(&payload.json).unwrap();
        assert!(!serde_reads(&payload.json), "{}", payload.name);
        assert!(
            refusal(&document(), &value).is_some(),
            "{}: {}",
            payload.name,
            payload.json
        );
    }
    // A bound the model in an arm holds itself to is the document of that model's own.
    for payload in judged_payloads() {
        let (_, document, serde_reads) = model(payload.read_as);
        let value: Value = serde_json::from_str(&payload.json).unwrap();
        assert_eq!(
            admits(&document(), &value),
            serde_reads(&payload.json),
            "{}: {}",
            payload.name,
            payload.json
        );
    }
}
