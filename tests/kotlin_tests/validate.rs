//! `validate()` on every Kotlin type the emitter writes: the text of each shape, and the messages
//! Rust's own `validate()` answers of the same values, which hold every rule in every build. A
//! test whose fixture writes a serde attribute the emitter reads is compiled with the `serde`
//! feature alone.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[cfg(feature = "serde")]
use super::kotlin_flattened::FLATTENED;
#[cfg(feature = "serde")]
use super::kotlin_inner::INNER;
#[cfg(feature = "serde")]
use super::kotlin_pathed::PATHED;
#[cfg(feature = "serde")]
use super::kotlin_plain_enum::plain_enum_unjudged;
#[cfg(feature = "serde")]
use super::read::{
    DECODED, brand_companion, brand_serializers, kept_inline, mapped, mapped_over_a_brand,
};
use super::read::{
    NO_ISSUES, drafted_serializers, judged_valid, judged_valid_over, kept_draft, to_valid,
    valid_class, valid_class_over, valid_unjudged, valid_unjudged_over,
};
#[cfg(feature = "serde")]
use super::tests::{
    audit_kotlin, correlation_id_kotlin, date_value_kotlin, dynamic_value_kotlin,
    external_tagged_kotlin, simple_choice_kotlin, status_kotlin, window_error_kotlin,
};
use super::tests::{
    generic_pair_kotlin, holds_tuple_kotlin, ping_kotlin, widget_kotlin, wrapper_kotlin,
};

/// What a type with no rule answers.
const NO_RULE: &str = "fun validate(): List<String> = emptyList(); fun validationIssues(): \
     List<String> = validate()";

/// What a holder writes beside a `validate()` it calls on a name the emitter has not written a
/// type for.
const FALLBACK: &str = "private fun Any?.validationIssues(): List<String> = emptyList()";

/// What a string answers where a declared type is asked for its text.
#[cfg(feature = "serde")]
const TEXT: &str = "private fun String.kotlinSchemaText(): String = this";

/// What a string and a collection answer where a declared type is asked for its length.
#[cfg(feature = "serde")]
const LENGTH: &str = "private fun String.kotlinSchemaLength(): Int = codePointCount(0, length); \
                      private fun Collection<*>.kotlinSchemaLength(): Int = size";

/// What a type that names a place writes a message under it with.
const UNDER: &str = "private fun kotlinSchemaUnder(place: String, violation: String): String { \
     val end = violation.indexOf('\\'', 1); \
     return if (violation.startsWith(\"'\") && end > 0) { \
     \"'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}\" \
     } else { \"'$place': $violation\" } }";

/// The parcel of the shared table: a bound, a field's check and a type's check.
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

/// A holder of a type declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shelf {
    pub later: Later,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Later {
    pub label: String,
}

/// A brand over a string, which answers its text.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Slug(String);

/// A brand over a brand: it answers through the brand it holds.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Badge(Slug);

#[cfg(feature = "serde")]
#[model_schema()]
pub type Title = String;

/// A bound on a brand declared below, on one declared above and on an alias of a string.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Entry {
    #[model_schema_prop(minLength = 3)]
    pub late: LateSlug,
    #[model_schema_prop(minLength = 3)]
    pub slug: Slug,
    #[model_schema_prop(minLength = 2)]
    pub titled: Title,
}

#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LateSlug(String);

/// A variant of several slots under serde's own tagging.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Freight {
    Held,
    Split(#[model_schema_prop(minLength = 1)] String, Parcel),
}

/// A variant of several slots under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Haul {
    Held,
    Split(#[model_schema_prop(minLength = 1)] String, Parcel),
}

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

fn within_parcel(value: &ParcelDraft) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() {
        Ok(())
    } else {
        Err(outside)
    }
}

fn parcel() -> Parcel {
    Parcel::model_schema_wrap_unjudged(ParcelDraft {
        fragile_item_codes: vec!["y".to_owned()],
        item_code_list: vec!["x".to_owned(), "x".to_owned()],
        label: String::new(),
    })
}

/// The statement that holds the text the declared type `read` answers to `least` characters,
/// under `place`.
#[cfg(feature = "serde")]
fn asked_statement(read: &str, place: &str, least: u32) -> String {
    let counted = format!("{read}.kotlinSchemaLength()");
    format!(
        "if ({counted} < {least}) violations.add(\"{place}too short: minimum length is {least}, \
         got ${{{counted}}}\")"
    )
}

