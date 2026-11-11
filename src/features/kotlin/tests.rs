//! Unit tests of the Kotlin emitter: a unit struct renders as an `object`, a braced empty struct as
//! a class, and what `validate()` and the two serializers of a type write for the shapes no
//! integration test can declare.

mod arrays;
mod preprocess;
mod texts;

use proc_macro2::{Span, TokenTree};

use super::read::{DECODED, INNER};
use super::validate::jvm_pattern;
use super::*;
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;

/// What a type that names a place writes a message under it with.
const UNDER: &str = "private fun kotlinSchemaUnder(place: String, violation: String): String { \
     val end = violation.indexOf('\\'', 1); \
     return if (violation.startsWith(\"'\") && end > 0) { \
     \"'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}\" \
     } else { \"'$place': $violation\" } }";

/// The read of an entry's value in the serializer of a map keyed by a brand: from its text under
/// the key's text.
const MAPPED_VALUE: &str =
    "kotlinSchemaDecoded(input.json, valueSerializer, entry.value, entry.key, \".\" + entry.key)";

/// The Kotlin source the tokens of an item publish: the first string literal they hold.
fn definition(tokens: TokenStream) -> String {
    fn first_literal(tokens: TokenStream) -> Option<String> {
        tokens.into_iter().find_map(|tree| match tree {
            TokenTree::Group(group) => first_literal(group.stream()),
            TokenTree::Literal(literal) => syn::parse_str::<syn::LitStr>(&literal.to_string())
                .ok()
                .map(|text| text.value()),
            TokenTree::Ident(_) | TokenTree::Punct(_) => None,
        })
    }
    first_literal(tokens).unwrap()
}

/// The serializer that reads `named` with no judgement where the plugin writes it.
fn generated(named: &str) -> String {
    format!(
        "@OptIn(ExperimentalSerializationApi::class) object {named}Unjudged : \
         KSerializer<{named}> by {named}.generatedSerializer()"
    )
}

/// The serializer of `named`, a type with no parameter: its unjudged read, then `validate()`.
fn judged(named: &str) -> String {
    format!(
        "object {named}Serializer : KSerializer<{named}>, () -> KSerializer<{named}> {{ \
         override fun invoke(): KSerializer<{named}> = {named}Unjudged; override val \
         descriptor: SerialDescriptor = {named}Unjudged.descriptor; override fun \
         serialize(encoder: Encoder, value: {named}) = {named}Unjudged.serialize(encoder, \
         value); override fun deserialize(decoder: Decoder): {named} {{ val read = \
         {named}Unjudged.deserialize(decoder); val violations = read.validate(); if \
         (violations.isNotEmpty()) throw SerializationException(violations.joinToString(\"; \"), \
         IllegalArgumentException(\"validate()\")); return read }} }}"
    )
}

/// The serializer of `named`, a type with the one parameter `parameter`, which `bound` follows.
fn judged_over(named: &str, parameter: &str, bound: &str, handed: &str) -> String {
    format!(
        "class {named}Serializer<{parameter}{bound}>({handed}: \
         KSerializer<{parameter}>) : KSerializer<{named}<{parameter}>>, () -> \
         KSerializer<{named}<{parameter}>> {{ private val kotlinSchemaUnjudged = \
         {named}Unjudged({handed}); override fun invoke(): \
         KSerializer<{named}<{parameter}>> = kotlinSchemaUnjudged; override val descriptor: \
         SerialDescriptor = kotlinSchemaUnjudged.descriptor; override fun serialize(encoder: \
         Encoder, value: {named}<{parameter}>) = kotlinSchemaUnjudged.serialize(encoder, \
         value); override fun deserialize(decoder: Decoder): {named}<{parameter}> {{ val read \
         = kotlinSchemaUnjudged.deserialize(decoder); val violations = read.validate(); if \
         (violations.isNotEmpty()) throw SerializationException(violations.joinToString(\"; \"), \
         IllegalArgumentException(\"validate()\")); return read }} }}"
    )
}

/// The serializer of a map keyed by the brand `named`, which reads each entry as `entry` says.
fn mapped_reading(named: &str, entry: &str) -> String {
    format!(
        "class {named}MapSerializer<KotlinSchemaValue>(private val keySerializer: \
         KSerializer<{named}>, private val valueSerializer: KSerializer<KotlinSchemaValue>) : \
         KSerializer<Map<{named}, KotlinSchemaValue>> {{ private val whole = \
         MapSerializer(keySerializer, valueSerializer); override val descriptor: SerialDescriptor \
         = whole.descriptor; override fun serialize(encoder: Encoder, value: Map<{named}, \
         KotlinSchemaValue>) = whole.serialize(encoder, value); {DECODED}{PATHED}override fun \
         deserialize(decoder: Decoder): Map<{named}, KotlinSchemaValue> {{ val input = decoder \
         as JsonDecoder; val element = input.decodeJsonElement(); return kotlinSchemaAt(input) \
         {{ (element as? JsonObject ?: throw SerializationException(\"expected an object, found \
         \" + element)).entries.associate {{ entry -> {entry} }} }} }} }}",
        PATHED = read::PATHED
    )
}

/// The serializer of a map keyed by the brand `named`, which has a key serializer of its own: each
/// key read by it from its text at the map's path, which judges it.
fn mapped(named: &str) -> String {
    mapped_reading(
        named,
        &format!(
            "kotlinSchemaDecoded(input.json, keySerializer, JsonPrimitive(entry.key), entry.key, \
             \"\") to {MAPPED_VALUE}"
        ),
    )
}

/// The serializer of a map keyed by the brand `named`, which has no key serializer: each key read
/// unjudged from its text at the map's path, then judged, each message under the key's text.
fn mapped_judging(named: &str) -> String {
    mapped_reading(
        named,
        &format!(
            "val key = kotlinSchemaDecoded(input.json, {named}Unjudged, JsonPrimitive(entry.key), \
             entry.key, \"\"); val violations = key.validate(); if (violations.isNotEmpty()) \
             throw SerializationException(violations.joinToString(\"; \") {{ \"'\" + entry.key + \
             \"': \" + it }}); key to {MAPPED_VALUE}"
        ),
    )
}

/// The serializer of a map keyed by the brand `named`, which holds a brand over a text: each key
/// read as [`mapped_judging`] reads it, where the refusal of the brand it holds is thrown under the
/// key's text as well.
fn mapped_over_a_brand(named: &str) -> String {
    mapped_reading(
        named,
        &format!(
            "val key = try {{ kotlinSchemaDecoded(input.json, {named}Unjudged, \
             JsonPrimitive(entry.key), entry.key, \"\") }} catch (refused: SerializationException) \
             {{ if (refused.cause?.message != \"validate()\") throw refused; throw \
             SerializationException(\"'\" + entry.key + \"': \" + refused.message.orEmpty()) }}; \
             val violations = key.validate(); if (violations.isNotEmpty()) throw \
             SerializationException(violations.joinToString(\"; \") {{ \"'\" + entry.key + \"': \" \
             + it }}); key to {MAPPED_VALUE}"
        ),
    )
}

/// The annotations of a type the plugin reads, which keeps the plugin's serializer.
fn kept(named: &str) -> String {
    format!(
        "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
         {named}Serializer::class) @KeepGeneratedSerializer"
    )
}

/// A parcel with one bound, written so that a type declared after it finds it written.
fn labelled_parcel() -> String {
    let item: ItemStruct = syn::parse_quote! {
        struct Parcel {
            #[model_schema_prop(minLength = 1)]
            label: String,
        }
    };
    plain(&item)
}

