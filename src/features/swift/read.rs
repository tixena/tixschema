//! The read of every Swift type the emitter writes. `init(swiftSchemaUnjudged:)` is the type's own
//! read, and `init(from:)` runs it and then `validate()`. A member is read unjudged wherever
//! `validate()` runs the `validate()` of what it holds, so the whole value is judged once.

use core::cell::RefCell;
use std::collections::HashMap;

use super::preprocess::{self, Written};
use super::validate::{item_of, under_option};
use super::{
    MapKeys, Shaped, arrayed, held_as_optional, held_at, keyed_map_wrapper, map_keys,
    swift_full_real_type, swift_resolve, through_alias,
};
use crate::field_type::{
    BUFFERED_WORDS, EXPECTED_VALUE, FieldDef, FieldDefType, is_sequence_wrapper,
};

/// The key type a map is read entry by entry under, and a flattened unit variant's entry written
/// under, written into each type that names it.
const ANY_KEY: &str = "private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; \
     var intValue: Int? { nil }; init?(stringValue: String) { self.stringValue = stringValue }; \
     init?(intValue: Int) { nil } }";

/// The Swift integers Foundation refuses a number outside the range of in an error no coding path
/// reaches, and reads a number written with a fraction or an exponent into: [`fitted`] reads them
/// through [`FITTED`].
const FIXED_WIDTH: [&str; 9] = [
    "Int", "Int8", "Int16", "Int32", "Int64", "UInt8", "UInt16", "UInt32", "UInt64",
];

