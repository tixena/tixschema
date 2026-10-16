//! `validate()` on each Kotlin type the emitter writes: every member's bounds, what the value it
//! holds reports and the checks it names, in the order declared, then the type's own checks. The
//! order and each message are the ones Rust's `validate()` writes.
//!
//! A brand with a bound, and a variant of an untagged enum, carry `kotlinSchemaBound()` beside
//! it: what the bounds alone answer, which is what chooses the variant a payload is read as.

extern crate alloc;

use alloc::collections::BTreeSet;
use core::cell::RefCell;
use core::mem;
use std::collections::HashMap;

use super::{kotlin_name, kotlin_template_text, kotlin_typename};
use crate::bound_message::{Bound, kotlin_violation};
use crate::features::model_schema_prop::{FunctionName, ModelSchemaPropMeta};
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};

/// The list `validate()` collects its messages in.
const VIOLATIONS: &str = "violations";

/// What a walk asks each value of a declared type it reaches for.
#[derive(Clone, Copy)]
enum Asked {
    /// What its own bounds answer, the checks it names apart: `kotlinSchemaBound()`.
    Bound,
    /// Every message of the value: `validate()`.
    Report,
}

/// A function a type's body carries beside `validate()`, for the statements that call it. Each is
/// private to the type that writes it: emitted types are joined into one file.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Helper {
    /// What a held value answers where its type carries no `validate()`: a type of the
    /// consumer's own, and an alias of a string or of a list. A member is found ahead of it.
    Fallback,
    /// A float as Rust's `Display` writes one: no exponent, and no `.0` on a whole one.
    Shown,
    /// What a string answers where a declared type is asked for its text: itself. A brand
    /// answers with a member of its own, which is found ahead of this.
    Text,
    /// What a held value answers for its own bounds where its type carries none: every type but
    /// a brand with a bound, whose member is found ahead of it.
    Unbounded,
    /// A violation under the place that holds the value it is about: written into the name the
    /// violation leads with, or in front of one that leads with none.
    Under,
}

/// The Kotlin type a number is held as, which is what a bound's literal is written for.
#[derive(Clone, Copy)]
enum Number {
    Double,
    Float,
    Signed,
    Unsigned,
}

/// How a string bound reads the text of a value.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Text {
    /// The type is no string and no declared type.
    Absent,
    /// The type is declared elsewhere, and is asked for its text.
    Asked,
    /// The value is the string itself.
    Own,
}

/// One member as `validate()` reads it.
pub struct Member<'member> {
    pub def: &'member FieldDef,
    /// The steps serde writes on the way to the member, as the text of a Kotlin string, and
    /// `None` where it writes none.
    pub place: Option<String>,
    /// The Kotlin expression the member is read from.
    pub read: String,
}

/// What a type the emitter wrote publishes to one written after it.
#[derive(Clone, Copy)]
struct Noted {
    /// Whether the type carries `kotlinSchemaBound()`, a brand with a bound of its own: `None`
    /// for an alias, which is whatever it names.
    bound: Option<bool>,
    /// Whether a value of the type answers its text: a brand that carries `kotlinSchemaText()`,
    /// and an alias of a string or of such a brand.
    text: bool,
    /// Whether `{Type}Unjudged` reads the type with no judgement.
    unjudged: bool,
    /// Whether the type carries `validate()`.
    validates: bool,
}

/// The `validate()` of one Kotlin type, as it is written.
pub struct Rules<'item> {
    /// What the bounds alone add: a brand's own, and those that choose a variant of an untagged
    /// enum. They are written into `kotlinSchemaBound()`.
    bounds: Vec<String>,
    /// Whether the members are those of a variant of an untagged enum, which its bounds choose.
    chooses: bool,
    /// The functions the statements call, each written once into the type's body.
    helpers: BTreeSet<Helper>,
    /// The type parameters the item declares `Serialize` for: a map's key typed with one is
    /// written into a place, as Rust writes it.
    serializable: &'item [String],
    statements: Vec<String>,
}

thread_local! {
    /// What each Rust ident this module wrote a type for publishes to a type written after it.
    static NOTED: RefCell<HashMap<String, Noted>> = RefCell::new(HashMap::new());
}