/// The Kotlin source of a struct that writes no argument.
fn plain(item: &ItemStruct) -> String {
    definition(struct_kotlin_tokens(item, &plain_args()))
}

/// The object that reads a number of Kotlin type `kotlin`, which serde words as `expected`,
/// declared under `named`.
fn number(named: &str, kotlin: &str, expected: &str) -> String {
    format!(
        "object {named} : KSerializer<{kotlin}> {{ private val held = serializer<{kotlin}>(); \
         override val descriptor: SerialDescriptor = held.descriptor; override fun \
         serialize(encoder: Encoder, value: {kotlin}) = held.serialize(encoder, value); \
         {PATHED}{FLOATING}override fun deserialize(decoder: Decoder): {kotlin} {{ val json = \
         decoder as? JsonDecoder ?: return held.deserialize(decoder); val element = \
         json.decodeJsonElement(); return kotlinSchemaAt(json) {{ if (element !is JsonPrimitive \
         || element.isString) throw SerializationException(\"invalid type: \" + (if (element is \
         JsonPrimitive) \"string \" else \"\") + element + \", expected {expected}\"); val text = \
         element.content; val wide = text.toDoubleOrNull(); if (wide != null && (text == \"-0\" || \
         text.any {{ it == '.' || it == 'e' || it == 'E' }})) {{ if (wide.isInfinite()) throw \
         SerializationException(\"number out of range\"); throw \
         SerializationException(\"invalid type: floating point `\" + floating(wide) + \"`, \
         expected {expected}\") }}; try {{ json.json.decodeFromJsonElement(held, element) }} catch \
         (refused: SerializationException) {{ throw \
         SerializationException(refused.message.orEmpty().replaceFirst(\" at path: \
         \\$.primitive\", \" at path: \\$\"), refused) }} }} }} }}",
        PATHED = read::PATHED,
        FLOATING = super::texts::FLOATING
    )
}

/// The arguments of an item that writes none: each member's own `model_schema_prop`, as written.
fn plain_args() -> KotlinArgs<'static> {
    KotlinArgs {
        check: &[],
        default_types: &[],
        max_length: None,
        member_meta: &|member| parse_model_schema_prop_attributes(&member.attrs),
        min_length: None,
        name_override: None,
        pattern: None,
        preprocess: &[],
        type_name: "Probe",
    }
}

/// A unit struct is Kotlin's own no-data type: `@Serializable object`, never a `class`.
#[test]
fn a_unit_struct_renders_as_an_object() {
    let item: ItemStruct = syn::parse_quote! {
        struct Ping;
    };
    let rendered = struct_kotlin_tokens(&item, &plain_args()).to_string();
    assert!(rendered.contains("object Ping"), "got: {rendered}");
    assert!(!rendered.contains("class Ping"), "got: {rendered}");
}

#[test]
fn a_braced_empty_struct_still_renders_as_a_class() {
    let item: ItemStruct = syn::parse_quote! {
        struct BracedEmpty {}
    };
    let rendered = struct_kotlin_tokens(&item, &plain_args()).to_string();
    assert!(rendered.contains("class BracedEmpty {"), "got: {rendered}");
    assert!(
        !rendered.contains("object BracedEmpty {"),
        "got: {rendered}"
    );
}

#[test]
fn a_pattern_is_written_as_the_jvm_reads_what_rust_reads() {
    for (written, read) in [
        ("^[a-z]+$", "^[a-z]+\\z"),
        ("^[\\t\\n\\v\\f\\r ]+$", "^[\\t\\n\\x0B\\f\\r ]+\\z"),
        ("^[$]\\$", "^[$]\\$"),
        ("^(?<first_name>[a-z]+)-(?:x|y)$", "^([a-z]+)-(?:x|y)\\z"),
        ("a$|b", "a\\z|b"),
    ] {
        assert_eq!(jvm_pattern(written), read);
    }
}

#[test]
fn text_inside_a_kotlin_string_opens_no_template_and_closes_no_string() {
    assert_eq!(
        kotlin_template_text("^\\d+\"x\"$oid\n"),
        "^\\\\d+\\\"x\\\"\\$oid\\n"
    );
}

/// A number is compared in the Kotlin type it is held as, and as a double where the bound is no
/// value of that type.
#[test]
fn a_range_is_compared_in_the_numbers_own_type() {
    let item: ItemStruct = syn::parse_quote! {
        struct Ranges {
            #[model_schema_prop(minimum = 1)]
            count: u8,
            #[model_schema_prop(minimum = -5)]
            depth: i64,
            #[model_schema_prop(minimum = 1)]
            half: u32,
            #[model_schema_prop(maximum = 120)]
            ratio: f64,
            #[model_schema_prop(maximum = 1.5)]
            tilt: f32,
        }
    };
    let kotlin = plain(&item);
    for compared in [
        "if (count < 1u) violations.add(\"'count': too small: minimum is 1, got ${count}\")",
        "if (depth < -5) violations.add(\"'depth': too small: minimum is -5, got ${depth}\")",
        "if (half < 1u) violations.add(",
        "if (ratio > 120.0) violations.add(\"'ratio': too large: maximum is 120, got \
         ${kotlinSchemaShown(ratio.toString())}\")",
        "if (tilt > 1.5f) violations.add(",
    ] {
        assert!(kotlin.contains(compared), "no `{compared}` in: {kotlin}");
    }
}

/// A member named after the list the method fills is read through `this`, and one named after
/// the method builds beside it.
#[test]
fn a_member_named_after_what_the_method_writes_is_still_read() {
    let item: ItemStruct = syn::parse_quote! {
        struct Probe {
            #[model_schema_prop(minLength = 1)]
            validate: String,
            #[model_schema_prop(minimum = 1)]
            violations: i32,
        }
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.contains("data class Probe(val validate: String, val violations: @Serializable(with = KotlinNumber1::class) Int) { fun validate(): List<String> {"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("if (validate.codePointCount(0, validate.length) < 1) violations.add("),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("if (this.violations < 1) violations.add(\"'violations': too small: minimum is 1, got ${this.violations}\")"),
        "got: {kotlin}"
    );
}

/// A map's key typed with a parameter is written into a place where the type declares
/// `Serialize` for it, and left out where it declares none, as Rust writes it.
#[test]
fn a_key_typed_with_a_parameter_is_written_where_the_type_declares_serialize() {
    let keyed: ItemStruct = syn::parse_quote! {
        struct Rack<Key: Ord + Serialize> {
            by_key: BTreeMap<Key, Parcel>,
        }
    };
    let written_under_its_key = plain(&keyed);
    assert!(
        written_under_its_key.contains(
            "for ((key, held) in byKey) { violations.addAll(held.validate().map { \
             kotlinSchemaUnder(\"by_key.${kotlinSchemaKey(key, keySerializer)}\", it) }) }"
        ),
        "got: {written_under_its_key}"
    );
    assert!(
        written_under_its_key.contains(
            "fun validate(): List<String> = validate(null); \
             fun validate(keySerializer: KSerializer<Key>?): List<String>"
        ),
        "got: {written_under_its_key}"
    );
    let unkeyed: ItemStruct = syn::parse_quote! {
        struct Shelf<Key> where Key: Ord {
            by_key: BTreeMap<Key, Parcel>,
        }
    };
    let written_under_the_field = plain(&unkeyed);
    assert!(
        written_under_the_field.contains(
            "for ((_, held) in byKey) { violations.addAll(held.validate().map { \
             kotlinSchemaUnder(\"by_key\", it) }) }"
        ),
        "got: {written_under_the_field}"
    );
}

/// A key that is no string is written as its serializer writes it: a plain enum under the name
/// serde writes the variant.
#[test]
fn a_key_of_a_declared_type_is_written_as_its_serializer_writes_it() {
    let item: ItemStruct = syn::parse_quote! {
        struct Docks {
            by_number: BTreeMap<u32, Parcel>,
            by_wing: BTreeMap<Wing, Parcel>,
        }
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.contains("kotlinSchemaUnder(\"by_number.$key\", it)"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "kotlinSchemaUnder(\"by_wing.${Json.encodeToJsonElement(serializer<Wing>(), \
             key).jsonPrimitive.content}\", it)"
        ),
        "got: {kotlin}"
    );
}

/// A brand that keys a map writes the serializer of a map keyed by it, and a member of such a map
/// names it on the map: a value is refused under the key's text, which kotlinx would state as the
/// brand's `toString()`. The key keeps the serializer it is read through. A brand over a text has
/// no key serializer, so the map's serializer judges its key under the key's text.
#[cfg(feature = "serde")]
#[test]
fn a_map_keyed_by_a_brand_is_read_through_the_brands_map_serializer() {
    let pitch: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Pitch(u32);
    };
    let tone: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Tone(String);
    };
    let held: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Held(Vec<String>);
    };
    let pitched = plain(&pitch);
    assert!(pitched.ends_with(&mapped("Pitch")), "got: {pitched}");
    let toned = plain(&tone);
    assert!(toned.ends_with(&mapped_judging("Tone")), "got: {toned}");
    assert!(
        !plain(&held).contains("MapSerializer"),
        "a list keys no map"
    );
    let item: ItemStruct = syn::parse_quote! {
        struct Chime {
            pitches: BTreeMap<Pitch, String>,
            tones: BTreeMap<Tone, String>,
            names: BTreeMap<String, String>,
        }
    };
    let kotlin = plain(&item);
    for declared in [
        "val pitches: @Serializable(with = PitchMapSerializer::class) Map<@Serializable(with = \
         PitchKeySerializer::class) Pitch, String>",
        "val tones: @Serializable(with = ToneMapSerializer::class) Map<Tone, String>",
        "val names: Map<String, String>",
    ] {
        assert!(kotlin.contains(declared), "{declared} is not in: {kotlin}");
    }
}