/// What a type whose reads [`fitted`] wrote holds: `SwiftSchemaFit`, which reads one value and
/// holds it in `value`. It refuses an integer that does not fit, and one written with a fraction
/// or an exponent, at its own coding path, in serde's words, which name the integer an `Optional`
/// holds as they name the integer. It tells such a number by the byte count of its text and its
/// exponent mark, which `JSONDecoder`'s own decoder holds in stored properties it reads through
/// `Mirror`; under any other decoder the number reads as Foundation reads it. A type parameter
/// filled with a `Float` reads one beyond its range as a `Double` narrowed to an infinity of its
/// sign, as serde reads an `f32`. It refuses too an integer whose coding path is one `userInfo`
/// names under `swiftSchemaFloated`: the paths where the payload of a preprocess holds a float,
/// which `JSONSerialization` writes back with no fraction.
const FITTED: &str = "private struct SwiftSchemaFit<SwiftSchemaLeaf: Decodable>: Decodable { \
     let value: SwiftSchemaLeaf; \
     init(from decoder: Decoder) throws { let single = try decoder.singleValueContainer(); do { \
     value = try single.decode(SwiftSchemaLeaf.self) } catch let refused where \
     SwiftSchemaLeaf.self == Float.self && !(refused is DecodingError) { do { value = \
     Float(try single.decode(Double.self)) as! SwiftSchemaLeaf } catch let wider where !(wider is \
     DecodingError) { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
     decoder.codingPath, debugDescription: Self.refusal(wider), underlyingError: wider)) } } \
     catch let refused where !(refused is DecodingError) { throw \
     DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
     debugDescription: Self.refusal(refused), underlyingError: refused)) }; \
     if let integer = value as? any FixedWidthInteger, Self.fractional(decoder, integer) || \
     Self.floated(decoder) { throw \
     DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
     debugDescription: Self.floating(try single.decode(Double.self)))) } }; \
     fileprivate static func fractional(_ decoder: Decoder, _ integer: any FixedWidthInteger) \
     -> Bool { guard let written = written(decoder) else { return false }; return \
     written.exponent || written.count != \"\\(integer)\".utf8.count }; \
     private static func floated(_ decoder: Decoder) -> Bool { guard let paths = \
     decoder.userInfo[CodingUserInfoKey(rawValue: \"swiftSchemaFloated\")!] as? Set<[String]> \
     else { return false }; return paths.contains(decoder.codingPath.map { $0.intValue.map(\
     String.init) ?? $0.stringValue }) }; \
     private static func written(_ decoder: Decoder) -> (count: Int, exponent: Bool)? { func \
     child(_ of: Any, _ label: String) -> Any? { Mirror(reflecting: of).children.first { \
     $0.label == label }?.value }; guard let values = child(decoder, \"values\") as? [Any], let \
     top = values.last, let number = child(top, \"number\") else { return nil }; let parts = \
     Mirror(reflecting: number).children.map(\\.value); guard parts.count == 2, let count = \
     child(parts[0], \"count\") as? Int, let exponent = parts[1] as? Bool else { return nil }; \
     return (count, exponent) }; private static var expected: String { let named = \
     String(describing: SwiftSchemaLeaf.self); var leaf = Substring(named); while \
     leaf.hasPrefix(\"Optional<\"), leaf.hasSuffix(\">\") { leaf = leaf.dropFirst(9).dropLast() \
     }; return [\"Int\": \
     \"isize\", \"Int8\": \"i8\", \"Int16\": \"i16\", \"Int32\": \"i32\", \"Int64\": \"i64\", \
     \"UInt8\": \"u8\", \"UInt16\": \"u16\", \"UInt32\": \"u32\", \"UInt64\": \"usize\"] \
     [String(leaf)] ?? named }; \
     private static func floating(_ number: Double) -> String { var spelled = \
     number.description; if let mark = spelled.firstIndex(of: \"e\") { let exponent = \
     spelled[spelled.index(after: mark)...]; let figures = exponent.drop { \"+-0\".contains($0) }; \
     spelled = String(spelled[..<mark]) + (exponent.hasPrefix(\"-\") ? \"e-\" : \"e+\") + \
     (figures.isEmpty ? \"0\" : String(figures)) }; return \"invalid type: floating point \
     `\\(spelled)`, expected \\(expected)\" }; \
     private static func refusal(_ refused: Error) -> String { let said = \"\\(refused)\"; let \
     text = \
     said.range(of: \"parsed: \\\"\").flatMap { start in said[start.upperBound...].firstIndex(of: \
     \"\\\"\").map { String(said[start.upperBound..<$0]) } } ?? said; let digits = \
     text.hasPrefix(\"-\") ? text.dropFirst() : Substring(text); if !digits.isEmpty, \
     digits.allSatisfy({ (\"0\"...\"9\").contains($0) }), text.hasPrefix(\"-\") ? Int64(text) != \
     nil : digits.count < 20 || (digits.count == 20 && digits <= \"18446744073709551615\") { \
     return \"invalid value: integer `\\(text)`, expected \\(expected)\" }; guard let number = \
     Double(text), number.isFinite else { return \"number out of range\" }; return \
     floating(number) } }";

/// The read every caller makes: the type's own, then `validate()` on the value read.
pub(super) const JUDGED: &str = "public init(from decoder: Decoder) throws { \
     try self.init(swiftSchemaUnjudged: decoder); let violations = validate(); \
     if !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(\
     codingPath: decoder.codingPath, debugDescription: violations.joined(separator: \"; \"))) } }";

/// What names, in the statements of a read, the keys the types around the type took: handed to
/// what is read beside the type, or skipped by the read of a map's entries ([`read_entries`]).
const TAKEN: [&str; 3] = [
    "swiftSchemaWithout: taken",
    "without: taken",
    "!taken.contains(",
];

/// What opens the type's own read, handed the keys the types around the type took.
pub(super) const TAKING: &str =
    "init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws";

/// What opens the type's own read.
pub(super) const UNJUDGED: &str = "init(swiftSchemaUnjudged decoder: Decoder) throws";

/// What reads the value of one entry of a map from where it is, and hands it to the sink.
pub(super) type EntryRead<'read> =
    dyn Fn(&Source, &dyn Fn(&str) -> String, &mut Locals) -> String + 'read;

thread_local! {
    /// How [`fit_held`] reads a value of each alias the emitter wrote that names a fixed-width
    /// integer, and each alias that declares a parameter, by the Swift name the alias is declared
    /// under.
    static FITTED_ALIASES: RefCell<HashMap<String, Recorded>> = RefCell::new(HashMap::new());
}

/// What [`note_alias`] records of an alias.
#[derive(Clone)]
enum Recorded {
    /// The parameters of an alias that declares some, and the Swift type it names over them, into
    /// which what fills them at a holder is substituted.
    Generic {
        parameters: Vec<String>,
        target: String,
    },
    /// How a value of an alias that declares no parameter is read.
    Read(Aliased),
}

/// How a value of an alias that names a fixed-width integer is read through `SwiftSchemaFit`.
#[derive(Clone)]
enum Aliased {
    /// As `SwiftSchemaFit` of the alias itself: it names the integer, or an `Optional` of it.
    Alone,
    /// As what it names is read, a list or a map of the integer: [`fit_held`]'s type and
    /// [`unfit`]'s mapping of the target.
    Through { fit: String, unfit: String },
}

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
    /// The decoder the initializer was handed, of whose object the type reads alone the keys the
    /// expression names.
    Keeping(String),
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

    /// The container the next map is read from, what its entries are gathered in, and the key of
    /// one entry.
    fn next_map(&mut self) -> [String; 3] {
        let nth = self.mapped;
        self.mapped += 1;
        ["keyed", "entries", "entry"].map(|stem| numbered(stem, nth))
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
            Self::Beside(_) | Self::Decoder | Self::Keeping(_) => "decoder".to_owned(),
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
            Self::Beside(_) | Self::Decoder | Self::Keeping(_) => (
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
            Self::Beside(_) | Self::Decoder | Self::Keeping(_) => {
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
            Self::Keeping(kept) => (
                format!(
                    "let {stem}Payload = (try SwiftSchemaPayload(from: decoder).value as? \
                     [String: Any] ?? [:]).filter {{ {kept}.contains($0.key) }}; "
                ),
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

    /// The value read as the type `read_as`, which may name the type parameters `parameters`,
    /// through `Decodable`, each fixed-width integer and type parameter it holds read through
    /// `SwiftSchemaFit` ([`fitted`]).
    pub(super) fn plain(&self, read_as: &str, parameters: &[String]) -> String {
        match self {
            Self::Beside(_) | Self::Decoder | Self::Keeping(_) => fit_held(read_as, parameters)
                .zip(unfit(read_as, parameters))
                .map_or_else(
                    || format!("{read_as}(from: decoder)"),
                    |(fit, unfit)| format!("{fit}(from: decoder){unfit}"),
                ),
            Self::Keyed {
                container,
                key,
                presence: _presence,
            } => fitted(
                &format!("{container}.decode("),
                read_as,
                &format!(".self, forKey: {key})"),
                parameters,
            ),
            Self::Listed(listed) => {
                fitted(&format!("{listed}.decode("), read_as, ".self)", parameters)
            }
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
            Self::Beside(_) | Self::Decoder | Self::Keeping(_) => {
                "decoder.unkeyedContainer()".to_owned()
            }
            Self::Keyed {
                container,
                key,
                presence: _presence,
            } => format!("{container}.nestedUnkeyedContainer(forKey: {key})"),
            Self::Listed(listed) => format!("{listed}.nestedUnkeyedContainer()"),
        }
    }
}

/// Records how [`fit_held`] reads a value of the alias the emitter declares as `named`, with the
/// type parameters `parameters`, over `target`, the Swift type it names, for a holder written
/// after it, or forgets an alias that declares no parameter and names no fixed-width integer.
pub(super) fn note_alias(named: &str, parameters: &[String], target: &str) {
    let noted = if parameters.is_empty() {
        read_of(target, &[]).map(Recorded::Read)
    } else {
        Some(Recorded::Generic {
            parameters: parameters.to_vec(),
            target: target.to_owned(),
        })
    };
    FITTED_ALIASES.with(|recorded| {
        if let Some(read) = noted {
            recorded.borrow_mut().insert(named.to_owned(), read);
        } else {
            recorded.borrow_mut().remove(named);
        }
    });
}

/// How a value of `target`, a Swift type an alias names, is read through `SwiftSchemaFit` under
/// the type parameters `parameters`: through the alias itself for an integer or an `Optional` of
/// one, and as `target` is read for a list or a map of one.
fn read_of(target: &str, parameters: &[String]) -> Option<Aliased> {
    if fits(target.trim_end_matches('?'), parameters) {
        return Some(Aliased::Alone);
    }
    fit_held(target, parameters)
        .zip(unfit(target, parameters))
        .map(|(fit, unfit)| Aliased::Through { fit, unfit })
}

/// How a value of the alias `read_as` names is read through `SwiftSchemaFit`, where it is one
/// [`note_alias`] recorded: an alias that declares parameters, written with what fills them, is
/// read as what it names with each parameter written as its filling, under the holder's
/// `parameters`.
fn aliased(read_as: &str, parameters: &[String]) -> Option<Aliased> {
    let (named, filled) = read_as
        .strip_suffix('>')
        .and_then(|open| open.split_once('<'))
        .map_or((read_as, None), |(named, fillings)| (named, Some(fillings)));
    let noted = FITTED_ALIASES.with(|recorded| recorded.borrow().get(named).cloned())?;
    match noted {
        Recorded::Generic {
            parameters: declared,
            target,
        } => {
            let split = top_level(filled?);
            if split.len() != declared.len() {
                return None;
            }
            read_of(&substituted(&target, &declared, &split), parameters)
        }
        Recorded::Read(read) => filled.is_none().then_some(read),
    }
}

/// The types `fillings`, written as a list of Swift types, holds, split on its top-level commas.
fn top_level(fillings: &str) -> Vec<&str> {
    let mut depth = 0_i32;
    let mut start = 0;
    let mut split = Vec::new();
    for (at, written) in fillings.char_indices() {
        match written {
            '[' | '<' | '(' => depth += 1_i32,
            ']' | '>' | ')' => depth -= 1_i32,
            ',' if depth == 0_i32 => {
                split.push(fillings[start..at].trim());
                start = at + 1;
            }
            _ => {}
        }
    }
    split.push(fillings[start..].trim());
    split
}

/// `target` with each of `parameters` it names, as a whole identifier, written as the filling at
/// its position in `fillings`.
fn substituted(target: &str, parameters: &[String], fillings: &[&str]) -> String {
    let mut written = String::new();
    let mut word = String::new();
    let flush = |pending: &mut String, into: &mut String| {
        let filling = parameters
            .iter()
            .position(|named| named == pending)
            .and_then(|at| fillings.get(at).copied());
        into.push_str(filling.unwrap_or(pending));
        pending.clear();
    };
    for letter in target.chars() {
        if letter.is_alphanumeric() || letter == '_' {
            word.push(letter);
        } else {
            flush(&mut word, &mut written);
            written.push(letter);
        }
    }
    flush(&mut word, &mut written);
    written
}

/// `read_as`, a Swift type, with each fixed-width integer, each alias of one and each of
/// `parameters` it holds, alone or in a list, as a map's value or under an `Optional`, read through
/// `SwiftSchemaFit`; `None` where it holds none there. What fills a type parameter may be such an
/// integer.
fn fit_held(read_as: &str, parameters: &[String]) -> Option<String> {
    if let Some(held) = read_as.strip_suffix('?') {
        return fit_held(held, parameters).map(|fit| format!("{fit}?"));
    }
    if let Some((key, held)) = bracketed(read_as) {
        let fit = fit_held(held, parameters)?;
        return Some(key.map_or_else(|| format!("[{fit}]"), |named| format!("[{named}: {fit}]")));
    }
    if fits(read_as, parameters) {
        return Some(format!("SwiftSchemaFit<{read_as}>"));
    }
    match aliased(read_as, parameters)? {
        Aliased::Through { fit, unfit: _unfit } => Some(fit),
        Aliased::Alone => None,
    }
}

/// What turns a value of [`fit_held`]'s type for `read_as` back into a `read_as`: `.value` of each
/// `SwiftSchemaFit`, behind a `?` under an `Optional`, in a `.map` over a list and a `.mapValues`
/// over a map. `None` exactly where [`fit_held`] answers `None`. The closure is passed in
/// parentheses, as a read may stand in the condition of an `if`, where Swift refuses a trailing one.
fn unfit(read_as: &str, parameters: &[String]) -> Option<String> {
    if let Some(held) = read_as.strip_suffix('?') {
        return unfit(held, parameters).map(|inner| format!("?{inner}"));
    }
    if let Some((key, held)) = bracketed(read_as) {
        let inner = unfit(held, parameters)?;
        let mapping = if key.is_some() { "mapValues" } else { "map" };
        return Some(format!(".{mapping}({{ $0{inner} }})"));
    }
    if fits(read_as, parameters) {
        return Some(".value".to_owned());
    }
    match aliased(read_as, parameters)? {
        Aliased::Through { fit: _fit, unfit } => Some(unfit),
        Aliased::Alone => None,
    }
}

/// Whether `read_as` is itself read through `SwiftSchemaFit`: a fixed-width integer, an alias of
/// one or of an `Optional` of one, or one of the type parameters `parameters`, which such an
/// integer may fill.
fn fits(read_as: &str, parameters: &[String]) -> bool {
    FIXED_WIDTH.contains(&read_as)
        || parameters.iter().any(|named| named == read_as)
        || matches!(aliased(read_as, parameters), Some(Aliased::Alone))
}

/// The key and the value of the map `[Key: Value]`, or no key and the item of the list `[Item]`;
/// `None` for a type written in no brackets.
fn bracketed(read_as: &str) -> Option<(Option<&str>, &str)> {
    let listed = read_as.strip_prefix('[')?.strip_suffix(']')?;
    let mut depth = 0_i32;
    let colon = listed.char_indices().find_map(|(at, written)| {
        match written {
            '[' | '<' | '(' => depth += 1_i32,
            ']' | '>' | ')' => depth -= 1_i32,
            ':' if depth == 0_i32 => return Some(at),
            _ => {}
        }
        None
    });
    Some(colon.map_or((None, listed), |at| {
        (Some(&listed[..at]), listed[at + 1..].trim())
    }))
}

/// The read `{call}{read_as}{rest}`, with each fixed-width integer and each type parameter of
/// `parameters` that `read_as` holds read through `SwiftSchemaFit`, which refuses at the
/// integer's own coding path, and mapped back to `read_as`. A `Float` is read as a `Double` and
/// narrowed, as serde reads an `f32`: Foundation refuses one beyond its range, which serde reads
/// as an infinity of its sign. A `call` that ends in `decodeIfPresent(` answers an `Optional`, so
/// the mapping is written behind a `?`. The expression is written after a `try`.
pub(super) fn fitted(call: &str, read_as: &str, rest: &str, parameters: &[String]) -> String {
    let present = call.ends_with("decodeIfPresent(");
    if let Some((fit, unfit)) = fit_held(read_as, parameters).zip(unfit(read_as, parameters)) {
        let behind = if present { "?" } else { "" };
        return format!("{call}{fit}{rest}{behind}{unfit}");
    }
    let Some(wide) = widened(read_as) else {
        return format!("{call}{read_as}{rest}");
    };
    let answered = if present {
        format!("{read_as}?")
    } else {
        read_as.to_owned()
    };
    narrowed(&answered, &format!("{call}{wide}{rest}"))
}

/// `read_as` with each `Float` it holds, alone or in a list, as a map's value or under an
/// `Optional`, held as a `Double`; `None` where it holds none there.
fn widened(read_as: &str) -> Option<String> {
    if read_as == "Float" {
        return Some("Double".to_owned());
    }
    if let Some(held) = read_as.strip_suffix('?') {
        return widened(held).map(|wide| format!("{wide}?"));
    }
    let (key, held) = bracketed(read_as)?;
    let wide = widened(held)?;
    Some(key.map_or_else(|| format!("[{wide}]"), |named| format!("[{named}: {wide}]")))
}

/// `wide`, read as the type [`widened`] holds `read_as` as, narrowed back to `read_as`. The
/// closure is passed in parentheses, as [`unfit`]'s is.
fn narrowed(read_as: &str, wide: &str) -> String {
    if read_as == "Float" {
        return format!("Float({wide})");
    }
    if widened(read_as).is_none() {
        return wide.to_owned();
    }
    if let Some(held) = read_as.strip_suffix('?') {
        return format!("({wide}).map({{ {} }})", narrowed(held, "$0"));
    }
    let Some((key, held)) = bracketed(read_as) else {
        return wide.to_owned();
    };
    let mapping = if key.is_some() { "mapValues" } else { "map" };
    format!("({wide}).{mapping}({{ {} }})", narrowed(held, "$0"))
}

/// What a type holds whose flattened members [`flattened`] reads: the refusal of a value inside
/// one, placed at the holder, and a value serde refuses as text that is not the document worded
/// as a value of the wrong kind.
fn flattened_function() -> String {
    format!(
        "private static func swiftSchemaFlattened(_ refused: DecodingError, at \
         path: [CodingKey]) -> DecodingError {{ func placed(_ context: DecodingError.Context, keeping \
         kept: [CodingKey] = []) -> DecodingError.Context {{ DecodingError.Context(codingPath: path + \
         kept, debugDescription: context.debugDescription == \"{EXPECTED_VALUE}\" ? \
         \"{BUFFERED_WORDS}\" : context.debugDescription, underlyingError: context.underlyingError) \
         }}; if case .keyNotFound(let key, let context) = refused {{ return .keyNotFound(key, \
         placed(context)) }}; if case .typeMismatch(let type, let context) = refused {{ return \
         .typeMismatch(type, placed(context)) }}; if case .valueNotFound(let type, let context) = \
         refused {{ return .valueNotFound(type, placed(context)) }}; if case .dataCorrupted(let context) \
         = refused {{ return .dataCorrupted(placed(context, keeping: \
         context.debugDescription.hasPrefix(\"unknown field `\") ? Array(context.codingPath.suffix(1)) \
         : [])) }}; return refused }}"
    )
}

/// What a type holds whose internally tagged variants [`buffered`] reads: the refusal of a value
/// serde refuses as text that is not the document, placed at the holder as a value of the wrong
/// kind. Any other refusal is as it is.
fn buffered_function() -> String {
    format!(
        "private static func swiftSchemaBuffered(_ refused: DecodingError, at path: [CodingKey]) -> \
         DecodingError {{ if case .dataCorrupted(let context) = refused, context.debugDescription == \
         \"{EXPECTED_VALUE}\" {{ return .dataCorrupted(DecodingError.Context(codingPath: path, \
         debugDescription: \"{BUFFERED_WORDS}\", underlyingError: context.underlyingError)) }}; \
         return refused }}"
    )
}

/// `read`, the statements that read a flattened member, placing a refusal inside it at the
/// holder: serde reads such a member from the keys its holder buffers. An unknown key keeps its
/// own step, and a missing one its key. Statements that catch every refusal themselves are left
/// as they are, as Swift refuses a `catch` no error reaches.
pub(super) fn flattened(read: &str) -> String {
    if catches_every_refusal(read) {
        return read.to_owned();
    }
    format!(
        "do {{ {read} }} catch let refused as DecodingError {{ throw \
         Self.swiftSchemaFlattened(refused, at: decoder.codingPath) }}"
    )
}

/// `read`, the statements that read the members of an internally tagged variant, placing the
/// refusal [`buffered_function`] words at the holder, as serde reads them from the content it
/// buffers.
pub(super) fn buffered(read: &str) -> String {
    format!(
        "do {{ {read} }} catch let refused as DecodingError {{ throw \
         Self.swiftSchemaBuffered(refused, at: decoder.codingPath) }}"
    )
}

/// Whether `read` is one `do { ... } catch { ... }` whose `catch` throws nothing.
fn catches_every_refusal(read: &str) -> bool {
    let Some(body) = read.strip_prefix("do { ") else {
        return false;
    };
    let Some(after) = block_end(body).map(|end| &body[end..]) else {
        return false;
    };
    let Some(handled) = after.strip_prefix(" catch { ") else {
        return false;
    };
    block_end(handled).is_some_and(|end| {
        end == handled.len() && !handled.contains("throw") && !handled.contains("try ")
    })
}

/// The index just past the `}` that closes a block whose `{` was just read, in Swift statements;
/// a brace inside a string literal is no brace.
fn block_end(statements: &str) -> Option<usize> {
    let mut depth = 1_usize;
    let mut quoted = false;
    let mut escaped = false;
    for (at, written) in statements.char_indices() {
        match (quoted, escaped, written) {
            (true, true, _) => escaped = false,
            (true, false, '\\') => escaped = true,
            (_, false, '"') => quoted = !quoted,
            (false, _, '{') => depth += 1,
            (false, _, '}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(at + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// The helpers the statements of `read` name: [`FITTED`], [`flattened_function`] and [`buffered_function`].
pub(super) fn helpers_for(read: &str) -> String {
    let mut helpers = String::new();
    if read.contains("SwiftSchemaFit<") || read.contains("SwiftSchemaPayload") {
        helpers.push_str(FITTED);
        helpers.push_str("; ");
    }
    if read.contains("swiftSchemaFlattened(") {
        helpers.push_str(&flattened_function());
        helpers.push_str("; ");
    }
    if read.contains("swiftSchemaBuffered(") {
        helpers.push_str(&buffered_function());
        helpers.push_str("; ");
    }
    helpers
}

/// The key type the statements of `read` name, where they read a map entry by entry.
pub(super) fn any_key(read: &str) -> String {
    if read.contains("SwiftSchemaAnyKey.self") {
        format!("{ANY_KEY}; ")
    } else {
        String::new()
    }
}

/// The key type `written`, a type's `encode(to:)`, names, where `reads`, the type's reads as
/// [`both`] writes them, declare none.
pub(super) fn any_key_written(reads: &str, written: &str) -> String {
    if reads.starts_with(ANY_KEY) {
        String::new()
    } else {
        any_key(written)
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

/// The statements that throw for a key of the object `decoder` holds that is none of `keys`, as
/// serde does for a type that refuses a key beside its own. A type read as the whole payload a
/// holder hands it (`SwiftSchemaPayload.read`) is handed the keys of the members beside it, and
/// refuses none.
pub(super) fn refusing_unknown(keys: &[String]) -> String {
    let unlisted = if keys.is_empty() {
        String::new()
    } else {
        format!(
            "where !{}.contains(unknown.stringValue) ",
            preprocess::listed(keys)
        )
    };
    format!(
        "if !(decoder.codingPath.isEmpty && \
         decoder.userInfo[CodingUserInfoKey(rawValue: \"swiftSchemaFlat\")!] != nil) {{ \
         for unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys \
         {unlisted}{{ throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         decoder.codingPath + [unknown], debugDescription: \"unknown field \
         `\\(unknown.stringValue)`\")) }} }}; "
    )
}

/// The read of a type alone, declared under `reach`, where it has one that takes the keys the types
/// around it took.
fn alone(reach: &str) -> String {
    format!(
        "{reach} init(swiftSchemaUnjudged decoder: Decoder) throws {{ \
         try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }}"
    )
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
    let refused = written
        .closed
        .as_deref()
        .filter(|_| written.own.is_empty())
        .map(refusing_unknown);
    // An untagged enum writes the key type its readers name ahead of them.
    let key = if written.beside.contains("SwiftSchemaAnyKey.self") {
        String::new()
    } else {
        any_key(&format!("{read}{}", refused.as_deref().unwrap_or_default()))
    };
    let payload = if !written.own.is_empty() || read.contains("SwiftSchemaPayload") {
        format!("{}; ", preprocess::payload_for(&read))
    } else {
        String::new()
    };
    let taking = names_taken(body) || names_taken(written.beside);
    let reach = if written.sealed {
        "fileprivate"
    } else {
        "public"
    };
    let opened = format!(
        "{key}{payload}{twins}{}",
        helpers_for(&format!("{read}{payload}"))
    );
    if let Some(refusing) = refused {
        return format!(
            "{opened}{reach} {TAKING} {{ {body} }}; {reach} {UNJUDGED} {{ {refusing}\
             try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }}; {JUDGED}; "
        );
    }
    if written.own.is_empty() {
        return if taking {
            format!(
                "{opened}{reach} {TAKING} {{ {body} }}; {}; {JUDGED}; ",
                alone(reach)
            )
        } else {
            format!("{opened}{reach} {UNJUDGED} {{ {body} }}; {JUDGED}; ")
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
         {reach} {TAKING} {{ self = try SwiftSchemaPayload.read(SwiftSchemaRead.self, \
         from: {corrected}, at: decoder.codingPath).value }}; {}; {JUDGED}; ",
        alone(reach)
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
            "var {listed} = try {}; var {items}: {} = []; while !{listed}.isAtEnd {{ {read} }}; \
             {}{}",
            source.unkeyed(),
            written_type(def, hint),
            refusing_another_length(def, &listed, &items),
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

/// Whether a read of `def` holds an array to the length it was written with: a level written as
/// `[T; N]`, an alias of one written above, or the values of a map Swift keys natively that hold
/// one. A map under a key Swift wraps, and every type the emitter writes, holds its own.
pub(super) fn holds_a_length(def: &FieldDef) -> bool {
    (0..def.array_depth).any(|level| def.fixed_length_at(level).is_some())
        || through_alias(def).is_some()
        || matches!(
            &def.field_type,
            FieldDefType::Map(key, value)
                if !matches!(map_keys(key), MapKeys::Wrapped)
                    && holds_a_length(&arrayed((**value).clone()))
        )
}

/// The statements that read from `source` the value of `shaped`, which [`holds_a_length`], and hand
/// it to `sink`: each level written as `[T; N]` item by item, refused where it holds another number
/// of items, as serde refuses it, a map's entries one by one, and what holds no such level in one
/// call. `stem` is what a local of that call is named from.
pub(super) fn counted(
    shaped: &Shaped,
    stem: &str,
    source: &Source,
    sink: &dyn Fn(&str) -> String,
    locals: &mut Locals,
) -> String {
    let def = &shaped.def;
    if !holds_a_length(def) {
        return shaped.read_whole(&shaped.real_type(), stem, source, sink);
    }
    if let Some(target) = through_alias(def) {
        return counted(&target, stem, source, sink, locals);
    }
    if def.is_optional() {
        let (holds, present) = source.holding();
        let read = counted(&shaped.at(under_option(def)), stem, &present, sink, locals);
        return format!("if {holds} {{ {read} }} else {{ {} }}", sink("nil"));
    }
    if def.array_depth > 0 {
        let [listed, items] = locals.next_list();
        let item = shaped.at(item_of(def));
        let read = counted(
            &item,
            stem,
            &Source::Listed(listed.clone()),
            &|held| format!("{items}.append({held})"),
            locals,
        );
        return format!(
            "var {listed} = try {}; var {items}: {} = []; while !{listed}.isAtEnd {{ {read} }}; \
             {}{}",
            source.unkeyed(),
            shaped.real_type(),
            refusing_another_length(def, &listed, &items),
            sink(&items)
        );
    }
    let FieldDefType::Map(_, value) = &def.field_type else {
        return shaped.read_whole(&shaped.real_type(), stem, source, sink);
    };
    let value_hint = format!("{}Value", shaped.hint);
    let value_def = arrayed((**value).clone());
    let (_, value_shape) = swift_resolve(&value_def, &value_hint, &mut Vec::new());
    let entry = Shaped {
        def: value_def,
        hint: value_hint,
        shape: value_shape,
    };
    read_entries(
        value,
        &shaped.real_type(),
        source,
        false,
        &|under, put, nested| counted(&entry, stem, under, put, nested),
        sink,
        locals,
    )
}

/// The statement that refuses `items`, the items of the outer level of `def` read from `listed`,
/// where that level is written as `[T; N]` and they are not `N`, in the words serde refuses an
/// array of another length with: nothing for a level of any length.
fn refusing_another_length(def: &FieldDef, listed: &str, items: &str) -> String {
    def.array_depth
        .checked_sub(1)
        .and_then(|level| def.fixed_length_at(level))
        .map_or_else(String::new, |length| {
            format!(
                "guard {items}.count == {length} else {{ throw DecodingError.dataCorrupted(\
                 DecodingError.Context(codingPath: {listed}.codingPath, debugDescription: \
                 \"invalid length \\({items}.count), expected {length}\")) }}; "
            )
        })
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
        value,
        &written_type(def, hint),
        source,
        false,
        &|under, put, nested| unjudged(value, &value_hint, under, put, nested),
        sink,
        locals,
    )
}

/// The statement that stores `held` under `key` in the dictionary `entries`: through `updateValue`
/// where `value`, what the dictionary holds, is an optional, as assigning a `nil` through the
/// subscript removes the entry.
pub(super) fn store_entry(entries: &str, key: &str, held: &str, value: &FieldDef) -> String {
    if held_as_optional(value) {
        format!("_ = {entries}.updateValue({held}, forKey: {key})")
    } else {
        format!("{entries}[{key}] = {held}")
    }
}

/// The statements that read from `source` the entries of a map Swift keys natively by text
/// into a `written`, each value read under its key by `value`, and hand the map to `sink`. Where
/// `skips_taken`, no entry is read under a key the types around the holder took: a map flattened
/// beside another member is refused, so no member of the holder takes one.
pub(super) fn read_entries(
    entry_value: &FieldDef,
    written: &str,
    source: &Source,
    skips_taken: bool,
    value: &EntryRead<'_>,
    sink: &dyn Fn(&str) -> String,
    locals: &mut Locals,
) -> String {
    let [keyed, entries, entry] = locals.next_map();
    let named = format!("{entry}.stringValue");
    let under = Source::Keyed {
        container: keyed.clone(),
        key: entry.clone(),
        presence: Presence::Known,
    };
    let read = value(
        &under,
        &|held| store_entry(&entries, &named, held, entry_value),
        locals,
    );
    let skipped = if skips_taken {
        format!(" where !taken.contains({entry}.stringValue)")
    } else {
        String::new()
    };
    format!(
        "let {keyed} = try {}; var {entries}: {written} = [:]; \
         for {entry} in {keyed}.allKeys{skipped} {{ {read} }}; {}",
        source.keyed(),
        sink(&entries)
    )
}

/// `stem`, then `stem1`, `stem2`: the name of the local of a kind written after `nth` others.
fn numbered(stem: &str, nth: usize) -> String {
    if nth == 0 {
        stem.to_owned()
    } else {
        format!("{stem}{nth}")
    }
}

/// The Swift type the emitter declares a value of `def` as.
fn written_type(def: &FieldDef, hint: &str) -> String {
    swift_full_real_type(def, hint, &mut Vec::new())
}

#[cfg(test)]
mod tests;
