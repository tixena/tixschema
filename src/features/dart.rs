//! Dart type and JSON-codec generation.

mod forms;

use core::cell::{Cell, RefCell};
use core::fmt::Write as _;
use core::iter::once;
use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt as _;
use syn::{Field, Fields, Ident, Item, ItemEnum, ItemStruct, ItemType, Variant};

use crate::bound_message::{Bound, OWN_BOUND_FIRST, dart_repeated, dart_violation};
use crate::features::model_schema_prop::{FunctionName, LiteralValue, ModelSchemaPropMeta};
use crate::features::serde::defaults::{SerdeDefaults, substituted};
use crate::features::serde::{
    brand_field, has_serde_skip_serializing, keys_taken_when_flattened, parse_serde_key_omission,
    struct_tag, tag_and_content_when_flattened, variant_names_when_flattened, variants_on_the_wire,
    written_slots,
};
use crate::features::text_forms::TextForm;
use crate::field_type::{
    FieldDef, FieldDefType, RESULT_ARMS, RangeKind, VariantKind, dart_from_json_argument,
    dart_lower_camel, dart_to_json_argument, format_number_literal, get_field_def, is_plain_enum,
    is_sequence_wrapper,
};
use crate::model_schema::variant_wire_kind;
use crate::rename_rule::{RenameRule, resolve_rename_rule};
use crate::utils::{
    MapKeyWire, compute_alias_export_name, compute_item_export_name, to_snake_case,
    type_parameters_in_scope,
};

#[cfg(feature = "serde")]
use crate::features::serde::{parse_serde_field_attributes, parse_serde_type_attributes};

/// The helper a type's `validate()` reads the text of a named type with: a string as it is, and a
/// wrapper through the value it holds.
const BOUND_TEXT_HELPER: &str = "static String _boundText(Object held) => held is String ? held : \
     _boundText((held as dynamic).value as Object);";

/// What a member of an emitted enum cannot be named beside the reserved words: a member every
/// Dart enum or object already has, and one the emitted enum declares itself.
const DART_ENUM_NAMES_TAKEN: [&str; 11] = [
    "fromJson",
    "fromJsonUnjudged",
    "hashCode",
    "index",
    "noSuchMethod",
    "runtimeType",
    "toJson",
    "toString",
    "validate",
    "values",
    "wireValue",
];

/// What a field of an emitted class cannot be named beside the reserved words: a type its codec is
/// written with, or a member the class has. Only names that never compiled are listed: a field
/// and a constructor share a name, so `fromJson` and `fromJsonUnjudged` are not.
const DART_FIELD_NAMES_TAKEN: [&str; 6] = [
    "dynamic",
    "noSuchMethod",
    "runtimeType",
    "toJson",
    "toString",
    "validate",
];

/// The words Dart reserves, which have no escape. A built-in identifier such as `get` or `import`
/// is not among them: Dart takes one as a member's name.
const DART_RESERVED: [&str; 33] = [
    "assert", "break", "case", "catch", "class", "const", "continue", "default", "do", "else",
    "enum", "extends", "false", "final", "finally", "for", "if", "in", "is", "new", "null",
    "rethrow", "return", "super", "switch", "this", "throw", "true", "try", "var", "void", "while",
    "with",
];

/// The helper a variant of an untagged enum asks a value it holds for the bound of its own with:
/// a brand that carries one answers `validateBound()`, and any other value has none.
const HELD_BOUND_HELPER: &str = "static List<String> _heldBound(Object held) { try { return \
     (held as dynamic).validateBound() as List<String>; } on NoSuchMethodError { return const \
     []; } }";

/// The helper a type's `validate()` quotes a `double` back with: a whole one without `.0`, as
/// Rust and Zod write it.
const NUMBER_HELPER: &str = "static String _number(double value) => value == \
     value.truncateToDouble() ? value.toStringAsFixed(0) : value.toString();";

/// The helper a type's `validate()` asks whether a set repeats an item with: two items are one
/// where they write one JSON text.
const REPEATS_HELPER: &str = "static bool _repeats(Iterable<Object?> items) { final seen = \
     <String>{}; for (final item in items) { if (!seen.add(jsonEncode(item))) return true; } \
     return false; }";

/// The helper a type's `validate()` writes a place in front of a message with: into the place the
/// message already names, or ahead of one that names none.
const UNDER_HELPER: &str = "static String _under(String place, String violation) { if \
     (violation.startsWith(\"'\")) { final end = violation.indexOf(\"'\", 1); if (end > 0) return \
     \"'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}\"; } return \
     \"'$place': $violation\"; }";

/// What the base of a sealed enum declares, which each of its variants overrides.
const VALIDATE_DECLARED: &str = "List<String> validate();";

/// One field this module has decided belongs on the wire: its Dart member name (the Rust field's
/// own spelling, `snake_case` included, rather than re-cased to Dart's own lower-camel
/// convention, and moved by [`dart_member`] off a name Dart will not take), its wire name, whether
/// the key always reaches the wire, whether it is a `#[serde(flatten)]` source, and the `FieldDef`
/// describing its type.
struct DartField {
    /// The names serde reads the member under beside its wire name.
    aliases: Vec<String>,
    /// Where the JSON of the default serde reads for an absent key is written.
    default_at: Option<String>,
    field_def: FieldDef,
    flatten: bool,
    member: String,
    /// Whether serde never writes the member: it is under `skip_serializing`.
    never_written: bool,
    /// The `Option` a flattened alias of one is read and written as, its value held in the
    /// alias's class.
    read_as: Option<FieldDef>,
    required: bool,
    wire_name: String,
}

impl DartField {
    /// The member as it is read and written flattened: the `Option` an alias of one names.
    fn wire_def(&self) -> &FieldDef {
        self.read_as.as_ref().unwrap_or(&self.field_def)
    }
}

/// The container attributes read off any enum, in the shape `process_enum` itself dispatches on —
/// `tag`/`content`/`untagged` default to "written none of them" without the `serde` feature, which
/// is what leaves an all-unit enum publishing the plain string-union shape regardless.
struct EnumTagAttrs {
    content: Option<String>,
    rename_all: Option<String>,
    rename_all_fields: Option<String>,
    tag: Option<String>,
    untagged: bool,
}

/// One tagged/untagged variant's payload, classified once so every builder below reads it the same
/// way. A `TupleMultiple` payload is folded into one `Tuple` [`FieldDef`], matching how a slot list
/// renders everywhere else in this module.
enum VariantPayload {
    Named(Vec<DartField>),
    Unit,
    Value(Box<FieldDef>),
}

/// What the class [`value_wrapper_tokens`] writes holds as its `value`.
enum Held<'held> {
    /// The slots of a tuple struct that is no brand and declares no slot or several, read from and
    /// written as the array serde writes them in, an empty one for a struct of no slot.
    Slots(&'held [FieldDef]),
    /// A value, read and written as its type is: a brand's, an alias's, and the one slot of a
    /// tuple struct that declares one, through the functions it names.
    Value(&'held FieldDef),
}

/// The generic converter parameters a `fromJson`/`toJson` pair threads through, beyond its own
/// primary argument — one pair per type parameter, since a Dart function value cannot itself be
/// generic the way the class it decodes can be. Shared by every generic struct, branded newtype,
/// alias, and tagged or untagged enum: a subclass extending a generic sealed base is itself generic
/// at the same parameters, whether or not its own payload happens to use one, so it carries the
/// same converters even when its own body never calls them.
struct GenericCodec {
    /// `, tFromJson` for each parameter: appended at a call site, after another `fromJson`'s own
    /// `json` argument.
    from_json_args: String,
    /// `, T Function(dynamic) tFromJson` for each parameter: appended in a declaration, after
    /// `fromJson`'s own `json` parameter.
    from_json_params: String,
    /// `dynamic Function(T) tToJson` for each parameter, joined by `, `: the whole parameter list
    /// of `toJson`.
    to_json_params: String,
}

/// The tag and content keys a discriminated enum's own `fromJson`/`toJson` dispatch turns on: which
/// object the payload sits in, and how the wire's own tag string is read.
struct TagShape<'shape> {
    content_key: Option<&'shape str>,
    merge_tag_into_object: bool,
    tag_key: &'shape str,
}

/// The pieces of a Dart class body built from a set of [`DartField`]s, before they are joined into
/// a `toJson` method one way or wrapped into one the other way — see `class_body_content` (the
/// plain join) and `tagged_variant_tokens` (which wraps [`Self::to_json_map`] under a tag key
/// instead, for a `Named` payload under adjacent or external tagging).
struct ClassBodyParts {
    ctor_params: String,
    field_decls: String,
    /// The `{ 'a': a, 'b': b }` map literal a plain `toJson()` returns, apart from the signature so
    /// a tagged variant can wrap it under a tag key.
    to_json_map: String,
}

impl ClassBodyParts {
    fn ctor_parameter_group(&self) -> String {
        if self.ctor_params.is_empty() {
            String::new()
        } else {
            format!("{{{}}}", self.ctor_params)
        }
    }
}

/// What the Dart backend reads of an item's `#[model_schema(...)]`: the name it publishes under,
/// its checks, its `preprocess`, the string check of a brand or of an enum, and how a member's
/// own keys are read.
pub struct DartTypeArgs<'args> {
    /// The functions `check` names, in the order written.
    pub check: &'args [FunctionName],
    /// The default type declared per type parameter, which the string check of a brand over one
    /// of its own parameters holds.
    pub default_types: &'args [(Ident, syn::Type)],
    pub max_length: Option<usize>,
    /// A member's `model_schema_prop`, held to its enum's string check where that reaches it.
    pub member_meta: &'args dyn Fn(&Field) -> ModelSchemaPropMeta,
    pub min_length: Option<usize>,
    pub name_override: Option<&'args str>,
    pub pattern: Option<&'args str>,
    /// The functions `preprocess` names on the type, in the order written.
    pub preprocess: &'args [FunctionName],
}

/// The bounds one value is held to, as `validate()` reports them: a length, then each pattern,
/// or a range.
struct Bounds<'held> {
    max_length: Option<usize>,
    maximum: Option<f64>,
    min_length: Option<usize>,
    minimum: Option<f64>,
    patterns: Vec<&'held str>,
}

impl<'held> Bounds<'held> {
    /// The bounds a member's `model_schema_prop` writes.
    fn of(meta: &'held ModelSchemaPropMeta) -> Self {
        Self {
            max_length: meta.max_length,
            maximum: meta.maximum,
            min_length: meta.min_length,
            minimum: meta.minimum,
            patterns: meta.patterns().collect(),
        }
    }
}

/// A helper a type's `validate()` or the bounds of a variant call.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Helper {
    BoundText,
    HeldBound,
    Number,
    Repeats,
    Under,
}

/// The helpers a type's `validate()` and the bounds of a variant call, each written into the
/// type that calls it.
#[derive(Default)]
struct Helpers {
    called: Vec<Helper>,
}

impl Helpers {
    /// Notes that `helper` is called.
    fn call(&mut self, helper: Helper) {
        if !self.calls(helper) {
            self.called.push(helper);
        }
    }

    fn calls(&self, helper: Helper) -> bool {
        self.called.contains(&helper)
    }
}

/// What a walk lists of the value a member holds.
#[derive(Clone, Copy)]
enum Asked {
    /// What chooses a variant of an untagged enum: the member's own bounds and the bound of
    /// each brand it holds, with no check and nothing a model reports.
    Bound,
    /// What `validate()` lists: the member's bounds, the `validate()` of each declared type it
    /// holds, then its checks.
    Rules,
}

/// How a declared type is read where a value holds it.
#[derive(Clone, Copy)]
enum Read {
    /// By its `fromJson`, which refuses a value its own `validate()` refuses.
    Judged,
    /// By its `fromJsonUnjudged`, where the `validate()` of what holds it runs its rules.
    Unjudged,
}

impl Read {
    /// The constructor of a declared type this read calls.
    const fn constructor(self) -> &'static str {
        match self {
            Self::Judged => "fromJson",
            Self::Unjudged => "fromJsonUnjudged",
        }
    }
}

/// One step of the place a message names.
#[derive(Clone)]
enum Step {
    /// A Dart expression a walk binds: the index of a list's item, or the key of a map's entry.
    Bound(String),
    /// A key or a position serde writes.
    Written(String),
}

/// One variant of a sealed enum as its subclass is written.
#[derive(Clone, Copy)]
struct VariantClass<'variant> {
    /// Whether serde reads the variant under an alias too.
    aliased: bool,
    payload: &'variant VariantPayload,
    subclass_name: &'variant str,
    /// The variant's `validate()` and its helpers, as [`validate_members`] writes them.
    validate: &'variant str,
    wire_tag: &'variant str,
}

/// What an unjudged read answers with.
enum Answer {
    /// Statements, the last of which returns or throws.
    Statements(String),
    /// One expression.
    Value(String),
}

/// What an unjudged read calls of the functions `preprocess` names on its type, and the names
/// it declares, each as [`declared`] writes it.
struct Corrects<'item> {
    /// What holds the payload the members are read from.
    json: String,
    /// What holds the payload of an enum serde writes under its variant's name.
    map: String,
    /// The functions the type names, in the order written.
    own: &'item [FunctionName],
    /// What holds the payload as it is taken, where the type names a function.
    payload: String,
    /// What holds the variant of an untagged enum whose bounds are asked.
    read: String,
    /// What holds the tag of an enum.
    tag: String,
}

impl<'item> Corrects<'item> {
    /// The read of a type that names `own`.
    fn of(own: &'item [FunctionName]) -> Self {
        Self {
            json: declared("json"),
            map: declared("map"),
            own,
            payload: declared("payload"),
            read: declared("read"),
            tag: declared("tag"),
        }
    }

    /// The read of a variant of the type: the same names, and no function of the type's own,
    /// which its base has called.
    fn of_a_variant(&self) -> Corrects<'static> {
        Corrects {
            json: self.json.clone(),
            map: self.map.clone(),
            own: &[],
            payload: self.payload.clone(),
            read: self.read.clone(),
            tag: self.tag.clone(),
        }
    }
}

/// What serde takes off a holder's object for a type flattened into it, which it hands no member
/// flattened after it.
#[derive(Clone)]
enum Taken {
    /// The one entry under one of the names, one list per variant: the variant an externally
    /// tagged enum is read from.
    FirstOf(Vec<Vec<String>>),
    /// Every entry under one of the keys: a struct's members.
    Keys(Vec<String>),
    /// Every entry under one of the keys, and no other: an `ObjectId`'s `$oid`, and a brand or an
    /// alias over one, whose read refuses any other key, as serde refuses one where the `ObjectId`
    /// is held as a value.
    Only(Vec<String>),
    /// Every entry under one of the keys of any other enum: the tag and the content key of one
    /// under both, which serde reads as a struct of the two, and none for every other.
    Tagged(Vec<String>),
}

/// How a type reads one of its type parameters flattened.
#[derive(Clone)]
enum Flattening {
    /// Flattened and nowhere else: the converter a holder hands for it is called with the object
    /// the type is read from, and with nothing else.
    Alone,
    /// Flattened and otherwise: its flattened reads call the converter the read takes under this
    /// name where a holder hands one, and the type parameter's own converter otherwise.
    Also(String),
    /// Not flattened.
    Never,
}

/// Whether the members of a type read one of its type parameters flattened, whether one reads it
/// any other way, and whether one reads it flattened under an `Option`, which is looked up by the
/// keys of what fills it.
#[derive(Default)]
struct ParameterReads {
    flattened: bool,
    looked_up: bool,
    plainly: bool,
}

/// The unjudged read of a type: the name it takes its payload under, and its body.
struct Unjudged {
    body: String,
    taken_as: String,
}

thread_local! {
    /// The Dart class/enum name each Rust ident publishes — the one thing a reference to a sibling
    /// item needs, since the reference's own field carries the type arguments (real Dart generics
    /// need no factory to bind them, unlike Zod). Independent of the three-surface `ALIAS_INFO`
    /// registry in `crate::utils`: Dart has no forward-reference or cycle problem to share
    /// bookkeeping over (whole-library resolution means declaration order never matters), so it
    /// stays its own small map.
    static DART_NAMES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());

    /// The fields of each struct a class has been emitted for, by their Rust names: what a
    /// client reading one member off a declared message needs to name it as the class does.
    static DART_FIELDS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());

    /// What serde takes off its holder's object for each struct and enum a class has been emitted
    /// for, read flattened.
    static DART_TAKEN_KEYS: RefCell<HashMap<String, Taken>> = RefCell::new(HashMap::new());

    /// How each type a class has been emitted for reads each of its type parameters flattened,
    /// itself or through a type that does, in the order declared.
    static DART_FLATTENINGS: RefCell<HashMap<String, Vec<Flattening>>> =
        RefCell::new(HashMap::new());

    /// How the item being emitted reads each of its type parameters flattened, by name.
    static DART_FLATTENING: RefCell<Vec<(String, Flattening)>> =
        const { RefCell::new(Vec::new()) };

    /// For each type a class has been emitted for, in the order its type parameters are declared,
    /// the name its read takes the keys of what fills one under, where it reads that parameter
    /// flattened under an `Option`, itself or through a type that does.
    static DART_LOOKED_UP: RefCell<HashMap<String, Vec<Option<String>>>> =
        RefCell::new(HashMap::new());

    /// The type parameters the item being emitted reads flattened under an `Option`, each with the
    /// name its read takes the keys of what fills it under.
    static DART_LOOKING: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };

    /// The type parameter a flattened member of the item being emitted is, while it is read.
    static DART_FLATTENED_READ: RefCell<Option<String>> = const { RefCell::new(None) };

    /// Whether each struct and enum a class has been emitted for hands the object it is read from
    /// to a function: the type names one, or a struct flattens a member that is handed one or whose
    /// type is not known to hand it to none.
    static DART_HANDED: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());

    /// The tag, then the content key where it has one, of each enum under a tag key a class has
    /// been emitted for: what a flattened `Option` of it is looked for by. Under a tag alone it
    /// takes no key off its holder's object, so these are no keys taken.
    static DART_TAG_KEYS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());

    /// Each externally tagged enum a class has been emitted for whose `toJson` writes a unit
    /// variant as its bare name: what a holder that flattens one writes as an entry over `null`.
    static DART_WRITES_A_NAME: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

    /// Each struct a class has been emitted for whose `toJson` is typed `Map<String, dynamic>`: a
    /// holder spreads what it writes with no cast. A wrapper's and an enum's answer `dynamic`.
    static DART_WRITES_AN_OBJECT: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

    /// Each brand and alias a class has been emitted for whose value is `()`, or another such
    /// type, and each untagged enum with a variant over one: its read takes the `null` an absent
    /// key reads as, which its holder refuses.
    static DART_READS_A_UNIT: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

    /// The name of each function the reads of the item being emitted call: what its `preprocess`
    /// names, and that of each of its members.
    static DART_CALLED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };

    /// The sealed class each shape of `Result` the item being emitted writes is declared as, by
    /// which of its arms holds `()`, and the declaration written after the item's own source. Every
    /// `Result` of one shape names one class.
    static DART_RESULTS: RefCell<Vec<([bool; 2], String, String)>> =
        const { RefCell::new(Vec::new()) };

    /// How many classes of a `Result` have been named, never reset: every one lands in the same
    /// consuming file, where no two may share a name.
    static DART_RESULT_COUNTER: Cell<u32> = const { Cell::new(0) };
}

/// The Dart tokens `item` earns, given what `args` reads of its `#[model_schema(...)]`, handed
/// over field by field rather than by widening that struct's visibility. Dispatches on the item's
/// own shape; an item this module has nothing to say about (anything but a struct, an enum or a
/// type alias) earns nothing.
pub fn dart_schema_dispatch(item: &Item, args: &DartTypeArgs<'_>) -> TokenStream {
    record_called(called_by(item, args));
    record_flattenings(item);
    let tokens = if let Item::Struct(item_struct) = item {
        struct_dart_tokens(item_struct, args)
    } else if let Item::Enum(item_enum) = item {
        enum_dart_tokens(item_enum, args)
    } else if let Item::Type(item_type) = item {
        alias_dart_tokens(item_type, args.name_override)
    } else {
        TokenStream::new()
    };
    record_called(Vec::new());
    DART_FLATTENING.with(|current| current.borrow_mut().clear());
    DART_LOOKING.with(|current| current.borrow_mut().clear());
    tokens
}

