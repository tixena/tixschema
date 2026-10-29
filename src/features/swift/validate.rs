//! `validate()` of every Swift type the emitter writes: a member's bounds, what the value it holds
//! reports and the checks it names, then the type's own checks, in the order the Rust `validate()`
//! runs them.

use core::cell::RefCell;
use core::ops::Range;
use std::collections::HashMap;

use regex_syntax::ast::parse::Parser as PatternParser;
use regex_syntax::ast::{
    AssertionKind, Ast, ClassSet, ClassSetItem, Literal, LiteralKind, SpecialLiteralKind,
};

use super::{SwiftTypeArgs, keys, result_case, swift_full_real_type, swift_member};
use crate::bound_message::{Bound, swift_literal_text, swift_repeated, swift_violation};
use crate::features::model_schema_prop::{FunctionName, LiteralValue, ModelSchemaPropMeta};
use crate::field_type::{
    FieldDef, FieldDefType, RESULT_ARMS, RangeKind, format_number_literal, get_field_def,
    is_sequence_wrapper,
};

/// The helpers a statement of `validate()` calls, each with the text that calls it.
const HELPERS: [(&str, &str); 3] = [
    ("swiftSchemaUnder(", UNDER_HELPER),
    ("swiftSchemaPlain(", PLAIN_HELPER),
    ("swiftSchemaRepeats(", REPEATS_HELPER),
];

/// What a float is quoted back with, written into each type that measures one: a whole float
/// without `.0`, as Rust writes it.
const PLAIN_HELPER: &str = "private static func swiftSchemaPlain<Number: BinaryFloatingPoint>(_ value: Number) -> String { \
     let written = \"\\(value)\"; \
     return written.hasSuffix(\".0\") ? String(written.dropLast(2)) : written }";

/// What asks whether a set repeats an item, written into each type that holds a set: two items
/// are one where they write one JSON text, their keys sorted.
const REPEATS_HELPER: &str = "private static func swiftSchemaRepeats<Item: Encodable>(_ items: [Item]) -> Bool { \
     let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys]; var seen = Set<Data>(); \
     for item in items { if let written = try? encoder.encode([item]), !seen.insert(written).inserted { return true } }; \
     return false }";

/// The locals `validate()` writes. A property named after one is read through `self`.
const TAKEN_BY_VALIDATE: [&str; 4] = ["errors", "held", "index", "key"];

/// What writes a place into a message, written into each type that names one: the rule the Rust
/// `validate()` writes a holding field's name by.
const UNDER_HELPER: &str = "private static func swiftSchemaUnder(_ place: String, _ violation: String) -> String { \
     if violation.hasPrefix(\"'\"), let end = violation.dropFirst().firstIndex(of: \"'\") { \
     return \"'\\(place).\\(violation[violation.index(after: violation.startIndex)..<end])'\\(violation[violation.index(after: end)...])\" }; \
     return \"'\\(place)': \\(violation)\" }";

/// The bindings one statement of `validate()` has written, from which the next is named.
#[derive(Default)]
struct Bindings {
    held: usize,
    index: usize,
    key: usize,
}

/// One member as `validate()` reads it.
pub(super) struct Checked<'member> {
    /// The expression the member is read from.
    pub access: String,
    pub def: &'member FieldDef,
    /// The key or the position serde writes the member at, or `None` where it writes neither.
    pub step: Option<String>,
}

/// What a name the emitter has written stands for, as a type that holds it reads it.
#[derive(Clone, Copy)]
struct Noted {
    /// Whether the type is a brand with a bound of its own, which `swiftSchemaValidateBound`
    /// answers for alone.
    bounded: bool,
    /// How what a length counts is read off a value of the type, where it holds a string or a list.
    length: Option<Length>,
    /// How the string a bound measures is read off a value of the type, where it holds one.
    text: Option<Text>,
    /// Whether the `validate()` of the type runs a rule, and so whether a holder calls it. A type
    /// is noted as running one until its own rules are known, which is what one that holds itself
    /// reads.
    validates: bool,
}

/// How Swift holds a number a range is written on.
#[derive(Clone, Copy)]
enum Number {
    Float,
    /// An integer, with the least and the greatest value its type holds.
    Whole(i128, i128),
}

/// One step of the place a value is reported at.
#[derive(Clone)]
enum Step {
    /// The Swift expression that writes the index of an item or the key of an entry.
    Read(String),
    /// A key or a position serde writes.
    Written(String),
}

/// How the string a bound measures is read off a value of a named type.
#[derive(Clone, Copy)]
pub(super) enum Text {
    /// A brand's wrapper, which answers it with its own `swiftSchemaBoundText`.
    Member,
    /// An alias of a string, which is the string.
    Plain,
}

/// How what a length counts is read off one value of a named type.
#[derive(Clone, Copy)]
pub(super) enum Length {
    /// An alias of a list, which is the list.
    Items,
    /// A brand's wrapper, which answers it with its own `swiftSchemaBoundLength`.
    Member,
    /// An alias of a string, which is the string.
    Plain,
}

/// What a walk below a member runs of each value of a named type it reaches.
#[derive(Clone, Copy)]
enum Walk {
    /// The bound of a brand alone, which chooses a variant of an untagged enum.
    Bound,
    /// Its `validate()`, and the repeat of each set the walk reaches.
    Reported,
    /// Its `validate()`.
    Rules,
}

