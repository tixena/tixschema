//! Which members a type may flatten, in which order, and with which strictness, so that serde reads
//! back every payload it writes.
//!
//! serde reads a holder's flattened members one at a time, in the order declared, from the keys the
//! holder did not read itself. A struct with no flattened member and no tag of its own, an enum
//! under no tag or under a tag and a content key, and an `Option` of either take their own keys and
//! remove them. Every other member reads every key still left and leaves them in place: one that
//! refuses a key it does not declare refuses the keys of the members beside it, and a map reads
//! them as its entries. The holder is closed only where every member takes its own keys.
//!
//! What a member is, the expansion reads from what `#[model_schema]` recorded when the member's
//! type was expanded above it. A type it has not met is read from the hidden constant each type
//! publishes, in a `const` the compiler evaluates, and a type parameter is held to the strictest
//! rule: last, and the only member beside it that reads every key.

use core::cell::RefCell;
#[cfg(any(feature = "dart", feature = "swift"))]
use core::fmt::Write as _;
use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens as _, quote, quote_spanned};
use syn::ext::IdentExt as _;
use syn::spanned::Spanned as _;
use syn::{Field, Fields, Generics, Item, ItemEnum, ItemStruct, Type, Variant};

use super::judged_read::unclaimed_parameter;
use super::ranges::def_writes_a_map;
use super::{
    ModelSchemaArgs, prefixed_guard_message, strict, written_as_its_slot, written_spelling,
};
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;
use crate::features::serde::{
    Tagging, derive_reads_a_default, parse_read_renames, parse_serde_field_attributes,
    parse_serde_key_omission, parse_serde_type_attributes, parse_written_renames, read_hook,
    struct_tag, variant_on_the_wire,
};
use crate::field_type::{FieldDef, FieldDefType, get_field_def};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{Declared, declared, written_names};

/// The bit of a type's hidden constant saying serde leaves its keys in the object it is flattened
/// from.
pub const REST: u16 = 1;
/// The bit saying the type refuses a key it does not declare, read from an object that holds one.
pub const REFUSES: u16 = 2;
/// The bit saying the type reads every key it is handed as an entry of a map.
pub const ABSORBS: u16 = 4;
/// The bit saying the expansion could not see what the type reads.
pub const UNSEEN: u16 = 8;
/// The bit saying the type reads from an object that holds none of its keys, which a flattened
/// `Option` of it then reads as `Some`.
pub const EMPTY: u16 = 16;
/// The bit saying the type is an externally tagged enum with a unit variant, which serde writes
/// flattened as an entry of the variant's name over `null`.
pub const ENTRY: u16 = 32;
/// The bit saying serde writes the type as no object, which it cannot flatten.
pub const NO_OBJECT: u16 = 64;
/// The bit saying serde writes the type as a plain enum's variant name, which it writes flattened
/// as a key over `null` that no other surface reads.
pub const PLAIN: u16 = 128;
/// The bit saying the type holds, at any depth, a map keyed by a number or a `bool`, whose keys
/// serde reads as text below a flattened member and then refuses.
pub const TEXT_KEYS: u16 = 256;
/// The bit saying serde writes the type as an `Option` of a value that reads from an object that
/// holds none of its keys, so that a holder that flattens it reads its `None` back as `Some`.
pub const OPTION_EMPTY: u16 = 512;
/// What a type no `#[model_schema]` expanded answers: the strictest reading.
const NOT_MET: u16 = REST | REFUSES | UNSEEN | EMPTY;
/// The hidden constant every type `#[model_schema]` expands publishes.
const FLAGS: &str = "__TIXSCHEMA_FLATTENED";
/// The hidden constant of the keys a holder that flattens the type sees it write or read.
const KEYS: &str = "__TIXSCHEMA_KEYS";
/// How many types deep a member is followed before what it reads is called unseen.
const DEPTH: usize = 8;
/// Why the expansion cannot see a member whose hidden constant answers for it.
const NOT_SEEN: &str = "declared below the type that flattens it, in another crate, or with no \
                        `#[model_schema]`";
/// [`NOT_SEEN`] of one of two sources a check names.
const NOT_SEEN_ONE: &str = "one of them declared below the type that flattens it, in another \
                            crate, or with no `#[model_schema]`";

/// How many classes [`class_coded`] reads.
const CODES: u8 = 5;

/// What a flattened member does with the keys serde hands it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Class {
    /// Reads every key still left into a map.
    Absorbing,
    /// Reads every key still left and leaves them in place, and refuses none.
    Open,
    /// Reads every key still left and refuses one it does not declare, through these types.
    Refusing(Vec<Culprit>),
    /// Takes its own keys and removes them.
    Taker,
    /// Reads keys the expansion cannot see, for the reason held: read as the strictest member.
    Unseen(String),
}

impl Class {
    /// Whether the member reads back only as the one member beside it that reads every key, and
    /// declared last.
    pub const fn exclusive(&self) -> bool {
        matches!(self, Self::Absorbing | Self::Refusing(_) | Self::Unseen(_))
    }

    /// Whether serde leaves the keys the member reads in the object for the members after it.
    pub const fn rest(&self) -> bool {
        !matches!(self, Self::Taker)
    }
}

/// A type that refuses a key it does not declare, and whether its author wrote the
/// `#[serde(deny_unknown_fields)]` that makes it refuse one, or the macro did. `holds_more` says
/// the type stands for the structs it holds as well, which the expansion could not see.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Culprit {
    pub by_hand: bool,
    pub holds_more: bool,
    pub name: String,
}

/// The keys a value is read and written under: a member's own, and every alias it reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct KeySet {
    read: Vec<String>,
    written: Vec<String>,
}

impl KeySet {
    /// A key read and written alike.
    fn both(key: String) -> Self {
        Self {
            read: vec![key.clone()],
            written: vec![key],
        }
    }

    fn extend(&mut self, more: &Self) {
        for key in &more.read {
            if !self.read.contains(key) {
                self.read.push(key.clone());
            }
        }
        for key in &more.written {
            if !self.written.contains(key) {
                self.written.push(key.clone());
            }
        }
    }
}

/// Whether a value reads from an object that holds none of its keys, by its parts.
#[derive(Clone)]
enum EmptyRule {
    /// Where every part does.
    AllOf(Vec<Part>),
    /// Where any of these does: the variants of an untagged enum.
    AnyOf(Vec<Self>),
    Never,
    /// A read its author's functions answer.
    Unknown,
}

/// One level of an item whose members serde reads together: a struct, or a variant of an enum.
struct Level<'item> {
    closed_by_hand: bool,
    /// Whether the macro writes `#[serde(deny_unknown_fields)]` on the item.
    closed_by_macro: bool,
    fields: &'item Fields,
    /// Its own key that serde leaves for the flattened members, and how a refusal names it.
    leftover: Option<String>,
    /// The holder's name, for the fix a refusal says.
    named: String,
    /// Each key the level declares itself, beside what declares it, as a refusal names it.
    own_keys: Vec<(String, KeySet)>,
    subject: String,
}

/// One flattened member of a holder, as a refusal names it.
struct Member<'item> {
    /// What [`blind_of`] names of its type, for a member whose type the expansion met.
    #[cfg(any(feature = "dart", feature = "swift"))]
    blind: Option<Blind>,
    class: Class,
    /// Its hidden constant, where the compiler reads what it is rather than the expansion.
    dynamic: Option<TokenStream>,
    /// The hidden constant of its keys, where the expansion cannot see every key it writes.
    dynamic_keys: Option<TokenStream>,
    field: &'item Field,
    keys: KeySet,
    name: String,
    /// For a member that is an `Option`, what it holds and whether that reads from an object that
    /// holds none of its keys, where the expansion sees it.
    optional: Option<Optional>,
    written: String,
}

/// What a flattened `Option` holds.
struct Optional {
    empty: Answer,
    /// The types that make what it holds refuse a key it does not declare, where it also reads
    /// every key still left: serde then reads the `Option` as `None` beside a key no member
    /// declares.
    refusing: Option<Vec<Culprit>>,
    written: String,
}

/// A part of a type that decides what the type refuses: a member it flattens, or the payload of a
/// newtype variant.
#[derive(Clone)]
enum Part {
    /// A member a `preprocess` reads: its functions are handed every key still left, and the
    /// member is read from what they answer.
    Corrected,
    Read(Box<FieldDef>),
    Unseen(String),
}

/// What a holder reads of a type it flattens, or that a variant holds as its payload.
#[derive(Clone, Default)]
struct Reads {
    absorbs: bool,
    /// serde writes the type as no object.
    no_object: bool,
    refuses: Vec<Culprit>,
    rest: bool,
    unseen: Option<Unseen>,
    /// What makes the type refuse every key beside its own when read whole, as the payload of a
    /// newtype variant: an externally tagged enum, read from one entry.
    whole_unseen: Option<String>,
}

impl Reads {
    /// Adds what a part of the type reads to what the type refuses.
    fn carry(&mut self, part: &Self) {
        self.absorbs |= part.absorbs;
        self.no_object |= part.no_object;
        for culprit in &part.refuses {
            if !self.refuses.contains(culprit) {
                self.refuses.push(culprit.clone());
            }
        }
        self.see(part.unseen.clone().map(|unseen| Unseen {
            itself: false,
            ..unseen
        }));
    }

    fn class(&self) -> Class {
        if !self.rest {
            Class::Taker
        } else if self.absorbs {
            Class::Absorbing
        } else if !self.refuses.is_empty() {
            Class::Refusing(self.refuses.clone())
        } else if let Some(unseen) = &self.unseen {
            Class::Unseen(unseen.why.clone())
        } else {
            Class::Open
        }
    }

    /// The bits of the hidden constant this answers, but [`EMPTY`] and [`ENTRY`].
    const fn flags(&self) -> u16 {
        let mut flags = 0;
        if self.rest {
            flags |= REST;
        }
        if self.absorbs {
            flags |= ABSORBS;
        }
        if !self.refuses.is_empty() {
            flags |= REFUSES;
        }
        if self.unseen.is_some() || self.whole_unseen.is_some() {
            flags |= REFUSES | UNSEEN;
        }
        if self.no_object {
            flags |= NO_OBJECT;
        }
        flags
    }

    /// What a holder reads of a value serde writes as no object, as `why` names it.
    fn no_object(why: String) -> Self {
        Self {
            no_object: true,
            ..Self::not_met(why, false)
        }
    }

    fn not_met(why: String, resolvable: bool) -> Self {
        Self {
            rest: true,
            unseen: Some(Unseen {
                itself: true,
                resolvable,
                why,
            }),
            ..Self::default()
        }
    }

    fn see(&mut self, unseen: Option<Unseen>) {
        match (&mut self.unseen, unseen) {
            (Some(held), Some(more)) => {
                held.itself &= more.itself;
                held.resolvable &= more.resolvable;
            }
            (held @ None, Some(more)) => *held = Some(more),
            (_, None) => {}
        }
    }
}

/// What `#[model_schema]` recorded of a type, for the holders that flatten it.
#[derive(Clone)]
enum Record {
    Shape(Box<Shape>),
    /// A type serde writes as its one slot.
    Through {
        parameters: Vec<String>,
        slot: Box<FieldDef>,
    },
}

/// One level of a recorded type that flattens a member: what a filling of its type parameters is
/// held to.
#[derive(Clone)]
struct RecordedLevel {
    /// Each flattened member, by name, as declared.
    flattened: Vec<(String, FieldDef)>,
    own: Vec<(String, KeySet)>,
    /// The variant it is, for an enum.
    variant: Option<String>,
}

#[derive(Clone)]
struct Shape {
    /// A type it holds that the expansion had not met when it expanded the type, whose read on
    /// the Dart and Swift surfaces is then written without seeing what it reads.
    #[cfg(any(feature = "dart", feature = "swift"))]
    blind: Option<Blind>,
    closed: Option<Culprit>,
    empty: EmptyRule,
    /// The types whose keys are this type's keys too.
    keyed: Vec<FieldDef>,
    keys: KeySet,
    levels: Vec<RecordedLevel>,
    members: Vec<Part>,
    parameters: Vec<String>,
    payloads: Vec<Part>,
    rest: bool,
    whole_unseen: Option<String>,
    writes: Writes,
}

/// What serde writes of a recorded type that a holder of it cannot read, or reads only where it sees
/// the type.
#[derive(Clone, Copy, Default)]
struct Writes {
    /// An externally tagged enum with a unit variant, which serde writes flattened as an entry of
    /// the variant's name over `null`.
    entry: bool,
    /// An enum serde writes a unit variant of as its name, where it is not flattened.
    names_a_variant: bool,
    /// serde writes the type, or a value of it, as no object: a tuple struct, and an enum whose
    /// variant it writes as a payload that is no object.
    no_object: bool,
}

/// Why the expansion cannot see what a type reads, and whether the hidden constant of a type the
/// holder can name answers for it.
#[derive(Clone)]
struct Unseen {
    /// The type itself is unseen, so whether serde leaves its keys is the strictest guess rather
    /// than what the expansion read.
    itself: bool,
    resolvable: bool,
    why: String,
}

/// What a holder's flattened members earn, the member a refusal names by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No order and no strictness reads the member back beside the others.
    ChangeShape(usize),
    /// Every member takes its own keys: the holder may refuse a key none declares.
    Closable,
    /// The holder is written `#[serde(deny_unknown_fields)]`, by its author or by the macro, and
    /// flattens a member that reads every key still left, which the holder then refuses.
    ClosedByHand(usize),
    /// The member refuses the keys of a member beside it in every order, unless it is nonstrict.
    Nonstrict(usize),
    /// A member reads every key still left: the holder reads past a key none declares.
    Open,
    /// The member reads back only declared after every other flattened member.
    Reorder(usize),
}

/// A type a type holds that the expansion had not met when it expanded the type that holds it
/// there, and how that type holds it.
#[cfg(any(feature = "dart", feature = "swift"))]
#[derive(Clone)]
struct Blind {
    held: Held,
    name: String,
}

/// How a type holds a part of it that a holder of the type reads through the type's own read.
#[cfg(any(feature = "dart", feature = "swift"))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Held {
    /// A member it flattens, which the Dart and Swift reads of the type hand keys serde does not.
    Flattened,
    /// The payload of a newtype variant of an enum under a tag alone, which every read hands what
    /// serde hands it.
    TaggedPayload,
    /// The payload of a newtype variant of an untagged enum, which the Swift read hands keys serde
    /// does not.
    UntaggedPayload,
}

/// What the expansion answers of a question it may not see the answer to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    No,
    Unknown,
    Yes,
}

impl Answer {
    /// Both `self` and `other`, where an unknown answer stays unknown unless the other decides.
    const fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::No, _) | (_, Self::No) => Self::No,
            (Self::Yes, Self::Yes) => Self::Yes,
            (Self::Unknown, Self::Unknown | Self::Yes) | (Self::Yes, Self::Unknown) => {
                Self::Unknown
            }
        }
    }

    /// Either `self` or `other`, where an unknown answer stays unknown unless the other decides.
    const fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Yes, _) | (_, Self::Yes) => Self::Yes,
            (Self::No, Self::No) => Self::No,
            (Self::Unknown, Self::Unknown | Self::No) | (Self::No, Self::Unknown) => Self::Unknown,
        }
    }
}

/// Where serde reads a source of keys at a level.
#[derive(Clone, Copy)]
enum Place {
    /// A flattened member, at its place among them, and whether serde takes its keys off the
    /// object.
    Flattened { at: usize, takes: bool },
    /// A key the holder writes and does not read, which serde leaves for its flattened members.
    Left,
    /// A member of the holder's own, read ahead of every flattened member.
    Own,
}

/// One source of keys at a level, in the order serde reads them.
struct Source {
    keys: KeySet,
    label: String,
    place: Place,
}

impl Source {
    /// The place among the flattened members of a flattened source.
    const fn flattened_at(&self) -> Option<usize> {
        if let Place::Flattened { at, takes: _takes } = self.place {
            Some(at)
        } else {
            None
        }
    }
}

/// A refusal: why the members do not read back, and the fix, a clause.
struct Refusal {
    fix: String,
    reason: String,
}

impl Refusal {
    fn message(&self) -> String {
        format!("{}. {}", self.reason, sentence(&self.fix))
    }
}