impl Helper {
    const fn written(self) -> &'static str {
        match self {
            Self::Fallback => "private fun Any?.validate(): List<String> = emptyList()",
            Self::Shown => {
                "private fun kotlinSchemaShown(shown: String): String = if (shown == \"NaN\" || \
                 shown.endsWith(\"Infinity\")) shown.replace(\"Infinity\", \"inf\") else \
                 java.math.BigDecimal(shown).stripTrailingZeros().toPlainString()"
            }
            Self::Text => "private fun String.kotlinSchemaText(): String = this",
            Self::Unbounded => "private fun Any?.kotlinSchemaBound(): List<String> = emptyList()",
            Self::Under => {
                "private fun kotlinSchemaUnder(place: String, violation: String): String { val \
                 end = violation.indexOf('\\'', 1); return if (violation.startsWith(\"'\") && end \
                 > 0) { \"'$place.${violation.substring(1, end)}'${violation.substring(end + \
                 1)}\" } else { \"'$place': $violation\" } }"
            }
        }
    }
}

impl Number {
    /// The comparison of `read` with `bound` under `operator`, the bound written as a literal of
    /// the number's own type where it is one, and both compared as doubles where it is not.
    fn compared(self, read: &str, operator: &str, bound: f64) -> String {
        let shown = bound.to_string();
        let whole = !shown.contains('.') && shown.len() < 19;
        let double = if shown.contains('.') {
            shown.clone()
        } else {
            format!("{shown}.0")
        };
        match self {
            Self::Double => format!("{read} {operator} {double}"),
            Self::Float => format!("{read} {operator} {shown}f"),
            Self::Signed if whole => format!("{read} {operator} {shown}"),
            Self::Unsigned if whole && !shown.starts_with('-') => {
                format!("{read} {operator} {shown}u")
            }
            Self::Signed | Self::Unsigned => format!("{read}.toDouble() {operator} {double}"),
        }
    }

    /// The number a field of this type is held as, or `None` for a type that is no number.
    const fn of(field_type: &FieldDefType) -> Option<Self> {
        if matches!(field_type, FieldDefType::F64) {
            Some(Self::Double)
        } else if matches!(field_type, FieldDefType::F32) {
            Some(Self::Float)
        } else if matches!(
            field_type,
            FieldDefType::U8 | FieldDefType::U16 | FieldDefType::U32
        ) {
            Some(Self::Unsigned)
        } else if field_type.is_numeric() {
            Some(Self::Signed)
        } else {
            None
        }
    }

    /// The Kotlin expression a message quotes `read` back with.
    fn quoted(self, read: &str) -> String {
        match self {
            Self::Double | Self::Float => format!("kotlinSchemaShown({read}.toString())"),
            Self::Signed | Self::Unsigned => read.to_owned(),
        }
    }
}

impl<'item> Rules<'item> {
    /// What the bounds written on `member` add, each measured on `read`, the value under the
    /// member's own `Option`.
    fn bound_statements(
        &mut self,
        member: &Member<'_>,
        held: &FieldDef,
        read: &str,
    ) -> Vec<String> {
        let Some(meta) = member.def.model_schema_prop_meta.as_ref() else {
            return Vec::new();
        };
        let place = member.place.as_deref();
        if held.is_list() {
            return count_statements(meta, read, place);
        }
        if let Some(number) = Number::of(&held.field_type) {
            return self.range_statements(meta, number, read, place);
        }
        if !bounds_a_text(meta) {
            return Vec::new();
        }
        self.text_read(held, read)
            .map_or_else(Vec::new, |text| text_statements(meta, &text, place))
    }

    /// Whether the type answers what its bounds alone hold a value to.
    pub const fn bounded(&self) -> bool {
        !self.bounds.is_empty()
    }

    /// What a brand's own bounds add, over the value it holds and under no place. `default` is
    /// the type declared for the parameter a brand holds bare: the bounds run where the value is
    /// of that type, and at no other filling.
    pub fn brand(&mut self, member: &Member<'_>, default: Option<&FieldDef>) {
        let bounds = if let Some(declared) = default {
            self.default_statements(member, declared)
        } else {
            let (held, read) = presented(member);
            let bound = self.bound_statements(member, &held, &read);
            behind(member, &read, &bound)
        };
        self.bounds.extend(bounds.iter().cloned());
        self.statements.extend(bounds);
    }

