//! Unit tests of the jiff support: the TypeScript type and the Zod schema each jiff type
//! renders as.

use super::*;

#[test]
fn each_jiff_type_names_its_typescript_type() {
    assert_eq!(get_civil_date_typescript_type(), "string");
    assert_eq!(get_civil_time_typescript_type(), "string");
    assert_eq!(get_civil_datetime_typescript_type(), "string");
    assert_eq!(get_timestamp_typescript_type(), "Date");
    assert_eq!(get_timestamp_number_typescript_type(), "number");
    assert_eq!(get_zoned_datetime_typescript_type(), "string");
}

#[test]
fn a_civil_date_and_a_timestamp_read_as_a_chrono_date_and_instant_do() {
    assert_eq!(get_civil_date_zod_schema(), "z.iso.date()");
    assert_eq!(
        get_timestamp_native_zod_schema(),
        "z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())"
    );
    assert_eq!(
        get_timestamp_number_zod_schema(),
        "z.preprocess((arg) => { if (arg instanceof Date) return arg.getTime(); if (typeof arg \
         === \"string\") return Date.parse(arg); return arg; }, z.number().int())"
    );
}

#[test]
fn a_civil_time_writes_its_seconds_and_the_fewest_digits_of_its_fraction() {
    assert_eq!(
        get_civil_time_zod_schema(),
        "z.preprocess((arg) => { if (typeof arg === \"number\") { const s = Math.floor(arg / 1000); const hh = String(Math.floor(s / 3600)).padStart(2, \"0\"); const mm = String(Math.floor((s % 3600) / 60)).padStart(2, \"0\"); const ss = String(s % 60).padStart(2, \"0\"); return `${hh}:${mm}:${ss}`; } if (typeof arg !== \"string\") return arg; const sent = /^\\d{2}:\\d{2}$/.test(arg) ? `${arg}:00` : arg; return sent.replace(/\\.(\\d*?)0+$/, (_, kept) => kept ? `.${kept}` : \"\"); }, z.iso.time())"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_civil_date_and_time_writes_a_t_its_seconds_and_the_fewest_digits_of_its_fraction() {
    assert_eq!(
        get_civil_datetime_zod_schema(),
        "z.preprocess((arg) => { if (typeof arg !== \"string\") return arg; const read = /^(\\d{4}-\\d{2}-\\d{2})[T ](\\d{2}:\\d{2})(:\\d{2}(?:\\.\\d+)?)?$/.exec(arg); if (!read) return arg; const sent = `${read[1]}T${read[2]}${read[3] ?? \":00\"}`; return sent.replace(/\\.(\\d*?)0+$/, (_, kept) => kept ? `.${kept}` : \"\"); }, z.string().regex(/^(?:(?:(?:[0-9][0-9][2468][048]|[0-9][0-9][13579][26]|[0-9][0-9]0[48]|[02468][048]00|[13579][26]00)-02-29|[0-9]{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12][0-9]|3[01])|(?:0[469]|11)-(?:0[1-9]|[12][0-9]|30)|(?:02)-(?:0[1-9]|1[0-9]|2[0-8])))T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\\.[0-9]*[1-9])?)$/))"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_zoned_date_and_time_is_held_to_the_text_jiff_writes() {
    assert_eq!(
        get_zoned_datetime_zod_schema(),
        "z.string().regex(/^(?:(?:(?:[0-9][0-9][2468][048]|[0-9][0-9][13579][26]|[0-9][0-9]0[48]|[02468][048]00|[13579][26]00)-02-29|[0-9]{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12][0-9]|3[01])|(?:0[469]|11)-(?:0[1-9]|[12][0-9]|30)|(?:02)-(?:0[1-9]|1[0-9]|2[0-8])))T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\\.[0-9]*[1-9])?[+-](?:[01][0-9]|2[0-3]):[0-5][0-9]\\[[A-Za-z0-9_\\/+:-]+\\])$/)"
    );
}
