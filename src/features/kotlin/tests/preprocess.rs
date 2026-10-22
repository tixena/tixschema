//! Unit tests of `preprocess` in the Kotlin emitter: where each function a type and its members
//! name is called, shape by shape, as the emitter writes the call.

use super::*;

/// What a type with no rule answers.
const NO_RULE: &str = "fun validate(): List<String> = emptyList()";

/// The functions `names` name, as an attribute writes them.
fn named(names: &[&str]) -> Vec<FunctionName> {
    names
        .iter()
        .map(|&name| FunctionName {
            name: name.to_owned(),
            written_at: Span::call_site(),
        })
        .collect()
}

/// The arguments of an item that names `functions` on itself and writes nothing else.
fn naming(functions: &[FunctionName]) -> KotlinArgs<'_> {
    KotlinArgs {
        preprocess: functions,
        ..plain_args()
    }
}

/// The Kotlin source of a struct that names `functions` on itself.
fn corrected_by(item: &ItemStruct, functions: &[&str]) -> String {
    definition(struct_kotlin_tokens(item, &naming(&named(functions))))
}

/// The Kotlin source of an enum that names `functions` on itself.
fn choice_corrected_by(item: &ItemEnum, functions: &[&str]) -> String {
    definition(enum_kotlin_tokens(item, &naming(&named(functions))))
}

/// The body of a serializer that hands `correction` on in place of the payload it was handed.
fn transform(correction: &str) -> String {
    format!(
        "{{ override fun transformDeserialize(element: JsonElement): JsonElement = {correction} }}"
    )
}

/// The serializer a member is read through: `read_by` behind `correction`.
fn member(named: &str, ty: &str, read_by: &str, correction: &str) -> String {
    format!(
        "object {named} : JsonTransformingSerializer<{ty}>({read_by}) {}",
        transform(correction)
    )
}

/// An object with what it holds under each of `arms` corrected, as the emitter writes it over
/// `payload`.
fn keyed(payload: &str, arms: &str) -> String {
    format!(
        "{payload}.let {{ whole: JsonElement -> if (whole is JsonObject) \
         JsonObject(whole.mapValues {{ (key, held) -> when (key) {{ {arms}; else -> held }} }}) \
         else whole }}"
    )
}

/// The README shows the parcel of its `preprocess` section as the emitter writes it, each line
/// break of the block standing for one space.
#[cfg(feature = "serde")]
#[test]
fn the_readme_shows_what_the_parcel_of_its_preprocess_emits() {
    let parcel: ItemStruct = syn::parse_quote! {
        #[serde(rename_all = "camelCase")]
        pub struct Parcel {
            #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
            pub label: String,
            pub item_code_list: Vec<String>,
            pub fragile_item_codes: Vec<String>,
        }
    };
    let emitted = corrected_by(&parcel, &["withoutFragile"]);
    let readme = include_str!("../../../../README.md");
    let declares = "\ndata class Parcel(@Serializable(with = ParcelLabelPreprocess::class) ";
    let opens = "```kotlin\n";
    let block = readme
        .find(declares)
        .and_then(|at| readme.get(..at))
        .and_then(|above| above.rfind(opens))
        .and_then(|start| readme.get(start + opens.len()..))
        .and_then(|rest| rest.split_once("\n```"))
        .unwrap()
        .0;
    let shown = block
        .lines()
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(shown, emitted);
}

