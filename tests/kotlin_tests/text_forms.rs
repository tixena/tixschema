//! What the Kotlin emitter writes for a text serde reads in one form alone: the serializer of each
//! form, the declaration that names it where the plugin reads, and the read the emitter writes of
//! a flattened map's key, over the models of `text_forms_models.rs`.

use super::text_forms_models::{
    chronometer_kotlin, instant_log_kotlin, letter_lines_kotlin, stamp_log_kotlin, time_log_kotlin,
};

/// What the serializer of a date refuses a text by: Zod's `z.iso.date()`, as Kotlin source writes
/// it.
const DATE: &str = concat!(
    r"(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|",
    r"\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|",
    r"(?:02)-(?:0[1-9]|1\\d|2[0-8])))"
);

/// What the serializer of a time refuses a text by: Zod's `z.iso.time()`, as Kotlin source writes
/// it.
const TIME: &str = r"(?:[01]\\d|2[0-3]):[0-5]\\d(?::[0-5]\\d(?:\\.\\d+)?)?";

/// The serializer `named` of the form that refuses a text where `refused` holds, in the words of
/// `expected`, matching it against `form` where it names one.
fn declared(named: &str, form: Option<&str>, refused: &str, expected: &str) -> String {
    let matched = form.map_or_else(String::new, |pattern| {
        format!("private val form = Regex(\"{pattern}\"); ")
    });
    format!(
        "object {named} : KSerializer<String> {{ {matched}override val descriptor: SerialDescriptor = \
         PrimitiveSerialDescriptor(\"{named}\", PrimitiveKind.STRING); override fun \
         serialize(encoder: Encoder, value: String) = encoder.encodeString(value); override fun \
         deserialize(decoder: Decoder): String {{ val text = decoder.decodeString(); if \
         ({refused}) throw SerializationException(\"expected {expected}, found \" + \
         JsonPrimitive(text)); return text }} }}"
    )
}

/// The name of the serializer `kotlin` declares for the form it names `expected`.
fn named_for(kotlin: &str, expected: &str) -> String {
    let named = kotlin
        .match_indices("object KotlinText")
        .find(|&(at, _)| {
            kotlin[at..]
                .split(" return text } }")
                .next()
                .is_some_and(|body| body.contains(&format!("\"expected {expected}, found \"")))
        })
        .map(|(at, _)| {
            let digits: String = kotlin[at + "object KotlinText".len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            format!("KotlinText{digits}")
        });
    assert!(named.is_some(), "no serializer of {expected} in {kotlin}");
    named.unwrap()
}

/// A `char` and a date are declared with the serializer of their form, on the property and as a
/// map's key, which the plugin reads them by, and each serializer is declared once beside the type.
#[test]
fn a_char_and_a_date_name_the_serializer_of_their_form_where_the_plugin_reads() {
    let kotlin = chronometer_kotlin::kotlin_definition();
    let letter = named_for(&kotlin, "a character");
    let date = named_for(&kotlin, "a date");
    assert!(
        kotlin.contains(&declared(
            &letter,
            None,
            "text.codePointCount(0, text.length) != 1",
            "a character"
        )),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(&declared(
            &date,
            Some(DATE),
            "!form.matches(text)",
            "a date"
        )),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(&format!(
            "data class Chronometer(@SerialName(\"by_date\") val byDate: Map<@Serializable(with = \
             {date}::class) String, UByte>, @SerialName(\"by_letter\") val byLetter: \
             Map<@Serializable(with = {letter}::class) String, UByte>, val day: \
             @Serializable(with = {date}::class) String, val letter: @Serializable(with = \
             {letter}::class) String) {{"
        )),
        "got: {kotlin}"
    );
    assert_eq!(
        kotlin.matches("object KotlinText").count(),
        2,
        "each form is read through one serializer of the type's own. got: {kotlin}"
    );
}

/// A time and a date and time are read through the serializer of their form in a list, as a map's
/// value and beside a key, and a date and time with its zone through none.
#[test]
fn a_time_and_a_date_and_time_are_read_through_their_form_in_every_place() {
    let timed = time_log_kotlin::kotlin_definition();
    let time = named_for(&timed, "a time");
    assert!(
        timed.contains(&declared(
            &time,
            Some(TIME),
            "!form.matches(text)",
            "a time"
        )),
        "got: {timed}"
    );
    assert!(
        timed.contains(&format!(
            "val times: List<@Serializable(with = {time}::class) String>, \
             @SerialName(\"to_time\") val toTime: Map<String, @Serializable(with = \
             {time}::class) String>"
        )),
        "got: {timed}"
    );
    let stamped = stamp_log_kotlin::kotlin_definition();
    let stamp = named_for(&stamped, "a date and time");
    assert!(
        stamped.contains(&declared(
            &stamp,
            Some(&format!(
                "{DATE}T(?:(?:[01]\\\\d|2[0-3]):[0-5]\\\\d:[0-5]\\\\d(?:\\\\.\\\\d+)?(?:Z)|{TIME})"
            )),
            "!form.matches(text)",
            "a date and time"
        )),
        "got: {stamped}"
    );
    let instants = instant_log_kotlin::kotlin_definition();
    assert!(
        !instants.contains("KotlinText"),
        "a date and time with its zone is read as any text. got: {instants}"
    );
}

/// The key of a map flattened alone is written and read through the serializer of its form, which
/// the read the emitter writes names.
#[test]
fn a_flattened_maps_key_is_read_through_the_serializer_of_its_form() {
    let kotlin = letter_lines_kotlin::kotlin_definition();
    let letter = named_for(&kotlin, "a character");
    assert!(
        kotlin.contains(&format!(
            "value.lines.forEach {{ (k, v) -> put(output.json.encodeToJsonElement({letter}, \
             k).jsonPrimitive.content, output.json.encodeToJsonElement(serializer<UByte>(), v)) }}"
        )),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(&format!(
            "val lines = obj.entries.associate {{ (k, v) -> \
             input.json.decodeFromJsonElement({letter}, JsonPrimitive(k)) to \
             input.json.decodeFromJsonElement(serializer<UByte>(), v) }}"
        )),
        "got: {kotlin}"
    );
}
