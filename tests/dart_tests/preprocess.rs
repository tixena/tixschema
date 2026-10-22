//! What the Dart backend writes for a `preprocess`: the unjudged read of each shape calls the
//! functions the type names on the payload it takes, and each member's on the payload of its key,
//! ahead of what is read from them. Each function answers its payload as it is here: the reads
//! are run under the Dart SDK in `tests/service_schema_emitted_client_tests/run_dart_preprocess.rs`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What a judged read does with the value its unjudged read answered.
const JUDGED: &str = "final violations = read.validate(); if (violations.isNotEmpty) throw \
     FormatException(violations.join('; ')); return read; }";

/// The parcel of the documents: a function on the type, and one on a field held to a bound.
#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
}

/// A type that names no function, and holds one that does.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

/// One member per kind of value a function is written on.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub code_list: Vec<String>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub held_parcel: Parcel,
    #[model_schema_prop(preprocess = ["unwrapped", "trimmed"])]
    pub item_label: String,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub seat_pair: (String, u32),
    #[model_schema_prop(preprocess = ["trimmed"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_note: Option<String>,
}

/// A unit struct.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClosedDock;

/// A tuple struct, one slot of which names a function.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stack(
    #[model_schema_prop(preprocess = ["trimmed"])] pub String,
    pub u32,
);

/// A brand with two functions.
#[model_schema(preprocess = ["unwrapped", "trimmed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Label(pub String);

/// A brand with a parameter.
#[model_schema(default_types(IdType = String), preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Tag<IdType>(pub IdType);

/// A type with a parameter.
#[model_schema(default_types(CodeType = String), preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(preprocess = ["trimmed"])]
    pub label: String,
}

/// An enum of unit variants.
#[model_schema(preprocess = ["trimmed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Grade {
    Express,
    Standard,
}

/// An enum of unit variants that names a check as well.
#[model_schema(check = ["rated"], preprocess = ["trimmed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Rating {
    Express,
    Unrated,
}

/// An enum serde writes under its variant's name.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Transfer {
    Held,
    Rail(#[model_schema_prop(preprocess = ["trimmed"])] String),
    Road {
        #[model_schema_prop(preprocess = ["trimmed"])]
        plate: String,
    },
    Split(#[model_schema_prop(preprocess = ["trimmed"])] String, u32),
}

/// A type whose function is handed an object, and one with no function.
#[model_schema(preprocess = ["withItsKeys"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tagged {
    pub keys: Vec<String>,
    pub tag: String,
}

/// The members of [`Tagged`], with no function on the type.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Listed {
    pub keys: Vec<String>,
    pub tag: String,
}

/// An enum under a tag key: a variant that flattens a type, and one whose slot names a function.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Shipment {
    Air {
        dock: String,
        #[serde(flatten)]
        tagged: Tagged,
    },
    Boxed(#[model_schema_prop(preprocess = ["withItsKeys"])] Listed),
    Ground,
}

/// An enum under a tag key and a content key.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Handover {
    Left(#[model_schema_prop(preprocess = ["trimmed"])] String),
    Signed { by: String },
}

/// An enum serde writes as the value of its variant, each member under a bound.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Reference {
    Named {
        #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
        name: String,
    },
    Numbered(#[model_schema_prop(minimum = 1, preprocess = ["unwrapped"])] u32),
}

/// A type flattened beside a member of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pinned {
    pub dock: String,
    #[serde(flatten)]
    pub tagged: Tagged,
}

/// A flattened member that names a function, between two members of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sorted {
    pub dock: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub listed: Listed,
    pub pier: String,
}

/// A type flattened into a holder with no member of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Account {
    #[serde(flatten)]
    pub tagged: Tagged,
}

/// A class whose functions are named after the two names its read declares.
#[model_schema(preprocess = ["payload"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Renamed {
    #[model_schema_prop(preprocess = ["json"])]
    pub label: String,
}