/// What a member typed with a declared type adds after its own bound: the type's `validate()`,
/// each message under the member's key.
#[cfg(feature = "serde")]
fn held_statement(read: &str) -> String {
    format!(
        "violations.addAll({read}.validationIssues().map {{ kotlinSchemaUnder(\"{read}\", it) }})"
    )
}

/// The statements a variant of several slots runs, each slot under `place` and its position.
fn split_statements(place: &str) -> String {
    format!(
        "{{ override fun validationIssues(): List<String> {{ val violations = \
         mutableListOf<String>(); if (value.slot0.codePointCount(0, value.slot0.length) < 1) violations.add(\"'{place}.0': \
         too short: minimum length is 1, got ${{value.slot0.codePointCount(0, \
         value.slot0.length)}}\"); violations.addAll(value.slot1.validationIssues().map {{ \
         kotlinSchemaUnder(\"{place}.1\", it) }}); return violations }}; {UNDER} }}"
    )
}

#[test]
fn every_fixture_is_written_as_serde_writes_it() {
    let held = parcel();
    assert_eq!(
        serde_json::to_value(ManifestDraft {
            parcel: held.clone()
        })
        .unwrap(),
        serde_json::json!({
            "parcel": { "fragileItemCodes": ["y"], "itemCodeList": ["x", "x"], "label": "" }
        })
    );
    assert_eq!(
        serde_json::to_value(ShelfDraft {
            later: LaterDraft {
                label: "a".to_owned()
            }
            .validate()
            .unwrap()
        })
        .unwrap(),
        serde_json::json!({ "later": { "label": "a" } })
    );
    assert_eq!(
        serde_json::to_value([
            FreightDraft::Held,
            FreightDraft::Split("a".to_owned(), held.clone())
        ])
        .unwrap()[0],
        serde_json::json!("Held")
    );
    assert_eq!(
        serde_json::to_value([HaulDraft::Held, HaulDraft::Split("a".to_owned(), held)]).unwrap()[0],
        serde_json::json!({ "kind": "Held" })
    );
    #[cfg(feature = "serde")]
    let titled: Title = "t".to_owned();
    #[cfg(feature = "serde")]
    assert_eq!(
        serde_json::to_value((
            Badge::new(Slug::new("b".to_owned()).unwrap()).unwrap(),
            EntryDraft {
                late: LateSlug::new("l".to_owned()).unwrap(),
                slug: Slug::new("s".to_owned()).unwrap(),
                titled,
            }
        ))
        .unwrap(),
        serde_json::json!(["b", { "late": "l", "slug": "s", "titled": "t" }])
    );
}

/// Rust's `validate()` reports each message the Kotlin `validate()` written for the fixture adds, in
/// the order it adds them: a member's check, then its bound, then the type's check, a held model's
/// under the member and a slot's under the variant and its position.
#[test]
fn rust_holds_each_fixture_to_every_rule_the_kotlin_validate_holds() {
    #[cfg(feature = "serde")]
    let (codes, held_slots) = ("itemCodeList", "held");
    #[cfg(not(feature = "serde"))]
    let (codes, held_slots) = ("item_code_list", "Split");
    let parcel_messages = [
        format!("'{codes}': duplicate item code x"),
        "'label': too short: minimum length is 1, got 0".to_owned(),
        "'fragileItemCodes.0': y is outside of itemCodeList".to_owned(),
    ];
    let under = |place: &str| -> Vec<String> {
        parcel_messages
            .iter()
            .map(|message| format!("'{place}.{}", &message[1..]))
            .collect()
    };
    assert_eq!(parcel().validation_issues(), parcel_messages);
    assert_eq!(
        ManifestDraft { parcel: parcel() }.validation_issues(),
        under("parcel")
    );
    #[cfg(feature = "serde")]
    assert_eq!(
        EntryDraft {
            late: LateSlug::new("l".to_owned()).unwrap(),
            slug: Slug::new("s".to_owned()).unwrap(),
            titled: "t".to_owned(),
        }
        .validate()
        .unwrap_err(),
        [
            "'late': too short: minimum length is 3, got 1",
            "'slug': too short: minimum length is 3, got 1",
            "'titled': too short: minimum length is 2, got 1",
        ]
    );
    let split = |place: &str| -> Vec<String> {
        let mut messages = vec![format!(
            "'{place}.0': too short: minimum length is 1, got 0"
        )];
        messages.extend(under(&format!("{place}.1")));
        messages
    };
    assert_eq!(
        FreightDraft::Split(String::new(), parcel())
            .validate()
            .unwrap_err(),
        split("Split")
    );
    assert_eq!(
        HaulDraft::Split(String::new(), parcel()).validation_issues(),
        split(held_slots)
    );
}

