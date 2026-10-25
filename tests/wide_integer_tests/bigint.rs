//! A `u64` and an `i64` in an example or in a default are bigint literals in the Zod source: the
//! text of each position, and the names `::core::any::type_name` answers that tell the writer
//! which integers are wide.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::any::type_name;

use super::bigint_models::{
    Defaulted, EitherMany, EitherOne, Entry, Extremes, FarLines, FarPlain, Ledger, Lines, Narrow,
    NoLines, Positions,
};
#[cfg(feature = "chrono")]
use super::bigint_models::{Hooked, Stamped};
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
use super::bigint_u64_models::{AccountNumber, Greatest, Keyed, Sizes, UnsignedDefault};

/// Asserts that `zod` holds `expected`.
#[cfg(feature = "zod")]
fn holds(zod: &str, expected: &str) {
    assert!(zod.contains(expected), "Missing: {expected}\nGot: {zod}");
}

/// The names the writer reads the declared type from.
#[test]
fn the_compiler_names_an_integer_and_the_pointers_to_it_as_the_writer_reads_them() {
    assert_eq!(type_name::<i64>(), "i64");
    assert_eq!(type_name::<u64>(), "u64");
    assert_eq!(type_name::<&i64>(), "&i64");
    assert_eq!(type_name::<&&mut i64>(), "&&mut i64");
    assert_eq!(type_name::<Box<i64>>(), "alloc::boxed::Box<i64>");
    assert_eq!(type_name::<Rc<i64>>(), "alloc::rc::Rc<i64>");
    assert_eq!(type_name::<Arc<i64>>(), "alloc::sync::Arc<i64>");
    assert_eq!(type_name::<usize>(), "usize");
    assert_eq!(type_name::<isize>(), "isize");
}

#[cfg(feature = "zod")]
#[test]
fn an_i64_in_an_example_and_a_default_is_a_bigint_literal() {
    let ledger = Ledger::zod_schema();
    holds(
        &ledger,
        r#"examples: [{"carried":9007199254740993n,"count":3,"signed":12n}]"#,
    );
    holds(&ledger, ".prefault(9007199254740993n)");
    holds(&ledger, "count: z.number().int()");
}

#[cfg(feature = "zod")]
#[test]
fn the_examples_a_type_answers_stay_json_values() {
    assert_eq!(
        Ledger::schema_example().to_string(),
        r#"{"carried":9007199254740993,"count":3,"signed":12}"#
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_bounds_of_an_i64_are_bigint_literals() {
    holds(
        &Extremes::zod_schema(),
        r#"examples: [{"greatest":9223372036854775807n,"least":-9223372036854775808n}]"#,
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_list_of_an_i64_is_a_list_of_bigint_literals() {
    let lines = Lines::zod_schema();
    holds(&lines, r#"examples: [{"lines":[12n,240n]}]"#);
    holds(&lines, ".prefault([12n,240n])");
    let no_lines = NoLines::zod_schema();
    holds(&no_lines, r#"examples: [{"lines":[]}]"#);
    holds(&no_lines, ".prefault([])");
    holds(
        &FarLines::zod_schema(),
        r#"examples: [{"lines":[12n,9007199254740993n]}]"#,
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_number_that_is_not_a_u64_or_an_i64_is_written_as_it_was() {
    holds(
        &Narrow::zod_schema(),
        r#"examples: [{"byte":7,"half":1.5,"whole":2.25,"wide":4000000000}]"#,
    );
    let defaulted = Defaulted::zod_schema();
    holds(&defaulted, ".prefault(7)");
    holds(&defaulted, ".prefault(1.5)");
    holds(&defaulted, ".prefault([12n,240n])");
}

#[cfg(all(feature = "zod", feature = "chrono"))]
#[test]
fn a_date_stays_a_number_under_as_number_and_under_a_function_of_its_authors() {
    holds(
        &Stamped::zod_schema(),
        r#"examples: [{"at":1700000000000}]"#,
    );
    holds(
        &Hooked::zod_schema(),
        r#"examples: [{"at":1700000000000,"text":"123"}]"#,
    );
}

#[cfg(feature = "zod")]
#[test]
fn an_i64_is_a_bigint_literal_in_each_position_serde_writes_one() {
    let expected = concat!(
        r#"examples: [{"adjacent":{"t":"Held","c":9n},"boxed":10n,"counted":14n,"deep":13n,"#,
        r#""keyed":{"a":1n},"maybe":5n,"named":{"Named":{"n":5n}},"pair":{"Pair":[3n,4]},"#,
        r#""plain":8,"shared":15n,"tag":6n,"wrapped":{"Wrapped":2n}}]"#
    );
    holds(&Positions::zod_schema(), expected);
}

#[cfg(feature = "zod")]
#[test]
fn an_untagged_enum_writes_its_i64_by_the_value_it_holds() {
    holds(
        &EitherMany::zod_schema(),
        "examples: [[12n,9007199254740993n]]",
    );
    holds(&EitherOne::zod_schema(), "examples: [240]");
    holds(&FarPlain::zod_schema(), "examples: [9007199254740993n]");
}

#[cfg(feature = "zod")]
#[test]
fn an_i64_default_in_a_variant_is_a_bigint_literal() {
    let entry = Entry::zod_schema();
    holds(&entry, ".prefault(9007199254740993n)");
    holds(&entry, ".prefault([])");
}

#[cfg(all(feature = "zod", not(any(feature = "swift", feature = "kotlin"))))]
#[test]
fn a_u64_is_a_bigint_literal_in_an_example_and_a_default() {
    holds(
        &AccountNumber::zod_schema(),
        r#"examples: [{"numbers":[12n,9007199254740993n],"signed":9007199254740993n,"unsigned":9007199254740993n}]"#,
    );
    holds(
        &Greatest::zod_schema(),
        r#"examples: [{"top":18446744073709551615n}]"#,
    );
    holds(
        &UnsignedDefault::zod_schema(),
        ".prefault(9007199254740993n)",
    );
}

#[cfg(all(feature = "zod", not(any(feature = "swift", feature = "kotlin"))))]
#[test]
fn a_usize_and_an_isize_are_numbers_until_a_number_cannot_hold_them() {
    holds(
        &Sizes::zod_schema(),
        r#"examples: [{"count":3,"far":9007199254740993n,"step":-3}]"#,
    );
}

#[cfg(all(feature = "zod", not(any(feature = "swift", feature = "kotlin"))))]
#[test]
fn the_key_of_a_map_keyed_by_a_u64_stays_a_string() {
    holds(&Keyed::zod_schema(), r#"examples: [{"keyed":{"42":1}}]"#);
}