/// An enum whose functions are named after the two names its dispatch declares.
#[model_schema(preprocess = ["tag"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Routed {
    Held,
    Rail(#[model_schema_prop(preprocess = ["map"])] String),
}

/// An untagged enum whose function is named after the name its attempt declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Counted {
    Text(String),
    Whole(#[model_schema_prop(minimum = 1, preprocess = ["read"])] u32),
}

/// A tuple struct whose slot's function is named after the name its tuple's read declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Coupled(
    #[model_schema_prop(preprocess = ["tuple"])] pub String,
    pub u32,
);

/// A type with a parameter whose function is named after the converter of the parameter.
#[model_schema(default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Keyed<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(preprocess = ["codeTypeFromJson"])]
    pub label: String,
}

const fn code_type_from_json(payload: Value) -> Value {
    payload
}

const fn json(payload: Value) -> Value {
    payload
}

const fn map(payload: Value) -> Value {
    payload
}

const fn payload(payload: Value) -> Value {
    payload
}

fn rated(value: &Rating) -> Result<(), Vec<String>> {
    if matches!(value, Rating::Unrated) {
        return Err(vec!["a parcel is rated".to_owned()]);
    }
    Ok(())
}

const fn read(payload: Value) -> Value {
    payload
}

const fn tag(payload: Value) -> Value {
    payload
}

const fn trimmed(payload: Value) -> Value {
    payload
}

const fn tuple(payload: Value) -> Value {
    payload
}

const fn unwrapped(payload: Value) -> Value {
    payload
}

const fn with_its_keys(payload: Value) -> Value {
    payload
}

const fn without_fragile(payload: Value) -> Value {
    payload
}

/// The judged read of `named` and the head of its unjudged one, which takes its payload as a
/// `kind` under `taken`, each with `converters` after the payload and `handed` after `json`.
fn pair(named: &str, kind: &str, taken: &str, converters: &str, handed: &str) -> String {
    format!(
        "factory {named}.fromJson({kind} json{converters}) {{ final read = \
         {named}.fromJsonUnjudged(json{handed}); {JUDGED} factory \
         {named}.fromJsonUnjudged({kind} {taken}{converters}) "
    )
}

/// The pair of a type with no parameter.
fn plain_pair(named: &str, kind: &str, taken: &str) -> String {
    pair(named, kind, taken, "", "")
}

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

