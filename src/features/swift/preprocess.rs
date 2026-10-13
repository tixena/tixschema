//! `preprocess` in the Swift read: the functions a type and its members name, each called on a
//! payload ahead of the value read from it. A type that names one on itself reads the keys the
//! types around it did not take, and so does a type that reads one from its own decoder.

use core::cell::RefCell;
use std::collections::HashSet;

use super::read::{Locals, Source, unjudged};
use super::{swift_member, swift_upper_camel};
use crate::bound_message::swift_literal_text;
use crate::features::model_schema_prop::FunctionName;
use crate::field_type::{FieldDef, FieldDefType};

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
    /// Whether `validate()` runs the `validate()` of a value the member holds.
    pub walked: bool,
}

/// How a member is read where `validate()` walks nothing it holds.
pub(super) enum Plain<'member> {
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
/// so each model it holds is read alone and judged with its holder.
pub(super) fn member(
    member: &Member<'_>,
    source: &Source,
    sink: &dyn Fn(&str) -> String,
    twins: &mut Vec<String>,
) -> String {
    let stem = member.stem;
    let (handed, path) = source.payload(stem, member.keys);
    let corrected = called(member.functions, &format!("{stem}Payload"));
    let read = |read_as: &str| {
        format!("try SwiftSchemaPayload.read({read_as}.self, from: {corrected}, at: {path})")
    };
    let value = if member.walked {
        let twin = format!("SwiftSchemaRead{}", swift_upper_camel(stem));
        let statements = unjudged(
            member.def,
            member.hint,
            &Source::Decoder,
            &|held| format!("self.value = {held}"),
            &mut Locals::default(),
        );
        twins.push(format!(
            "private struct {twin}: Decodable {{ let value: {}; \
             init(from decoder: Decoder) throws {{ {statements} }} }}",
            member.real_type
        ));
        sink(&format!("{}.value", read(&twin)))
    } else {
        match member.plain {
            Plain::Declared => sink(&read(member.real_type)),
            Plain::Wire { convert, wire } => {
                let local = format!("wire{}", swift_upper_camel(stem));
                format!("let {local} = {}; {}", read(wire), sink(&convert(&local)))
            }
        }
    };
    let read_where_held = format!("{handed}{value}");
    match source.absent_as_nil() {
        Some(holds_key) => format!(
            "if {holds_key} {{ {read_where_held} }} else {{ {} }}",
            sink("nil")
        ),
        None => read_where_held,
    }
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