/// A type's functions are called by its unjudged serializer, on the whole payload and in the
/// order written, ahead of the plugin's serializer: a class, a unit struct, and a type with a
/// parameter, whose serializer stays a class over one serializer per parameter.
#[test]
fn a_types_functions_are_called_by_its_unjudged_serializer() {
    let parcel: ItemStruct = syn::parse_quote! {
        struct Parcel { label: String }
    };
    assert_eq!(
        corrected_by(&parcel, &["unwrapped", "withoutFragile"]),
        format!(
            "{} data class Parcel(val label: String) {{ {NO_RULE} }} \
             @OptIn(ExperimentalSerializationApi::class) object ParcelUnjudged : \
             JsonTransformingSerializer<Parcel>(Parcel.generatedSerializer()) {} {}",
            kept("Parcel"),
            transform("withoutFragile(unwrapped(element))"),
            judged("Parcel")
        )
    );
    let closed: ItemStruct = syn::parse_quote! {
        struct ClosedDock;
    };
    assert_eq!(
        corrected_by(&closed, &["unwrapped"]),
        format!(
            "{} object ClosedDock {{ {NO_RULE} }} @OptIn(ExperimentalSerializationApi::class) \
             object ClosedDockUnjudged : \
             JsonTransformingSerializer<ClosedDock>(ClosedDock.generatedSerializer()) {} {}",
            kept("ClosedDock"),
            transform("unwrapped(element)"),
            judged("ClosedDock")
        )
    );
    let generic: ItemStruct = syn::parse_quote! {
        struct Crate<CodeType> { codes: Vec<CodeType> }
    };
    assert_eq!(
        corrected_by(&generic, &["unwrapped"]),
        format!(
            "{} data class Crate<CodeType>(val codes: List<CodeType>) {{ {NO_RULE} }} \
             @OptIn(ExperimentalSerializationApi::class) class \
             CrateUnjudged<CodeType>(codeTypeSerializer: KSerializer<CodeType>) : \
             JsonTransformingSerializer<Crate<CodeType>>(Crate.\
             generatedSerializer(codeTypeSerializer)) {} {}",
            kept("Crate"),
            transform("unwrapped(element)"),
            judged_over("Crate", "CodeType", "", "codeTypeSerializer")
        )
    );
}

/// A type that names no function is written as it is written without the key.
#[test]
fn a_type_that_names_no_function_keeps_the_plugins_serializer_under_its_name() {
    let parcel: ItemStruct = syn::parse_quote! {
        struct Parcel { label: String }
    };
    assert_eq!(
        corrected_by(&parcel, &[]),
        format!(
            "{} data class Parcel(val label: String) {{ {NO_RULE} }} {} {}",
            kept("Parcel"),
            generated("Parcel"),
            judged("Parcel")
        )
    );
}

/// A brand's functions are handed the payload of the value it is written over. A brand with a
/// parameter delegates to the plugin's serializer and reads through it, so its own serializer
/// writes a value as it writes one without the key.
#[cfg(feature = "serde")]
#[test]
fn a_brands_functions_are_handed_the_payload_of_its_value() {
    let code: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Code(String);
    };
    let branded = corrected_by(&code, &["unwrapped", "trimmed"]);
    assert!(
        branded.contains(&format!(
            "}} @OptIn(ExperimentalSerializationApi::class) object CodeUnjudged : \
             JsonTransformingSerializer<Code>(Code.generatedSerializer()) {} object \
             CodeSerializer : ",
            transform("trimmed(unwrapped(element))")
        )),
        "got: {branded}"
    );
    let user: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct UserId<IdType>(IdType);
    };
    let filled = corrected_by(&user, &["trimmed"]);
    assert!(
        filled.contains(
            "} @OptIn(ExperimentalSerializationApi::class) class \
             UserIdUnjudged<IdType>(private val idTypeSerializer: KSerializer<IdType>) : \
             KSerializer<UserId<IdType>> by UserId.generatedSerializer(idTypeSerializer) { \
             override fun deserialize(decoder: Decoder): UserId<IdType> { val input = decoder \
             as JsonDecoder; return \
             input.json.decodeFromJsonElement(UserId.generatedSerializer(idTypeSerializer), \
             trimmed(input.decodeJsonElement())) } } class UserIdSerializer<IdType>("
        ),
        "got: {filled}"
    );
    assert!(
        corrected_by(&user, &[]).contains(
            "} @OptIn(ExperimentalSerializationApi::class) class \
             UserIdUnjudged<IdType>(idTypeSerializer: KSerializer<IdType>) : \
             KSerializer<UserId<IdType>> by UserId.generatedSerializer(idTypeSerializer) class \
             UserIdSerializer<IdType>("
        ),
        "a brand that names no function keeps the delegation alone"
    );
}

