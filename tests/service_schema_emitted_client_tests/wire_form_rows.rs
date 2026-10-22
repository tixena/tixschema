//! What each emitted client is handed of [`WIRE_FORMS`], and what it prints: the payload it sends
//! of each row, or `refused`, and whether it reads each payload any client sends.

use core::fmt::Write as _;

use super::wire_forms_models::{Places, Sends, WIRE_FORMS, WireFormRow, places};

/// What a model holds, as each client's own type holds it.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Held {
    /// A `bool`, the literal one among them.
    Flag,
    /// A `DateTime<Utc>`.
    Instant,
    /// A `DateTime<Utc>` under `as_number`.
    Millis,
    /// A number, read from its text by the client's own parse of the Rust type it is named by.
    Number(&'static str),
    /// A plain enum.
    Slot,
    /// A text: a `char`, a `String`, the literal one and the chrono dates and times.
    Text,
}

/// What the model `held_by` holds.
pub fn held(held_by: &str) -> Held {
    match held_by {
        "ConveyedFlag" | "ConveyedLiteralFlag" => Held::Flag,
        "ConveyedInstant" => Held::Instant,
        "ConveyedMillis" => Held::Millis,
        "ConveyedSlotted" => Held::Slot,
        "ConveyedU8" | "ConveyedLiteralCount" => Held::Number("u8"),
        "ConveyedU16" => Held::Number("u16"),
        "ConveyedU32" => Held::Number("u32"),
        "ConveyedI8" => Held::Number("i8"),
        "ConveyedI16" => Held::Number("i16"),
        "ConveyedI32" => Held::Number("i32"),
        "ConveyedI64" => Held::Number("i64"),
        "ConveyedIsize" => Held::Number("isize"),
        "ConveyedF32" => Held::Number("f32"),
        "ConveyedF64" => Held::Number("f64"),
        _ => Held::Text,
    }
}

/// The rows a client is handed, those its column states it holds, each with what it prints.
pub fn handed(column: fn(&WireFormRow) -> Sends) -> Vec<(&'static WireFormRow, String)> {
    WIRE_FORMS
        .iter()
        .filter(|row| !matches!(column(row), Sends::Unheld | Sends::Unemitted))
        .map(|row| {
            let printed = row
                .payload(column(row))
                .unwrap_or_else(|| "refused".to_owned());
            (row, printed)
        })
        .collect()
}

/// What the client prints of the rows it is handed, one line each.
pub fn printed(rows: &[(&WireFormRow, String)]) -> String {
    rows.iter().fold(String::new(), |mut written, (_, line)| {
        let _ = writeln!(written, "{line}");
        written
    })
}

/// Every payload any client sends, with the model it is read as, of the models `client` has a
/// type for.
pub fn sent_by_any(client: fn(&WireFormRow) -> Sends) -> Vec<(&'static str, String)> {
    WIRE_FORMS
        .iter()
        .filter(|row| client(row) != Sends::Unemitted)
        .flat_map(|row| row.sent().into_iter().map(|sent| (row.held_by, sent)))
        .collect()
}

/// The model `held_by` holding `value` in each of its places, written by `named`, `keyed` naming
/// the map of `key` the client writes, `listed` the list of `value`.
pub fn constructed(
    held_by: &str,
    named: &str,
    keyed: &dyn Fn(&str) -> String,
    listed: &dyn Fn(&str) -> String,
    value: &str,
    key: &str,
) -> String {
    match places(held_by) {
        Places::Keyed => format!(
            "{held_by}({named}{}, {named_value}{value}, {named_values}{})",
            keyed(key),
            listed(value),
            named = named.replace("NAME", "keyed"),
            named_value = named.replace("NAME", "value"),
            named_values = named.replace("NAME", "values"),
        ),
        Places::Listed => format!(
            "{held_by}({}{value}, {}{})",
            named.replace("NAME", "value"),
            named.replace("NAME", "values"),
            listed(value)
        ),
        Places::Lone => format!("{held_by}({}{value})", named.replace("NAME", "value")),
    }
}
