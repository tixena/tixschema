//! Unit tests of the chrono support: the TypeScript type and the Zod schema each chrono type
//! renders as.

use super::*;

#[test]
fn test_typescript_types() {
    assert_eq!(get_naive_date_typescript_type(), "string");
    assert_eq!(get_naive_time_typescript_type(), "string");
    assert_eq!(get_naive_datetime_typescript_type(), "string");
    assert_eq!(get_datetime_typescript_type(), "Date");
    assert_eq!(get_datetime_number_typescript_type(), "number");
}

#[test]
fn test_zod_schemas() {
    assert_eq!(get_naive_date_zod_schema(), "z.iso.date()");
    assert_eq!(
        get_datetime_native_zod_schema(),
        "z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())"
    );
}

#[test]
fn test_datetime_number_zod_schema_is_self_contained_arrow() {
    let schema = get_datetime_number_zod_schema();
    assert!(schema.starts_with("z.preprocess((arg) =>"));
    assert!(schema.contains("arg instanceof Date) return arg.getTime();"));
    assert!(schema.contains("typeof arg === \"string\") return Date.parse(arg);"));
    assert!(schema.ends_with("z.number().int())"));
    assert!(!schema.contains("z.iso.datetime"));
}

#[test]
fn test_naive_time_zod_schema_accepts_millis() {
    let schema = get_naive_time_zod_schema();
    assert!(schema.starts_with("z.preprocess((arg) =>"));
    assert!(schema.contains("typeof arg === \"number\""));
    assert!(schema.contains("Math.floor(arg / 1000)"));
    assert!(schema.contains("padStart(2, \"0\")"));
    assert!(schema.contains("/^\\d{2}:\\d{2}$/.test(arg)) return `${arg}:00`;"));
    assert!(schema.ends_with("z.iso.time())"));
}

#[test]
fn test_naive_datetime_zod_schema_writes_the_form_serde_reads() {
    let schema = get_naive_datetime_zod_schema();
    assert!(schema.starts_with("z.preprocess((arg) =>"));
    assert!(schema.contains("(:\\d{2}(?:\\.\\d+)?)?Z?$/.exec(arg)"));
    assert!(schema.contains("`${read[1]}${read[2] ?? \":00\"}`"));
    assert!(schema.ends_with("z.iso.datetime({ local: true }))"));
}