thread_local! {
    /// What each type `#[model_schema]` has expanded so far reads, flattened.
    static RECORDS: RefCell<HashMap<String, Record>> = RefCell::new(HashMap::new());
}

/// The verdict of a holder whose flattened members are `classes`, in the order declared.
/// `closed_by_hand` says `#[serde(deny_unknown_fields)]` is written on it, and `leftover` that it
/// leaves a key of its own for its flattened members to read: its own tag, or a member serde
/// writes and never reads.
pub fn verdict(classes: &[Class], closed_by_hand: bool, leftover: bool) -> Verdict {
    let rest: Vec<usize> = (0..classes.len())
        .filter(|&at| classes[at].rest())
        .collect();
    if closed_by_hand && let Some(&first) = rest.first() {
        return Verdict::ClosedByHand(first);
    }
    let first_of =
        |wanted: fn(&Class) -> bool| rest.iter().copied().find(|&at| wanted(&classes[at]));
    // A member the expansion cannot see is held to the rule of a type parameter: the only member
    // beside it that reads every key, which a key the holder leaves is not.
    let beside_a_member = rest.len() > 1;
    let beside_any = beside_a_member || (leftover && !rest.is_empty());
    if beside_any && let Some(at) = first_of(|class| matches!(class, Class::Absorbing)) {
        return Verdict::ChangeShape(at);
    }
    if beside_a_member && let Some(at) = first_of(|class| matches!(class, Class::Unseen(_))) {
        return Verdict::ChangeShape(at);
    }
    if beside_any && let Some(at) = first_of(|class| matches!(class, Class::Refusing(_))) {
        return Verdict::Nonstrict(at);
    }
    if let [only] = rest.as_slice()
        && classes[*only].exclusive()
        && *only + 1 != classes.len()
    {
        return Verdict::Reorder(*only);
    }
    if rest.is_empty() {
        Verdict::Closable
    } else {
        Verdict::Open
    }
}

/// `tokens` as a refusal writes them, a path that opens with `::` among them.
fn spelled<T>(tokens: &T) -> String
where
    T: quote::ToTokens,
{
    written_spelling(tokens).replace(":: ", "::")
}

fn recorded(name: &str) -> Option<Record> {
    RECORDS.with(|records| records.borrow().get(name).cloned())
}

/// The names of the type parameters `generics` declares.
fn parameter_names(generics: &Generics) -> Vec<String> {
    generics
        .type_params()
        .map(|parameter| parameter.ident.to_string())
        .collect()
}

/// The definition of `ty`, each of `parameters` written as a type parameter.
fn def_of(ty: &Type, parameters: &[String]) -> FieldDef {
    let mut def = get_field_def("flattened", ty, "");
    def.erase_type_parameters(parameters);
    def
}

/// `def` with each type parameter `parameters` names filled with the argument at its position.
fn filled(def: &FieldDef, parameters: &[String], arguments: &[FieldDef]) -> FieldDef {
    if let FieldDefType::TypeParam(name) = &def.field_type
        && let Some(argument) = parameters
            .iter()
            .position(|parameter| parameter == name)
            .and_then(|at| arguments.get(at))
    {
        let mut placed = argument.clone();
        placed.array_depth = placed.array_depth.saturating_add(def.array_depth);
        for level in &def.nullable_levels {
            let shifted = level.saturating_add(argument.array_depth);
            if !placed.nullable_levels.contains(&shifted) {
                placed.nullable_levels.push(shifted);
            }
        }
        return placed;
    }
    let mut copy = def.clone();
    if let FieldDefType::SiblingType(_, held) = &mut copy.field_type {
        for argument in held.iter_mut() {
            *argument = filled(argument, parameters, arguments);
        }
    }
    copy
}

/// `def` without the `Option` around it.
fn under_option(def: &FieldDef) -> FieldDef {
    let mut held = def.clone();
    held.nullable_levels
        .retain(|level| *level != def.array_depth);
    held
}

/// What a holder reads of a member `def` describes.
fn reads(def: &FieldDef, depth: usize) -> Reads {
    if depth > DEPTH {
        return Reads::not_met(
            "a type held deeper than tixschema follows".to_owned(),
            false,
        );
    }
    if def.array_depth != 0 {
        return Reads::no_object("a list, which serde writes as no object".to_owned());
    }
    #[cfg(feature = "mongodb")]
    if matches!(def.field_type, FieldDefType::ObjectId) {
        // serde reads its `$oid` from every key still left and refuses none beside it.
        return Reads {
            rest: true,
            ..Reads::default()
        };
    }
    if let FieldDefType::Map(_, _) = &def.field_type {
        Reads {
            absorbs: true,
            rest: true,
            ..Reads::default()
        }
    } else if let FieldDefType::TypeParam(name) = &def.field_type {
        Reads::not_met(format!("the type parameter `{name}`"), false)
    } else if let FieldDefType::SiblingType(name, arguments) = &def.field_type {
        named_reads(def, name, arguments, depth)
    } else {
        Reads::no_object("a value serde writes as no object".to_owned())
    }
}

/// What a holder reads of the type `name`, filled with `arguments`, which `def` describes.
fn named_reads(def: &FieldDef, name: &str, arguments: &[FieldDef], depth: usize) -> Reads {
    if let Some(Declared::Alias(target)) = declared(name) {
        return syn::parse_str::<Type>(&target).map_or_else(
            |_| Reads::not_met(format!("the alias `{name}`"), false),
            |ty| {
                reads(
                    &get_field_def("flattened", &ty, ""),
                    depth.saturating_add(1),
                )
            },
        );
    }
    let Some(record) = recorded(name) else {
        return Reads::not_met(format!("`{name}`, {NOT_SEEN}"), true);
    };
    let shape = match record {
        Record::Through { parameters, slot } => {
            return reads(
                &filled(&slot, &parameters, arguments),
                depth.saturating_add(1),
            );
        }
        Record::Shape(shape) => shape,
    };
    let mut read = Reads {
        absorbs: def_writes_a_map(def),
        no_object: shape.writes.no_object,
        refuses: shape.closed.clone().into_iter().collect(),
        rest: shape.rest,
        unseen: None,
        whole_unseen: shape.whole_unseen.clone(),
    };
    for (part, payload) in shape
        .members
        .iter()
        .map(|member| (member, false))
        .chain(shape.payloads.iter().map(|payload| (payload, true)))
    {
        match part {
            Part::Corrected => {}
            Part::Read(held) => {
                let found = reads(
                    &filled(held, &shape.parameters, arguments),
                    depth.saturating_add(1),
                );
                if payload || found.rest {
                    read.carry(&found);
                }
                if payload && let Some(why) = found.whole_unseen {
                    read.see(Some(Unseen {
                        itself: false,
                        resolvable: false,
                        why,
                    }));
                }
            }
            Part::Unseen(why) => read.see(Some(Unseen {
                itself: false,
                resolvable: false,
                why: why.clone(),
            })),
        }
    }
    read
}

/// The keys a value `def` describes is written and read under, as far as the expansion sees.
fn keys(def: &FieldDef, depth: usize) -> KeySet {
    #[cfg(feature = "mongodb")]
    if matches!(def.field_type, FieldDefType::ObjectId) && def.array_depth == 0 {
        return KeySet::both("$oid".to_owned());
    }
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return KeySet::default();
    };
    if depth > DEPTH || def.array_depth != 0 {
        return KeySet::default();
    }
    if let Some(Declared::Alias(target)) = declared(name) {
        return syn::parse_str::<Type>(&target).map_or_else(
            |_| KeySet::default(),
            |ty| {
                keys(
                    &get_field_def("flattened", &ty, ""),
                    depth.saturating_add(1),
                )
            },
        );
    }
    match recorded(name) {
        Some(Record::Through { parameters, slot }) => keys(
            &filled(&slot, &parameters, arguments),
            depth.saturating_add(1),
        ),
        Some(Record::Shape(shape)) => {
            let mut found = shape.keys.clone();
            for held in &shape.keyed {
                found.extend(&keys(
                    &filled(held, &shape.parameters, arguments),
                    depth.saturating_add(1),
                ));
            }
            found
        }
        None => KeySet::default(),
    }
}

/// Whether [`keys`] answers every key a value `def` describes is written and read under: every
/// type it reaches was recorded above it. A type reached deeper than [`DEPTH`] holds itself, and
/// adds no key it has not added already.
fn keys_complete(def: &FieldDef, depth: usize) -> bool {
    #[cfg(feature = "mongodb")]
    if matches!(def.field_type, FieldDefType::ObjectId) {
        return true;
    }
    if let FieldDefType::TypeParam(_) = &def.field_type {
        return false;
    }
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return true;
    };
    if def.array_depth != 0 || depth > DEPTH {
        return true;
    }
    if let Some(Declared::Alias(target)) = declared(name) {
        return syn::parse_str::<Type>(&target).is_ok_and(|ty| {
            keys_complete(
                &get_field_def("flattened", &ty, ""),
                depth.saturating_add(1),
            )
        });
    }
    match recorded(name) {
        Some(Record::Through { parameters, slot }) => keys_complete(
            &filled(&slot, &parameters, arguments),
            depth.saturating_add(1),
        ),
        Some(Record::Shape(shape)) => shape.keyed.iter().all(|held| {
            keys_complete(
                &filled(held, &shape.parameters, arguments),
                depth.saturating_add(1),
            )
        }),
        None => false,
    }
}

/// The hidden constant of the keys of `ty`, as a `const` beside the declaration reads it, or
/// `None` where it cannot name `ty`.
fn named_keys(ty: &Type, parameters: &[String]) -> Option<TokenStream> {
    let written = peeled(ty).to_token_stream();
    if names_the_declaration(written.clone(), parameters) {
        return None;
    }
    let named = with_static_lifetimes(written);
    let keys = proc_macro2::Ident::new(KEYS, Span::call_site());
    Some(quote! { <#named>::#keys })
}

/// The hidden constant of the keys of a part `ty` of a type, where the expansion cannot see every
/// key it writes and can name it.
fn unseen_keys(ty: &Type, parameters: &[String]) -> Option<TokenStream> {
    (!keys_complete(&def_of(ty, parameters), 0))
        .then(|| named_keys(ty, parameters))
        .flatten()
}

/// Whether a value `def` describes reads from an object that holds none of its keys, where the
/// expansion sees it: a map, an `Option`, and a type every member of which reads when its key is
/// absent.
fn empty_of(def: &FieldDef, depth: usize) -> Answer {
    if depth > DEPTH {
        return Answer::Unknown;
    }
    if def.array_depth != 0 {
        return Answer::No;
    }
    if def.is_optional() {
        return Answer::Yes;
    }
    if let FieldDefType::Map(_, _) = &def.field_type {
        return Answer::Yes;
    }
    if let FieldDefType::TypeParam(_) = &def.field_type {
        return Answer::Unknown;
    }
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return Answer::No;
    };
    if let Some(Declared::Alias(target)) = declared(name) {
        return syn::parse_str::<Type>(&target).map_or(Answer::Unknown, |ty| {
            empty_of(
                &get_field_def("flattened", &ty, ""),
                depth.saturating_add(1),
            )
        });
    }
    match recorded(name) {
        Some(Record::Through { parameters, slot }) => empty_of(
            &filled(&slot, &parameters, arguments),
            depth.saturating_add(1),
        ),
        Some(Record::Shape(shape)) => empty_by(
            &shape.empty,
            &shape.parameters,
            arguments,
            depth.saturating_add(1),
        ),
        None => Answer::Unknown,
    }
}

/// [`empty_of`] of a type whose rule is `rule`, its `parameters` filled with `arguments`.
fn empty_by(
    rule: &EmptyRule,
    parameters: &[String],
    arguments: &[FieldDef],
    depth: usize,
) -> Answer {
    match rule {
        EmptyRule::AllOf(parts) => parts.iter().fold(Answer::Yes, |held, part| {
            held.and(match part {
                Part::Read(def) => empty_of(&filled(def, parameters, arguments), depth),
                Part::Corrected | Part::Unseen(_) => Answer::Unknown,
            })
        }),
        EmptyRule::AnyOf(rules) => rules.iter().fold(Answer::No, |held, variant| {
            held.or(empty_by(variant, parameters, arguments, depth))
        }),
        EmptyRule::Never => Answer::No,
        EmptyRule::Unknown => Answer::Unknown,
    }
}

/// Whether a value `def` describes is an externally tagged enum with a unit variant, as far as the
/// expansion sees.
fn entry_of(def: &FieldDef, depth: usize) -> bool {
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return false;
    };
    if depth > DEPTH || def.array_depth != 0 {
        return false;
    }
    match recorded(name) {
        Some(Record::Through { parameters, slot }) => entry_of(
            &filled(&slot, &parameters, arguments),
            depth.saturating_add(1),
        ),
        Some(Record::Shape(shape)) => shape.writes.entry,
        None => false,
    }
}

/// Whether serde reads `field`.
fn read_by_serde(field: &Field) -> bool {
    !parse_serde_key_omission(&field.attrs).skips_deserializing
}

fn flattened(field: &Field) -> bool {
    parse_serde_field_attributes(&field.attrs).flatten && read_by_serde(field)
}

/// Whether a member names a function of its author's that serde reads it through: a
/// `preprocess`, `deserialize_with` or `with`.
fn read_by_a_function(field: &Field) -> bool {
    read_hook(&field.attrs).is_some()
        || !parse_model_schema_prop_attributes(&field.attrs)
            .preprocess
            .is_empty()
}

/// What a flattened member, or a payload, decides of its type.
fn part(field: &Field, parameters: &[String]) -> Part {
    if read_hook(&field.attrs).is_some() {
        return Part::Unseen("read through a function of its author's".to_owned());
    }
    if !parse_model_schema_prop_attributes(&field.attrs)
        .preprocess
        .is_empty()
    {
        return Part::Corrected;
    }
    Part::Read(Box::new(def_of(&field.ty, parameters)))
}

/// What a holder reads of its flattened member `field`.
fn member_reads(field: &Field, parameters: &[String]) -> Reads {
    match part(field, parameters) {
        Part::Corrected => Reads {
            rest: true,
            ..Reads::default()
        },
        Part::Read(def) => reads(&def, 0),
        Part::Unseen(why) => Reads::not_met(why, false),
    }
}

/// Whether serde reads `field`, a member under a key of its own, from an object that does not
/// hold its key: it is skipped, defaulted by itself or by its container, or an `Option` serde
/// reads with no function of its author's.
fn absent_reads(field: &Field, container_defaulted: bool, parameters: &[String]) -> bool {
    let omission = parse_serde_key_omission(&field.attrs);
    omission.skips_deserializing
        || omission.defaulted
        || container_defaulted
        || (read_hook(&field.attrs).is_none() && def_of(&field.ty, parameters).is_optional())
}

/// Every key the members of `fields` serde writes or reads under their own keys declare, each
/// beside the member that declares it.
fn own_keys(fields: &Fields, rename_all: Option<&str>) -> Vec<(String, KeySet)> {
    let rule = resolve_rename_rule(rename_all);
    let mut found = Vec::new();
    for field in fields {
        let Some(ident) = &field.ident else {
            continue;
        };
        let meta = parse_serde_field_attributes(&field.attrs);
        let omission = parse_serde_key_omission(&field.attrs);
        if meta.flatten || omission.absent_from_wire() {
            continue;
        }
        let name = ident.unraw().to_string();
        let mut keys = KeySet::default();
        if !omission.skips_deserializing {
            keys.read.push(
                parse_read_renames(&field.attrs)
                    .rename
                    .unwrap_or_else(|| rule.apply_to_field(&name)),
            );
            for alias in meta.aliases {
                if !keys.read.contains(&alias) {
                    keys.read.push(alias);
                }
            }
        }
        keys.written.push(
            parse_written_renames(&field.attrs)
                .rename
                .unwrap_or_else(|| rule.apply_to_field(&name)),
        );
        found.push((name, keys));
    }
    found
}

/// The keys of `own` together.
fn joined(own: &[(String, KeySet)]) -> KeySet {
    let mut keys = KeySet::default();
    for (_, held) in own {
        keys.extend(held);
    }
    keys
}

/// Each of `keys` beside the member that declares it, as a refusal names it.
fn sources_of(keys: Vec<(String, KeySet)>) -> Vec<(String, KeySet)> {
    keys.into_iter()
        .map(|(member, held)| (format!("field `{member}`"), held))
        .collect()
}