/// A brand over a brand over a text writes the serializer of a map keyed by it, which places the
/// refusal of the brand it holds under the key's text, as it places its own.
#[cfg(feature = "serde")]
#[test]
fn a_map_keyed_by_a_brand_over_a_text_brand_places_the_inner_refusal_under_the_key() {
    let tone: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Tone(String);
    };
    let outer: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Outer(Tone);
    };
    let toned = plain(&tone);
    assert!(toned.ends_with(&mapped_judging("Tone")), "got: {toned}");
    let nested = plain(&outer);
    assert!(
        nested.ends_with(&mapped_over_a_brand("Outer")),
        "got: {nested}"
    );
    let item: ItemStruct = syn::parse_quote! {
        struct Chord {
            outer: BTreeMap<Outer, String>,
        }
    };
    let kotlin = plain(&item);
    let declared = "val outer: @Serializable(with = OuterMapSerializer::class) Map<Outer, String>";
    assert!(kotlin.contains(declared), "{declared} is not in: {kotlin}");
}

/// Only a brand that holds a string, or a brand or an alias that answers one, answers its text.
#[cfg(feature = "serde")]
#[test]
fn a_brand_over_anything_but_a_text_answers_none() {
    let text: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct TextBrand(String);
    };
    let over_text: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct OverTextBrand(TextBrand);
    };
    let record: ItemStruct = syn::parse_quote! {
        struct PlainRecord { label: String }
    };
    assert!(plain(&text).contains("fun kotlinSchemaText(): String = value;"));
    assert!(plain(&record).contains("fun validate(): List<String> = emptyList()"));
    assert!(
        plain(&over_text).contains("fun kotlinSchemaText(): String = value.kotlinSchemaText();"),
        "a brand over a brand written above answers through it"
    );
    for silent in [
        syn::parse_quote! { #[serde(transparent)] struct NumberBrand(u32); },
        syn::parse_quote! { #[serde(transparent)] struct ListBrand(Vec<String>); },
        syn::parse_quote! { #[serde(transparent)] struct RecordBrand(PlainRecord); },
        syn::parse_quote! { #[serde(transparent)] struct UnseenBrand(NotWrittenYet); },
        syn::parse_quote! { #[serde(transparent)] struct ParameterBrand<Held>(Held); },
    ] {
        let kotlin = plain(&silent);
        assert!(!kotlin.contains("kotlinSchemaText"), "got: {kotlin}");
    }
}

/// A brand that holds one of its parameters bare runs its bounds where the value is of the type
/// declared for the parameter, and writes none where that type is written with arguments.
#[cfg(feature = "serde")]
#[test]
fn a_brand_with_a_parameter_is_bounded_at_its_declared_default() {
    let item: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct UserId<IdType>(IdType);
    };
    let bounded = |default_types: &[(Ident, Type)]| {
        definition(struct_kotlin_tokens(
            &item,
            &KotlinArgs {
                default_types,
                min_length: Some(3),
                ..plain_args()
            },
        ))
    };
    let parameter = Ident::new("IdType", Span::call_site());
    let held = "{ val violations = mutableListOf<String>(); if (value is String) { if \
                (value.codePointCount(0, value.length) < 3) violations.add(\"too short: minimum \
                length is 3, got ${value.codePointCount(0, value.length)}\") }; return violations }";
    assert!(
        bounded(&[]).contains(&format!(
            "{{ fun validate(): List<String> {held}; fun kotlinSchemaBound(): List<String> {held} }}"
        )),
        "a parameter with no declared type is bounded as a string"
    );
    let branded = bounded(&[(parameter.clone(), syn::parse_quote!(Slug))]);
    assert!(
        branded.contains("if (value is Slug) { if (value.kotlinSchemaText().codePointCount(0, "),
        "got: {branded}"
    );
    let filled = bounded(&[(parameter, syn::parse_quote!(Other<String>))]);
    assert!(
        filled.contains("{ fun validate(): List<String> = emptyList() }"),
        "got: {filled}"
    );
}

/// A type's own check is called on the whole value, on a plain enum and on a unit struct as well.
#[test]
fn a_types_own_check_is_called_on_the_whole_value() {
    let check = [FunctionName {
        name: "rated".to_owned(),
        written_at: Span::call_site(),
    }];
    let args = KotlinArgs {
        check: &check,
        ..plain_args()
    };
    let called = "fun validate(): List<String> { val violations = mutableListOf<String>(); \
                  violations.addAll(rated(this)); return violations }";
    let graded: ItemEnum = syn::parse_quote! {
        enum Grade { Express, Unrated }
    };
    assert_eq!(
        definition(enum_kotlin_tokens(&graded, &args)),
        format!(
            "{} enum class Grade {{ Express, Unrated; {called} }} {} {}",
            kept("Grade"),
            generated("Grade"),
            judged("Grade")
        )
    );
    let closed: ItemStruct = syn::parse_quote! {
        struct ClosedDock;
    };
    assert_eq!(
        definition(struct_kotlin_tokens(&closed, &args)),
        format!(
            "{} object ClosedDock {{ {called} }} {} {}",
            kept("ClosedDock"),
            generated("ClosedDock"),
            judged("ClosedDock")
        )
    );
}

/// The README shows the parcel and the manifest of its checks as the emitter writes them, each
/// line break of the block standing for one space.
#[cfg(feature = "serde")]
#[test]
fn the_readme_shows_what_the_parcel_of_the_documents_emits() {
    let within = [FunctionName {
        name: "withinParcel".to_owned(),
        written_at: Span::call_site(),
    }];
    let parcel: ItemStruct = syn::parse_quote! {
        #[serde(rename_all = "camelCase")]
        pub struct Parcel {
            #[model_schema_prop(minLength = 1)]
            pub label: String,
            #[model_schema_prop(check = ["withoutDuplicates"])]
            pub item_code_list: Vec<String>,
            pub fragile_item_codes: Vec<String>,
        }
    };
    let manifest: ItemStruct = syn::parse_quote! {
        pub struct Manifest {
            pub parcel: Parcel,
        }
    };
    let checked = KotlinArgs {
        check: &within,
        ..plain_args()
    };
    let emitted = [
        definition(struct_kotlin_tokens(&parcel, &checked)),
        plain(&manifest),
    ];
    let readme = include_str!("../../../docs/guide/kotlin.md");
    let opens = "```kotlin\n@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
                 ParcelSerializer::class)";
    let block = readme
        .find(opens)
        .and_then(|start| readme.get(start + "```kotlin\n".len()..))
        .and_then(|rest| rest.split_once("\n```"))
        .unwrap()
        .0;
    let shown: Vec<String> = block
        .split("\n\n")
        .map(|declaration| {
            declaration
                .lines()
                .map(str::trim_start)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    assert_eq!(shown, emitted);
}

/// A type with a parameter writes both serializers as classes over one serializer per parameter,
/// and the plugin's serializer is the one the unjudged read hands on.
#[test]
fn a_type_with_a_parameter_takes_one_serializer_per_parameter() {
    let item: ItemStruct = syn::parse_quote! {
        struct Crate<CodeType> {
            codes: Vec<CodeType>,
            #[model_schema_prop(minLength = 1)]
            label: String,
        }
    };
    assert_eq!(
        plain(&item),
        format!(
            "{} data class Crate<CodeType>(val codes: List<CodeType>, val label: String) {{ fun \
             validate(): List<String> {{ val violations = mutableListOf<String>(); if \
             (label.codePointCount(0, label.length) < 1) violations.add(\"'label': too short: \
             minimum length is 1, got ${{label.codePointCount(0, label.length)}}\"); return \
             violations }} }} @OptIn(ExperimentalSerializationApi::class) class \
             CrateUnjudged<CodeType>(codeTypeSerializer: KSerializer<CodeType>) : \
             KSerializer<Crate<CodeType>> by Crate.generatedSerializer(codeTypeSerializer) {}",
            kept("Crate"),
            judged_over("Crate", "CodeType", "", "codeTypeSerializer")
        )
    );
}

/// A holder names the unjudged serializer of each model `validate()` walks, where it is written:
/// on the property, and on the type inside a list, a nullable and a map's values.
#[test]
fn a_holder_reads_each_model_it_walks_with_no_judgement() {
    let crated: ItemStruct = syn::parse_quote! {
        struct Crate<CodeType> { codes: Vec<CodeType> }
    };
    let coded: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Code(String);
    };
    let holder: ItemStruct = syn::parse_quote! {
        struct Depot {
            by_dock: BTreeMap<String, Parcel>,
            code: Code,
            crated: Crate<Parcel>,
            first: Parcel,
            grid: Vec<Vec<Option<Parcel>>>,
            listed: Vec<Parcel>,
            lists_by_dock: BTreeMap<String, Vec<Parcel>>,
            pair: (String, Parcel),
            spare: Option<Parcel>,
            #[model_schema_prop(minLength = 3)]
            stamped: Code,
            trailing: Trailer,
        }
    };
    let written = [labelled_parcel(), plain(&crated), plain(&coded)];
    assert_eq!(written.len(), 3);
    let kotlin = plain(&holder);
    let unjudged = "@Serializable(with = ParcelUnjudged::class)";
    assert!(
        kotlin.contains(&format!(
            "data class Depot(@SerialName(\"by_dock\") val byDock: Map<String, {unjudged} \
             Parcel>, @Serializable(with = CodeUnjudged::class) val code: Code, \
             @Serializable(with = CrateUnjudged::class) val crated: Crate<Parcel>, {unjudged} val \
             first: Parcel, val grid: List<List<{unjudged} Parcel?>>, val listed: \
             List<{unjudged} Parcel>, @SerialName(\"lists_by_dock\") val listsByDock: \
             Map<String, List<{unjudged} Parcel>>, val pair: KotlinTuple1, {unjudged} val spare: \
             Parcel? = null, @Serializable(with = CodeUnjudged::class) val stamped: Code, val trailing: \
             Trailer) {{"
        )),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "KotlinTuple1(kotlinSchemaDecoded(input.json, serializer<String>(), array[0], \
             \"0\", \"[0]\"), kotlinSchemaDecoded(input.json, ParcelUnjudged, array[1], \"1\", \
             \"[1]\"))"
        ),
        "the wrapper of a tuple the holder walks reads a model in a position unjudged. got: \
         {kotlin}"
    );
    assert!(
        kotlin.ends_with(&format!("{} {}", generated("Depot"), judged("Depot"))),
        "got: {kotlin}"
    );
}

/// A brand's value class keeps the plugin's serializer, and answers what its bounds alone hold
/// the value to beside `validate()`, which calls its checks as well. A map keyed by it judges each
/// key by `validate()` under the key's text.
#[cfg(feature = "serde")]
#[test]
fn a_brand_is_judged_by_its_own_serializer_and_answers_its_bounds_alone() {
    let item: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Code(String);
    };
    let check = [FunctionName {
        name: "unreserved".to_owned(),
        written_at: Span::call_site(),
    }];
    let args = KotlinArgs {
        check: &check,
        min_length: Some(2),
        ..plain_args()
    };
    let bound = "if (value.codePointCount(0, value.length) < 2) violations.add(\"too short: \
                 minimum length is 2, got ${value.codePointCount(0, value.length)}\")";
    assert_eq!(
        definition(struct_kotlin_tokens(&item, &args)),
        format!(
            "@OptIn(ExperimentalSerializationApi::class) @JvmInline @Serializable(with = \
             CodeSerializer::class) @KeepGeneratedSerializer value class Code(val value: String) \
             {{ fun kotlinSchemaText(): String = value; fun kotlinSchemaLength(): Int = \
             value.codePointCount(0, value.length); fun validate(): List<String> {{ val \
             violations = mutableListOf<String>(); {bound}; violations.addAll(unreserved(this)); \
             return violations }}; fun kotlinSchemaBound(): List<String> {{ val violations = \
             mutableListOf<String>(); {bound}; return violations }} }} {} {} {}",
            generated("Code"),
            judged("Code"),
            mapped_judging("Code")
        )
    );
    let unbounded = plain(&item);
    assert!(!unbounded.contains("kotlinSchemaBound"), "got: {unbounded}");
}

/// A brand with a parameter writes its serializers over the parameter's own.
#[cfg(feature = "serde")]
#[test]
fn a_brand_with_a_parameter_takes_the_serializer_of_what_it_holds() {
    let item: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct UserId<IdType>(IdType);
    };
    let kotlin = plain(&item);
    assert_eq!(
        kotlin,
        format!(
            "@OptIn(ExperimentalSerializationApi::class) @JvmInline @Serializable(with = \
             UserIdSerializer::class) @KeepGeneratedSerializer value class UserId<IdType>(val \
             value: IdType) {{ fun validate(): List<String> = emptyList() }} \
             @OptIn(ExperimentalSerializationApi::class) class \
             UserIdUnjudged<IdType>(idTypeSerializer: KSerializer<IdType>) : \
             KSerializer<UserId<IdType>> by UserId.generatedSerializer(idTypeSerializer) {}",
            judged_over("UserId", "IdType", "", "idTypeSerializer")
        )
    );
}

