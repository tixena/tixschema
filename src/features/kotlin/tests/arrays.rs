//! Unit tests of a fixed array in the Kotlin emitter: the class each length is read through, where
//! the plugin reads it and where the emitter writes the read, and the reads that keep the serializer
//! they had.

use super::*;

/// The class of a list of `length` items, declared under `named`.
fn array(named: &str, length: usize) -> String {
    format!(
        "class {named}<Item>(item: KSerializer<Item>) : KSerializer<List<Item>> {{ private val \
         listed = ListSerializer(item); override val descriptor: SerialDescriptor = \
         listed.descriptor; override fun serialize(encoder: Encoder, value: List<Item>) = \
         listed.serialize(encoder, value); override fun deserialize(decoder: Decoder): \
         List<Item> {{ val read = listed.deserialize(decoder); if (read.size != {length}) throw \
         SerializationException(\"invalid length \" + read.size + \", expected {length}\"); \
         return read }} }}"
    )
}

/// Each level written `[T; N]` names the class of its length where the plugin reads it, the outer
/// one over the inner, and each class is declared once ahead of the type.
#[test]
fn each_level_of_one_length_names_the_class_of_its_length_where_the_plugin_reads() {
    let item: ItemStruct = syn::parse_quote! {
        struct Clasping {
            grid: [[u8; 2]; 3],
            pairs: Vec<[u8; 2]>,
            maybe: Option<[u8; 2]>,
        }
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.starts_with(&format!(
            "{} {} {} data class Clasping(val grid: @Serializable(with = KotlinArray2::class) \
             List<@Serializable(with = KotlinArray1::class) List<UByte>>, val pairs: \
             List<@Serializable(with = KotlinArray1::class) List<UByte>>, val maybe: \
             @Serializable(with = KotlinArray1::class) List<UByte>? = null) {{",
            array("KotlinArray1", 2),
            array("KotlinArray2", 3),
            kept("Clasping")
        )),
        "got: {kotlin}"
    );
}

/// Where the emitter writes the read, it composes the class of each length over the serializer of
/// the items, and the declaration is written as it was.
#[test]
fn where_the_emitter_writes_the_read_it_composes_the_class_of_the_length() {
    let item: ItemStruct = syn::parse_quote! {
        struct Slots([u8; 2], Vec<[u8; 2]>);
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.starts_with(&array("KotlinArray1", 2)),
        "got: {kotlin}"
    );
    for held in [
        "data class Slots(val slot0: List<UByte>, val slot1: List<List<UByte>>) {",
        "return Slots(input.json.decodeFromJsonElement(KotlinArray1(serializer<UByte>()), \
         array[0]), \
         input.json.decodeFromJsonElement(ListSerializer(KotlinArray1(serializer<UByte>())), \
         array[1]))",
    ] {
        assert!(kotlin.contains(held), "no `{held}` in: {kotlin}");
    }
    assert_eq!(
        kotlin.matches("class KotlinArray").count(),
        1,
        "each length is declared once: {kotlin}"
    );
}

/// A fixed array of one of the class's type parameters names the class of its length, which the
/// plugin hands the parameter's serializer.
#[test]
fn a_fixed_array_of_a_type_parameter_names_the_class_of_its_length() {
    let item: ItemStruct = syn::parse_quote! {
        struct Cringling<T> {
            pair: [T; 2],
        }
    };
    let kotlin = plain(&item);
    assert!(
        kotlin.contains(
            "data class Cringling<T>(val pair: @Serializable(with = KotlinArray1::class) List<T>)"
        ),
        "got: {kotlin}"
    );
}

/// A list of no length written keeps the read it had: no class is declared.
#[test]
fn a_list_of_no_length_keeps_the_read_it_had() {
    let item: ItemStruct = syn::parse_quote! {
        struct Grasping(Vec<u8>, Vec<Vec<u8>>);
    };
    let kotlin = plain(&item);
    assert!(
        !kotlin.contains("KotlinArray")
            && kotlin.contains(
                "input.json.decodeFromJsonElement(serializer<List<UByte>>(), array[0]), \
                 input.json.decodeFromJsonElement(serializer<List<List<UByte>>>(), array[1])"
            ),
        "got: {kotlin}"
    );
}