    /// What the checks `member` names add, each called on `read` and its messages written under
    /// the member's place.
    fn check_statements(&mut self, member: &Member<'_>, read: &str) -> Vec<String> {
        let Some(meta) = member.def.model_schema_prop_meta.as_ref() else {
            return Vec::new();
        };
        meta.check
            .iter()
            .map(|check| {
                let answered = format!("{}({read})", kotlin_name(&check.name));
                self.collected(&answered, member.place.as_deref())
            })
            .collect()
    }

    /// The rules of a variant of an untagged enum: `validate()`, and the bounds that choose the
    /// variant, which are each member's own and those of each brand it holds.
    pub const fn choosing(serializable: &'item [String]) -> Self {
        Self {
            bounds: Vec::new(),
            chooses: true,
            helpers: BTreeSet::new(),
            serializable,
            statements: Vec::new(),
        }
    }

    /// The statement that collects what `answered` lists, each message under `place`.
    fn collected(&mut self, answered: &str, place: Option<&str>) -> String {
        place.map_or_else(
            || format!("{VIOLATIONS}.addAll({answered})"),
            |under| {
                self.helpers.insert(Helper::Under);
                format!(
                    "{VIOLATIONS}.addAll({answered}.map {{ kotlinSchemaUnder(\"{under}\", it) }})"
                )
            },
        )
    }

    /// What a brand's bounds add at `declared`, the type declared for the parameter it holds
    /// bare.
    fn default_statements(&mut self, member: &Member<'_>, declared: &FieldDef) -> Vec<String> {
        let Some(meta) = member.def.model_schema_prop_meta.as_ref() else {
            return Vec::new();
        };
        if !bounds_a_text(meta) {
            return Vec::new();
        }
        let Some(text) = self.text_read(declared, &member.read) else {
            return Vec::new();
        };
        vec![format!(
            "if ({} is {}) {{ {} }}",
            member.read,
            kotlin_typename(declared),
            text_statements(meta, &text, None).join("; ")
        )]
    }

    /// What the value `read` holds answers `asked`: each value of a declared type under it,
    /// through a nullable, each level of a list, a map's values and a tuple's positions. `depth`
    /// numbers the names the walk binds.
    fn held_statements(
        &mut self,
        def: &FieldDef,
        read: &str,
        place: Option<&str>,
        depth: usize,
        asked: Asked,
    ) -> Vec<String> {
        let below = depth.saturating_add(1);
        if def.is_optional() {
            let held = bound_name("held", depth);
            let inside = self.held_statements(&present(def), &held, place, below, asked);
            return wrapped(&inside, |inner| {
                format!("{read}?.let {{ {held} -> {inner} }}")
            });
        }
        if def.array_depth > 0 {
            let (index, held) = (bound_name("index", depth), bound_name("held", depth));
            let at = stepped(place, &format!("${index}"));
            let inside = self.held_statements(&item_of(def), &held, Some(&at), below, asked);
            return wrapped(&inside, |inner| {
                format!("{read}.forEachIndexed {{ {index}, {held} -> {inner} }}")
            });
        }
        if let FieldDefType::SiblingType(name, arguments) = &def.field_type {
            if let [element] = arguments.as_slice()
                && is_sequence_wrapper(name)
            {
                let listed = def.collection_element_field(element);
                return self.held_statements(&listed, read, place, depth, asked);
            }
            self.method_asked(name, asked)
                .map_or_else(Vec::new, |method| {
                    vec![self.collected(&format!("{read}.{method}()"), place)]
                })
        } else if let FieldDefType::Map(key, value) = &def.field_type {
            let (entry, held) = (bound_name("key", depth), bound_name("held", depth));
            let written = self.key_step(key, &entry);
            let at = written.as_ref().map_or_else(
                || place.map(ToOwned::to_owned),
                |key_text| Some(stepped(place, key_text)),
            );
            let inside = self.held_statements(value, &held, at.as_deref(), below, asked);
            let bound = if written.is_some() {
                entry.as_str()
            } else {
                "_"
            };
            wrapped(&inside, |inner| {
                format!("for (({bound}, {held}) in {read}) {{ {inner} }}")
            })
        } else if let FieldDefType::Tuple(positions) = &def.field_type {
            let mut found = Vec::new();
            for (at, position) in positions.iter().enumerate() {
                let slot = format!("{read}.slot{at}");
                let under = stepped(place, &at.to_string());
                found.extend(self.held_statements(position, &slot, Some(&under), depth, asked));
            }
            found
        } else if let FieldDefType::Range(kind, bound) = &def.field_type {
            let mut found = Vec::new();
            for member in kind.members() {
                let held = format!("{read}.{member}");
                let under = stepped(place, member);
                found.extend(self.held_statements(bound, &held, Some(&under), depth, asked));
            }
            found
        } else {
            Vec::new()
        }
    }

