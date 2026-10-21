//! The read of every Swift type the emitter writes. `init(swiftSchemaUnjudged:)` is the type's own
//! read, and `init(from:)` runs it and then `validate()`. A member is read unjudged wherever
//! `validate()` runs the `validate()` of what it holds, so the whole value is judged once.

use super::preprocess::{self, PAYLOAD, Written};
use super::validate::{item_of, under_option};
use super::{MapKeys, held_at, keyed_map_wrapper, map_keys, swift_full_real_type};
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};

/// The read of a type alone, where it has one that takes the keys the types around it took.
const ALONE: &str = "public init(swiftSchemaUnjudged decoder: Decoder) throws { \
     try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }";

/// The key type a map is read entry by entry under, written into each type that reads one so.
const ANY_KEY: &str = "private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; \
     var intValue: Int? { nil }; init?(stringValue: String) { self.stringValue = stringValue }; \
     init?(intValue: Int) { nil } }";

/// The read every caller makes: the type's own, then `validate()` on the value read.
pub(super) const JUDGED: &str = "public init(from decoder: Decoder) throws { \
     try self.init(swiftSchemaUnjudged: decoder); let violations = validate(); \
     if !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(\
     codingPath: decoder.codingPath, debugDescription: violations.joined(separator: \"; \"))) } }";

/// What names, in the statements of a read, the keys the types around the type took: handed to
/// what is read beside the type, or skipped by the read of a map's entries ([`read_entries`]).
const TAKEN: [&str; 4] = [
    "swiftSchemaWithout: taken",
    "without: taken",
    "!taken.contains(",
    "!(taken + ",
];

/// What opens the type's own read, handed the keys the types around the type took.
pub(super) const TAKING: &str =
    "init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws";

/// What opens the type's own read.
pub(super) const UNJUDGED: &str = "init(swiftSchemaUnjudged decoder: Decoder) throws";

/// What reads the value of one entry of a map from where it is, and hands it to the sink.
pub(super) type EntryRead<'read> =
    dyn Fn(&Source, &dyn Fn(&str) -> String, &mut Locals) -> String + 'read;

/// The locals one initializer has written, from which the next are named.
#[derive(Default)]
pub(super) struct Locals {
    listed: usize,
    mapped: usize,
}

/// What an absent key is to the read of the value under it.
#[derive(Clone, Copy)]
pub(super) enum Presence {
    /// Nothing: the key was read off the container, or tested for.
    Known,
    /// The `nil` an optional member is read as.
    Nil,
    /// The error a plain `decode` answers.
    Refused,
}

/// Where a value is read from.
#[derive(Clone)]
pub(super) enum Source {
    /// The decoder the initializer was handed, whose object the type itself reads keys of: the
    /// expression is those keys, and the ones the types around the type took.
    Beside(String),
    /// The decoder the initializer was handed.
    Decoder,
    /// Under `key` of the keyed container `container`.
    Keyed {
        container: String,
        key: String,
        presence: Presence,
    },
    /// The next value of the unkeyed container named.
    Listed(String),
}

impl Locals {
    /// The container the next list is read from, and what its items are gathered in.
    fn next_list(&mut self) -> [String; 2] {
        let nth = self.listed;
        self.listed += 1;
        ["listed", "items"].map(|stem| numbered(stem, nth))
    }

    /// The container the next map is read from, what its entries are gathered in, the key of one
    /// entry, and that key as a number.
    fn next_map(&mut self) -> [String; 4] {
        let nth = self.mapped;
        self.mapped += 1;
        ["keyed", "entries", "entry", "number"].map(|stem| numbered(stem, nth))
    }
}

impl Source {
    /// What holds where the source holds a key an absent one of which is read as `nil`, or `None`
    /// for a source that refuses an absent key or has none.
    pub(super) fn absent_as_nil(&self) -> Option<String> {
        if let Self::Keyed {
            container,
            key,
            presence: Presence::Nil,
        } = self
        {
            Some(format!("{container}.contains({key})"))
        } else {
            None
        }
    }

    /// The decoder a held type is handed.
    fn decoder(&self) -> String {
        match self {
            Self::Beside(_) | Self::Decoder => "decoder".to_owned(),
            Self::Keyed {
                container,
                key,
                presence: _presence,
            } => format!("{container}.superDecoder(forKey: {key})"),
            Self::Listed(listed) => format!("{listed}.superDecoder()"),
        }
    }

