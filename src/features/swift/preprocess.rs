//! `preprocess` in the Swift read: the functions a type and its members name, each called on a
//! payload ahead of the value read from it. A type that names one on itself reads the keys the
//! types around it did not take, and so does a type that reads one from its own decoder. The keys
//! a struct takes where it is flattened, and the names an externally tagged enum takes its one
//! entry under, are noted here too: a member flattened after either is not handed them. So are the
//! tag and the content key of an enum under a tag key, which a flattened `Option` is tested by.

use core::cell::RefCell;
use core::iter;
use std::collections::{HashMap, HashSet};

use super::read::{Locals, Source, counted, unjudged};
use super::{Shaped, swift_member, swift_upper_camel};
use crate::bound_message::swift_literal_text;
use crate::features::model_schema_prop::FunctionName;
use crate::field_type::{FieldDef, FieldDefType};

/// The one key an `ObjectId` is written under.
const OID: &str = "$oid";

/// Any JSON value as a function is handed it, and the read of a type back from one, written into
/// each type that names a function. `read` writes the coding path of the value into its errors.
pub(super) const PAYLOAD: &str = "private struct SwiftSchemaPayload: Decodable { let value: Any; \
     init(from decoder: Decoder) throws { \
     if let keyed = try? decoder.container(keyedBy: Key.self) { \
     var members: [String: Any] = [:]; for key in keyed.allKeys { \
     members[key.stringValue] = try keyed.decode(SwiftSchemaPayload.self, forKey: key).value }; \
     value = members } else if var listed = try? decoder.unkeyedContainer() { \
     var items: [Any] = []; while !listed.isAtEnd { \
     items.append(try listed.decode(SwiftSchemaPayload.self).value) }; value = items } \
     else { let single = try decoder.singleValueContainer(); \
     if single.decodeNil() { value = NSNull() } \
     else if let flag = try? single.decode(Bool.self) { value = flag } \
     else if let whole = try? single.decode(Int64.self) { value = whole } \
     else if let number = try? single.decode(Double.self) { value = number } \
     else { value = try single.decode(String.self) } } }; \
     init(from decoder: Decoder, without taken: [String]) throws { \
     let whole = try SwiftSchemaPayload(from: decoder).value; \
     guard var members = whole as? [String: Any] else { value = whole; return }; \
     for key in taken { members.removeValue(forKey: key) }; value = members }; \
     static func read<Read: Decodable>(_ type: Read.Type, from payload: Any, \
     at path: [CodingKey]) throws -> Read { \
     guard JSONSerialization.isValidJSONObject([payload]), \
     let written = try? JSONSerialization.data(withJSONObject: payload, \
     options: [.fragmentsAllowed]) else { \
     throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: path, \
     debugDescription: \"a preprocess answered a value that is not JSON\")) }; \
     func placed(_ context: DecodingError.Context) -> DecodingError.Context { \
     DecodingError.Context(codingPath: path + context.codingPath, \
     debugDescription: context.debugDescription, underlyingError: context.underlyingError) }; \
     do { return try JSONDecoder().decode(type, from: written) } \
     catch DecodingError.typeMismatch(let asked, let context) { \
     throw DecodingError.typeMismatch(asked, placed(context)) } \
     catch DecodingError.valueNotFound(let asked, let context) { \
     throw DecodingError.valueNotFound(asked, placed(context)) } \
     catch DecodingError.keyNotFound(let key, let context) { \
     throw DecodingError.keyNotFound(key, placed(context)) } \
     catch DecodingError.dataCorrupted(let context) { \
     throw DecodingError.dataCorrupted(placed(context)) } }; \
     private struct Key: CodingKey { var stringValue: String; var intValue: Int? { nil }; \
     init?(stringValue: String) { self.stringValue = stringValue }; \
     init?(intValue: Int) { nil } } }";

/// A name serde reads a variant of an externally tagged enum flattened into a holder under.
#[derive(Clone)]
pub(super) struct Entry {
    /// The variant's name, or an alias of it.
    pub name: String,
    /// Whether the variant holds no value, which the enum's own read takes as its name alone.
    pub unit: bool,
    /// The name the enum's own read takes the variant under.
    pub variant: String,
}