    /// The step the key of a map's entry adds to a place, as the text serde writes for the key,
    /// or `None` for a key typed with a parameter the item declares no `Serialize` for.
    fn key_step(&self, key: &FieldDef, entry: &str) -> Option<String> {
        if let FieldDefType::TypeParam(parameter) = &key.field_type {
            self.serializable
                .iter()
                .any(|declared| declared == parameter)
                .then(|| format!("${entry}"))
        } else if matches!(key.field_type, FieldDefType::SiblingType(_, _)) {
            Some(format!(
                "${{Json.encodeToJsonElement(serializer<{}>(), {entry}).jsonPrimitive.content}}",
                kotlin_typename(key)
            ))
        } else {
            Some(format!("${entry}"))
        }
    }

    /// What `member` adds: its bounds, what the value it holds reports where no bound of its own
    /// stands in for that, then the checks it names. Where the bounds choose a variant, they are
    /// the member's own and those of each brand it holds.
    pub fn member(&mut self, member: &Member<'_>) {
        let (held, read) = presented(member);
        let place = member.place.as_deref();
        let depth = usize::from(member.def.is_optional());
        let mut statements = self.bound_statements(member, &held, &read);
        let asks_the_held = statements.is_empty() || held.is_list();
        if self.chooses {
            let mut choosing = statements.clone();
            if asks_the_held {
                choosing.extend(self.held_statements(&held, &read, place, depth, Asked::Bound));
            }
            self.bounds.extend(behind(member, &read, &choosing));
        }
        if asks_the_held {
            statements.extend(self.held_statements(&held, &read, place, depth, Asked::Report));
        }
        statements.extend(self.check_statements(member, &read));
        self.statements.extend(behind(member, &read, &statements));
    }

