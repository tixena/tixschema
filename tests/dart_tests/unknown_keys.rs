//! The statements the Dart read of a strict class opens with, and the helper it reads each member
//! with.

/// The helper a class reads each member under its key with, which puts the key ahead of the path
/// a refusal carries.
pub const READ_AT: &str = "static Read _readAt<Read>(Map<String, dynamic>? object, Object key, \
     Read Function() read) { try { return read(); } catch (refused) { if (object != null && \
     !object.containsKey(key)) throw FormatException('missing field `$key`', <Object>[key]); if \
     (refused is TypeError) throw FormatException('$refused', <Object>[key]); if (refused is \
     FormatException) { final at = refused.source; throw FormatException(refused.message, \
     <Object>[key, if (at is List<Object>) ...at]); } rethrow; } }";

/// The helper a class reads the members it takes from content its holder buffers with, which words
/// a refusal of the shape of an externally tagged enum's value as a value of the wrong kind.
pub const READ_BUFFERED: &str = "static Read _readBuffered<Read>(Read Function() read) { try { \
     return read(); } on FormatException catch (refused) { if (refused.message != 'expected \
     value') rethrow; throw const FormatException('invalid type: expected string or map'); } }";

/// The helper a class reads a flattened member with, which names the holder for a refusal inside
/// it but for the key a `missing field` or an unknown key names.
pub const READ_FLATTENED: &str = "static Read _readFlattened<Read>(Read Function() read) { try { \
     return _readBuffered(read); } on FormatException catch (refused) { final at = refused.source; if (at is! \
     List<Object> || at.isEmpty) rethrow; final message = refused.message; throw \
     FormatException(message, <Object>[if (message.startsWith('missing field ') || \
     message.startsWith('Unknown key of a ')) at.last]); } }";

/// The statements that refuse a key of `json` that is none of `keys`, in the read of `class`.
pub fn refusing(class: &str, keys: &[&str]) -> String {
    refusing_in("json", class, keys)
}

/// The statements that refuse a key of the object `object` names that is none of `keys`.
pub fn refusing_in(object: &str, class: &str, keys: &[&str]) -> String {
    let listed: Vec<String> = keys.iter().map(|key| format!("'{key}'")).collect();
    format!(
        "for (final key in {object}.keys) {{ if (!const <String>{{{}}}.contains(key)) throw \
         FormatException('Unknown key of a {class}: $key', <Object>[key]); }} ",
        listed.join(", ")
    )
}

/// The statements a plain enum `named` reads its variant with from the payload `json` names: a
/// string, or an object of one key over `null`, whose key `arms` answers.
pub fn plain_enum_read(named: &str, json: &str, arms: &str) -> String {
    format!(
        "final key = switch ({json}) {{ final String text => text, final Map<String, dynamic> \
         object when object.length == 1 => object.keys.first, _ => throw const \
         FormatException('expected value') }}; final found = switch (key) {{ {arms} _ => throw \
         FormatException('Unknown {named}: ' + key) }}; if ({json} is Map && {json}.values.first \
         != null) throw FormatException('invalid type: expected unit', <Object>[key]); return \
         found;"
    )
}