/// One member that names functions, as its read is written.
pub(super) struct Member<'member> {
    pub def: &'member FieldDef,
    pub functions: &'member [FunctionName],
    /// What the types written for the member are named from.
    pub hint: &'member str,
    /// The key type of the container the member is read from, where its key is written `.name`.
    pub keys: &'member str,
    pub plain: Plain<'member>,
    /// The Swift type the member is declared as.
    pub real_type: &'member str,
    /// What the member's locals and the type written for it are named from.
    pub stem: &'member str,
    /// The unit variants of the externally tagged enum a flattened member holds, which its read
    /// takes from an entry over `null` in what its functions answer, as serde writes it.
    pub units: &'member [Entry],
    /// Whether `validate()` runs the `validate()` of a value the member holds.
    pub walked: bool,
}

/// How a member is read where `validate()` walks nothing it holds.
pub(super) enum Plain<'member> {
    /// Item by item, through a type written for it, where it holds an array of a fixed length.
    Counted(&'member Shaped),
    /// As the type it is declared as.
    Declared,
    /// As the type `wire`, which `convert` turns into the declared one from the local it is
    /// handed.
    Wire {
        convert: &'member dyn Fn(&str) -> String,
        wire: &'member str,
    },
}

/// What the reads of a type are written from beside their own statements.
#[derive(Default)]
pub(super) struct Written<'named> {
    /// What else the type declares that reads a payload: the readers of an untagged enum.
    pub beside: &'named str,
    /// The name the type is declared under.
    pub named: &'named str,
    /// The functions the type names on itself, in the order written.
    pub own: &'named [FunctionName],
    /// The types written for the members read from what their functions answer.
    pub twins: Vec<String>,
}

thread_local! {
    /// The Rust idents of the types the emitter has written with a read that takes the keys the
    /// types around them took.
    static TAKING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

    /// The keys serde takes off its holder's object for each struct the emitter has written, read
    /// flattened: what serde hands no member flattened after it.
    static TAKEN_KEYS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());

    /// The names each externally tagged enum the emitter has written takes its one entry of its
    /// holder's object under, read flattened.
    static ENTRIES: RefCell<HashMap<String, Vec<Entry>>> = RefCell::new(HashMap::new());

    /// The tag, then the content key where it has one, of each enum under a tag key the emitter
    /// has written.
    static TAG_KEYS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());

    /// The tag and the content key serde takes off its holder's object for each enum under both
    /// the emitter has written, read flattened: what serde hands no member flattened after it.
    static TAKEN_TAG_KEYS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());

    /// The keys each brand, struct of one slot and alias over an `ObjectId` the emitter has
    /// written reads alone, refusing any other beside them.
    static ONLY_KEYS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());

    /// The Rust ident and the Swift name of each brand and struct of one slot of those that
    /// declares no parameter, in the order written: what a type parameter filled at one is told by
    /// as it is read.
    static ONLY_NAMED: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// Records whether the type `rust_ident` names reads the keys the types around it took.
pub(super) fn note(rust_ident: &str, takes: bool) {
    TAKING.with(|names| {
        if takes {
            names.borrow_mut().insert(rust_ident.to_owned());
        } else {
            names.borrow_mut().remove(rust_ident);
        }
    });
}

/// Whether the type `rust_ident` names was written, above, with a read that takes the keys the
/// types around it took.
pub(super) fn takes(rust_ident: &str) -> bool {
    TAKING.with(|names| names.borrow().contains(rust_ident))
}

/// Records `keys`, those serde takes for the struct `rust_ident` flattened into a holder, for a
/// holder written after it.
pub(super) fn note_keys(rust_ident: &str, keys: Vec<String>) {
    TAKEN_KEYS.with(|recorded| {
        recorded.borrow_mut().insert(rust_ident.to_owned(), keys);
    });
}