    /// What holds where the source holds a value and no `null`, and the source once it does.
    fn holding(&self) -> (String, Self) {
        match self {
            Self::Beside(_) | Self::Decoder => (
                "try !decoder.singleValueContainer().decodeNil()".to_owned(),
                self.clone(),
            ),
            Self::Keyed {
                container,
                key,
                presence,
            } => {
                let held = format!("try !{container}.decodeNil(forKey: {key})");
                let test = if matches!(presence, Presence::Nil) {
                    format!("{container}.contains({key}), {held}")
                } else {
                    held
                };
                let present = Self::Keyed {
                    container: container.clone(),
                    key: key.clone(),
                    presence: Presence::Known,
                };
                (test, present)
            }
            Self::Listed(listed) => (format!("try !{listed}.decodeNil()"), self.clone()),
        }
    }

    /// The container the entries of a map are read from.
    fn keyed(&self) -> String {
        match self {
            Self::Beside(_) | Self::Decoder => {
                "decoder.container(keyedBy: SwiftSchemaAnyKey.self)".to_owned()
            }
            Self::Keyed {
                container,
                key,
                presence: _presence,
            } => format!(
                "{container}.nestedContainer(keyedBy: SwiftSchemaAnyKey.self, forKey: {key})"
            ),
            Self::Listed(listed) => {
                format!("{listed}.nestedContainer(keyedBy: SwiftSchemaAnyKey.self)")
            }
        }
    }

    /// The statements that hold in `{stem}Payload` the payload under the source, as a function is
    /// handed it, and the coding path that payload sits at. `keys` is the key type of the
    /// container, for a key written `.name`.
    pub(super) fn payload(&self, stem: &str, keys: &str) -> (String, String) {
        match self {
            Self::Beside(taken) => (
                format!(
                    "let {stem}Payload = try SwiftSchemaPayload(from: decoder, \
                     without: {taken}).value; "
                ),
                "decoder.codingPath".to_owned(),
            ),
            Self::Decoder => (
                format!("let {stem}Payload = try SwiftSchemaPayload(from: decoder).value; "),
                "decoder.codingPath".to_owned(),
            ),
            Self::Keyed {
                container,
                key,
                presence: _presence,
            } => {
                let named = if key.starts_with('.') {
                    format!("{keys}{key}")
                } else {
                    key.clone()
                };
                (
                    format!(
                        "let {stem}Payload = try {container}.decode(SwiftSchemaPayload.self, \
                         forKey: {key}).value; "
                    ),
                    format!("{container}.codingPath + [{named}]"),
                )
            }
            Self::Listed(listed) => (
                format!(
                    "let {stem}Decoder = try {listed}.superDecoder(); \
                     let {stem}Payload = try SwiftSchemaPayload(from: {stem}Decoder).value; "
                ),
                format!("{stem}Decoder.codingPath"),
            ),
        }
    }

    /// The value read as the type `read_as` through `Decodable`.
    pub(super) fn plain(&self, read_as: &str) -> String {
        match self {
            Self::Beside(_) | Self::Decoder => format!("{read_as}(from: decoder)"),
            Self::Keyed {
                container,
                key,
                presence: _presence,
            } => format!("{container}.decode({read_as}.self, forKey: {key})"),
            Self::Listed(listed) => format!("{listed}.decode({read_as}.self)"),
        }
    }

    /// The statement that refuses an absent key as a plain `decode` does. `superDecoder` hands
    /// back a decoder over `null` for one.
    fn refusing_an_absent_key(&self) -> String {
        if let Self::Keyed {
            container,
            key,
            presence: Presence::Refused,
        } = self
        {
            format!("_ = try {container}.decodeNil(forKey: {key}); ")
        } else {
            String::new()
        }
    }

    /// The container the items of a list are read from.
    fn unkeyed(&self) -> String {
        match self {
            Self::Beside(_) | Self::Decoder => "decoder.unkeyedContainer()".to_owned(),
            Self::Keyed {
                container,
                key,
                presence: _presence,
            } => format!("{container}.nestedUnkeyedContainer(forKey: {key})"),
            Self::Listed(listed) => format!("{listed}.nestedUnkeyedContainer()"),
        }
    }
}

/// The key type the statements of `read` name, where they read a map entry by entry.
pub(super) fn any_key(read: &str) -> String {
    if read.contains("SwiftSchemaAnyKey.self") {
        format!("{ANY_KEY}; ")
    } else {
        String::new()
    }
}

/// Whether `statements` name the keys the types around the type took.
pub(super) fn names_taken(statements: &str) -> bool {
    TAKEN.iter().any(|named| statements.contains(named))
}

