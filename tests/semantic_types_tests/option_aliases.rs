//! An alias of an `Option` held outside a field is written in place, in the nullable form of the
//! `Option` it names, on TypeScript and Zod, and the JSON Schema admits the `null` serde writes
//! there. A field keeps the alias's name, but under `nullable`, which writes it in place.

#[cfg(feature = "jsonschema")]
use serde_json::Value;

#[cfg(feature = "jsonschema")]
use super::documents::admits;
#[cfg(feature = "typescript")]
use super::option_alias_models::haply_schema;
#[cfg(any(feature = "typescript", feature = "zod"))]
use super::option_alias_models::plummet_schema;
#[cfg(feature = "jsonschema")]
use super::option_alias_models::{Bottomed, Groundage, Grounding};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::option_alias_models::{
    Chanced, Deepsea, Handlead, Heaved, Leadsman, Lineman, Plumbline, Reckoned, Shoaled, Sinker,
};
use super::option_alias_models::{LEADS, serde_wrote};
#[cfg(feature = "kotlin")]
use super::option_alias_models::{
    chanced_kotlin, handlead_kotlin, haply_kotlin, lineman_kotlin, plummet_kotlin, shoaled_kotlin,
};

/// The Zod schema of a `u32`, the value an alias of an `Option<u32>` holds.
#[cfg(feature = "zod")]
const COUNT: &str = "z.number().int().min(0).max(4294967295)";

/// serde writes `None` as `null` wherever the alias is held outside a field, and reads back each
/// payload it writes.
#[test]
fn serde_writes_none_as_null_wherever_an_alias_of_an_option_is_held_outside_a_field() {
    for (read_as, json) in LEADS {
        assert_eq!(
            serde_wrote(read_as, json).as_deref(),
            Some(json),
            "serde does not write back {json} read as {read_as}"
        );
    }
}

/// TypeScript writes the alias in place, with `null`, in every position outside a field, and an
/// `Option` written as a type argument the same way. A field keeps the alias's name.
#[cfg(feature = "typescript")]
#[test]
fn typescript_writes_an_alias_of_an_option_outside_a_field_with_null() {
    let leadsman = Leadsman::ts_definition();
    for written in [
        "keyed: Partial<Record<string, number | null>>;",
        "listed: Array<number | null>;",
        "nested: Array<Array<number | null>>;",
        "paired: [number | null, number];",
        "relisted: Array<number | null>;",
        "resulted: { Ok: number | null } | { Err: string };",
        "swung: Swung<number | null>;",
        "swung_option: Swung<number | null>;",
    ] {
        assert!(leadsman.contains(written), "no `{written}` in: {leadsman}");
    }
    for (definition, written) in [
        (
            Reckoned::ts_definition(),
            "export type Reckoned = [number | null, number];",
        ),
        (Chanced::ts_definition(), "listed: Array<string | null>;"),
        (Chanced::ts_definition(), "kept?: HaplyType<string>;"),
        (Handlead::ts_definition(), "kept?: PlummetType;"),
        (Shoaled::ts_definition(), "kept?: PlummetType;"),
        (Shoaled::ts_definition(), "marked: Array<number | null>;"),
        (Plumbline::ts_definition(), "kept?: PlummetType;"),
        (Lineman::ts_definition(), "kept: number | null;"),
        (Handlead::ts_definition(), "listed: Array<number | null>;"),
        (
            Deepsea::ts_definition(),
            "export type Deepsea = number | null;",
        ),
        (Sinker::ts_definition(), "\"Depth\": number | null;"),
        (
            Sinker::ts_definition(),
            "\"Pair\": [number | null, number];",
        ),
        (Heaved::ts_definition(), "body: number | null;"),
        (
            plummet_schema::Schema::ts_definition(),
            "export type PlummetType = number | undefined;",
        ),
        (
            haply_schema::Schema::ts_definition(),
            "export type HaplyType<T> = T | undefined;",
        ),
    ] {
        assert!(
            definition.contains(written),
            "no `{written}` in: {definition}"
        );
    }
}