/// Records the keys `value`, the value of the brand, the struct of one slot or the alias
/// `rust_ident`, reads alone ([`only_keys`]), for a holder written after it: none where `value` is
/// `None`, a value read through a function of the type's own or an `Option`. `named` is the Swift
/// name of a brand or a struct of one slot that declares no parameter.
pub(super) fn note_only_keys(rust_ident: &str, named: Option<&str>, value: Option<&FieldDef>) {
    let keys = value
        .filter(|held| !held.is_optional())
        .map_or_else(Vec::new, only_keys);
    let swift_name = named.filter(|_| !keys.is_empty());
    ONLY_KEYS.with(|recorded| {
        if keys.is_empty() {
            recorded.borrow_mut().remove(rust_ident);
        } else {
            recorded.borrow_mut().insert(rust_ident.to_owned(), keys);
        }
    });
    ONLY_NAMED.with(|recorded| {
        let mut named_alone = recorded.borrow_mut();
        named_alone.retain(|(ident, _)| ident != rust_ident);
        if let Some(written) = swift_name {
            named_alone.push((rust_ident.to_owned(), written.to_owned()));
        }
    });
}

/// The test that a flattened member `def`, the type parameter `parameter` itself, holds an
/// `ObjectId` or a brand or a struct of one slot over one written above, whose read refuses any
/// key beside `$oid`, and those keys: `None` for any other member. A Swift alias is the type it
/// names.
pub(super) fn filled_alone(def: &FieldDef, parameter: &str) -> Option<(String, String)> {
    if def.array_depth > 0
        || def.is_optional()
        || !functions(def).is_empty()
        || !matches!(def.field_type, FieldDefType::TypeParam(_))
    {
        return None;
    }
    // A name the type need not declare: a crate with no `ObjectId` builds.
    let names: Vec<String> = iter::once("ObjectId".to_owned())
        .filter(|_| cfg!(feature = "mongodb"))
        .chain(ONLY_NAMED.with(|recorded| {
            recorded
                .borrow()
                .iter()
                .map(|(_, written)| written.clone())
                .collect::<Vec<_>>()
        }))
        .collect();
    (!names.is_empty()).then(|| {
        (
            format!(
                "{}.contains(String(describing: {parameter}.self))",
                listed(&names)
            ),
            listed(&[OID.to_owned()]),
        )
    })
}

/// The keys a member `def` describes, flattened into its holder, is read from alone, its read
/// refusing any other: an `ObjectId`'s `$oid`, and those of a type over one written above. None
/// for any other member, and for one that names a function, which serde reads as a value.
pub(super) fn only_keys(def: &FieldDef) -> Vec<String> {
    if !functions(def).is_empty() || def.array_depth > 0 {
        return Vec::new();
    }
    #[cfg(feature = "mongodb")]
    if matches!(def.field_type, FieldDefType::ObjectId) {
        return vec![OID.to_owned()];
    }
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return Vec::new();
    };
    ONLY_KEYS.with(|recorded| recorded.borrow().get(name).cloned().unwrap_or_default())
}

/// The keys a member `def` describes, flattened into its holder, takes off the object serde hands
/// it: those of a struct written above, the tag and the content key of an enum under both written
/// above, those [`only_keys`] answers, and none for a member that names a function, which serde
/// reads as a value.
pub(super) fn keys_taken_by(def: &FieldDef) -> Vec<String> {
    if !functions(def).is_empty() {
        return Vec::new();
    }
    let only = only_keys(def);
    if !only.is_empty() {
        return only;
    }
    let keys = struct_keys(def);
    if !keys.is_empty() {
        return keys;
    }
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return Vec::new();
    };
    TAKEN_TAG_KEYS.with(|recorded| recorded.borrow().get(name).cloned().unwrap_or_default())
}

/// Records `keys`, the tag and the content key serde takes for the enum `rust_ident` flattened into
/// a holder, for a holder written after it: none for an enum under any other tagging, or one that
/// names a function on itself.
pub(super) fn note_tag_and_content(rust_ident: &str, keys: Vec<String>) {
    TAKEN_TAG_KEYS.with(|recorded| {
        if keys.is_empty() {
            recorded.borrow_mut().remove(rust_ident);
        } else {
            recorded.borrow_mut().insert(rust_ident.to_owned(), keys);
        }
    });
}