    /// The method a value of the declared type `name` is asked through, and `None` where the
    /// type is known to carry no bound of its own.
    fn method_asked(&mut self, name: &str, asked: Asked) -> Option<&'static str> {
        match asked {
            Asked::Bound => {
                let carried = carries_a_bound(name);
                if carried.is_none() {
                    self.helpers.insert(Helper::Unbounded);
                }
                (carried != Some(false)).then_some("kotlinSchemaBound")
            }
            Asked::Report => {
                if !validates(name) {
                    self.helpers.insert(Helper::Fallback);
                }
                Some("validate")
            }
        }
    }

    pub const fn of(serializable: &'item [String]) -> Self {
        Self {
            bounds: Vec::new(),
            chooses: false,
            helpers: BTreeSet::new(),
            serializable,
            statements: Vec::new(),
        }
    }

    /// The type's own checks, each called on the whole value, its messages as it writes them.
    pub fn own(&mut self, checks: &[FunctionName]) {
        for check in checks {
            self.statements.push(format!(
                "{VIOLATIONS}.addAll({}(this))",
                kotlin_name(&check.name)
            ));
        }
    }

    /// What `minimum` and `maximum` add, each compared in the number's own type.
    fn range_statements(
        &mut self,
        meta: &ModelSchemaPropMeta,
        number: Number,
        read: &str,
        place: Option<&str>,
    ) -> Vec<String> {
        let quoted = number.quoted(read);
        let mut statements = Vec::new();
        if let Some(least) = meta.minimum {
            let reported = kotlin_violation(Bound::Minimum(least), place, &quoted);
            let broken = number.compared(read, "<", least);
            statements.push(format!("if ({broken}) {VIOLATIONS}.add({reported})"));
        }
        if let Some(most) = meta.maximum {
            let reported = kotlin_violation(Bound::Maximum(most), place, &quoted);
            let broken = number.compared(read, ">", most);
            statements.push(format!("if ({broken}) {VIOLATIONS}.add({reported})"));
        }
        if !statements.is_empty() && matches!(number, Number::Double | Number::Float) {
            self.helpers.insert(Helper::Shown);
        }
        statements
    }

    /// The member a brand answers its text with, where it holds a string, or a declared type
    /// that answers its own: `None` for a brand over anything else.
    pub fn text_member(&mut self, held: &FieldDef) -> Option<String> {
        match text_of(held) {
            Text::Own => Some("fun kotlinSchemaText(): String = value".to_owned()),
            Text::Asked if answers_a_text(held) => {
                self.helpers.insert(Helper::Text);
                Some("fun kotlinSchemaText(): String = value.kotlinSchemaText()".to_owned())
            }
            Text::Absent | Text::Asked => None,
        }
    }

    /// The Kotlin expression a string bound measures on `read`, a value typed `def`: the value
    /// where it is a string, what a declared type answers for its text, and `None` for a type
    /// that is neither.
    fn text_read(&mut self, def: &FieldDef, read: &str) -> Option<String> {
        match text_of(def) {
            Text::Absent => None,
            Text::Asked => {
                self.helpers.insert(Helper::Text);
                Some(format!("{read}.kotlinSchemaText()"))
            }
            Text::Own => Some(read.to_owned()),
        }
    }

    /// The members the type's body carries: `validate()`, written as `modifier` opens it,
    /// `kotlinSchemaBound()` where the type answers its bounds alone, and each function their
    /// statements call.
    pub fn written(&mut self, modifier: &str) -> String {
        let statements = mem::take(&mut self.statements);
        let validate = if statements.is_empty() {
            format!("{modifier}fun validate(): List<String> = emptyList()")
        } else {
            format!(
                "{modifier}fun validate(): List<String> {{ val {VIOLATIONS} = \
                 mutableListOf<String>(); {}; return {VIOLATIONS} }}",
                statements.join("; ")
            )
        };
        let mut members = vec![validate];
        let bounds = mem::take(&mut self.bounds);
        if !bounds.is_empty() {
            members.push(format!(
                "fun kotlinSchemaBound(): List<String> {{ val {VIOLATIONS} = \
                 mutableListOf<String>(); {}; return {VIOLATIONS} }}",
                bounds.join("; ")
            ));
        }
        members.extend(
            mem::take(&mut self.helpers)
                .into_iter()
                .map(|helper| helper.written().to_owned()),
        );
        members.join("; ")
    }
}

/// Whether the declared type `def` names is one the emitter wrote that answers its text.
fn answers_a_text(def: &FieldDef) -> bool {
    def.named_shape().is_some_and(|(named, _)| {
        NOTED.with(|seen| seen.borrow().get(named).is_some_and(|noted| noted.text))
    })
}

/// Whether `validate()` asks what a member typed `def` holds for what it reports: no bound of the
/// member's own stands in for that, a list apart. A model is read unjudged there, and only there.
pub fn asks_the_held(def: &FieldDef) -> bool {
    let member = Member {
        def,
        place: None,
        read: String::new(),
    };
    let (held, read) = presented(&member);
    held.is_list()
        || Rules::of(&[])
            .bound_statements(&member, &held, &read)
            .is_empty()
}

/// `statements`, which read `read`, behind the member's own `Option` where it has one.
fn behind(member: &Member<'_>, read: &str, statements: &[String]) -> Vec<String> {
    if member.def.is_optional() {
        wrapped(statements, |inner| {
            format!("{}?.let {{ {read} -> {inner} }}", member.read)
        })
    } else {
        statements.to_vec()
    }
}