/// The serializer the emitter writes for a tuple struct is its unjudged read, a model in a slot
/// read unjudged there, and the struct's own serializer judges what it read.
#[test]
fn a_tuple_struct_reads_its_slots_unjudged_and_judges_the_whole() {
    let item: ItemStruct = syn::parse_quote! {
        struct Consignment(Vec<String>, Parcel);
    };
    let parcel = labelled_parcel();
    assert!(parcel.ends_with(&judged("Parcel")), "got: {parcel}");
    assert_eq!(
        plain(&item),
        format!(
            "@Serializable(with = ConsignmentSerializer::class) data class Consignment(val slot0: \
             List<String>, val slot1: Parcel) {{ fun validate(): List<String> {{ val violations = \
             mutableListOf<String>(); violations.addAll(slot1.validate().map {{ \
             kotlinSchemaUnder(\"1\", it) }}); return violations }}; {UNDER} }} object \
             ConsignmentUnjudged : KSerializer<Consignment> {{ override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"Consignment\"); override fun \
             serialize(encoder: Encoder, value: Consignment) {{ val output = encoder as \
             JsonEncoder; output.encodeJsonElement(buildJsonArray {{ \
             add(output.json.encodeToJsonElement(serializer<List<String>>(), value.slot0)); \
             add(output.json.encodeToJsonElement(ParcelUnjudged, value.slot1)) }}) }}; \
             {DECODED}{PATHED}override fun deserialize(decoder: Decoder): Consignment {{ val \
             input = decoder as JsonDecoder; val element = input.decodeJsonElement(); return \
             kotlinSchemaAt(input) {{ val array = element.let {{ it as? JsonArray ?: throw \
             SerializationException(\"expected an array, found \" + it) }}; if (array.size != 2) \
             throw SerializationException(\"invalid length \" + array.size + \", expected 2\"); \
             Consignment(kotlinSchemaDecoded(input.json, serializer<List<String>>(), array[0], \
             \"0\", \"[0]\"), kotlinSchemaDecoded(input.json, ParcelUnjudged, array[1], \"1\", \
             \"[1]\")) }} }} }} {}",
            judged("Consignment")
        )
    );
}