#[test]
fn every_declared_type_is_constructible() {
    let texts =
        |written: &[&str]| -> Vec<String> { written.iter().map(|&text| text.to_owned()).collect() };
    let parcel = Parcel {
        fragile_item_codes: Vec::new(),
        item_code_list: texts(&["x"]),
        label: "A".to_owned(),
    };
    let manifest = Manifest {
        parcel: parcel.clone(),
    };
    assert_eq!(manifest.parcel.item_code_list, ["x"]);
    assert_eq!(manifest.parcel.fragile_item_codes, Vec::<String>::new());
    let ticket = Ticket {
        code_list: texts(&["x"]),
        held_parcel: parcel,
        item_label: "A".to_owned(),
        seat_pair: ("a".to_owned(), 1),
        short_note: None,
    };
    assert_eq!(ticket.held_parcel.label, "A");
    assert_eq!(ticket.code_list.len() + ticket.item_label.len(), 2);
    assert_eq!((ticket.seat_pair.1, ticket.short_note), (1, None));
    assert_eq!(serde_json::to_string(&ClosedDock).unwrap(), "{}");
    let stack = Stack("a".to_owned(), 2);
    assert_eq!((stack.0.as_str(), stack.1), ("a", 2));
    assert_eq!(Label("A".to_owned()).0, "A");
    assert_eq!(Tag(7_u32).0, 7);
    let filled = Crate {
        codes: vec![7_u32],
        label: "A".to_owned(),
    };
    assert_eq!((filled.codes.len(), filled.label.as_str()), (1, "A"));
    assert_eq!(
        serde_json::to_string(&[Grade::Express, Grade::Standard]).unwrap(),
        r#"["Express","Standard"]"#
    );
    assert_eq!(
        serde_json::to_string(&[Rating::Express, Rating::Unrated]).unwrap(),
        r#"["Express","Unrated"]"#
    );
    assert_eq!(rated(&Rating::Express), Ok(()));
    assert_eq!(rated(&Rating::Unrated).unwrap_err(), ["a parcel is rated"]);
    let tagged = Tagged {
        keys: Vec::new(),
        tag: "t".to_owned(),
    };
    let listed = Listed {
        keys: Vec::new(),
        tag: "t".to_owned(),
    };
    let pinned = Pinned {
        dock: "north".to_owned(),
        tagged: tagged.clone(),
    };
    assert_eq!(
        (pinned.dock.as_str(), pinned.tagged.tag.as_str()),
        ("north", "t")
    );
    let sorted = Sorted {
        dock: "north".to_owned(),
        listed: listed.clone(),
        pier: "p".to_owned(),
    };
    assert_eq!(sorted.dock.len() + sorted.pier.len(), 6);
    assert_eq!(sorted.listed.keys, Vec::<String>::new());
    assert_eq!(Account { tagged }.tagged.keys, Vec::<String>::new());
    assert_eq!(listed.tag, "t");
    assert_eq!(
        Renamed {
            label: "A".to_owned()
        }
        .label,
        "A"
    );
    let keyed = Keyed {
        codes: vec![7_u32],
        label: "A".to_owned(),
    };
    assert_eq!((keyed.codes.len(), keyed.label.as_str()), (1, "A"));
    let coupled = Coupled("A".to_owned(), 7);
    assert_eq!((coupled.0.as_str(), coupled.1), ("A", 7));
}

#[test]
fn every_declared_variant_is_constructible() {
    let transfers = [
        Transfer::Held,
        Transfer::Rail("north".to_owned()),
        Transfer::Road {
            plate: "AB 1".to_owned(),
        },
        Transfer::Split("a".to_owned(), 2),
    ];
    assert_eq!(transfers.len(), 4);
    let tagged = Tagged {
        keys: Vec::new(),
        tag: "t".to_owned(),
    };
    let shipments = [
        Shipment::Air {
            dock: "north".to_owned(),
            tagged,
        },
        Shipment::Boxed(Listed {
            keys: Vec::new(),
            tag: "t".to_owned(),
        }),
        Shipment::Ground,
    ];
    assert_eq!(shipments.len(), 3);
    let handovers = [
        Handover::Left("door".to_owned()),
        Handover::Signed { by: "A".to_owned() },
    ];
    assert_eq!(handovers.len(), 2);
    let references = [
        Reference::Named {
            name: "north".to_owned(),
        },
        Reference::Numbered(7),
    ];
    assert_eq!(references.len(), 2);
    let routed = [Routed::Held, Routed::Rail("north".to_owned())];
    assert_eq!(routed.len(), 2);
    let counted = [Counted::Text("a".to_owned()), Counted::Whole(7)];
    assert_eq!(counted.len(), 2);
}

/// The sample of the documents: the type's function takes the payload the read is handed, and the
/// members are read from what it answers. The field's function is called where the payload holds
/// the key, and the judged read is the text it was.
#[test]
fn a_class_calls_its_function_on_its_payload_and_a_fields_on_its_key() {
    assert_written(
        &parcel_dart::dart_definition(),
        &format!(
            "{}{{ final json = withoutFragile(payload) as Map<String, dynamic>; return \
             Parcel(fragile_item_codes: (json['fragileItemCodes'] as List<dynamic>).map((e) => e \
             as String).toList(),item_code_list: (json['itemCodeList'] as List<dynamic>).map((e) \
             => e as String).toList(),label: (json.containsKey('label') ? trimmed(json['label']) : \
             null) as String,); }} Map<String, dynamic> toJson() =>",
            plain_pair("Parcel", "Map<String, dynamic>", "payload")
        ),
    );
}