/// Whether serde writes a member of `fields` under a key and never reads it.
fn writes_an_unread_key(fields: &Fields) -> bool {
    fields.iter().any(|field| {
        let omitted = parse_serde_key_omission(&field.attrs);
        omitted.skips_deserializing && !omitted.omits_key
    })
}

/// The casing serde applies to the members of `variant` of `item_enum`.
fn variant_rename_all(item_enum: &ItemEnum, variant: &Variant) -> Option<String> {
    parse_read_renames(&variant.attrs)
        .rename_all
        .or_else(|| parse_serde_type_attributes(&item_enum.attrs).rename_all_fields)
}

/// The names serde reads and writes each variant of `item_enum` under, and each alias.
fn variant_names(item_enum: &ItemEnum) -> KeySet {
    let read_rule = resolve_rename_rule(parse_read_renames(&item_enum.attrs).rename_all.as_deref());
    let written_rule = resolve_rename_rule(
        parse_written_renames(&item_enum.attrs)
            .rename_all
            .as_deref(),
    );
    let mut found = KeySet::default();
    for variant in item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
    {
        let name = variant.ident.unraw().to_string();
        let mut named = KeySet::default();
        named.read.push(
            parse_read_renames(&variant.attrs)
                .rename
                .unwrap_or_else(|| read_rule.apply_to_variant(&name)),
        );
        named
            .read
            .extend(parse_serde_field_attributes(&variant.attrs).aliases);
        named.written.push(
            parse_written_renames(&variant.attrs)
                .rename
                .unwrap_or_else(|| written_rule.apply_to_variant(&name)),
        );
        found.extend(&named);
    }
    found
}

/// The single slot of a newtype variant.
fn newtype_slot(variant: &Variant) -> Option<&Field> {
    match &variant.fields {
        Fields::Unnamed(slots) if slots.unnamed.len() == 1 => slots.unnamed.first(),
        Fields::Named(_) | Fields::Unnamed(_) | Fields::Unit => None,
    }
}

/// Whether serde reads `item_enum` from every key still left: under a tag alone, or untagged.
fn reads_every_key(item_enum: &ItemEnum) -> bool {
    let tagging = Tagging::of(item_enum);
    if let Tagging::Internal { tag: _tag } = tagging {
        true
    } else {
        matches!(tagging, Tagging::Untagged)
    }
}

/// The members of `item` that decide what it refuses, each with whether it is the payload of a
/// newtype variant: the members a struct, or a struct variant of an enum serde reads from every
/// key still left, flattens, and the payload of each newtype variant of such an enum.
fn parts_of(item: &Item) -> Vec<(&Field, bool)> {
    if let Item::Struct(item_struct) = item {
        if written_as_its_slot(item_struct).is_some() {
            return Vec::new();
        }
        return item_struct
            .fields
            .iter()
            .filter(|field| flattened(field))
            .map(|field| (field, false))
            .collect();
    }
    let Item::Enum(item_enum) = item else {
        return Vec::new();
    };
    if !reads_every_key(item_enum) {
        return Vec::new();
    }
    let mut parts = Vec::new();
    for variant in item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
    {
        if let Some(slot) = newtype_slot(variant) {
            if read_by_serde(slot) {
                parts.push((slot, true));
            }
        } else {
            parts.extend(
                variant
                    .fields
                    .iter()
                    .filter(|field| flattened(field))
                    .map(|field| (field, false)),
            );
        }
    }
    parts
}

/// The rule of whether `fields`, read under `container_defaulted`, read from an object that holds
/// none of their keys.
fn fields_empty(fields: &Fields, container_defaulted: bool, parameters: &[String]) -> EmptyRule {
    let mut parts = Vec::new();
    for field in fields {
        if !read_by_serde(field) {
            continue;
        }
        if parse_serde_field_attributes(&field.attrs).flatten {
            if read_by_a_function(field) {
                return EmptyRule::Unknown;
            }
            parts.push(Part::Read(Box::new(def_of(&field.ty, parameters))));
        } else if !absent_reads(field, container_defaulted, parameters) {
            return EmptyRule::Never;
        } else {
            // Read where its key is absent, so it adds nothing to the rule.
        }
    }
    EmptyRule::AllOf(parts)
}

/// The levels of `fields` that flatten a member, for the fillings of the type's parameters.
fn recorded_level(
    fields: &Fields,
    rename_all: Option<&str>,
    parameters: &[String],
    variant: Option<String>,
) -> Option<RecordedLevel> {
    let flattened_members: Vec<(String, FieldDef)> = fields
        .iter()
        .filter(|field| flattened(field))
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            Some((ident.unraw().to_string(), def_of(&field.ty, parameters)))
        })
        .collect();
    (!flattened_members.is_empty()).then(|| RecordedLevel {
        flattened: flattened_members,
        own: own_keys(fields, rename_all),
        variant,
    })
}

/// The type `def` names, or one it holds, that the expansion has not met, or whose own read was
/// written without meeting one: `named` is the type being recorded, which holds itself.
#[cfg(any(feature = "dart", feature = "swift"))]
fn blind_of(def: &FieldDef, named: &str, held: Held, depth: usize) -> Option<Blind> {
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return None;
    };
    if depth > DEPTH || name == named || name == "Self" {
        return None;
    }
    if let Some(Declared::Alias(target)) = declared(name) {
        let ty = syn::parse_str::<Type>(&target).ok()?;
        return blind_of(
            &get_field_def("flattened", &ty, ""),
            named,
            held,
            depth.saturating_add(1),
        );
    }
    match recorded(name) {
        Some(Record::Through { parameters, slot }) => blind_of(
            &filled(&slot, &parameters, arguments),
            named,
            held,
            depth.saturating_add(1),
        ),
        Some(Record::Shape(shape)) => shape.blind.clone().filter(|found| found.name != named),
        None => Some(Blind {
            held,
            name: name.clone(),
        }),
    }
}

/// `shape`, the record of the type `named`, with the part of it [`blind_part`] names, the
/// payloads of its variants held by an `untagged` enum or by a tagged one.
#[cfg(any(feature = "dart", feature = "swift"))]
fn blinded(mut shape: Shape, named: &str, untagged: bool) -> Shape {
    let payloads = if untagged {
        Held::UntaggedPayload
    } else {
        Held::TaggedPayload
    };
    shape.blind = blind_part(&shape, named, payloads);
    shape
}

/// `shape` as it is, in a build with no Dart and no Swift read.
#[cfg(not(any(feature = "dart", feature = "swift")))]
const fn blinded(shape: Shape, _named: &str, _untagged: bool) -> Shape {
    shape
}

/// The first part of `shape`, the record of the type `named`, that [`blind_of`] names.
#[cfg(any(feature = "dart", feature = "swift"))]
fn blind_part(shape: &Shape, named: &str, payloads: Held) -> Option<Blind> {
    let blind = |part: &Part, held: Held| {
        if let Part::Read(def) = part {
            blind_of(def, named, held, 0)
        } else {
            None
        }
    };
    shape
        .members
        .iter()
        .find_map(|part| blind(part, Held::Flattened))
        .or_else(|| shape.payloads.iter().find_map(|part| blind(part, payloads)))
}

fn struct_record(item_struct: &ItemStruct, args: &ModelSchemaArgs) -> Record {
    let parameters = parameter_names(&item_struct.generics);
    if let Some(slot) = written_as_its_slot(item_struct) {
        return Record::Through {
            slot: Box::new(def_of(&slot.ty, &parameters)),
            parameters,
        };
    }
    let corrected = !args.preprocess.is_empty();
    let mut shape = Shape {
        #[cfg(any(feature = "dart", feature = "swift"))]
        blind: None,
        closed: strict::struct_culprit(item_struct, args),
        empty: if corrected {
            EmptyRule::Unknown
        } else {
            fields_empty(
                &item_struct.fields,
                derive_reads_a_default(&item_struct.attrs),
                &parameters,
            )
        },
        keyed: Vec::new(),
        keys: KeySet::default(),
        levels: Vec::new(),
        members: Vec::new(),
        parameters: parameters.clone(),
        payloads: Vec::new(),
        rest: corrected,
        whole_unseen: None,
        writes: Writes::default(),
    };
    if matches!(item_struct.fields, Fields::Unnamed(_)) {
        shape.rest = true;
        shape.writes.no_object = true;
        shape.empty = EmptyRule::Never;
        shape.members.push(Part::Unseen(
            "a tuple, which serde writes as no object".to_owned(),
        ));
        return Record::Shape(Box::new(shape));
    }
    let rename_all = parse_read_renames(&item_struct.attrs).rename_all;
    shape.keys = joined(&own_keys(&item_struct.fields, rename_all.as_deref()));
    if let Some((tag, _)) = struct_tag(item_struct) {
        shape.keys.extend(&KeySet::both(tag));
        shape.rest = true;
    }
    shape.levels.extend(recorded_level(
        &item_struct.fields,
        rename_all.as_deref(),
        &parameters,
        None,
    ));
    for (field, _) in parts_of(&Item::Struct(item_struct.clone())) {
        shape.rest = true;
        shape.keyed.push(def_of(&field.ty, &parameters));
        shape.members.push(part(field, &parameters));
    }
    Record::Shape(Box::new(blinded(
        shape,
        &item_struct.ident.to_string(),
        false,
    )))
}

/// The rule of whether a variant of an untagged enum reads from an object that holds none of its
/// keys.
fn variant_empty(variant: &Variant, defaulted: bool, parameters: &[String]) -> EmptyRule {
    if let Some(slot) = newtype_slot(variant) {
        return if read_hook(&slot.attrs).is_some() {
            EmptyRule::Unknown
        } else {
            EmptyRule::AllOf(vec![Part::Read(Box::new(def_of(&slot.ty, parameters)))])
        };
    }
    match &variant.fields {
        Fields::Named(_) => fields_empty(&variant.fields, defaulted, parameters),
        Fields::Unit => EmptyRule::AllOf(Vec::new()),
        Fields::Unnamed(_) => EmptyRule::Never,
    }
}

fn enum_record(item_enum: &ItemEnum, args: &ModelSchemaArgs) -> Record {
    let parameters = parameter_names(&item_enum.generics);
    let corrected = !args.preprocess.is_empty();
    let mut shape = Shape {
        #[cfg(any(feature = "dart", feature = "swift"))]
        blind: None,
        closed: strict::enum_culprit(item_enum, args),
        empty: if corrected {
            EmptyRule::Unknown
        } else {
            EmptyRule::Never
        },
        keyed: Vec::new(),
        keys: KeySet::default(),
        levels: Vec::new(),
        members: Vec::new(),
        parameters: parameters.clone(),
        payloads: Vec::new(),
        rest: corrected,
        whole_unseen: None,
        writes: Writes::default(),
    };
    let on_the_wire = || {
        item_enum
            .variants
            .iter()
            .filter(|variant| variant_on_the_wire(variant))
    };
    for variant in on_the_wire() {
        let rename_all = variant_rename_all(item_enum, variant);
        shape.levels.extend(recorded_level(
            &variant.fields,
            rename_all.as_deref(),
            &parameters,
            Some(variant.ident.unraw().to_string()),
        ));
    }
    match Tagging::of(item_enum) {
        tagging @ (Tagging::External | Tagging::Plain) => {
            shape.keys = variant_names(item_enum);
            // A unit variant of an enum all of whose variants are unit variants every surface
            // reads as an entry over `null` wherever the enum is declared.
            shape.writes.names_a_variant =
                on_the_wire().any(|variant| matches!(variant.fields, Fields::Unit));
            shape.writes.entry =
                matches!(tagging, Tagging::External) && shape.writes.names_a_variant;
            shape.whole_unseen =
                Some("an externally tagged enum, which serde reads from one entry".to_owned());
            return Record::Shape(Box::new(shape));
        }
        Tagging::Adjacent { content, tag } => {
            shape.keys = KeySet::both(tag);
            shape.keys.extend(&KeySet::both(content));
            return Record::Shape(Box::new(shape));
        }
        Tagging::Internal { tag } => shape.keys = KeySet::both(tag),
        Tagging::Untagged => {
            if !corrected {
                let defaulted = derive_reads_a_default(&item_enum.attrs);
                shape.empty = EmptyRule::AnyOf(
                    on_the_wire()
                        .map(|variant| variant_empty(variant, defaulted, &parameters))
                        .collect(),
                );
            }
        }
    }
    shape.rest = true;
    // The enum's own `deny_unknown_fields` reaches its struct variants, and its unit variants where
    // the macro reads them refusing a key: a newtype variant is read by its payload.
    let refusing_units = strict::refuses_beside_a_unit(item_enum, args);
    if !on_the_wire().any(|variant| matches!(variant.fields, Fields::Named(_))) && !refusing_units {
        shape.closed = None;
    }
    for variant in on_the_wire().filter(|variant| newtype_slot(variant).is_none()) {
        let rename_all = variant_rename_all(item_enum, variant);
        shape
            .keys
            .extend(&joined(&own_keys(&variant.fields, rename_all.as_deref())));
    }
    for (field, payload) in parts_of(&Item::Enum(item_enum.clone())) {
        shape.keyed.push(def_of(&field.ty, &parameters));
        if payload {
            shape.writes.no_object |= read_hook(&field.attrs).is_none()
                && writes_no_object(&def_of(&field.ty, &parameters), 0);
            shape.payloads.push(part(field, &parameters));
        } else {
            shape.members.push(part(field, &parameters));
        }
    }
    Record::Shape(Box::new(blinded(
        shape,
        &item_enum.ident.to_string(),
        matches!(Tagging::of(item_enum), Tagging::Untagged),
    )))
}

/// Whether serde can write a value `def` describes as no object, where the expansion sees it: a
/// list, an `Option`, which it writes `None` of as `null`, a value that is no struct, an enum it
/// writes a variant of as its name, and a type that holds one as the payload a variant is written
/// as.
fn writes_no_object(def: &FieldDef, depth: usize) -> bool {
    if depth > DEPTH {
        return false;
    }
    if def.array_depth != 0 || def.is_optional() {
        return true;
    }
    if let FieldDefType::Map(_, _) | FieldDefType::TypeParam(_) = &def.field_type {
        return false;
    }
    #[cfg(feature = "mongodb")]
    if matches!(def.field_type, FieldDefType::ObjectId) {
        return false;
    }
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return true;
    };
    match recorded(name) {
        Some(Record::Through { parameters, slot }) => writes_no_object(
            &filled(&slot, &parameters, arguments),
            depth.saturating_add(1),
        ),
        Some(Record::Shape(shape)) => shape.writes.no_object || shape.writes.names_a_variant,
        None => false,
    }
}

/// Records what `item` reads, flattened, for the holders expanded after it.
pub(super) fn record(item: &Item, args: &ModelSchemaArgs) {
    let (name, held) = if let Item::Struct(item_struct) = item {
        (
            item_struct.ident.to_string(),
            struct_record(item_struct, args),
        )
    } else if let Item::Enum(item_enum) = item {
        (item_enum.ident.to_string(), enum_record(item_enum, args))
    } else if let Item::Type(alias) = item {
        // An alias is read through what it names, with each of its parameters filled.
        let parameters = parameter_names(&alias.generics);
        (
            alias.ident.to_string(),
            Record::Through {
                slot: Box::new(def_of(&alias.ty, &parameters)),
                parameters,
            },
        )
    } else {
        return;
    };
    RECORDS.with(|records| records.borrow_mut().insert(name, held));
}

/// Whether every member `fields` flattens takes its own keys, so a holder of them may refuse a key
/// none declares. A member of a type the expansion did not see is held to take them, and the
/// `const` its holder earns refuses the build where its hidden constant says it does not.
pub fn takes_only(fields: &Fields, generics: &Generics) -> bool {
    let parameters = parameter_names(generics);
    fields.iter().filter(|field| flattened(field)).all(|field| {
        let read = member_reads(field, &parameters);
        !read.rest
            || read
                .unseen
                .as_ref()
                .is_some_and(|unseen| unseen.itself && unseen.resolvable)
    })
}

