//! Unit tests of `preprocess` in the Kotlin emitter: where each function a type and its members
//! name is called, shape by shape, as the emitter writes the call.

use super::*;
use crate::utils::note_rule_less;

/// What the draft of a type with no rule answers.
const NO_ISSUES: &str = "fun validationIssues(): List<String> = emptyList()";

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
        type_name: "Probe",
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

/// The body of a serializer of `ty` that reads through `held` what `correction` answers in place
/// of the payload it was handed, from its text.
fn transform(ty: &str, held: &str, correction: &str) -> String {
    let decoded = if correction.contains("kotlinSchemaKinded(") {
        DECODED
    } else {
        ""
    };
    format!(
        "{{ private val held = {held}; override val descriptor: SerialDescriptor = \
         held.descriptor; override fun serialize(encoder: Encoder, value: {ty}) = \
         held.serialize(encoder, value); {decoded}{PATHED}override fun deserialize(decoder: \
         Decoder): {ty} {{ val input = decoder as JsonDecoder; val element = \
         input.decodeJsonElement(); return kotlinSchemaAt(input) {{ \
         input.json.decodeFromString(held, transformDeserialize(element).toString()) }} }}; \
         private fun transformDeserialize(element: JsonElement): JsonElement = {correction} }}"
    )
}

