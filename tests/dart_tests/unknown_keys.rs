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