/// `tokens` with every lifetime written `'static`, which a `const` beside the declaration can name.
fn with_static_lifetimes(tokens: TokenStream) -> TokenStream {
    let mut after_a_quote = false;
    tokens
        .into_iter()
        .map(|token| {
            let placed = if let TokenTree::Group(group) = &token {
                let mut copy = proc_macro2::Group::new(
                    group.delimiter(),
                    with_static_lifetimes(group.stream()),
                );
                copy.set_span(group.span());
                TokenTree::Group(copy)
            } else if let TokenTree::Ident(ident) = &token
                && after_a_quote
            {
                TokenTree::Ident(proc_macro2::Ident::new("static", ident.span()))
            } else {
                token
            };
            after_a_quote = matches!(&placed, TokenTree::Punct(punct) if punct.as_char() == '\'');
            placed
        })
        .collect()
}

/// Whether `tokens` name one of `parameters` or `Self`, which a `const` beside the declaration
/// cannot name.
fn names_the_declaration(tokens: TokenStream, parameters: &[String]) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Group(group) => names_the_declaration(group.stream(), parameters),
        TokenTree::Ident(ident) => {
            let name = ident.unraw().to_string();
            name == "Self" || parameters.contains(&name)
        }
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

/// What `ty` holds under its outermost wrapper named one of `wrappers`, or `None` where it is no
/// such wrapper.
fn wrapped<'ty>(ty: &'ty Type, wrappers: &[&str]) -> Option<&'ty Type> {
    if let Type::Paren(parenthesized) = ty {
        return wrapped(&parenthesized.elem, wrappers);
    }
    if let Type::Group(grouped) = ty {
        return wrapped(&grouped.elem, wrappers);
    }
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    if path.qself.is_some() || !wrappers.contains(&last.ident.to_string().as_str()) {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    arguments.args.iter().rev().find_map(|argument| {
        if let syn::GenericArgument::Type(inner) = argument {
            Some(inner)
        } else {
            None
        }
    })
}

/// `ty` without the `Option` and the transparent pointers around it, and without each recorded type
/// serde writes as the one type parameter it holds, whose hidden constant answers for it.
fn peeled(ty: &Type) -> &Type {
    wrapped(ty, &["Option", "Box", "Rc", "Arc", "Cow"])
        .or_else(|| through_argument(ty))
        .map_or(ty, peeled)
}

/// The argument `ty` fills the type parameter with that the recorded type it names writes as its
/// one slot, or `None` where it names no such type.
fn through_argument(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    let Some(Record::Through { parameters, slot }) = recorded(&last.ident.unraw().to_string())
    else {
        return None;
    };
    let FieldDefType::TypeParam(name) = &slot.field_type else {
        return None;
    };
    if slot.array_depth != 0 || slot.is_optional() {
        return None;
    }
    let at = parameters.iter().position(|parameter| parameter == name)?;
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| {
            if let syn::GenericArgument::Type(held) = argument {
                Some(held)
            } else {
                None
            }
        })
        .nth(at)
}

/// The hidden constant of `ty`, as a `const` beside the declaration reads it, or `None` where it
/// cannot name `ty`.
fn named_flags(ty: &Type, parameters: &[String]) -> Option<TokenStream> {
    let written = peeled(ty).to_token_stream();
    if names_the_declaration(written.clone(), parameters) {
        return None;
    }
    let named = with_static_lifetimes(written);
    let flags = proc_macro2::Ident::new(FLAGS, Span::call_site());
    Some(quote! { <#named>::#flags })
}

fn struct_level<'item>(
    item_struct: &'item ItemStruct,
    args: &ModelSchemaArgs,
) -> Option<Level<'item>> {
    if !matches!(item_struct.fields, Fields::Named(_)) {
        return None;
    }
    let named = item_struct.ident.to_string();
    let tag = struct_tag(item_struct).map(|(key, _)| key);
    let leftover = tag
        .as_ref()
        .map(|key| format!("its own tag `{key}`"))
        .or_else(|| {
            writes_an_unread_key(&item_struct.fields)
                .then(|| "a member serde writes and never reads".to_owned())
        });
    let rename_all = parse_read_renames(&item_struct.attrs).rename_all;
    let mut own = sources_of(own_keys(&item_struct.fields, rename_all.as_deref()));
    if let Some(key) = tag {
        own.push((
            "its own tag".to_owned(),
            KeySet {
                read: Vec::new(),
                written: vec![key],
            },
        ));
    }
    Some(Level {
        closed_by_macro: strict::closes_struct(item_struct, args),
        closed_by_hand: strict::closed_by_hand(&item_struct.attrs),
        fields: &item_struct.fields,
        leftover,
        subject: format!("type `{named}`"),
        named,
        own_keys: own,
    })
}

fn variant_levels<'item>(item_enum: &'item ItemEnum, args: &ModelSchemaArgs) -> Vec<Level<'item>> {
    let named = item_enum.ident.to_string();
    let tag = if let Tagging::Internal { tag } = Tagging::of(item_enum) {
        Some(tag)
    } else {
        None
    };
    let closed_by_macro = strict::closes_enum(item_enum, args);
    item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
        .filter(|variant| matches!(variant.fields, Fields::Named(_)))
        .map(|variant| {
            let rename_all = variant_rename_all(item_enum, variant);
            let mut own = sources_of(own_keys(&variant.fields, rename_all.as_deref()));
            if let Some(key) = &tag {
                own.push(("the enum's tag".to_owned(), KeySet::both(key.clone())));
            }
            Level {
                closed_by_macro,
                closed_by_hand: strict::closed_by_hand(&item_enum.attrs),
                fields: &variant.fields,
                leftover: writes_an_unread_key(&variant.fields)
                    .then(|| "a member serde writes and never reads".to_owned()),
                subject: format!("type `{named}`, variant `{}`", variant.ident.unraw()),
                named: named.clone(),
                own_keys: own,
            }
        })
        .collect()
}

/// What a flattened `Option` member typed `ty` holds, or `None` for a member that is no `Option`.
fn optional_of(field: &Field, parameters: &[String]) -> Option<Optional> {
    if read_by_a_function(field) {
        return None;
    }
    let def = def_of(&field.ty, parameters);
    if def.array_depth != 0 {
        return None;
    }
    if !def.is_optional() {
        // An alias or a brand serde writes as an `Option` is read as one.
        let slot = under_option(&written_as(&def, 0)?);
        return Some(Optional {
            empty: empty_of(&slot, 0),
            refusing: refusing_culprits(&slot),
            written: spelled(&field.ty),
        });
    }
    let inner = wrapped(&field.ty, &["Option"]);
    let held = inner.map_or_else(|| under_option(&def), |ty| def_of(ty, parameters));
    Some(Optional {
        empty: empty_of(&held, 0),
        refusing: refusing_culprits(&held),
        written: inner.map_or_else(|| spelled(&field.ty), spelled),
    })
}

/// The types that make a value `def` describes refuse a key it does not declare, where it reads
/// every key still left and the expansion sees that it refuses one.
fn refusing_culprits(def: &FieldDef) -> Option<Vec<Culprit>> {
    let read = reads(def, 0);
    // A value serde can write as no object is refused for that where it is flattened.
    if read.no_object {
        return None;
    }
    if let Class::Refusing(culprits) = read.class() {
        Some(culprits)
    } else {
        None
    }
}

/// The `Option` the recorded alias or brand `def` names is written as, where it is written as one.
fn written_as(def: &FieldDef, depth: usize) -> Option<FieldDef> {
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return None;
    };
    if depth > DEPTH || def.array_depth != 0 {
        return None;
    }
    let Some(Record::Through { parameters, slot }) = recorded(name) else {
        return None;
    };
    let held = filled(&slot, &parameters, arguments);
    if held.array_depth == 0 && held.is_optional() {
        Some(held)
    } else {
        written_as(&held, depth.saturating_add(1))
    }
}

fn members_of<'item>(level: &Level<'item>, parameters: &[String]) -> Vec<Member<'item>> {
    level
        .fields
        .iter()
        .filter(|field| flattened(field))
        .map(|field| {
            let read = member_reads(field, parameters);
            let class = read.class();
            let resolvable = read.unseen.as_ref().is_some_and(|unseen| unseen.resolvable);
            let dynamic = (matches!(class, Class::Unseen(_)) && resolvable)
                .then(|| named_flags(&field.ty, parameters))
                .flatten();
            let dynamic_keys = if read_by_a_function(field) || under_a_cfg(field) {
                None
            } else {
                unseen_keys(&field.ty, parameters)
            };
            Member {
                #[cfg(any(feature = "dart", feature = "swift"))]
                blind: if dynamic.is_some() || read_by_a_function(field) {
                    None
                } else {
                    blind_of(
                        &def_of(&field.ty, parameters),
                        &level.named,
                        Held::Flattened,
                        0,
                    )
                },
                dynamic,
                dynamic_keys,
                class,
                field,
                keys: if read_by_a_function(field) {
                    KeySet::default()
                } else {
                    keys(&def_of(&field.ty, parameters), 0)
                },
                name: field
                    .ident
                    .as_ref()
                    .map_or_else(String::new, |ident| ident.unraw().to_string()),
                optional: optional_of(field, parameters),
                written: spelled(&field.ty),
            }
        })
        .collect()
}

/// `` `a` ``, `` `a` and `b` `` or `` `a`, `b` and `c` ``.
fn listed(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    match quoted.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [ahead @ .., last] => format!("{} and {last}", ahead.join(", ")),
    }
}

/// `clause` with its first letter in capitals, and a full stop.
fn sentence(clause: &str) -> String {
    let mut written = clause.to_owned();
    if let Some(first) = written.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    written.push('.');
    written
}