impl Bindings {
    fn next_held(&mut self) -> String {
        numbered("held", &mut self.held)
    }

    fn next_index(&mut self) -> String {
        numbered("index", &mut self.index)
    }

    fn next_key(&mut self) -> String {
        numbered("key", &mut self.key)
    }
}

impl Number {
    /// `minimum` as the literal a value is compared with, or `None` where no value of the type is
    /// below it.
    fn least(self, minimum: f64) -> Option<String> {
        match self {
            Self::Float => Some(format!("{minimum}")),
            Self::Whole(lowest, highest) => {
                let written = whole(minimum)?;
                (lowest < written && written <= highest).then(|| written.to_string())
            }
        }
    }

    /// `maximum` as the literal a value is compared with, or `None` where no value of the type is
    /// above it.
    fn most(self, maximum: f64) -> Option<String> {
        match self {
            Self::Float => Some(format!("{maximum}")),
            Self::Whole(lowest, highest) => {
                let written = whole(maximum)?;
                (lowest <= written && written < highest).then(|| written.to_string())
            }
        }
    }
}

thread_local! {
    /// What each Rust ident the emitter has written stands for, for a type declared after it.
    static NOTED: RefCell<HashMap<String, Noted>> = RefCell::new(HashMap::new());
}

/// Records what `rust_ident` stands for: `text` says how the string a bound measures is read off a
/// value of it, and `validates` whether its `validate()` runs a rule.
pub(super) fn note(rust_ident: &str, text: Option<Text>, validates: bool) {
    NOTED.with(|names| {
        names.borrow_mut().insert(
            rust_ident.to_owned(),
            Noted {
                bounded: false,
                length: None,
                text,
                validates,
            },
        );
    });
}

/// Records how what a length counts is read off a value of the name noted as `rust_ident`.
pub(super) fn note_length(rust_ident: &str, length: Option<Length>) {
    NOTED.with(|names| {
        if let Some(seen) = names.borrow_mut().get_mut(rust_ident) {
            seen.length = length;
        }
    });
}

/// Records that the name noted as `rust_ident` stands for a brand with a bound of its own.
pub(super) fn note_bounded(rust_ident: &str) {
    NOTED.with(|names| {
        if let Some(seen) = names.borrow_mut().get_mut(rust_ident) {
            seen.bounded = true;
        }
    });
}

/// What `rust_ident` was noted as, or `None` for a name the emitter has not written yet.
fn noted(rust_ident: &str) -> Option<Noted> {
    NOTED.with(|names| names.borrow().get(rust_ident).copied())
}

/// `stem`, then `stem1`, `stem2`: the name of the next binding of a kind.
fn numbered(stem: &str, written: &mut usize) -> String {
    let name = if *written == 0 {
        stem.to_owned()
    } else {
        format!("{stem}{written}")
    };
    *written += 1;
    name
}

/// The expression `validate()` reads the property `name` by: through `self` where a local the
/// method writes has the name.
pub(super) fn property_read(name: &str) -> String {
    let stem = name
        .trim_matches('`')
        .trim_end_matches(|written: char| written.is_ascii_digit());
    if TAKEN_BY_VALIDATE.contains(&stem) {
        format!("self.{name}")
    } else {
        name.to_owned()
    }
}

/// `validate()` as a type declares it, with the helpers its statements call. A type with no rule
/// answers an empty list.
pub(super) fn method(rules: &[String]) -> String {
    if rules.is_empty() {
        return "public func validate() -> [String] { [] }".to_owned();
    }
    let body = rules.join("; ");
    format!(
        "public func validate() -> [String] {{ var errors: [String] = []; {body}; return errors }}{}",
        helpers(&body, "")
    )
}

/// The helpers `called` names that `declared` does not write, each after a separator.
pub(super) fn helpers(called: &str, declared: &str) -> String {
    HELPERS
        .iter()
        .filter(|(call, helper)| called.contains(call) && !declared.contains(helper))
        .fold(String::new(), |mut written, (_, helper)| {
            written.push_str("; ");
            written.push_str(helper);
            written
        })
}

/// The member a brand answers for its bounds alone with, the checks it names apart: what chooses
/// a variant of an untagged enum that holds the brand.
pub(super) fn bound_member(bounds: &[String]) -> String {
    format!(
        "public func swiftSchemaValidateBound() -> [String] {{ var errors: [String] = []; {}; \
         return errors }}",
        bounds.join("; ")
    )
}

/// What chooses a variant of an untagged enum, of one member: its own bounds, and the bound of a
/// brand it holds. A check and a model choose nothing.
pub(super) fn bound_alone(owner: &str, member: &Checked<'_>) -> Vec<String> {
    let meta = member.def.model_schema_prop_meta.as_ref();
    let mut rules = meta.map_or_else(Vec::new, |written| bound_rules(owner, member, written));
    rules.extend(held_walk(
        Walk::Bound,
        owner,
        member.def,
        &member.access,
        &written_step(member.step.as_deref()),
        &mut Bindings::default(),
    ));
    rules
}

/// Whether `validate()` runs the `validate()` of a value a member of `def` holds: what the read
/// of the member leaves unjudged.
pub(super) fn walks(def: &FieldDef) -> bool {
    reaches(def)
}

