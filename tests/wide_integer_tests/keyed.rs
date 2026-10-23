//! What the TypeScript surface writes over a map keyed by a `u64` or an `i64`: the key stays a
//! `number`, and each declaration that writes such a map carries a `@deprecated` note.

#![cfg(feature = "typescript")]

#[cfg(not(any(feature = "swift", feature = "kotlin")))]
use super::keyed_models::WideUnsignedKeyed;
use super::keyed_models::{
    WideKeyed, WideKeyedAdjacent, WideKeyedBrand, WideKeyedEvent, WideKeyedSlot, WideKeyedSlots,
    wide_keyed_alias_schema,
};
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The line of a `JSDoc` block that carries the note, at a member's indent.
const NOTED: &str = "   * @deprecated A map key above 2^53 - 1 (9007199254740991) in size is not \
                     held exactly as a number.";

/// The note as a `JSDoc` block of one line.
const NOTE_BLOCK: &str = "/** @deprecated A map key above 2^53 - 1 (9007199254740991) in size is \
                          not held exactly as a number. */";

// The README's example, declared as the README declares it: a doc comment would be its JSDoc.
#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Histogram {
    pub counts: BTreeMap<i64, u32>,
    pub labels: BTreeMap<u32, String>,
}

/// The member `key` of type `ts`, under the block a member with no doc of its own is written
/// under, closed by the note.
fn noted_member(key: &str, ts: &str) -> String {
    format!("   * {key}\n   * \n{NOTED}\n   */\n  {key}: {ts};")
}

/// Asserts `written` holds each of `expected`.
fn holds_each(written: &str, expected: &[String]) {
    for held in expected {
        assert!(written.contains(held), "missing `{held}` in:\n{written}");
    }
}

/// A member carries the note where its type writes such a map at any depth, or names a type above
/// that carries it on itself.
#[test]
fn each_member_that_writes_a_map_keyed_by_an_i64_carries_the_note() {
    let written = WideKeyed::ts_definition();
    holds_each(
        &written,
        &[
            noted_member("aliased", "Partial<Record<number, boolean>>"),
            noted_member("branded", "WideKeyedBrand"),
            noted_member("direct", "Partial<Record<number, boolean>>"),
            noted_member(
                "filled",
                "WideKeyedHolder<Partial<Record<number, boolean>>>",
            ),
            noted_member("listed", "Array<Partial<Record<number, boolean>>>"),
            noted_member("maybe", "Partial<Record<number, boolean>> | undefined"),
            noted_member("named", "WideKeyedAliasType"),
            noted_member(
                "nested",
                "Partial<Record<string, Partial<Record<number, boolean>>>>",
            ),
            noted_member("paired", "[Partial<Record<number, boolean>>, number]"),
            noted_member("slots", "WideKeyedSlots"),
            noted_member("slotted", "Array<WideKeyedSlot>"),
            "   * narrow\n   * \n   */\n  narrow: Partial<Record<number, boolean>>;".to_owned(),
        ],
    );
    assert_eq!(
        written.matches(NOTED).count(),
        11,
        "one note per member that writes such a map:\n{written}"
    );
}

/// Zod reads the key as the integer `number` it read before the note.
#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_of_such_a_map_keys_it_by_an_integer_number() {
    let schema = WideKeyed::zod_schema();
    holds_each(
        &schema,
        &["direct: z.record(z.number().int(), z.boolean()),".to_owned()],
    );
    assert!(!schema.contains("@deprecated"), "the note in: {schema}");
}

#[test]
fn a_variant_carries_the_note_on_each_member_and_on_the_key_its_slots_are_under() {
    let external = WideKeyedEvent::ts_definition();
    holds_each(
        &external,
        &[
            format!(
                "   * Counted\n   * \n{NOTED}\n   */\n  \"Counted\": [Partial<Record<number, \
                 boolean>>, number];"
            ),
            format!(
                "   * Tallied\n   * \n{NOTED}\n   */\n  \"Tallied\": Partial<Record<number, \
                 boolean>>;"
            ),
            "   * Kept\n   * \n   */\n  \"Kept\": {\n".to_owned(),
            noted_member("keyed", "Partial<Record<number, boolean>>"),
            "   * keyed\n   * \n   */\n  keyed: Partial<Record<number, boolean>>;".to_owned(),
        ],
    );
    assert_eq!(external.matches(NOTED).count(), 3, "in:\n{external}");

    let adjacent = WideKeyedAdjacent::ts_definition();
    holds_each(
        &adjacent,
        &[
            format!(
                "  /**\n   * Tuple: [element 0, element 1]\n   * \n{NOTED}\n   */\n  body: \
                 [Partial<Record<number, boolean>>, number];"
            ),
            format!(
                "  /**\n   * Tuple value\n   * \n{NOTED}\n   */\n  body: Partial<Record<number, \
                 boolean>>;"
            ),
            noted_member("keyed", "Partial<Record<number, boolean>>"),
        ],
    );
    assert_eq!(adjacent.matches(NOTED).count(), 3, "in:\n{adjacent}");
}

/// A type whose value is such a map, or holds one in a slot, carries the note on itself, and on
/// the name it republishes itself under.
#[test]
fn a_type_whose_value_writes_such_a_map_carries_the_note() {
    let type_noted = NOTED.trim_start_matches("  ");
    assert_eq!(
        WideKeyedSlot::ts_definition(),
        format!(
            "/**\n * A map keyed by an `i64` as the value of a struct of one slot.\n * \n\
             {type_noted}\n */\nexport type WideKeyedSlot = Partial<Record<number, boolean>>;"
        )
    );
    assert_eq!(
        WideKeyedSlots::ts_definition(),
        format!(
            "/**\n * A map keyed by an `i64` in a struct of two slots.\n * \n{type_noted}\n \
             */\nexport type WideKeyedSlots = [Partial<Record<number, boolean>>, number];"
        )
    );
    assert_eq!(
        wide_keyed_alias_schema::Schema::ts_definition(),
        format!(
            "/**\n * An alias of a map keyed by an `i64`.\n * \n{type_noted}\n */\nexport type \
             WideKeyedAliasType = Partial<Record<number, boolean>>;\n\n{NOTE_BLOCK}\nexport \
             type WideKeyedAlias = WideKeyedAliasType;"
        )
    );
    assert!(
        WideKeyedBrand::ts_definition().starts_with(&format!(
            "{NOTE_BLOCK}\nexport type WideKeyedBrand = Partial<Record<number, boolean>> & "
        )),
        "in: {}",
        WideKeyedBrand::ts_definition()
    );
}

#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[test]
fn each_member_that_writes_a_map_keyed_by_a_u64_carries_the_note() {
    holds_each(
        &WideUnsignedKeyed::ts_definition(),
        &[
            noted_member("direct", "Partial<Record<number, boolean>>"),
            noted_member("listed", "Array<Partial<Record<number, boolean>>>"),
            noted_member("maybe", "Partial<Record<number, boolean>> | undefined"),
            noted_member(
                "nested",
                "Partial<Record<string, Partial<Record<number, boolean>>>>",
            ),
        ],
    );
}

#[test]
fn the_readme_shows_the_note_the_macro_writes() {
    let readme = include_str!("../../README.md");
    let declared = Histogram::ts_definition();
    assert!(
        readme.contains(&format!("```typescript\n{declared}\n```")),
        "the README no longer shows:\n{declared}"
    );
}
