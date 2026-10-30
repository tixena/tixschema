//! Unit tests of a text serde reads in one form alone in the Kotlin emitter: where each shape names
//! the serializer of the form, and the reads that keep the serializer they had.

use super::*;

/// The serializer of a `char`, declared under `named`, which writes and reads one code point alone.
fn letter(named: &str) -> String {
    format!(
        "object {named} : KSerializer<String> {{ override val descriptor: SerialDescriptor = \
         PrimitiveSerialDescriptor(\"{named}\", PrimitiveKind.STRING); override fun \
         serialize(encoder: Encoder, value: String) {{ if (value.codePointCount(0, value.length) \
         != 1) throw SerializationException(\"expected a character, found \" + \
         JsonPrimitive(value)); encoder.encodeString(value) }} override fun deserialize(decoder: Decoder): String {{ val text = decoder.decodeString(); if \
         (text.codePointCount(0, text.length) != 1) throw SerializationException(\"expected a \
         character, found \" + JsonPrimitive(text)); return text }} }}"
    )
}

/// The Kotlin source of an alias.
fn aliased(item: &ItemType) -> String {
    definition(alias_kotlin_tokens(item, None))
}

/// The Kotlin source of an enum that writes nothing on itself.
fn choice(item: &ItemEnum) -> String {
    definition(enum_kotlin_tokens(item, &plain_args()))
}

/// The value of a brand over a `char` names the serializer of its form, which the brand declares.
#[cfg(feature = "serde")]
#[test]
fn a_brand_over_a_char_names_the_serializer_of_its_form_on_its_value() {
    let item: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Initial(char);
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.starts_with(&format!(
            "{} @OptIn(ExperimentalSerializationApi::class) @JvmInline @Serializable(with = \
             InitialSerializer::class) @KeepGeneratedSerializer value class Initial(val value: \
             @Serializable(with = KotlinText1::class) String) {{",
            letter("KotlinText1")
        )),
        "got: {kotlin}"
    );
}

/// A slot of a tuple struct, an item of a list in one and the value of a variant are read through
/// the serializer of their form the emitter composes, and declared as they were.
#[test]
fn where_the_emitter_writes_the_read_it_composes_the_serializer_of_the_form() {
    let item: ItemStruct = syn::parse_quote! {
        struct Marks(char, Vec<Option<char>>);
    };
    let marks = plain(&item);
    assert!(marks.starts_with(&letter("KotlinText1")), "got: {marks}");
    assert!(
        marks.contains(
            "data class Marks(val slot0: String, val slot1: List<String?>) { fun validate(): \
             List<String> = emptyList() }"
        ),
        "got: {marks}"
    );
    assert!(
        marks.contains(
            "return Marks(kotlinSchemaDecoded(input.json, KotlinText1, array[0], \"0\"), \
             kotlinSchemaDecoded(input.json, ListSerializer(KotlinText1.nullable), array[1], \
             \"1\"))"
        ),
        "got: {marks}"
    );
    let signal: ItemEnum = syn::parse_quote! {
        enum Signal {
            Flag(char),
            Mark { letter: char },
        }
    };
    let signalled = choice(&signal);
    assert!(
        signalled.contains(
            "@Serializable data class SignalMark(val letter: @Serializable(with = \
             KotlinText2::class) String) : Signal"
        ),
        "got: {signalled}"
    );
    assert!(
        signalled.contains(
            "\"Flag\" -> SignalFlag(kotlinSchemaDecoded(input.json, KotlinText2, data, \"Flag\"))"
        ),
        "got: {signalled}"
    );
}

/// An alias of a `char` names the serializer of its form in its own declaration, which a holder
/// the plugin reads reads it by, and a holder whose read the emitter writes composes the
/// serializer of the form from the type the alias names.
#[test]
fn an_alias_names_the_serializer_of_its_form_and_a_written_read_composes_it() {
    let alias: ItemType = syn::parse_quote! {
        type Initial = char;
    };
    assert_eq!(
        aliased(&alias),
        format!(
            "{} typealias InitialType = @Serializable(with = KotlinText1::class) String; \
             typealias Initial = InitialType;",
            letter("KotlinText1")
        )
    );
    let signed: ItemStruct = syn::parse_quote! {
        struct Signed {
            initial: Initial,
        }
    };
    let signed_read = plain(&signed);
    assert!(
        signed_read.contains("data class Signed(val initial: InitialType) {"),
        "got: {signed_read}"
    );
    assert!(
        !signed_read.contains("object KotlinText"),
        "got: {signed_read}"
    );
    let initials: ItemStruct = syn::parse_quote! {
        struct Initials(Initial);
    };
    let initials_read = plain(&initials);
    assert!(
        initials_read.starts_with(&letter("KotlinText2")),
        "got: {initials_read}"
    );
    assert!(
        initials_read.contains(
            "return Initials(kotlinSchemaDecoded(input.json, KotlinText2, \
             input.decodeJsonElement(), \"0\"))"
        ),
        "got: {initials_read}"
    );
}

/// A type argument names the serializer of its form where the plugin reads it, an alias with type
/// parameters and a type declared below among them, and is handed to the serializer of a model the
/// emitter wrote above where the emitter writes the read. A serializer the emitter writes reads an
/// alias with type parameters, and a type declared below, by the reified serializer it had.
#[test]
fn a_type_argument_names_the_serializer_of_its_form_and_is_handed_to_a_model_written_above() {
    let pair: ItemType = syn::parse_quote! {
        type Pair<T> = Vec<T>;
    };
    assert_eq!(
        aliased(&pair),
        "typealias PairType<T> = List<T>; typealias Pair<T> = PairType<T>;"
    );
    let held: ItemStruct = syn::parse_quote! {
        struct Held<T> {
            held: T,
        }
    };
    assert!(
        plain(&held).contains("data class Held<T>(val held: T)"),
        "a model is written above the types that hold it"
    );
    let pairing: ItemStruct = syn::parse_quote! {
        struct Pairing {
            held: Held<char>,
            later: Later<char>,
            pair: Pair<char>,
        }
    };
    let pairing_read = plain(&pairing);
    assert!(
        pairing_read.contains(
            "data class Pairing(@Serializable(with = HeldUnjudged::class) val held: \
             Held<@Serializable(with = KotlinText1::class) String>, val later: \
             Later<@Serializable(with = KotlinText1::class) String>, val pair: \
             PairType<@Serializable(with = KotlinText1::class) String>) {"
        ),
        "got: {pairing_read}"
    );
    let paired: ItemStruct = syn::parse_quote! {
        struct Paired(Held<char>, Later<char>, Pair<char>);
    };
    let paired_read = plain(&paired);
    assert!(
        paired_read.contains(
            "return Paired(kotlinSchemaDecoded(input.json, HeldUnjudged(KotlinText2), \
             array[0], \"0\"), kotlinSchemaDecoded(input.json, serializer<Later<String>>(), \
             array[1], \"1\"), kotlinSchemaDecoded(input.json, serializer<PairType<String>>(), \
             array[2], \"2\"))"
        ),
        "got: {paired_read}"
    );
}