/// Records how `item` reads each of its type parameters flattened, for its own reads and for a
/// holder emitted after it: a member flattened that is the parameter itself, under an `Option` or
/// not, and a type argument that is the parameter, handed to a type that reads it flattened.
fn record_flattenings(item: &Item) {
    let (ident, generics, members) = if let Item::Struct(item_struct) = item {
        (
            &item_struct.ident,
            &item_struct.generics,
            read_members(item_struct.fields.iter()),
        )
    } else if let Item::Enum(item_enum) = item {
        (
            &item_enum.ident,
            &item_enum.generics,
            read_members(
                item_enum
                    .variants
                    .iter()
                    .flat_map(|variant| variant.fields.iter()),
            ),
        )
    } else if let Item::Type(item_type) = item {
        (
            &item_type.ident,
            &item_type.generics,
            vec![(false, item_type.ty.as_ref())],
        )
    } else {
        return;
    };
    let parameters = type_parameters_in_scope(generics);
    let mut looked_up: Vec<Option<String>> = Vec::new();
    let flattenings: Vec<(String, Flattening)> = parameters
        .iter()
        .map(|parameter| {
            let mut reads = ParameterReads::default();
            for &(flattened, ty) in &members {
                let mut def = get_field_def("", ty, "");
                def.erase_type_parameters(&parameters);
                parameter_reads(&def, parameter, flattened, &mut reads);
            }
            let flattening = match (reads.flattened, reads.plainly) {
                (true, false) => Flattening::Alone,
                (true, true) => Flattening::Also(declared(&format!(
                    "{}FromFlattenedJson",
                    dart_lower_camel(parameter)
                ))),
                (false, _) => Flattening::Never,
            };
            looked_up.push(
                reads
                    .looked_up
                    .then(|| declared(&format!("{}Keys", dart_lower_camel(parameter)))),
            );
            (parameter.clone(), flattening)
        })
        .collect();
    DART_FLATTENINGS.with(|recorded| {
        recorded.borrow_mut().insert(
            ident.to_string(),
            flattenings
                .iter()
                .map(|(_, flattening)| flattening.clone())
                .collect(),
        );
    });
    let looking: Vec<(String, String)> = parameters
        .iter()
        .zip(&looked_up)
        .filter_map(|(parameter, keys)| Some((parameter.clone(), keys.clone()?)))
        .collect();
    DART_LOOKED_UP.with(|recorded| {
        recorded.borrow_mut().insert(ident.to_string(), looked_up);
    });
    DART_FLATTENING.with(|current| *current.borrow_mut() = flattenings);
    DART_LOOKING.with(|current| *current.borrow_mut() = looking);
}

/// Each member of `fields` serde reads, and whether it is flattened.
fn read_members<'item>(
    fields: impl Iterator<Item = &'item Field>,
) -> Vec<(bool, &'item syn::Type)> {
    fields
        .filter(|field| !parse_serde_key_omission(&field.attrs).absent_from_wire())
        .map(|field| (field_is_flatten(&field.attrs), &field.ty))
        .collect()
}

/// How `def`, a member read flattened where `flattened` holds, reads the type parameter
/// `parameter`, added to `reads`.
fn parameter_reads(def: &FieldDef, parameter: &str, flattened: bool, reads: &mut ParameterReads) {
    match &def.field_type {
        FieldDefType::TypeParam(name) => {
            if name == parameter {
                if flattened && def.array_depth == 0 {
                    reads.flattened = true;
                    reads.looked_up |= def.is_optional();
                } else {
                    reads.plainly = true;
                }
            }
        }
        FieldDefType::SiblingType(name, arguments) => {
            let through = flattenings_of(name);
            let looked_up = looked_up_by(name);
            for (at, argument) in arguments.iter().enumerate() {
                let bare = argument.field_type == FieldDefType::TypeParam(parameter.to_owned())
                    && argument.array_depth == 0
                    && !argument.is_optional();
                reads.looked_up |= bare && looked_up.get(at).is_some_and(Option::is_some);
                match through.get(at) {
                    Some(Flattening::Alone) if bare => reads.flattened = true,
                    Some(Flattening::Also(_)) if bare => {
                        reads.flattened = true;
                        reads.plainly = true;
                    }
                    Some(Flattening::Alone | Flattening::Also(_) | Flattening::Never) | None => {
                        parameter_reads(argument, parameter, false, reads);
                    }
                }
            }
        }
        FieldDefType::Map(first, second) | FieldDefType::Result(first, second) => {
            parameter_reads(first, parameter, false, reads);
            parameter_reads(second, parameter, false, reads);
        }
        FieldDefType::Range(_, bound) => parameter_reads(bound, parameter, false, reads),
        FieldDefType::Tuple(elements) => {
            for element in elements {
                parameter_reads(element, parameter, false, reads);
            }
        }
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::Unknown
        | FieldDefType::Usize => {}
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => {}
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => {}
    }
}

/// How the type `rust_ident` names reads each of its type parameters flattened: nothing for a
/// type declared below the one asking.
fn flattenings_of(rust_ident: &str) -> Vec<Flattening> {
    DART_FLATTENINGS
        .with(|recorded| recorded.borrow().get(rust_ident).cloned())
        .unwrap_or_default()
}

/// The name the read of the type `rust_ident` names takes the keys of what fills each of its type
/// parameters under, where it reads the parameter flattened under an `Option`: nothing for a type
/// declared below the one asking.
fn looked_up_by(rust_ident: &str) -> Vec<Option<String>> {
    DART_LOOKED_UP
        .with(|recorded| recorded.borrow().get(rust_ident).cloned())
        .unwrap_or_default()
}

/// The name the read of the item being emitted takes the keys of what fills its type parameter
/// `parameter` under: `None` for a parameter it reads flattened under no `Option`.
fn looked_up_argument(parameter: &str) -> Option<String> {
    DART_LOOKING.with(|current| {
        current
            .borrow()
            .iter()
            .find_map(|(name, keys)| (name == parameter).then(|| keys.clone()))
    })
}

/// The name the read of the item being emitted takes the converter of the flattened reads of its
/// type parameter `parameter` under: `None` for a parameter it reads flattened alone or never.
fn flattened_argument(parameter: &str) -> Option<String> {
    DART_FLATTENING.with(|current| {
        current
            .borrow()
            .iter()
            .find_map(|(name, flattening)| match flattening {
                Flattening::Also(argument) if name == parameter => Some(argument.clone()),
                Flattening::Alone | Flattening::Also(_) | Flattening::Never => None,
            })
    })
}

/// The name of each function the reads of `item` call: what the `preprocess` of the type names,
/// and that of each member.
fn called_by(item: &Item, args: &DartTypeArgs<'_>) -> Vec<String> {
    let members: Vec<&Field> = if let Item::Struct(item_struct) = item {
        item_struct.fields.iter().collect()
    } else if let Item::Enum(item_enum) = item {
        item_enum
            .variants
            .iter()
            .flat_map(|variant| variant.fields.iter())
            .collect()
    } else {
        Vec::new()
    };
    args.preprocess
        .iter()
        .map(|function| function.name.clone())
        .chain(members.into_iter().flat_map(|member| {
            (args.member_meta)(member)
                .preprocess
                .into_iter()
                .map(|function| function.name)
        }))
        .collect()
}

/// Records `called`, the name of each function the reads of the item being emitted call.
fn record_called(called: Vec<String>) {
    DART_CALLED.with(|held| *held.borrow_mut() = called);
}

/// `word` as a name a read of the item being emitted declares: itself, or numbered where the
/// item calls a function under it, which the name would hide.
fn declared(word: &str) -> String {
    DART_CALLED.with(|held| {
        let called = held.borrow();
        let mut named = word.to_owned();
        let mut tried: usize = 1;
        while called.contains(&named) {
            tried = tried.saturating_add(1);
            named = format!("{word}{tried}");
        }
        named
    })
}

/// The name a read takes the converter of the type parameter `parameter` under.
fn from_json_argument(parameter: &str) -> String {
    declared(&dart_from_json_argument(parameter))
}

/// The Dart name registered for `rust_ident`, or `None` for a type declared below the one asking —
/// which falls back to its own Rust ident, harmless since a renamed item always re-publishes that
/// ident too, as a `typedef`.
pub fn lookup_dart_name(rust_ident: &str) -> Option<String> {
    DART_NAMES.with(|names| names.borrow().get(rust_ident).cloned())
}

/// Records the Dart name `rust_ident` publishes under, for a later sibling reference to read back.
fn register_dart_name(rust_ident: &str, export_name: &str) {
    DART_NAMES.with(|names| {
        names
            .borrow_mut()
            .insert(rust_ident.to_owned(), export_name.to_owned());
    });
}

/// The module `{ident}_dart` publishes `dart_definition()` from, never an inherent `impl {ident}`:
/// an alias's `impl` is its target's, which collides with the target's own or is refused for a
/// foreign or primitive type.
///
/// Where a member is read as a default, the module also declares what is handed the JSON of the
/// defaults, which `dart_definition()` writes in place of each placeholder, between the quotes of
/// a Dart string.
fn dart_module_tokens(
    rust_ident: &str,
    span: proc_macro2::Span,
    dart_source: &str,
    defaults: &SerdeDefaults,
) -> TokenStream {
    let classes: Vec<String> = DART_RESULTS
        .with(RefCell::take)
        .into_iter()
        .map(|(_, _, declared)| declared)
        .collect();
    let full_source = if classes.is_empty() {
        dart_source.to_owned()
    } else {
        format!("{dart_source}\n\n{}", classes.join("\n\n"))
    };
    let module_ident = dart_module_ident(rust_ident, span);
    let holder = defaults.holder();
    if holder.is_empty() {
        return quote! {
            pub mod #module_ident {
                pub fn dart_definition() -> ::std::string::String {
                    #full_source.to_owned()
                }
            }
        };
    }
    let written = Ident::new("written", proc_macro2::Span::call_site());
    let replaced = substituted(
        &quote! { SerdeDefaults },
        &written,
        &[('\\', "\\\\"), ('\'', "\\'"), ('$', "\\$")],
    );
    let values = defaults.values_impl(&module_ident);
    quote! {
        pub mod #module_ident {
            #holder

            pub fn dart_definition() -> ::std::string::String {
                let mut #written = ::std::string::String::from(#full_source);
                #replaced
                #written
            }
        }

        #values
    }
}

/// The `{ident}_dart` module ident an item's own `dart_definition()` publishes from — read back
/// rather than re-derived by anything naming that module for an item it did not itself declare (a
/// service registry naming a message's or a fault type's own Dart module, for instance).
pub fn dart_module_ident(rust_ident: &str, span: proc_macro2::Span) -> Ident {
    Ident::new(&format!("{}_dart", to_snake_case(rust_ident)), span)
}

/// The `typedef {rust_ident}{generics} = {export_name}{generics};` a renamed item re-publishes
/// under its own Rust ident — the alias a reference declared above the rename still resolves
/// through. Empty for an item that already publishes under its own ident.
fn ident_typedef(rust_ident: &str, export_name: &str, generic_params: &str) -> String {
    if rust_ident == export_name {
        String::new()
    } else {
        format!("\n\ntypedef {rust_ident}{generic_params} = {export_name}{generic_params};")
    }
}

/// `<T, U>` for the type parameters `generics` declares, or the empty string for none.
fn dart_generic_params(generics: &syn::Generics) -> String {
    let parameters = type_parameters_in_scope(generics);
    if parameters.is_empty() {
        String::new()
    } else {
        format!("<{}>", parameters.join(", "))
    }
}

/// One member's `#[model_schema_prop(...)]` metadata, folded into the `FieldDef` `get_field_def`
/// built for it — `get_field_def` reads the Rust type alone, so a member carrying `nullable`,
/// `ts_optional`, `as_number`, a bound or a check needs this filled in separately, exactly as
/// `process_field` does for the other three surfaces.
fn field_def_with_prop_meta(name: &str, member: &Field, args: &DartTypeArgs<'_>) -> FieldDef {
    let mut field_def = get_field_def(name, &member.ty, "");
    field_def.model_schema_prop_meta = Some((args.member_meta)(member));
    field_def
}

/// The `#[serde(rename = "...")]` a field or variant earns, honored only where the `serde` feature
/// reads serde attributes at all.
#[cfg(feature = "serde")]
fn rename_override(attrs: &[syn::Attribute]) -> Option<String> {
    parse_serde_field_attributes(attrs).rename
}

#[cfg(not(feature = "serde"))]
const fn rename_override(_attrs: &[syn::Attribute]) -> Option<String> {
    None
}

/// The names serde reads a field or a variant under beside its own, each `alias` written on it:
/// none where serde does not read it.
#[cfg(feature = "serde")]
fn read_aliases(attrs: &[syn::Attribute]) -> Vec<String> {
    if parse_serde_key_omission(attrs).skips_deserializing {
        return Vec::new();
    }
    parse_serde_field_attributes(attrs).aliases
}

#[cfg(not(feature = "serde"))]
const fn read_aliases(_attrs: &[syn::Attribute]) -> Vec<String> {
    Vec::new()
}

/// `name` and then each of `aliases`: every name serde reads a variant under.
fn read_as(name: String, aliases: Vec<String>) -> Vec<String> {
    once(name).chain(aliases).collect()
}

/// The pattern a `switch` takes any of `names` by.
fn any_of(names: &[String]) -> String {
    names
        .iter()
        .map(|name| quoted(name))
        .collect::<Vec<_>>()
        .join(" || ")
}

/// `name`, a key or a name serde writes, as a Dart string literal between single quotes, which
/// reads `$` as an interpolation and `\` as an escape.
pub fn quoted(name: &str) -> String {
    format!("'{}'", dart_escaped(name, '\''))
}

/// Whether `attrs` carries `#[serde(flatten)]` — always `false` without the `serde` feature, since
/// nothing else reads what a serde attribute means in that build either.
#[cfg(feature = "serde")]
fn field_is_flatten(attrs: &[syn::Attribute]) -> bool {
    parse_serde_field_attributes(attrs).flatten
}

#[cfg(not(feature = "serde"))]
const fn field_is_flatten(_attrs: &[syn::Attribute]) -> bool {
    false
}

/// `name` as a Dart member: itself, or with a trailing underscore where Dart reserves it or the
/// emitted type has taken it (`taken`). Dart has no escape for a reserved word, so the member
/// moves and the wire key stays. `written` is every name the item itself writes, which the moved
/// name steps past.
fn dart_member(name: &str, taken: &[&str], written: &[String]) -> String {
    if !DART_RESERVED.contains(&name) && !taken.contains(&name) {
        return name.to_owned();
    }
    let mut moved = format!("{name}_");
    while written.contains(&moved) {
        moved.push('_');
    }
    moved
}

/// The member an emitted class declares for its field `name`, beside its fields `fields`: what a
/// client reading that field off the message writes.
#[cfg(feature = "serde")]
pub fn dart_field_member(name: &str, fields: &[String]) -> String {
    dart_member(name, &DART_FIELD_NAMES_TAKEN, fields)
}

/// The fields of the struct `rust_ident`, where its class was emitted above; none otherwise.
#[cfg(feature = "serde")]
pub fn dart_fields_of(rust_ident: &str) -> Vec<String> {
    DART_FIELDS.with(|fields| fields.borrow().get(rust_ident).cloned().unwrap_or_default())
}

/// Records the fields the struct `rust_ident` declares, for a client emitted after it.
pub fn record_dart_fields(rust_ident: &str, fields: Vec<String>) {
    DART_FIELDS.with(|recorded| {
        recorded.borrow_mut().insert(rust_ident.to_owned(), fields);
    });
}

/// Records `taken`, what serde takes for the type `rust_ident` flattened into a holder, for a
/// holder emitted after it.
fn record_taken_keys(rust_ident: &str, taken: Taken) {
    DART_TAKEN_KEYS.with(|recorded| {
        recorded.borrow_mut().insert(rust_ident.to_owned(), taken);
    });
}

/// The tag, then the content key where it has one, of an enum under a tag key: what a flattened
/// `Option` of it is looked for by. None for an untagged enum, and for one that names a function
/// on itself, which is read from what the function answers.
fn tag_keys(tag_attrs: &EnumTagAttrs, preprocess: &[FunctionName]) -> Vec<String> {
    if tag_attrs.untagged || !preprocess.is_empty() {
        return Vec::new();
    }
    tag_attrs
        .tag
        .iter()
        .chain(&tag_attrs.content)
        .cloned()
        .collect()
}

/// Records `keys`, the tag and the content key of the enum `rust_ident`, for a holder emitted after
/// it: none for an enum under no tag key, or one that names a function on itself.
fn record_tag_keys(rust_ident: &str, keys: Vec<String>) {
    DART_TAG_KEYS.with(|recorded| {
        if keys.is_empty() {
            recorded.borrow_mut().remove(rust_ident);
        } else {
            recorded.borrow_mut().insert(rust_ident.to_owned(), keys);
        }
    });
}

/// The keys a flattened `Option` typed `def` is looked for by in the object it is handed: `taken`,
/// the keys its type takes, and where it takes none, the tag and the content key of an enum under
/// a tag key whose class was emitted above.
fn looked_for_by(def: &FieldDef, taken: &[String]) -> Vec<String> {
    if !taken.is_empty() {
        return taken.to_vec();
    }
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return Vec::new();
    };
    DART_TAG_KEYS.with(|recorded| recorded.borrow().get(name).cloned().unwrap_or_default())
}

/// Records that the externally tagged enum `rust_ident` writes a unit variant as its bare name, for
/// a holder emitted after it.
fn record_writes_a_name(rust_ident: &str) {
    DART_WRITES_A_NAME.with(|recorded| {
        recorded.borrow_mut().insert(rust_ident.to_owned());
    });
}

/// Records whether the class emitted for `rust_ident` reads a value `reads_a_unit` answers for.
fn note_reads_a_unit(rust_ident: &str, reads: bool) {
    DART_READS_A_UNIT.with(|recorded| {
        if reads {
            recorded.borrow_mut().insert(rust_ident.to_owned());
        } else {
            recorded.borrow_mut().remove(rust_ident);
        }
    });
}

/// Records whether the class emitted for `rust_ident` declares its `toJson` as `Map<String,
/// dynamic>`.
fn note_writes_an_object(rust_ident: &str, writes: bool) {
    DART_WRITES_AN_OBJECT.with(|recorded| {
        if writes {
            recorded.borrow_mut().insert(rust_ident.to_owned());
        } else {
            recorded.borrow_mut().remove(rust_ident);
        }
    });
}

/// Whether Dart types what a value typed `def` writes as `Map<String, dynamic>` already: a map,
/// the README's `ObjectId`, and a struct `DART_WRITES_AN_OBJECT` notes. A type not known keeps
/// `dynamic`.
fn writes_an_object(def: &FieldDef) -> bool {
    if def.array_depth > 0 || def.is_optional() {
        return false;
    }
    if let FieldDefType::SiblingType(name, _) = &def.field_type {
        return DART_WRITES_AN_OBJECT.with(|recorded| recorded.borrow().contains(name));
    }
    #[cfg(feature = "mongodb")]
    if matches!(def.field_type, FieldDefType::ObjectId) {
        return true;
    }
    matches!(def.field_type, FieldDefType::Map(_, _))
}

/// Whether a value typed `def` is `()`, or a type emitted above that `DART_READS_A_UNIT` notes:
/// its read takes `null`, and so the `null` an absent key reads as.
fn reads_a_unit(def: &FieldDef) -> bool {
    def.is_unit()
        || matches!(
            &def.field_type,
            FieldDefType::SiblingType(name, generics)
                if generics.is_empty()
                    && def.array_depth == 0
                    && def.nullable_levels.is_empty()
                    && DART_READS_A_UNIT.with(|recorded| recorded.borrow().contains(name))
        )
}