/// A tuple struct of one slot is read from the slot's value and written as it, as serde writes it,
/// whatever the slot's attributes say: the two serializers are written over that read, and
/// `validate()` names the slot by nothing.
#[test]
fn a_tuple_struct_of_one_slot_is_the_value_of_its_slot() {
    let item: ItemStruct = syn::parse_quote! {
        struct Sealed(#[model_schema_prop(minLength = 1)] String);
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.contains(&format!(
            "object SealedUnjudged : KSerializer<Sealed> {{ override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"Sealed\"); override fun \
             serialize(encoder: Encoder, value: Sealed) {{ val output = encoder as JsonEncoder; \
             output.encodeJsonElement(output.json.encodeToJsonElement(serializer<String>(), \
             value.slot0)) }}; {DECODED}{PATHED}override fun deserialize(decoder: Decoder): \
             Sealed {{ val input = decoder as JsonDecoder; val element = \
             input.decodeJsonElement(); return kotlinSchemaAt(input) {{ \
             Sealed(kotlinSchemaDecoded(input.json, serializer<String>(), element, \"0\", \"\")) \
             }} }} }}"
        )),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "violations.add(\"too short: minimum length is 1, got ${slot0.codePointCount(0, \
             slot0.length)}\")"
        ),
        "got: {kotlin}"
    );
    assert!(kotlin.ends_with(&judged("Sealed")), "got: {kotlin}");
    let skipped: ItemStruct = syn::parse_quote! {
        struct Unsealed(#[serde(skip)] String);
    };
    let unsealed = plain(&skipped);
    assert!(
        unsealed.contains(
            "data class Unsealed(val slot0: String) { fun validate(): List<String> = emptyList() \
             }"
        ),
        "got: {unsealed}"
    );
    assert!(
        unsealed.contains(
            "val element = input.decodeJsonElement(); return kotlinSchemaAt(input) { \
             Unsealed(kotlinSchemaDecoded(input.json, serializer<String>(), element, \"0\", \
             \"\")) }"
        ),
        "got: {unsealed}"
    );
}

/// The one slot serde writes of a struct that declares two is the array of that slot, and
/// `validate()` names it by its position there, as Rust's names it.
#[cfg(feature = "serde")]
#[test]
fn the_one_slot_left_of_two_is_named_by_its_position() {
    let item: ItemStruct = syn::parse_quote! {
        struct Clasped(#[serde(skip)] u8, #[model_schema_prop(minLength = 1)] String);
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.contains(
            "violations.add(\"'0': too short: minimum length is 1, got \
             ${slot0.codePointCount(0, slot0.length)}\")"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "output.encodeJsonElement(buildJsonArray { \
             add(output.json.encodeToJsonElement(serializer<String>(), value.slot0)) })"
        ),
        "got: {kotlin}"
    );
    let variant: ItemEnum = syn::parse_quote! {
        enum Clasping {
            Held(#[serde(skip)] u8, #[model_schema_prop(minLength = 1)] String),
        }
    };
    let clasping = definition(enum_kotlin_tokens(&variant, &plain_args()));
    assert!(
        clasping.contains(
            "violations.add(\"'Held.0': too short: minimum length is 1, got \
             ${value.slot0.codePointCount(0, value.slot0.length)}\")"
        ),
        "got: {clasping}"
    );
}

/// A struct with a flattened member reads the model it merges, and the values of the map it
/// merges, unjudged, and its own serializer judges the whole.
#[cfg(feature = "serde")]
#[test]
fn a_struct_with_a_flattened_member_reads_what_it_merges_unjudged() {
    let base: ItemStruct = syn::parse_quote! {
        struct Base { name: String }
    };
    let item: ItemStruct = syn::parse_quote! {
        struct Yard {
            #[serde(flatten)]
            base: Base,
            gate: String,
        }
    };
    let docks: ItemStruct = syn::parse_quote! {
        struct Docks {
            #[serde(flatten)]
            by_dock: BTreeMap<String, Parcel>,
        }
    };
    let written = [labelled_parcel(), plain(&base)];
    assert_eq!(written.len(), 2);
    let kotlin = plain(&item);
    for held in [
        "@Serializable(with = YardSerializer::class) data class Yard(val base: Base, val gate: \
         String) {",
        "} object YardUnjudged : KSerializer<Yard> { override val descriptor: SerialDescriptor = \
         buildClassSerialDescriptor(\"Yard\"); override fun serialize(",
        "output.json.encodeToJsonElement(BaseUnjudged, value.base).jsonObject.forEach { (k, v) \
         -> put(k, v) }",
        "val baseKeys = setOf(\"name\"); val base = kotlinSchemaFlattened { \
         input.json.decodeFromString(BaseUnjudged, JsonObject(obj.filterKeys { it in \
         setOf(\"name\") }).toString()) }",
    ] {
        assert!(kotlin.contains(held), "no `{held}` in: {kotlin}");
    }
    assert!(kotlin.ends_with(&judged("Yard")), "got: {kotlin}");
    let map = plain(&docks);
    for held in [
        "@Serializable(with = DocksSerializer::class) data class Docks(@SerialName(\"by_dock\") \
         val byDock: Map<String, Parcel>) {",
        "value.byDock.forEach { (k, v) -> put(k, output.json.encodeToJsonElement(ParcelUnjudged, \
         v)) }",
        "val byDock = kotlinSchemaFlattened { obj.mapValues { (k, v) -> \
         kotlinSchemaDecoded(input.json, ParcelUnjudged, v, k, \".\" + k) } }",
    ] {
        assert!(map.contains(held), "no `{held}` in: {map}");
    }
    assert!(map.ends_with(&judged("Docks")), "got: {map}");
}

/// A flattened struct the emitter wrote takes the keys serde reads it from: each name and alias of
/// a member it reads, and neither its own tag nor a member serde never reads. One it has not
/// written takes the `elementNames` of its descriptor.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_struct_takes_the_keys_serde_reads_where_the_emitter_wrote_it() {
    let pennant: ItemStruct = syn::parse_quote! {
        #[serde(tag = "kind")]
        struct Pennant {
            #[serde(alias = "name")]
            label: String,
            #[serde(skip_deserializing)]
            spare: u8,
        }
    };
    let item: ItemStruct = syn::parse_quote! {
        struct Hoist {
            #[serde(flatten)]
            pennant: Pennant,
            #[serde(flatten)]
            unseen: Unseen,
        }
    };
    let written = plain(&pennant);
    assert!(written.ends_with(&judged("Pennant")), "got: {written}");
    let kotlin = plain(&item);
    for held in [
        "val pennantKeys = setOf(\"label\", \"name\"); val pennant = kotlinSchemaFlattened { \
         input.json.decodeFromString(PennantUnjudged, JsonObject(obj.filterKeys { it in \
         setOf(\"label\", \"name\", \"spare\", \"kind\") }).toString()) };",
        "val unseenKeys = (serializer<Unseen>()).descriptor.elementNames.toSet(); val unseen = \
         kotlinSchemaFlattened { lenient.decodeFromString(serializer<Unseen>(), \
         kotlinSchemaFlat(serializer<Unseen>(), JsonObject(obj - pennantKeys)).toString()) };",
    ] {
        assert!(kotlin.contains(held), "no `{held}` in: {kotlin}");
    }
}

/// A struct the emitter writes the serializer of refuses a key no member reads, under a `Json`
/// that does not ignore unknown keys: the names of its own members and its own tag, every key a
/// flattened struct the emitter wrote reads, an alias, a member serde never reads and its tag among
/// them, the entry a flattened externally tagged enum took, and the keys a type the emitter has
/// not written publishes, through a placeholder. Beside a flattened map or a struct with a
/// flattened member of its own, no key is refused.
#[cfg(feature = "serde")]
#[test]
fn a_key_no_member_reads_is_refused_where_every_key_a_member_reads_is_known() {
    let refused = |read: &str, named: &str| {
        format!(
            "if (!input.json.configuration.ignoreUnknownKeys) (obj{read}).keys.firstOrNull()?.let \
             {{ throw SerializationException(\"Encountered an unknown key '\" + it + \"'\") }}; \
             {named}("
        )
    };
    let burgee: ItemStruct = syn::parse_quote! {
        #[serde(tag = "flag")]
        struct Burgee {
            label: String,
            #[serde(default, skip_serializing)]
            spare: Option<u32>,
        }
    };
    let ensign: ItemStruct = syn::parse_quote! {
        #[serde(tag = "kind")]
        struct Ensign {
            #[serde(alias = "name")]
            label: String,
            #[serde(skip_deserializing)]
            spare: u8,
        }
    };
    let tack: ItemEnum = syn::parse_quote! {
        enum Tack { Bay { label: String }, Calm }
    };
    let dipped: ItemStruct = syn::parse_quote! {
        struct Dipped {
            #[serde(flatten)]
            ensign: Ensign,
            #[serde(flatten)]
            tack: Tack,
            note: String,
        }
    };
    let written_burgee = plain(&burgee);
    assert!(
        written_burgee.contains(&refused(
            " - setOf(\"label\", \"spare\") - \"flag\"",
            "Burgee"
        )),
        "got: {written_burgee}"
    );
    let written = [
        plain(&ensign),
        definition(enum_kotlin_tokens(&tack, &plain_args())),
    ];
    assert_eq!(written.len(), 2, "each is written ahead of its holder");
    let written_dipped = plain(&dipped);
    assert!(
        written_dipped.contains(&refused(
            " - setOf(\"note\") - setOf(\"label\", \"name\", \"spare\", \"kind\") - \
             tackEntry.key",
            "Dipped"
        )),
        "got: {written_dipped}"
    );
    let hoisted: ItemStruct = syn::parse_quote! {
        struct Hoisted {
            #[serde(flatten)]
            unseen: Unseen,
            note: String,
        }
    };
    let written_hoisted = plain(&hoisted);
    assert!(
        written_hoisted.contains("Encountered an unknown key")
            && written_hoisted.contains("tixschema-keys-0"),
        "a member the emitter has not written is subtracted through the keys its type publishes, \
         got: {written_hoisted}"
    );
    let any_key: [ItemStruct; 2] = [
        syn::parse_quote! {
            struct Counted {
                #[serde(flatten)]
                counts: BTreeMap<String, u8>,
            }
        },
        syn::parse_quote! {
            struct Rigged {
                #[serde(flatten)]
                dipped: Dipped,
                note: String,
            }
        },
    ];
    for item in &any_key {
        let kotlin = plain(item);
        assert!(
            !kotlin.contains("Encountered an unknown key"),
            "got: {kotlin}"
        );
    }
}

/// A flattened `Option` of an enum under a tag key the emitter wrote is looked for by its tag, then
/// its content key, in the whole object its holder reads, as TypeScript and Zod read it. One under
/// a tag alone takes the keys its descriptor names, and one under a tag and a content key takes
/// both. One that names a function on itself has no key to be looked for by, and is `null`
/// wherever its read throws.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_option_of_an_enum_under_a_tag_key_is_looked_for_by_its_tag() {
    let hank: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind")]
        enum Hank { Calm, Round { turns: u32 } }
    };
    let clew: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind", content = "body")]
        enum Clew { Calm, Round { turns: u32 } }
    };
    let brail: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind")]
        enum Brail { Calm, Round { turns: u32 } }
    };
    let item: ItemStruct = syn::parse_quote! {
        struct Rigging {
            #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
            hank: Option<Hank>,
            #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
            clew: Option<Clew>,
            #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
            brail: Option<Brail>,
            note: String,
        }
    };
    let functions = [FunctionName {
        name: "asGiven".to_owned(),
        written_at: Span::call_site(),
    }];
    let written = [
        definition(enum_kotlin_tokens(&hank, &plain_args())),
        definition(enum_kotlin_tokens(&clew, &plain_args())),
        definition(enum_kotlin_tokens(
            &brail,
            &KotlinArgs {
                preprocess: &functions,
                type_name: "Probe",
                ..plain_args()
            },
        )),
    ];
    assert_eq!(written.len(), 3, "each enum is written ahead of its holder");
    let kotlin = plain(&item);
    let decoded = |serializer: &str, from: &str| {
        format!("lenient.decodeFromString({serializer}, {from}.toString())")
    };
    let after_clew = "JsonObject(obj - setOf(\"note\") - hankKeys - clewKeys)";
    for held in [
        format!(
            "val hankKeys = (HankUnjudged).descriptor.elementNames.toSet(); val hank = \
             kotlinSchemaFlattened {{ if (obj.keys.any {{ it in setOf(\"kind\") }}) {} else null \
             }};",
            decoded("HankUnjudged", "obj")
        ),
        format!(
            "val clewKeys = setOf(\"kind\", \"body\"); val clew = kotlinSchemaFlattened {{ if \
             (obj.keys.any {{ it in setOf(\"kind\", \"body\") }}) {} else null }};",
            decoded("ClewUnjudged", "obj")
        ),
        format!(
            "val brailKeys = (BrailUnjudged).descriptor.elementNames.toSet(); val brail = \
             kotlinSchemaFlattened {{ if (brailKeys.isEmpty()) runCatching {{ {} }}.getOrNull() \
             else if ({after_clew}.keys.any {{ it in brailKeys }}) {} else null }};",
            decoded("BrailUnjudged", after_clew),
            decoded("BrailUnjudged", after_clew)
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
}

/// An enum under a tag alone is read by a serializer the emitter writes: it takes the tag off
/// the object and reads the rest as the variant, and writes the variant with the tag ahead of it.
/// A newtype variant reads the rest as the value it holds, and writes that value's members beside
/// the tag, as serde writes them.
#[cfg(feature = "serde")]
#[test]
fn an_enum_under_a_tag_alone_is_read_and_written_by_hand_over_its_variants() {
    let item: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind")]
        enum Lift {
            Boxed(Parcel),
            Held,
            Loose {
                #[model_schema_prop(maxLength = 3)]
                label: String,
            },
        }
    };
    let parcel = labelled_parcel();
    assert!(parcel.ends_with(&judged("Parcel")), "got: {parcel}");
    let merged = |variant: &str| {
        format!(
            "is Lift{variant} -> buildJsonObject {{ put(\"kind\", \"{variant}\"); \
             output.json.encodeToJsonElement(serializer<Lift{variant}>(), \
             value).jsonObject.forEach {{ (k, v) -> put(k, v) }} }}"
        )
    };
    let rest = |variant: &str| {
        format!(
            "\"{variant}\" -> kotlinSchemaInner(input.json, \"\", serializer<Lift{variant}>(), \
             JsonObject(obj - \"kind\"))"
        )
    };
    assert_eq!(
        definition(enum_kotlin_tokens(&item, &plain_args())),
        format!(
            "@Serializable(with = LiftSerializer::class) sealed interface Lift {{ fun validate(): \
             List<String> }} @SerialName(\"Boxed\") @Serializable data class \
             LiftBoxed(@Serializable(with = ParcelUnjudged::class) val value: Parcel) : Lift {{ \
             override fun validate(): List<String> {{ val violations = mutableListOf<String>(); \
             violations.addAll(value.validate()); return violations }} }} @SerialName(\"Held\") \
             @Serializable data object LiftHeld : Lift {{ override fun validate(): List<String> = \
             emptyList() }} @SerialName(\"Loose\") @Serializable data class LiftLoose(val label: \
             String) : Lift {{ override fun validate(): List<String> {{ val violations = \
             mutableListOf<String>(); if \
             (label.codePointCount(0, label.length) > 3) violations.add(\"'label': too long: \
             maximum length is 3, got ${{label.codePointCount(0, label.length)}}\"); return \
             violations }} }} object LiftUnjudged : KSerializer<Lift> {{ override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"Lift\"); override fun \
             serialize(encoder: Encoder, value: Lift) {{ val output = encoder as JsonEncoder; \
             output.encodeJsonElement(when (value) {{ {}; is LiftHeld -> buildJsonObject {{ \
             put(\"kind\", \"Held\") }}; {} }}) }}; {DECODED}{PATHED}{INNER}override fun \
             deserialize(decoder: Decoder): Lift {{ val input = decoder as JsonDecoder; val \
             element = input.decodeJsonElement(); return kotlinSchemaAt(input) {{ val obj = \
             element.let {{ it as? JsonObject ?: throw SerializationException(\"expected an \
             object, found \" + it) }}; val tag = ((obj[\"kind\"] ?: throw \
             SerializationException(\"missing field `kind`\")) as? JsonPrimitive)?.content; when \
             (tag) {{ {}; \"Held\" -> {{ if (!input.json.configuration.ignoreUnknownKeys) (obj \
             - \"kind\").keys.firstOrNull()?.let {{ throw SerializationException(\"Encountered an \
             unknown key '\" + it + \"'\") }}; LiftHeld }}; {} else -> throw \
             SerializationException(\"unknown tag \" + tag) }} }} }} }} {}",
            "is LiftBoxed -> buildJsonObject { put(\"kind\", \"Boxed\"); \
             output.json.encodeToJsonElement(ParcelUnjudged, value.value).jsonObject.forEach { \
             (k, v) -> put(k, v) } }",
            merged("Loose"),
            "\"Boxed\" -> LiftBoxed(kotlinSchemaDecoded(input.json, ParcelUnjudged, \
             JsonObject(obj - \"kind\"), \"Boxed\", \"\"))",
            rest("Loose"),
            judged("Lift")
        )
    );
}