/// The name a walk binds at `depth`: `held`, then `held2` below it, so no lambda shadows another.
fn bound_name(stem: &str, depth: usize) -> String {
    if depth == 0 {
        stem.to_owned()
    } else {
        format!("{stem}{}", depth.saturating_add(1))
    }
}

/// Whether `meta` writes a length or a pattern.
const fn bounds_a_text(meta: &ModelSchemaPropMeta) -> bool {
    meta.min_length.is_some() || meta.max_length.is_some() || meta.pattern.is_some()
}

/// Whether the type the emitter wrote under `rust_ident` carries `kotlinSchemaBound()`, and
/// `None` where it wrote none under that name, or an alias.
fn carries_a_bound(rust_ident: &str) -> Option<bool> {
    NOTED.with(|seen| seen.borrow().get(rust_ident).and_then(|noted| noted.bound))
}

/// What `minLength` and `maxLength` add on a list, whose items they count.
fn count_statements(meta: &ModelSchemaPropMeta, read: &str, place: Option<&str>) -> Vec<String> {
    let counted = format!("{read}.size");
    let mut statements = Vec::new();
    if let Some(least) = meta.min_length.or(meta.min_items) {
        let reported = kotlin_violation(Bound::MinItems(least), place, &counted);
        statements.push(format!(
            "if ({counted} < {least}) {VIOLATIONS}.add({reported})"
        ));
    }
    if let Some(most) = meta.max_length.or(meta.max_items) {
        let reported = kotlin_violation(Bound::MaxItems(most), place, &counted);
        statements.push(format!(
            "if ({counted} > {most}) {VIOLATIONS}.add({reported})"
        ));
    }
    statements
}

/// The item of a list `def` holds: `def` one level of its list down.
fn item_of(def: &FieldDef) -> FieldDef {
    let mut item = def.clone();
    item.array_depth = def.array_depth.saturating_sub(1);
    item
}

/// `pattern` as the JVM's regular expressions read what the `regex` crate reads in it: `$` as
/// the end of the text alone, `\v` as the one vertical tab, and a named group as a plain one.
pub fn jvm_pattern(pattern: &str) -> String {
    let mut written = String::with_capacity(pattern.len());
    let mut in_class = false;
    let mut rest = pattern.chars().peekable();
    while let Some(ch) = rest.next() {
        if ch == '\\' {
            match rest.next() {
                Some('v') => written.push_str("\\x0B"),
                Some(escaped) => {
                    written.push('\\');
                    written.push(escaped);
                }
                None => written.push('\\'),
            }
        } else if in_class {
            in_class = ch != ']';
            written.push(ch);
        } else if ch == '[' {
            in_class = true;
            written.push(ch);
        } else if ch == '$' {
            written.push_str("\\z");
        } else if ch == '(' {
            written.push(ch);
            // A group's name is all the JVM reads differently: it takes no `_`.
            let mut name = rest.clone();
            if name.next() == Some('?')
                && name.next() == Some('<')
                && name.peek().is_some_and(|first| !matches!(first, '=' | '!'))
            {
                rest = name;
                rest.by_ref().find(|closing| *closing == '>');
            }
        } else {
            written.push(ch);
        }
    }
    written
}

/// Notes an alias: a value of it answers its text where one of the type it names does.
pub fn note_alias(rust_ident: &str, target: &FieldDef) {
    let text = match text_of(target) {
        Text::Absent => false,
        Text::Asked => answers_a_text(target),
        Text::Own => true,
    };
    note(
        rust_ident,
        Noted {
            bound: None,
            text,
            unjudged: false,
            validates: false,
        },
    );
}

/// Notes a brand, which carries `validate()`, `kotlinSchemaText()` where `text` says so and
/// `kotlinSchemaBound()` where `bound` does.
pub fn note_brand(rust_ident: &str, text: bool, bound: bool) {
    note(
        rust_ident,
        Noted {
            bound: Some(bound),
            text,
            unjudged: true,
            validates: true,
        },
    );
}

/// Notes a type that carries `validate()`, answers no text, and is read by two serializers.
pub fn note_model(rust_ident: &str) {
    note(
        rust_ident,
        Noted {
            bound: Some(false),
            text: false,
            unjudged: true,
            validates: true,
        },
    );
}