/// Whether a member typed `def`, flattened into its holder, writes a unit variant as its bare name,
/// which the holder writes as an entry of that name over `null`, as serde writes it.
fn writes_a_name(def: &FieldDef) -> bool {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return false;
    };
    DART_WRITES_A_NAME.with(|recorded| recorded.borrow().contains(name))
}

/// The entries `encode`, what a value of `def` writes, adds to the object it joins: a unit variant
/// of an externally tagged enum emitted above as an entry of its name over `null`, as serde writes
/// it, where the enum's own `toJson` writes the bare name. A value Dart types
/// `Map<String, dynamic>` already is not cast to it.
fn joined_entries(def: &FieldDef, encode: &str) -> String {
    if writes_a_name(def) {
        format!(
            "(switch ({encode}) {{ final String name => {{name: null}}, final written => written \
             as Map<String, dynamic> }})"
        )
    } else if writes_an_object(def) {
        encode.to_owned()
    } else {
        format!("({encode} as Map<String, dynamic>)")
    }
}

/// What a member typed `def`, flattened into its holder, takes off the object serde hands it:
/// what was recorded for a struct or an enum whose class was emitted above, and nothing for a
/// member that names a function, which reads it as a value.
fn taken_by(def: &FieldDef) -> Taken {
    let none = Taken::Keys(Vec::new());
    if !member_functions(def).is_empty() {
        return none;
    }
    let object_id = object_id_keys(def);
    if !object_id.is_empty() {
        return Taken::Only(object_id);
    }
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return none;
    };
    DART_TAKEN_KEYS
        .with(|recorded| recorded.borrow().get(name).cloned())
        .unwrap_or(none)
}

/// The one key an `ObjectId` written as itself, under an `Option` or not, is read from: `$oid`,
/// and none for any other member.
#[cfg(feature = "mongodb")]
fn object_id_keys(def: &FieldDef) -> Vec<String> {
    if matches!(def.field_type, FieldDefType::ObjectId) && def.array_depth == 0 {
        vec!["$oid".to_owned()]
    } else {
        Vec::new()
    }
}

/// No member is an `ObjectId` without `mongodb`.
#[cfg(not(feature = "mongodb"))]
const fn object_id_keys(_def: &FieldDef) -> Vec<String> {
    Vec::new()
}

/// The Rust ident of the type a member typed `def` names, which serde names an enum by in a
/// refusal, and nothing for a member that names no type.
fn named_type(def: &FieldDef) -> &str {
    if let FieldDefType::SiblingType(name, _) = &def.field_type {
        name
    } else {
        ""
    }
}

/// The keys serde reads the struct a member typed `def` describes under, whatever function the
/// member names: those recorded for a struct whose class was emitted above, and none for any other
/// type.
fn struct_keys(def: &FieldDef) -> Vec<String> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return Vec::new();
    };
    match DART_TAKEN_KEYS.with(|recorded| recorded.borrow().get(name).cloned()) {
        Some(Taken::Keys(keys)) => keys,
        Some(Taken::FirstOf(_) | Taken::Tagged(_) | Taken::Only(_)) | None => Vec::new(),
    }
}

/// Whether a member typed `def`, flattened into its holder, hands the object it is handed to no
/// function: a map that names none, and a struct whose class was emitted above that hands it to
/// none, where the member names none.
fn hands_to_none(def: &FieldDef) -> bool {
    if !member_functions(def).is_empty() {
        return false;
    }
    if let FieldDefType::SiblingType(name, _) = &def.field_type {
        DART_HANDED.with(|recorded| recorded.borrow().get(name) == Some(&false))
    } else {
        matches!(def.field_type, FieldDefType::Map(..))
    }
}

/// Whether a member typed `def`, flattened into its holder, is read from the whole object the
/// holder reads, as TypeScript and Zod read it: a struct, and an enum under a tag key, whose class
/// was emitted above and that hands that object to no function. One that hands it to a function
/// is handed what serde hands it.
fn reads_the_whole(def: &FieldDef) -> bool {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return false;
    };
    hands_to_none(def)
        && (DART_TAKEN_KEYS
            .with(|recorded| matches!(recorded.borrow().get(name), Some(Taken::Keys(_))))
            || DART_TAG_KEYS.with(|recorded| recorded.borrow().contains_key(name)))
}

/// `name` as a method or a parameter of an emitted client, moved off a reserved word and off
/// `taken`, the names written around it.
#[cfg(feature = "serde")]
pub fn dart_local(name: &str, taken: &[&str]) -> String {
    dart_member(name, taken, &[])
}

/// The wire name a field with Rust name `rust_name` and its own `rename` writes under, once
/// `rule` — the container's own `rename_all`, [`RenameRule::None`] without the `serde` feature —
/// has had its say. An explicit rename always wins over the container's rule, matching serde
/// itself.
fn wire_field_name(rust_name: &str, rename: Option<&str>, rule: RenameRule) -> String {
    rename.map_or_else(|| rule.apply_to_field(rust_name), ToOwned::to_owned)
}

/// A container's own `rename_all`, or [`RenameRule::None`] without the `serde` feature to read it
/// with — shared by a struct and an enum, whose variant names read the very same attribute.
#[cfg(feature = "serde")]
fn container_rename_rule(attrs: &[syn::Attribute]) -> RenameRule {
    let meta = parse_serde_type_attributes(attrs);
    resolve_rename_rule(meta.rename_all.as_deref())
}

#[cfg(not(feature = "serde"))]
fn container_rename_rule(_attrs: &[syn::Attribute]) -> RenameRule {
    resolve_rename_rule(None)
}

/// The container attributes an enum's own dispatch reads, or every field left at its default
/// without the `serde` feature to read one with.
#[cfg(feature = "serde")]
fn enum_tag_attrs(attrs: &[syn::Attribute]) -> EnumTagAttrs {
    let meta = parse_serde_type_attributes(attrs);
    EnumTagAttrs {
        content: meta.content,
        rename_all: meta.rename_all,
        rename_all_fields: meta.rename_all_fields,
        tag: meta.tag,
        untagged: meta.untagged,
    }
}

#[cfg(not(feature = "serde"))]
const fn enum_tag_attrs(_attrs: &[syn::Attribute]) -> EnumTagAttrs {
    EnumTagAttrs {
        content: None,
        rename_all: None,
        rename_all_fields: None,
        tag: None,
        untagged: false,
    }
}

/// Walks a named-field struct's or a struct-shaped enum variant's fields into [`DartField`]s,
/// dropping any field a serde attribute takes off the wire in both directions — the same field
/// `collect_struct_fields` drops for the other three surfaces.
fn collect_dart_fields(
    fields: &Fields,
    rule: RenameRule,
    type_parameters: &[String],
    args: &DartTypeArgs<'_>,
    default_at: &dyn Fn(&str) -> Option<String>,
) -> Vec<DartField> {
    let Fields::Named(named) = fields else {
        return Vec::new();
    };
    let written: Vec<String> = named
        .named
        .iter()
        .filter_map(|field| Some(field.ident.as_ref()?.unraw().to_string()))
        .collect();
    let mut collected = Vec::new();
    for field in &named.named {
        let Some(ident) = field.ident.as_ref() else {
            continue;
        };
        let rust_name = ident.unraw().to_string();
        let omission = parse_serde_key_omission(&field.attrs);
        if omission.absent_from_wire() {
            continue;
        }
        let wire_name = wire_field_name(&rust_name, rename_override(&field.attrs).as_deref(), rule);
        let mut field_def = field_def_with_prop_meta(&rust_name, field, args);
        field_def.erase_type_parameters(type_parameters);
        let flatten = field_is_flatten(&field.attrs);
        // A flattened member has no key to leave out, and serde reads it whatever it writes.
        if omission.omits_key && !flatten && !field_def.is_optional() {
            // Dart has no absent-key spelling: a `Map<String, dynamic>` answers a dropped key and
            // an explicit `null` alike, so an omitted key is the outer level's own nullability.
            field_def.nullable_levels.push(field_def.array_depth);
        }
        collected.push(DartField {
            aliases: read_aliases(&field.attrs),
            default_at: default_at(&rust_name),
            required: !omission.omits_key || !field_def.is_optional(),
            flatten,
            read_as: if flatten && field_def.array_depth == 0 && !field_def.is_optional() {
                field_def.written_in_place()
            } else {
                None
            },
            field_def,
            member: dart_member(&rust_name, &DART_FIELD_NAMES_TAKEN, &written),
            never_written: has_serde_skip_serializing(&field.attrs),
            wire_name,
        });
    }
    collected
}

// Dart type and JSON-codec rendering over `&FieldDef`, kept in this module so `field_type.rs`
// carries no Dart-shaped knowledge.

/// Whether `field` carries `#[model_schema_prop(as_number)]` — the same question
/// `FieldDef::has_as_number` answers privately for the other surfaces, read here directly off the
/// public `model_schema_prop_meta` field since that private method is not reachable from this
/// module.
#[cfg(feature = "chrono")]
fn has_as_number(field: &FieldDef) -> bool {
    field
        .model_schema_prop_meta
        .as_ref()
        .is_some_and(|meta| meta.as_number)
}

/// The Dart type before the outer `?` an [`FieldDef::is_optional`] field carries: the scalar
/// match, then one `List<…>` per array level, an inner level written `?` where
/// [`FieldDef::is_nullable_at`] says so. Mirrors `typescript_base`.
fn dart_base(field: &FieldDef) -> String {
    let scalar = match &field.field_type {
        // An opaque type parameter (`FieldDef::opaque_parameters`), and a type written as no path.
        FieldDefType::Unknown => "dynamic".to_owned(),
        FieldDefType::TypeParam(name) => name.clone(),
        FieldDefType::Tuple(elements) => tuple_typename(elements),
        FieldDefType::Range(kind, bound) => {
            let held = dart_typename(bound);
            let rendered = kind
                .members()
                .iter()
                .map(|member| format!("{held} {member}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({{{rendered}}})")
        }
        FieldDefType::Result(ok, err) => result_typename(ok, err),
        FieldDefType::SiblingType(name, generics) => {
            if let [element] = generics.as_slice()
                && is_sequence_wrapper(name)
            {
                // Re-enters the whole rendering as the arrayed field the wrapper stands for,
                // carrying this field's own array levels with it — see `collection_element_field`.
                return dart_base(&field.collection_element_field(element));
            }
            let class_name = lookup_dart_name(name).unwrap_or_else(|| name.clone());
            if generics.is_empty() {
                class_name
            } else {
                let arguments = generics
                    .iter()
                    .map(dart_typename)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{class_name}<{arguments}>")
            }
        }
        FieldDefType::Map(key, value) => {
            format!(
                "Map<{}, {}>",
                dart_map_key_typename(key),
                dart_typename(value)
            )
        }
        FieldDefType::Boolean => "bool".to_owned(),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            "String".to_owned()
        }
        FieldDefType::BooleanLiteral(_) => "bool".to_owned(),
        FieldDefType::NumberLiteral(_) => "double".to_owned(),
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize => "int".to_owned(),
        FieldDefType::F32 | FieldDefType::F64 => "double".to_owned(),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => "ObjectId".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            "String".to_owned()
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => {
            if has_as_number(field) {
                "int".to_owned()
            } else {
                "DateTime".to_owned()
            }
        }
    };
    (0..field.array_depth).fold(scalar, |wrapped, level| {
        if level == 0 && field.writes_bytes() {
            return "Uint8List".to_owned();
        }
        let item = if field.is_nullable_at(level) {
            format!("{wrapped}?")
        } else {
            wrapped
        };
        format!("List<{item}>")
    })
}

/// The Dart record of `elements`, one position each: `()` for none, which is `()`'s type.
fn tuple_typename(elements: &[FieldDef]) -> String {
    let rendered = elements
        .iter()
        .map(dart_typename)
        .collect::<Vec<_>>()
        .join(", ");
    if elements.len() == 1 {
        format!("({rendered},)")
    } else {
        format!("({rendered})")
    }
}

/// The key type a `Map<…>` is written with — the key's own semantic Dart type, since (unlike
/// TypeScript's `Record`) a Dart `Map` places no restriction on its key type at all: a stringified
/// wire key (a bool, an integer, an enum, a timestamp) still reads back as its own real Dart type,
/// the string form living only in [`dart_map_key_decode`]/[`dart_map_key_encode`].
fn dart_map_key_typename(key: &FieldDef) -> String {
    if key.parameter_shape_name().is_some() {
        return "String".to_owned();
    }
    match key.map_key_wire() {
        MapKeyWire::Boolean => "bool".to_owned(),
        #[cfg(feature = "chrono")]
        MapKeyWire::Timestamp => "DateTime".to_owned(),
        MapKeyWire::Enumerated
        | MapKeyWire::Named
        | MapKeyWire::Signed64
        | MapKeyWire::Unsigned64 => dart_base(key),
    }
}

/// The expression that reads one `Map` key back from the `String` JSON always writes it as — the
/// counterpart of [`dart_map_key_typename`]. `expr` names a `String`. No `validate()` reaches a
/// key, so a declared type there is read judged.
fn dart_map_key_decode(key: &FieldDef, expr: &str) -> String {
    if key.parameter_shape_name().is_some() {
        return expr.to_owned();
    }
    match key.map_key_wire() {
        MapKeyWire::Boolean => format!("({expr} == \"true\")"),
        #[cfg(feature = "chrono")]
        MapKeyWire::Timestamp => format!("DateTime.parse({expr})"),
        MapKeyWire::Enumerated => format!("{}.fromJson({expr})", dart_base(key)),
        MapKeyWire::Named | MapKeyWire::Signed64 | MapKeyWire::Unsigned64 => {
            match &key.field_type {
                FieldDefType::U8
                | FieldDefType::U16
                | FieldDefType::U32
                | FieldDefType::U64
                | FieldDefType::I8
                | FieldDefType::I16
                | FieldDefType::I32
                | FieldDefType::I64
                | FieldDefType::Usize
                | FieldDefType::Isize => {
                    let read = format!("int.parse({expr})");
                    forms::number_held(&key.field_type, &read).unwrap_or(read)
                }
                FieldDefType::F32 | FieldDefType::F64 => format!("double.parse({expr})"),
                // The key is a `String` already, and the analyzer reports a cast of it.
                FieldDefType::String | FieldDefType::StringLiteral(_) => expr.to_owned(),
                FieldDefType::Char => forms::text_read(TextForm::Char, expr),
                FieldDefType::Boolean
                | FieldDefType::BooleanLiteral(_)
                | FieldDefType::NumberLiteral(_)
                | FieldDefType::SiblingType(_, _)
                | FieldDefType::Map(_, _)
                | FieldDefType::Range(_, _)
                | FieldDefType::Result(_, _)
                | FieldDefType::Tuple(_)
                | FieldDefType::TypeParam(_)
                | FieldDefType::Unknown => dart_decode_expr(key, expr, Read::Judged),
                #[cfg(feature = "mongodb")]
                FieldDefType::ObjectId => dart_decode_expr(key, expr, Read::Judged),
                #[cfg(feature = "chrono")]
                FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
                    TextForm::of(key)
                        .map_or_else(|| expr.to_owned(), |form| forms::text_read(form, expr))
                }
                #[cfg(feature = "chrono")]
                FieldDefType::DateTime => dart_decode_expr(key, expr, Read::Judged),
            }
        }
    }
}

/// The expression that writes one `Map` key as the `String` JSON always carries it as — the
/// counterpart of [`dart_map_key_decode`]. `expr` names a value of [`dart_map_key_typename`]'s own
/// type.
fn dart_map_key_encode(key: &FieldDef, expr: &str) -> String {
    if key.parameter_shape_name().is_some() {
        return expr.to_owned();
    }
    match key.map_key_wire() {
        MapKeyWire::Boolean => format!("({expr} ? \"true\" : \"false\")"),
        #[cfg(feature = "chrono")]
        MapKeyWire::Timestamp => format!("{expr}.toUtc().toIso8601String()"),
        MapKeyWire::Enumerated => format!("({expr}).toJson()"),
        MapKeyWire::Named | MapKeyWire::Signed64 | MapKeyWire::Unsigned64 => {
            match &key.field_type {
                FieldDefType::U8
                | FieldDefType::U16
                | FieldDefType::U32
                | FieldDefType::U64
                | FieldDefType::I8
                | FieldDefType::I16
                | FieldDefType::I32
                | FieldDefType::I64
                | FieldDefType::Usize
                | FieldDefType::Isize
                | FieldDefType::F32
                | FieldDefType::F64 => format!(
                    "({}).toString()",
                    forms::number_held(&key.field_type, expr).unwrap_or_else(|| expr.to_owned())
                ),
                // The key is a `String` already, and the analyzer reports a cast of it.
                FieldDefType::String | FieldDefType::StringLiteral(_) => expr.to_owned(),
                FieldDefType::Char => forms::text_sent(TextForm::Char, expr),
                FieldDefType::Boolean
                | FieldDefType::BooleanLiteral(_)
                | FieldDefType::NumberLiteral(_)
                | FieldDefType::SiblingType(_, _)
                | FieldDefType::Map(_, _)
                | FieldDefType::Range(_, _)
                | FieldDefType::Result(_, _)
                | FieldDefType::Tuple(_)
                | FieldDefType::TypeParam(_)
                | FieldDefType::Unknown => {
                    format!("({}) as String", dart_encode_expr(key, expr, false))
                }
                #[cfg(feature = "mongodb")]
                FieldDefType::ObjectId => {
                    format!("({}) as String", dart_encode_expr(key, expr, false))
                }
                #[cfg(feature = "chrono")]
                FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
                    TextForm::of(key)
                        .map_or_else(|| expr.to_owned(), |form| forms::text_sent(form, expr))
                }
                #[cfg(feature = "chrono")]
                FieldDefType::DateTime => {
                    format!("({}) as String", dart_encode_expr(key, expr, false))
                }
            }
        }
    }
}

/// The Dart type `field` renders as: [`dart_base`] plus the `?` an [`FieldDef::is_optional`] field
/// carries — Dart has one nullable spelling for all three of a bare `Option<T>`,
/// `#[model_schema_prop(ts_optional)]` and `#[model_schema_prop(nullable)]`, since a Dart
/// `Map<String, dynamic>` answers a dropped key and an explicit `null` alike.
pub fn dart_typename(field: &FieldDef) -> String {
    let base = dart_base(field);
    if field.is_optional() {
        format!("{base}?")
    } else {
        base
    }
}

/// Decodes a value of `ty` out of the dynamically typed JSON `expr`, as a model field of that type
/// is decoded, a declared type by its judged read.
#[cfg(feature = "serde")]
pub fn dart_json_decode(ty: &syn::Type, expr: &str) -> String {
    dart_decode_expr(&get_field_def("value", ty, ""), expr, Read::Judged)
}

/// Encodes the Dart value `expr` of `ty` into a `jsonEncode`-safe value, as a model field of that
/// type is encoded. `promoted`: Dart narrows `expr`, being a parameter or a local.
#[cfg(feature = "serde")]
pub fn dart_json_encode(ty: &syn::Type, expr: &str, promoted: bool) -> String {
    dart_encode_expr(&get_field_def("value", ty, ""), expr, promoted)
}