/// An enum under serde's own tagging reads the model a variant holds unjudged, in its one slot and
/// in a list there, and its own serializer judges the variant it read.
#[test]
fn an_enum_under_its_variants_names_reads_what_a_variant_holds_unjudged() {
    let item: ItemEnum = syn::parse_quote! {
        enum Freight {
            Boxed(Parcel),
            Held,
            Many(Vec<Parcel>),
        }
    };
    let parcel = labelled_parcel();
    assert!(parcel.ends_with(&judged("Parcel")), "got: {parcel}");
    let kotlin = definition(enum_kotlin_tokens(&item, &plain_args()));
    for held in [
        "@Serializable(with = FreightSerializer::class) sealed interface Freight { fun \
         validate(): List<String> } @Serializable data class FreightBoxed(@Serializable(with = \
         ParcelUnjudged::class) val value: Parcel) : Freight {",
        "@Serializable data class FreightMany(val value: List<@Serializable(with = \
         ParcelUnjudged::class) Parcel>) : Freight {",
        "} object FreightUnjudged : KSerializer<Freight> { override val descriptor: \
         SerialDescriptor = buildClassSerialDescriptor(\"Freight\"); override fun \
         serialize(encoder: Encoder, value: Freight) { val output = encoder as JsonEncoder; \
         output.encodeJsonElement(when (value) { is FreightBoxed -> buildJsonObject { \
         put(\"Boxed\", output.json.encodeToJsonElement(ParcelUnjudged, value.value)) }; is \
         FreightHeld -> JsonPrimitive(\"Held\"); is FreightMany -> buildJsonObject { \
         put(\"Many\", output.json.encodeToJsonElement(ListSerializer(ParcelUnjudged), \
         value.value)) } }) };",
        "override fun deserialize(decoder: Decoder): Freight { val input = decoder as \
         JsonDecoder; val element = input.decodeJsonElement(); return kotlinSchemaAt(input) { val \
         (tag, data) = if (element is JsonPrimitive && element.isString) element.content to \
         JsonNull else (element as? JsonObject)?.entries?.singleOrNull()?.let { it.key to \
         it.value } ?: throw SerializationException(\"expected value\"); ",
        "when (tag) { \"Boxed\" -> kotlinSchemaUnder(\".\" + tag) { \
         FreightBoxed(kotlinSchemaDecoded(input.json, ParcelUnjudged, data, \"Boxed\", \"\")) }; \
         \"Held\" -> kotlinSchemaUnder(\".\" + tag) { if (data is JsonNull) FreightHeld else \
         throw SerializationException(\"Freight: \" + tag + \" takes no value\") }; \"Many\" -> \
         kotlinSchemaUnder(\".\" + tag) { FreightMany(kotlinSchemaDecoded(input.json, \
         ListSerializer(ParcelUnjudged), data, \"Many\", \"\")) } else -> throw \
         SerializationException(\"unknown tag \" + tag) } } } }",
    ] {
        assert!(kotlin.contains(held), "no `{held}` in: {kotlin}");
    }
    assert!(kotlin.ends_with(&judged("Freight")), "got: {kotlin}");
}