/// Notes a plain enum, which carries `validate()` and keeps the one serializer the plugin writes.
pub fn note_plain_enum(rust_ident: &str) {
    note(
        rust_ident,
        Noted {
            bound: Some(false),
            text: false,
            unjudged: false,
            validates: true,
        },
    );
}

fn note(rust_ident: &str, noted: Noted) {
    NOTED.with(|seen| {
        seen.borrow_mut().insert(rust_ident.to_owned(), noted);
    });
}

/// Every pattern `meta` holds a text to: the member's own, then its enum's where both are written.
fn patterns_of(meta: &ModelSchemaPropMeta) -> Vec<&str> {
    #[cfg(any(feature = "serde", feature = "zod", feature = "jsonschema"))]
    let held = meta.pattern.iter().chain(&meta.enum_pattern);
    #[cfg(not(any(feature = "serde", feature = "zod", feature = "jsonschema")))]
    let held = meta.pattern.iter();
    held.map(String::as_str).collect()
}

/// `def` with its own `Option` taken off.
fn present(def: &FieldDef) -> FieldDef {
    let mut held = def.clone();
    held.nullable_levels
        .retain(|level| *level != def.array_depth);
    held
}

/// The value `member` holds under its own `Option`, and the expression it is read from: a name
/// of its own where the member is nullable.
fn presented(member: &Member<'_>) -> (FieldDef, String) {
    if member.def.is_optional() {
        (present(member.def), bound_name("held", 0))
    } else {
        (member.def.clone(), member.read.clone())
    }
}

/// Whether the emitter wrote a type under `rust_ident` that `{Type}Unjudged` reads.
pub fn reads_unjudged(rust_ident: &str) -> bool {
    NOTED.with(|seen| {
        seen.borrow()
            .get(rust_ident)
            .is_some_and(|noted| noted.unjudged)
    })
}

/// `place` with `step` written after it.
fn stepped(place: Option<&str>, step: &str) -> String {
    place.map_or_else(|| step.to_owned(), |under| format!("{under}.{step}"))
}

/// How a string bound reads the text of a value typed `def`.
fn text_of(def: &FieldDef) -> Text {
    if def.is_list() || def.is_optional() {
        Text::Absent
    } else if matches!(def.field_type, FieldDefType::String) {
        Text::Own
    } else if matches!(def.named_shape(), Some((_, false))) {
        Text::Asked
    } else {
        Text::Absent
    }
}

/// What `minLength`, `maxLength` and each pattern add on `text`, a length counted in Unicode
/// code points.
fn text_statements(meta: &ModelSchemaPropMeta, text: &str, place: Option<&str>) -> Vec<String> {
    let counted = format!("{text}.codePointCount(0, {text}.length)");
    let mut statements = Vec::new();
    if let Some(least) = meta.min_length {
        let reported = kotlin_violation(Bound::MinLength(least), place, &counted);
        statements.push(format!(
            "if ({counted} < {least}) {VIOLATIONS}.add({reported})"
        ));
    }
    if let Some(most) = meta.max_length {
        let reported = kotlin_violation(Bound::MaxLength(most), place, &counted);
        statements.push(format!(
            "if ({counted} > {most}) {VIOLATIONS}.add({reported})"
        ));
    }
    for pattern in patterns_of(meta) {
        let reported = kotlin_violation(Bound::Pattern(pattern), place, &counted);
        statements.push(format!(
            "if (!Regex(\"{}\").containsMatchIn({text})) {VIOLATIONS}.add({reported})",
            kotlin_template_text(&jvm_pattern(pattern))
        ));
    }
    statements
}

/// Whether the emitter wrote a type that carries `validate()` under `rust_ident`.
fn validates(rust_ident: &str) -> bool {
    NOTED.with(|seen| {
        seen.borrow()
            .get(rust_ident)
            .is_some_and(|noted| noted.validates)
    })
}

/// `statements` as the one statement `around` writes them in, or nothing where there are none.
fn wrapped<F>(statements: &[String], around: F) -> Vec<String>
where
    F: FnOnce(&str) -> String,
{
    if statements.is_empty() {
        Vec::new()
    } else {
        vec![around(&statements.join("; "))]
    }
}