/// Whether a member `def` describes, flattened into its holder, is read from the whole object its
/// holder reads, as TypeScript and Zod read it: a struct, and an enum under a tag key that names no
/// function on itself, written above, where the member names no function.
pub(super) fn reads_the_whole(def: &FieldDef) -> bool {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return false;
    };
    functions(def).is_empty()
        && (TAKEN_KEYS.with(|recorded| recorded.borrow().contains_key(name))
            || TAG_KEYS.with(|recorded| recorded.borrow().contains_key(name)))
}

/// The keys serde reads the struct a member `def` describes under, whatever function the member
/// names: those of a struct written above, and none for any other type.
pub(super) fn struct_keys(def: &FieldDef) -> Vec<String> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return Vec::new();
    };
    TAKEN_KEYS.with(|recorded| recorded.borrow().get(name).cloned().unwrap_or_default())
}

/// Records `keys`, the tag and the content key of the enum `rust_ident`, for a holder written after
/// it: none for an enum under no tag key, or one that names a function on itself.
pub(super) fn note_tag_keys(rust_ident: &str, keys: Vec<String>) {
    TAG_KEYS.with(|recorded| {
        if keys.is_empty() {
            recorded.borrow_mut().remove(rust_ident);
        } else {
            recorded.borrow_mut().insert(rust_ident.to_owned(), keys);
        }
    });
}

/// The keys a flattened `Option` of `def` is tested by, in the object it is handed: those of a
/// struct written above, and the tag and the content key of an enum under a tag key written above.
/// None for any other type, and for a member that names a function.
pub(super) fn keys_standing_for(def: &FieldDef) -> Vec<String> {
    let keys = keys_taken_by(def);
    if !keys.is_empty() || !functions(def).is_empty() {
        return keys;
    }
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return Vec::new();
    };
    TAG_KEYS.with(|recorded| recorded.borrow().get(name).cloned().unwrap_or_default())
}

/// Records `entries`, the names the enum `rust_ident` takes its entry under where it is flattened,
/// for a holder written after it: none for an enum serde does not read from one entry.
pub(super) fn note_entries(rust_ident: &str, entries: Vec<Entry>) {
    ENTRIES.with(|recorded| {
        if entries.is_empty() {
            recorded.borrow_mut().remove(rust_ident);
        } else {
            recorded.borrow_mut().insert(rust_ident.to_owned(), entries);
        }
    });
}

/// The enum a flattened member `def` describes, under an `Option` or not, and the names it takes
/// its one entry under: `None` for any other type, an enum written below, and a member that names
/// a function, which serde reads as a value.
pub(super) fn entries_of(def: &FieldDef) -> Option<(&str, Vec<Entry>)> {
    if !functions(def).is_empty() || def.array_depth > 0 {
        return None;
    }
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return None;
    };
    ENTRIES
        .with(|recorded| recorded.borrow().get(name).cloned())
        .map(|entries| (name.as_str(), entries))
}

/// The local the names of the entries the member `stem` found are held in: every member flattened
/// after it is handed the object without them.
pub(super) fn found(stem: &str) -> String {
    format!("{stem}Found")
}