/// A plain enum that names a function is read by `{Type}Unjudged`, the one serializer it has,
/// over the plugin's. One that names none keeps the plugin's alone.
#[test]
fn a_plain_enum_that_names_a_function_is_read_by_its_unjudged_serializer() {
    let grade: ItemEnum = syn::parse_quote! {
        enum Grade { Express, Standard }
    };
    let members = format!("enum class Grade {{ Express, Standard; {NO_RULE} }}");
    assert_eq!(
        choice_corrected_by(&grade, &["unwrapped"]),
        format!(
            "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
             GradeUnjudged::class) @KeepGeneratedSerializer {members} \
             @OptIn(ExperimentalSerializationApi::class) object GradeUnjudged : \
             JsonTransformingSerializer<Grade>(Grade.generatedSerializer()) {}",
            transform("unwrapped(element)")
        )
    );
    assert_eq!(
        choice_corrected_by(&grade, &[]),
        format!("@Serializable {members}")
    );
}

/// A serializer the emitter writes calls the type's functions on the payload it decodes, ahead
/// of everything it reads from it: a struct with a flattened member, a tuple struct, and an enum
/// under each tagging, an untagged one ahead of the choice of a variant.
#[cfg(feature = "serde")]
#[test]
fn a_serializer_the_emitter_writes_corrects_the_payload_it_decodes() {
    let decoded = "val input = decoder as JsonDecoder; ";
    let yard: ItemStruct = syn::parse_quote! {
        struct Yard {
            #[serde(flatten)]
            base: Base,
            gate: String,
        }
    };
    let pile: ItemStruct = syn::parse_quote! {
        struct Pile(Vec<String>, u32);
    };
    for (written, read) in [
        (
            corrected_by(&yard, &["unwrapped"]),
            "val obj = unwrapped(input.decodeJsonElement()).let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }; val baseKeys = ",
        ),
        (
            corrected_by(&pile, &["unwrapped", "listed"]),
            "val array = listed(unwrapped(input.decodeJsonElement())).let { it as? JsonArray ?: \
             throw SerializationException(\"expected an array, found \" + it) }; if (array.size \
             != 2) throw SerializationException(\"invalid length \" + array.size + \", expected \
             2\"); return Pile(",
        ),
    ] {
        assert!(
            written.contains(&format!("{decoded}{read}")),
            "no `{read}` in: {written}"
        );
    }
    let beside: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind", content = "detail")]
        enum Handover { Left(String), Signed { by: String } }
    };
    let under_its_name: ItemEnum = syn::parse_quote! {
        enum Transfer { Held, Rail(String) }
    };
    let under_a_tag: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind")]
        enum Shipment { Air { flight: String }, Ground }
    };
    let untagged: ItemEnum = syn::parse_quote! {
        #[serde(untagged)]
        enum Reference { Named { name: String }, Numbered(u32) }
    };
    for (item, read) in [
        (
            &beside,
            "val obj = unwrapped(input.decodeJsonElement()).let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }; val tag = \
             (obj[\"kind\"] ?: throw SerializationException(\"missing field `kind`\")).let { it \
             as? JsonPrimitive ?: throw SerializationException(\"expected a string, found \" + \
             it) }.content; ",
        ),
        (
            &under_its_name,
            "val element = unwrapped(input.decodeJsonElement()); val (tag, data) = ",
        ),
        (
            &under_a_tag,
            "val obj = unwrapped(input.decodeJsonElement()).let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }; val tag = \
             (obj[\"kind\"] as? JsonPrimitive)?.content; ",
        ),
        (
            &untagged,
            "val element = unwrapped(input.decodeJsonElement()); return runCatching { ",
        ),
    ] {
        let written = choice_corrected_by(item, &["unwrapped"]);
        assert!(
            written.contains(&format!("{decoded}{read}")),
            "no `{read}` in: {written}"
        );
    }
}