/// The names of the members declared after the one at `at`, as a refusal lists them.
fn later_than(members: &[Member<'_>], at: usize) -> String {
    let names: Vec<&str> = members
        .iter()
        .skip(at.saturating_add(1))
        .map(|later| later.name.as_str())
        .collect();
    listed(&names)
}

/// What the author changes so that none of `culprits` refuses a key.
fn nonstrict_fix(culprits: &[Culprit]) -> String {
    let marked: Vec<&str> = culprits
        .iter()
        .filter(|culprit| !culprit.by_hand)
        .map(|culprit| culprit.name.as_str())
        .collect();
    let written: Vec<&str> = culprits
        .iter()
        .filter(|culprit| culprit.by_hand)
        .map(|culprit| culprit.name.as_str())
        .collect();
    let mut fix = Vec::new();
    if !marked.is_empty() {
        fix.push(format!(
            "mark {} `#[model_schema(nonstrict)]`",
            listed(&marked)
        ));
    }
    if !written.is_empty() {
        fix.push(format!(
            "take `#[serde(deny_unknown_fields)]` off {}",
            listed(&written)
        ));
    }
    let mut joined_fix = fix.join(", and ");
    if culprits.iter().any(|culprit| culprit.holds_more) {
        joined_fix.push_str(
            ", with the struct each of its newtype variants holds and each member it flattens",
        );
    }
    joined_fix
}

/// What a member that reads every key still left does with `beside`: the keys of another member,
/// or a key its holder leaves, by its class.
fn beside_reason(member: &Member<'_>, class: &Class, beside: &str, a_member: bool) -> String {
    let ty = &member.written;
    match class {
        Class::Absorbing => format!("`{ty}` reads {beside} as entries of its map"),
        Class::Unseen(why) => format!(
            "tixschema cannot see which keys `{ty}` reads, as it is {why}, and it refuses \
             {beside} wherever it refuses a key it does not declare"
        ),
        Class::Refusing(_) | Class::Open | Class::Taker => {
            let order = if a_member { " in every order" } else { "" };
            format!("`{ty}` refuses a key it does not declare, so it refuses {beside}{order}")
        }
    }
}

/// The fix of a member that cannot read every key beside another, a clause.
fn beside_fix(member: &Member<'_>, class: &Class) -> String {
    match class {
        Class::Refusing(culprits) => nonstrict_fix(culprits),
        Class::Absorbing => format!("write `{}` under a key of its own", member.name),
        Class::Unseen(_) | Class::Open | Class::Taker => format!(
            "write `{}` under a key of its own, or flatten beside it only members serde takes \
             their own keys of: a struct with no `#[serde(flatten)]` member and no tag of its \
             own, or an enum under no tag or under a tag and a content key",
            member.name
        ),
    }
}

/// Why a member that reads every key still left cannot stand ahead of `later`, by its class.
fn ahead_reason(member: &Member<'_>, class: &Class, later: &str) -> String {
    let ty = &member.written;
    match class {
        Class::Absorbing => format!(
            "`{ty}` reads every key still left as an entry of its map, so it reads the keys of \
             {later} as its own"
        ),
        Class::Unseen(why) => format!(
            "tixschema cannot see which keys `{ty}` reads, as it is {why}, and one that reads \
             every key still left and refuses one it does not declare refuses the keys of {later}"
        ),
        Class::Refusing(_) | Class::Open | Class::Taker => format!(
            "`{ty}` reads every key still left and refuses one it does not declare, so serde \
             refuses the keys of {later} that it wrote itself"
        ),
    }
}

/// `member` as a refusal names it: its name, and the type it is written as.
fn named(member: &Member<'_>) -> String {
    format!("`{}` (`{}`)", member.name, member.written)
}

/// The refusal `verdict` earns `level` whose members are of `classes`, or `None` for a verdict
/// that reads back.
fn refusal_of(
    level: &Level<'_>,
    members: &[Member<'_>],
    classes: &[Class],
    verdict: Verdict,
) -> Option<Refusal> {
    let subject = &level.subject;
    match verdict {
        Verdict::Closable | Verdict::Open => None,
        Verdict::ClosedByHand(at) => Some(closed_refusal(level, members, classes, at)),
        Verdict::Reorder(at) => {
            let member = &members[at];
            let later = later_than(members, at);
            Some(Refusal {
                fix: format!(
                    "declare `{}` after every other flattened member",
                    member.name
                ),
                reason: format!(
                    "{subject} flattens {} ahead of {later}: {}",
                    named(member),
                    ahead_reason(member, &classes[at], &later)
                ),
            })
        }
        Verdict::Nonstrict(at) | Verdict::ChangeShape(at) => {
            beside_refusal(level, members, classes, (at, verdict))
        }
    }
}

/// The refusal of `level`, written `#[serde(deny_unknown_fields)]` by its author or by the macro,
/// that flattens the member at `at`, which reads every key still left.
fn closed_refusal(
    level: &Level<'_>,
    members: &[Member<'_>],
    classes: &[Class],
    at: usize,
) -> Refusal {
    let member = &members[at];
    if level.closed_by_hand {
        return Refusal {
            fix: format!(
                "take `#[serde(deny_unknown_fields)]` off `{}`: tixschema leaves it off a type \
                 that flattens such a member",
                level.named
            ),
            reason: format!(
                "{} is written `#[serde(deny_unknown_fields)]` and flattens {}, whose keys serde \
                 leaves in the object it reads it from, so the type refuses them even with no \
                 other member beside it",
                level.subject,
                named(member)
            ),
        };
    }
    let mut fix = format!("mark `{}` `#[model_schema(nonstrict)]`", level.named);
    if let Some(more) =
        verdict_of_open(level, classes).and_then(|held| refusal_of(level, members, classes, held))
    {
        fix.push_str(", and ");
        fix.push_str(&more.fix);
    }
    Refusal {
        fix,
        reason: format!(
            "{} flattens {}, {NOT_SEEN}: every member tixschema could see takes its own keys, so \
             it wrote `#[serde(deny_unknown_fields)]` on `{}`, and `{}` reads every key still \
             left, which `{}` then refuses",
            level.subject,
            named(member),
            level.named,
            member.written,
            level.named
        ),
    }
}

/// The refusal of the member at `at` of `level`, which `verdict` says reads every key still left
/// beside another member, or beside a key the holder leaves.
fn beside_refusal(
    level: &Level<'_>,
    members: &[Member<'_>],
    classes: &[Class],
    (at, verdict): (usize, Verdict),
) -> Option<Refusal> {
    let subject = &level.subject;
    let member = &members[at];
    let beside_member = classes
        .iter()
        .enumerate()
        .find(|(place, held)| *place != at && held.rest())
        .map(|(place, _)| named(&members[place]));
    let beside_member_named = beside_member.is_some();
    let (opening, beside) = match (beside_member, &level.leftover) {
        (Some(other), _) => (
            format!(
                "{subject} flattens {} and {other}, and each reads every key still left",
                named(member)
            ),
            format!("the keys of {other}"),
        ),
        (None, Some(leftover)) => (
            format!(
                "{subject} flattens {}, which reads every key still left, and has {leftover}, \
                 which serde leaves for it to read",
                named(member)
            ),
            leftover.clone(),
        ),
        (None, None) => return None,
    };
    let refusing: Vec<usize> = (0..classes.len())
        .filter(|&place| matches!(classes[place], Class::Refusing(_)))
        .collect();
    if matches!(verdict, Verdict::Nonstrict(_)) && refusing.len() > 1 {
        let types: Vec<&str> = refusing
            .iter()
            .map(|&place| members[place].written.as_str())
            .collect();
        let mut culprits: Vec<Culprit> = Vec::new();
        for &place in &refusing {
            if let Class::Refusing(held) = &classes[place] {
                for culprit in held {
                    if !culprits.contains(culprit) {
                        culprits.push(culprit.clone());
                    }
                }
            }
        }
        return Some(Refusal {
            fix: nonstrict_fix(&culprits),
            reason: format!(
                "{opening}: {} each refuse a key they do not declare, so each refuses the keys of \
                 the others in every order",
                listed(&types)
            ),
        });
    }
    Some(Refusal {
        fix: beside_fix(member, &classes[at]),
        reason: format!(
            "{opening}: {}",
            beside_reason(member, &classes[at], &beside, beside_member_named)
        ),
    })
}

/// The verdict `level` would earn open: where the macro closed it, the one marking it nonstrict
/// leaves.
fn verdict_of_open(level: &Level<'_>, classes: &[Class]) -> Option<Verdict> {
    let open = verdict(classes, false, level.leftover.is_some());
    matches!(
        open,
        Verdict::ChangeShape(_) | Verdict::Nonstrict(_) | Verdict::Reorder(_)
    )
    .then_some(open)
}

/// The sources of `own`, the keys of a level's own members: a key it reads is read ahead of every
/// flattened member, and one it only writes is left for them.
fn own_sources(own: &[(String, KeySet)]) -> Vec<Source> {
    own.iter()
        .map(|(label, held)| Source {
            keys: held.clone(),
            label: label.clone(),
            place: if held.read.is_empty() {
                Place::Left
            } else {
                Place::Own
            },
        })
        .collect()
}

/// Each source of keys at `level` whose members are `members` of `classes`, in the order serde
/// reads them.
fn sources(level: &Level<'_>, members: &[Member<'_>], classes: &[Class]) -> Vec<Source> {
    let mut found = own_sources(&level.own_keys);
    found.extend(members.iter().enumerate().map(|(at, member)| Source {
        keys: member.keys.clone(),
        label: format!("flattened `{}` (`{}`)", member.name, member.written),
        place: Place::Flattened {
            at,
            takes: !classes[at].rest(),
        },
    }));
    found
}

/// Whether serde hands `reader` the keys `writer` writes: a member of the holder's own reads first,
/// and a flattened member reads every key the holder leaves and every key a member that takes its
/// own keys has not taken ahead of it.
const fn sees(reader: &Source, writer: &Source) -> bool {
    if let Place::Flattened {
        at: write_at,
        takes,
    } = writer.place
    {
        if let Place::Flattened {
            at: read_at,
            takes: _reader_takes,
        } = reader.place
        {
            return read_at < write_at || !takes;
        }
        return matches!(reader.place, Place::Own);
    }
    matches!(writer.place, Place::Left) && reader.flattened_at().is_some()
}

/// Each key two sources of `level` collide on, with the place of the flattened member a refusal is
/// spanned on and what it says. A key two sources both write is written twice; a key one reads that
/// another writes, which serde hands it, is read by the wrong member.
fn collisions(subject: &str, found: &[Source]) -> Vec<(usize, String)> {
    let mut refused = Vec::new();
    for (writer_at, writer) in found.iter().enumerate() {
        for (reader_at, reader) in found.iter().enumerate() {
            let Some(spanned) = reader.flattened_at().max(writer.flattened_at()) else {
                continue;
            };
            if reader_at == writer_at {
                continue;
            }
            let twice = writer
                .keys
                .written
                .iter()
                .find(|key| reader.keys.written.contains(key));
            if let Some(key) = twice {
                if reader_at < writer_at {
                    refused.push((
                        spanned,
                        format!(
                            "{subject} has {} and {}, which both write the key `{key}`: serde \
                             writes it twice in one object, and a JSON object holds each key \
                             once, so every reader of the object keeps one of the two values. \
                             {}",
                            reader.label,
                            writer.label,
                            rename_fix(&[reader, writer])
                        ),
                    ));
                }
                continue;
            }
            let Some(key) = writer
                .keys
                .written
                .iter()
                .find(|key| reader.keys.read.contains(key))
                .filter(|_| sees(reader, writer))
            else {
                continue;
            };
            let both_take = matches!(
                (reader.place, writer.place),
                (
                    Place::Flattened { at: before, takes: true },
                    Place::Flattened { at: after, takes: true },
                ) if before < after
            );
            let fix = if let (true, Some(reader_name)) = (both_take, member_name(&reader.label)) {
                format!(
                    "Declare `{reader_name}` after every member that writes `{key}`, or rename \
                     the key in one of them."
                )
            } else {
                rename_fix(&[reader, writer])
            };
            refused.push((
                spanned,
                format!(
                    "{subject} has {} and {}: {} reads the key `{key}`, which {} writes, and serde \
                     hands it to {}, so {} reads back without it. {fix}",
                    reader.label,
                    writer.label,
                    reader.label,
                    writer.label,
                    reader.label,
                    writer.label
                ),
            ));
        }
    }
    refused
}

/// The fix of a key two of `sources` declare: rename it, or write a flattened one of them under a
/// key of its own.
fn rename_fix(sources: &[&Source]) -> String {
    let flattened: Vec<&str> = sources
        .iter()
        .filter_map(|source| member_name(&source.label))
        .collect();
    let mut names: Vec<String> = Vec::new();
    for name in flattened {
        let quoted = format!("`{name}`");
        if !names.contains(&quoted) {
            names.push(quoted);
        }
    }
    match names.as_slice() {
        [] => "Rename the key in one of them.".to_owned(),
        [only] => format!("Rename the key in one of them, or write {only} under a key of its own."),
        [first, .., last] => format!(
            "Rename the key in one of them, or write {first} or {last} under a key of its own."
        ),
    }
}

/// The member a source label names, for a flattened member.
fn member_name(label: &str) -> Option<&str> {
    label.strip_prefix("flattened `")?.split('`').next()
}

/// The refusal of each flattened `Option` of `members` that serde reads back as `Some` from an
/// object that holds none of its keys, where the expansion sees it does, or cannot see whether it
/// does as it holds a type parameter.
fn empty_options(level: &Level<'_>, members: &[Member<'_>]) -> Vec<(usize, String)> {
    members
        .iter()
        .enumerate()
        .filter_map(|(at, member)| {
            let optional = member.optional.as_ref()?;
            let held = &optional.written;
            let opening = format!(
                "{} flattens `{}` (`{}`)",
                level.subject, member.name, member.written
            );
            (optional.empty == Answer::Yes).then(|| {
                let reason = if flattens_a_map(member.field) {
                    format!(
                        "serde writes `None` as no key, and reads an object that holds no entry of \
                         `{held}` as `Some` of a map of no entry"
                    )
                } else {
                    format!(
                        "`{held}` reads from an object that holds none of its keys, so serde writes \
                         `None` as no key and reads it back as `Some`"
                    )
                };
                (
                    at,
                    format!("{opening}: {reason}. {}", empty_fix(member, held)),
                )
            })
        })
        .collect()
}

/// The refusal of each flattened `Option` of `members` whose type reads every key still left and
/// refuses one it does not declare: serde reads a flattened `Option` as `None` wherever what it
/// holds refuses the object it is handed, so a key no member declares turns it into `None`.
fn refusing_options(level: &Level<'_>, members: &[Member<'_>]) -> Vec<(usize, String)> {
    members
        .iter()
        .enumerate()
        .filter_map(|(at, member)| {
            let optional = member.optional.as_ref()?;
            let culprits = optional.refusing.as_ref()?;
            Some((
                at,
                refusing_option_message(level, member, &optional.written, culprits),
            ))
        })
        .collect()
}

/// The refusal of the flattened `Option` `member` of `level`, which holds `held`, a type that reads
/// every key still left and refuses one through `culprits`.
fn refusing_option_message(
    level: &Level<'_>,
    member: &Member<'_>,
    held: &str,
    culprits: &[Culprit],
) -> String {
    format!(
        "{} flattens {}: `{held}` reads every key still left and refuses one it does not \
         declare, and serde reads a flattened `Option` as `None` wherever what it holds refuses \
         the object it is handed, so a key no member declares turns the value into `None` rather \
         than being refused. {}",
        level.subject,
        named(member),
        sentence(&format!(
            "{}, flatten `{held}` without the `Option`, or write `{}` under a key of its own",
            nonstrict_fix(culprits),
            member.name
        ))
    )
}

/// Whether `field` flattens an `Option` of a map.
fn flattens_a_map(field: &Field) -> bool {
    !read_by_a_function(field)
        && wrapped(&field.ty, &["Option"]).is_some_and(|inner| {
            matches!(
                get_field_def("", inner, "").field_type,
                FieldDefType::Map(_, _)
            )
        })
}

/// The fix of a flattened `Option` of `held` that reads from an object that holds none of its keys.
fn empty_fix(member: &Member<'_>, held: &str) -> String {
    if flattens_a_map(member.field) {
        format!(
            "Flatten the map without the `Option`, which serde reads as a map of no entry where \
             none is, or write `{}` under a key of its own.",
            member.name
        )
    } else {
        format!(
            "Give `{held}` a member serde requires, flatten `{held}` without the `Option`, or \
             write `{}` under a key of its own.",
            member.name
        )
    }
}

/// The refusals a holder built with a Dart or a Swift read earns, which those reads write when the
/// holder expands and so hand a member every key the expansion cannot see is taken: a member that
/// refuses a key it does not declare flattened after one whose type the expansion has not met, and
/// a member whose type holds one the expansion had not met when it expanded that type, beside
/// another key.
#[cfg(any(feature = "dart", feature = "swift"))]
fn unmet_placements(level: &Level<'_>, members: &[Member<'_>]) -> Vec<(usize, String)> {
    let mut refused = Vec::new();
    let unmet_ahead = |at: usize| {
        members
            .iter()
            .take(at)
            .find(|member| member.dynamic.is_some())
    };
    for (at, member) in members.iter().enumerate() {
        if let (Class::Refusing(_), Some(ahead)) = (&member.class, unmet_ahead(at)) {
            refused.push((
                at,
                format!(
                    "{} flattens {} after {}, {NOT_SEEN}: the Dart and Swift reads of `{}` are \
                     written when it expands, before they can see which keys `{}` takes, and \
                     hand `{}`, which refuses a key it does not declare, the keys of `{}` that \
                     serde takes off the object. Declare `{}` above `{}`, or write `{}` under a \
                     key of its own.",
                    level.subject,
                    named(member),
                    named(ahead),
                    level.named,
                    ahead.written,
                    member.written,
                    ahead.name,
                    ahead.written,
                    level.named,
                    ahead.name
                ),
            ));
        }
        let beside_a_key = members.len() > 1 || !level.own_keys.is_empty();
        let blind = member.blind.as_ref().filter(|found| {
            beside_a_key
                && (found.held == Held::Flattened
                    || (found.held == Held::UntaggedPayload && cfg!(feature = "swift")))
        });
        if let Some(found) = blind {
            let (surfaces, were, they, hand) = if found.held == Held::Flattened {
                ("the Dart and Swift reads", "were", "they", "hand")
            } else {
                ("the Swift read", "was", "it", "hands")
            };
            refused.push((
                at,
                format!(
                    "{} flattens {} beside other keys, and `{}` holds `{}`, which was declared \
                     below `{}`, in another crate, or with no `#[model_schema]`: {surfaces} of \
                     `{}` {were} written before {they} could see which keys `{}` reads, and {hand} it \
                     the keys beside `{}` that serde does not. Declare `{}` above `{}`, or write \
                     `{}` under a key of its own.",
                    level.subject,
                    named(member),
                    member.written,
                    found.name,
                    member.written,
                    member.written,
                    found.name,
                    member.name,
                    found.name,
                    member.written,
                    member.name
                ),
            ));
        }
    }
    refused
}

/// Nothing, in a build with no Dart and no Swift read.
#[cfg(not(any(feature = "dart", feature = "swift")))]
const fn unmet_placements(_level: &Level<'_>, _members: &[Member<'_>]) -> Vec<(usize, String)> {
    Vec::new()
}

/// The check a holder built with a Dart or a Swift read earns for a member at `at` whose type the
/// expansion has not met: where its hidden constant says it refuses a key it does not declare, it
/// reads back on those surfaces only last, beside members whose types were declared above and take
/// their own keys.
#[cfg(any(feature = "dart", feature = "swift"))]
fn unmet_refusing(
    level: &Level<'_>,
    members: &[Member<'_>],
    at: usize,
    flags: &TokenStream,
) -> Option<(TokenStream, String)> {
    let member = &members[at];
    let others: Vec<&str> = members
        .iter()
        .enumerate()
        .filter(|(place, _)| *place != at)
        .map(|(_, other)| other.name.as_str())
        .collect();
    let after_takers = members
        .iter()
        .enumerate()
        .all(|(place, other)| place == at || (other.dynamic.is_none() && !other.class.rest()));
    let last = at.saturating_add(1) == members.len();
    if others.is_empty() || (last && after_takers) {
        return None;
    }
    let mut fix = format!(
        "declare `{}` above `{}`, mark it `#[model_schema(nonstrict)]`",
        member.written, level.named
    );
    if after_takers {
        let _ = write!(
            fix,
            ", declare `{}` after every other flattened member",
            member.name
        );
    }
    let _ = write!(fix, ", or write `{}` under a key of its own", member.name);
    Some((
        quote! { #flags & #REFUSES != 0 },
        format!(
            "{} flattens {}, {NOT_SEEN}, which refuses a key it does not declare, beside {}: \
             the Dart and Swift reads of `{}` are written when it expands, before they can see \
             which keys `{}` takes, and hand it keys of the members beside it that serde does \
             not. {}",
            level.subject,
            named(member),
            listed(&others),
            level.named,
            member.written,
            sentence(&fix)
        ),
    ))
}

/// Nothing, in a build with no Dart and no Swift read.
#[cfg(not(any(feature = "dart", feature = "swift")))]
const fn unmet_refusing(
    _level: &Level<'_>,
    _members: &[Member<'_>],
    _at: usize,
    _flags: &TokenStream,
) -> Option<(TokenStream, String)> {
    None
}

/// The class each code of [`class_code`] stands for, for the member `member`.
fn class_coded(code: u8, member: &Member<'_>) -> Class {
    match code {
        0 => Class::Taker,
        1 => Class::Open,
        2 => Class::Refusing(vec![Culprit {
            by_hand: false,
            holds_more: true,
            name: spelled(peeled(&member.field.ty)),
        }]),
        3 => Class::Absorbing,
        _ => Class::Unseen(NOT_SEEN.to_owned()),
    }
}

/// The class a hidden constant `flags` stands for, as a code [`class_coded`] reads.
fn class_code(flags: &TokenStream) -> TokenStream {
    quote! {
        (if #flags & #REST == 0 {
            0_u8
        } else if #flags & #ABSORBS != 0 {
            3_u8
        } else if #flags & #UNSEEN != 0 {
            4_u8
        } else if #flags & #REFUSES != 0 {
            2_u8
        } else {
            1_u8
        })
    }
}

/// Every assignment of a class code to each of `count` members.
fn assignments(count: usize) -> Vec<Vec<u8>> {
    let mut all: Vec<Vec<u8>> = vec![Vec::new()];
    for _ in 0..count {
        all = all
            .into_iter()
            .flat_map(|held| {
                (0..CODES).map(move |code| {
                    let mut longer = held.clone();
                    longer.push(code);
                    longer
                })
            })
            .collect();
    }
    all
}