/// The statements that read `member`, the enum `named` flattened into its holder, from the one
/// entry of the object without `taken` under one of `entries`' names, at the holder's coding path,
/// and hand it to `sink`. No entry is refused, as serde refuses it, but where `optional`, which
/// reads `nil` there; an entry the enum does not read refuses the payload, as TypeScript and Zod
/// refuse it. Swift keeps no order of an object's keys, so the first of two entries is not known:
/// entries under two names are refused, as TypeScript and Zod refuse them, naming each variant
/// they belong to once, in the order the enum declares them.
pub(super) fn entry(
    member: &Member<'_>,
    (named, entries): (&str, &[Entry]),
    taken: &str,
    optional: bool,
    sink: &dyn Fn(&str) -> String,
    twins: &mut Vec<String>,
) -> String {
    let stem = member.stem;
    let found = found(stem);
    let enum_name = swift_literal_text(named);
    let refused = |description: &str| {
        format!(
            "throw DecodingError.dataCorrupted(DecodingError.Context(\
             codingPath: decoder.codingPath, debugDescription: \"{description}\"))"
        )
    };
    let listed: Vec<String> = entries
        .iter()
        .map(|entry| format!("\"{}\"", swift_literal_text(&entry.name)))
        .collect();
    let mut variants: Vec<(&str, Vec<String>)> = Vec::new();
    for entry in entries {
        let quoted = format!("\"{}\"", swift_literal_text(&entry.name));
        match variants.last_mut() {
            Some((variant, spelled)) if *variant == entry.variant => spelled.push(quoted),
            Some(_) | None => variants.push((&entry.variant, vec![quoted])),
        }
    }
    let grouped: Vec<String> = variants
        .iter()
        .map(|(variant, spelled)| {
            format!(
                "(\"{}\", [{}])",
                swift_literal_text(variant),
                spelled.join(", ")
            )
        })
        .collect();
    let opened = format!(
        "let {stem}Object = try SwiftSchemaPayload(from: decoder, without: {taken}).value \
         as? [String: Any] ?? [:]; \
         let {found} = [{}].filter {{ {stem}Object[$0] != nil }}; \
         if {found}.count > 1 {{ let {stem}Variants = [{}].filter {{ variant in \
         variant.1.contains {{ {found}.contains($0) }} }}.map {{ variant in variant.0 }}; {} }}; ",
        listed.join(", "),
        grouped.join(", "),
        refused(&format!(
            "more than one variant of enum {enum_name} found in flattened data: \
             \\({stem}Variants.joined(separator: \", \"))"
        ))
    );
    let held = format!("let {stem}Key = {found}.first, let {stem}Held = {stem}Object[{stem}Key]");
    let read = format!(
        "{}; {}",
        entry_payload(stem, entries),
        value(member, "decoder.codingPath", sink, twins)
    );
    if optional {
        format!("{opened}if {held} {{ {read} }} else {{ {} }}", sink("nil"))
    } else {
        format!(
            "{opened}guard {held} else {{ {} }}; {read}",
            refused(&format!(
                "no variant of enum {enum_name} found in flattened data"
            ))
        )
    }
}

/// The statement that holds in `{stem}Payload` what the enum's own read takes of the entry
/// `{stem}Held` under `{stem}Key`: a unit variant's name, over `null` alone, as TypeScript and Zod
/// read it, and any other entry under the name the read takes its variant under.
fn entry_payload(stem: &str, entries: &[Entry]) -> String {
    let cases: Vec<String> = entries
        .iter()
        .filter_map(|entry| {
            let name = swift_literal_text(&entry.name);
            let variant = swift_literal_text(&entry.variant);
            if entry.unit {
                Some(format!(
                    "case \"{name}\" where {stem}Held is NSNull: {stem}Payload = \"{variant}\""
                ))
            } else if entry.name == entry.variant {
                None
            } else {
                Some(format!(
                    "case \"{name}\": {stem}Payload = [\"{variant}\": {stem}Held]"
                ))
            }
        })
        .collect();
    if cases.is_empty() {
        format!("let {stem}Payload: Any = [{stem}Key: {stem}Held]")
    } else {
        format!(
            "let {stem}Payload: Any; switch {stem}Key {{ {}; \
             default: {stem}Payload = [{stem}Key: {stem}Held] }}",
            cases.join("; ")
        )
    }
}

/// Whether `def` names such a type under no list: one a holder that reads it from its own decoder
/// hands the keys taken.
pub(super) fn beside(def: &FieldDef) -> bool {
    !def.is_list() && matches!(&def.field_type, FieldDefType::SiblingType(named, _) if takes(named))
}

/// The keys a type read from the decoder of its holder is not handed, as a Swift expression: the
/// ones the types around the holder took, then `own`, the keys of the holder's own members.
pub(super) fn taken_with<'own>(own: impl Iterator<Item = &'own str>) -> String {
    let written: Vec<String> = own
        .map(|key| format!("\"{}\"", swift_literal_text(key)))
        .collect();
    if written.is_empty() {
        "taken".to_owned()
    } else {
        format!("taken + [{}]", written.join(", "))
    }
}