/// Whether a walk of `def` reaches a value whose `validate()` it runs.
pub(super) fn reaches(def: &FieldDef) -> bool {
    held_walk(Walk::Rules, "", def, "", &[], &mut Bindings::default()).is_some()
}

/// The calls of a type's own checks, each on the whole value, its messages as it writes them.
pub(super) fn own_checks(checks: &[FunctionName]) -> Vec<String> {
    checks
        .iter()
        .map(|check| format!("errors += {}(self)", swift_member(&check.name)))
        .collect()
}

/// The `switch` an enum's `validate()` runs the rules of the case it holds in, or `None` where no
/// case has one. `variants` is every case with what it runs, on the `payload` it holds.
pub(super) fn held_case(variants: &[(String, Vec<String>)]) -> Option<String> {
    let arms: Vec<String> = variants
        .iter()
        .filter(|(_, run)| !run.is_empty())
        .map(|(named, run)| format!("case .{named}(let payload): {}", run.join("; ")))
        .collect();
    if arms.is_empty() {
        return None;
    }
    let rest = if arms.len() == variants.len() {
        ""
    } else {
        "; default: break"
    };
    Some(format!("switch self {{ {}{rest} }}", arms.join("; ")))
}

/// What a case adds to its enum's `validate()` for a payload that has a `validate()` of its own:
/// what the payload reports, under `step`. Nothing where the payload runs no rule.
pub(super) fn payload_rules(owner: &str, step: Option<&str>, ruled: bool) -> Vec<String> {
    if !ruled {
        return Vec::new();
    }
    vec![reporting(owner, &written_step(step), "payload.validate()")]
}

/// What the slots of a tuple add to its `validate()`, each under its position.
pub(super) fn slot_rules(owner: &str, slots: &[FieldDef]) -> Vec<String> {
    slots
        .iter()
        .enumerate()
        .flat_map(|(position, slot)| {
            member_rules(
                owner,
                &Checked {
                    access: format!("slot{position}"),
                    def: slot,
                    step: Some(position.to_string()),
                },
            )
        })
        .collect()
}

/// What the bounds of a range add to its `validate()`, each under the key serde writes it under.
pub(super) fn range_rules(owner: &str, kind: RangeKind, bound: &FieldDef) -> Vec<String> {
    kind.members()
        .iter()
        .flat_map(|member| {
            member_rules(
                owner,
                &Checked {
                    access: property_read(member),
                    def: bound,
                    step: Some((*member).to_owned()),
                },
            )
        })
        .collect()
}

/// What the arms of a `Result` add to its `validate()`: the value of the arm it holds, under the key
/// serde writes that arm under.
pub(super) fn result_rules(owner: &str, ok: &FieldDef, err: &FieldDef) -> Vec<String> {
    RESULT_ARMS
        .into_iter()
        .zip([ok, err])
        .filter(|(_, def)| !def.is_unit())
        .filter_map(|(arm, def)| {
            let rules = member_rules(
                owner,
                &Checked {
                    access: "payload".to_owned(),
                    def,
                    step: Some(arm.to_owned()),
                },
            );
            (!rules.is_empty()).then(|| {
                format!(
                    "if case .{}(let payload) = self {{ {} }}",
                    result_case(arm),
                    rules.join("; ")
                )
            })
        })
        .collect()
}

/// What the one slot of a tuple struct that declares one adds to its `validate()`: serde writes the
/// slot's value as the struct, so the slot is reported under no step.
pub(super) fn lone_slot_rules(owner: &str, slot: &FieldDef) -> Vec<String> {
    member_rules(
        owner,
        &Checked {
            access: "value".to_owned(),
            def: slot,
            step: None,
        },
    )
}

/// What the slots serde reads of a tuple struct that declares several add to its `validate()`:
/// what the tuple it wraps reports, each slot under its position.
pub(super) fn tuple_struct_rules(owner: &str, slots: &[FieldDef]) -> Vec<String> {
    if slot_rules(owner, slots).is_empty() {
        Vec::new()
    } else {
        vec!["errors += value.validate()".to_owned()]
    }
}

/// What a member adds to `validate()`: the repeat of a set it is, its bounds, what the value it
/// holds reports, then the checks it names.
pub(super) fn member_rules(owner: &str, member: &Checked<'_>) -> Vec<String> {
    let meta = member.def.model_schema_prop_meta.as_ref();
    let bounds = meta.map_or_else(Vec::new, |written| bound_rules(owner, member, written));
    let mut rules = own_repeat_rules(owner, member);
    let place = written_step(member.step.as_deref());
    let mut bindings = Bindings::default();
    let held = held_walk(
        Walk::Reported,
        owner,
        &member.def.without_own_set(),
        &member.access,
        &place,
        &mut bindings,
    );
    rules.extend(held);
    rules.extend(bounds);
    if let Some(written) = meta {
        rules.extend(check_rules(owner, member, &written.check));
    }
    rules
}

/// What a member that is a set adds where it repeats an item, on the value it holds under an
/// `Option`, under the member's step.
pub(super) fn own_repeat_rules(owner: &str, member: &Checked<'_>) -> Vec<String> {
    if !member.def.is_set() {
        return Vec::new();
    }
    let place = written_step(member.step.as_deref());
    if member.def.is_optional() {
        return vec![format!(
            "if let held = {} {{ {} }}",
            member.access,
            repeated(owner, "held", &place)
        )];
    }
    vec![repeated(owner, &member.access, &place)]
}