/// The model of the shared table: each member in the order declared, its bound, what it holds and
/// its checks, then the type's check.
#[cfg(feature = "serde")]
#[test]
fn a_class_runs_each_members_rules_and_then_its_own_check() {
    assert_eq!(
        super::tests::renumbered(&parcel_kotlin::kotlin_definition()),
        format!(
            "{} data class ParcelDraft(val fragileItemCodes: List<String>, val itemCodeList: \
             List<String>, val label: String) {{ fun validationIssues(): List<String> {{ val \
             violations = mutableListOf<String>(); \
             violations.addAll(withoutDuplicates(itemCodeList).map {{ \
             kotlinSchemaUnder(\"itemCodeList\", it) }}); if (label.codePointCount(0, \
             label.length) < 1) violations.add(\"'label': too short: minimum length is 1, got \
             ${{label.codePointCount(0, label.length)}}\"); \
             violations.addAll(withinParcel(this)); return violations }}; {UNDER} {} }} {}",
            kept_draft("Parcel"),
            to_valid("Parcel", ""),
            drafted_serializers(
                "Parcel",
                "val fragileItemCodes: List<String> get() = draft.fragileItemCodes; val \
                 itemCodeList: List<String> get() = draft.itemCodeList; val label: String get() = \
                 draft.label; "
            )
        )
    );
    assert_eq!(
        super::tests::renumbered(&manifest_kotlin::kotlin_definition()),
        format!(
            "{} data class ManifestDraft(@Serializable(with = ParcelUnjudged::class) val parcel: \
             Parcel) {{ fun validationIssues(): List<String> {{ val violations = \
             mutableListOf<String>(); \
             violations.addAll(parcel.validationIssues().map {{ kotlinSchemaUnder(\"parcel\", \
             it) }}); return violations }}; {UNDER} {} }} {}",
            kept_draft("Manifest"),
            to_valid("Manifest", ""),
            drafted_serializers("Manifest", "val parcel: Parcel get() = draft.parcel; ")
        )
    );
}

#[test]
fn a_holder_of_a_type_declared_below_writes_what_a_type_with_no_validate_answers() {
    assert_eq!(
        super::tests::renumbered(&shelf_kotlin::kotlin_definition()),
        format!(
            "{} data class ShelfDraft(val later: Later) {{ fun validationIssues(): List<String> \
             {{ val violations = mutableListOf<String>(); violations.addAll(later.validationIssues().map \
             {{ kotlinSchemaUnder(\"later\", it) }}); return violations }}; {FALLBACK}; {UNDER} \
             {} }} {}",
            kept_draft("Shelf"),
            to_valid("Shelf", ""),
            drafted_serializers("Shelf", "val later: Later get() = draft.later; ")
        )
    );
    assert_eq!(
        super::tests::renumbered(&later_kotlin::kotlin_definition()),
        format!(
            "{} data class LaterDraft(val label: String) {{ {NO_ISSUES} {} }} {}",
            kept_draft("Later"),
            to_valid("Later", ""),
            drafted_serializers("Later", "val label: String get() = draft.label; ")
        )
    );
}