/// The serializer a member is read through: `read_by` behind `correction`.
fn member(named: &str, ty: &str, read_by: &str, correction: &str, key: &str) -> String {
    let kinded = format!("kotlinSchemaKinded({read_by}, {correction}, \"{key}\")");
    format!(
        "object {named} : KSerializer<{ty}> {}",
        transform(ty, read_by, &kinded)
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
    let readme = include_str!("../../../../docs/guide/kotlin.md");
    let declares = "\ndata class ParcelDraft(@Serializable(with = ParcelLabelPreprocess::class) ";
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
    note_rule_less(true);
    let parcel: ItemStruct = syn::parse_quote! {
        struct Parcel { label: String }
    };
    assert_eq!(
        corrected_by(&parcel, &["unwrapped", "withoutFragile"]),
        format!(
            "{} data class ParcelDraft(val label: String) {{ {NO_ISSUES} {} }} \
             @OptIn(ExperimentalSerializationApi::class) object ParcelDraftUnjudged : \
             KSerializer<ParcelDraft> {} {} {} {}",
            kept_draft("Parcel"),
            to_valid_free("Parcel", ""),
            transform(
                "ParcelDraft",
                "ParcelDraft.generatedSerializer()",
                "withoutFragile(unwrapped(element))"
            ),
            with_of(
                valid_class("Parcel", "val label: String get() = draft.label; "),
                "fun of(label: String): Parcel = ParcelDraft(label = label).validate()"
            ),
            valid_unjudged("Parcel"),
            judged_valid("Parcel")
        )
    );
    let closed: ItemStruct = syn::parse_quote! {
        struct ClosedDock;
    };
    assert_eq!(
        corrected_by(&closed, &["unwrapped"]),
        format!(
            "{} object ClosedDockDraft {{ {NO_ISSUES} {} }} \
             @OptIn(ExperimentalSerializationApi::class) object ClosedDockDraftUnjudged : \
             KSerializer<ClosedDockDraft> {} {} {} {}",
            kept_draft("ClosedDock"),
            to_valid_free("ClosedDock", ""),
            transform(
                "ClosedDockDraft",
                "ClosedDockDraft.generatedSerializer()",
                "unwrapped(element)"
            ),
            valid_class_with(
                "ClosedDock",
                "",
                "; val instance = checked(ClosedDockDraft); fun of(): ClosedDock = \
                 ClosedDockDraft.validate()"
            ),
            valid_unjudged("ClosedDock"),
            judged_valid("ClosedDock")
        )
    );
    let generic: ItemStruct = syn::parse_quote! {
        struct Crate<CodeType> { codes: Vec<CodeType> }
    };
    assert_eq!(
        corrected_by(&generic, &["unwrapped"]),
        format!(
            "{} data class CrateDraft<CodeType>(val codes: List<CodeType>) {{ {NO_ISSUES} {} }} \
             @OptIn(ExperimentalSerializationApi::class) class \
             CrateDraftUnjudged<CodeType>(codeTypeSerializer: KSerializer<CodeType>) : \
             KSerializer<CrateDraft<CodeType>> {} {} {} {}",
            kept_draft("Crate"),
            to_valid_free("Crate", "<CodeType>"),
            transform(
                "CrateDraft<CodeType>",
                "CrateDraft.generatedSerializer(codeTypeSerializer)",
                "unwrapped(element)"
            ),
            with_of(
                valid_class_over(
                    "Crate",
                    "<CodeType>",
                    "val codes: List<CodeType> get() = draft.codes; "
                ),
                "fun <CodeType> of(codes: List<CodeType>): Crate<CodeType> = \
                 CrateDraft<CodeType>(codes = codes).validate()"
            ),
            valid_unjudged_over("Crate", "CodeType", "", "codeTypeSerializer"),
            judged_valid_over("Crate", "CodeType", "", "codeTypeSerializer")
        )
    );
}

/// A type that names no function is written as it is written without the key.
#[test]
fn a_type_that_names_no_function_keeps_the_plugins_serializer_under_its_name() {
    note_rule_less(true);
    let parcel: ItemStruct = syn::parse_quote! {
        struct Parcel { label: String }
    };
    assert_eq!(
        corrected_by(&parcel, &[]),
        format!(
            "{} data class ParcelDraft(val label: String) {{ {NO_ISSUES} {} }} {} {} {} {}",
            kept_draft("Parcel"),
            to_valid_free("Parcel", ""),
            generated("ParcelDraft"),
            with_of(
                valid_class("Parcel", "val label: String get() = draft.label; "),
                "fun of(label: String): Parcel = ParcelDraft(label = label).validate()"
            ),
            valid_unjudged("Parcel"),
            judged_valid("Parcel")
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
            "}} }} @OptIn(ExperimentalSerializationApi::class) private object CodeUnjudged : \
             KSerializer<Code> {} object CodeSerializer : ",
            transform(
                "Code",
                "Code.generatedSerializer()",
                "trimmed(unwrapped(element))"
            )
        )),
        "got: {branded}"
    );
    let user: ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct UserId<IdType>(IdType);
    };
    let filled = corrected_by(&user, &["trimmed"]);
    assert!(
        filled.contains(&format!(
            "}} }} @OptIn(ExperimentalSerializationApi::class) private class \
             UserIdUnjudged<IdType>(private val idTypeSerializer: KSerializer<IdType>) : \
             KSerializer<UserId<IdType>> by UserId.generatedSerializer(idTypeSerializer) {{ \
             {DECODED}{PATHED}override fun deserialize(decoder: Decoder): UserId<IdType> {{ val \
             input = decoder as JsonDecoder; val element = trimmed(input.decodeJsonElement()); \
             return kotlinSchemaAt(input) {{ \
             input.json.decodeFromString(UserId.generatedSerializer(idTypeSerializer), \
             kotlinSchemaKinded(UserId.generatedSerializer(idTypeSerializer), element, \
             \"UserId\").toString()) }} }} }} class UserIdSerializer<IdType>("
        )),
        "got: {filled}"
    );
    assert!(
        corrected_by(&user, &[]).contains(
            "} } @OptIn(ExperimentalSerializationApi::class) private class \
             UserIdUnjudged<IdType>(idTypeSerializer: KSerializer<IdType>) : \
             KSerializer<UserId<IdType>> by UserId.generatedSerializer(idTypeSerializer) class \
             UserIdSerializer<IdType>("
        ),
        "a brand that names no function keeps the delegation alone"
    );
}