/// Whether `reads`, the reads of a type as [`both`] writes them, hold one that takes the keys the
/// types around the type took.
pub(super) fn takes_keys(reads: &str) -> bool {
    reads.contains(TAKING)
}

/// The reads of a type whose own statements are `body`: its own, and the one that judges. Its own
/// is written twice where it takes the keys the types around the type took, and reads `body`
/// from what the type's functions answer where it names some. The types they name come first.
pub(super) fn both(body: &str, written: &Written<'_>) -> String {
    let twins = written.twins.iter().fold(String::new(), |mut held, twin| {
        held.push_str(&preprocess::naming_its_owner(twin, written.named));
        held.push_str("; ");
        held
    });
    let read = format!("{}{twins}{body}", written.beside);
    // An untagged enum writes the key type its readers name ahead of them.
    let key = if written.beside.contains("SwiftSchemaAnyKey.self") {
        String::new()
    } else {
        any_key(&read)
    };
    let payload = if !written.own.is_empty() || read.contains("SwiftSchemaPayload") {
        format!("{PAYLOAD}; ")
    } else {
        String::new()
    };
    let taking = names_taken(body) || names_taken(written.beside);
    let opened = format!("{key}{payload}{twins}");
    if written.own.is_empty() {
        return if taking {
            format!("{opened}public {TAKING} {{ {body} }}; {ALONE}; {JUDGED}; ")
        } else {
            format!("{opened}public {UNJUDGED} {{ {body} }}; {JUDGED}; ")
        };
    }
    let owner = written.named;
    let untaken = if taking {
        "let taken: [String] = []; "
    } else {
        ""
    };
    let corrected = preprocess::called(
        written.own,
        "try SwiftSchemaPayload(from: decoder, without: taken).value",
    );
    format!(
        "{opened}private struct SwiftSchemaRead: Decodable {{ let value: {owner}; \
         init(from decoder: Decoder) throws {{ value = try {owner}(swiftSchemaCorrected: decoder) }} }}; \
         private init(swiftSchemaCorrected decoder: Decoder) throws {{ {untaken}{body} }}; \
         public {TAKING} {{ self = try SwiftSchemaPayload.read(SwiftSchemaRead.self, \
         from: {corrected}, at: decoder.codingPath).value }}; {ALONE}; {JUDGED}; "
    )
}

/// The statements that read from `source` a value of the type `read_as`, which takes the keys the
/// types around it took, and hand it to `sink`: `None` for a source that hands no such keys.
pub(super) fn beside(
    source: &Source,
    read_as: &str,
    sink: &dyn Fn(&str) -> String,
) -> Option<String> {
    let Source::Beside(taken) = source else {
        return None;
    };
    Some(sink(&format!(
        "try {read_as}(swiftSchemaUnjudged: decoder, swiftSchemaWithout: {taken})"
    )))
}

/// The statements that read a value of the type `read_as` from `source` through its unjudged
/// initializer, and hand it to `sink`.
pub(super) fn held(source: &Source, read_as: &str, sink: &dyn Fn(&str) -> String) -> String {
    let read = format!("try {read_as}(swiftSchemaUnjudged: {})", source.decoder());
    format!("{}{}", source.refusing_an_absent_key(), sink(&read))
}

