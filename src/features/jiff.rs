//! jiff date/time type support.

/// The statement of a preprocessor that writes the fraction ending `sent` in the fewest digits it
/// needs, as jiff writes it, and no fraction for one of zeros alone.
#[cfg(any(test, feature = "zod"))]
const TRIMMED: &str =
    "return sent.replace(/\\.(\\d*?)0+$/, (_, kept) => kept ? `.${kept}` : \"\");";

/// Generates TypeScript type name for `civil::Date`.
pub fn get_civil_date_typescript_type() -> String {
    "string".to_owned()
}

/// Generates TypeScript type name for `civil::Time`.
pub fn get_civil_time_typescript_type() -> String {
    "string".to_owned()
}

/// Generates TypeScript type name for `civil::DateTime`.
pub fn get_civil_datetime_typescript_type() -> String {
    "string".to_owned()
}

/// Generates TypeScript type name for a `Timestamp` (native `Date` default).
pub fn get_timestamp_typescript_type() -> String {
    "Date".to_owned()
}

/// Generates TypeScript type name for a `Timestamp` rendered with the
/// `#[model_schema_prop(as_number)]` opt-out (epoch milliseconds).
pub fn get_timestamp_number_typescript_type() -> String {
    "number".to_owned()
}

/// Generates TypeScript type name for `ZonedDateTime`, held as the one text jiff's serde
/// writes, zone included.
pub fn get_zoned_datetime_typescript_type() -> String {
    "string".to_owned()
}

/// Generates Zod schema for `civil::Date` (date validation).
/// Uses Zod v4's `z.iso.date()` syntax (`z.string().date()` is deprecated).
#[cfg(any(test, feature = "zod"))]
pub fn get_civil_date_zod_schema() -> String {
    "z.iso.date()".to_owned()
}

/// Generates Zod schema for `civil::Time` (time validation).
/// Stays a TS `string` (`z.iso.time()`) but the inline preprocessor also accepts
/// millis-since-start-of-day, converting it to an `HH:MM:SS` string before validation, writes the
/// seconds `HH:MM` leaves out, and the fraction in the fewest digits it needs, as jiff writes
/// them. `z.string().time()` is deprecated.
#[cfg(any(test, feature = "zod"))]
pub fn get_civil_time_zod_schema() -> String {
    format!(
        "z.preprocess((arg) => {{ if (typeof arg === \"number\") {{ const s = Math.floor(arg / \
         1000); const hh = String(Math.floor(s / 3600)).padStart(2, \"0\"); const mm = \
         String(Math.floor((s % 3600) / 60)).padStart(2, \"0\"); const ss = String(s % \
         60).padStart(2, \"0\"); return `${{hh}}:${{mm}}:${{ss}}`; }} if (typeof arg !== \
         \"string\") return arg; const sent = /^\\d{{2}}:\\d{{2}}$/.test(arg) ? `${{arg}}:00` : arg; \
         {TRIMMED} }}, z.iso.time())"
    )
}

/// Generates Zod schema for `civil::DateTime` (local date and time validation). The inline
/// preprocessor writes a text in the form jiff writes: a `T` where jiff reads a space, the seconds
/// `HH:MM` leaves out, and the fraction in the fewest digits it needs. The text is then held to
/// that form whole, which refuses the `Z` jiff refuses on a civil date and time, and which
/// `z.iso.datetime({ local: true })` reads.
#[cfg(feature = "zod")]
pub fn get_civil_datetime_zod_schema() -> String {
    use crate::features::text_forms::{DATE, TIME_SENT};
    format!(
        "z.preprocess((arg) => {{ if (typeof arg !== \"string\") return arg; const read = \
         /^(\\d{{4}}-\\d{{2}}-\\d{{2}})[T ](\\d{{2}}:\\d{{2}})(:\\d{{2}}(?:\\.\\d+)?)?$/.exec(arg); if \
         (!read) return arg; const sent = `${{read[1]}}T${{read[2]}}${{read[3] ?? \":00\"}}`; \
         {TRIMMED} }}, z.string().regex(/{}/))",
        regex_literal(&format!("{DATE}T{TIME_SENT}"))
    )
}

/// Generates Zod schema for `Timestamp` rendered as a native TS `Date` (default).
/// A `Date` instance, an ISO string or epoch milliseconds is coerced to a `Date`, which a BSON
/// `Date` is written from (`MongoDB` TTL). Anything else is refused before the coercion, because
/// `z.coerce.date()` alone reads `null`, `true` and `false` as instants of 1970.
#[cfg(any(test, feature = "zod"))]
pub fn get_timestamp_native_zod_schema() -> String {
    "z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())".to_owned()
}

/// Generates Zod schema for a `Timestamp` rendered with the
/// `#[model_schema_prop(as_number)]` opt-out (epoch milliseconds).
///
/// A self-contained inline coercer: `Date` → `getTime()`, ISO string → `Date.parse`,
/// otherwise passed through to `z.number().int()`, which refuses a fraction of a millisecond, as
/// serde does. No imported helper, so generated modules remain self-contained.
#[cfg(any(test, feature = "zod"))]
pub fn get_timestamp_number_zod_schema() -> String {
    "z.preprocess((arg) => { if (arg instanceof Date) return arg.getTime(); \
if (typeof arg === \"string\") return Date.parse(arg); return arg; }, z.number().int())"
        .to_owned()
}

/// Generates Zod schema for `ZonedDateTime`, held as the one text jiff's serde writes, zone
/// included. The text is validated whole against the RFC 9557 form jiff writes.
#[cfg(feature = "zod")]
pub fn get_zoned_datetime_zod_schema() -> String {
    use crate::features::text_forms::zoned_text;
    format!("z.string().regex(/{}/)", regex_literal(&zoned_text()))
}

/// `form`, held whole, as a JavaScript regex literal writes it between its `/` delimiters.
#[cfg(feature = "zod")]
fn regex_literal(form: &str) -> String {
    use crate::utils::{escape_js_regex_literal, js_spelling};
    // The forms are written in constructs both engines read, which the spelling never refuses.
    escape_js_regex_literal(&js_spelling(&format!("^(?:{form})$")).unwrap())
}

#[cfg(test)]
mod tests;