/// Where the plugin reads a member, the member's functions are called by a serializer of the
/// member's own, which the property names and which reads through the serializer the member has
/// without the key: the plugin's for a text, a nullable one for an `Option`, and a model's
/// unjudged one where the holder's `validate()` asks the model.
#[test]
fn a_member_the_plugin_reads_is_read_through_a_serializer_of_its_own() {
    let written = [labelled_parcel()];
    assert_eq!(written.len(), 1);
    let chit: ItemStruct = syn::parse_quote! {
        struct Chit {
            #[model_schema_prop(preprocess = ["trimmed", "lowered"])]
            code: String,
            #[model_schema_prop(preprocess = ["trimmed"])]
            note: Option<String>,
            #[model_schema_prop(preprocess = ["unwrapped"])]
            parcels: Vec<Parcel>,
            plain: String,
            #[model_schema_prop(preprocess = ["unwrapped"])]
            spare_parcel: Option<Parcel>,
        }
    };
    let kotlin = plain(&chit);
    let opens = format!(
        "{} data class Chit(@Serializable(with = ChitCodePreprocess::class) val code: String, \
         @Serializable(with = ChitNotePreprocess::class) val note: String? = null, \
         @Serializable(with = ChitParcelsPreprocess::class) val parcels: List<Parcel>, val \
         plain: String, @SerialName(\"spare_parcel\") @Serializable(with = \
         ChitSpareParcelPreprocess::class) val spareParcel: Parcel? = null) {{ fun validate(): ",
        kept("Chit")
    );
    assert!(kotlin.starts_with(&opens), "got: {kotlin}");
    let closes = format!(
        "}} {} {} {} {} {} {}",
        generated("Chit"),
        judged("Chit"),
        member(
            "ChitCodePreprocess",
            "String",
            "serializer<String>()",
            "lowered(trimmed(element))"
        ),
        member(
            "ChitNotePreprocess",
            "String?",
            "serializer<String?>()",
            "trimmed(element)"
        ),
        member(
            "ChitParcelsPreprocess",
            "List<Parcel>",
            "ListSerializer(ParcelUnjudged)",
            "unwrapped(element)"
        ),
        member(
            "ChitSpareParcelPreprocess",
            "Parcel?",
            "ParcelUnjudged.nullable",
            "unwrapped(element)"
        )
    );
    assert!(kotlin.ends_with(&closes), "got: {kotlin}");
}

/// A tuple written as a member's type is asked for its name once, so the member and the
/// serializer it is read through name one wrapper.
#[test]
fn a_tuple_member_that_names_a_function_names_one_wrapper() {
    let paired: ItemStruct = syn::parse_quote! {
        struct Paired {
            #[model_schema_prop(preprocess = ["unwrapped"])]
            pair: (String, u32),
        }
    };
    let kotlin = plain(&paired);
    for held in [
        "data class Paired(@Serializable(with = PairedPairPreprocess::class) val pair: \
         KotlinTuple1) {",
        "object PairedPairPreprocess : \
         JsonTransformingSerializer<KotlinTuple1>(serializer<KotlinTuple1>()) {",
    ] {
        assert!(kotlin.contains(held), "no `{held}` in: {kotlin}");
    }
    assert!(!kotlin.contains("KotlinTuple2"), "got: {kotlin}");
}

/// The plugin takes no serializer of a member's own where the member's type reaches a type
/// parameter, so the type's unjudged serializer calls that member's functions at its key, after
/// the type's own. A member of the same type that reaches none keeps a serializer of its own.
#[test]
fn a_member_that_reaches_a_parameter_is_corrected_at_its_key() {
    let hamper: ItemStruct = syn::parse_quote! {
        struct Hamper<CodeType> {
            #[model_schema_prop(preprocess = ["unwrapped"])]
            codes: Vec<CodeType>,
            #[model_schema_prop(preprocess = ["unwrapped", "listed"])]
            held: CodeType,
            #[model_schema_prop(preprocess = ["trimmed"])]
            label: String,
        }
    };
    let arms = "\"codes\" -> unwrapped(held); \"held\" -> listed(unwrapped(held))";
    for (functions, whole) in [
        (&[][..], "element"),
        (&["withoutFragile"][..], "withoutFragile(element)"),
    ] {
        assert_eq!(
            corrected_by(&hamper, functions),
            format!(
                "{} data class Hamper<CodeType>(val codes: List<CodeType>, val held: CodeType, \
                 @Serializable(with = HamperLabelPreprocess::class) val label: String) {{ \
                 {NO_RULE} }} @OptIn(ExperimentalSerializationApi::class) class \
                 HamperUnjudged<CodeType>(codeTypeSerializer: KSerializer<CodeType>) : \
                 JsonTransformingSerializer<Hamper<CodeType>>(Hamper.\
                 generatedSerializer(codeTypeSerializer)) {} {} {}",
                kept("Hamper"),
                transform(&keyed(whole, arms)),
                judged_over("Hamper", "CodeType", "", "codeTypeSerializer"),
                member(
                    "HamperLabelPreprocess",
                    "String",
                    "serializer<String>()",
                    "trimmed(element)"
                )
            )
        );
    }
}