/// The statements that read from `source` a value `validate()` walks, each value whose
/// `validate()` the walk runs through its unjudged initializer, and hand the value to `sink`.
/// `hint` is what the types written for `def` are named from.
pub(super) fn unjudged(
    def: &FieldDef,
    hint: &str,
    source: &Source,
    sink: &dyn Fn(&str) -> String,
    locals: &mut Locals,
) -> String {
    if def.is_optional() {
        let (holds, present) = source.holding();
        let read = unjudged(&under_option(def), hint, &present, sink, locals);
        return format!("if {holds} {{ {read} }} else {{ {} }}", sink("nil"));
    }
    if def.array_depth > 0 {
        let [listed, items] = locals.next_list();
        let read = unjudged(
            &item_of(def),
            hint,
            &Source::Listed(listed.clone()),
            &|item| format!("{items}.append({item})"),
            locals,
        );
        return format!(
            "var {listed} = try {}; var {items}: {} = []; while !{listed}.isAtEnd {{ {read} }}; {}",
            source.unkeyed(),
            written_type(def, hint),
            sink(&items)
        );
    }
    match &def.field_type {
        FieldDefType::Map(key, value) => {
            map_unjudged(def, (key, value), hint, source, sink, locals)
        }
        FieldDefType::SiblingType(named, arguments)
            if arguments.len() == 1 && is_sequence_wrapper(named) =>
        {
            let items = arguments
                .first()
                .map_or_else(|| def.clone(), |item| def.collection_element_field(item));
            unjudged(&items, hint, source, sink, locals)
        }
        FieldDefType::SiblingType(named, _) => {
            let read_as = written_type(def, hint);
            preprocess::takes(named)
                .then(|| beside(source, &read_as, sink))
                .flatten()
                .unwrap_or_else(|| held(source, &read_as, sink))
        }
        FieldDefType::Range(_, _)
        | FieldDefType::Result(_, _)
        | FieldDefType::Tuple(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::StringLiteral(_)
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Boolean
        | FieldDefType::Char
        | FieldDefType::String
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::Usize
        | FieldDefType::Isize => held(source, &written_type(def, hint), sink),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => held(source, &written_type(def, hint), sink),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => held(source, &written_type(def, hint), sink),
    }
}

/// [`unjudged`] of the map `def` over `key` and `value`: through the wrapper written for it, or
/// entry by entry where Swift keys it natively.
fn map_unjudged(
    def: &FieldDef,
    (key, value): (&FieldDef, &FieldDef),
    hint: &str,
    source: &Source,
    sink: &dyn Fn(&str) -> String,
    locals: &mut Locals,
) -> String {
    if matches!(map_keys(key), MapKeys::Wrapped) {
        let (wrapper, generic_params) = keyed_map_wrapper(hint, key, value);
        let read_as = held_at(&wrapper, &generic_params);
        return held(source, &read_as, &|read| sink(&format!("{read}.storage")));
    }
    let value_hint = format!("{hint}Value");
    read_entries(
        key,
        &written_type(def, hint),
        source,
        None,
        &|under, put, nested| unjudged(value, &value_hint, under, put, nested),
        sink,
        locals,
    )
}

/// The statements that read from `source` the entries of a map whose `key` Swift keys natively
/// into a `written`, each value read under its key by `value`, and hand the map to `sink`. No
/// entry is read under a key `skipping` holds, a Swift expression of the keys taken.
pub(super) fn read_entries(
    key: &FieldDef,
    written: &str,
    source: &Source,
    skipping: Option<&str>,
    value: &EntryRead<'_>,
    sink: &dyn Fn(&str) -> String,
    locals: &mut Locals,
) -> String {
    let [keyed, entries, entry, number] = locals.next_map();
    let (refusal, named) = if matches!(map_keys(key), MapKeys::Whole) {
        (whole_key(&keyed, &entry, &number), number)
    } else {
        (String::new(), format!("{entry}.stringValue"))
    };
    let under = Source::Keyed {
        container: keyed.clone(),
        key: entry.clone(),
        presence: Presence::Known,
    };
    let read = value(
        &under,
        &|held| format!("{entries}[{named}] = {held}"),
        locals,
    );
    let skipped = skipping.map_or_else(String::new, |taken| {
        format!(" where !{}.contains({entry}.stringValue)", grouped(taken))
    });
    format!(
        "let {keyed} = try {}; var {entries}: {written} = [:]; \
         for {entry} in {keyed}.allKeys{skipped} {{ {refusal}{read} }}; {}",
        source.keyed(),
        sink(&entries)
    )
}

/// `keys`, a Swift expression of a list of keys, as the receiver of a call.
fn grouped(keys: &str) -> String {
    if keys.contains(' ') {
        format!("({keys})")
    } else {
        keys.to_owned()
    }
}

/// `stem`, then `stem1`, `stem2`: the name of the local of a kind written after `nth` others.
fn numbered(stem: &str, nth: usize) -> String {
    if nth == 0 {
        stem.to_owned()
    } else {
        format!("{stem}{nth}")
    }
}

/// The statement that reads the key `entry` of the container `keyed` as `number`, and refuses one
/// that is no number as the read of a `[Int: Value]` does.
fn whole_key(keyed: &str, entry: &str, number: &str) -> String {
    format!(
        "guard let {number} = Int({entry}.stringValue) else {{ \
         throw DecodingError.typeMismatch(Int.self, DecodingError.Context(\
         codingPath: {keyed}.codingPath + [{entry}], \
         debugDescription: \"Expected Int key but found String key instead.\")) }}; "
    )
}

/// The Swift type the emitter declares a value of `def` as.
fn written_type(def: &FieldDef, hint: &str) -> String {
    swift_full_real_type(def, hint, &mut Vec::new())
}

#[cfg(test)]
mod tests;