/// A holder names no function: it reads the model through the model's own unjudged read, which
/// calls the model's functions, and its text is what it was.
#[test]
fn a_holder_reads_a_model_through_the_models_own_read() {
    assert_written(
        &manifest_dart::dart_definition(),
        &format!(
            "{}=> Manifest(parcel: Parcel.fromJsonUnjudged(json['parcel']),); Map<String, \
             dynamic> toJson() =>",
            plain_pair("Manifest", "Map<String, dynamic>", "json")
        ),
    );
}

/// The member's own read is written around the corrected payload, whatever the member holds.
/// Several functions run in the order written, each over the answer of the one before. A read
/// that names its payload twice holds the corrected one under a name, so each function is called
/// once.
#[test]
fn a_fields_function_is_called_ahead_of_the_fields_read_whatever_it_holds() {
    assert_written(
        &ticket_dart::dart_definition(),
        &format!(
            "{}=> Ticket(code_list: ((json.containsKey('codeList') ? unwrapped(json['codeList']) \
             : null) as List<dynamic>).map((e) => e as String).toList(),held_parcel: \
             Parcel.fromJsonUnjudged((json.containsKey('heldParcel') ? \
             unwrapped(json['heldParcel']) : null)),item_label: (json.containsKey('itemLabel') ? \
             trimmed(unwrapped(json['itemLabel'])) : null) as String,seat_pair: switch \
             ((json.containsKey('seatPair') ? unwrapped(json['seatPair']) : null) as \
             List<dynamic>) {{ final tuple when tuple.length == 2 => (tuple[0] as String, tuple[1] \
             as int), final tuple => throw FormatException('invalid length ${{tuple.length}}, \
             expected 2') }},short_note: switch (json.containsKey('shortNote') ? \
             trimmed(json['shortNote']) : null) {{ final corrected => (corrected == null ? null : \
             corrected as String) }},); Map<String, dynamic> toJson() =>",
            plain_pair("Ticket", "Map<String, dynamic>", "json")
        ),
    );
}

/// A unit struct reads no member, so it holds what its function answers under no name.
#[test]
fn a_unit_struct_calls_its_function_on_its_payload() {
    assert_written(
        &closed_dock_dart::dart_definition(),
        &format!(
            "{}{{ unwrapped(payload) as Map<String, dynamic>; return ClosedDock(); }} Map<String, \
             dynamic> toJson() =>",
            plain_pair("ClosedDock", "Map<String, dynamic>", "payload")
        ),
    );
}

/// A slot is held by every payload that is read, so its function is called with no test.
#[test]
fn a_tuple_struct_calls_its_function_on_its_payload_and_a_slots_on_its_position() {
    assert_written(
        &stack_dart::dart_definition(),
        &format!(
            "{}{{ final json = unwrapped(payload); return Stack(switch (json as List<dynamic>) \
             {{ final tuple when tuple.length == 2 => (trimmed(tuple[0]) as String, tuple[1] as \
             int), final tuple => throw FormatException('invalid length ${{tuple.length}}, \
             expected 2') }}); }} dynamic toJson() =>",
            plain_pair("Stack", "dynamic", "payload")
        ),
    );
}

#[test]
fn a_brand_calls_its_functions_in_the_order_written() {
    assert_written(
        &label_dart::dart_definition(),
        &format!(
            "{}{{ final json = trimmed(unwrapped(payload)); return Label(json as String); }} \
             dynamic toJson() => value;",
            plain_pair("Label", "dynamic", "payload")
        ),
    );
}