/// A brand over a string answers its text with a member, and a brand over a brand through the
/// brand it holds; a map keyed by the brand over a brand throws the refusal of the brand it holds
/// under the key's text.
#[cfg(feature = "serde")]
#[test]
fn a_brand_answers_its_text() {
    assert_eq!(
        super::tests::renumbered(&slug_kotlin::kotlin_definition()),
        format!(
            "{} value class Slug private constructor(val value: String) {{ fun \
             kotlinSchemaText(): String = value; fun kotlinSchemaLength(): Int = \
             value.codePointCount(0, value.length); {NO_RULE}; {} }} {} {}",
            kept_inline("Slug"),
            brand_companion("Slug", "String"),
            brand_serializers("Slug"),
            mapped("Slug")
        )
    );
    assert_eq!(
        super::tests::renumbered(&badge_kotlin::kotlin_definition()),
        format!(
            "{} value class Badge private constructor(val value: Slug) {{ fun \
             kotlinSchemaText(): String = value.kotlinSchemaText(); fun kotlinSchemaLength(): Int \
             = value.kotlinSchemaLength(); {NO_RULE}; {LENGTH}; {TEXT}; {} }} {} {}",
            kept_inline("Badge"),
            brand_companion("Badge", "Slug"),
            brand_serializers("Badge"),
            mapped_over_a_brand("Badge")
        )
    );
}

/// A holder asks a declared type for its text and answers for a string itself, so a brand
/// declared above, one declared below and an alias of a string are each measured.
#[cfg(feature = "serde")]
#[test]
fn a_bound_on_a_declared_type_measures_the_text_the_type_answers() {
    assert_eq!(
        super::tests::renumbered(&entry_kotlin::kotlin_definition()),
        format!(
            "{} data class EntryDraft(val late: LateSlug, @Serializable(with = SlugUnjudged::class) \
             val slug: Slug, val titled: TitleType) {{ fun validationIssues(): List<String> {{ val \
             violations = mutableListOf<String>(); {}; {}; {}; {}; {}; {}; return violations }}; \
             private fun Any?.validationIssues(): List<String> = emptyList(); {LENGTH}; {UNDER} {} }} {}",
            kept_draft("Entry"),
            held_statement("late"),
            asked_statement("late", "'late': ", 3),
            held_statement("slug"),
            asked_statement("slug", "'slug': ", 3),
            held_statement("titled"),
            asked_statement("titled", "'titled': ", 2),
            to_valid("Entry", ""),
            drafted_serializers(
                "Entry",
                "val late: LateSlug get() = draft.late; val slug: Slug get() = draft.slug; val \
                 titled: TitleType get() = draft.titled; "
            )
        )
    );
}

/// A slot of a variant of several is named by the step serde writes the variant under and by its
/// position: the variant's name where no tag is written.
#[test]
fn a_slot_of_a_variant_of_several_is_named_by_the_variant_and_its_position() {
    let freight = super::tests::renumbered(&freight_kotlin::kotlin_definition());
    assert!(
        freight.contains(&format!(") : FreightDraft {}", split_statements("Split"))),
        "got: {freight}"
    );
}

/// Under a tag and a content key, the same slot is named by the content key and its position.
#[cfg(feature = "serde")]
#[test]
fn a_slot_of_a_variant_of_several_is_named_by_the_content_key_and_its_position() {
    let haul = super::tests::renumbered(&haul_kotlin::kotlin_definition());
    assert!(
        haul.contains(&format!(") : HaulDraft {}", split_statements("held"))),
        "got: {haul}"
    );
}

