//! The TypeScript types and Zod schemas of jiff's date and time types.

/// Writes the fraction ending `sent` as jiff writes it: in the fewest digits, and none for zeros.
#[cfg(any(test, feature = "zod"))]
const TRIMMED: &str =
    "return sent.replace(/\\.(\\d*?)0+$/, (_, kept) => kept ? `.${kept}` : \"\");";

pub fn get_civil_date_typescript_type() -> String {
    "string".to_owned()
}

pub fn get_civil_time_typescript_type() -> String {
    "string".to_owned()
}

pub fn get_civil_datetime_typescript_type() -> String {
    "string".to_owned()
}

pub fn get_timestamp_typescript_type() -> String {
    "Date".to_owned()
}

pub fn get_timestamp_number_typescript_type() -> String {
    "number".to_owned()
}

pub fn get_zoned_datetime_typescript_type() -> String {
    "string".to_owned()
}

#[cfg(any(test, feature = "zod"))]
pub fn get_civil_date_zod_schema() -> String {
    "z.iso.date()".to_owned()
}

/// The preprocessor also reads milliseconds since the start of the day, and writes the seconds
/// `HH:MM` leaves out and the fraction in the fewest digits, as jiff writes them.
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

/// The preprocessor writes the text in the form jiff writes: a `T` where jiff reads a space, the
/// seconds `HH:MM` leaves out, and the fraction in the fewest digits. The text is then held to
/// that form whole, which refuses the `Z` jiff refuses on a civil date and time.
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

/// Coerces a `Date`, an ISO string or epoch milliseconds to a `Date`, and refuses anything else
/// first: `z.coerce.date()` alone reads `null`, `true` and `false` as instants of 1970.
#[cfg(any(test, feature = "zod"))]
pub fn get_timestamp_native_zod_schema() -> String {
    "z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())".to_owned()
}

/// The epoch milliseconds of `as_number`, coerced inline so a generated module imports no helper:
/// `Date` → `getTime()`, ISO string → `Date.parse`, anything else to `z.number().int()`, which
/// refuses a fraction of a millisecond, as serde does.
#[cfg(any(test, feature = "zod"))]
pub fn get_timestamp_number_zod_schema() -> String {
    "z.preprocess((arg) => { if (arg instanceof Date) return arg.getTime(); \
if (typeof arg === \"string\") return Date.parse(arg); return arg; }, z.number().int())"
        .to_owned()
}

/// Holds the text whole to the RFC 9557 form jiff writes, zone named.
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