/// A type with a parameter takes its converters after the payload in both reads, and calls its
/// function once, whatever fills it.
#[test]
fn a_type_with_a_parameter_calls_its_function_at_every_filling() {
    let id_converter = ", IdType Function(dynamic) idTypeFromJson";
    assert_written(
        &tag_dart::dart_definition(),
        &format!(
            "{}{{ final json = unwrapped(payload); return Tag(idTypeFromJson(json)); }} dynamic \
             toJson(",
            pair(
                "Tag",
                "dynamic",
                "payload",
                id_converter,
                ", idTypeFromJson"
            )
        ),
    );
    let code_converter = ", CodeType Function(dynamic) codeTypeFromJson";
    assert_written(
        &crate_dart::dart_definition(),
        &format!(
            "{}{{ final json = unwrapped(payload) as Map<String, dynamic>; return Crate(codes: \
             (json['codes'] as List<dynamic>).map((e) => codeTypeFromJson(e)).toList(),label: \
             (json.containsKey('label') ? trimmed(json['label']) : null) as String,); }} \
             Map<String, dynamic> toJson(",
            pair(
                "Crate",
                "Map<String, dynamic>",
                "payload",
                code_converter,
                ", codeTypeFromJson"
            )
        ),
    );
}

/// A plain enum takes the text serde writes for it. One that names no check has nothing to
/// judge, and its `fromJson` is its unjudged read.
#[test]
fn a_plain_enum_calls_its_function_on_the_text_it_takes() {
    assert_written(
        &grade_dart::dart_definition(),
        "static Grade fromJson(String json) => fromJsonUnjudged(json); static Grade \
         fromJsonUnjudged(String payload) { final json = trimmed(payload) as String; return \
         switch (json) { 'Express' => Grade.express,'Standard' => Grade.standard, _ => throw \
         FormatException('Unknown Grade: ' + json) }; } String toJson() => wireValue;",
    );
    assert_written(
        &rating_dart::dart_definition(),
        &format!(
            "static Rating fromJson(String json) {{ final read = Rating.fromJsonUnjudged(json); \
             {JUDGED} static Rating fromJsonUnjudged(String payload) {{ final json = \
             trimmed(payload) as String; return switch (json) {{ 'Express' => \
             Rating.express,'Unrated' => Rating.unrated, _ => throw FormatException('Unknown \
             Rating: ' + json) }}; }} String toJson() => wireValue;"
        ),
    );
}

/// The base of a sealed enum calls the enum's function once, ahead of its dispatch, and each
/// variant reads its own members: a slot from the value under the variant's name, a named member
/// from its key.
#[test]
fn an_enum_under_its_variants_name_calls_its_function_in_its_base() {
    let transfer = transfer_dart::dart_definition();
    assert_written(
        &transfer,
        &format!(
            "{}{{ final json = unwrapped(payload); if (json is String) {{ switch (json) {{ case \
             'Held': return TransferHeld(); default: throw FormatException('Unknown Transfer: ' + \
             json); }} }} final map = json as Map<String, dynamic>; if (map.isEmpty) {{ throw \
             FormatException('empty object for Transfer'); }} if (map.length > 1) {{ throw \
             FormatException('more than one key for Transfer'); }} final tag = map.keys.first; \
             switch (tag) {{ case 'Held': return map[tag] == null ? TransferHeld() : throw \
             FormatException('Transfer: ' + tag + ' takes no value'); case 'Rail': return \
             TransferRail(trimmed(map['Rail']) as String); case 'Road': return \
             TransferRoad.fromJsonUnjudged(map['Road']); case 'Split': return \
             TransferSplit(switch (map['Split'] as List<dynamic>) {{ final tuple when \
             tuple.length == 2 => (trimmed(tuple[0]) as String, tuple[1] as int), final tuple => \
             throw FormatException('invalid length ${{tuple.length}}, expected 2') }}); default: \
             throw FormatException('Unknown Transfer: ' + tag); }} }} dynamic toJson();",
            plain_pair("Transfer", "dynamic", "payload")
        ),
    );
    assert_written(
        &transfer,
        &format!(
            "{}=> TransferRoad(plate: (json.containsKey('plate') ? trimmed(json['plate']) : null) \
             as String,); @override dynamic toJson() =>",
            plain_pair("TransferRoad", "Map<String, dynamic>", "json")
        ),
    );
}