/// `keys` as a Swift array of strings.
pub(super) fn listed(keys: &[String]) -> String {
    let written: Vec<String> = keys
        .iter()
        .map(|key| format!("\"{}\"", swift_literal_text(key)))
        .collect();
    format!("[{}]", written.join(", "))
}

/// Whether the object a flattened member reads from `decoder` holds one of `keys`, as a Swift
/// condition: the object less `without`, the keys it is read without, where it is handed fewer
/// keys than its holder's object holds.
pub(super) fn holds_one_of(keys: &[String], without: Option<&str>) -> String {
    let listed: Vec<String> = keys
        .iter()
        .map(|key| format!("\"{}\"", swift_literal_text(key)))
        .collect();
    let handed = match without {
        Some("taken") => " && !taken.contains($0.stringValue)".to_owned(),
        Some(taken) => format!(" && !({taken}).contains($0.stringValue)"),
        None => String::new(),
    };
    format!(
        "try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: {{ \
         [{}].contains($0.stringValue){handed} }})",
        listed.join(", ")
    )
}

/// The functions the member `def` describes names, in the order written.
pub(super) fn functions(def: &FieldDef) -> &[FunctionName] {
    def.model_schema_prop_meta
        .as_ref()
        .map_or(&[], |meta| meta.preprocess.as_slice())
}

/// `payload` as `functions` correct it: each called on the answer of the one before, in the order
/// written, under the name the attribute writes.
pub(super) fn called(functions: &[FunctionName], payload: &str) -> String {
    functions.iter().fold(payload.to_owned(), |held, function| {
        format!("{}({held})", swift_member(&function.name))
    })
}

/// The statements that read `member` from `source` through its functions, and hand the value to
/// `sink`. A member `validate()` walks is read through a type written for it, added to `twins`,
/// so each model it holds is read alone and judged with its holder, and so is one that holds an
/// array to its length, read item by item.
pub(super) fn member(
    member: &Member<'_>,
    source: &Source,
    sink: &dyn Fn(&str) -> String,
    twins: &mut Vec<String>,
) -> String {
    let (handed, path) = source.payload(member.stem, member.keys);
    let read_where_held = format!("{handed}{}", value(member, &path, sink, twins));
    match source.absent_as_nil() {
        Some(holds_key) => format!(
            "if {holds_key} {{ {read_where_held} }} else {{ {} }}",
            sink("nil")
        ),
        None => read_where_held,
    }
}

/// The statements that read `member`, the type a flattened `Option` holds, from `source` through
/// its functions, and hand the value to `sink`, as the holder's Zod schema reads it. Where the read
/// throws, an answer of `null` among it, the value is `nil` where the object the functions are
/// handed holds none of `keys`, or no key at all where `keys` is empty, and the read's error is
/// thrown where it holds one.
pub(super) fn optional(
    member: &Member<'_>,
    source: &Source,
    keys: &[String],
    sink: &dyn Fn(&str) -> String,
    twins: &mut Vec<String>,
) -> String {
    let stem = member.stem;
    let (handed, path) = source.payload(stem, member.keys);
    let object = format!("({stem}Payload as? [String: Any] ?? [:])");
    let held = if keys.is_empty() {
        format!("!{object}.isEmpty")
    } else {
        let listed: Vec<String> = keys
            .iter()
            .map(|key| format!("\"{}\"", swift_literal_text(key)))
            .collect();
        format!(
            "[{}].contains(where: {{ {object}[$0] != nil }})",
            listed.join(", ")
        )
    };
    format!(
        "{handed}do {{ {} }} catch {{ if {held} {{ throw error }}; {} }}",
        value(member, &path, sink, twins),
        sink("nil")
    )
}