/// The statement that reports the set `value` where it repeats an item, under `place`.
fn repeated(owner: &str, value: &str, place: &[Step]) -> String {
    broken(
        &format!("{owner}.swiftSchemaRepeats({value})"),
        &swift_repeated(place_text(place).as_deref()),
    )
}

/// A brand's own bounds on the value it holds. A bare parameter is measured where the value is of
/// its declared default's type, and at no other filling.
pub(super) fn brand_rules(
    owner: &str,
    value_field: &FieldDef,
    args: &SwiftTypeArgs<'_>,
) -> Vec<String> {
    let meta = ModelSchemaPropMeta {
        max_length: args.max_length,
        min_length: args.min_length,
        pattern: args.pattern.map(ToOwned::to_owned),
        ..ModelSchemaPropMeta::default()
    };
    if let FieldDefType::TypeParam(parameter) = &value_field.field_type
        && !value_field.is_list()
    {
        let filling = declared_default(args, parameter);
        let checks = bound_checks(owner, &filling, "held", None, &meta);
        if checks.is_empty() {
            return Vec::new();
        }
        let read_as = swift_full_real_type(&filling, owner, &mut Vec::new());
        return vec![format!(
            "if let held = value as? {read_as} {{ {} }}",
            checks.join("; ")
        )];
    }
    bound_rules(
        owner,
        &Checked {
            access: "value".to_owned(),
            def: value_field,
            step: None,
        },
        &meta,
    )
}

/// The member a brand over `inner` answers its text with, which a bound written on a field typed
/// with the brand measures: `value` for a brand over a string, and the member of the brand it holds
/// for a brand over a brand. `None` for a brand that holds no string.
pub(super) fn brand_text_member(inner: &FieldDef) -> Option<String> {
    let read = match text_of(inner)? {
        Text::Member => "value.swiftSchemaBoundText",
        Text::Plain => "value",
    };
    Some(format!(
        "public var swiftSchemaBoundText: String {{ {read} }}"
    ))
}

/// The member a brand over `inner` answers its length with, which a length written on a field
/// typed with the brand counts: the code points of a string, the items of a list, and the member of
/// the brand it holds for a brand over a brand. `None` for a brand that holds neither.
pub(super) fn brand_length_member(inner: &FieldDef) -> Option<String> {
    let read = length_of(inner).map(|length| length_expression("value", length))?;
    Some(format!(
        "public var swiftSchemaBoundLength: Int {{ {read} }}"
    ))
}

/// How what a length counts is read off one value of `def`, where it is known to hold a string or a
/// list.
pub(super) fn length_of(def: &FieldDef) -> Option<Length> {
    if def.is_optional() {
        return None;
    }
    if def.is_list() {
        return Some(Length::Items);
    }
    if matches!(def.field_type, FieldDefType::String) {
        return Some(Length::Plain);
    }
    noted(declared_name(def)?)?.length
}

/// The expression that counts `value`, which holds what `length` says.
fn length_expression(value: &str, length: Length) -> String {
    match length {
        Length::Items => format!("{value}.count"),
        Length::Member => format!("{value}.swiftSchemaBoundLength"),
        Length::Plain => format!("{value}.unicodeScalars.count"),
    }
}

/// The expression that counts one value of `def` held at `value` for a length written on it, where
/// it names a type: a string or a list the emitter wrote, or the wrapper of a brand, which answers
/// for itself. `None` for a value that is none of these.
fn length_read(def: &FieldDef, value: &str) -> Option<String> {
    if def.is_list() || matches!(def.field_type, FieldDefType::String) {
        return length_of(def).map(|length| length_expression(value, length));
    }
    declared_name(def)?;
    let length = noted(declared_name(def)?).and_then(|seen| seen.length);
    Some(length_expression(value, length.unwrap_or(Length::Member)))
}

/// How the string is read off one value of `def`, where it is known to hold one: a string and an
/// alias of one as they are, and a brand over one through its member.
pub(super) fn text_of(def: &FieldDef) -> Option<Text> {
    if def.is_list() || def.is_optional() {
        return None;
    }
    if matches!(def.field_type, FieldDefType::String) {
        return Some(Text::Plain);
    }
    noted(declared_name(def)?)?.text
}

/// Whether `def` names one type whose `validate()` a holder runs, as an alias of it does.
pub(super) fn names_a_validated_type(def: &FieldDef) -> bool {
    named_alone(def).is_some_and(validates)
}

/// Whether `def` names one brand with a bound of its own, as an alias of it does.
pub(super) fn names_a_bounded_type(def: &FieldDef) -> bool {
    named_alone(def).is_some_and(bounded)
}

/// The name `def` is written with where it names one type declared elsewhere, under no wrapper.
fn named_alone(def: &FieldDef) -> Option<&str> {
    if def.is_list() || def.is_optional() {
        return None;
    }
    let FieldDefType::SiblingType(named, _) = &def.field_type else {
        return None;
    };
    Some(named)
}

/// The name `def` is written with where it names a type declared elsewhere with no argument.
fn declared_name(def: &FieldDef) -> Option<&str> {
    let FieldDefType::SiblingType(named, arguments) = &def.field_type else {
        return None;
    };
    arguments.is_empty().then_some(named.as_str())
}