/// Under a tag key serde hands a variant the object without the tag. A slot's function is handed
/// that, and a type flattened into a variant the keys neither the tag nor the variant's own
/// members take.
#[test]
fn an_enum_under_a_tag_key_hands_a_function_the_object_without_the_tag() {
    let shipment = shipment_dart::dart_definition();
    assert_written(
        &shipment,
        &format!(
            "{}{{ final json = unwrapped(payload) as Map<String, dynamic>; final tag = \
             json['kind'] as String; switch (tag) {{ case 'Air': return \
             ShipmentAir.fromJsonUnjudged(json); case 'Boxed': return \
             ShipmentBoxed(Listed.fromJsonUnjudged(withItsKeys(({{...json}}..remove('kind'))))); \
             case 'Ground': return ShipmentGround(); default: throw FormatException('Unknown \
             Shipment: ' + tag); }} }} dynamic toJson();",
            plain_pair("Shipment", "Map<String, dynamic>", "payload")
        ),
    );
    assert_written(
        &shipment,
        &format!(
            "{}=> ShipmentAir(dock: json['dock'] as String,tagged: \
             Tagged.fromJsonUnjudged(({{...json}}..remove('kind')..remove('dock'))),); \
             Map<String, dynamic> toJson() =>",
            plain_pair("ShipmentAir", "Map<String, dynamic>", "json")
        ),
    );
}

/// A slot under a content key is read from that key, and its function is called where the
/// payload holds it.
#[test]
fn an_enum_under_a_content_key_calls_a_slots_function_where_the_key_is_held() {
    assert_written(
        &handover_dart::dart_definition(),
        &format!(
            "{}{{ final json = unwrapped(payload) as Map<String, dynamic>; final tag = \
             json['kind'] as String; switch (tag) {{ case 'Left': return \
             HandoverLeft((json.containsKey('detail') ? trimmed(json['detail']) : null) as \
             String); case 'Signed': return HandoverSigned.fromJsonUnjudged(json['detail']); \
             default: throw FormatException('Unknown Handover: ' + tag); }} }} dynamic toJson();",
            plain_pair("Handover", "Map<String, dynamic>", "payload")
        ),
    );
}

/// An untagged enum calls its own function once, ahead of every attempt, and each attempt calls
/// the functions of the variant it tries.
#[test]
fn an_untagged_enum_calls_its_function_ahead_of_every_attempt() {
    let reference = reference_dart::dart_definition();
    assert_written(
        &reference,
        &format!(
            "{}{{ final json = unwrapped(payload); try {{ final read = \
             ReferenceNamed.fromJsonUnjudged(json as Map<String, dynamic>); if \
             (read._bounds().isEmpty) return read; }} catch (_) {{}} try {{ final read = \
             ReferenceNumbered(unwrapped(json) as int); if (read._bounds().isEmpty) return read; \
             }} catch (_) {{}} throw FormatException('No variant of Reference matched'); }} \
             dynamic toJson();",
            plain_pair("Reference", "dynamic", "payload")
        ),
    );
    assert_written(
        &reference,
        &format!(
            "{}=> ReferenceNamed(name: (json.containsKey('name') ? trimmed(json['name']) : null) \
             as String,); Map<String, dynamic> toJson() =>",
            plain_pair("ReferenceNamed", "Map<String, dynamic>", "json")
        ),
    );
}