/// The converters a holder hands the type `name` for `arguments`, one per type parameter, each
/// reading the argument judged: no `validate()` reaches a value behind a parameter. A value that
/// reads one key alone is handed that key alone where the type reads its parameter flattened
/// alone, and by a converter of the flattened reads where it reads it otherwise too. A parameter
/// of the holder's own is handed the holder's converter of its flattened reads there. Where the
/// type reads a parameter flattened under an `Option`, it is handed the keys of the argument, where
/// they are known, and a parameter of the holder's own the keys the holder is handed for it.
fn argument_converters(name: &str, arguments: &[FieldDef]) -> String {
    let through = flattenings_of(name);
    let looked_up = looked_up_by(name);
    let mut converters = Vec::new();
    let mut flattened = Vec::new();
    let mut keyed = Vec::new();
    for (at, argument) in arguments.iter().enumerate() {
        let alone = match taken_by(argument) {
            Taken::Only(keys) if !argument.is_optional() => Some(only("e", &keys)),
            Taken::FirstOf(_) | Taken::Keys(_) | Taken::Only(_) | Taken::Tagged(_) => None,
        };
        let held = if let FieldDefType::TypeParam(parameter) = &argument.field_type
            && argument.array_depth == 0
            && !argument.is_optional()
        {
            flattened_argument(parameter).map(|named| (named, from_json_argument(parameter)))
        } else {
            None
        };
        let read = |handed: &str| {
            format!(
                "(e) => {}",
                dart_decode_expr(argument, handed, Read::Judged)
            )
        };
        match (through.get(at), alone, held) {
            (Some(Flattening::Alone), Some(handed), _) => converters.push(read(&handed)),
            (Some(Flattening::Alone), None, Some((named, own))) => {
                converters.push(format!("(e) => ({named} ?? {own})(e)"));
            }
            (Some(Flattening::Also(argument_name)), Some(handed), _) => {
                converters.push(read("e"));
                flattened.push(format!("{argument_name}: {}", read(&handed)));
            }
            (Some(Flattening::Also(argument_name)), None, Some((named, _))) => {
                converters.push(read("e"));
                flattened.push(format!("{argument_name}: {named}"));
            }
            (Some(Flattening::Alone | Flattening::Also(_) | Flattening::Never) | None, _, _) => {
                converters.push(read("e"));
            }
        }
        if let Some(Some(named)) = looked_up.get(at)
            && let Some(keys) = argument_keys(argument)
        {
            keyed.push(format!("{named}: {keys}"));
        }
    }
    converters.extend(flattened);
    converters.extend(keyed);
    converters.join(", ")
}

/// The keys a type that reads its type parameter flattened under an `Option` is handed for
/// `argument`, which fills it: the keys an `Option` of the argument flattened in its place is
/// looked for by, and for a type parameter of the holder's own, the keys the holder is handed for
/// it. `None` where none are known.
fn argument_keys(argument: &FieldDef) -> Option<String> {
    if argument.array_depth > 0 || argument.is_optional() {
        return None;
    }
    if let FieldDefType::TypeParam(parameter) = &argument.field_type {
        return looked_up_argument(parameter);
    }
    let taken = match taken_by(argument) {
        Taken::Keys(keys) | Taken::Only(keys) | Taken::Tagged(keys) => keys,
        Taken::FirstOf(_) => Vec::new(),
    };
    let keys = looked_for_by(argument, &taken);
    (!keys.is_empty()).then(|| format!("const [{}]", listed_names(&keys)))
}

/// The expression that decodes `field`'s value out of the raw, dynamically-typed JSON `expr`
/// names — the whole field including its outer optionality, which is asked once here rather than
/// by every caller. `read` is how a declared type the value holds is read.
fn dart_decode_expr(field: &FieldDef, expr: &str, read: Read) -> String {
    let unwrapped = dart_decode_at(field, field.array_depth, expr, read);
    if field.is_optional() {
        format!("({expr} == null ? null : {unwrapped})")
    } else {
        unwrapped
    }
}

/// [`dart_decode_expr`] before the outer optionality: peels `level` `List<dynamic>` levels off
/// `expr`, an inner level's own nullability read off [`FieldDef::is_nullable_at`], then dispatches
/// on [`FieldDef::field_type`] at level `0`. A level written as `[T; N]` binds its array once,
/// under a name the read declares, and refuses one of another length, as serde and Zod refuse it.
fn dart_decode_at(field: &FieldDef, level: u8, expr: &str, read: Read) -> String {
    if level > 0 {
        let inner_level = level - 1;
        let inner = dart_decode_at(field, inner_level, "e", read);
        let mapper = if field.is_nullable_at(inner_level) {
            format!("(e) => (e == null ? null : {inner})")
        } else {
            format!("(e) => {inner}")
        };
        let Some(length) = field.fixed_length_at(inner_level) else {
            let listed = format!("({expr} as List<dynamic>).map({mapper}).toList()");
            return if field.writes_bytes() && inner_level == 0 {
                format!("Uint8List.fromList({listed})")
            } else {
                listed
            };
        };
        let array = declared("array");
        return format!(
            "switch ({expr} as List<dynamic>) {{ final {array} when {array}.length == {length} => \
             {array}.map({mapper}).toList(), final {array} => throw FormatException('invalid \
             length ${{{array}.length}}, expected {length}') }}"
        );
    }
    match &field.field_type {
        // An opaque type parameter (`FieldDef::opaque_parameters`), and a type written as no path.
        FieldDefType::Unknown => expr.to_owned(),
        FieldDefType::TypeParam(name) => parameter_read(name, expr),
        FieldDefType::Tuple(elements) if elements.is_empty() => unit_decode(expr),
        FieldDefType::Tuple(elements) => tuple_decode(elements, expr, read),
        FieldDefType::Range(kind, bound) => range_decode(*kind, bound, expr, read),
        FieldDefType::Result(ok, err) => result_decode(ok, err, expr, read),
        FieldDefType::SiblingType(name, generics) => {
            if let [element] = generics.as_slice()
                && is_sequence_wrapper(name)
            {
                // The field's own levels are walked already: what is left is the wrapper's.
                let combined = field.collection_element_field(element);
                let wrapped = combined.array_depth.saturating_sub(field.array_depth);
                return dart_decode_at(&combined, wrapped, expr, read);
            }
            let class_name = lookup_dart_name(name).unwrap_or_else(|| name.clone());
            let constructor = read.constructor();
            if generics.is_empty() {
                format!("{class_name}.{constructor}({expr})")
            } else {
                let converters = argument_converters(name, generics);
                format!("{class_name}.{constructor}({expr}, {converters})")
            }
        }
        FieldDefType::Map(key, value) => map_decode(
            key,
            value,
            &format!("({expr} as Map<String, dynamic>)"),
            read,
        ),
        FieldDefType::Boolean => format!("{expr} as bool"),
        FieldDefType::String | FieldDefType::StringLiteral(_) => format!("{expr} as String"),
        FieldDefType::BooleanLiteral(_) => format!("{expr} as bool"),
        FieldDefType::NumberLiteral(_) => format!("({expr} as num).toDouble()"),
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize => {
            let taken = format!("{expr} as int");
            forms::number_held(&field.field_type, &taken).unwrap_or(taken)
        }
        FieldDefType::F32 | FieldDefType::F64 => {
            let taken = format!("({expr} as num).toDouble()");
            forms::number_held(&field.field_type, &taken).unwrap_or(taken)
        }
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => format!("ObjectId.fromJson({expr})"),
        FieldDefType::Char => forms::text_read(TextForm::Char, &format!("{expr} as String")),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            TextForm::of(field).map_or_else(
                || format!("{expr} as String"),
                |form| forms::text_read(form, &format!("{expr} as String")),
            )
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => {
            if has_as_number(field) {
                format!("{expr} as int")
            } else {
                format!("DateTime.parse({expr} as String)")
            }
        }
    }
}

/// The map of `key` to `value` the entries of `object` hold, an expression Dart types
/// `Map<String, dynamic>`.
fn map_decode(key: &FieldDef, value: &FieldDef, object: &str, read: Read) -> String {
    let key_ty = dart_map_key_typename(key);
    let value_ty = dart_typename(value);
    let key_decode = dart_map_key_decode(key, "e.key");
    let value_decode = dart_decode_expr(value, "e.value", read);
    format!(
        "Map<{key_ty}, {value_ty}>.fromEntries({object}.entries.map((e) => MapEntry({key_decode}, \
         {value_decode})))"
    )
}

/// The record the array `expr` holds, each position read as its element of `elements`. The array
/// is bound once, under a name the read declares, and one of another length is refused, as serde
/// and Zod refuse it.
fn tuple_decode(elements: &[FieldDef], expr: &str, read: Read) -> String {
    let array = declared("tuple");
    let rendered = elements
        .iter()
        .enumerate()
        .map(|(index, element)| member_decode(element, &format!("{array}[{index}]"), None, read))
        .collect::<Vec<_>>()
        .join(", ");
    let record = if elements.len() == 1 {
        format!("({rendered},)")
    } else {
        format!("({rendered})")
    };
    let length = elements.len();
    format!(
        "switch ({expr} as List<dynamic>) {{ final {array} when {array}.length == {length} => \
         {record}, final {array} => throw FormatException('invalid length ${{{array}.length}}, \
         expected {length}') }}"
    )
}

/// The array the record `expr` of `elements` is written as, each position written as its element.
fn tuple_encode(elements: &[FieldDef], expr: &str) -> String {
    let rendered = elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            dart_encode_expr(element, &format!("({expr}).${}", index + 1), false)
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{rendered}]")
}

/// `decode`, the read of `field` under `key` of an object, refused where the object does not hold
/// `key` (`held`) and `field` reads a `()`: an absent key reads as the `null` `()` is read from,
/// where serde refuses it. A read under no key is `decode` as it is.
fn keyed_decode(field: &FieldDef, decode: String, held: Option<&str>, key: Option<&str>) -> String {
    match (held, key) {
        (Some(holds), Some(named)) if reads_a_unit(field) => format!(
            "({holds} ? {decode} : throw FormatException('missing field `{}`'))",
            dart_escaped(named, '\'')
        ),
        _ => decode,
    }
}

/// The read of a variant's one value from `source`, under the content key `key` where it has one:
/// [`member_decode`], held to the key by [`keyed_decode`].
fn content_decode(field: &FieldDef, source: &str, held: Option<&str>, key: Option<&str>) -> String {
    let read = member_decode(field, source, held, Read::Unjudged);
    keyed_decode(field, read, held, key)
}

/// `()` read from `expr`: the `null` serde writes for it, and nothing else, as serde reads it.
fn unit_decode(expr: &str) -> String {
    let unit = declared("unit");
    format!(
        "switch ({expr}) {{ null => (), final {unit} => throw FormatException('invalid type: \
         ${{jsonEncode({unit})}}, expected unit') }}"
    )
}