/// Where the emitter writes the read, a member's functions are called in place, on the payload
/// under the member's key: a member of a struct with a flattened one, a nullable one only where
/// the payload holds its key, and a slot of a tuple struct.
#[cfg(feature = "serde")]
#[test]
fn a_member_the_emitter_reads_is_corrected_in_place() {
    let written = [labelled_parcel()];
    assert_eq!(written.len(), 1);
    let bonded: ItemStruct = syn::parse_quote! {
        struct Bonded {
            #[model_schema_prop(preprocess = ["trimmed"])]
            dock: String,
            #[model_schema_prop(preprocess = ["trimmed", "lowered"])]
            note: Option<String>,
            #[serde(flatten)]
            parcel: Parcel,
        }
    };
    let kotlin = plain(&bonded);
    for held in [
        "data class Bonded(val dock: String, val note: String? = null, val parcel: Parcel) {",
        "val dock = input.json.decodeFromJsonElement(serializer<String>(), \
         trimmed((obj[\"dock\"] ?: throw SerializationException(\"missing field `dock`\"))));",
        "val note = obj[\"note\"]?.let { input.json.decodeFromJsonElement(serializer<String?>(), \
         lowered(trimmed(it))) };",
        "val parcel = input.json.decodeFromJsonElement(ParcelUnjudged, JsonObject(obj.filterKeys \
         { it in setOf(\"label\") }));",
    ] {
        assert!(kotlin.contains(held), "no `{held}` in: {kotlin}");
    }
    assert!(!kotlin.contains("Preprocess"), "got: {kotlin}");
    let stretch: ItemStruct = syn::parse_quote! {
        struct Stretch(#[model_schema_prop(preprocess = ["trimmed"])] String, u32);
    };
    let slots = plain(&stretch);
    assert!(
        slots.contains(
            "return Stretch(input.json.decodeFromJsonElement(serializer<String>(), \
             trimmed(array[0])), input.json.decodeFromJsonElement(serializer<UInt>(), array[1]))"
        ),
        "got: {slots}"
    );
}

/// A flattened member is handed the keys neither its holder's own members nor a struct flattened
/// ahead of it take where a function reads it, its type's or its own, or its type is one the
/// emitter has not written. A struct the emitter wrote that names no function is read from the
/// whole object, as TypeScript and Zod read it. A type that names a function, and one the emitter
/// has not written, take none of the keys they read.
#[cfg(feature = "serde")]
#[test]
fn every_flattened_member_is_handed_what_no_member_ahead_of_it_takes() {
    let base: ItemStruct = syn::parse_quote! {
        struct Base { name: String }
    };
    let keyed_type: ItemStruct = syn::parse_quote! {
        struct Keyed { tag: String }
    };
    let written = [plain(&base), corrected_by(&keyed_type, &["withItsKeys"])];
    assert_eq!(written.len(), 2);
    let fastened: ItemStruct = syn::parse_quote! {
        struct Fastened {
            #[serde(flatten)]
            base: Base,
            dock: String,
            gate: String,
            #[serde(flatten)]
            keyed: Keyed,
            #[serde(flatten)]
            later: Later,
            #[serde(flatten)]
            again: Base,
        }
    };
    let kotlin = plain(&fastened);
    let rest = "JsonObject(obj - setOf(\"dock\", \"gate\") - baseKeys)";
    let whole = "val base = input.json.decodeFromJsonElement(BaseUnjudged, \
                 JsonObject(obj.filterKeys { it in setOf(\"name\") }));";
    for held in [
        whole.to_owned(),
        format!("val keyed = lenient.decodeFromJsonElement(KeyedUnjudged, {rest});"),
        format!("val later = lenient.decodeFromJsonElement(serializer<Later>(), {rest});"),
        whole.replace("val base", "val again"),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
    // A holder of the holder hands it the rest too: what it flattens is handed to a function.
    let outer: ItemStruct = syn::parse_quote! {
        struct Outer {
            #[serde(flatten)]
            fastened: Fastened,
            yard: String,
        }
    };
    let around = plain(&outer);
    assert!(
        around.contains(
            "val fastened = lenient.decodeFromJsonElement(FastenedUnjudged, JsonObject(obj - \
             setOf(\"yard\")));"
        ),
        "got: {around}"
    );
    let girdled: ItemStruct = syn::parse_quote! {
        struct Girdled {
            #[model_schema_prop(preprocess = ["withItsKeys"])]
            #[serde(flatten)]
            base: Base,
            dock: String,
        }
    };
    let named_by_the_member = plain(&girdled);
    assert!(
        named_by_the_member.contains(
            "val base = lenient.decodeFromJsonElement(BaseUnjudged, withItsKeys(JsonObject(obj - \
             setOf(\"dock\"))));"
        ),
        "got: {named_by_the_member}"
    );
    let strapped: ItemStruct = syn::parse_quote! {
        struct Strapped {
            #[model_schema_prop(preprocess = ["withItsKeys"])]
            #[serde(flatten)]
            rest: BTreeMap<String, String>,
        }
    };
    let named_by_the_map = plain(&strapped);
    assert!(
        named_by_the_map.contains(
            "val rest = withItsKeys(obj).let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }.mapValues { (_, v) -> \
             input.json.decodeFromJsonElement(serializer<String>(), v) }"
        ),
        "got: {named_by_the_map}"
    );
}

/// A member of a variant the plugin reads is read through a serializer of its own, named after
/// the variant's class. A slot is corrected by the enum's serializer: in place where it reads
/// the slot itself, and under the `value` key of the object it hands the plugin under a tag
/// alone. A slot of several is corrected by the wrapper that reads the array.
#[cfg(feature = "serde")]
#[test]
fn a_member_and_a_slot_of_a_variant_are_corrected_where_each_is_read() {
    let written = [labelled_parcel()];
    assert_eq!(written.len(), 1);
    let beside: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind", content = "detail")]
        enum Portage {
            Boxed {
                #[model_schema_prop(preprocess = ["trimmed"])]
                label: String,
            },
            Lone(#[model_schema_prop(preprocess = ["trimmed"])] String),
            Split(#[model_schema_prop(preprocess = ["trimmed"])] String, u32),
        }
    };
    let portage = definition(enum_kotlin_tokens(&beside, &plain_args()));
    let boxed = member(
        "PortageBoxedLabelPreprocess",
        "String",
        "serializer<String>()",
        "trimmed(element)",
    );
    for held in [
        "@Serializable data class PortageBoxed(@Serializable(with = \
         PortageBoxedLabelPreprocess::class) val label: String) : Portage {",
        "@Serializable data class PortageLone(val value: String) : Portage {",
        "\"Boxed\" -> input.json.decodeFromJsonElement(serializer<PortageBoxed>(), (data ?: \
         throw SerializationException(\"missing field `detail`\"))); ",
        "\"Lone\" -> PortageLone(input.json.decodeFromJsonElement(serializer<String>(), \
         trimmed((data ?: throw SerializationException(\"missing field `detail`\"))))); ",
        "(input.json.decodeFromJsonElement(serializer<String>(), trimmed(array[0])), \
         input.json.decodeFromJsonElement(serializer<UInt>(), array[1])) } }",
    ] {
        assert!(portage.contains(held), "no `{held}` in: {portage}");
    }
    assert!(portage.ends_with(&boxed), "got: {portage}");
    let under_a_tag: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind")]
        enum Lift {
            Boxed(#[model_schema_prop(preprocess = ["unwrapped"])] Parcel),
            Loose {
                #[model_schema_prop(preprocess = ["trimmed"])]
                label: String,
            },
        }
    };
    let lift = definition(enum_kotlin_tokens(&under_a_tag, &plain_args()));
    for held in [
        format!(
            "\"Boxed\" -> input.json.decodeFromJsonElement(serializer<LiftBoxed>(), {}); ",
            keyed("JsonObject(obj - \"kind\")", "\"value\" -> unwrapped(held)")
        ),
        "\"Loose\" -> input.json.decodeFromJsonElement(serializer<LiftLoose>(), JsonObject(obj - \
         \"kind\")) else -> "
            .to_owned(),
        "data class LiftLoose(@Serializable(with = LiftLooseLabelPreprocess::class) val label: \
         String) : Lift {"
            .to_owned(),
    ] {
        assert!(lift.contains(&held), "no `{held}` in: {lift}");
    }
}

/// An untagged enum corrects a slot as it tries the variant, and a member of a variant through
/// the member's own serializer, so each variant is tried on what its own functions answer.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_enum_corrects_each_variant_as_it_tries_it() {
    let untagged: ItemEnum = syn::parse_quote! {
        #[serde(untagged)]
        enum Guess {
            Coded {
                #[model_schema_prop(preprocess = ["trimmed"])]
                code: String,
            },
            Short(#[model_schema_prop(maxLength = 2, preprocess = ["trimmed"])] String),
            Whole(String),
        }
    };
    let guess = definition(enum_kotlin_tokens(&untagged, &plain_args()));
    for held in [
        "@Serializable data class GuessCoded(@Serializable(with = \
         GuessCodedCodePreprocess::class) val code: String) : Guess {",
        "@JvmInline @Serializable value class GuessShort(val value: String) : Guess {",
        ".recoverCatching { input.json.decodeFromJsonElement(serializer<GuessCoded>(), element) \
         as Guess }",
        ".recoverCatching { input.json.decodeFromJsonElement(serializer<GuessShort>(), \
         trimmed(element)).also { chosen -> ",
        ".recoverCatching { input.json.decodeFromJsonElement(serializer<GuessWhole>(), element) \
         as Guess }",
    ] {
        assert!(guess.contains(held), "no `{held}` in: {guess}");
    }
}

/// A member and a slot of a variant that reach a parameter of their enum are corrected by the
/// enum's serializer: the member at its key in the object the plugin is handed, the slot in
/// place.
#[cfg(feature = "serde")]
#[test]
fn a_variant_member_that_reaches_a_parameter_is_corrected_by_the_enums_serializer() {
    let generic: ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind", content = "detail")]
        enum Pick<CodeType> {
            Many {
                #[model_schema_prop(preprocess = ["unwrapped"])]
                codes: Vec<CodeType>,
            },
            One(#[model_schema_prop(preprocess = ["unwrapped"])] CodeType),
        }
    };
    let pick = definition(enum_kotlin_tokens(&generic, &plain_args()));
    for held in [
        "@Serializable data class PickMany<CodeType>(val codes: List<CodeType>) : \
         Pick<CodeType> {"
            .to_owned(),
        format!(
            "\"Many\" -> input.json.decodeFromJsonElement(PickMany.serializer(codeTypeSerializer), \
             {}); ",
            keyed(
                "(data ?: throw SerializationException(\"missing field `detail`\"))",
                "\"codes\" -> unwrapped(held)"
            )
        ),
        "\"One\" -> PickOne(input.json.decodeFromJsonElement(codeTypeSerializer, \
         unwrapped((data ?: throw SerializationException(\"missing field `detail`\"))))) else -> "
            .to_owned(),
    ] {
        assert!(pick.contains(&held), "no `{held}` in: {pick}");
    }
    assert!(!pick.contains("Preprocess"), "got: {pick}");
}
