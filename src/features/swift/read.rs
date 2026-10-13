//! The read of every Swift type the emitter writes. `init(swiftSchemaUnjudged:)` is the type's own
//! read, and `init(from:)` runs it and then `validate()`. A member is read unjudged wherever
//! `validate()` runs the `validate()` of what it holds, so the whole value is judged once.

use super::validate::{item_of, under_option};
use super::{MapKeys, map_keys, swift_full_real_type};
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};

/// The key type a map is read entry by entry under, written into each type that reads one so.
const ANY_KEY: &str = "private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; \
     var intValue: Int? { nil }; init?(stringValue: String) { self.stringValue = stringValue }; \
     init?(intValue: Int) { nil } }";

/// The read every caller makes: the type's own, then `validate()` on the value read.
pub(super) const JUDGED: &str = "public init(from decoder: Decoder) throws { \
     try self.init(swiftSchemaUnjudged: decoder); let violations = validate(); \
     if !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(\
     codingPath: decoder.codingPath, debugDescription: violations.joined(separator: \"; \"))) } }";

/// What opens the type's own read.
pub(super) const UNJUDGED: &str = "init(swiftSchemaUnjudged decoder: Decoder) throws";

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
    /// The decoder a held type is handed.
    fn decoder(&self) -> String {
        match self {
            Self::Decoder => "decoder".to_owned(),
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
            Self::Decoder => (
                "try !decoder.singleValueContainer().decodeNil()".to_owned(),
                Self::Decoder,
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
            Self::Decoder => "decoder.container(keyedBy: SwiftSchemaAnyKey.self)".to_owned(),
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

    /// The value read as the type `read_as` through `Decodable`.
    pub(super) fn plain(&self, read_as: &str) -> String {
        match self {
            Self::Decoder => format!("{read_as}(from: decoder)"),
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
            Self::Decoder => "decoder.unkeyedContainer()".to_owned(),
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

/// The two reads of a type: its own, whose statements are `body`, and the one that judges. The
/// key type `body` names comes first.
pub(super) fn both(body: &str) -> String {
    format!(
        "{}public {UNJUDGED} {{ {body} }}; {JUDGED}; ",
        any_key(body)
    )
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
        FieldDefType::Map(key, value) => match map_keys(key) {
            MapKeys::Wrapped => {
                let wrapper = format!("{hint}KeyedMap");
                held(source, &wrapper, &|read| sink(&format!("{read}.storage")))
            }
            keys @ (MapKeys::Text | MapKeys::Whole) => {
                let [keyed, entries, entry, number] = locals.next_map();
                let (refusal, named) = if matches!(keys, MapKeys::Whole) {
                    (whole_key(&keyed, &entry, &number), number)
                } else {
                    (String::new(), format!("{entry}.stringValue"))
                };
                let under = Source::Keyed {
                    container: keyed.clone(),
                    key: entry.clone(),
                    presence: Presence::Known,
                };
                let read = unjudged(
                    value,
                    &format!("{hint}Value"),
                    &under,
                    &|held| format!("{entries}[{named}] = {held}"),
                    locals,
                );
                format!(
                    "let {keyed} = try {}; var {entries}: {} = [:]; \
                     for {entry} in {keyed}.allKeys {{ {refusal}{read} }}; {}",
                    source.keyed(),
                    written_type(def, hint),
                    sink(&entries)
                )
            }
        },
        FieldDefType::SiblingType(named, arguments)
            if arguments.len() == 1 && is_sequence_wrapper(named) =>
        {
            let items = arguments
                .first()
                .map_or_else(|| def.clone(), |item| def.collection_element_field(item));
            unjudged(&items, "Element", source, sink, locals)
        }
        FieldDefType::SiblingType(_, _)
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