#[test]
fn every_shape_carries_validate() {
    for (definition, written) in [
        (
            super::tests::renumbered(&ping_kotlin::kotlin_definition()),
            format!(
                "{} object PingDraft {{ {NO_ISSUES} {} }} {}",
                kept_draft("Ping"),
                to_valid("Ping", ""),
                drafted_serializers("Ping", "")
            ),
        ),
        #[cfg(feature = "serde")]
        (
            super::tests::renumbered(&correlation_id_kotlin::kotlin_definition()),
            format!(
                "{} value class CorrelationId private constructor(val value: String) {{ \
                 fun kotlinSchemaText(): String = value; fun kotlinSchemaLength(): Int = \
                 value.codePointCount(0, value.length); {NO_RULE}; {} }} {} {}",
                kept_inline("CorrelationId"),
                brand_companion("CorrelationId", "String"),
                brand_serializers("CorrelationId"),
                mapped("CorrelationId")
            ),
        ),
        (
            super::tests::renumbered(&wrapper_kotlin::kotlin_definition()),
            format!(
                "{} data class WrapperDraft<T>(val value: T) {{ {NO_ISSUES} {} }} \
                 @OptIn(ExperimentalSerializationApi::class) class \
                 WrapperDraftUnjudged<T>(tSerializer: KSerializer<T>) : KSerializer<WrapperDraft<T>> \
                 by WrapperDraft.generatedSerializer(tSerializer) {} {} {}",
                kept_draft("Wrapper"),
                to_valid("Wrapper", "<T>"),
                valid_class_over("Wrapper", "<T>", "val value: T get() = draft.value; "),
                valid_unjudged_over("Wrapper", "T", "", "tSerializer"),
                judged_valid_over("Wrapper", "T", "", "tSerializer")
            ),
        ),
        (
            super::tests::renumbered(&widget_kotlin::kotlin_definition()),
            format!(
                "{} data class RenamedWidgetDraft(val label: String) {{ {NO_ISSUES} {} }} {} typealias \
                 Widget = RenamedWidget;",
                kept_draft("RenamedWidget"),
                to_valid("RenamedWidget", ""),
                drafted_serializers("RenamedWidget", "val label: String get() = draft.label; ")
            ),
        ),
    ] {
        assert_eq!(definition, written);
    }
    let pair = super::tests::renumbered(&generic_pair_kotlin::kotlin_definition());
    assert!(
        pair.contains(&format!(
            "data class GenericPairDraft<T>(val slot0: T, val slot1: UInt) {{ {NO_ISSUES} {} }} \
             class GenericPairDraftUnjudged<T: Any>(",
            to_valid("GenericPair", "<T>")
        )),
        "got: {pair}"
    );
    assert!(
        pair.ends_with(&format!(
            "{} {}",
            valid_unjudged_over("GenericPair", "T", ": Any", "tSerializer"),
            judged_valid_over("GenericPair", "T", ": Any", "tSerializer")
        )),
        "got: {pair}"
    );
    let holder = super::tests::renumbered(&holds_tuple_kotlin::kotlin_definition());
    assert!(
        holder.contains(&format!(
            "{} data class HoldsTupleDraft(val coords: KotlinTuple",
            kept_draft("HoldsTuple")
        )) && holder.contains(&format!(
            ") {{ {NO_ISSUES} {} }} ",
            to_valid("HoldsTuple", "")
        )) && holder.ends_with(&format!(
            "{} {}",
            valid_unjudged("HoldsTuple"),
            judged_valid("HoldsTuple")
        )),
        "got: {holder}"
    );
}

