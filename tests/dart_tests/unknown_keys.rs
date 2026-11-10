//! The statements the Dart read of a strict class opens with.

/// The statements that refuse a key of `json` that is none of `keys`, in the read of `class`.
pub fn refusing(class: &str, keys: &[&str]) -> String {
    refusing_in("json", class, keys)
}

/// The statements that refuse a key of the object `object` names that is none of `keys`.
pub fn refusing_in(object: &str, class: &str, keys: &[&str]) -> String {
    let listed: Vec<String> = keys.iter().map(|key| format!("'{key}'")).collect();
    format!(
        "for (final key in {object}.keys) {{ if (!const <String>{{{}}}.contains(key)) throw \
         FormatException('Unknown key of a {class}: $key'); }} ",
        listed.join(", ")
    )
}