/// Zod writes the alias in place, as the nullable schema of the type it names, in every position
/// outside a field, and an `Option` written as a type argument the same way. A field keeps the
/// alias's binding.
#[cfg(feature = "zod")]
#[test]
fn zod_writes_an_alias_of_an_option_outside_a_field_as_a_nullable_schema() {
    let held = format!("z.nullable({COUNT})");
    let leadsman = Leadsman::zod_schema();
    for written in [
        format!("keyed: z.record(z.string(), {held}),"),
        format!("listed: z.array({held}),"),
        format!("nested: z.array(z.array({held})),"),
        format!("paired: z.tuple([{held}, {COUNT}]),"),
        format!("relisted: z.array({held}),"),
        format!(
            "resulted: z.union([z.strictObject({{ Ok: {held} }}), z.strictObject({{ Err: \
             z.string() }})]),"
        ),
        format!("swung: Swung$SchemaFactory({held}),"),
        format!("swung_option: Swung$SchemaFactory({held}),"),
    ] {
        assert!(leadsman.contains(&written), "no `{written}` in: {leadsman}");
    }
    for (schema, written) in [
        (
            Chanced::zod_schema(),
            "listed: z.array(z.nullable(z.string())),".to_owned(),
        ),
        (
            Chanced::zod_schema(),
            "kept: HaplyType$SchemaFactory(z.string()).optional(),".to_owned(),
        ),
        (
            Handlead::zod_schema(),
            "kept: PlummetType$Schema.optional(),".to_owned(),
        ),
        (
            Shoaled::zod_schema(),
            "kept: PlummetType$Schema.optional(),".to_owned(),
        ),
        (Shoaled::zod_schema(), format!("marked: z.array({held}),")),
        (
            Plumbline::zod_schema(),
            "kept: PlummetType$Schema.optional(),".to_owned(),
        ),
        (
            Lineman::zod_schema(),
            format!("kept: z.union([{COUNT}, z.null()]),"),
        ),
        (Handlead::zod_schema(), format!("listed: z.array({held}),")),
        (
            Reckoned::zod_schema(),
            format!("const Reckoned$RawSchema = z.tuple([{held}, {COUNT}]);"),
        ),
        (
            Deepsea::zod_schema(),
            format!("const Deepsea$RawSchema = {held};"),
        ),
        (Sinker::zod_schema(), format!("\"Depth\": {held},")),
        (
            Sinker::zod_schema(),
            format!("\"Pair\": z.tuple([{held}, {COUNT}]),"),
        ),
        (Heaved::zod_schema(), format!("body: {held},")),
        (
            plummet_schema::Schema::zod_schema(),
            format!(
                "const PlummetType$RawSchema = z.union([z.null().transform(() => undefined), \
                 {COUNT}, z.undefined()]).prefault(undefined);"
            ),
        ),
    ] {
        assert!(schema.contains(&written), "no `{written}` in: {schema}");
    }
}

/// The JSON Schema of each type admits each payload serde writes, the `null` it writes for the
/// alias among them.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_admits_the_null_serde_writes_for_an_alias_of_an_option() {
    for (read_as, json) in LEADS {
        let document = match read_as {
            "Leadsman" => Leadsman::json_schema(),
            "Handlead" => Handlead::json_schema(),
            "Chanced" => Chanced::json_schema(),
            "Shoaled" => Shoaled::json_schema(),
            "Lineman" => Lineman::json_schema(),
            "Plumbline" => Plumbline::json_schema(),
            "Reckoned" => Reckoned::json_schema(),
            "Deepsea" => Deepsea::json_schema(),
            "Sinker" => Sinker::json_schema(),
            "Bottomed" => Bottomed::json_schema(),
            "Groundage" => Groundage::json_schema(),
            "Grounding" => Grounding::json_schema(),
            _ => Heaved::json_schema(),
        };
        let payload: Value = serde_json::from_str(json).unwrap();
        assert!(
            admits(&document, &payload),
            "the JSON Schema of {read_as} refuses {json}, which serde writes: {document}"
        );
    }
}

/// The JSON Schema requires the key of a field of an alias of an `Option` under `nullable`, which
/// serde always writes, and leaves out of `required` the key it leaves out for `None`.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_requires_the_key_serde_always_writes_for_an_alias_of_an_option() {
    for (document, required) in [
        (Lineman::json_schema(), serde_json::json!(["kept"])),
        (Shoaled::json_schema(), serde_json::json!(["marked"])),
    ] {
        assert_eq!(document["required"], required, "in: {document}");
    }
}

/// Kotlin writes no second `?` over an alias of an `Option` that declares no type parameter, and
/// writes one that declares a parameter in place: kotlinc refuses a `typealias` of a nullable
/// type parameter, and reports a second `?` as redundant.
#[cfg(feature = "kotlin")]
#[test]
fn kotlin_writes_one_nullable_mark_over_an_alias_of_an_option() {
    assert_eq!(
        haply_kotlin::kotlin_definition(),
        "",
        "no typealias of a nullable parameter"
    );
    for (definition, written) in [
        (
            plummet_kotlin::kotlin_definition(),
            "typealias PlummetType = UInt?;",
        ),
        (
            handlead_kotlin::kotlin_definition(),
            "data class Handlead(val kept: PlummetType = null, val listed: List<PlummetType>)",
        ),
        (
            shoaled_kotlin::kotlin_definition(),
            "data class Shoaled(val kept: PlummetType = null, val marked: List<PlummetType>)",
        ),
        (
            lineman_kotlin::kotlin_definition(),
            "data class Lineman(val kept: PlummetType)",
        ),
        (
            chanced_kotlin::kotlin_definition(),
            "data class Chanced(val kept: String? = null, val listed: List<String?>)",
        ),
    ] {
        assert!(
            definition.contains(written),
            "no `{written}` in: {definition}"
        );
    }
}

/// The README's example of an alias of an `Option` states what the holder writes for it.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn the_readme_states_what_a_holder_writes_for_an_alias_of_an_option() {
    let readme = include_str!("../../README.md");
    let handlead = Handlead::ts_definition();
    let alias = plummet_schema::Schema::ts_definition();
    let listed = format!("listed: z.array(z.nullable({COUNT}))");
    for (stated, written) in [
        (
            "pub struct Handlead {",
            include_str!("option_alias_models.rs").to_owned(),
        ),
        ("export type Handlead = {", handlead.clone()),
        ("  kept?: PlummetType;", handlead.clone()),
        ("  listed: Array<number | null>;", handlead),
        ("export type PlummetType = number | undefined;", alias),
        (listed.as_str(), Handlead::zod_schema()),
    ] {
        assert!(
            readme.contains(stated),
            "the README does not state `{stated}`"
        );
        assert!(
            written.contains(stated.trim_start()),
            "the README states `{stated}`, and the holder writes: {written}"
        );
    }
}