/// Whether a holder runs the `validate()` of the type `rust_ident` names: one that runs a rule,
/// and one of a name the emitter has not written yet, a type declared below.
fn validates(rust_ident: &str) -> bool {
    noted(rust_ident).is_none_or(|seen| seen.validates)
}

/// Whether `rust_ident` names a brand the emitter has written with a bound of its own.
fn bounded(rust_ident: &str) -> bool {
    noted(rust_ident).is_some_and(|seen| seen.bounded)
}

/// What reads the string a bound measures off one value of `def`: nothing for a string and for an
/// alias of one the emitter has written, and the member a brand's wrapper answers its text with
/// for every other named type, declared above or below. `None` for a value that is neither.
fn text_read(def: &FieldDef) -> Option<&'static str> {
    if matches!(def.field_type, FieldDefType::String) {
        return Some("");
    }
    let written = noted(declared_name(def)?).and_then(|seen| seen.text);
    if matches!(written, Some(Text::Plain)) {
        Some("")
    } else {
        Some(".swiftSchemaBoundText")
    }
}

/// The type declared for `parameter` in `default_types`, or `String` where none is.
fn declared_default(args: &SwiftTypeArgs<'_>, parameter: &str) -> FieldDef {
    args.default_types
        .iter()
        .find(|(named, _)| named == parameter)
        .map_or_else(
            || get_field_def("", &syn::parse_quote!(String), ""),
            |(_, filling)| get_field_def("", filling, ""),
        )
}

/// `step` as the place of a member that has one.
fn written_step(step: Option<&str>) -> Vec<Step> {
    step.map(|written| Step::Written(written.to_owned()))
        .into_iter()
        .collect()
}

/// The steps as the text of a Swift string literal, joined by dots, or `None` for no step.
fn place_text(steps: &[Step]) -> Option<String> {
    if steps.is_empty() {
        return None;
    }
    let written: Vec<String> = steps
        .iter()
        .map(|step| match step {
            Step::Read(read) => format!("\\({read})"),
            Step::Written(key) => swift_literal_text(key),
        })
        .collect();
    Some(written.join("."))
}

/// The statement that adds `reported`, a list of messages, each under `steps`.
fn reporting(owner: &str, steps: &[Step], reported: &str) -> String {
    place_text(steps).map_or_else(
        || format!("errors += {reported}"),
        |place| format!("errors += {reported}.map {{ {owner}.swiftSchemaUnder(\"{place}\", $0) }}"),
    )
}

/// The statement that reports `violation` where `condition` holds.
fn broken(condition: &str, violation: &str) -> String {
    format!("if {condition} {{ errors.append({violation}) }}")
}

/// The member's own bounds, on the value it holds under an `Option`. A literal is the only rule
/// of its member.
fn bound_rules(owner: &str, member: &Checked<'_>, meta: &ModelSchemaPropMeta) -> Vec<String> {
    if let Some(literal) = &meta.literal {
        return vec![literal_rule(
            owner,
            member.def,
            &member.access,
            &written_step(member.step.as_deref()),
            literal,
            &mut Bindings::default(),
        )];
    }
    if !member.def.is_optional() {
        return bound_checks(
            owner,
            member.def,
            &member.access,
            member.step.as_deref(),
            meta,
        );
    }
    let checks = bound_checks(owner, member.def, "held", member.step.as_deref(), meta);
    if checks.is_empty() {
        return checks;
    }
    vec![format!(
        "if let held = {} {{ {} }}",
        member.access,
        checks.join("; ")
    )]
}

/// What `literal` adds for the value `def` describes at `value`, under `place`: each item of a
/// list holds it, under its index, and an `Option` where it holds a value.
fn literal_rule(
    owner: &str,
    def: &FieldDef,
    value: &str,
    place: &[Step],
    literal: &LiteralValue,
    bindings: &mut Bindings,
) -> String {
    if def.is_optional() {
        let held = bindings.next_held();
        let below = literal_rule(owner, &under_option(def), &held, place, literal, bindings);
        return format!("if let {held} = {value} {{ {below} }}");
    }
    if def.array_depth > 0 {
        let (index, held) = (bindings.next_index(), bindings.next_held());
        let mut at = place.to_vec();
        at.push(Step::Read(index.clone()));
        let below = literal_rule(owner, &item_of(def), &held, &at, literal, bindings);
        return format!("for ({index}, {held}) in {value}.enumerated() {{ {below} }}");
    }
    let mut shown = value.to_owned();
    let condition = match literal {
        LiteralValue::Bool(true) => format!("!{value}"),
        LiteralValue::Bool(false) => value.to_owned(),
        LiteralValue::Str(text) => format!("{value} != \"{}\"", swift_literal_text(text)),
        LiteralValue::Integer(_) | LiteralValue::Number(_) => {
            let number = literal.number().unwrap_or_default();
            if matches!(number_kind(&def.field_type), Some(Number::Float)) {
                shown = format!("{owner}.swiftSchemaPlain({value})");
                let written = number.to_string();
                if written.contains('.') {
                    format!("{value} != {written}")
                } else {
                    format!("{value} != {written}.0")
                }
            } else if let LiteralValue::Integer(whole) = literal {
                format!("{value} != {whole}")
            } else {
                format!("{value} != {}", format_number_literal(number))
            }
        }
    };
    let said = match place {
        [] => swift_violation(Bound::Literal(literal), None, &shown),
        [Step::Written(step)] => swift_violation(Bound::Literal(literal), Some(step), &shown),
        _ => format!(
            "{owner}.swiftSchemaUnder(\"{}\", {})",
            place_text(place).unwrap_or_default(),
            swift_violation(Bound::Literal(literal), None, &shown)
        ),
    };
    broken(&condition, &said)
}