/// The statements that read `member` from the JSON of its default, written between the quotes
/// at `literal`, at the coding path `path`: as it is read from a payload, with no function
/// called for it.
pub(super) fn defaulted(
    member: &Member<'_>,
    literal: &str,
    path: &str,
    sink: &dyn Fn(&str) -> String,
    twins: &mut Vec<String>,
) -> String {
    format!(
        "let {}Payload = try JSONSerialization.jsonObject(with: Data(\"{literal}\".utf8), \
         options: [.fragmentsAllowed]); {}",
        member.stem,
        value(member, path, sink, twins)
    )
}

/// The statements that read `member` from `{stem}Payload`, which sits at the coding path `path`,
/// through its functions, and hand the value to `sink`.
fn value(
    member: &Member<'_>,
    path: &str,
    sink: &dyn Fn(&str) -> String,
    twins: &mut Vec<String>,
) -> String {
    let stem = member.stem;
    let corrected = unit_entry_read(
        &called(member.functions, &format!("{stem}Payload")),
        member.units,
    );
    let read = |read_as: &str| {
        format!("try SwiftSchemaPayload.read({read_as}.self, from: {corrected}, at: {path})")
    };
    let held = |value: &str| format!("self.value = {value}");
    let statements = if member.walked {
        unjudged(
            member.def,
            member.hint,
            &Source::Decoder,
            &held,
            &mut Locals::default(),
        )
    } else {
        match member.plain {
            Plain::Counted(shaped) => counted(
                shaped,
                stem,
                &Source::Decoder,
                &held,
                &mut Locals::default(),
            ),
            Plain::Declared => return sink(&read(member.real_type)),
            Plain::Wire { convert, wire } => {
                let local = format!("wire{}", swift_upper_camel(stem));
                return format!("let {local} = {}; {}", read(wire), sink(&convert(&local)));
            }
        }
    };
    let twin = format!("SwiftSchemaRead{}", swift_upper_camel(stem));
    twins.push(format!(
        "private struct {twin}: Decodable {{ let value: {}; \
         init(from decoder: Decoder) throws {{ {statements} }} }}",
        member.real_type
    ));
    sink(&format!("{}.value", read(&twin)))
}

/// `answer` with an object of the one entry of a unit variant of `units` over `null` read as the
/// name the enum's own read takes the variant under, which reads it as its bare name.
fn unit_entry_read(answer: &str, units: &[Entry]) -> String {
    if units.is_empty() {
        return answer.to_owned();
    }
    let cases: Vec<String> = units
        .iter()
        .map(|entry| {
            format!(
                "case \"{}\": return \"{}\"",
                swift_literal_text(&entry.name),
                swift_literal_text(&entry.variant)
            )
        })
        .collect();
    format!(
        "{{ () -> Any in let answer: Any = {answer}; guard let entries = answer as? [String: Any], \
         entries.count == 1, let entry = entries.first, entry.value is NSNull else {{ return \
         answer }}; switch entry.key {{ {}; default: return answer }} }}()",
        cases.join("; ")
    )
}

/// The unit variants of the externally tagged enum written above that `def`, a flattened member or
/// the slot of a variant under a tag alone, holds, under an `Option` or not, whatever function it
/// names.
pub(super) fn units_of(def: &FieldDef) -> Vec<Entry> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return Vec::new();
    };
    ENTRIES.with(|recorded| {
        recorded
            .borrow()
            .get(name)
            .map_or_else(Vec::new, |entries| {
                entries.iter().filter(|entry| entry.unit).cloned().collect()
            })
    })
}

/// `written` with the type `Self` names in `named` under that name: in a type declared inside
/// `named`, `Self` is that type.
pub(super) fn naming_its_owner(written: &str, named: &str) -> String {
    let in_a_word = |read: char| read.is_alphanumeric() || read == '_';
    let mut renamed = String::with_capacity(written.len());
    let mut rest = written;
    while let Some((before, after)) = rest.split_once("Self") {
        renamed.push_str(before);
        let alone = !renamed.ends_with(in_a_word) && !after.starts_with(in_a_word);
        renamed.push_str(if alone { named } else { "Self" });
        rest = after;
    }
    renamed.push_str(rest);
    renamed
}

#[cfg(test)]
mod tests;