/// A plain enum carries the method after its members.
#[cfg(feature = "serde")]
#[test]
fn a_plain_enum_carries_validate() {
    assert_eq!(
        super::tests::renumbered(&status_kotlin::kotlin_definition()),
        format!(
            "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
             StatusUnjudged::class) @KeepGeneratedSerializer enum class Status {{ \
             @SerialName(\"active\") Active, @SerialName(\"inactive\") Inactive, \
             @SerialName(\"pending\") Pending; {NO_RULE} }} {}",
            plain_enum_unjudged(
                "Status",
                &[
                    ("Active", "active", &[]),
                    ("Inactive", "inactive", &[]),
                    ("Pending", "pending", &[])
                ],
                "element"
            )
        )
    );
}

/// A sealed interface declares the method, and each variant overrides it, under every tagging.
#[cfg(feature = "serde")]
#[test]
fn a_sealed_interface_declares_validate_and_each_variant_overrides_it() {
    let overridden = format!("{{ override {NO_ISSUES} }}");
    for (definition, interface, variants) in [
        (
            super::tests::renumbered(&window_error_kotlin::kotlin_definition()),
            "WindowError",
            vec![
                "data object WindowErrorNotFound : WindowErrorDraft",
                "data class WindowErrorRateLimited(@SerialName(\"retry_after_ms\") val \
                 retryAfterMs: Long) : WindowErrorDraft",
            ],
        ),
        (
            super::tests::renumbered(&dynamic_value_kotlin::kotlin_definition()),
            "DynamicValue",
            vec![
                "data class DynamicValueFlag(val value: Boolean) : DynamicValueDraft",
                "data object DynamicValueNothing : DynamicValueDraft",
                "data class DynamicValueNumber(val value: Long) : DynamicValueDraft",
            ],
        ),
        (
            super::tests::renumbered(&external_tagged_kotlin::kotlin_definition()),
            "ExternalTagged",
            vec![
                "data class ExternalTaggedPing(val nonce: Int) : ExternalTaggedDraft",
                "data class ExternalTaggedPong(val nonce: Int) : ExternalTaggedDraft",
            ],
        ),
        (
            super::tests::renumbered(&date_value_kotlin::kotlin_definition()),
            "DateValue",
            vec![
                "value class DateValueEpoch(val value: Long) : DateValueDraft",
                "value class DateValueIso(val value: String) : DateValueDraft",
            ],
        ),
    ] {
        let types = super::tests::without_number_readers(&definition);
        assert!(
            types.contains(&format!(
                "sealed interface {interface}Draft {{ fun validationIssues(): List<String>; {} }} ",
                to_valid(interface, "")
            )),
            "got: {definition}"
        );
        for variant in variants {
            assert!(
                types.contains(&format!("{variant} {overridden}")),
                "no `{variant}` with its override in: {definition}"
            );
        }
    }
}

/// A flattened member writes no key, so what it reports is written under no name.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_member_is_reported_under_no_name() {
    let audit = super::tests::renumbered(&audit_kotlin::kotlin_definition());
    assert!(
        audit.contains(&format!(
            "data class AuditDraft(val id: String, val stamp: Stamp) {{ fun \
                 validationIssues(): List<String> {{ val violations = mutableListOf<String>(); \
                 violations.addAll(stamp.validationIssues()); return violations }} {} }} \
                 object AuditDraftUnjudged",
            to_valid("Audit", "")
        )),
        "got: {audit}"
    );
}

/// The serializer the emitter writes for a struct with a flattened member and for a tuple struct
/// is its unjudged one, a model it holds read unjudged, and the type's own serializer follows it.
/// The struct refuses a key neither its own member nor the struct it flattens reads.
#[cfg(feature = "serde")]
#[test]
fn the_serializer_the_emitter_writes_for_each_struct_shape_is_its_unjudged_one() {
    for (definition, written, own) in [
        (
            super::tests::renumbered(&audit_kotlin::kotlin_definition()),
            "object AuditDraftUnjudged : KSerializer<AuditDraft> { override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"Audit\"); override fun \
             serialize(encoder: Encoder, value: AuditDraft) { val output = encoder as \
             JsonEncoder; \
             output.encodeJsonElement(buildJsonObject { put(\"id\", \
             output.json.encodeToJsonElement(serializer<String>(), value.id)); \
             output.json.encodeToJsonElement(StampUnjudged, value.stamp).jsonObject.forEach { (k, \
             v) -> put(k, v) } }) }; override fun deserialize(decoder: Decoder): AuditDraft { val \
             input = decoder as JsonDecoder; val element = input.decodeJsonElement(); return \
             kotlinSchemaAt(input) { val obj = element.let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }; val id = \
             kotlinSchemaDecoded(input.json, serializer<String>(), (obj[\"id\"] ?: throw \
             SerializationException(\"missing field `id`\")), \"id\", \".id\"); val stampKeys = \
             setOf(\"created_at\", \"updated_at\"); val stamp = kotlinSchemaFlattened { \
             input.json.decodeFromString(StampUnjudged, JsonObject(obj.filterKeys { it in \
             setOf(\"created_at\", \"updated_at\") }).toString()) }; if \
             (!input.json.configuration.ignoreUnknownKeys) (obj - setOf(\"id\") - \
             setOf(\"created_at\", \"updated_at\")).keys.firstOrNull()?.let { throw \
             SerializationException(\"Encountered an unknown key '\" + it + \"'\") }; AuditDraft(id \
             = id, stamp = stamp) } } }",
            format!(
                "{} {} {}",
                valid_class(
                    "Audit",
                    "val id: String get() = draft.id; val stamp: Stamp get() = draft.stamp; "
                ),
                valid_unjudged("Audit"),
                judged_valid("Audit")
            ),
        ),
        (
            super::tests::renumbered(&generic_pair_kotlin::kotlin_definition()),
            "class GenericPairDraftUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : \
             KSerializer<GenericPairDraft<T>> { override val descriptor: SerialDescriptor = \
             buildClassSerialDescriptor(\"GenericPair\"); override fun serialize(encoder: \
             Encoder, value: GenericPairDraft<T>) { val output = encoder as JsonEncoder; \
             output.encodeJsonElement(buildJsonArray { \
             add(output.json.encodeToJsonElement(tSerializer, value.slot0)); \
             add(output.json.encodeToJsonElement(KotlinNumber1, value.slot1)) }) }; override fun \
             deserialize(decoder: Decoder): GenericPairDraft<T> { val input = decoder as \
             JsonDecoder; \
             val element = input.decodeJsonElement(); return kotlinSchemaAt(input) { val array = \
             element.let { it as? JsonArray ?: throw SerializationException(\"expected an array, \
             found \" + it) }; if (array.size != 2) throw SerializationException(\"invalid length \
             \" + array.size + \", expected 2\"); GenericPairDraft(kotlinSchemaDecoded(input.json, \
             tSerializer, array[0], \"0\", \"[0]\"), kotlinSchemaDecoded(input.json, \
             KotlinNumber1, array[1], \"1\", \"[1]\")) } } }",
            format!(
                "{} {} {}",
                valid_class_over(
                    "GenericPair",
                    "<T>",
                    "val slot0: T get() = draft.slot0; val slot1: UInt get() = draft.slot1; "
                ),
                valid_unjudged_over("GenericPair", "T", ": Any", "tSerializer"),
                judged_valid_over("GenericPair", "T", ": Any", "tSerializer")
            ),
        ),
    ] {
        let unjudged = with_helpers(written);
        assert!(
            definition.ends_with(&format!("{unjudged} {own}")),
            "the serializers moved in: {definition}"
        );
    }
}

/// `written`, a serializer the emitter writes, behind the private functions its read calls.
#[cfg(feature = "serde")]
fn with_helpers(written: &str) -> String {
    let pathed = written.contains("kotlinSchemaAt(")
        || written.contains("kotlinSchemaInner(")
        || written.contains("kotlinSchemaDecoded(");
    let helpers = [
        (written.contains("kotlinSchemaDecoded("), DECODED),
        (pathed, PATHED),
        (written.contains("kotlinSchemaInner("), INNER),
        (written.contains("kotlinSchemaFlattened {"), FLATTENED),
    ]
    .iter()
    .filter(|(used, _)| *used)
    .map(|(_, helper)| *helper)
    .collect::<String>();
    written.replacen(
        "override fun deserialize",
        &format!("{helpers}override fun deserialize"),
        1,
    )
}

/// The serializer the emitter writes for an enum under each tagging is its unjudged one, and the
/// type's own serializer follows it.
#[cfg(feature = "serde")]
#[test]
fn the_serializer_the_emitter_writes_for_each_enum_shape_is_its_unjudged_one() {
    for (definition, written, own) in [
        (
            super::tests::renumbered(&simple_choice_kotlin::kotlin_definition()),
            "object SimpleChoiceDraftUnjudged : KSerializer<SimpleChoiceDraft> { override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"SimpleChoice\"); override fun \
             serialize(encoder: Encoder, value: SimpleChoiceDraft) { val output = encoder as \
             JsonEncoder; output.encodeJsonElement(when (value) { is SimpleChoiceNone -> \
             JsonPrimitive(\"None\"); is SimpleChoiceValue -> buildJsonObject { put(\"Value\", \
             output.json.encodeToJsonElement(serializer<String>(), value.value)) } }) }; override \
             fun deserialize(decoder: Decoder): SimpleChoiceDraft { val input = decoder as \
             JsonDecoder; val element = input.decodeJsonElement(); return kotlinSchemaAt(input) { \
             val (tag, data) = if (element is JsonPrimitive && element.isString) \
             element.content to JsonNull else (element as? JsonObject)?.entries?.singleOrNull()\
             ?.let { it.key to it.value } ?: throw SerializationException(\"expected value\"); when \
             (tag) { \"None\" -> kotlinSchemaUnder(\".\" + tag) { if (data is JsonNull) \
             SimpleChoiceNone else throw SerializationException(\"SimpleChoice: \" + tag + \" \
             takes no value\") }; \"Value\" -> kotlinSchemaUnder(\".\" + tag) { \
             SimpleChoiceValue(kotlinSchemaDecoded(input.json, serializer<String>(), data, \
             \"Value\", \"\")) } else -> throw SerializationException(\"unknown tag \" + tag) } } \
             } }",
            format!(
                "{} {} {}",
                valid_class("SimpleChoice", ""),
                valid_unjudged("SimpleChoice"),
                judged_valid("SimpleChoice")
            ),
        ),
        (
            super::tests::renumbered(&dynamic_value_kotlin::kotlin_definition()),
            "object DynamicValueDraftUnjudged : KSerializer<DynamicValueDraft> { override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"DynamicValue\"); override fun \
             serialize(encoder: Encoder, value: DynamicValueDraft) { val output = encoder as \
             JsonEncoder; output.encodeJsonElement(when (value) { is DynamicValueFlag -> \
             buildJsonObject { put(\"type\", \"Flag\"); put(\"value\", \
             output.json.encodeToJsonElement(KotlinNumber1, value.value)) }; is \
             DynamicValueNothing -> buildJsonObject { put(\"type\", \"Nothing\") }; is \
             DynamicValueNumber -> buildJsonObject { put(\"type\", \"Number\"); put(\"value\", \
             output.json.encodeToJsonElement(KotlinNumber2, value.value)) } }) }; override fun \
             deserialize(decoder: Decoder): DynamicValueDraft { val input = decoder as JsonDecoder; \
             val element = input.decodeJsonElement(); return kotlinSchemaAt(input) { val obj = \
             element.let { it as? JsonObject ?: throw SerializationException(\"expected an \
             object, found \" + it) }; val tag = (obj[\"type\"] ?: throw \
             SerializationException(\"missing field `type`\")).let { it as? JsonPrimitive ?: \
             throw SerializationException(\"expected a string, found \" + it) }.content; val data \
             = obj[\"value\"]; if (!input.json.configuration.ignoreUnknownKeys) (obj - \"type\" - \
             \"value\").keys.firstOrNull()?.let { throw SerializationException(\"Encountered an \
             unknown key '\" + it + \"'\") }; when (tag) { \"Flag\" -> \
             kotlinSchemaUnder(\".value\") { DynamicValueFlag(kotlinSchemaDecoded(input.json, \
             KotlinNumber1, (data ?: throw SerializationException(\"missing field `value`\")), \
             \"value\", \"\")) }; \"Nothing\" -> kotlinSchemaUnder(\".value\") { if (data == \
             null) DynamicValueNothing else throw SerializationException(\"DynamicValue: \" + tag \
             + \" takes no value\") }; \"Number\" -> kotlinSchemaUnder(\".value\") { \
             DynamicValueNumber(kotlinSchemaDecoded(input.json, KotlinNumber2, (data ?: throw \
             SerializationException(\"missing field `value`\")), \"value\", \"\")) } else -> \
             throw SerializationException(\"unknown tag \" + tag) } } } }",
            format!(
                "{} {} {}",
                valid_class("DynamicValue", ""),
                valid_unjudged("DynamicValue"),
                judged_valid("DynamicValue")
            ),
        ),
        (
            super::tests::renumbered(&date_value_kotlin::kotlin_definition()),
            "object DateValueDraftUnjudged : KSerializer<DateValueDraft> { override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"DateValue\"); override fun \
             serialize(encoder: Encoder, value: DateValueDraft) { val output = encoder as JsonEncoder; \
             output.encodeJsonElement(when (value) { is DateValueEpoch -> \
             output.json.encodeToJsonElement(serializer<DateValueEpoch>(), value); is \
             DateValueIso -> output.json.encodeToJsonElement(serializer<DateValueIso>(), value) \
             }) }; override fun deserialize(decoder: Decoder): DateValueDraft { val input = decoder as \
             JsonDecoder; val element = input.decodeJsonElement(); return kotlinSchemaAt(input) { \
             runCatching { error(\"unreachable\") as Nothing }.recoverCatching { \
             input.json.decodeFromJsonElement(serializer<DateValueEpoch>(), element) as DateValueDraft \
             }.recoverCatching { input.json.decodeFromJsonElement(serializer<DateValueIso>(), \
             element) as DateValueDraft }.getOrElse { throw SerializationException(\"No variant of \
             DateValue matched\") } } } }",
            format!(
                "{} {} {}",
                valid_class("DateValue", ""),
                valid_unjudged("DateValue"),
                judged_valid("DateValue")
            ),
        ),
    ] {
        let unjudged = with_helpers(written);
        assert!(
            definition.ends_with(&format!("{unjudged} {own}")),
            "the serializers moved in: {definition}"
        );
    }
}