/// The bounds of `meta` on `value`, which holds what `def` describes under no `Option`: a list is
/// counted, a string measured in Unicode code points and matched, a number compared.
fn bound_checks(
    owner: &str,
    def: &FieldDef,
    value: &str,
    place: Option<&str>,
    meta: &ModelSchemaPropMeta,
) -> Vec<String> {
    if def.is_list() {
        return length_checks(&format!("{value}.count"), place, meta, true);
    }
    if meta.patterns().next().is_none()
        && let Some(measured) = length_read(def, value)
    {
        return length_checks(&measured, place, meta, false);
    }
    if let Some(read) = text_read(def) {
        let text = format!("{value}{read}");
        let measured = format!("{text}.unicodeScalars.count");
        let mut checks = length_checks(&measured, place, meta, false);
        checks.extend(meta.patterns().map(|pattern| {
            broken(
                &format!(
                    "{text}.range(of: \"{}\", options: .regularExpression) == nil",
                    swift_literal_text(&swift_pattern(pattern))
                ),
                &swift_violation(Bound::Pattern(pattern), place, &text),
            )
        }));
        return checks;
    }
    range_checks(owner, def, value, place, meta)
}

/// `minLength` and `maxLength` on what `measured` counts: the items of a list, or the code points
/// of a string.
fn length_checks(
    measured: &str,
    place: Option<&str>,
    meta: &ModelSchemaPropMeta,
    counts_items: bool,
) -> Vec<String> {
    let mut checks = Vec::new();
    if let Some(least) = meta.min_length {
        let bound = if counts_items {
            Bound::MinItems(least)
        } else {
            Bound::MinLength(least)
        };
        checks.push(broken(
            &format!("{measured} < {least}"),
            &swift_violation(bound, place, measured),
        ));
    }
    if let Some(most) = meta.max_length {
        let bound = if counts_items {
            Bound::MaxItems(most)
        } else {
            Bound::MaxLength(most)
        };
        checks.push(broken(
            &format!("{measured} > {most}"),
            &swift_violation(bound, place, measured),
        ));
    }
    checks
}

/// `minimum` and `maximum` on a number, each written where a value of the type can break it.
fn range_checks(
    owner: &str,
    def: &FieldDef,
    value: &str,
    place: Option<&str>,
    meta: &ModelSchemaPropMeta,
) -> Vec<String> {
    let Some(kind) = number_kind(&def.field_type) else {
        return Vec::new();
    };
    let observed = match kind {
        Number::Float => format!("{owner}.swiftSchemaPlain({value})"),
        Number::Whole(_, _) => value.to_owned(),
    };
    let mut checks = Vec::new();
    if let Some(minimum) = meta.minimum
        && let Some(written) = kind.least(minimum)
    {
        checks.push(broken(
            &format!("{value} < {written}"),
            &swift_violation(Bound::Minimum(minimum), place, &observed),
        ));
    }
    if let Some(maximum) = meta.maximum
        && let Some(written) = kind.most(maximum)
    {
        checks.push(broken(
            &format!("{value} > {written}"),
            &swift_violation(Bound::Maximum(maximum), place, &observed),
        ));
    }
    checks
}