/// The classes of `members`, each member whose hidden constant answers for it at the class
/// `codes` assigns it, in order.
fn assigned(members: &[Member<'_>], codes: &[u8]) -> Vec<Class> {
    let mut answered = codes.iter();
    members
        .iter()
        .map(|member| {
            if member.dynamic.is_some() {
                answered
                    .next()
                    .map_or(Class::Taker, |&code| class_coded(code, member))
            } else {
                member.class.clone()
            }
        })
        .collect()
}

/// The order and strictness refusal `level` earns with its members of `classes`.
fn order_refusal(
    level: &Level<'_>,
    members: &[Member<'_>],
    classes: &[Class],
) -> Option<(usize, String)> {
    let closed = level.closed_by_hand || level.closed_by_macro;
    let held = verdict(classes, closed, level.leftover.is_some());
    let named = match held {
        Verdict::ChangeShape(at)
        | Verdict::ClosedByHand(at)
        | Verdict::Nonstrict(at)
        | Verdict::Reorder(at) => at,
        Verdict::Closable | Verdict::Open => return None,
    };
    refusal_of(level, members, classes, held).map(|refusal| (named, refusal.message()))
}

/// The refusals of `level` the expansion answers itself: each key two sources collide on, each
/// flattened `Option` that reads back as `Some`, a member it has not met placed where the Dart and
/// Swift reads cannot read it, and the order and strictness refusal where no member's hidden
/// constant answers for it, or where every class the constants could answer earns the same one.
fn level_refusals(level: &Level<'_>, members: &[Member<'_>]) -> Vec<(usize, String)> {
    let classes: Vec<Class> = members.iter().map(|member| member.class.clone()).collect();
    let mut refused = collisions(&level.subject, &sources(level, members, &classes));
    refused.extend(empty_options(level, members));
    refused.extend(refusing_options(level, members));
    refused.extend(unmet_placements(level, members));
    let dynamic = members
        .iter()
        .filter(|member| member.dynamic.is_some())
        .count();
    let mut found = assignments(dynamic)
        .into_iter()
        .map(|codes| order_refusal(level, members, &assigned(members, &codes)));
    let first = found.next().flatten();
    if let Some(certain) = first.filter(|held| found.all(|other| other.as_ref() == Some(held))) {
        refused.push(certain);
    }
    refused
}

/// The refusal of a variant of the untagged `item_enum` that reads every object serde writes for a
/// variant declared after it: the variant reads from an object that holds none of its keys, and
/// refuses no key it does not declare.
fn untagged_shadow_errors(item_enum: &ItemEnum, args: &ModelSchemaArgs) -> Vec<TokenStream> {
    if !matches!(Tagging::of(item_enum), Tagging::Untagged) {
        return Vec::new();
    }
    let parameters = parameter_names(&item_enum.generics);
    let closed = strict::closed_by_hand(&item_enum.attrs) || strict::closes_enum(item_enum, args);
    let defaulted = derive_reads_a_default(&item_enum.attrs);
    let variants: Vec<&Variant> = item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
        .collect();
    let reading: Vec<usize> = (0..variants.len())
        .filter(|&at| reads_any_object(variants[at], closed, defaulted, &parameters))
        .collect();
    let Some(&first) = reading.first() else {
        return Vec::new();
    };
    let shadowed: Vec<&Variant> = variants
        .iter()
        .skip(first.saturating_add(1))
        .copied()
        .filter(|variant| writes_an_object(variant, &parameters))
        .collect();
    if shadowed.is_empty() {
        return Vec::new();
    }
    let enum_name = item_enum.ident.unraw();
    let variant_name = variants[first].ident.unraw();
    let later: Vec<String> = shadowed
        .iter()
        .map(|variant| variant.ident.unraw().to_string())
        .collect();
    let later_names: Vec<&str> = later.iter().map(String::as_str).collect();
    let opening = format!(
        "type `{enum_name}`: variant `{variant_name}` of the untagged `{enum_name}` reads every \
         object, as every member it flattens reads from an object that holds none of its keys, \
         and it refuses no key it does not declare"
    );
    let message = if let [_, second, ..] = reading.as_slice() {
        format!(
            "{opening}, and so does variant `{}`, so serde reads every value of the later one as \
             `{variant_name}`. Give one of them a member serde requires.",
            variants[*second].ident.unraw()
        )
    } else {
        let declared_after = if later_names.len() == 1 {
            format!("variant {}", listed(&later_names))
        } else {
            format!("variants {}", listed(&later_names))
        };
        format!(
            "{opening}, so serde never reads {declared_after}, declared after it, which serde \
             writes as an object. Declare `{variant_name}` after every other variant."
        )
    };
    vec![
        syn::Error::new_spanned(variants[first], prefixed_guard_message(&message))
            .to_compile_error(),
    ]
}

/// Whether `variant` of an untagged enum `closed` or not reads every object.
fn reads_any_object(
    variant: &Variant,
    closed: bool,
    defaulted: bool,
    parameters: &[String],
) -> bool {
    if !matches!(variant.fields, Fields::Named(_)) || !variant.fields.iter().any(flattened) {
        return false;
    }
    let mut absorbs = false;
    for field in &variant.fields {
        if !read_by_serde(field) {
            continue;
        }
        if !parse_serde_field_attributes(&field.attrs).flatten {
            if !absent_reads(field, defaulted, parameters) {
                return false;
            }
            continue;
        }
        if read_by_a_function(field) {
            return false;
        }
        let def = def_of(&field.ty, parameters);
        let class = reads(&def, 0).class();
        absorbs |= matches!(class, Class::Absorbing);
        let reads_past = def.is_optional()
            || (matches!(class, Class::Taker | Class::Open | Class::Absorbing)
                && empty_of(&def, 0) == Answer::Yes);
        if !reads_past {
            return false;
        }
    }
    !closed || absorbs
}

/// Whether serde can write `variant` of an untagged enum as an object: a variant of named members,
/// or one that holds a type serde writes as one, where the expansion sees it does not.
fn writes_an_object(variant: &Variant, parameters: &[String]) -> bool {
    let Some(slot) = newtype_slot(variant) else {
        return matches!(variant.fields, Fields::Named(_));
    };
    let def = def_of(&slot.ty, parameters);
    def.array_depth == 0 && !reads(&def, 0).no_object
}

/// The refusal of each filling, in a member `item` writes or in one of its `default_types`, of a
/// type parameter a type recorded above flattens, where the filled type does not read back: a key
/// it writes that another member of the level writes or reads, and an `Option` it fills that reads
/// back as `Some`.
fn filling_errors(item: &Item, args: &ModelSchemaArgs) -> Vec<TokenStream> {
    let (name, generics, fields): (String, &Generics, Vec<&Field>) =
        if let Item::Struct(item_struct) = item {
            (
                item_struct.ident.to_string(),
                &item_struct.generics,
                item_struct.fields.iter().collect(),
            )
        } else if let Item::Enum(item_enum) = item {
            (
                item_enum.ident.to_string(),
                &item_enum.generics,
                item_enum
                    .variants
                    .iter()
                    .flat_map(|variant| variant.fields.iter())
                    .collect(),
            )
        } else {
            return Vec::new();
        };
    let parameters = parameter_names(generics);
    let mut refused = Vec::new();
    for field in fields {
        let mut found = Vec::new();
        fillings_in(&field.ty, &mut found);
        let member = field
            .ident
            .as_ref()
            .map_or_else(String::new, |ident| format!("field `{}`: ", ident.unraw()));
        for (spanned, holder, arguments) in found {
            let filling: Vec<FieldDef> = arguments
                .iter()
                .map(|argument| def_of(argument, &parameters))
                .collect();
            for message in filled_refusals(&holder, &filling) {
                refused.push(
                    syn::Error::new_spanned(
                        spanned,
                        prefixed_guard_message(&format!(
                            "type `{name}`: {member}`{}` fills `{holder}`, {message}",
                            spelled(spanned)
                        )),
                    )
                    .to_compile_error(),
                );
            }
        }
    }
    for (entry, filling) in &args.default_types {
        let Some(at) = parameters.iter().position(|parameter| entry == parameter) else {
            continue;
        };
        let mut arguments: Vec<FieldDef> = parameters
            .iter()
            .map(|parameter| {
                let mut def = get_field_def("", &syn::parse_quote! { u8 }, "");
                def.field_type = FieldDefType::TypeParam(parameter.clone());
                def
            })
            .collect();
        arguments[at] = def_of(filling, &parameters);
        let own = if let Item::Struct(item_struct) = item {
            struct_record(item_struct, args)
        } else if let Item::Enum(item_enum) = item {
            enum_record(item_enum, args)
        } else {
            continue;
        };
        let Record::Shape(shape) = own else {
            continue;
        };
        for message in filled_refusals_of(&shape, &arguments) {
            refused.push(
                syn::Error::new_spanned(
                    filling,
                    prefixed_guard_message(&format!(
                        "type `{name}`: `default_types` entry `{entry}`: `{}` fills `{name}`, \
                         {message}",
                        spelled(filling)
                    )),
                )
                .to_compile_error(),
            );
        }
    }
    refused
}

/// Every type written in `ty` with type arguments, each beside the name of the type and the
/// arguments: where it names a recorded type that flattens one of its parameters.
fn fillings_in<'ty>(ty: &'ty Type, found: &mut Vec<(&'ty Type, String, Vec<&'ty Type>)>) {
    let inner = if let Type::Paren(parenthesized) = ty {
        Some(&*parenthesized.elem)
    } else if let Type::Group(grouped) = ty {
        Some(&*grouped.elem)
    } else if let Type::Array(array) = ty {
        Some(&*array.elem)
    } else if let Type::Slice(slice) = ty {
        Some(&*slice.elem)
    } else if let Type::Reference(reference) = ty {
        Some(&*reference.elem)
    } else {
        None
    };
    if let Some(held) = inner {
        return fillings_in(held, found);
    }
    if let Type::Tuple(tuple) = ty {
        for held in &tuple.elems {
            fillings_in(held, found);
        }
        return;
    }
    let Type::Path(path) = ty else {
        return;
    };
    let Some(last) = path.path.segments.last() else {
        return;
    };
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return;
    };
    let held: Vec<&Type> = arguments
        .args
        .iter()
        .filter_map(|argument| {
            if let syn::GenericArgument::Type(argued) = argument {
                Some(argued)
            } else {
                None
            }
        })
        .collect();
    for argued in &held {
        fillings_in(argued, found);
    }
    let named = last.ident.unraw().to_string();
    let flattens_a_parameter = matches!(
        recorded(&named),
        Some(Record::Shape(shape)) if !shape.parameters.is_empty() && !shape.levels.is_empty()
    );
    if flattens_a_parameter {
        found.push((ty, named, held));
    }
}

/// What a filling of the recorded type `holder` with `arguments` earns, each the rest of a refusal
/// after the type it names.
fn filled_refusals(holder: &str, arguments: &[FieldDef]) -> Vec<String> {
    if let Some(Record::Shape(shape)) = recorded(holder) {
        filled_refusals_of(&shape, arguments)
    } else {
        Vec::new()
    }
}

/// What a filling of the type recorded as `shape` with `arguments` earns, as
/// [`filled_refusals`] answers it.
fn filled_refusals_of(shape: &Shape, arguments: &[FieldDef]) -> Vec<String> {
    let mut refused = Vec::new();
    for level in &shape.levels {
        let reaches = level
            .flattened
            .iter()
            .any(|(_, def)| names_a_parameter(def, &shape.parameters));
        if !reaches {
            continue;
        }
        let subject = level.variant.as_ref().map_or_else(
            || "which then".to_owned(),
            |variant| format!("whose variant `{variant}` then"),
        );
        let members: Vec<(String, FieldDef)> = level
            .flattened
            .iter()
            .map(|(name, def)| (name.clone(), filled(def, &shape.parameters, arguments)))
            .collect();
        let mut found = own_sources(&sources_of(level.own.clone()));
        found.extend(members.iter().enumerate().map(|(at, (name, def))| Source {
            keys: keys(def, 0),
            label: format!("flattened `{name}`"),
            place: Place::Flattened {
                at,
                takes: !reads(def, 0).rest,
            },
        }));
        for (_, message) in collisions(&subject, &found) {
            let filling = "fill it with a type that writes no key another of its members declares";
            refused.push(if message.ends_with("Rename the key in one of them.") {
                message.replace(
                    "Rename the key in one of them.",
                    &format!("Rename the key in one of them, or {filling}."),
                )
            } else {
                message.replace(
                    "Rename the key in one of them, or write",
                    &format!("Rename the key in one of them, {filling}, or write"),
                )
            });
        }
        for (name, def) in &members {
            if def.array_depth == 0
                && def.is_optional()
                && empty_of(&under_option(def), 0) == Answer::Yes
            {
                refused.push(format!(
                    "{subject} flattens `{name}` as an `Option` of a type that reads from an \
                     object that holds none of its keys, so serde writes `None` as no key and \
                     reads it back as `Some`. Fill it with a type serde requires a member of."
                ));
            }
        }
    }
    refused
}

/// Whether `def` names one of `parameters`, at any depth.
fn names_a_parameter(def: &FieldDef, parameters: &[String]) -> bool {
    if let FieldDefType::TypeParam(name) = &def.field_type {
        return parameters.contains(name);
    }
    if let FieldDefType::SiblingType(_, arguments) = &def.field_type {
        return arguments
            .iter()
            .any(|argument| names_a_parameter(argument, parameters));
    }
    false
}

