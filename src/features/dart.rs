//! Dart type and JSON-codec generation.

use core::cell::RefCell;
use core::fmt::Write as _;
use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt as _;
use syn::{Field, Fields, Ident, Item, ItemEnum, ItemStruct, ItemType, Variant};

use crate::bound_message::{Bound, dart_violation};
use crate::features::model_schema_prop::{FunctionName, ModelSchemaPropMeta};
use crate::features::serde::defaults::{SerdeDefaults, substituted};
use crate::features::serde::{brand_field, parse_serde_key_omission, struct_tag};
use crate::field_type::{
    FieldDef, FieldDefType, VariantKind, classify_variant, dart_from_json_argument,
    dart_lower_camel, dart_to_json_argument, get_field_def, is_plain_enum, is_sequence_wrapper,
};
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
    /// Where the JSON of the default serde reads for an absent key is written.
    default_at: Option<String>,
    field_def: FieldDef,
    flatten: bool,
    member: String,
    required: bool,
    wire_name: String,
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

    /// This read for what the member `field` holds: judged where the member's own bound stands
    /// in for what its type declares, so that no `validate()` of the holder runs it.
    fn of_member(self, field: &FieldDef) -> Self {
        if walks_what_it_holds(field) {
            self
        } else {
            Self::Judged
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

    /// The name of each function the reads of the item being emitted call: what its `preprocess`
    /// names, and that of each of its members.
    static DART_CALLED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// The Dart tokens `item` earns, given what `args` reads of its `#[model_schema(...)]`, handed
/// over field by field rather than by widening that struct's visibility. Dispatches on the item's
/// own shape; an item this module has nothing to say about (anything but a struct, an enum or a
/// type alias) earns nothing.
pub fn dart_schema_dispatch(item: &Item, args: &DartTypeArgs<'_>) -> TokenStream {
    record_called(called_by(item, args));
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
    tokens
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
    let module_ident = dart_module_ident(rust_ident, span);
    let holder = defaults.holder();
    if holder.is_empty() {
        return quote! {
            pub mod #module_ident {
                pub fn dart_definition() -> ::std::string::String {
                    #dart_source.to_owned()
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
                let mut #written = ::std::string::String::from(#dart_source);
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
    defaults: &SerdeDefaults,
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
        if omission.omits_key && !field_def.is_optional() {
            // Dart has no absent-key spelling: a `Map<String, dynamic>` answers a dropped key and
            // an explicit `null` alike, so an omitted key is the outer level's own nullability.
            field_def.nullable_levels.push(field_def.array_depth);
        }
        collected.push(DartField {
            default_at: defaults.at(&rust_name),
            required: !omission.omits_key,
            flatten: field_is_flatten(&field.attrs),
            field_def,
            member: dart_member(&rust_name, &DART_FIELD_NAMES_TAKEN, &written),
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
        FieldDefType::Unknown => "dynamic".to_owned(),
        FieldDefType::TypeParam(name) => name.clone(),
        FieldDefType::Tuple(elements) => {
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
        let item = if field.is_nullable_at(level) {
            format!("{wrapped}?")
        } else {
            wrapped
        };
        format!("List<{item}>")
    })
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
        MapKeyWire::Enumerated | MapKeyWire::Named => dart_base(key),
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
        MapKeyWire::Named => match &key.field_type {
            FieldDefType::U8
            | FieldDefType::U16
            | FieldDefType::U32
            | FieldDefType::U64
            | FieldDefType::I8
            | FieldDefType::I16
            | FieldDefType::I32
            | FieldDefType::I64
            | FieldDefType::Usize
            | FieldDefType::Isize => format!("int.parse({expr})"),
            FieldDefType::F32 | FieldDefType::F64 => format!("double.parse({expr})"),
            FieldDefType::Boolean
            | FieldDefType::Char
            | FieldDefType::String
            | FieldDefType::StringLiteral(_)
            | FieldDefType::BooleanLiteral(_)
            | FieldDefType::NumberLiteral(_)
            | FieldDefType::SiblingType(_, _)
            | FieldDefType::Map(_, _)
            | FieldDefType::Tuple(_)
            | FieldDefType::TypeParam(_)
            | FieldDefType::Unknown => dart_decode_expr(key, expr, Read::Judged),
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => dart_decode_expr(key, expr, Read::Judged),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate
            | FieldDefType::NaiveTime
            | FieldDefType::NaiveDateTime
            | FieldDefType::DateTime => dart_decode_expr(key, expr, Read::Judged),
        },
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
        MapKeyWire::Timestamp => format!("{expr}.toIso8601String()"),
        MapKeyWire::Enumerated => format!("({expr}).toJson()"),
        MapKeyWire::Named => match &key.field_type {
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
            | FieldDefType::F64 => format!("({expr}).toString()"),
            FieldDefType::Boolean
            | FieldDefType::Char
            | FieldDefType::String
            | FieldDefType::StringLiteral(_)
            | FieldDefType::BooleanLiteral(_)
            | FieldDefType::NumberLiteral(_)
            | FieldDefType::SiblingType(_, _)
            | FieldDefType::Map(_, _)
            | FieldDefType::Tuple(_)
            | FieldDefType::TypeParam(_)
            | FieldDefType::Unknown => {
                format!("({}) as String", dart_encode_expr(key, expr, false))
            }
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => format!("({}) as String", dart_encode_expr(key, expr, false)),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate
            | FieldDefType::NaiveTime
            | FieldDefType::NaiveDateTime
            | FieldDefType::DateTime => {
                format!("({}) as String", dart_encode_expr(key, expr, false))
            }
        },
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
/// on [`FieldDef::field_type`] at level `0`.
fn dart_decode_at(field: &FieldDef, level: u8, expr: &str, read: Read) -> String {
    if level > 0 {
        let inner_level = level - 1;
        let inner = dart_decode_at(field, inner_level, "e", read);
        let mapper = if field.is_nullable_at(inner_level) {
            format!("(e) => (e == null ? null : {inner})")
        } else {
            format!("(e) => {inner}")
        };
        return format!("({expr} as List<dynamic>).map({mapper}).toList()");
    }
    match &field.field_type {
        FieldDefType::Unknown => expr.to_owned(),
        FieldDefType::TypeParam(name) => format!("{}({expr})", from_json_argument(name)),
        FieldDefType::Tuple(elements) => {
            let rendered = elements
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    member_decode(
                        element,
                        &format!("({expr} as List<dynamic>)[{index}]"),
                        None,
                        read.of_member(element),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            if elements.len() == 1 {
                format!("({rendered},)")
            } else {
                format!("({rendered})")
            }
        }
        FieldDefType::SiblingType(name, generics) => {
            if let [element] = generics.as_slice()
                && is_sequence_wrapper(name)
            {
                let combined = field.collection_element_field(element);
                return dart_decode_at(&combined, combined.array_depth, expr, read);
            }
            let class_name = lookup_dart_name(name).unwrap_or_else(|| name.clone());
            let constructor = read.constructor();
            if generics.is_empty() {
                format!("{class_name}.{constructor}({expr})")
            } else {
                // No `validate()` reaches a value behind a parameter, which judges itself.
                let converters = generics
                    .iter()
                    .map(|argument| {
                        format!("(e) => {}", dart_decode_expr(argument, "e", Read::Judged))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{class_name}.{constructor}({expr}, {converters})")
            }
        }
        FieldDefType::Map(key, value) => {
            let key_ty = dart_map_key_typename(key);
            let value_ty = dart_typename(value);
            let key_decode = dart_map_key_decode(key, "e.key");
            let value_decode = dart_decode_expr(value, "e.value", read);
            format!(
                "Map<{key_ty}, {value_ty}>.fromEntries(({expr} as Map<String, dynamic>).entries.map((e) => MapEntry({key_decode}, {value_decode})))"
            )
        }
        FieldDefType::Boolean => format!("{expr} as bool"),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            format!("{expr} as String")
        }
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
        | FieldDefType::Isize => format!("{expr} as int"),
        FieldDefType::F32 | FieldDefType::F64 => format!("({expr} as num).toDouble()"),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => format!("ObjectId.fromJson({expr})"),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            format!("{expr} as String")
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
        write!(acc, "..remove('{key}')").unwrap();
        acc
    });
    format!("({{...{json}}}{removed})")
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
        FieldDefType::Unknown => expr.to_owned(),
        FieldDefType::TypeParam(name) => format!("{}({expr})", dart_to_json_argument(name)),
        FieldDefType::Tuple(elements) => {
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
        FieldDefType::SiblingType(name, generics) => {
            if let [element] = generics.as_slice()
                && is_sequence_wrapper(name)
            {
                let combined = field.collection_element_field(element);
                return dart_encode_at(&combined, combined.array_depth, expr);
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
        | FieldDefType::Char
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::U8
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
        | FieldDefType::F64 => expr.to_owned(),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => format!("({expr}).toJson()"),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            expr.to_owned()
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => {
            if has_as_number(field) {
                expr.to_owned()
            } else {
                format!("({expr}).toIso8601String()")
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
        return item_elements(expr, place, depth, asked, |item, within, below| {
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
        });
    }
    match &field.field_type {
        FieldDefType::SiblingType(name, arguments) => {
            if let [element] = arguments.as_slice()
                && is_sequence_wrapper(name)
            {
                return item_elements(expr, place, depth, asked, |item, within, below| {
                    held_elements(element, item, within, below, asked, helpers)
                });
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
/// `place`.
fn own_bound_elements(
    field: &FieldDef,
    expr: &str,
    place: &[Step],
    depth: usize,
    helpers: &mut Helpers,
) -> Vec<String> {
    field
        .model_schema_prop_meta
        .as_ref()
        .map_or_else(Vec::new, |meta| {
            bound_elements(field, expr, place, &Bounds::of(meta), depth, helpers)
        })
}

/// Whether `validate()` runs what the member `field` holds: a member's own bound stands in for
/// what its type declares, a list's count apart.
fn walks_what_it_holds(field: &FieldDef) -> bool {
    field.is_list() || own_bound_elements(field, "", &[], 0, &mut Helpers::default()).is_empty()
}

/// What a member adds, in the order Rust's `validate()` runs it: its own bounds, what the value
/// it holds reports where the member writes no bound or is a list, then its checks. Where a
/// bound is `asked`, the bounds name no place and no check is called.
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
    let mut elements = own_bound_elements(field, expr, named, depth, helpers);
    if walks_what_it_holds(field) {
        elements.extend(held_elements(field, expr, place, depth, asked, helpers));
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

/// A brand's `validate()`: its string check over the value it wraps, under no place, then the
/// checks it names. A brand over one of its own parameters holds the declared default of that
/// parameter and no other filling. A brand that carries a bound writes `validateBound()` ahead of
/// it, the bound alone, which a variant of an untagged enum that holds the brand is chosen by.
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
    let from_json_params = parameters.iter().fold(String::new(), |mut acc, parameter| {
        write!(
            acc,
            ", {parameter} Function(dynamic) {}",
            from_json_argument(parameter)
        )
        .unwrap();
        acc
    });
    let from_json_args = parameters.iter().fold(String::new(), |mut acc, parameter| {
        write!(acc, ", {}", from_json_argument(parameter)).unwrap();
        acc
    });
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
/// flattened one from the keys neither `tag_key` nor another member takes.
fn constructor_arguments(fields: &[DartField], json: &str, tag_key: Option<&str>) -> String {
    let taken: Vec<&str> = tag_key
        .into_iter()
        .chain(
            fields
                .iter()
                .filter(|field| !field.flatten)
                .map(|field| field.wire_name.as_str()),
        )
        .collect();
    fields.iter().fold(String::new(), |mut acc, field| {
        let decode = if field.flatten {
            // A flattened member is walked whatever it writes.
            member_decode(
                &field.field_def,
                &without(json, &taken),
                None,
                Read::Unjudged,
            )
        } else {
            let read = Read::Unjudged.of_member(&field.field_def);
            let held = format!("{json}.containsKey('{}')", field.wire_name);
            let present = member_decode(
                &field.field_def,
                &format!("{json}['{}']", field.wire_name),
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
                None => present,
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
        .map(|field| {
            let encode = dart_encode_expr(&field.field_def, &field.member, false);
            if field.flatten {
                if field.field_def.is_optional() {
                    format!(
                        "if ({} != null) ...({encode} as Map<String, dynamic>),",
                        field.member
                    )
                } else {
                    format!("...({encode} as Map<String, dynamic>),")
                }
            } else if field.required {
                format!("'{}': {encode},", field.wire_name)
            } else {
                format!(
                    "if ({} != null) '{}': {encode},",
                    field.member, field.wire_name
                )
            }
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
/// names on the object it takes, and reads each member from what they answer. `tag_key` is the
/// key that object holds its variant's tag under.
fn class_reads(
    class_name: &str,
    fields: &[DartField],
    tag_key: Option<&str>,
    codec: &GenericCodec,
    corrects: &Corrects<'_>,
) -> String {
    let arguments = constructor_arguments(fields, &corrects.json, tag_key);
    let unjudged = unjudged_read(
        corrects,
        " as Map<String, dynamic>",
        !fields.is_empty(),
        &Answer::Value(format!("{class_name}({arguments})")),
    );
    read_pair(class_name, "Map<String, dynamic>", codec, &unjudged)
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
            &value_field,
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

    record_dart_fields(
        &rust_ident,
        item_struct
            .fields
            .iter()
            .filter_map(|field| Some(field.ident.as_ref()?.unraw().to_string()))
            .collect(),
    );

    let rule = container_rename_rule(&item_struct.attrs);
    let defaults = SerdeDefaults::of(item_struct);
    let fields = collect_dart_fields(&item_struct.fields, rule, &type_parameters, args, &defaults);
    let generic_params = dart_generic_params(&item_struct.generics);
    let codec = generic_codec(&type_parameters);
    // A struct's own tag is written ahead of its fields and read past, as serde does both.
    let own_tag: Vec<String> = struct_tag(item_struct)
        .into_iter()
        .map(|(key, named)| format!("'{key}': '{named}',"))
        .collect();
    let mut helpers = Helpers::default();
    let mut rules = field_elements(&fields, &[], Asked::Rules, &mut helpers);
    rules.extend(own_check_elements(args.check));
    let corrects = Corrects::of(args.preprocess);
    let body = class_body(
        &export_name,
        &generic_params,
        &fields,
        &own_tag,
        &codec,
        &class_reads(&export_name, &fields, None, &codec, &corrects),
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
/// single-slot tuple struct, or a type alias. Each publishes a class wrapping `value`, whose
/// reads and `toJson` delegate to `value_field`'s own, and whose `validate()` is `validate`.
/// `read` is how a declared type the value holds is read, which is unjudged where `validate`
/// runs it. The unjudged read calls what `corrects` names on the payload it takes.
fn value_wrapper_tokens(
    ident: &Ident,
    generics: &syn::Generics,
    name_override: Option<&str>,
    value_field: &FieldDef,
    read: Read,
    corrects: &Corrects<'_>,
    validate: &str,
) -> TokenStream {
    let rust_ident = ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, name_override);
    register_dart_name(&rust_ident, &export_name);

    let generic_params = dart_generic_params(generics);
    let parameters = type_parameters_in_scope(generics);
    let codec = generic_codec(&parameters);
    let decode = dart_decode_expr(value_field, &corrects.json, read);
    let encode = dart_encode_expr(value_field, "value", false);
    let value_ty = dart_typename(value_field);
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

/// The Dart tokens for a non-branded tuple struct: the single-slot ("bare value") shape shares
/// [`value_wrapper_tokens`] with a branded newtype; a wider tuple struct wraps the fixed-size Dart
/// record its slots describe.
fn tuple_struct_dart_tokens(item_struct: &ItemStruct, args: &DartTypeArgs<'_>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let Fields::Unnamed(unnamed) = &item_struct.fields else {
        return TokenStream::new();
    };
    let slots: Vec<FieldDef> = unnamed
        .unnamed
        .iter()
        .filter(|slot| !parse_serde_key_omission(&slot.attrs).absent_from_wire())
        .map(|slot| {
            let mut field_def = field_def_with_prop_meta("slot", slot, args);
            field_def.erase_type_parameters(&type_parameters);
            field_def
        })
        .collect();
    let mut helpers = Helpers::default();
    // serde writes a struct that declares one slot as that slot's value, which is no step.
    let mut rules = if let ([lone], 1) = (slots.as_slice(), unnamed.unnamed.len()) {
        member_elements(lone, "value.$1", &[], 0, Asked::Rules, &mut helpers)
    } else {
        slot_elements(&slots, "value", &[], 0, Asked::Rules, &mut helpers)
    };
    rules.extend(own_check_elements(args.check));
    let tuple_field = FieldDef {
        absent_from_wire: false,
        array_depth: 0,
        array_lengths: Vec::new(),
        docs: String::new(),
        field_type: FieldDefType::Tuple(slots),
        model_schema_prop_meta: None,
        name: "value".to_owned(),
        nullable_levels: Vec::new(),
        omits_value: false,
        #[cfg(any(feature = "zod", feature = "jsonschema"))]
        serde_default: None,
        #[cfg(feature = "jsonschema")]
        type_span: proc_macro2::Span::call_site(),
    };
    value_wrapper_tokens(
        &item_struct.ident,
        &item_struct.generics,
        args.name_override,
        &tuple_field,
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
        &target,
        if named { Read::Unjudged } else { Read::Judged },
        // An alias is refused a `preprocess`: it has no read of its own.
        &Corrects::of(&[]),
        &validate_members(&rules, &Helpers::default(), false),
    )
}

/// The Dart tokens an enum earns: a plain enhanced enum for an all-unit, untagged-by-default
/// declaration; a sealed class hierarchy for every tagged or untagged shape otherwise — mirroring
/// exactly the shape `process_enum` itself dispatches a declaration to.
fn enum_dart_tokens(item_enum: &ItemEnum, args: &DartTypeArgs<'_>) -> TokenStream {
    let rust_ident = item_enum.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_dart_name(&rust_ident, &export_name);
    let tag_attrs = enum_tag_attrs(&item_enum.attrs);
    let writes_bare_variant_names =
        tag_attrs.tag.is_none() && tag_attrs.content.is_none() && !tag_attrs.untagged;
    let dart_source = if is_plain_enum(item_enum) && writes_bare_variant_names {
        plain_enum_dart_source(item_enum, &rust_ident, &export_name, args)
    } else if tag_attrs.untagged {
        untagged_enum_dart_source(item_enum, &rust_ident, &export_name, &tag_attrs, args)
    } else {
        tagged_enum_dart_source(item_enum, &rust_ident, &export_name, &tag_attrs, args)
    };
    dart_module_tokens(
        &rust_ident,
        item_enum.ident.span(),
        &dart_source,
        &SerdeDefaults::none(),
    )
}

/// One tagged/untagged variant's payload — see [`VariantPayload`]. A `TupleMultiple` payload is
/// folded into one `Tuple` [`FieldDef`], matching how a slot list renders everywhere else in this
/// module.
fn variant_payload(
    variant: &Variant,
    field_rule: RenameRule,
    type_parameters: &[String],
    args: &DartTypeArgs<'_>,
) -> VariantPayload {
    match classify_variant(variant) {
        VariantKind::Unit => VariantPayload::Unit,
        VariantKind::Named => VariantPayload::Named(collect_dart_fields(
            &variant.fields,
            field_rule,
            type_parameters,
            args,
            &SerdeDefaults::none(),
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
            let slots: Vec<FieldDef> = variant
                .fields
                .iter()
                .map(|field| {
                    let mut field_def = field_def_with_prop_meta("slot", field, args);
                    field_def.erase_type_parameters(type_parameters);
                    field_def
                })
                .collect();
            VariantPayload::Value(Box::new(FieldDef {
                absent_from_wire: false,
                array_depth: 0,
                array_lengths: Vec::new(),
                docs: String::new(),
                field_type: FieldDefType::Tuple(slots),
                model_schema_prop_meta: None,
                name: "value".to_owned(),
                nullable_levels: Vec::new(),
                omits_value: false,
                #[cfg(any(feature = "zod", feature = "jsonschema"))]
                serde_default: None,
                #[cfg(feature = "jsonschema")]
                type_span: proc_macro2::Span::call_site(),
            }))
        }
    }
}

/// The Dart expression reading a tagged variant's own payload slot out of the top-level `json` (or,
/// under external tagging, the `map` the base class's own dispatch already bound): the content key
/// under adjacent tagging, `json` itself under internal tagging (whose object merges the tag beside
/// the payload's own fields), or `map['{wire_tag}']` under external tagging, whose dispatch has
/// already read the wire tag as the object's one key. Beside it, the test that the payload holds
/// the slot, where it may not: a content key.
fn tagged_payload_json_expr(
    wire_tag: &str,
    shape: &TagShape<'_>,
    corrects: &Corrects<'_>,
) -> (String, Option<String>) {
    let (json, map) = (&corrects.json, &corrects.map);
    shape.content_key.map_or_else(
        || {
            if shape.merge_tag_into_object {
                (json.clone(), None)
            } else {
                (format!("{map}['{wire_tag}']"), None)
            }
        },
        |content| {
            (
                format!("{json}['{content}']"),
                Some(format!("{json}.containsKey('{content}')")),
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
    match (shape.content_key, own_encode) {
        (Some(content), Some(encode)) => {
            format!(
                "{{ '{}': '{wire_tag}', '{content}': {encode} }}",
                shape.tag_key
            )
        }
        (Some(_), None) => format!("{{ '{}': '{wire_tag}' }}", shape.tag_key),
        (None, Some(encode)) => format!("{{ '{wire_tag}': {encode} }}"),
        (None, None) => format!("'{wire_tag}'"),
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
/// base's unjudged dispatch (everything after `case '{wire_tag}':`). `corrects` holds the names
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
        payload,
        subclass_name,
        validate,
        wire_tag,
    } = *variant;
    let (payload_expr, held) = tagged_payload_json_expr(wire_tag, shape, corrects);
    let extends = format!("extends {export_name}{generic_params}");
    match payload {
        VariantPayload::Unit => {
            let own_encode = if shape.merge_tag_into_object {
                format!("{{ '{}': '{wire_tag}' }}", shape.tag_key)
            } else {
                tagged_wrap_encode(shape, wire_tag, None)
            };
            let class_text = format!(
                "class {subclass_name}{generic_params} {extends} {{ const {subclass_name}(); \
                 @override dynamic toJson({tp}) => {own_encode}; {validate} }}",
                tp = codec.to_json_params,
            );
            (class_text, format!("{subclass_name}{generic_params}()"))
        }
        VariantPayload::Named(fields) => {
            let own = corrects.of_a_variant();
            let class_text = if shape.merge_tag_into_object {
                let extra = vec![format!("'{}': '{wire_tag}',", shape.tag_key)];
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
            let source = if shape.merge_tag_into_object && !member_functions(field_def).is_empty() {
                without(&payload_expr, &[shape.tag_key])
            } else {
                payload_expr
            };
            let decode = member_decode(
                field_def,
                &source,
                held.as_deref(),
                Read::Unjudged.of_member(field_def),
            );
            let own_encode = dart_encode_expr(field_def, "value", false);
            let wrapped_encode = if shape.merge_tag_into_object {
                // Internal tagging with a non-`Named` payload: serde admits only an object-shaped
                // one, so the tag merges into that object.
                format!(
                    "{{ '{}': '{wire_tag}', ...({own_encode} as Map<String, dynamic>) }}",
                    shape.tag_key
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
        let subclass_name = format!("{export_name}{variant_rust_name}");
        let payload = variant_payload(variant, field_rule, &type_parameters, args);
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
                payload: &payload,
                subclass_name: &subclass_name,
                validate: &validate,
                wire_tag: &wire_tag,
            },
            &shape,
            &corrects,
        );
        subclasses.push(class_text);
        if is_external && matches!(payload, VariantPayload::Unit) {
            is_string_dispatch_arms.push(format!("case '{wire_tag}': return {from_json};"));
        } else {
            map_dispatch_arms.push(format!("case '{wire_tag}': return {from_json};"));
        }
    }

    let (json, map, tag) = (&corrects.json, &corrects.map, &corrects.tag);
    let (payload_ty, held_as, dispatch) = if is_external {
        (
            "dynamic",
            "",
            format!(
                "if ({json} is String) {{ switch ({json}) \
                 {{ {} default: throw ArgumentError('Unknown {export_name}: ' + {json}); }} }} \
                 final {map} = {json} as Map<String, dynamic>; final {tag} = {map}.keys.first; \
                 switch ({tag}) \
                 {{ {} default: throw ArgumentError('Unknown {export_name}: ' + {tag}); }}",
                is_string_dispatch_arms.join(" "),
                map_dispatch_arms.join(" "),
            ),
        )
    } else {
        (
            "Map<String, dynamic>",
            " as Map<String, dynamic>",
            format!(
                "final {tag} = {json}['{tag_key}'] as String; switch ({tag}) {{ {} default: \
                 throw ArgumentError('Unknown {export_name}: ' + {tag}); }}",
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
    for variant in &item_enum.variants {
        let variant_rust_name = variant.ident.to_string();
        let subclass_name = format!("{export_name}{variant_rust_name}");
        let payload = variant_payload(variant, field_rule, &type_parameters, args);
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
                let decode =
                    member_decode(field_def, json, None, Read::Unjudged.of_member(field_def));
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

    let unjudged = unjudged_read(
        &corrects,
        "",
        true,
        &Answer::Statements(format!(
            "{} throw ArgumentError('No variant of {export_name} matched');",
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
/// Dart will not take; `written` is every member's lower-camel name) and wire value.
fn plain_enum_member(variant: &Variant, rule: RenameRule, written: &[String]) -> (String, String) {
    let rust_name = variant.ident.to_string();
    let member_name = dart_member(
        &dart_lower_camel(&rust_name),
        &DART_ENUM_NAMES_TAKEN,
        written,
    );
    let wire = rename_override(&variant.attrs).unwrap_or_else(|| rule.apply_to_variant(&rust_name));
    (member_name, wire)
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
    let members: Vec<(String, String)> = item_enum
        .variants
        .iter()
        .map(|variant| plain_enum_member(variant, rule, &written))
        .collect();
    let member_list = members
        .iter()
        .map(|(name, wire)| format!("{name}('{wire}')"))
        .collect::<Vec<_>>()
        .join(", ");
    let match_arms = members.iter().fold(String::new(), |mut acc, (name, wire)| {
        write!(acc, "'{wire}' => {export_name}.{name},").unwrap();
        acc
    });
    let validate = validate_members(&own_check_elements(checks), &Helpers::default(), false);
    let unjudged = unjudged_read(
        &corrects,
        " as String",
        true,
        &Answer::Value(format!(
            "switch ({json}) {{ {match_arms} _ => throw ArgumentError('Unknown {export_name}: ' \
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