/// How Swift holds the number `leaf` is, or `None` for a type that is no number.
fn number_kind(leaf: &FieldDefType) -> Option<Number> {
    let whole_in = |lowest: i128, highest: i128| Some(Number::Whole(lowest, highest));
    match leaf {
        FieldDefType::F32 | FieldDefType::F64 => Some(Number::Float),
        FieldDefType::I8 => whole_in(i128::from(i8::MIN), i128::from(i8::MAX)),
        FieldDefType::I16 => whole_in(i128::from(i16::MIN), i128::from(i16::MAX)),
        FieldDefType::I32 => whole_in(i128::from(i32::MIN), i128::from(i32::MAX)),
        FieldDefType::I64 | FieldDefType::Isize => {
            whole_in(i128::from(i64::MIN), i128::from(i64::MAX))
        }
        FieldDefType::U8 => whole_in(0, i128::from(u8::MAX)),
        FieldDefType::U16 => whole_in(0, i128::from(u16::MAX)),
        FieldDefType::U32 => whole_in(0, i128::from(u32::MAX)),
        FieldDefType::U64 | FieldDefType::Usize => whole_in(0, i128::from(u64::MAX)),
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::Map(_, _)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Range(_, _)
        | FieldDefType::Result(_, _)
        | FieldDefType::SiblingType(_, _)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::Tuple(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown => None,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => None,
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => None,
    }
}

/// A bound as the integer Rust compares an integer with: its fraction dropped.
fn whole(bound: f64) -> Option<i128> {
    bound.trunc().to_string().parse().ok()
}

/// The calls of the checks a member names, each on the value the member holds under an `Option`,
/// its messages under the member's step.
fn check_rules(owner: &str, member: &Checked<'_>, checks: &[FunctionName]) -> Vec<String> {
    if checks.is_empty() {
        return Vec::new();
    }
    let place = written_step(member.step.as_deref());
    let optional = member.def.is_optional();
    let value = if optional { "held" } else { &member.access };
    let calls: Vec<String> = checks
        .iter()
        .map(|check| {
            reporting(
                owner,
                &place,
                &format!("{}({value})", swift_member(&check.name)),
            )
        })
        .collect();
    if optional {
        vec![format!(
            "if let held = {} {{ {} }}",
            member.access,
            calls.join("; ")
        )]
    } else {
        calls
    }
}

/// `def` as the value it holds where it holds one.
pub(super) fn under_option(def: &FieldDef) -> FieldDef {
    let mut held = def.clone();
    held.nullable_levels
        .retain(|level| *level != def.array_depth);
    held
}

/// What each item of the list `def` describes is.
pub(super) fn item_of(def: &FieldDef) -> FieldDef {
    let mut item = def.clone();
    item.array_depth = def.array_depth.saturating_sub(1);
    item.nullable_levels
        .retain(|level| *level <= item.array_depth);
    item.set_levels.retain(|level| *level < item.array_depth);
    item
}

/// The statement that runs, of every value `def` holds that has it, what `walk` names: its
/// `validate()`, or the bound of a brand alone. Each report is under `place` and what the walk
/// adds to it: the index of an item, the key of an entry. `None` where it holds no such value. A
/// value behind a type parameter is not reached.
fn held_walk(
    walk: Walk,
    owner: &str,
    def: &FieldDef,
    value: &str,
    place: &[Step],
    bindings: &mut Bindings,
) -> Option<String> {
    if def.is_optional() {
        let held = bindings.next_held();
        let below = held_walk(walk, owner, &under_option(def), &held, place, bindings)?;
        return Some(format!("if let {held} = {value} {{ {below} }}"));
    }
    if def.array_depth > 0 {
        let repeat = (matches!(walk, Walk::Reported)
            && def.is_set_at(def.array_depth.saturating_sub(1)))
        .then(|| repeated(owner, value, place));
        let (index, held) = (bindings.next_index(), bindings.next_held());
        let mut at = place.to_vec();
        at.push(Step::Read(index.clone()));
        let below = held_walk(walk, owner, &item_of(def), &held, &at, bindings)
            .map(|inner| format!("for ({index}, {held}) in {value}.enumerated() {{ {inner} }}"));
        return match (repeat, below) {
            (Some(asked), Some(walked)) => Some(format!("{asked}; {walked}")),
            (asked, walked) => asked.or(walked),
        };
    }
    if let FieldDefType::Map(key, entry) = &def.field_type {
        let (named, held) = (bindings.next_key(), bindings.next_held());
        let mut at = place.to_vec();
        at.push(Step::Read(key_text(key, &named)));
        let below = held_walk(walk, owner, entry, &held, &at, bindings)?;
        return Some(format!(
            "for ({named}, {held}) in {value}.sorted(by: {{ {} }}) {{ {below} }}",
            key_order(key)
        ));
    }
    if let FieldDefType::Tuple(slots) = &def.field_type {
        return (!matches!(walk, Walk::Bound) && !slot_rules(owner, slots).is_empty())
            .then(|| reporting(owner, place, &format!("{value}.validate()")));
    }
    if let FieldDefType::Range(kind, bound) = &def.field_type {
        if !matches!(walk, Walk::Bound) {
            return (!range_rules(owner, *kind, bound).is_empty())
                .then(|| reporting(owner, place, &format!("{value}.validate()")));
        }
        // What chooses a variant is asked of each bound in place, as Rust's read asks it.
        let bounds: Vec<String> = kind
            .members()
            .iter()
            .filter_map(|member| {
                let mut at = place.to_vec();
                at.push(Step::Written((*member).to_owned()));
                held_walk(
                    walk,
                    owner,
                    bound,
                    &format!("{value}.{member}"),
                    &at,
                    bindings,
                )
            })
            .collect();
        return (!bounds.is_empty()).then(|| bounds.join("; "));
    }
    if let FieldDefType::Result(ok, err) = &def.field_type {
        if !matches!(walk, Walk::Bound) {
            return (!result_rules(owner, ok, err).is_empty())
                .then(|| reporting(owner, place, &format!("{value}.validate()")));
        }
        // What chooses a variant is asked of the arm the value holds, in place.
        let arms: Vec<String> = RESULT_ARMS
            .into_iter()
            .zip([ok, err])
            .filter(|(_, held)| !held.is_unit())
            .filter_map(|(arm, held)| {
                let named = bindings.next_held();
                let mut at = place.to_vec();
                at.push(Step::Written(arm.to_owned()));
                let below = held_walk(walk, owner, held, &named, &at, bindings)?;
                Some(format!(
                    "if case .{}(let {named}) = {value} {{ {below} }}",
                    result_case(arm)
                ))
            })
            .collect();
        return (!arms.is_empty()).then(|| arms.join("; "));
    }
    let FieldDefType::SiblingType(named, arguments) = &def.field_type else {
        return None;
    };
    if let [item] = arguments.as_slice()
        && is_sequence_wrapper(named)
    {
        return held_walk(
            walk,
            owner,
            &def.collection_element_field(item),
            value,
            place,
            bindings,
        );
    }
    let (asked, answers) = match walk {
        Walk::Bound => ("swiftSchemaValidateBound", bounded(named)),
        Walk::Reported | Walk::Rules => ("validate", validates(named)),
    };
    answers.then(|| reporting(owner, place, &format!("{value}.{asked}()")))
}

/// The text serde writes for the key of a map's entry, read off `named`.
fn key_text(written: &FieldDef, named: &str) -> String {
    let key = keys::peeled(written);
    if let Some((_, value)) = keys::brand_of(&key) {
        return key_text(&value, &format!("{named}.value"));
    }
    if matches!(key.field_type, FieldDefType::Boolean) {
        return format!("{named} ? \"true\" : \"false\"");
    }
    #[cfg(feature = "chrono")]
    if matches!(key.field_type, FieldDefType::DateTime) {
        return format!(
            "{{ let f = ISO8601DateFormatter(); f.formatOptions = [{}]; return f }}().string(from: {named}){}",
            super::FRACTIONAL_OPTIONS,
            super::CHRONO_WHOLE_SECONDS
        );
    }
    if matches!(key.field_type, FieldDefType::SiblingType(_, _)) {
        return format!("{named}.rawValue");
    }
    named.to_owned()
}

/// What a map's entries are walked in the order of: the key itself where Swift orders its type,
/// and the text serde writes for it where it does not.
fn key_order(written: &FieldDef) -> String {
    let key = keys::peeled(written);
    if let Some((_, value)) = keys::brand_of(&key) {
        return key_order(&value)
            .replace("$0.key", "$0.key.value")
            .replace("$1.key", "$1.key.value");
    }
    if matches!(key.field_type, FieldDefType::Boolean) {
        return "!$0.key && $1.key".to_owned();
    }
    if matches!(key.field_type, FieldDefType::SiblingType(_, _)) {
        return "$0.key.rawValue < $1.key.rawValue".to_owned();
    }
    "$0.key < $1.key".to_owned()
}

/// `pattern` as the Foundation regex engine reads it the way the `regex` crate reads it. There `$`
/// also matches ahead of a line terminator that ends the text, and `\v` in a class is every
/// vertical space, so the first is written `\z` and the second `\u000B`.
fn swift_pattern(pattern: &str) -> String {
    let Ok(parsed) = PatternParser::new().parse(pattern) else {
        return pattern.to_owned();
    };
    let mut edits: Vec<(Range<usize>, &'static str)> = Vec::new();
    respell(&parsed, &mut edits);
    edits.sort_unstable_by_key(|(span, _)| span.start);
    let mut written = String::with_capacity(pattern.len());
    let mut cut = 0;
    for (span, replacement) in edits {
        written.push_str(&pattern[cut..span.start]);
        written.push_str(replacement);
        cut = span.end;
    }
    written.push_str(&pattern[cut..]);
    written
}

/// Collects into `edits` what [`swift_pattern`] writes another way.
fn respell(ast: &Ast, edits: &mut Vec<(Range<usize>, &'static str)>) {
    match ast {
        Ast::Assertion(assertion) => {
            if matches!(assertion.kind, AssertionKind::EndLine) {
                edits.push((
                    assertion.span.start.offset..assertion.span.end.offset,
                    "\\z",
                ));
            }
        }
        Ast::Literal(literal) => respell_literal(literal, edits),
        Ast::ClassBracketed(class) => respell_class(&class.kind, edits),
        Ast::Repetition(repetition) => respell(&repetition.ast, edits),
        Ast::Group(group) => respell(&group.ast, edits),
        Ast::Alternation(alternation) => {
            for branch in &alternation.asts {
                respell(branch, edits);
            }
        }
        Ast::Concat(concat) => {
            for part in &concat.asts {
                respell(part, edits);
            }
        }
        Ast::Empty(_) | Ast::Flags(_) | Ast::Dot(_) | Ast::ClassUnicode(_) | Ast::ClassPerl(_) => {}
    }
}

fn respell_class(set: &ClassSet, edits: &mut Vec<(Range<usize>, &'static str)>) {
    match set {
        ClassSet::Item(item) => respell_class_item(item, edits),
        ClassSet::BinaryOp(op) => {
            respell_class(&op.lhs, edits);
            respell_class(&op.rhs, edits);
        }
    }
}

fn respell_class_item(item: &ClassSetItem, edits: &mut Vec<(Range<usize>, &'static str)>) {
    match item {
        ClassSetItem::Literal(literal) => respell_literal(literal, edits),
        ClassSetItem::Range(range) => {
            respell_literal(&range.start, edits);
            respell_literal(&range.end, edits);
        }
        ClassSetItem::Bracketed(class) => respell_class(&class.kind, edits),
        ClassSetItem::Union(union) => {
            for member in &union.items {
                respell_class_item(member, edits);
            }
        }
        ClassSetItem::Empty(_)
        | ClassSetItem::Ascii(_)
        | ClassSetItem::Unicode(_)
        | ClassSetItem::Perl(_) => {}
    }
}

fn respell_literal(literal: &Literal, edits: &mut Vec<(Range<usize>, &'static str)>) {
    if matches!(
        literal.kind,
        LiteralKind::Special(SpecialLiteralKind::VerticalTab)
    ) {
        edits.push((
            literal.span.start.offset..literal.span.end.offset,
            "\\u000B",
        ));
    }
}

#[cfg(test)]
mod tests;