/// The record of the range of `kind` the object `expr` holds, each bound read as `bound` from the
/// key serde writes it under. The object is bound once, under a name the read declares, and one
/// that holds any other key is refused, as serde refuses it.
fn range_decode(kind: RangeKind, bound: &FieldDef, expr: &str, read: Read) -> String {
    let object = declared("range");
    let written = kind
        .members()
        .iter()
        .map(|member| format!("key == {}", quoted(member)))
        .collect::<Vec<_>>()
        .join(" || ");
    let rendered = kind
        .members()
        .iter()
        .map(|member| {
            let decoded =
                member_decode(bound, &format!("{object}[{}]", quoted(member)), None, read);
            format!("{member}: {decoded}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "switch ({expr} as Map<String, dynamic>) {{ final {object} when {object}.keys.every((key) \
         => {written}) => ({rendered}), final {object} => throw FormatException('Unknown key of a \
         {}: ${{{object}.keys}}') }}",
        kind.name()
    )
}

/// The sealed class a `Result` of the arms `ok` and `err` is written through, declared once per
/// item for each shape: generic over the type of each arm that holds a value, an arm of `()` being
/// a subclass with no value.
fn result_class(ok: &FieldDef, err: &FieldDef) -> String {
    let shape = [ok.is_unit(), err.is_unit()];
    DART_RESULTS.with(|results| {
        let mut written = results.borrow_mut();
        if let Some((_, named, _)) = written.iter().find(|(declared, _, _)| *declared == shape) {
            return named.clone();
        }
        let index = DART_RESULT_COUNTER.with(|counter| {
            let next = counter.get().saturating_add(1);
            counter.set(next);
            next
        });
        let named = format!("DartResult{index}");
        let held: Vec<&str> = RESULT_ARMS
            .into_iter()
            .zip(shape)
            .filter(|(_, unit)| !unit)
            .map(|(arm, _)| arm)
            .collect();
        let generic_params = if held.is_empty() {
            String::new()
        } else {
            format!("<{}>", held.join(", "))
        };
        let arms = RESULT_ARMS.into_iter().zip(shape).map(|(arm, unit)| {
            let (constructor, value) = if unit {
                (String::new(), String::new())
            } else {
                ("this.value".to_owned(), format!(" final {arm} value;"))
            };
            format!(
                "class {named}{arm}{generic_params} extends {named}{generic_params} \
                 {{ const {named}{arm}({constructor});{value} }}"
            )
        });
        let declared = format!(
            "sealed class {named}{generic_params} {{ const {named}(); }} {}",
            arms.collect::<Vec<_>>().join(" ")
        );
        written.push((shape, named.clone(), declared));
        named
    })
}

/// The Dart type of a `Result` of the arms `ok` and `err`.
fn result_typename(ok: &FieldDef, err: &FieldDef) -> String {
    format!("{}{}", result_class(ok, err), result_arguments(ok, err))
}

/// The type arguments of the class of a `Result` of the arms `ok` and `err`: the type of each arm
/// that holds a value.
fn result_arguments(ok: &FieldDef, err: &FieldDef) -> String {
    let held: Vec<String> = [ok, err]
        .into_iter()
        .filter(|arm| !arm.is_unit())
        .map(dart_typename)
        .collect();
    if held.is_empty() {
        String::new()
    } else {
        format!("<{}>", held.join(", "))
    }
}

/// The `Result` the object `expr` holds: its one key names the arm, whose value is read as that
/// arm is. An object of any other keys is refused, as serde refuses it, and so is a value under
/// an arm of `()` other than `null`.
fn result_decode(ok: &FieldDef, err: &FieldDef, expr: &str, read: Read) -> String {
    let (class, arguments) = (result_class(ok, err), result_arguments(ok, err));
    let object = declared("result");
    let arms = RESULT_ARMS.into_iter().zip([ok, err]).map(|(arm, def)| {
        let key = quoted(arm);
        let held = format!("{object}[{key}]");
        let named = format!("{object}.length == 1 && {object}.containsKey({key})");
        if def.is_unit() {
            format!("final {object} when {named} && {held} == null => {class}{arm}{arguments}()")
        } else {
            let decoded = member_decode(def, &held, None, read);
            format!("final {object} when {named} => {class}{arm}{arguments}({decoded})")
        }
    });
    format!(
        "switch ({expr} as Map<String, dynamic>) {{ {}, final {object} => throw \
         FormatException('Not a Result: ${{{object}.keys}}') }}",
        arms.collect::<Vec<_>>().join(", ")
    )
}

/// The object the `Result` `expr` is written as: the key of its arm, over the arm's value, and
/// over `null` for an arm of `()`.
fn result_encode(ok: &FieldDef, err: &FieldDef, expr: &str) -> String {
    let class = result_class(ok, err);
    let held = declared("held");
    let arms = RESULT_ARMS.into_iter().zip([ok, err]).map(|(arm, def)| {
        let key = quoted(arm);
        if def.is_unit() {
            format!("{class}{arm}() => <String, dynamic>{{{key}: null}}")
        } else {
            let encoded = dart_encode_expr(def, &held, true);
            format!("{class}{arm}(value: final {held}) => <String, dynamic>{{{key}: {encoded}}}")
        }
    });
    format!(
        "switch ({expr}) {{ {} }}",
        arms.collect::<Vec<_>>().join(", ")
    )
}

/// `payload` as `functions` correct it: each called on the answer of the one before, in the order
/// written, under the name the attribute writes.
fn corrected(functions: &[FunctionName], payload: &str) -> String {
    functions.iter().fold(payload.to_owned(), |held, function| {
        format!("{}({held})", function.name)
    })
}

/// The functions the `preprocess` of the member `field` names, in the order written.
fn member_functions(field: &FieldDef) -> &[FunctionName] {
    field
        .model_schema_prop_meta
        .as_ref()
        .map_or(&[], |meta| meta.preprocess.as_slice())
}

/// The read of the member `field` from `source`, which each function its `preprocess` names
/// corrects first. `held` is the test that the payload holds the member's key, where it may not:
/// no function is called for a key it does not hold. A read that names its payload more than
/// once holds the corrected one under a name, so each function is called once.
fn member_decode(field: &FieldDef, source: &str, held: Option<&str>, read: Read) -> String {
    let functions = member_functions(field);
    if functions.is_empty() {
        return dart_decode_expr(field, source, read);
    }
    let answered = corrected(functions, source);
    let (payload, inline) = held.map_or_else(
        || (answered.clone(), answered.clone()),
        |holds| {
            let chosen = format!("{holds} ? {answered} : null");
            (chosen.clone(), format!("({chosen})"))
        },
    );
    let decode = dart_decode_expr(field, &inline, read);
    if decode.matches(inline.as_str()).count() == 1 {
        return decode;
    }
    format!(
        "switch ({payload}) {{ final corrected => {} }}",
        dart_decode_expr(field, "corrected", read)
    )
}

/// The object `json` names without the keys `taken`: what serde hands a member that is read
/// from the keys the others leave.
fn without(json: &str, taken: &[&str]) -> String {
    if taken.is_empty() {
        return json.to_owned();
    }
    let removed = taken.iter().fold(String::new(), |mut acc, key| {
        write!(acc, "..remove({})", quoted(key)).unwrap();
        acc
    });
    format!("({{...{json}}}{removed})")
}

/// An object of the entries of the object `json` names under `keys`, and of no other: what a type
/// that refuses any other key is handed of its holder's object.
fn only(json: &str, keys: &[String]) -> String {
    format!(
        "(switch ({json}) {{ final whole => {{ for (final key in const [{}]) \
         if (whole.containsKey(key)) key: whole[key] }} }})",
        listed_names(keys)
    )
}

/// `names` as the elements of a Dart list of strings.
fn listed_names(names: &[String]) -> String {
    names
        .iter()
        .map(|name| quoted(name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// An object of the one entry of the object `json` names under one of `names`, one list per
/// variant of the enum `enum_name`, and of none where no entry is: what serde hands an externally
/// tagged enum flattened into its holder, wherever that entry is. Entries under two names are
/// refused, as TypeScript and Zod refuse them, naming each variant they belong to once, in the
/// order the enum declares them.
fn first_of(json: &str, enum_name: &str, names: &[Vec<String>]) -> String {
    let variants = names
        .iter()
        .map(|variant| format!("[{}]", listed_names(variant)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "(switch ({json}) {{ final whole => whole.keys.where(const [{}].contains).length > 1 ? \
         throw FormatException('more than one variant of enum {} found in flattened data: ' + \
         [for (final variant in const [{variants}]) if (variant.any(whole.containsKey)) \
         variant.first].join(', ')) : {{ for (final key in \
         whole.keys.where(const [{}].contains).take(1)) key: whole[key] }} }})",
        listed_names(&names.concat()),
        dart_escaped(enum_name, '\''),
        listed_names(&names.concat())
    )
}

/// The read of an externally tagged enum typed `def` flattened into its holder from `first`, the
/// object [`first_of`] answers. An optional one reads no value where that object holds no entry,
/// and is refused where the entry does not read, as TypeScript and Zod refuse it.
fn first_entry_decode(def: &FieldDef, first: &str) -> String {
    if !def.is_optional() {
        return dart_decode_expr(def, first, Read::Unjudged);
    }
    let read = dart_decode_at(def, def.array_depth, "entry", Read::Unjudged);
    format!("(switch ({first}) {{ final entry => entry.isEmpty ? null : {read} }})")
}

/// The read of a member typed `def` flattened into its holder from `handed`, the object it is
/// handed, whose type takes `taken`. An optional one that names no function reads no value where
/// `handed` holds none of the keys it is looked for by ([`looked_for_by`]), and is refused where
/// its type does not read, as TypeScript and Zod refuse it. Where no key is known, a map among
/// them, it reads no value wherever its type does not read, as serde reads it. An optional one
/// that names a function and holds no map is read by [`corrected_option`].
fn flattened_decode(def: &FieldDef, handed: &str, taken: &[String]) -> String {
    let parameter = if let FieldDefType::TypeParam(parameter) = &def.field_type
        && def.array_depth == 0
    {
        Some(parameter.clone())
    } else {
        None
    };
    DART_FLATTENED_READ.with(|reading| reading.replace(parameter));
    let read = flattened_read(def, handed, taken);
    DART_FLATTENED_READ.with(|reading| reading.replace(None));
    read
}

/// The read of the type parameter `parameter` from `expr`: through the converter of its flattened
/// reads where a holder hands one and `expr` is the flattened member that parameter is.
fn parameter_read(parameter: &str, expr: &str) -> String {
    let converter = from_json_argument(parameter);
    DART_FLATTENED_READ
        .with(|reading| reading.borrow().as_deref() == Some(parameter))
        .then(|| flattened_argument(parameter))
        .flatten()
        .map_or_else(
            || format!("{converter}({expr})"),
            |named| format!("({named} ?? {converter})({expr})"),
        )
}

/// The read [`flattened_decode`] answers.
fn flattened_read(def: &FieldDef, handed: &str, taken: &[String]) -> String {
    let functions = member_functions(def);
    let holds_a_map = matches!(def.field_type, FieldDefType::Map(..));
    if def.is_optional() && !functions.is_empty() && !holds_a_map {
        return corrected_option(def, functions, handed);
    }
    if !functions.is_empty() {
        return member_decode(def, handed, None, Read::Unjudged);
    }
    // `handed` is typed `Map<String, dynamic>` already, which a map is read from uncast.
    let read = if let FieldDefType::Map(key, value) = &def.field_type
        && def.array_depth == 0
    {
        map_decode(key, value, handed, Read::Unjudged)
    } else {
        dart_decode_at(def, def.array_depth, handed, Read::Unjudged)
    };
    if !def.is_optional() {
        return read;
    }
    let caught = format!("(() {{ try {{ return {read}; }} catch (_) {{ return null; }} }})()");
    if let FieldDefType::TypeParam(parameter) = &def.field_type
        && def.array_depth == 0
        && let Some(named) = looked_up_argument(parameter)
    {
        // The keys of what fills the parameter, where a holder hands them.
        return format!(
            "({named} == null ? {caught} : ({handed}.keys.any({named}.contains) ? {read} : null))"
        );
    }
    let keys = looked_for_by(def, taken);
    if keys.is_empty() {
        return caught;
    }
    format!(
        "({handed}.keys.any(const [{}].contains) ? {read} : null)",
        listed_names(&keys)
    )
}

/// The read of an optional member typed `def` flattened into its holder from `handed`, the object
/// it is handed, through `functions`, as the holder's Zod schema reads it: the type it holds, read
/// from what they answer, and where that read throws, an answer of `null` among it, no value where
/// `handed` holds none of a struct's keys, or no key at all for a type whose keys are not known,
/// and refused otherwise. An enum's tag stands for none of its variants' members.
fn corrected_option(def: &FieldDef, functions: &[FunctionName], handed: &str) -> String {
    let answer = declared("corrected");
    let read = dart_decode_at(def, def.array_depth, &answer, Read::Unjudged);
    let keys = struct_keys(def);
    let held = if keys.is_empty() {
        format!("{handed}.isNotEmpty")
    } else {
        format!(
            "{handed}.keys.any(const [{}].contains)",
            listed_names(&keys)
        )
    };
    format!(
        "switch ({}) {{ final {answer} => (() {{ try {{ return {read}; }} catch (_) {{ if ({held}) \
         rethrow; return null; }} }})() }}",
        corrected(functions, handed)
    )
}

/// The object `json` names without its first entry, in the object's order, under one of `names`:
/// what serde hands a member flattened after an externally tagged enum, which takes that entry.
fn without_first(json: &str, names: &[String]) -> String {
    format!(
        "(switch ({{...{json}}}) {{ final rest => \
         rest..remove(rest.keys.where(const [{}].contains).firstOrNull) }})",
        listed_names(names)
    )
}

/// The expression that encodes `field`'s Dart value `expr` into a `jsonEncode`-safe value (`null`,
/// `bool`, `num`, `String`, `List<dynamic>` or `Map<String, dynamic>`) — the whole field including
/// its outer optionality, mirroring [`dart_decode_expr`].
fn dart_encode_expr(field: &FieldDef, expr: &str, promoted: bool) -> String {
    let unwrapped = dart_encode_at(field, field.array_depth, expr);
    if !field.is_optional() {
        return unwrapped;
    }
    if promoted || unwrapped == expr {
        return format!("({expr} == null ? null : {unwrapped})");
    }
    let asserted = dart_encode_at(field, field.array_depth, &format!("{expr}!"));
    format!("({expr} == null ? null : {asserted})")
}

/// [`dart_encode_expr`] before the outer optionality — the encode counterpart of [`dart_decode_at`].
fn dart_encode_at(field: &FieldDef, level: u8, expr: &str) -> String {
    if level > 0 {
        let inner_level = level - 1;
        let inner = dart_encode_at(field, inner_level, "e");
        let mapper = if field.is_nullable_at(inner_level) {
            format!("(e) => (e == null ? null : {inner})")
        } else {
            format!("(e) => {inner}")
        };
        return format!("{expr}.map({mapper}).toList()");
    }
    match &field.field_type {
        // An opaque type parameter (`FieldDef::opaque_parameters`), and a type written as no path.
        FieldDefType::Unknown => expr.to_owned(),
        FieldDefType::TypeParam(name) => format!("{}({expr})", dart_to_json_argument(name)),
        FieldDefType::Tuple(elements) if elements.is_empty() => "null".to_owned(),
        FieldDefType::Tuple(elements) => tuple_encode(elements, expr),
        FieldDefType::Range(kind, bound) => {
            let rendered = kind
                .members()
                .iter()
                .map(|member| {
                    let encoded = dart_encode_expr(bound, &format!("({expr}).{member}"), false);
                    format!("{}: {encoded}", quoted(member))
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("<String, dynamic>{{{rendered}}}")
        }
        FieldDefType::Result(ok, err) => result_encode(ok, err, expr),
        FieldDefType::SiblingType(name, generics) => {
            if let [element] = generics.as_slice()
                && is_sequence_wrapper(name)
            {
                // The field's own levels are walked already: what is left is the wrapper's.
                let combined = field.collection_element_field(element);
                let wrapped = combined.array_depth.saturating_sub(field.array_depth);
                return dart_encode_at(&combined, wrapped, expr);
            }
            if generics.is_empty() {
                format!("({expr}).toJson()")
            } else {
                let converters = generics
                    .iter()
                    .map(|argument| format!("(e) => {}", dart_encode_expr(argument, "e", true)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({expr}).toJson({converters})")
            }
        }
        FieldDefType::Map(key, value) => {
            let key_encode = dart_map_key_encode(key, "e.key");
            let value_encode = dart_encode_expr(value, "e.value", false);
            format!(
                "Map<String, dynamic>.fromEntries(({expr}).entries.map((e) => MapEntry({key_encode}, {value_encode})))"
            )
        }
        FieldDefType::Boolean
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::F64 => expr.to_owned(),
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32 => {
            forms::number_held(&field.field_type, expr).unwrap_or_else(|| expr.to_owned())
        }
        FieldDefType::Char => forms::text_sent(TextForm::Char, expr),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => format!("({expr}).toJson()"),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            TextForm::of(field).map_or_else(|| expr.to_owned(), |form| forms::text_sent(form, expr))
        }
        // A `DateTime` of no zone is written with its zone, as serde reads a `DateTime<Tz>`.
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => {
            if has_as_number(field) {
                expr.to_owned()
            } else {
                format!("({expr}).toUtc().toIso8601String()")
            }
        }
    }
}

// `validate()` over `&FieldDef`: what each emitted type lists of the value it holds, in the order
// Rust's own `validate()` runs it, each message under the steps serde writes on the way to it.

/// `text` as it is written between two `quote`s in a Dart string, which reads `$` as the start of
/// an interpolation.
pub fn dart_escaped(text: &str, quote: char) -> String {
    let mut escaped = String::with_capacity(text.len());
    for written in text.chars() {
        match written {
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '$' | '\\' => {
                escaped.push('\\');
                escaped.push(written);
            }
            _ => {
                if written == quote {
                    escaped.push('\\');
                }
                escaped.push(written);
            }
        }
    }
    escaped
}

/// `bound` as a Dart number: as Rust writes it, with `.0` on a whole number too large for an
/// `int`.
fn dart_number(bound: f64) -> String {
    let written = bound.to_string();
    if written.parse::<i64>().is_ok() || written.contains('.') {
        written
    } else {
        format!("{written}.0")
    }
}

/// The name a walk binds at `depth`: the outermost takes the bare `word`.
fn binder(word: &str, depth: usize) -> String {
    if depth == 0 {
        word.to_owned()
    } else {
        format!("{word}{depth}")
    }
}

/// `place` as the text a Dart string holds between two `quote`s, its steps joined by dots.
fn dart_place(place: &[Step], quote: char) -> String {
    place
        .iter()
        .map(|step| match step {
            Step::Bound(bound) => {
                if bound
                    .chars()
                    .all(|written| written.is_ascii_alphanumeric() || written == '_')
                {
                    format!("${bound}")
                } else {
                    format!("${{{bound}}}")
                }
            }
            Step::Written(written) => dart_escaped(written, quote),
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// `place` with `step` after it.
fn stepped(place: &[Step], step: Step) -> Vec<Step> {
    let mut within = place.to_vec();
    within.push(step);
    within
}

/// Several elements as the one element a guard or a loop takes, or `None` for none.
fn grouped(mut elements: Vec<String>) -> Option<String> {
    if elements.len() > 1 {
        return Some(format!("...[ {}, ]", elements.join(", ")));
    }
    elements.pop()
}

/// `elements` behind `guard`, an `if` or a `for` written without its body.
fn guarded(guard: &str, elements: Vec<String>) -> Vec<String> {
    grouped(elements)
        .map(|inner| format!("{guard} {inner}"))
        .into_iter()
        .collect()
}

/// The value `field` holds where it holds one, and the depth below it: `expr` itself, or the name
/// the returned guard binds it to where the field is optional.
fn present(field: &FieldDef, expr: &str, depth: usize) -> (String, Option<String>, usize) {
    if field.is_optional() {
        let held = binder("held", depth);
        let guard = format!("if ({expr} case final {held}?)");
        (held, Some(guard), depth.saturating_add(1))
    } else {
        (expr.to_owned(), None, depth)
    }
}

/// What `call` answers, a list of messages, as it is listed under `place`.
fn reported(call: &str, place: &[Step], helpers: &mut Helpers) -> String {
    if place.is_empty() {
        return format!("...{call}");
    }
    helpers.call(Helper::Under);
    format!(
        "for (final violation in {call}) _under('{}', violation)",
        dart_place(place, '\'')
    )
}

/// The length and the patterns `bounds` hold `text` to, a Dart `String`: its Unicode code points
/// are counted, and a pattern matches anywhere in it.
fn text_elements(text: &str, named: Option<&str>, bounds: &Bounds<'_>) -> Vec<String> {
    let length = format!("{text}.runes.length");
    let mut elements = Vec::new();
    if let Some(least) = bounds.min_length {
        let said = dart_violation(Bound::MinLength(least), named, &length);
        elements.push(format!("if ({length} < {least}) {said}"));
    }
    if let Some(most) = bounds.max_length {
        let said = dart_violation(Bound::MaxLength(most), named, &length);
        elements.push(format!("if ({length} > {most}) {said}"));
    }
    for pattern in &bounds.patterns {
        let said = dart_violation(Bound::Pattern(pattern), named, "");
        elements.push(format!(
            "if (!RegExp(\"{}\").hasMatch({text})) {said}",
            dart_escaped(pattern, '"')
        ));
    }
    elements
}

/// The count `bounds` hold `list` to, a Dart `List`.
fn count_elements(list: &str, named: Option<&str>, bounds: &Bounds<'_>) -> Vec<String> {
    let count = format!("{list}.length");
    let mut elements = Vec::new();
    if let Some(least) = bounds.min_length {
        let said = dart_violation(Bound::MinItems(least), named, &count);
        elements.push(format!("if ({count} < {least}) {said}"));
    }
    if let Some(most) = bounds.max_length {
        let said = dart_violation(Bound::MaxItems(most), named, &count);
        elements.push(format!("if ({count} > {most}) {said}"));
    }
    elements
}

/// The range `bounds` hold `number` to, quoted back as `shown` writes it.
fn range_elements(
    number: &str,
    shown: &str,
    named: Option<&str>,
    bounds: &Bounds<'_>,
) -> Vec<String> {
    let mut elements = Vec::new();
    if let Some(least) = bounds.minimum {
        let said = dart_violation(Bound::Minimum(least), named, shown);
        elements.push(format!("if ({number} < {}) {said}", dart_number(least)));
    }
    if let Some(most) = bounds.maximum {
        let said = dart_violation(Bound::Maximum(most), named, shown);
        elements.push(format!("if ({number} > {}) {said}", dart_number(most)));
    }
    elements
}

/// What `bounds` hold `value` to, by what `field` is: a list's count, a string's length and
/// patterns, the text of a named type, or a number's range.
fn measured_elements(
    field: &FieldDef,
    value: &str,
    named: Option<&str>,
    bounds: &Bounds<'_>,
    helpers: &mut Helpers,
) -> Vec<String> {
    if field.is_list() {
        return count_elements(value, named, bounds);
    }
    match &field.field_type {
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            text_elements(value, named, bounds)
        }
        FieldDefType::SiblingType(_, _) => {
            let elements = text_elements(&format!("_boundText({value})"), named, bounds);
            if !elements.is_empty() {
                helpers.call(Helper::BoundText);
            }
            elements
        }
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize => range_elements(value, value, named, bounds),
        FieldDefType::F32 | FieldDefType::F64 | FieldDefType::NumberLiteral(_) => {
            let elements = range_elements(value, &format!("_number({value})"), named, bounds);
            if !elements.is_empty() {
                helpers.call(Helper::Number);
            }
            elements
        }
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Map(_, _)
        | FieldDefType::Range(_, _)
        | FieldDefType::Result(_, _)
        | FieldDefType::Tuple(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown => Vec::new(),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => Vec::new(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => Vec::new(),
    }
}

/// What `bounds` add for the value `field` holds at `expr`, each message led by `place`. An
/// optional is measured where it holds a value.
fn bound_elements(
    field: &FieldDef,
    expr: &str,
    place: &[Step],
    bounds: &Bounds<'_>,
    depth: usize,
    helpers: &mut Helpers,
) -> Vec<String> {
    let (value, guard, _) = present(field, expr, depth);
    let named = (!place.is_empty()).then(|| dart_place(place, '"'));
    let measured = measured_elements(field, &value, named.as_deref(), bounds, helpers);
    match guard {
        Some(held) => guarded(&held, measured),
        None => measured,
    }
}

/// The text serde writes for the key `expr` of a map: the key itself where it already is that
/// text, and otherwise what `toJson` writes for it.
fn dart_map_key_text(key: &FieldDef, expr: &str) -> String {
    if dart_map_key_typename(key) == "String" {
        expr.to_owned()
    } else {
        dart_map_key_encode(key, expr)
    }
}

/// What each item of the list `expr` reports, where `of_item` answers for one item given the
/// name it is bound to, its place and the depth below it. A bound names no place, and its walk
/// binds no index.
fn item_elements<F>(
    expr: &str,
    place: &[Step],
    depth: usize,
    asked: Asked,
    of_item: F,
) -> Vec<String>
where
    F: FnOnce(&str, &[Step], usize) -> Vec<String>,
{
    let (index, item) = (binder("index", depth), binder("held", depth));
    let within = stepped(place, Step::Bound(index.clone()));
    let inner = of_item(&item, &within, depth.saturating_add(1));
    let walked = match asked {
        Asked::Bound => format!("for (final {item} in {expr})"),
        Asked::Rules => format!("for (final ({index}, {item}) in {expr}.indexed)"),
    };
    guarded(&walked, inner)
}

/// What the set `expr` adds where it repeats an item, led by `place`. `level` is the set's own
/// list level of `field`, and its items are compared by the JSON the codec writes for each. A
/// value of a type parameter writes its own JSON: the converter of the parameter is the codec's.
fn repeat_elements(
    field: &FieldDef,
    level: u8,
    expr: &str,
    place: &[Step],
    helpers: &mut Helpers,
) -> Vec<String> {
    let item_level = level.saturating_sub(1);
    let mut opaque = field.clone();
    opaque.opaque_parameters();
    let written = dart_encode_at(&opaque, item_level, "e");
    let items = if written == "e" {
        expr.to_owned()
    } else if field.is_nullable_at(item_level) {
        format!("{expr}.map((e) => (e == null ? null : {written}))")
    } else {
        format!("{expr}.map((e) => {written})")
    };
    helpers.call(Helper::Repeats);
    let named = (!place.is_empty()).then(|| dart_place(place, '"'));
    vec![format!(
        "if (_repeats({items})) {}",
        dart_repeated(named.as_deref())
    )]
}

/// What the member `field` adds where its own value is a set that repeats an item, at `expr`,
/// where it holds a value.
fn own_repeat_elements(
    field: &FieldDef,
    expr: &str,
    place: &[Step],
    depth: usize,
    helpers: &mut Helpers,
) -> Vec<String> {
    if !field.is_set() {
        return Vec::new();
    }
    let listed = field.without_own_set();
    let (value, guard, _) = present(field, expr, depth);
    let repeated = repeat_elements(&listed, listed.array_depth, &value, place, helpers);
    match guard {
        Some(held) => guarded(&held, repeated),
        None => repeated,
    }
}

/// [`held_at`] at the list `expr`, whose items are at level `inner_level` of `field`: the repeat
/// of a set where every message is asked, then what each item reports.
fn listed_at(
    field: &FieldDef,
    inner_level: u8,
    expr: &str,
    place: &[Step],
    depth: usize,
    asked: Asked,
    helpers: &mut Helpers,
) -> Vec<String> {
    let mut elements = if matches!(asked, Asked::Rules) && field.is_set_at(inner_level) {
        repeat_elements(field, inner_level.saturating_add(1), expr, place, helpers)
    } else {
        Vec::new()
    };
    elements.extend(item_elements(
        expr,
        place,
        depth,
        asked,
        |item, within, below| {
            if field.is_nullable_at(inner_level) {
                let held = binder("held", below);
                let inner = held_at(
                    field,
                    inner_level,
                    &held,
                    within,
                    below.saturating_add(1),
                    asked,
                    helpers,
                );
                guarded(&format!("if ({item} case final {held}?)"), inner)
            } else {
                held_at(field, inner_level, item, within, below, asked, helpers)
            }
        },
    ));
    elements
}

/// [`held_elements`] below the optional `field` may be: `level` list levels are still to walk,
/// and at none the value is what `field_type` names.
fn held_at(
    field: &FieldDef,
    level: u8,
    expr: &str,
    place: &[Step],
    depth: usize,
    asked: Asked,
    helpers: &mut Helpers,
) -> Vec<String> {
    if let Some(inner_level) = level.checked_sub(1) {
        return listed_at(field, inner_level, expr, place, depth, asked, helpers);
    }
    match &field.field_type {
        FieldDefType::SiblingType(name, arguments) => {
            if let [element] = arguments.as_slice()
                && is_sequence_wrapper(name)
            {
                // The field's own levels are walked already: what is left is the wrapper's.
                let combined = field.collection_element_field(element);
                let wrapped = combined.array_depth.saturating_sub(field.array_depth);
                return held_at(&combined, wrapped, expr, place, depth, asked, helpers);
            }
            vec![match asked {
                Asked::Bound => {
                    helpers.call(Helper::HeldBound);
                    format!("..._heldBound({expr})")
                }
                Asked::Rules => reported(&format!("{expr}.validate()"), place, helpers),
            }]
        }
        FieldDefType::Map(key, value) => {
            let entry = binder("entry", depth);
            let within = stepped(
                place,
                Step::Bound(dart_map_key_text(key, &format!("{entry}.key"))),
            );
            let inner = held_elements(
                value,
                &format!("{entry}.value"),
                &within,
                depth.saturating_add(1),
                asked,
                helpers,
            );
            guarded(&format!("for (final {entry} in {expr}.entries)"), inner)
        }
        FieldDefType::Tuple(positions) => {
            slot_elements(positions, expr, place, depth, asked, helpers)
        }
        FieldDefType::Range(kind, bound) => {
            let mut elements = Vec::new();
            for member in kind.members() {
                elements.extend(member_elements(
                    bound,
                    &format!("{expr}.{member}"),
                    &stepped(place, Step::Written((*member).to_owned())),
                    depth,
                    asked,
                    helpers,
                ));
            }
            elements
        }
        FieldDefType::Result(ok, err) => {
            result_elements([ok, err], expr, place, depth, asked, helpers)
        }
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::Unknown
        | FieldDefType::Usize => Vec::new(),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => Vec::new(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => Vec::new(),
    }
}

/// What the arms of the `Result` `expr` report of the value the one it holds holds, each message
/// under the key serde writes the arm under.
fn result_elements(
    [ok, err]: [&FieldDef; 2],
    expr: &str,
    place: &[Step],
    depth: usize,
    asked: Asked,
    helpers: &mut Helpers,
) -> Vec<String> {
    let class = result_class(ok, err);
    let held = binder("held", depth);
    let mut elements = Vec::new();
    for (arm, def) in RESULT_ARMS.into_iter().zip([ok, err]) {
        if def.is_unit() {
            continue;
        }
        let inner = held_elements(
            def,
            &held,
            &stepped(place, Step::Written(arm.to_owned())),
            depth.saturating_add(1),
            asked,
            helpers,
        );
        elements.extend(guarded(
            &format!("if ({expr} case {class}{arm}(value: final {held}))"),
            inner,
        ));
    }
    elements
}

/// What the value `field` holds at `expr` reports of itself, each message under `place`: the
/// `validate()` of a declared type, or the bound of its own where that is `asked`, reached through
/// an optional, each level of a list, a map's values and a tuple's positions. A value behind a
/// type parameter is not reached.
fn held_elements(
    field: &FieldDef,
    expr: &str,
    place: &[Step],
    depth: usize,
    asked: Asked,
    helpers: &mut Helpers,
) -> Vec<String> {
    let (value, guard, below) = present(field, expr, depth);
    let inner = held_at(
        field,
        field.array_depth,
        &value,
        place,
        below,
        asked,
        helpers,
    );
    match guard {
        Some(held) => guarded(&held, inner),
        None => inner,
    }
}

/// What the checks `field` names add: one call per check on the value it holds at `expr`, where
/// it holds one, each message under `place`.
fn check_elements(
    field: &FieldDef,
    expr: &str,
    place: &[Step],
    depth: usize,
    helpers: &mut Helpers,
) -> Vec<String> {
    let Some(meta) = &field.model_schema_prop_meta else {
        return Vec::new();
    };
    let (value, guard, _) = present(field, expr, depth);
    let calls: Vec<String> = meta
        .check
        .iter()
        .map(|check| reported(&format!("{}({value})", check.name), place, helpers))
        .collect();
    match guard {
        Some(held) => guarded(&held, calls),
        None => calls,
    }
}

/// The bounds the member `field` writes for the value it holds at `expr`, each message led by
/// `place`. A literal is the only rule of its member.
fn own_bound_elements(
    field: &FieldDef,
    expr: &str,
    place: &[Step],
    depth: usize,
    helpers: &mut Helpers,
) -> Vec<String> {
    let Some(meta) = &field.model_schema_prop_meta else {
        return Vec::new();
    };
    if let Some(literal) = &meta.literal {
        let (value, guard, below) = present(field, expr, depth);
        let held = literal_elements(
            field,
            field.array_depth,
            literal,
            &value,
            place,
            below,
            helpers,
        );
        return match guard {
            Some(around) => guarded(&around, held),
            None => held,
        };
    }
    bound_elements(field, expr, place, &Bounds::of(meta), depth, helpers)
}

/// What `literal` adds for the value at `expr`, `level` list levels of `field` above the value it
/// admits: each item of a list holds it, under its index.
fn literal_elements(
    field: &FieldDef,
    level: u8,
    literal: &LiteralValue,
    expr: &str,
    place: &[Step],
    depth: usize,
    helpers: &mut Helpers,
) -> Vec<String> {
    if let Some(inner_level) = level.checked_sub(1) {
        let (index, item) = (binder("index", depth), binder("held", depth));
        let within = stepped(place, Step::Bound(index.clone()));
        let below = depth.saturating_add(1);
        let inner = if field.is_nullable_at(inner_level) {
            let held = binder("held", below);
            let inside = literal_elements(
                field,
                inner_level,
                literal,
                &held,
                &within,
                below.saturating_add(1),
                helpers,
            );
            guarded(&format!("if ({item} case final {held}?)"), inside)
        } else {
            literal_elements(field, inner_level, literal, &item, &within, below, helpers)
        };
        return guarded(
            &format!("for (final ({index}, {item}) in {expr}.indexed)"),
            inner,
        );
    }
    let mut shown = expr.to_owned();
    let broken = match literal {
        LiteralValue::Bool(true) => format!("!{expr}"),
        LiteralValue::Bool(false) => expr.to_owned(),
        LiteralValue::Str(text) => format!("{expr} != '{}'", dart_escaped(text, '\'')),
        LiteralValue::Integer(_) | LiteralValue::Number(_) => {
            let number = literal.number().unwrap_or_default();
            if matches!(field.field_type, FieldDefType::F32 | FieldDefType::F64) {
                helpers.call(Helper::Number);
                shown = format!("_number({expr})");
                format!("{expr} != {}", dart_number(number))
            } else if let LiteralValue::Integer(value) = literal {
                format!("{expr} != {value}")
            } else {
                format!("{expr} != {}", format_number_literal(number))
            }
        }
    };
    let named = (!place.is_empty()).then(|| dart_place(place, '"'));
    vec![format!(
        "if ({broken}) {}",
        dart_violation(Bound::Literal(literal), named.as_deref(), &shown)
    )]
}

/// What a member adds, in the order Rust's `validate()` runs it: its own bounds, what the value
/// it holds reports, then its checks. A set that
/// repeats an item is reported ahead of its count. Where a bound is `asked`, the bounds name no
/// place, and no repeat and no check is asked.
fn member_elements(
    field: &FieldDef,
    expr: &str,
    place: &[Step],
    depth: usize,
    asked: Asked,
    helpers: &mut Helpers,
) -> Vec<String> {
    let named: &[Step] = match asked {
        Asked::Bound => &[],
        Asked::Rules => place,
    };
    let (mut elements, walked) = match asked {
        Asked::Bound => (Vec::new(), field.clone()),
        Asked::Rules => (
            own_repeat_elements(field, expr, place, depth, helpers),
            field.without_own_set(),
        ),
    };
    let own = own_bound_elements(field, expr, named, depth, helpers);
    let held = held_elements(&walked, expr, place, depth, asked, helpers);
    if OWN_BOUND_FIRST {
        elements.extend(own);
        elements.extend(held);
    } else {
        elements.extend(held);
        elements.extend(own);
    }
    if matches!(asked, Asked::Rules) {
        elements.extend(check_elements(field, expr, place, depth, helpers));
    }
    elements
}

/// What the positions of the record `expr` add, each a member under its position in the array
/// serde writes.
fn slot_elements(
    slots: &[FieldDef],
    expr: &str,
    place: &[Step],
    depth: usize,
    asked: Asked,
    helpers: &mut Helpers,
) -> Vec<String> {
    let mut elements = Vec::new();
    for (slot, at) in slots.iter().zip(0_usize..) {
        elements.extend(member_elements(
            slot,
            &format!("{expr}.${}", at.saturating_add(1)),
            &stepped(place, Step::Written(at.to_string())),
            depth,
            asked,
            helpers,
        ));
    }
    elements
}

/// What the members of a class add, each under the key serde writes it at below `under`. A
/// flattened member writes no key: what it holds and its checks are reported under `under` alone.
fn field_elements(
    fields: &[DartField],
    under: &[Step],
    asked: Asked,
    helpers: &mut Helpers,
) -> Vec<String> {
    let mut elements = Vec::new();
    for field in fields {
        if field.flatten {
            elements.extend(held_elements(
                &field.field_def,
                &field.member,
                under,
                0,
                asked,
                helpers,
            ));
            if matches!(asked, Asked::Rules) {
                elements.extend(check_elements(
                    &field.field_def,
                    &field.member,
                    under,
                    0,
                    helpers,
                ));
            }
        } else {
            elements.extend(member_elements(
                &field.field_def,
                &field.member,
                &stepped(under, Step::Written(field.wire_name.clone())),
                0,
                asked,
                helpers,
            ));
        }
    }
    elements
}

/// The calls of the checks a type names, each on the whole value, its messages as it writes them.
fn own_check_elements(checks: &[FunctionName]) -> Vec<String> {
    checks
        .iter()
        .map(|check| format!("...{}(this)", check.name))
        .collect()
}

/// A type's `validate()` over `rules`, the elements of the list it answers, and each helper they
/// call, as the members of the type's class. A type with no rule answers an empty list.
fn validate_members(rules: &[String], helpers: &Helpers, overrides: bool) -> String {
    let annotation = if overrides { "@override " } else { "" };
    let answered = if rules.is_empty() {
        "const []".to_owned()
    } else {
        format!("[ {}, ]", rules.join(", "))
    };
    let mut written = format!("{annotation}List<String> validate() => {answered};");
    for (called, helper) in [
        (helpers.calls(Helper::Under), UNDER_HELPER),
        (helpers.calls(Helper::BoundText), BOUND_TEXT_HELPER),
        (helpers.calls(Helper::Number), NUMBER_HELPER),
        (helpers.calls(Helper::Repeats), REPEATS_HELPER),
    ] {
        if called {
            written.push(' ');
            written.push_str(helper);
        }
    }
    written
}

/// The def of the type `default_types` declares for `parameter`, or of `String` where it declares
/// none.
fn declared_default(parameter: &str, default_types: &[(Ident, syn::Type)]) -> FieldDef {
    default_types
        .iter()
        .find(|(declared, _)| declared == parameter)
        .map_or_else(
            || get_field_def("value", &syn::parse_quote!(String), ""),
            |(_, filling)| get_field_def("value", filling, ""),
        )
}

/// A brand's `validate()`: a repeat of the set it wraps and its string check over the value, under
/// no place, then the checks it names. A brand over one of its own parameters holds the declared
/// default of that parameter and no other filling. A brand that carries a bound writes
/// `validateBound()` ahead of it, the bound alone, which a variant of an untagged enum that holds
/// the brand is chosen by.
fn brand_validate(value_field: &FieldDef, args: &DartTypeArgs<'_>) -> String {
    let mut helpers = Helpers::default();
    let bounds = Bounds {
        max_length: args.max_length,
        maximum: None,
        min_length: args.min_length,
        minimum: None,
        patterns: args.pattern.into_iter().collect(),
    };
    let mut rules = if let FieldDefType::TypeParam(parameter) = &value_field.field_type
        && !value_field.is_list()
        && !value_field.is_optional()
    {
        let filling = declared_default(parameter, args.default_types);
        let held = binder("held", 0);
        let measured = bound_elements(&filling, &held, &[], &bounds, 1, &mut helpers);
        guarded(
            &format!("if (value case final {} {held})", dart_typename(&filling)),
            measured,
        )
    } else {
        bound_elements(value_field, "value", &[], &bounds, 0, &mut helpers)
    };
    let bound_alone = (!rules.is_empty()).then(|| {
        format!(
            "List<String> validateBound() => [ {}, ]; ",
            rules.join(", ")
        )
    });
    let mut repeated = own_repeat_elements(value_field, "value", &[], 0, &mut helpers);
    repeated.append(&mut rules);
    rules = repeated;
    rules.extend(own_check_elements(args.check));
    format!(
        "{}{}",
        bound_alone.unwrap_or_default(),
        validate_members(&rules, &helpers, false)
    )
}

/// A variant's `validate()`: its members under `under`, the steps serde writes on the way to
/// them, then the checks the enum names, each on the variant.
fn variant_validate(payload: &VariantPayload, under: &[Step], checks: &[FunctionName]) -> String {
    let mut helpers = Helpers::default();
    let mut rules = variant_elements(payload, under, Asked::Rules, &mut helpers);
    rules.extend(own_check_elements(checks));
    validate_members(&rules, &helpers, true)
}

/// What the members of a variant add under `under`, the steps serde writes on the way to them.
fn variant_elements(
    payload: &VariantPayload,
    under: &[Step],
    asked: Asked,
    helpers: &mut Helpers,
) -> Vec<String> {
    match payload {
        VariantPayload::Named(fields) => field_elements(fields, under, asked, helpers),
        VariantPayload::Unit => Vec::new(),
        VariantPayload::Value(held) => member_elements(held, "value", under, 0, asked, helpers),
    }
}

/// `_bounds()` of a variant of an untagged enum, written ahead of its `validate()`: what chooses
/// it once its payload decodes, as serde's read of the enum chooses it. `None` for a variant
/// that holds nothing to ask.
fn variant_bounds(payload: &VariantPayload) -> Option<String> {
    let mut helpers = Helpers::default();
    let rules = variant_elements(payload, &[], Asked::Bound, &mut helpers);
    if rules.is_empty() {
        return None;
    }
    let mut written = format!("List<String> _bounds() => [ {}, ]; ", rules.join(", "));
    if helpers.calls(Helper::HeldBound) {
        written.push_str(HELD_BOUND_HELPER);
        written.push(' ');
    }
    Some(written)
}

/// The body of a judged read over `unjudged`, the call of the unjudged one: the value it
/// answers, refused with what its `validate()` answers of it, joined as the Rust read joins it.
fn judged_body(unjudged: &str) -> String {
    format!(
        "{{ final read = {unjudged}; final violations = read.validate(); if \
         (violations.isNotEmpty) throw FormatException(violations.join('; ')); return read; }}"
    )
}

/// The two reads of the class `named`, each taking its payload as a `kind` and then the
/// converters of `codec`: `fromJson`, which judges what `fromJsonUnjudged` answers, and
/// `fromJsonUnjudged`, which is `unjudged`.
fn read_pair(named: &str, kind: &str, codec: &GenericCodec, unjudged: &Unjudged) -> String {
    format!(
        "factory {named}.fromJson({kind} json{parameters}) {} factory \
         {named}.fromJsonUnjudged({kind} {taken_as}{parameters}) {body}",
        judged_body(&format!(
            "{named}.fromJsonUnjudged(json{})",
            codec.from_json_args
        )),
        parameters = codec.from_json_params,
        taken_as = unjudged.taken_as,
        body = unjudged.body,
    )
}

/// The unjudged read that answers `answer` from the payload it takes. Where the type names
/// functions, they take that payload first, in the order written, and `answer` reads what they
/// answer, held as `held_as` casts it, under no name where `reads` says `answer` reads nothing.
fn unjudged_read(corrects: &Corrects<'_>, held_as: &str, reads: bool, answer: &Answer) -> Unjudged {
    if corrects.own.is_empty() {
        return Unjudged {
            body: match answer {
                Answer::Statements(statements) => format!("{{ {statements} }}"),
                Answer::Value(value) => format!("=> {value};"),
            },
            taken_as: corrects.json.clone(),
        };
    }
    let answered = corrected(corrects.own, &corrects.payload);
    let held = if reads {
        format!("final {} = {answered}{held_as};", corrects.json)
    } else {
        format!("{answered}{held_as};")
    };
    let rest = match answer {
        Answer::Statements(statements) => statements.clone(),
        Answer::Value(value) => format!("return {value};"),
    };
    Unjudged {
        body: format!("{{ {held} {rest} }}"),
        taken_as: corrects.payload.clone(),
    }
}

fn generic_codec(parameters: &[String]) -> GenericCodec {
    let mut from_json_params = parameters.iter().fold(String::new(), |mut acc, parameter| {
        write!(
            acc,
            ", {parameter} Function(dynamic) {}",
            from_json_argument(parameter)
        )
        .unwrap();
        acc
    });
    let mut from_json_args = parameters.iter().fold(String::new(), |mut acc, parameter| {
        write!(acc, ", {}", from_json_argument(parameter)).unwrap();
        acc
    });
    let named_arguments: Vec<(String, String)> = parameters
        .iter()
        .filter_map(|parameter| {
            flattened_argument(parameter)
                .map(|named| (format!("{parameter} Function(dynamic)? {named}"), named))
        })
        .chain(parameters.iter().filter_map(|parameter| {
            looked_up_argument(parameter).map(|named| (format!("List<String>? {named}"), named))
        }))
        .collect();
    if !named_arguments.is_empty() {
        let declared_as: Vec<&str> = named_arguments
            .iter()
            .map(|(declared, _)| declared.as_str())
            .collect();
        write!(from_json_params, ", {{{}}}", declared_as.join(", ")).unwrap();
        for (_, named) in &named_arguments {
            write!(from_json_args, ", {named}: {named}").unwrap();
        }
    }
    let to_json_params = parameters
        .iter()
        .map(|parameter| {
            format!(
                "dynamic Function({parameter}) {}",
                dart_to_json_argument(parameter)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    GenericCodec {
        from_json_args,
        from_json_params,
        to_json_params,
    }
}

/// The arguments the constructor of a class is read with from the object `json` names: each
/// member from its key, where the payload holds it for a member that names a function, and a
/// flattened one at its place, from the entries neither `tag_key`, a member of the holder's own nor
/// a member flattened ahead of it takes, an externally tagged enum from the first of them under a
/// variant's name, an `ObjectId` and a type over one from its `$oid` entry alone, an optional one
/// as no value where none is, an optional struct as no value
/// where none of its keys is, an optional enum under a tag key as no value where neither its tag
/// nor its content key is, an optional map as no value where an entry does not read, and an
/// optional one that names a function as no value where its type does not read from what the
/// function answers and none of a struct's keys, or no key at all, is.
fn constructor_arguments(fields: &[DartField], json: &str, tag_key: Option<&str>) -> String {
    let mut taken_ahead: Vec<String> = tag_key
        .into_iter()
        .chain(
            fields
                .iter()
                .filter(|field| !field.flatten)
                .map(|field| field.wire_name.as_str()),
        )
        .map(str::to_owned)
        .collect();
    // The entry an enum takes depends on the entries left to it, so the keys taken ahead of one
    // are removed first, into `source`, and the keys from `removed` on after it.
    let mut source = json.to_owned();
    let mut removed = 0;
    fields.iter().fold(String::new(), |mut acc, field| {
        let decode = if field.flatten {
            let keys: Vec<&str> = taken_ahead[removed..].iter().map(String::as_str).collect();
            let handed = without(&source, &keys);
            let def = field.wire_def();
            // A flattened member is walked whatever it writes.
            let decode = match taken_by(def) {
                Taken::Keys(taken) | Taken::Tagged(taken) => {
                    let whole = if reads_the_whole(def) { json } else { &handed };
                    let decode = flattened_decode(def, whole, &taken);
                    for key in taken {
                        if !taken_ahead.contains(&key) {
                            taken_ahead.push(key);
                        }
                    }
                    decode
                }
                Taken::Only(taken) => {
                    let decode = flattened_decode(def, &only(&handed, &taken), &taken);
                    for key in taken {
                        if !taken_ahead.contains(&key) {
                            taken_ahead.push(key);
                        }
                    }
                    decode
                }
                Taken::FirstOf(names) => {
                    source = without_first(&handed, &names.concat());
                    removed = taken_ahead.len();
                    let first = first_of(&handed, named_type(def), &names);
                    first_entry_decode(def, &first)
                }
            };
            if field.read_as.is_some() {
                format!("{}({decode})", dart_typename(&field.field_def))
            } else {
                decode
            }
        } else {
            let read = Read::Unjudged;
            let key = quoted(&field.wire_name);
            let held = format!("{json}.containsKey({key})");
            let present = member_decode(
                &field.field_def,
                &format!("{json}[{key}]"),
                Some(&held),
                read,
            );
            // The default is read as a payload is, and no function is called for it.
            match &field.default_at {
                Some(at) => {
                    let absent =
                        dart_decode_expr(&field.field_def, &format!("jsonDecode('{at}')"), read);
                    format!("{held} ? ({present}) : ({absent})")
                }
                None => keyed_decode(
                    &field.field_def,
                    present,
                    Some(&held),
                    Some(&field.wire_name),
                ),
            }
        };
        write!(acc, "{}: {decode},", field.member).unwrap();
        acc
    })
}

/// The constructor, field declarations, and `toJson` body for a set of [`DartField`]s — shared
/// by a named-field struct and a struct-shaped (`Named`) enum variant. `extra_to_json` entries
/// are written into the `toJson` map ahead of the fields (a discriminant key, say).
fn class_body_parts(fields: &[DartField], extra_to_json: &[String]) -> ClassBodyParts {
    let ctor_params: String = fields
        .iter()
        .map(|field| {
            if field.required {
                format!("required this.{},", field.member)
            } else {
                format!("this.{},", field.member)
            }
        })
        .collect();
    let field_decls = fields.iter().fold(String::new(), |mut acc, field| {
        write!(
            acc,
            "final {} {};",
            dart_typename(&field.field_def),
            field.member
        )
        .unwrap();
        acc
    });
    let to_json_entries: String = fields
        .iter()
        .filter(|field| !field.never_written)
        .map(|field| {
            field.read_as.as_ref().map_or_else(
                || {
                    let encode = dart_encode_expr(&field.field_def, &field.member, false);
                    if field.flatten {
                        let entries = joined_entries(&field.field_def, &encode);
                        if field.field_def.is_optional() {
                            format!("if ({} != null) ...{entries},", field.member)
                        } else {
                            format!("...{entries},")
                        }
                    } else if field.required {
                        format!("{}: {encode},", quoted(&field.wire_name))
                    } else {
                        format!(
                            "if ({} != null) {}: {encode},",
                            field.member,
                            quoted(&field.wire_name)
                        )
                    }
                },
                |held| {
                    let value = format!("{}.value", field.member);
                    let entries = joined_entries(held, &dart_encode_expr(held, &value, false));
                    format!("if ({value} != null) ...{entries},")
                },
            )
        })
        .collect();
    let extra: String = extra_to_json.join(" ");
    ClassBodyParts {
        ctor_params,
        field_decls,
        to_json_map: format!("{{ {extra}{to_json_entries} }}"),
    }
}

/// The two reads of the class `class_name` over `fields`: the unjudged one calls what `corrects`
/// names on the object it takes, and reads each member from what they answer, under its wire name
/// or an alias. `tag_key` is the key that object holds its variant's tag under.
fn class_reads(
    class_name: &str,
    fields: &[DartField],
    tag_key: Option<&str>,
    codec: &GenericCodec,
    corrects: &Corrects<'_>,
) -> String {
    tag_checked_reads(class_name, fields, tag_key, None, codec, corrects)
}

/// [`class_reads`], which, handed `own_tag`, a struct's own tag key and the struct's name, refuses
/// an object that holds any other value under the key, or none, ahead of the members.
fn tag_checked_reads(
    class_name: &str,
    fields: &[DartField],
    tag_key: Option<&str>,
    own_tag: Option<&(String, String)>,
    codec: &GenericCodec,
    corrects: &Corrects<'_>,
) -> String {
    let named = declared("named");
    let (naming, read_from) = named_by_aliases(fields, &corrects.json, &named).map_or_else(
        || (String::new(), corrects.json.as_str()),
        |naming| (format!("{naming} "), named.as_str()),
    );
    let arguments = constructor_arguments(fields, read_from, tag_key);
    let answer = match own_tag {
        Some((key, held)) => Answer::Statements(format!(
            "{naming}{} return {class_name}({arguments});",
            own_tag_held(read_from, key, held)
        )),
        None if naming.is_empty() => Answer::Value(format!("{class_name}({arguments})")),
        None => Answer::Statements(format!("{naming}return {class_name}({arguments});")),
    };
    let unjudged = unjudged_read(
        corrects,
        " as Map<String, dynamic>",
        !fields.is_empty() || own_tag.is_some(),
        &answer,
    );
    read_pair(class_name, "Map<String, dynamic>", codec, &unjudged)
}

/// The statements that refuse the object `object` names where it holds no `key`, or holds another
/// value than `named` under it: the tag of a struct under its own tag, which TypeScript and Zod
/// refuse in the same places, where serde reads past it.
fn own_tag_held(object: &str, key: &str, named: &str) -> String {
    let key_text = dart_escaped(key, '\'');
    let name_text = dart_escaped(named, '\'');
    format!(
        "if (!{object}.containsKey('{key_text}')) throw FormatException('missing field \
         `{key_text}`'); if ({object}['{key_text}'] != '{name_text}') throw \
         FormatException('expected tag \"{name_text}\", found ' + \
         jsonEncode({object}['{key_text}']));"
    )
}

/// What copies the object `json` names into `named`, each entry under an alias of a member of
/// `fields` moved to the member's wire name in its place, and refuses a member given under two of
/// its names, as serde reads them. `None` where no member has an alias.
fn named_by_aliases(fields: &[DartField], json: &str, named: &str) -> Option<String> {
    let arms: Vec<String> = fields
        .iter()
        .filter(|field| !field.flatten && !field.aliases.is_empty())
        .map(|field| format!("{} => {}", any_of(&field.aliases), quoted(&field.wire_name)))
        .collect();
    if arms.is_empty() {
        return None;
    }
    Some(format!(
        "final {named} = <String, dynamic>{{}}; for (final entry in {json}.entries) {{ final key = \
         switch (entry.key) {{ {}, final other => other }}; if ({named}.containsKey(key)) throw \
         FormatException('duplicate field `$key`'); {named}[key] = entry.value; }}",
        arms.join(", ")
    ))
}

/// The Dart class body, everything a class's own braces hold, for a set of [`DartField`]s: shared
/// by a named-field struct and a `Named` enum variant under internal tagging, whose tag key merges
/// into this map through `extra_to_json`. `reads` is what [`class_reads`] answers for it.
fn class_body_content(
    class_name: &str,
    fields: &[DartField],
    extra_to_json: &[String],
    codec: &GenericCodec,
    reads: &str,
    validate: &str,
) -> String {
    let parts = class_body_parts(fields, extra_to_json);
    format!(
        "const {class_name}({ctor_group}); {field_decls} {reads} \
         Map<String, dynamic> toJson({tp}) => {to_json_map}; {validate}",
        ctor_group = parts.ctor_parameter_group(),
        field_decls = parts.field_decls,
        to_json_map = parts.to_json_map,
        tp = codec.to_json_params,
    )
}

/// [`class_body_content`], wrapped as the standalone `class X<T> { … }` a named-field struct
/// publishes (as opposed to a `Named` enum variant's subclass, which wraps the same content in its
/// own `extends` declaration instead — see [`tagged_variant_tokens`]).
fn class_body(
    class_name: &str,
    generic_params: &str,
    fields: &[DartField],
    extra_to_json: &[String],
    codec: &GenericCodec,
    reads: &str,
    validate: &str,
) -> String {
    let content = class_body_content(class_name, fields, extra_to_json, codec, reads, validate);
    format!("class {class_name}{generic_params} {{ {content} }}")
}

/// The Dart tokens a named-field struct earns: a class, plus a `typedef` under its own Rust ident
/// when `name = "..."` moved its published name elsewhere.
fn struct_dart_tokens(item_struct: &ItemStruct, args: &DartTypeArgs<'_>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    if let Some(held) = brand_field(item_struct) {
        let value_field = brand_value_field(held, &type_parameters, args);
        // A brand's `validate()` runs nothing of what it wraps, which judges itself.
        return value_wrapper_tokens(
            &item_struct.ident,
            &item_struct.generics,
            args.name_override,
            &Held::Value(&value_field),
            Read::Judged,
            &Corrects::of(args.preprocess),
            &brand_validate(&value_field, args),
        );
    }
    if matches!(item_struct.fields, Fields::Unnamed(_)) {
        return tuple_struct_dart_tokens(item_struct, args);
    }

    let rust_ident = item_struct.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_dart_name(&rust_ident, &export_name);
    note_writes_an_object(&rust_ident, true);

    record_dart_fields(
        &rust_ident,
        item_struct
            .fields
            .iter()
            .filter_map(|field| Some(field.ident.as_ref()?.unraw().to_string()))
            .collect(),
    );

    let rule = container_rename_rule(&item_struct.attrs);
    record_taken_keys(
        &rust_ident,
        Taken::Keys(keys_taken_when_flattened(
            &item_struct.fields,
            rule,
            args.preprocess,
        )),
    );
    let defaults = SerdeDefaults::of(item_struct);
    let fields = collect_dart_fields(
        &item_struct.fields,
        rule,
        &type_parameters,
        args,
        &|field| defaults.at(None, field),
    );
    let hands = !args.preprocess.is_empty()
        || fields
            .iter()
            .any(|field| field.flatten && !hands_to_none(field.wire_def()));
    DART_HANDED.with(|recorded| {
        recorded.borrow_mut().insert(rust_ident.clone(), hands);
    });
    let generic_params = dart_generic_params(&item_struct.generics);
    let codec = generic_codec(&type_parameters);
    // A struct's own tag is written ahead of its fields, as serde writes it, and read under the
    // struct's name alone, as TypeScript and Zod read it, where serde reads past it.
    let own_tag = struct_tag(item_struct);
    let tag_written: Vec<String> = own_tag
        .iter()
        .map(|(key, named)| format!("{}: {},", quoted(key), quoted(named)))
        .collect();
    let mut helpers = Helpers::default();
    let mut rules = field_elements(&fields, &[], Asked::Rules, &mut helpers);
    rules.extend(own_check_elements(args.check));
    let corrects = Corrects::of(args.preprocess);
    let body = class_body(
        &export_name,
        &generic_params,
        &fields,
        &tag_written,
        &codec,
        &tag_checked_reads(
            &export_name,
            &fields,
            None,
            own_tag.as_ref(),
            &codec,
            &corrects,
        ),
        &validate_members(&rules, &helpers, false),
    );
    let typedef = ident_typedef(&rust_ident, &export_name, &generic_params);
    let dart_source = format!("{body}{typedef}");

    dart_module_tokens(
        &rust_ident,
        item_struct.ident.span(),
        &dart_source,
        &defaults,
    )
}

/// The `FieldDef` of the field a brand is the value of, its type parameters already erased.
fn brand_value_field(
    held: &Field,
    type_parameters: &[String],
    args: &DartTypeArgs<'_>,
) -> FieldDef {
    let mut field_def = field_def_with_prop_meta("value", held, args);
    field_def.erase_type_parameters(type_parameters);
    field_def
}

/// The Dart tokens for a value that carries one wrapped value on the wire: a branded newtype, a
/// tuple struct, or a type alias. Each publishes a class wrapping `value`, whose reads and
/// `toJson` delegate to what `held` reads and writes, and whose `validate()` is `validate`.
/// `read` is how a declared type the value holds is read, which is unjudged where `validate`
/// runs it. The unjudged read calls what `corrects` names on the payload it takes.
fn value_wrapper_tokens(
    ident: &Ident,
    generics: &syn::Generics,
    name_override: Option<&str>,
    held: &Held<'_>,
    read: Read,
    corrects: &Corrects<'_>,
    validate: &str,
) -> TokenStream {
    let rust_ident = ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, name_override);
    register_dart_name(&rust_ident, &export_name);
    note_writes_an_object(&rust_ident, false);
    note_reads_a_unit(
        &rust_ident,
        matches!(held, Held::Value(value_field) if reads_a_unit(value_field)),
    );
    // A brand, a struct of one slot or an alias over an enum that writes a unit variant as its bare
    // name writes it so as well.
    if let Held::Value(value) = held
        && !value.is_list()
        && !value.is_optional()
        && writes_a_name(value)
    {
        record_writes_a_name(&rust_ident);
    }
    // A value that reads one key alone is handed that key alone where it is flattened.
    if let (Held::Value(value), []) = (held, corrects.own)
        && !value.is_optional()
        && let Taken::Only(keys) = taken_by(value)
    {
        record_taken_keys(&rust_ident, Taken::Only(keys));
    }

    let generic_params = dart_generic_params(generics);
    let parameters = type_parameters_in_scope(generics);
    let codec = generic_codec(&parameters);
    let (value_ty, decode, encode) = match held {
        Held::Slots(slots) => (
            tuple_typename(slots),
            tuple_decode(slots, &corrects.json, read),
            tuple_encode(slots, "value"),
        ),
        Held::Value(value_field) => (
            dart_typename(value_field),
            member_decode(value_field, &corrects.json, None, read),
            dart_encode_expr(value_field, "value", false),
        ),
    };
    let unjudged = unjudged_read(
        corrects,
        "",
        decode.contains(corrects.json.as_str()),
        &Answer::Value(format!("{export_name}({decode})")),
    );
    let reads = read_pair(&export_name, "dynamic", &codec, &unjudged);
    let body = format!(
        "class {export_name}{generic_params} {{ const {export_name}(this.value); final {value_ty} value; \
         {reads} dynamic toJson({tp}) => {encode}; {validate} }}",
        tp = codec.to_json_params,
    );
    let typedef = ident_typedef(&rust_ident, &export_name, &generic_params);
    let dart_source = format!("{body}{typedef}");

    dart_module_tokens(
        &rust_ident,
        ident.span(),
        &dart_source,
        &SerdeDefaults::none(),
    )
}

/// The Dart tokens for a non-branded tuple struct. One that declares a single slot is that slot's
/// value, which serde writes whatever the slot's attributes say, and shares
/// [`value_wrapper_tokens`] with a branded newtype; a wider tuple struct wraps the fixed-size Dart
/// record of the slots serde writes, and one of no slot the empty record, read from the empty
/// array.
fn tuple_struct_dart_tokens(item_struct: &ItemStruct, args: &DartTypeArgs<'_>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let Fields::Unnamed(unnamed) = &item_struct.fields else {
        return TokenStream::new();
    };
    let slot_def = |slot: &Field, name: &str| {
        let mut field_def = field_def_with_prop_meta(name, slot, args);
        field_def.erase_type_parameters(&type_parameters);
        field_def
    };
    let mut helpers = Helpers::default();
    if let Some(only) = unnamed.unnamed.first()
        && unnamed.unnamed.len() == 1
    {
        let value_field = slot_def(only, "value");
        // The slot is the value serde writes for the struct, which is no step.
        let mut rules = member_elements(&value_field, "value", &[], 0, Asked::Rules, &mut helpers);
        rules.extend(own_check_elements(args.check));
        return value_wrapper_tokens(
            &item_struct.ident,
            &item_struct.generics,
            args.name_override,
            &Held::Value(&value_field),
            Read::Unjudged,
            &Corrects::of(args.preprocess),
            &validate_members(&rules, &helpers, false),
        );
    }
    let slots: Vec<FieldDef> = unnamed
        .unnamed
        .iter()
        .filter(|slot| !parse_serde_key_omission(&slot.attrs).absent_from_wire())
        .map(|slot| slot_def(slot, "slot"))
        .collect();
    let mut rules = slot_elements(&slots, "value", &[], 0, Asked::Rules, &mut helpers);
    rules.extend(own_check_elements(args.check));
    value_wrapper_tokens(
        &item_struct.ident,
        &item_struct.generics,
        args.name_override,
        &Held::Slots(&slots),
        Read::Unjudged,
        &Corrects::of(args.preprocess),
        &validate_members(&rules, &helpers, false),
    )
}

/// The Dart tokens a `type X = ...;` alias earns: a class wrapping the target's own value, exactly
/// like a branded newtype — see [`value_wrapper_tokens`].
fn alias_dart_tokens(item_type: &ItemType, name_override: Option<&str>) -> TokenStream {
    let export_name = compute_alias_export_name(&item_type.ident.to_string(), name_override);
    let type_parameters = type_parameters_in_scope(&item_type.generics);
    let mut target = get_field_def(&export_name, &item_type.ty, "");
    target.erase_type_parameters(&type_parameters);
    // An alias is another name for its target: where that is a declared type itself, its rules
    // are the target's.
    let named = !target.is_list()
        && !target.is_optional()
        && matches!(target.field_type, FieldDefType::SiblingType(_, _));
    let rules: Vec<String> = named
        .then(|| "...value.validate()".to_owned())
        .into_iter()
        .collect();
    value_wrapper_tokens(
        &item_type.ident,
        &item_type.generics,
        Some(&export_name),
        &Held::Value(&target),
        if named { Read::Unjudged } else { Read::Judged },
        // An alias is refused a `preprocess`: it has no read of its own.
        &Corrects::of(&[]),
        &validate_members(&rules, &Helpers::default(), false),
    )
}

/// The Dart tokens an enum earns: a plain enhanced enum for an all-unit, untagged-by-default
/// declaration; a sealed class hierarchy for every tagged or untagged shape otherwise — mirroring
/// exactly the shape `process_enum` itself dispatches a declaration to.
fn enum_dart_tokens(declared: &ItemEnum, args: &DartTypeArgs<'_>) -> TokenStream {
    // Plain or not as every variant declared reads, as `process_enum` dispatches it.
    let plain = is_plain_enum(declared);
    let item_enum = &variants_on_the_wire(declared);
    let rust_ident = item_enum.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_dart_name(&rust_ident, &export_name);
    note_writes_an_object(&rust_ident, false);
    let tag_attrs = enum_tag_attrs(&item_enum.attrs);
    let names = variant_names_when_flattened(item_enum, args.preprocess);
    record_taken_keys(
        &rust_ident,
        if names.is_empty() {
            Taken::Tagged(tag_and_content_when_flattened(item_enum, args.preprocess))
        } else {
            Taken::FirstOf(names)
        },
    );
    record_tag_keys(&rust_ident, tag_keys(&tag_attrs, args.preprocess));
    DART_HANDED.with(|recorded| {
        recorded
            .borrow_mut()
            .insert(rust_ident.clone(), !args.preprocess.is_empty());
    });
    let writes_bare_variant_names =
        tag_attrs.tag.is_none() && tag_attrs.content.is_none() && !tag_attrs.untagged;
    let defaults = SerdeDefaults::of_enum(item_enum);
    let dart_source = if plain && writes_bare_variant_names {
        plain_enum_dart_source(item_enum, &rust_ident, &export_name, args)
    } else if tag_attrs.untagged {
        untagged_enum_dart_source(
            item_enum,
            &rust_ident,
            &export_name,
            &tag_attrs,
            args,
            &defaults,
        )
    } else {
        tagged_enum_dart_source(
            item_enum,
            &rust_ident,
            &export_name,
            &tag_attrs,
            args,
            &defaults,
        )
    };
    dart_module_tokens(&rust_ident, item_enum.ident.span(), &dart_source, &defaults)
}

/// One tagged/untagged variant's payload — see [`VariantPayload`]. A `TupleMultiple` payload is
/// folded into one `Tuple` [`FieldDef`], matching how a slot list renders everywhere else in this
/// module. A member is read as the default `defaults` holds for it in this variant.
fn variant_payload(
    variant: &Variant,
    field_rule: RenameRule,
    type_parameters: &[String],
    args: &DartTypeArgs<'_>,
    defaults: &SerdeDefaults,
) -> VariantPayload {
    let variant_name = variant.ident.to_string();
    match variant_wire_kind(variant) {
        VariantKind::Unit => VariantPayload::Unit,
        VariantKind::Named => VariantPayload::Named(collect_dart_fields(
            &variant.fields,
            field_rule,
            type_parameters,
            args,
            &|field| defaults.at(Some(&variant_name), field),
        )),
        VariantKind::TupleSingle => {
            let slot = variant.fields.iter().next();
            VariantPayload::Value(Box::new(slot.map_or_else(
                || get_field_def("value", &syn::parse_quote!(()), ""),
                |field| {
                    let mut field_def = field_def_with_prop_meta("value", field, args);
                    field_def.erase_type_parameters(type_parameters);
                    field_def
                },
            )))
        }
        VariantKind::TupleMultiple => {
            VariantPayload::Value(Box::new(written_slots(variant, |field| {
                let mut field_def = field_def_with_prop_meta("slot", field, args);
                field_def.erase_type_parameters(type_parameters);
                field_def
            })))
        }
    }
}

/// The Dart expression reading a tagged variant's own payload slot out of the top-level `json` (or,
/// under external tagging, the `map` the base class's own dispatch already bound): the content key
/// under adjacent tagging, `json` itself under internal tagging (whose object merges the tag beside
/// the payload's own fields), or `map[{entry}]` under external tagging, whose dispatch has already
/// read the variant's name as the object's one key, `entry` being that key. Beside it, the test
/// that the payload holds the slot, where it may not: a content key.
fn tagged_payload_json_expr(
    entry: &str,
    shape: &TagShape<'_>,
    corrects: &Corrects<'_>,
) -> (String, Option<String>) {
    let (json, map) = (&corrects.json, &corrects.map);
    shape.content_key.map_or_else(
        || {
            if shape.merge_tag_into_object {
                (json.clone(), None)
            } else {
                (format!("{map}[{entry}]"), None)
            }
        },
        |content| {
            let key = quoted(content);
            (
                format!("{json}[{key}]"),
                Some(format!("{json}.containsKey({key})")),
            )
        },
    )
}

/// The `toJson` this module writes for a variant whose own object does not carry the tag key beside
/// its fields: `{'{tag_key}': '{wire_tag}', '{content_key}': {own_encode}}` under adjacent tagging,
/// `{'{tag_key}': '{wire_tag}'}` for a `Unit` variant with no content key at all (real adjacent
/// tagging's own shape for one), or `'{wire_tag}': {own_encode}` under external tagging (no
/// `content_key`) — external's own `Unit` case (`own_encode` absent, no `content_key`) is the bare
/// wire string external tagging actually writes for one.
fn tagged_wrap_encode(shape: &TagShape<'_>, wire_tag: &str, own_encode: Option<&str>) -> String {
    let (tag_key, tag) = (quoted(shape.tag_key), quoted(wire_tag));
    match (shape.content_key, own_encode) {
        (Some(content), Some(encode)) => {
            format!("{{ {tag_key}: {tag}, {}: {encode} }}", quoted(content))
        }
        (Some(_), None) => format!("{{ {tag_key}: {tag} }}"),
        (None, Some(encode)) => format!("{{ {tag}: {encode} }}"),
        (None, None) => tag,
    }
}

/// The steps serde writes on the way to a tagged variant's members: the content key where the
/// enum declares one, nothing where the members sit beside the tag, and otherwise the name the
/// variant is written under.
fn tagged_variant_steps(wire_tag: &str, shape: &TagShape<'_>) -> Vec<Step> {
    shape.content_key.map_or_else(
        || {
            if shape.merge_tag_into_object {
                Vec::new()
            } else {
                vec![Step::Written(wire_tag.to_owned())]
            }
        },
        |content| vec![Step::Written(content.to_owned())],
    )
}

/// The Dart tokens for one tagged-union variant: the subclass definition, and the arm of the
/// base's unjudged dispatch (everything after the `case` of its names). `corrects` holds the names
/// the base's read declares, which the read of a variant with named members declares too.
fn tagged_variant_tokens(
    export_name: &str,
    generic_params: &str,
    codec: &GenericCodec,
    variant: &VariantClass<'_>,
    shape: &TagShape<'_>,
    corrects: &Corrects<'_>,
) -> (String, String) {
    let VariantClass {
        aliased,
        payload,
        subclass_name,
        validate,
        wire_tag,
    } = *variant;
    // A variant read under an alias too reads its value from the entry the dispatch took.
    let entry = if aliased {
        corrects.tag.clone()
    } else {
        quoted(wire_tag)
    };
    let (payload_expr, held) = tagged_payload_json_expr(&entry, shape, corrects);
    let extends = format!("extends {export_name}{generic_params}");
    match payload {
        VariantPayload::Unit => {
            let own_encode = if shape.merge_tag_into_object {
                format!("{{ {}: {} }}", quoted(shape.tag_key), quoted(wire_tag))
            } else {
                tagged_wrap_encode(shape, wire_tag, None)
            };
            let class_text = format!(
                "class {subclass_name}{generic_params} {extends} {{ const {subclass_name}(); \
                 @override dynamic toJson({tp}) => {own_encode}; {validate} }}",
                tp = codec.to_json_params,
            );
            let built = format!("{subclass_name}{generic_params}()");
            // Under a content key Zod reads the variant as a strict object of the tag alone, so a
            // content is refused, `null` among it, which serde reads.
            let read = match held {
                Some(holds_content) => format!(
                    "{holds_content} ? throw FormatException('{export_name}: ' + {} + ' takes no \
                     value') : {built}",
                    corrects.tag
                ),
                None => built,
            };
            (class_text, read)
        }
        VariantPayload::Named(fields) => {
            let own = corrects.of_a_variant();
            let class_text = if shape.merge_tag_into_object {
                let extra = vec![format!("{}: {},", quoted(shape.tag_key), quoted(wire_tag))];
                // serde hands a variant the object without its tag.
                let reads = class_reads(subclass_name, fields, Some(shape.tag_key), codec, &own);
                let content =
                    class_body_content(subclass_name, fields, &extra, codec, &reads, validate);
                format!("class {subclass_name}{generic_params} {extends} {{ {content} }}")
            } else {
                // Adjacent or external tagging: the variant's field map carries no tag, so `toJson`
                // wraps it under the tag (and the content key).
                let parts = class_body_parts(fields, &[]);
                let wrapped = tagged_wrap_encode(shape, wire_tag, Some(&parts.to_json_map));
                let reads = class_reads(subclass_name, fields, None, codec, &own);
                format!(
                    "class {subclass_name}{generic_params} {extends} {{ const {subclass_name}({ctor_group}); \
                     {field_decls} {reads} @override dynamic toJson({tp}) => {wrapped}; \
                     {validate} }}",
                    ctor_group = parts.ctor_parameter_group(),
                    field_decls = parts.field_decls,
                    tp = codec.to_json_params,
                )
            };
            let from_json = format!(
                "{subclass_name}{generic_params}.fromJsonUnjudged({payload_expr}{fa})",
                fa = codec.from_json_args,
            );
            (class_text, from_json)
        }
        VariantPayload::Value(field_def) => {
            // serde hands a slot beside its tag the object without the tag.
            let source = if shape.merge_tag_into_object {
                without(&payload_expr, &[shape.tag_key])
            } else {
                payload_expr
            };
            let decode = content_decode(field_def, &source, held.as_deref(), shape.content_key);
            let own_encode = dart_encode_expr(field_def, "value", false);
            let wrapped_encode = if shape.merge_tag_into_object {
                // Internal tagging with a non-`Named` payload: serde admits only an object-shaped
                // one, so the tag merges into that object.
                format!(
                    "{{ {}: {}, ...{} }}",
                    quoted(shape.tag_key),
                    quoted(wire_tag),
                    joined_entries(field_def, &own_encode)
                )
            } else {
                tagged_wrap_encode(shape, wire_tag, Some(&own_encode))
            };
            let class_text = format!(
                "class {subclass_name}{generic_params} {extends} {{ const {subclass_name}(this.value); \
                 final {} value; @override dynamic toJson({tp}) => {wrapped_encode}; {validate} }}",
                dart_typename(field_def),
                tp = codec.to_json_params,
            );
            (
                class_text,
                format!("{subclass_name}{generic_params}({decode})"),
            )
        }
    }
}

/// The Dart tokens a tagged enum (internal, adjacent, or the default external form) earns: a
/// `sealed` base class whose unjudged read dispatches on the tag and whose `fromJson` judges
/// what it answers, and one subclass per variant.
fn tagged_enum_dart_source(
    item_enum: &ItemEnum,
    rust_ident: &str,
    export_name: &str,
    tag_attrs: &EnumTagAttrs,
    args: &DartTypeArgs<'_>,
    defaults: &SerdeDefaults,
) -> String {
    let tag_key = tag_attrs.tag.as_deref().unwrap_or("type");
    let shape = TagShape {
        content_key: tag_attrs.content.as_deref(),
        merge_tag_into_object: tag_attrs.tag.is_some() && tag_attrs.content.is_none(),
        tag_key,
    };
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let type_parameters = type_parameters_in_scope(&item_enum.generics);
    let generic_params = dart_generic_params(&item_enum.generics);
    let codec = generic_codec(&type_parameters);
    let corrects = Corrects::of(args.preprocess);

    let mut subclasses = Vec::new();
    let mut is_string_dispatch_arms = Vec::new();
    let mut map_dispatch_arms = Vec::new();
    // Internal and adjacent tagging read the tag off a named key of the object; external tagging
    // reads it off the object's one key, and answers a bare string for a `Unit` variant.
    let is_external = shape.content_key.is_none() && !shape.merge_tag_into_object;
    for variant in &item_enum.variants {
        let variant_rust_name = variant.ident.to_string();
        let wire_tag = rename_override(&variant.attrs)
            .unwrap_or_else(|| variant_rule.apply_to_variant(&variant_rust_name));
        let names = read_as(wire_tag.clone(), read_aliases(&variant.attrs));
        let subclass_name = format!("{export_name}{variant_rust_name}");
        let payload = variant_payload(variant, field_rule, &type_parameters, args, defaults);
        let validate = variant_validate(
            &payload,
            &tagged_variant_steps(&wire_tag, &shape),
            args.check,
        );
        let (class_text, from_json) = tagged_variant_tokens(
            export_name,
            &generic_params,
            &codec,
            &VariantClass {
                aliased: names.len() > 1,
                payload: &payload,
                subclass_name: &subclass_name,
                validate: &validate,
                wire_tag: &wire_tag,
            },
            &shape,
            &corrects,
        );
        subclasses.push(class_text);
        let arm = format!("case {}: return {from_json};", any_of(&names));
        if is_external && matches!(payload, VariantPayload::Unit) {
            is_string_dispatch_arms.push(arm);
            // serde reads a unit variant from an entry of its name over `null` as well, the one
            // it writes flattened.
            map_dispatch_arms.push(format!(
                "case {}: return {map}[{tag}] == null ? {from_json} : throw \
                 FormatException('{export_name}: ' + {tag} + ' takes no value');",
                any_of(&names),
                map = corrects.map,
                tag = corrects.tag,
            ));
        } else {
            map_dispatch_arms.push(arm);
        }
    }
    if is_external && !is_string_dispatch_arms.is_empty() {
        record_writes_a_name(rust_ident);
    }

    let (json, map, tag) = (&corrects.json, &corrects.map, &corrects.tag);
    let (payload_ty, held_as, dispatch) = if is_external {
        // serde and Zod read the variant from an object of one key, and refuse any other.
        (
            "dynamic",
            "",
            format!(
                "if ({json} is String) {{ switch ({json}) \
                 {{ {} default: throw FormatException('Unknown {export_name}: ' + {json}); }} }} \
                 final {map} = {json} as Map<String, dynamic>; if ({map}.isEmpty) {{ throw \
                 FormatException('empty object for {export_name}'); }} if ({map}.length > 1) {{ \
                 throw FormatException('more than one key for {export_name}'); }} \
                 final {tag} = {map}.keys.first; switch ({tag}) \
                 {{ {} default: throw FormatException('Unknown {export_name}: ' + {tag}); }}",
                is_string_dispatch_arms.join(" "),
                map_dispatch_arms.join(" "),
            ),
        )
    } else {
        (
            "Map<String, dynamic>",
            " as Map<String, dynamic>",
            format!(
                "final {tag} = {json}[{}] as String; switch ({tag}) {{ {} default: \
                 throw FormatException('Unknown {export_name}: ' + {tag}); }}",
                quoted(tag_key),
                map_dispatch_arms.join(" "),
            ),
        )
    };
    let unjudged = unjudged_read(&corrects, held_as, true, &Answer::Statements(dispatch));
    let reads = read_pair(export_name, payload_ty, &codec, &unjudged);

    let base = format!(
        "sealed class {export_name}{generic_params} {{ const {export_name}(); {reads} \
         dynamic toJson({tp}); {VALIDATE_DECLARED} }}",
        tp = codec.to_json_params,
    );
    let typedef = ident_typedef(rust_ident, export_name, &generic_params);
    format!("{base} {}{typedef}", subclasses.join(" "))
}

/// The Dart tokens a `#[serde(untagged)]` enum earns: a `sealed` base class whose unjudged read
/// tries each variant's own decode in turn (Dart has no runtime structural type inspection to
/// dispatch on ahead of time, unlike Zod's `z.union`), returning the first that does not throw and
/// whose `_bounds()` hold, and whose `fromJson` judges what that answers.
fn untagged_enum_dart_source(
    item_enum: &ItemEnum,
    rust_ident: &str,
    export_name: &str,
    tag_attrs: &EnumTagAttrs,
    args: &DartTypeArgs<'_>,
    defaults: &SerdeDefaults,
) -> String {
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let type_parameters = type_parameters_in_scope(&item_enum.generics);
    let generic_params = dart_generic_params(&item_enum.generics);
    let codec = generic_codec(&type_parameters);
    let extends = format!("extends {export_name}{generic_params}");
    let corrects = Corrects::of(args.preprocess);
    let (json, read) = (&corrects.json, &corrects.read);

    let mut subclasses = Vec::new();
    let mut attempts = Vec::new();
    let mut reads_unit = false;
    for variant in &item_enum.variants {
        let variant_rust_name = variant.ident.to_string();
        let subclass_name = format!("{export_name}{variant_rust_name}");
        let payload = variant_payload(variant, field_rule, &type_parameters, args, defaults);
        reads_unit |= matches!(&payload, VariantPayload::Value(held) if reads_a_unit(held));
        let bounds = variant_bounds(&payload);
        // serde writes an untagged variant as its value, so no step leads to its members.
        let validate = format!(
            "{}{}",
            bounds.as_deref().unwrap_or_default(),
            variant_validate(&payload, &[], args.check)
        );
        let (class_text, from_json) = match &payload {
            VariantPayload::Named(fields) => {
                let reads = class_reads(
                    &subclass_name,
                    fields,
                    None,
                    &codec,
                    &corrects.of_a_variant(),
                );
                let content =
                    class_body_content(&subclass_name, fields, &[], &codec, &reads, &validate);
                (
                    format!("class {subclass_name}{generic_params} {extends} {{ {content} }}"),
                    format!(
                        "{subclass_name}{generic_params}.fromJsonUnjudged({json} as Map<String, dynamic>{fa})",
                        fa = codec.from_json_args,
                    ),
                )
            }
            VariantPayload::Value(field_def) => {
                let decode = member_decode(field_def, json, None, Read::Unjudged);
                let encode = dart_encode_expr(field_def, "value", false);
                (
                    format!(
                        "class {subclass_name}{generic_params} {extends} {{ const {subclass_name}(this.value); \
                         final {} value; @override dynamic toJson({tp}) => {encode}; {validate} }}",
                        dart_typename(field_def),
                        tp = codec.to_json_params,
                    ),
                    format!("{subclass_name}{generic_params}({decode})"),
                )
            }
            VariantPayload::Unit => {
                // Refused for every member of a real untagged enum, so this arm exists only to keep
                // the match exhaustive.
                (
                    format!(
                        "class {subclass_name}{generic_params} {extends} {{ const {subclass_name}(); \
                         @override dynamic toJson({tp}) => null; {validate} }}",
                        tp = codec.to_json_params,
                    ),
                    format!("{subclass_name}{generic_params}()"),
                )
            }
        };
        subclasses.push(class_text);
        attempts.push(if bounds.is_some() {
            format!(
                "try {{ final {read} = {from_json}; if ({read}._bounds().isEmpty) return {read}; }} \
                 catch (_) {{}}"
            )
        } else {
            format!("try {{ return {from_json}; }} catch (_) {{}}")
        });
    }

    note_reads_a_unit(rust_ident, reads_unit);
    let unjudged = unjudged_read(
        &corrects,
        "",
        true,
        &Answer::Statements(format!(
            "{} throw FormatException('No variant of {export_name} matched');",
            attempts.join(" ")
        )),
    );
    let reads = read_pair(export_name, "dynamic", &codec, &unjudged);
    let base = format!(
        "sealed class {export_name}{generic_params} {{ const {export_name}(); {reads} \
         dynamic toJson({tp}); {VALIDATE_DECLARED} }}",
        tp = codec.to_json_params,
    );
    let typedef = ident_typedef(rust_ident, export_name, &generic_params);
    format!("{base} {}{typedef}", subclasses.join(" "))
}

/// One plain-enum variant's Dart member name (lower-camel of its Rust ident, moved off a name
/// Dart will not take; `written` is every member's lower-camel name), and its wire value
/// followed by each alias serde reads it under.
fn plain_enum_member(
    variant: &Variant,
    rule: RenameRule,
    written: &[String],
) -> (String, Vec<String>) {
    let rust_name = variant.ident.to_string();
    let member_name = dart_member(
        &dart_lower_camel(&rust_name),
        &DART_ENUM_NAMES_TAKEN,
        written,
    );
    let wire = rename_override(&variant.attrs).unwrap_or_else(|| rule.apply_to_variant(&rust_name));
    (member_name, read_as(wire, read_aliases(&variant.attrs)))
}

/// The Dart tokens a plain (all-unit, string-wire) enum earns: a `String`-backed enhanced enum,
/// whose `validate()` runs the checks the enum names, and whose `fromJson` judges where it
/// names one. `fromJsonUnjudged` is what a holder reads it by, and calls the functions the
/// enum's `preprocess` names on the text it takes.
fn plain_enum_dart_source(
    item_enum: &ItemEnum,
    rust_ident: &str,
    export_name: &str,
    args: &DartTypeArgs<'_>,
) -> String {
    let checks = args.check;
    let corrects = Corrects::of(args.preprocess);
    let json = &corrects.json;
    let rule = container_rename_rule(&item_enum.attrs);
    let written: Vec<String> = item_enum
        .variants
        .iter()
        .map(|variant| dart_lower_camel(&variant.ident.to_string()))
        .collect();
    let members: Vec<(String, Vec<String>)> = item_enum
        .variants
        .iter()
        .map(|variant| plain_enum_member(variant, rule, &written))
        .collect();
    let member_list = members
        .iter()
        .map(|(name, names)| format!("{name}({})", quoted(&names[0])))
        .collect::<Vec<_>>()
        .join(", ");
    let match_arms = members
        .iter()
        .fold(String::new(), |mut acc, (name, names)| {
            write!(acc, "{} => {export_name}.{name},", any_of(names)).unwrap();
            acc
        });
    let validate = validate_members(&own_check_elements(checks), &Helpers::default(), false);
    let unjudged = unjudged_read(
        &corrects,
        " as String",
        true,
        &Answer::Value(format!(
            "switch ({json}) {{ {match_arms} _ => throw FormatException('Unknown {export_name}: ' \
             + {json}) }}"
        )),
    );
    let (matched, taken_as) = (&unjudged.body, &unjudged.taken_as);
    // An enum that names no check has nothing to refuse, and one that names no function either
    // reads as it did.
    let reads = if !checks.is_empty() {
        format!(
            "static {export_name} fromJson(String json) {} static {export_name} \
             fromJsonUnjudged(String {taken_as}) {matched}",
            judged_body(&format!("{export_name}.fromJsonUnjudged(json)"))
        )
    } else if corrects.own.is_empty() {
        format!(
            "static {export_name} fromJson(String json) {matched} static {export_name} \
             fromJsonUnjudged(String json) => fromJson(json);"
        )
    } else {
        format!(
            "static {export_name} fromJson(String json) => fromJsonUnjudged(json); static \
             {export_name} fromJsonUnjudged(String {taken_as}) {matched}"
        )
    };
    let body = format!(
        "enum {export_name} {{ {member_list}; const {export_name}(this.wireValue); final String wireValue; \
         {reads} String toJson() => wireValue; {validate} }}"
    );
    let typedef = ident_typedef(rust_ident, export_name, "");
    format!("{body}{typedef}")
}

#[cfg(test)]
mod tests;