/// An untagged enum takes the first variant whose payload decodes and whose bounds hold: each
/// member's own, and those of each brand it holds. A check and what a held model reports choose
/// none, so a variant that holds no bound is taken as it decodes.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_enum_takes_the_first_variant_whose_bounds_hold() {
    let coded: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Code(String);
    };
    let item: ItemEnum = syn::parse_quote! {
        #[serde(untagged)]
        enum Reference {
            Boxed(Parcel),
            Coded(Code),
            Named {
                #[model_schema_prop(minLength = 1)]
                name: String,
            },
            Text(String),
            Trailing(Trailer),
        }
    };
    let bounded = KotlinArgs {
        min_length: Some(2),
        ..plain_args()
    };
    let written = [
        labelled_parcel(),
        definition(struct_kotlin_tokens(&coded, &bounded)),
    ];
    assert_eq!(written.len(), 2);
    let kotlin = definition(enum_kotlin_tokens(&item, &plain_args()));
    let opens = "val violations = mutableListOf<String>();";
    let named = "if (name.codePointCount(0, name.length) < 1) violations.add(\"'name': too \
                 short: minimum length is 1, got ${name.codePointCount(0, name.length)}\")";
    let asked = format!(
        "{{ override fun validate(): List<String> {{ {opens} violations.addAll(value.validate()); \
         return violations }}; fun kotlinSchemaBound(): List<String> {{ {opens} \
         violations.addAll(value.kotlinSchemaBound()); return violations }}"
    );
    for held in [
        format!(
            "value class ReferenceBoxed(@Serializable(with = ParcelUnjudged::class) val value: \
             Parcel) : Reference {{ override fun validate(): List<String> {{ {opens} \
             violations.addAll(value.validate()); return violations }} }} @JvmInline"
        ),
        format!(
            "value class ReferenceCoded(@Serializable(with = CodeUnjudged::class) val value: \
             Code) : Reference {asked} }} @Serializable"
        ),
        format!(
            "data class ReferenceNamed(val name: String) : Reference {{ override fun validate(): \
             List<String> {{ {opens} {named}; return violations }}; fun kotlinSchemaBound(): \
             List<String> {{ {opens} {named}; return violations }} }} @JvmInline"
        ),
        "value class ReferenceText(val value: String) : Reference { override fun validate(): \
         List<String> = emptyList() } @JvmInline"
            .to_owned(),
        format!(
            "value class ReferenceTrailing(val value: Trailer) : Reference {asked}; private fun \
             Any?.validate(): List<String> = emptyList(); private fun Any?.kotlinSchemaBound(): \
             List<String> = emptyList() }} object ReferenceUnjudged : KSerializer<Reference> {{"
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
    let taken = |variant: &str| {
        format!(
            ".recoverCatching {{ \
             input.json.decodeFromJsonElement(serializer<Reference{variant}>(), element) as \
             Reference }}"
        )
    };
    let held_to_its_bounds = |variant: &str| {
        format!(
            ".recoverCatching {{ \
             input.json.decodeFromJsonElement(serializer<Reference{variant}>(), element).also {{ \
             chosen -> if (chosen.kotlinSchemaBound().isNotEmpty()) error(\"a bound of \
             Reference{variant} is broken\") }} as Reference }}"
        )
    };
    assert!(
        kotlin.contains(&format!(
            "return kotlinSchemaAt(input) {{ runCatching {{ error(\"unreachable\") as Nothing \
             }}{}{}{}{}{}.getOrElse {{ throw SerializationException(\"No variant of Reference \
             matched\") }} }} }} }} {}",
            taken("Boxed"),
            held_to_its_bounds("Coded"),
            held_to_its_bounds("Named"),
            taken("Text"),
            held_to_its_bounds("Trailing"),
            judged("Reference")
        )),
        "got: {kotlin}"
    );
}

/// A type that holds itself reads each of its own unjudged, and judges the whole once.
#[test]
fn a_type_that_holds_itself_reads_itself_unjudged() {
    let item: ItemStruct = syn::parse_quote! {
        struct Node {
            children: Vec<Node>,
            #[model_schema_prop(minLength = 1)]
            label: String,
        }
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.contains(
            "data class Node(val children: List<@Serializable(with = NodeUnjudged::class) Node>, \
             val label: String) {"
        ),
        "got: {kotlin}"
    );
}

/// The wrapper of a tuple that reaches a type parameter of its holder, and its serializer, declare
/// that parameter, and the holder hands it: where the plugin reads the holder, through the type
/// it names, and where the emitter writes the read, through the serializer it composes.
#[test]
fn a_tuple_that_reaches_a_type_parameter_declares_it() {
    let paired: ItemStruct = syn::parse_quote! {
        struct Paired<Stowed> {
            pair: (u8, Stowed),
        }
    };
    let written = plain(&paired);
    let wrapper = written
        .split("data class ")
        .nth(1)
        .and_then(|declared| declared.split('<').next())
        .unwrap_or_default()
        .to_owned();
    assert!(wrapper.starts_with("KotlinTuple"), "got: {written}");
    for held in [
        format!("data class {wrapper}<Stowed>(val slot0: UByte, val slot1: Stowed)"),
        format!(
            "class {wrapper}Serializer<Stowed : Any>(private val stowedSerializer: \
             KSerializer<Stowed>) : KSerializer<{wrapper}<Stowed>>"
        ),
        "add(output.json.encodeToJsonElement(stowedSerializer, value.slot1))".to_owned(),
        format!("data class Paired<Stowed>(val pair: {wrapper}<Stowed>)"),
    ] {
        assert!(written.contains(&held), "no `{held}` in: {written}");
    }
    let stowed: ItemStruct = syn::parse_quote! {
        struct StowedPair<Stowed>((String, Stowed));
    };
    let slotted = plain(&stowed);
    for held in [
        "data class StowedPair<Stowed>(val slot0: KotlinTuple",
        "Serializer(stowedSerializer), value.slot0))",
    ] {
        assert!(slotted.contains(held), "no `{held}` in: {slotted}");
    }
    let plain_pair: ItemStruct = syn::parse_quote! {
        struct PlainPair {
            pair: (u8, String),
        }
    };
    let unparameterized = plain(&plain_pair);
    assert!(
        unparameterized.contains("(val slot0: UByte, val slot1: String) object KotlinTuple"),
        "a tuple that reaches no parameter keeps its object: {unparameterized}"
    );
}

/// A plain enum that reads a variant under an alias takes the plugin's descriptor, with and
/// without a check, and names no internal serialization API.
#[cfg(feature = "serde")]
#[test]
fn a_plain_enum_with_an_alias_names_no_internal_serialization_api() {
    let swell: ItemEnum = syn::parse_quote! {
        enum Swell {
            #[serde(alias = "Low")]
            Ebb,
            Flood,
        }
    };
    let check = [FunctionName {
        name: "calm".to_owned(),
        written_at: Span::call_site(),
    }];
    for args in [
        KotlinArgs {
            type_name: "Swell",
            ..plain_args()
        },
        KotlinArgs {
            check: &check,
            type_name: "Swell",
            ..plain_args()
        },
    ] {
        let written = definition(enum_kotlin_tokens(&swell, &args));
        for absent in ["InternalSerializationApi", "buildSerialDescriptor"] {
            assert!(!written.contains(absent), "`{absent}` in: {written}");
        }
        for present in [
            "@OptIn(ExperimentalSerializationApi::class)",
            "@KeepGeneratedSerializer enum class Swell",
            "override val descriptor: SerialDescriptor = Swell.generatedSerializer().descriptor;",
        ] {
            assert!(written.contains(present), "no `{present}` in: {written}");
        }
    }
}