/// A flattened member is read from the keys the members beside it do not take, as serde hands
/// them, and a function it names is handed those keys. A holder with no member of its own hands
/// on the payload it took.
#[test]
fn a_flattened_member_is_read_from_the_keys_its_holder_does_not_take() {
    assert_written(
        &pinned_dart::dart_definition(),
        &format!(
            "{}=> Pinned(dock: json['dock'] as String,tagged: \
             Tagged.fromJsonUnjudged(({{...json}}..remove('dock'))),); Map<String, dynamic> \
             toJson() =>",
            plain_pair("Pinned", "Map<String, dynamic>", "json")
        ),
    );
    assert_written(
        &sorted_dart::dart_definition(),
        &format!(
            "{}=> Sorted(dock: json['dock'] as String,listed: \
             Listed.fromJsonUnjudged(withItsKeys(({{...json}}..remove('dock')..remove('pier')))),pier: \
             json['pier'] as String,); Map<String, dynamic> toJson() =>",
            plain_pair("Sorted", "Map<String, dynamic>", "json")
        ),
    );
    assert_written(
        &account_dart::dart_definition(),
        &format!(
            "{}=> Account(tagged: Tagged.fromJsonUnjudged(json),); Map<String, dynamic> toJson() \
             =>",
            plain_pair("Account", "Map<String, dynamic>", "json")
        ),
    );
}

/// A name the read declares is numbered where a function is called under it, so the call
/// reaches the function. The judged read calls none, and keeps its names but that of a
/// converter, which it hands on.
#[test]
fn a_name_the_read_declares_is_numbered_where_a_function_is_called_under_it() {
    assert_written(
        &renamed_dart::dart_definition(),
        &format!(
            "{}{{ final json2 = payload(payload2) as Map<String, dynamic>; return Renamed(label: \
             (json2.containsKey('label') ? json(json2['label']) : null) as String,); }} \
             Map<String, dynamic> toJson() =>",
            plain_pair("Renamed", "Map<String, dynamic>", "payload2")
        ),
    );
    assert_written(
        &routed_dart::dart_definition(),
        &format!(
            "{}{{ final json = tag(payload); if (json is String) {{ switch (json) {{ case 'Held': \
             return RoutedHeld(); default: throw FormatException('Unknown Routed: ' + json); }} }} \
             final map2 = json as Map<String, dynamic>; if (map2.isEmpty) {{ throw \
             FormatException('empty object for Routed'); }} if (map2.length > 1) {{ throw \
             FormatException('more than one key for Routed'); }} final tag2 = map2.keys.first; \
             switch (tag2) {{ case 'Held': return map2[tag2] == null ? RoutedHeld() : throw \
             FormatException('Routed: ' + tag2 + ' takes no value'); case 'Rail': return \
             RoutedRail(map(map2['Rail']) as String); default: throw FormatException('Unknown \
             Routed: ' + tag2); }} }} dynamic toJson();",
            plain_pair("Routed", "dynamic", "payload")
        ),
    );
    assert_written(
        &counted_dart::dart_definition(),
        &format!(
            "{}{{ try {{ return CountedText(json as String); }} catch (_) {{}} try {{ final read2 \
             = CountedWhole(read(json) as int); if (read2._bounds().isEmpty) return read2; }} \
             catch (_) {{}} throw FormatException('No variant of Counted matched'); }} dynamic \
             toJson();",
            plain_pair("Counted", "dynamic", "json")
        ),
    );
    assert_written(
        &coupled_dart::dart_definition(),
        &format!(
            "{}=> Coupled(switch (json as List<dynamic>) {{ final tuple2 when tuple2.length == 2 \
             => (tuple(tuple2[0]) as String, tuple2[1] as int), final tuple2 => throw \
             FormatException('invalid length ${{tuple2.length}}, expected 2') }});",
            plain_pair("Coupled", "dynamic", "json")
        ),
    );
    assert_written(
        &keyed_dart::dart_definition(),
        &format!(
            "{}=> Keyed(codes: (json['codes'] as List<dynamic>).map((e) => \
             codeTypeFromJson2(e)).toList(),label: (json.containsKey('label') ? \
             codeTypeFromJson(json['label']) : null) as String,); Map<String, dynamic> \
             toJson(dynamic Function(CodeType) codeTypeToJson) =>",
            pair(
                "Keyed",
                "Map<String, dynamic>",
                "json",
                ", CodeType Function(dynamic) codeTypeFromJson2",
                ", codeTypeFromJson2"
            )
        ),
    );
}