/// A plain enum is read by `{Type}Unjudged`, the one serializer it has, over the plugin's, which
/// reads the payload the functions it names answer.
#[test]
fn a_plain_enum_is_read_by_its_unjudged_serializer() {
    note_rule_less(true);
    let grade: ItemEnum = syn::parse_quote! {
        enum Grade { Express, Standard }
    };
    let members = format!(
        "enum class GradeDraft {{ Express, Standard; {NO_ISSUES} {} }}",
        to_valid_free("Grade", "")
    );
    for (functions, correction) in [
        (&["unwrapped"][..], "unwrapped(element)"),
        (&[][..], "element"),
    ] {
        assert_eq!(
            choice_corrected_by(&grade, functions),
            format!(
                "{} {members} {} {} {} {}",
                kept_plain_draft("Grade"),
                plain_enum_unjudged("GradeDraft", &["Express", "Standard"], correction),
                valid_class_with(
                    "Grade",
                    "",
                    "; val Express = checked(GradeDraft.Express); val Standard = checked(GradeDraft.Standard)"
                ),
                valid_unjudged("Grade"),
                judged_valid("Grade")
            )
        );
    }
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
            "val element = unwrapped(input.decodeJsonElement()); return kotlinSchemaAt(input) { \
             val obj = element.let { it as? JsonObject ?: throw SerializationException(\"expected \
             an object, found \" + it) }; val baseKeys = ",
        ),
        (
            corrected_by(&pile, &["unwrapped", "listed"]),
            "val element = listed(unwrapped(input.decodeJsonElement())); return \
             kotlinSchemaAt(input) { val array = element.let { it as? JsonArray ?: throw \
             SerializationException(\"expected an array, found \" + it) }; if (array.size != 2) \
             throw SerializationException(\"invalid length \" + array.size + \", expected 2\"); \
             PileDraft(",
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
            "val element = unwrapped(input.decodeJsonElement()); return kotlinSchemaAt(input) { \
             val obj = element.let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }; val tag = \
             (obj[\"kind\"] ?: throw SerializationException(\"missing field `kind`\")).let { it \
             as? JsonPrimitive ?: throw SerializationException(\"expected a string, found \" + \
             it) }.content; ",
        ),
        (
            &under_its_name,
            "val element = unwrapped(input.decodeJsonElement()); return kotlinSchemaAt(input) { \
             val (tag, data) = ",
        ),
        (
            &under_a_tag,
            "val element = unwrapped(input.decodeJsonElement()); return kotlinSchemaAt(input) { \
             val obj = element.let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }; val tag = \
             ((obj[\"kind\"] ?: throw SerializationException(\"missing field `kind`\")) as? \
             JsonPrimitive)?.content; ",
        ),
        (
            &untagged,
            "val element = unwrapped(input.decodeJsonElement()); return kotlinSchemaAt(input) { \
             runCatching { ",
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
        "{} data class ChitDraft(@Serializable(with = ChitCodePreprocess::class) val code: String, \
         @Serializable(with = ChitNotePreprocess::class) val note: String? = null, \
         @Serializable(with = ChitParcelsPreprocess::class) val parcels: List<Parcel>, val \
         plain: String, @SerialName(\"spare_parcel\") @Serializable(with = \
         ChitSpareParcelPreprocess::class) val spareParcel: Parcel? = null) {{ fun \
         validationIssues(): ",
        kept_draft("Chit")
    );
    assert!(kotlin.starts_with(&opens), "got: {kotlin}");
    let closes = format!(
        "}} {} {} {} {} {} {} {} {}",
        generated("ChitDraft"),
        with_of(
            valid_class(
                "Chit",
                "val code: String get() = draft.code; val note: String? get() = draft.note; val \
                 parcels: List<Parcel> get() = draft.parcels; val plain: String get() = \
                 draft.plain; val spareParcel: Parcel? get() = draft.spareParcel; "
            ),
            "fun of(code: String, note: String? = null, parcels: List<Parcel>, plain: String, \
             spareParcel: Parcel? = null): Chit = ChitDraft(code = code, note = note, parcels = \
             parcels, plain = plain, spareParcel = spareParcel).validate()"
        ),
        valid_unjudged("Chit"),
        judged_valid("Chit"),
        member(
            "ChitCodePreprocess",
            "String",
            "serializer<String>()",
            "lowered(trimmed(element))",
            "code"
        ),
        member(
            "ChitNotePreprocess",
            "String?",
            "serializer<String?>()",
            "trimmed(element)",
            "note"
        ),
        member(
            "ChitParcelsPreprocess",
            "List<Parcel>",
            "ListSerializer(ParcelUnjudged)",
            "unwrapped(element)",
            "parcels"
        ),
        member(
            "ChitSpareParcelPreprocess",
            "Parcel?",
            "ParcelUnjudged.nullable",
            "unwrapped(element)",
            "spare_parcel"
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
        "data class PairedDraft(@Serializable(with = PairedPairPreprocess::class) val pair: \
         KotlinTuple1) {",
        "object PairedPairPreprocess : KSerializer<KotlinTuple1> { private val held = \
         serializer<KotlinTuple1>();",
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
    note_rule_less(true);
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
                "{} data class HamperDraft<CodeType>(val codes: List<CodeType>, val held: CodeType, \
                 @Serializable(with = HamperLabelPreprocess::class) val label: String) {{ \
                 {NO_ISSUES} {} }} @OptIn(ExperimentalSerializationApi::class) class \
                 HamperDraftUnjudged<CodeType>(codeTypeSerializer: KSerializer<CodeType>) : \
                 KSerializer<HamperDraft<CodeType>> {} {} {} {} {}",
                kept_draft("Hamper"),
                to_valid_free("Hamper", "<CodeType>"),
                transform(
                    "HamperDraft<CodeType>",
                    "HamperDraft.generatedSerializer(codeTypeSerializer)",
                    &keyed(whole, arms)
                ),
                with_of(
                    valid_class_over(
                        "Hamper",
                        "<CodeType>",
                        "val codes: List<CodeType> get() = draft.codes; val held: CodeType get() = \
                         draft.held; val label: String get() = draft.label; "
                    ),
                    "fun <CodeType> of(codes: List<CodeType>, held: CodeType, label: String): \
                     Hamper<CodeType> = HamperDraft<CodeType>(codes = codes, held = held, label = \
                     label).validate()"
                ),
                valid_unjudged_over("Hamper", "CodeType", "", "codeTypeSerializer"),
                judged_valid_over("Hamper", "CodeType", "", "codeTypeSerializer"),
                member(
                    "HamperLabelPreprocess",
                    "String",
                    "serializer<String>()",
                    "trimmed(element)",
                    "label"
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
        "data class BondedDraft(val dock: String, val note: String? = null, val parcel: Parcel) {",
        "val dock = kotlinSchemaDecoded(input.json, serializer<String>(), trimmed((obj[\"dock\"] \
         ?: throw SerializationException(\"missing field `dock`\"))), \"dock\", \".dock\");",
        "val note = obj[\"note\"]?.let { kotlinSchemaDecoded(input.json, serializer<String?>(), \
         lowered(trimmed(it)), \"note\", \".note\") };",
        "val parcel = kotlinSchemaFlattened { input.json.decodeFromString(ParcelUnjudged, \
         JsonObject(obj.filterKeys { it in setOf(\"label\") }).toString()) };",
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
            "StretchDraft(kotlinSchemaDecoded(input.json, serializer<String>(), \
             trimmed(array[0]), \"0\", \"[0]\"), kotlinSchemaDecoded(input.json, KotlinNumber1, \
             array[1], \"1\", \"[1]\"))"
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
    let whole = "val base = kotlinSchemaFlattened { input.json.decodeFromString(BaseUnjudged, \
                 JsonObject(obj.filterKeys { it in setOf(\"name\") }).toString()) };";
    for held in [
        whole.to_owned(),
        format!(
            "val keyed = kotlinSchemaFlattened {{ lenient.decodeFromString(KeyedUnjudged, \
             {rest}.toString()) }};"
        ),
        format!(
            "val later = kotlinSchemaFlattened {{ lenient.decodeFromString(serializer<Later>(), \
             kotlinSchemaFlat(serializer<Later>(), {rest}).toString()) }};"
        ),
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
            "val fastened = kotlinSchemaFlattened { lenient.decodeFromString(FastenedUnjudged, \
             JsonObject(obj - setOf(\"yard\")).toString()) };"
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
            "val base = kotlinSchemaFlattened { lenient.decodeFromString(BaseUnjudged, \
             withItsKeys(JsonObject(obj - setOf(\"dock\"))).toString()) };"
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
            "val rest = kotlinSchemaFlattened { withItsKeys(obj).let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }.mapValues { (k, v) -> \
             kotlinSchemaDecoded(input.json, serializer<String>(), v, k, \".\" + k) } }"
        ),
        "got: {named_by_the_map}"
    );
}

/// A member of a variant the plugin reads is read through a serializer of its own, named after
/// the variant's class. A slot is corrected in place by the enum's serializer, which reads it
/// itself, from the object without the tag under a tag alone. A slot of several is corrected by
/// the wrapper that reads the array.
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
        "label",
    );
    for held in [
        "@Serializable data class PortageBoxed(@Serializable(with = \
         PortageBoxedLabelPreprocess::class) val label: String) : PortageDraft {",
        "@Serializable data class PortageLone(val value: String) : PortageDraft {",
        "\"Boxed\" -> kotlinSchemaInner(input.json, \".detail\", serializer<PortageBoxed>(), \
         (data ?: throw SerializationException(\"missing field `detail`\"))); ",
        "\"Lone\" -> kotlinSchemaUnder(\".detail\") { PortageLone(kotlinSchemaDecoded(input.json, \
         serializer<String>(), trimmed((data ?: throw SerializationException(\"missing field \
         `detail`\"))), \"detail\", \"\")) }; ",
        "(kotlinSchemaDecoded(input.json, serializer<String>(), trimmed(array[0]), \"0\", \
         \"[0]\"), kotlinSchemaDecoded(input.json, KotlinNumber1, array[1], \"1\", \"[1]\")) } }",
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
        "\"Boxed\" -> LiftBoxed(kotlinSchemaDecoded(input.json, ParcelUnjudged, \
         unwrapped(JsonObject(obj - \"kind\")), \"Boxed\", \"\")); "
            .to_owned(),
        "\"Loose\" -> kotlinSchemaFlattened { kotlinSchemaInner(input.json, \"\", \
         serializer<LiftLoose>(), JsonObject(obj - \"kind\")) } else -> "
            .to_owned(),
        "data class LiftLoose(@Serializable(with = LiftLooseLabelPreprocess::class) val label: \
         String) : LiftDraft {"
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
         GuessCodedCodePreprocess::class) val code: String) : GuessDraft {",
        "@JvmInline @Serializable value class GuessShort(val value: String) : GuessDraft {",
        ".recoverCatching { input.json.decodeFromJsonElement(serializer<GuessCoded>(), element) \
         as GuessDraft }",
        ".recoverCatching { input.json.decodeFromJsonElement(serializer<GuessShort>(), \
         trimmed(element)).also { chosen -> ",
        ".recoverCatching { input.json.decodeFromJsonElement(serializer<GuessWhole>(), element) \
         as GuessDraft }",
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
         PickDraft<CodeType> {"
            .to_owned(),
        format!(
            "\"Many\" -> kotlinSchemaInner(input.json, \".detail\", \
             PickMany.serializer(codeTypeSerializer), {}); ",
            keyed(
                "(data ?: throw SerializationException(\"missing field `detail`\"))",
                "\"codes\" -> unwrapped(held)"
            )
        ),
        "\"One\" -> kotlinSchemaUnder(\".detail\") { PickOne(kotlinSchemaDecoded(input.json, \
         codeTypeSerializer, unwrapped((data ?: throw SerializationException(\"missing field \
         `detail`\"))), \"detail\", \"\")) } else -> "
            .to_owned(),
    ] {
        assert!(pick.contains(&held), "no `{held}` in: {pick}");
    }
    assert!(!pick.contains("Preprocess"), "got: {pick}");
}