/// The check of the dynamic member `member` of `level` that serde writes it as no object.
fn no_object_check(
    level: &Level<'_>,
    member: &Member<'_>,
    flags: &TokenStream,
) -> (TokenStream, String) {
    let held = spelled(peeled(&member.field.ty));
    (
        quote! { #flags & #NO_OBJECT != 0 },
        format!(
            "{} flattens `{}` (`{}`), {NOT_SEEN}: serde writes `{held}` as no object, which it \
             cannot flatten into `{}`. Write `{}` under a key of its own.",
            level.subject, member.name, member.written, level.named, member.name
        ),
    )
}

/// The checks of the dynamic member at `at` of `level` that refuse it whatever its order: a map
/// beside another key, a value serde writes as a plain enum's variant name, a map keyed by a number
/// or a `bool` it holds, and a value serde writes as an `Option` whose `None` reads back as `Some`.
fn dynamic_shape_checks(
    level: &Level<'_>,
    members: &[Member<'_>],
    at: usize,
    flags: &TokenStream,
) -> Vec<(TokenStream, String)> {
    let member = &members[at];
    let opening = format!(
        "{} flattens `{}` (`{}`), {NOT_SEEN}",
        level.subject, member.name, member.written
    );
    let held = spelled(peeled(&member.field.ty));
    let mut checks = Vec::new();
    let mut beside: Vec<String> = level
        .own_keys
        .iter()
        .map(|(label, _)| label.clone())
        .collect();
    beside.extend(
        members
            .iter()
            .enumerate()
            .filter(|(place, _)| *place != at)
            .map(|(_, other)| format!("flattened {}", named(other))),
    );
    if let Some(first) = beside.first() {
        checks.push((
            quote! { #flags & #ABSORBS != 0 },
            format!(
                "{opening}: `{held}` reads every key it is handed as an entry of its map, and \
                 serde writes the map's entries at one level with {first}, so an entry under one \
                 of their keys is written twice and their keys are read as entries. Write `{}` \
                 under a key of its own.",
                member.name
            ),
        ));
    }
    checks.push((
        quote! { #flags & #PLAIN != 0 },
        format!(
            "{opening}: serde writes `{held}` as a plain enum's variant name, which it writes \
             flattened as a key over `null` that no other surface reads as `{held}`. Write `{}` \
             under a key of its own.",
            member.name
        ),
    ));
    checks.push((
        quote! { #flags & #TEXT_KEYS != 0 },
        format!(
            "{opening}: `{held}` holds a map keyed by a number or a `bool`: {} Key the map by a \
             `String`, or write `{}` under a key of its own.",
            super::ranges::KEYS_BELOW_A_FLATTEN,
            member.name
        ),
    ));
    checks.push((
        quote! { #flags & #OPTION_EMPTY != 0 },
        format!(
            "{opening}: serde writes `{held}` as an `Option` of a value that reads from an object \
             that holds none of its keys, so it writes `None` flattened as no key and reads it \
             back as `Some`. Write `{}` under a key of its own.",
            member.name
        ),
    ));
    checks
}

/// The checks of the dynamic member at `at` of `level` that do not turn on its class: an
/// externally tagged enum with a unit variant, which the reads of the holder cannot read without
/// seeing it, a member that refuses a key the Dart and Swift reads hand it, and an `Option` of it
/// that reads back as `Some`, or as `None` beside a key no member declares.
fn dynamic_member_checks(
    level: &Level<'_>,
    members: &[Member<'_>],
    at: usize,
    flags: &TokenStream,
) -> Vec<(TokenStream, String)> {
    let member = &members[at];
    let opening = format!(
        "{} flattens `{}` (`{}`), {NOT_SEEN}",
        level.subject, member.name, member.written
    );
    let held = spelled(peeled(&member.field.ty));
    let mut checks = vec![(
        quote! { #flags & #ENTRY != 0 },
        format!(
            "{opening}: `{held}` is an externally tagged enum with a unit variant, which serde \
             writes flattened as an entry of the variant's name over `null`, and the Zod, Dart \
             and Swift reads of `{}` read that entry only for an enum declared above it. Declare \
             `{held}` above `{}`, give each unit variant of it a field, or write `{}` under a key \
             of its own.",
            level.named, level.named, member.name
        ),
    )];
    checks.extend(unmet_refusing(level, members, at, flags));
    if member.optional.is_some() {
        checks.push((
            quote! { #flags & #EMPTY != 0 },
            format!(
                "{opening}: `{held}` reads from an object that holds none of its keys, or \
                 tixschema cannot see that it does not, so serde writes `None` as no key and \
                 reads it back as `Some`. {}",
                empty_fix(member, &held)
            ),
        ));
        let code = class_code(flags);
        let culprits = vec![Culprit {
            by_hand: false,
            holds_more: true,
            name: held.clone(),
        }];
        checks.push((
            quote! { #code == 2_u8 },
            refusing_option_message(level, member, &held, &culprits),
        ));
    }
    checks
}

/// The labels and the keys of each source of keys at `level` whose members are `members`, each
/// the tokens of a `&[&str]` a `const` reads, and whether the expansion could not see every key of
/// it, which only the compiler then compares.
fn key_sources(level: &Level<'_>, members: &[Member<'_>]) -> Vec<(String, TokenStream, bool)> {
    let literal = |keys: &KeySet| {
        let mut all = keys.read.clone();
        for key in &keys.written {
            if !all.contains(key) {
                all.push(key.clone());
            }
        }
        quote! { &[#(#all),*] }
    };
    let mut found: Vec<(String, TokenStream, bool)> = level
        .own_keys
        .iter()
        .map(|(label, keys)| (label.clone(), literal(keys), false))
        .collect();
    found.extend(members.iter().map(|member| {
        let label = format!("flattened {}", named(member));
        member.dynamic_keys.clone().map_or_else(
            || (label.clone(), literal(&member.keys), false),
            |keys| (label.clone(), keys, true),
        )
    }));
    found
}

/// The check that two sources of keys at `level`, `first` and `second`, share no key, where the
/// compiler alone sees the keys of one of them: it names the key it finds.
fn shared_key_check(
    level: &Level<'_>,
    first: &(String, TokenStream, bool),
    second: &(String, TokenStream, bool),
    span: Span,
) -> TokenStream {
    let flattened: Vec<String> = [&first.0, &second.0]
        .iter()
        .filter_map(|label| member_name(label))
        .map(|name| format!("`{name}`"))
        .collect();
    let written = match flattened.as_slice() {
        [only] => only.clone(),
        [one, other] if one == other => one.clone(),
        [one, other] => format!("{one} or {other}"),
        _ => "one of them".to_owned(),
    };
    let ahead = prefixed_guard_message(&format!(
        "{} has {} and {}, {NOT_SEEN_ONE}, which both declare the key `",
        level.subject, first.0, second.0
    ));
    let after = format!(
        "`: serde writes it for each of them into one object, which holds a key once, so one of \
         the two does not read back. Rename the key in one of them, or write {written} under a \
         key of its own."
    );
    let (one, other) = (&first.1, &second.1);
    quote_spanned! { span=>
        {
            const TIXSCHEMA_SHARED: ::core::option::Option<&str> = tixschema_shared_key(#one, #other);
            const TIXSCHEMA_KEY: &str = match TIXSCHEMA_SHARED {
                ::core::option::Option::Some(key) => key,
                ::core::option::Option::None => "",
            };
            const TIXSCHEMA_AHEAD: &str = #ahead;
            const TIXSCHEMA_AFTER: &str = #after;
            const TIXSCHEMA_LENGTH: usize =
                TIXSCHEMA_AHEAD.len() + TIXSCHEMA_KEY.len() + TIXSCHEMA_AFTER.len();
            const TIXSCHEMA_BYTES: [u8; TIXSCHEMA_LENGTH] =
                tixschema_joined([TIXSCHEMA_AHEAD, TIXSCHEMA_KEY, TIXSCHEMA_AFTER]);
            const TIXSCHEMA_MESSAGE: &str = match ::core::str::from_utf8(&TIXSCHEMA_BYTES) {
                ::core::result::Result::Ok(text) => text,
                ::core::result::Result::Err(_) => "",
            };
            ::core::assert!(TIXSCHEMA_SHARED.is_none(), "{}", TIXSCHEMA_MESSAGE);
        }
    }
}

/// The `const` functions the checks of [`shared_key_check`] call.
fn key_functions() -> TokenStream {
    quote! {
        const fn tixschema_same_key(one: &str, other: &str) -> bool {
            let (left, right) = (one.as_bytes(), other.as_bytes());
            if left.len() != right.len() {
                return false;
            }
            let mut at = 0;
            while at < left.len() {
                if left[at] != right[at] {
                    return false;
                }
                at += 1;
            }
            true
        }
        const fn tixschema_shared_key(
            one: &[&'static str],
            other: &[&'static str],
        ) -> ::core::option::Option<&'static str> {
            let mut at = 0;
            while at < one.len() {
                let mut beside = 0;
                while beside < other.len() {
                    if tixschema_same_key(one[at], other[beside]) {
                        return ::core::option::Option::Some(one[at]);
                    }
                    beside += 1;
                }
                at += 1;
            }
            ::core::option::Option::None
        }
        const fn tixschema_joined<const LENGTH: usize>(parts: [&str; 3]) -> [u8; LENGTH] {
            let mut joined = [0_u8; LENGTH];
            let (mut at, mut part) = (0, 0);
            while part < parts.len() {
                let bytes = parts[part].as_bytes();
                let mut byte = 0;
                while byte < bytes.len() {
                    joined[at] = bytes[byte];
                    at += 1;
                    byte += 1;
                }
                part += 1;
            }
            joined
        }
    }
}

/// The trait whose constants a type no `#[model_schema]` expanded answers, named `fallback`: the
/// strictest bits, and no key.
fn fallback_trait(fallback: &Ident) -> TokenStream {
    let constant = proc_macro2::Ident::new(FLAGS, Span::call_site());
    let keys = proc_macro2::Ident::new(KEYS, Span::call_site());
    quote! {
        trait #fallback {
            const #constant: u16 = #NOT_MET;
            const #keys: &'static [&'static str] = &[];
        }
        impl<T: ?::core::marker::Sized> #fallback for T {}
    }
}

/// The `const` a holder earns for its members whose hidden constants answer for them, or nothing
/// where it has none: one assertion per class the constants could answer that earns a refusal,
/// and one per two sources of keys whose keys only the compiler sees.
fn dynamic_checks(level: &Level<'_>, members: &[Member<'_>]) -> Option<TokenStream> {
    let dynamic: Vec<(usize, &TokenStream)> = members
        .iter()
        .enumerate()
        .filter_map(|(at, member)| member.dynamic.as_ref().map(|flags| (at, flags)))
        .collect();
    let sources = key_sources(level, members);
    let mut key_checks = Vec::new();
    for (at, first) in sources.iter().enumerate() {
        for second in sources.iter().skip(at.saturating_add(1)) {
            if !(first.2 || second.2) {
                continue;
            }
            let spanned = members
                .iter()
                .find(|member| member.dynamic_keys.is_some())
                .map_or_else(Span::call_site, |member| member.field.ty.span());
            key_checks.push(shared_key_check(level, first, second, spanned));
        }
    }
    if dynamic.is_empty() && key_checks.is_empty() {
        return None;
    }
    // A member serde writes as no object is refused first, then one whose shape no order reads
    // back, its order and strictness next: a fix of one leaves the checks after it to say what else
    // the holder needs.
    let mut found: Vec<(TokenStream, String)> = dynamic
        .iter()
        .map(|&(at, flags)| no_object_check(level, &members[at], flags))
        .collect();
    for &(at, flags) in &dynamic {
        found.extend(dynamic_shape_checks(level, members, at, flags));
    }
    let codes: Vec<TokenStream> = dynamic.iter().map(|(_, flags)| class_code(flags)).collect();
    for assignment in assignments(dynamic.len()) {
        let Some((_, message)) = order_refusal(level, members, &assigned(members, &assignment))
        else {
            continue;
        };
        let condition = codes
            .iter()
            .zip(&assignment)
            .map(|(code, wanted)| quote! { #code == #wanted })
            .fold(quote! { true }, |held, more| quote! { #held && #more });
        found.push((condition, message));
    }
    for &(at, flags) in &dynamic {
        found.extend(dynamic_member_checks(level, members, at, flags));
    }
    let mut merged: Vec<(TokenStream, String)> = Vec::new();
    for (condition, message) in found {
        if let Some((held, _)) = merged.iter_mut().find(|(_, said)| *said == message) {
            *held = quote! { #held || (#condition) };
        } else {
            merged.push((quote! { (#condition) }, message));
        }
    }
    let span = dynamic.first().map_or_else(Span::call_site, |&(first, _)| {
        members[first].field.ty.span()
    });
    let checks = merged.iter().map(|(condition, message)| {
        // The message is a format string: a brace in it is written twice.
        let text = prefixed_guard_message(message)
            .replace('{', "{{")
            .replace('}', "}}");
        quote_spanned! { span=>
            ::core::assert!(!(#condition), #text);
        }
    });
    let written: Vec<String> = members
        .iter()
        .flat_map(|member| written_names(member.field.ty.to_token_stream()))
        .collect();
    let fallback = fallback_trait(&unclaimed_parameter("NotMet", &written));
    let functions = (!key_checks.is_empty()).then(key_functions);
    Some(quote! {
        const _: () = {
            #fallback
            #functions
            #(#key_checks)*
            #(#checks)*
        };
    })
}

/// Each level of `item` that flattens a member, with the members it flattens.
fn judged_levels<'item>(
    item: &'item Item,
    args: &ModelSchemaArgs,
) -> Vec<(Level<'item>, Vec<Member<'item>>)> {
    let (levels, parameters) = if let Item::Struct(item_struct) = item {
        (
            struct_level(item_struct, args)
                .into_iter()
                .collect::<Vec<_>>(),
            parameter_names(&item_struct.generics),
        )
    } else if let Item::Enum(item_enum) = item {
        (
            variant_levels(item_enum, args),
            parameter_names(&item_enum.generics),
        )
    } else {
        return Vec::new();
    };
    levels
        .into_iter()
        .filter_map(|level| {
            let members = members_of(&level, &parameters);
            (!members.is_empty()).then_some((level, members))
        })
        .collect()
}

/// The compile errors `item`'s flattened members earn, and those of the fillings it writes of a
/// type that flattens a type parameter.
pub(super) fn errors(item: &Item, args: &ModelSchemaArgs) -> Vec<TokenStream> {
    let mut refused: Vec<TokenStream> = judged_levels(item, args)
        .iter()
        .flat_map(|(level, members)| {
            level_refusals(level, members)
                .into_iter()
                .map(|(at, message)| {
                    syn::Error::new_spanned(members[at].field, prefixed_guard_message(&message))
                        .to_compile_error()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    if let Item::Enum(item_enum) = item {
        refused.extend(untagged_shadow_errors(item_enum, args));
        refused.extend(held_open_errors(item_enum, args));
    }
    refused.extend(filling_errors(item, args));
    refused
}

/// The refusal of `item_enum` where a variant flattens a member that reads every key still left,
/// which keeps the macro from writing `#[serde(deny_unknown_fields)]` on the enum, while another
/// level of it would refuse a key no member declares: the object under its tag and content key, a
/// struct variant whose flattened members all take their own keys, or a unit variant under a tag
/// alone. serde reads the attribute on the enum alone, for every level of it.
fn held_open_errors(item_enum: &ItemEnum, args: &ModelSchemaArgs) -> Vec<TokenStream> {
    if !strict::closable_but_for_flattening(item_enum, args) || strict::closes_enum(item_enum, args)
    {
        return Vec::new();
    }
    let parameters = parameter_names(&item_enum.generics);
    let on_the_wire: Vec<&Variant> = item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
        .collect();
    let opening = on_the_wire.iter().find_map(|variant| {
        if takes_only(&variant.fields, &item_enum.generics) {
            return None;
        }
        variant
            .fields
            .iter()
            .filter(|field| flattened(field))
            .find_map(|field| {
                let class = member_reads(field, &parameters).class();
                class.rest().then_some((*variant, field))
            })
    });
    let Some((held_open, field)) = opening else {
        return Vec::new();
    };
    let tagging = Tagging::of(item_enum);
    let where_read = if let Tagging::Adjacent { content, tag } = &tagging {
        Some(format!(
            "beside the tag `{tag}` and the content key `{content}`"
        ))
    } else {
        None
    }
    .or_else(|| {
        on_the_wire
            .iter()
            .find(|variant| {
                variant.ident != held_open.ident
                    && matches!(variant.fields, Fields::Named(_))
                    && takes_only(&variant.fields, &item_enum.generics)
            })
            .map(|variant| format!("in variant `{}`", variant.ident.unraw()))
    })
    .or_else(|| {
        matches!(tagging, Tagging::Internal { tag: _tag })
            .then(|| {
                on_the_wire
                    .iter()
                    .find(|variant| matches!(variant.fields, Fields::Unit))
                    .map(|variant| format!("beside the tag in variant `{}`", variant.ident.unraw()))
            })
            .flatten()
    });
    let Some(refusing) = where_read else {
        return Vec::new();
    };
    let enum_name = item_enum.ident.unraw();
    let variant = held_open.ident.unraw();
    let member = field
        .ident
        .as_ref()
        .map_or_else(String::new, |ident| ident.unraw().to_string());
    let message = format!(
        "type `{enum_name}`: variant `{variant}` flattens `{member}` (`{}`), which reads every key \
         still left, so tixschema cannot write `#[serde(deny_unknown_fields)]` on `{enum_name}` \
         without refusing the keys of `{member}` there, and serde reads that attribute for every \
         variant of an enum at once: it then reads past a key no member declares {refusing} as \
         well. Mark `{enum_name}` `#[model_schema(nonstrict)]`, or hold the members of `{variant}` \
         in a struct of their own and write `{variant}` as a newtype variant over it.",
        spelled(&field.ty)
    );
    vec![syn::Error::new_spanned(field, prefixed_guard_message(&message)).to_compile_error()]
}

/// The hidden constant `item` publishes, which a holder that cannot see it reads it by, and the
/// `const` checks of `item`'s own members whose hidden constant answers for them. `item` is
/// recorded already.
pub(super) fn tokens(item: &Item, args: &ModelSchemaArgs) -> TokenStream {
    let checks = judged_levels(item, args)
        .iter()
        .filter_map(|(level, members)| dynamic_checks(level, members))
        .collect::<Vec<_>>();
    let (name, generics) = if let Item::Struct(item_struct) = item {
        (&item_struct.ident, &item_struct.generics)
    } else if let Item::Enum(item_enum) = item {
        (&item_enum.ident, &item_enum.generics)
    } else {
        return quote! { #(#checks)* };
    };
    let value = flags_value(item, &name.to_string(), generics);
    let keys_value = keys_value(item, &name.to_string(), generics);
    let constant = proc_macro2::Ident::new(FLAGS, Span::call_site());
    let keys = proc_macro2::Ident::new(KEYS, Span::call_site());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let buffered = buffered_checks(item);
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #[doc = " What a holder that flattens this type reads of it: the bits `#[model_schema]` holds the order and the strictness of its flattened members to."]
            pub const #constant: u16 = #value;
            #[doc = " The keys a holder that flattens this type sees it write or read, which no other source of keys of that holder may declare."]
            pub const #keys: &'static [&'static str] = #keys_value;
        }

        #(#checks)*
        #buffered
    }
}

/// The bits a part of a type adds to its hidden constant, where the part's own constant answers
/// for them: every bit of a payload, which is read whole, and of a member that reads every key.
fn dynamic_part(named: &TokenStream, payload: bool) -> TokenStream {
    if payload {
        quote! { (#named & (#REFUSES | #ABSORBS | #UNSEEN | #NO_OBJECT)) }
    } else {
        quote! {
            (if #named & #REST != 0 { #named & (#REFUSES | #ABSORBS | #UNSEEN | #NO_OBJECT) } else { #named & #NO_OBJECT })
        }
    }
}

/// Whether the recorded type `name` reaches itself through a member it flattens or the payload of a
/// variant serde reads from every key still left.
fn reaches_itself(name: &str) -> bool {
    let parts = match recorded(name) {
        Some(Record::Shape(shape)) => shape
            .keyed
            .iter()
            .map(|held| filled(held, &shape.parameters, &[]))
            .collect(),
        Some(Record::Through { parameters, slot }) => vec![filled(&slot, &parameters, &[])],
        None => Vec::new(),
    };
    parts.iter().any(|part| reaches(part, name, 1))
}

/// Whether a value `def` describes is the type `target`, or holds it where [`reaches_itself`]
/// looks.
fn reaches(def: &FieldDef, target: &str, depth: usize) -> bool {
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return false;
    };
    if depth > DEPTH {
        return false;
    }
    if name == target {
        return true;
    }
    if let Some(Declared::Alias(aliased)) = declared(name) {
        return syn::parse_str::<Type>(&aliased).is_ok_and(|ty| {
            reaches(
                &get_field_def("flattened", &ty, ""),
                target,
                depth.saturating_add(1),
            )
        });
    }
    match recorded(name) {
        Some(Record::Through { parameters, slot }) => reaches(
            &filled(&slot, &parameters, arguments),
            target,
            depth.saturating_add(1),
        ),
        Some(Record::Shape(shape)) => shape.keyed.iter().any(|held| {
            reaches(
                &filled(held, &shape.parameters, arguments),
                target,
                depth.saturating_add(1),
            )
        }),
        None => false,
    }
}

/// The value of the hidden constant of the keys of `item`, named `name`: the keys the expansion sees,
/// with those of each part it could not see read from that part's own constant.
fn keys_value(item: &Item, name: &str, generics: &Generics) -> TokenStream {
    let parameters = parameter_names(generics);
    let mut own = get_field_def("own", &syn::parse_quote! { u8 }, "");
    own.field_type = FieldDefType::SiblingType(name.to_owned(), Vec::new());
    if reaches_itself(name) {
        // A type that flattens itself has no finite value, so no holder of it is written.
        return quote! { &[] };
    }
    let seen = keys(&own, 0);
    let mut all = seen.read.clone();
    for key in &seen.written {
        if !all.contains(key) {
            all.push(key.clone());
        }
    }
    let mut parts: Vec<TokenStream> = Vec::new();
    match recorded(name) {
        Some(Record::Through {
            parameters: _parameters,
            slot: _slot,
        }) => {
            if let Item::Struct(item_struct) = item
                && let Some(slot) = written_as_its_slot(item_struct)
            {
                parts.extend(unseen_keys(&slot.ty, &parameters));
            }
        }
        Some(Record::Shape(_)) => {
            for (field, _) in parts_of(item) {
                if !read_by_a_function(field) && !under_a_cfg(field) {
                    parts.extend(unseen_keys(&field.ty, &parameters));
                }
            }
        }
        None => {}
    }
    if parts.is_empty() {
        return quote! { &[#(#all),*] };
    }
    let fallback = fallback_trait(&unclaimed_parameter(
        "NotMet",
        &written_names(item.to_token_stream()),
    ));
    quote! {
        {
            #fallback
            const TIXSCHEMA_PARTS: &[&[&str]] = &[&[#(#all),*], #(#parts),*];
            const TIXSCHEMA_LENGTH: usize = {
                let (mut length, mut part) = (0, 0);
                while part < TIXSCHEMA_PARTS.len() {
                    length += TIXSCHEMA_PARTS[part].len();
                    part += 1;
                }
                length
            };
            const TIXSCHEMA_KEYS: [&str; TIXSCHEMA_LENGTH] = {
                let mut keys = [""; TIXSCHEMA_LENGTH];
                let (mut at, mut part) = (0, 0);
                while part < TIXSCHEMA_PARTS.len() {
                    let mut key = 0;
                    while key < TIXSCHEMA_PARTS[part].len() {
                        keys[at] = TIXSCHEMA_PARTS[part][key];
                        at += 1;
                        key += 1;
                    }
                    part += 1;
                }
                keys
            };
            &TIXSCHEMA_KEYS
        }
    }
}

/// The value of the hidden constant of `item`, named `name`: what a holder reads of it, with each
/// part the expansion could not see read from that part's own constant.
fn flags_value(item: &Item, name: &str, generics: &Generics) -> TokenStream {
    let parameters = parameter_names(generics);
    let mut dynamic = Vec::new();
    let mut own = get_field_def("own", &syn::parse_quote! { u8 }, "");
    own.field_type = FieldDefType::SiblingType(name.to_owned(), Vec::new());
    let mut flags = match recorded(name) {
        None => NOT_MET,
        Some(Record::Through {
            slot,
            parameters: _parameters,
        }) => {
            let read = reads(&slot, 0);
            let resolvable = read.unseen.as_ref().is_some_and(|unseen| unseen.resolvable);
            let slot_type = if let Item::Struct(item_struct) = item {
                written_as_its_slot(item_struct).map(|held| &held.ty)
            } else {
                None
            };
            slot_type
                .filter(|_| resolvable)
                .and_then(|ty| named_flags(ty, &parameters))
                .map_or_else(
                    || read.flags(),
                    |named| {
                        dynamic.push(named);
                        0
                    },
                )
        }
        Some(Record::Shape(shape)) => {
            let mut seen = Reads {
                absorbs: def_writes_a_map(&own),
                no_object: shape.writes.no_object,
                refuses: shape.closed.into_iter().collect(),
                rest: shape.rest,
                unseen: None,
                whole_unseen: shape.whole_unseen,
            };
            for (field, payload) in parts_of(item) {
                let part = member_reads(field, &parameters);
                let resolvable = part.unseen.as_ref().is_some_and(|unseen| unseen.resolvable);
                if let Some(named) = named_flags(&field.ty, &parameters).filter(|_| resolvable) {
                    dynamic.push(dynamic_part(&named, payload));
                } else if payload || part.rest {
                    seen.carry(&part);
                    if payload && let Some(why) = part.whole_unseen {
                        seen.see(Some(Unseen {
                            itself: false,
                            resolvable: false,
                            why,
                        }));
                    }
                } else {
                    // A member that takes its own keys adds nothing to what its holder refuses.
                }
            }
            seen.flags()
        }
    };
    // A slot whose own constant answers for it answers these bits too.
    let through_a_constant = !dynamic.is_empty()
        && matches!(
            recorded(name),
            Some(Record::Through {
                parameters: _parameters,
                slot: _slot
            })
        );
    if !through_a_constant {
        // A type whose read the expansion cannot see through is held to read from an object of
        // none of its keys, as one no `#[model_schema]` expanded is.
        if empty_of(&own, 0) != Answer::No {
            flags |= EMPTY;
        }
        if entry_of(&own, 0) {
            flags |= ENTRY;
        }
    }
    flags |= written_flags(item, &parameters, &mut dynamic);
    if dynamic.is_empty() {
        return quote! { #flags };
    }
    let fallback = fallback_trait(&unclaimed_parameter(
        "NotMet",
        &written_names(item.to_token_stream()),
    ));
    quote! {
        {
            #fallback
            #flags #(| #dynamic)*
        }
    }
}

/// The bits of the hidden constant of `item` that say what serde writes of it that no holder may
/// flatten: a plain enum's variant name, an `Option` whose `None` reads back as `Some`, and a map
/// keyed by a number or a `bool` at any depth. What a part the expansion could not see answers is
/// pushed to `dynamic`.
fn written_flags(item: &Item, parameters: &[String], dynamic: &mut Vec<TokenStream>) -> u16 {
    let mut flags = 0;
    if super::written_as_a_variant_name(item) {
        flags |= PLAIN;
    }
    if let Some(slot) = optional_slot(item) {
        let held = under_option(&def_of(&slot.ty, parameters));
        let answered = (!keys_complete(&held, 0))
            .then(|| named_flags(&slot.ty, parameters))
            .flatten();
        if let Some(named) = answered {
            dynamic.push(quote! { (if #named & #EMPTY != 0 { #OPTION_EMPTY } else { 0 }) });
        } else if empty_of(&held, 0) != Answer::No {
            flags |= OPTION_EMPTY;
        } else {
            // What the slot holds requires a key, so serde reads `None` back.
        }
    }
    let flags_ident = proc_macro2::Ident::new(FLAGS, Span::call_site());
    for field in read_members(item) {
        if !super::bson_refused_keys(&field.ty, super::KeyAsked::Refused).is_empty() {
            flags |= TEXT_KEYS;
        }
        dynamic.extend(
            unseen_types(&field.ty, parameters)
                .into_iter()
                .map(|named| quote! { (<#named>::#flags_ident & #TEXT_KEYS) }),
        );
    }
    flags
}

/// The slot `item` is written as, where serde writes it as an `Option` of what the slot holds.
fn optional_slot(item: &Item) -> Option<&Field> {
    let Item::Struct(item_struct) = item else {
        return None;
    };
    let slot = written_as_its_slot(item_struct)?;
    let def = def_of(&slot.ty, &parameter_names(&item_struct.generics));
    (def.array_depth == 0 && def.is_optional()).then_some(slot)
}

/// Every member of `item` serde reads itself, rather than through a function of its author's.
fn read_members(item: &Item) -> Vec<&Field> {
    let fields: Vec<&Field> = if let Item::Struct(item_struct) = item {
        item_struct.fields.iter().collect()
    } else if let Item::Enum(item_enum) = item {
        item_enum
            .variants
            .iter()
            .filter(|variant| variant_on_the_wire(variant))
            .flat_map(|variant| variant.fields.iter())
            .collect()
    } else {
        Vec::new()
    };
    fields
        .into_iter()
        .filter(|field| read_by_serde(field) && !read_by_a_function(field) && !under_a_cfg(field))
        .collect()
}

/// Whether a `cfg` decides whether `field` is declared at all: a `const` that names its type is
/// not under that `cfg`.
fn under_a_cfg(field: &Field) -> bool {
    field.attrs.iter().any(|attr| attr.path().is_ident("cfg"))
}

/// Each type `ty` names that no `#[model_schema]` above recorded, as a `const` beside the
/// declaration names it: behind a wrapper, in a map's value, in a tuple, and as a type argument.
fn unseen_types(ty: &Type, parameters: &[String]) -> Vec<TokenStream> {
    let mut found = Vec::new();
    collect_unseen(ty, parameters, &mut found);
    found
}

fn collect_unseen(ty: &Type, parameters: &[String], found: &mut Vec<TokenStream>) {
    let wrapping = if let Type::Paren(parenthesized) = ty {
        Some(&*parenthesized.elem)
    } else if let Type::Group(grouped) = ty {
        Some(&*grouped.elem)
    } else if let Type::Array(array) = ty {
        Some(&*array.elem)
    } else if let Type::Slice(slice) = ty {
        Some(&*slice.elem)
    } else if let Type::Reference(reference) = ty {
        Some(&*reference.elem)
    } else {
        None
    };
    if let Some(inner) = wrapping {
        return collect_unseen(inner, parameters, found);
    }
    if let Type::Tuple(tuple) = ty {
        for inner in &tuple.elems {
            collect_unseen(inner, parameters, found);
        }
        return;
    }
    let Type::Path(path) = ty else {
        return;
    };
    let Some(last) = path.path.segments.last() else {
        return;
    };
    if let syn::PathArguments::AngleBracketed(arguments) = &last.arguments {
        for argument in &arguments.args {
            if let syn::GenericArgument::Type(inner) = argument {
                collect_unseen(inner, parameters, found);
            }
        }
    }
    let FieldDefType::SiblingType(name, _) = get_field_def("", ty, "").field_type else {
        return;
    };
    if last.ident.unraw() != name || recorded(&name).is_some() || declared(&name).is_some() {
        return;
    }
    let written = ty.to_token_stream();
    if names_the_declaration(written.clone(), parameters) {
        return;
    }
    let named = with_static_lifetimes(written);
    let tokens = quote! { #named };
    let spelled_out = tokens.to_string();
    if !found.iter().any(|seen| seen.to_string() == spelled_out) {
        found.push(tokens);
    }
}

/// The `const` of an enum serde reads from a copy of the payload it buffers, under a tag alone or
/// untagged, that refuses each member of a variant whose type no `#[model_schema]` above recorded
/// where its hidden constant says it holds a map keyed by a number or a `bool`, and the payload of
/// a newtype variant under a tag alone that serde writes as no object.
fn buffered_checks(item: &Item) -> Option<TokenStream> {
    let Item::Enum(item_enum) = item else {
        return None;
    };
    let said = match Tagging::of(item_enum) {
        Tagging::Internal { tag: _tag } => super::ranges::KEYS_IN_A_TAGGED_ENUM,
        Tagging::Untagged => super::ranges::KEYS_IN_AN_UNTAGGED_ENUM,
        Tagging::Adjacent {
            content: _content,
            tag: _tag,
        } => return None,
        Tagging::External | Tagging::Plain => return None,
    };
    let parameters = parameter_names(&item_enum.generics);
    let flags_ident = proc_macro2::Ident::new(FLAGS, Span::call_site());
    let tag = if let Tagging::Internal { tag } = Tagging::of(item_enum) {
        Some(tag)
    } else {
        None
    };
    let mut checks = Vec::new();
    for variant in item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
    {
        let payload = newtype_slot(variant)
            .filter(|slot| read_by_serde(slot) && !read_by_a_function(slot) && !under_a_cfg(slot));
        if let (Some(key), Some(slot)) = (&tag, payload)
            && let Some(named) = named_flags(&slot.ty, &parameters)
                .filter(|_| !keys_complete(&def_of(&slot.ty, &parameters), 0))
        {
            let message = prefixed_guard_message(&format!(
                "type `{}`: variant `{}` holds `{}`, declared below `{}`, in another crate, or \
                 with no `#[model_schema]`, which serde does not write as an object, so nothing of \
                 it joins the tag `{key}`. Name a `content` key so the value gets an object of its \
                 own, or wrap it in a struct whose fields can sit beside the tag.",
                item_enum.ident.unraw(),
                variant.ident.unraw(),
                spelled(&slot.ty),
                item_enum.ident.unraw()
            ))
            .replace('{', "{{")
            .replace('}', "}}");
            let span = slot.ty.span();
            checks.push(quote_spanned! { span=>
                ::core::assert!(#named & (#PLAIN | #NO_OBJECT) == 0, #message);
            });
        }
        for (position, field) in variant.fields.iter().enumerate() {
            if !read_by_serde(field)
                || read_by_a_function(field)
                || under_a_cfg(field)
                || parse_serde_field_attributes(&field.attrs).flatten
            {
                continue;
            }
            let member = field.ident.as_ref().map_or_else(
                || format!("slot {position}"),
                |ident| format!("field `{}`", ident.unraw()),
            );
            for named in unseen_types(&field.ty, &parameters) {
                let message = prefixed_guard_message(&format!(
                    "type `{}`: variant `{}`, {member}: `{}`, declared below `{}`, in another crate, \
                     or with no `#[model_schema]`, holds a map keyed by a number or a `bool`: \
                     {said} Key the map by a `String`.",
                    item_enum.ident.unraw(),
                    variant.ident.unraw(),
                    spelled(&named),
                    item_enum.ident.unraw()
                ))
                .replace('{', "{{")
                .replace('}', "}}");
                let span = field.ty.span();
                checks.push(quote_spanned! { span=>
                    ::core::assert!(<#named>::#flags_ident & #TEXT_KEYS == 0, #message);
                });
            }
        }
    }
    if checks.is_empty() {
        return None;
    }
    let fallback = fallback_trait(&unclaimed_parameter(
        "NotMet",
        &written_names(item.to_token_stream()),
    ));
    Some(quote! {
        const _: () = {
            #fallback
            #(#checks)*
        };
    })
}

#[cfg(test)]
mod tests;
