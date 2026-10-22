//! Chrono date/time type support.

/// Generates TypeScript type name for `NaiveDate`.
pub fn get_naive_date_typescript_type() -> String {
    "string".to_owned()
}

/// Generates TypeScript type name for `NaiveTime`.
pub fn get_naive_time_typescript_type() -> String {
    "string".to_owned()
}

/// Generates TypeScript type name for `NaiveDateTime`.
pub fn get_naive_datetime_typescript_type() -> String {
    "string".to_owned()
}

/// Generates TypeScript type name for `DateTime<Tz>` (native `Date` default).
pub fn get_datetime_typescript_type() -> String {
    "Date".to_owned()
}

/// Generates TypeScript type name for a `DateTime<Tz>` rendered with the
/// `#[model_schema_prop(as_number)]` opt-out (epoch milliseconds).
pub fn get_datetime_number_typescript_type() -> String {
    "number".to_owned()
}

/// Generates Zod schema for `NaiveDate` (date validation).
/// Uses Zod v4's `z.iso.date()` syntax (`z.string().date()` is deprecated).
#[cfg(any(test, feature = "zod"))]
pub fn get_naive_date_zod_schema() -> String {
    "z.iso.date()".to_owned()
}

/// Generates Zod schema for `NaiveTime` (time validation).
/// Stays a TS `string` (`z.iso.time()`) but the inline preprocessor also accepts
/// millis-since-start-of-day, converting it to an `HH:MM:SS` string before validation, and
/// writes the seconds `HH:MM` leaves out, as serde writes them.
/// `z.string().time()` is deprecated.
#[cfg(any(test, feature = "zod"))]
pub fn get_naive_time_zod_schema() -> String {
    "z.preprocess((arg) => { if (typeof arg === \"number\") { const s = Math.floor(arg / 1000); \
const hh = String(Math.floor(s / 3600)).padStart(2, \"0\"); \
const mm = String(Math.floor((s % 3600) / 60)).padStart(2, \"0\"); \
const ss = String(s % 60).padStart(2, \"0\"); return `${hh}:${mm}:${ss}`; } \
if (typeof arg === \"string\" && /^\\d{2}:\\d{2}$/.test(arg)) return `${arg}:00`; return arg; }, \
z.iso.time())"
        .to_owned()
}

/// Generates Zod schema for `NaiveDateTime` (local datetime validation).
/// Uses Zod v4's `z.iso.datetime()` syntax (`z.string().datetime()` is deprecated). The inline
/// preprocessor writes a text in the form serde reads: it drops a `Z`, which serde refuses on a
/// date and time with no zone, and writes the seconds `HH:MM` leaves out.
#[cfg(any(test, feature = "zod"))]
pub fn get_naive_datetime_zod_schema() -> String {
    "z.preprocess((arg) => { if (typeof arg !== \"string\") return arg; \
const read = /^(\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2})(:\\d{2}(?:\\.\\d+)?)?Z?$/.exec(arg); \
return read ? `${read[1]}${read[2] ?? \":00\"}` : arg; }, z.iso.datetime({ local: true }))"
        .to_owned()
}

/// Generates Zod schema for `DateTime<Tz>` rendered as a native TS `Date` (default).
/// A `Date` instance, an ISO string or epoch milliseconds is coerced to a `Date`, which a BSON
/// `Date` is written from (`MongoDB` TTL). Anything else is refused before the coercion, because
/// `z.coerce.date()` alone reads `null`, `true` and `false` as instants of 1970.
#[cfg(any(test, feature = "zod"))]
pub fn get_datetime_native_zod_schema() -> String {
    "z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())".to_owned()
}

/// Generates Zod schema for a `DateTime<Tz>` rendered with the
/// `#[model_schema_prop(as_number)]` opt-out (epoch milliseconds).
///
/// A self-contained inline coercer: `Date` → `getTime()`, ISO string → `Date.parse`,
/// otherwise passed through to `z.number().int()`, which refuses a fraction of a millisecond, as
/// serde does. No imported helper, so generated modules remain self-contained.
#[cfg(any(test, feature = "zod"))]
pub fn get_datetime_number_zod_schema() -> String {
    "z.preprocess((arg) => { if (arg instanceof Date) return arg.getTime(); \
if (typeof arg === \"string\") return Date.parse(arg); return arg; }, z.number().int())"
        .to_owned()
}

#[cfg(test)]
mod tests;
