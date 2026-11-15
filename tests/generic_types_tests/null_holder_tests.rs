//! The TypeScript valid type of a model that holds a type whose draft admits `null` admits it,
//! whether the held type is declared above the holder, below it, behind an alias or in another
//! crate, and for a type parameter filled with an `Option`, and still refuses a value built by hand.

use super::null_holder_models::{
    Boxed, ChoiceAbove, ChoiceAliased, ChoiceApart, ChoiceBelow, HeldAbove, HeldAliased, HeldApart,
    HeldBelow, HeldPlainAbove, HeldPlainBelow, HeldSlot, Plain, PlainBelow, ReadingAbove,
    ReadingBelow, Slot, reading_alias_schema,
};
use super::{CONSUMER_PROJECT, PREAMBLE, built_from, compiled, loaded_from};
use core::fmt::Write as _;
use std::fs;
use untagged_members_apart::ReadingApart;

/// A holder whose valid type admits `null`, and a value of its draft that is not `null`, built by
/// hand.
const NULLABLE_HOLDERS: [(&str, &str); 9] = [
    ("HeldAbove", "4"),
    ("HeldBelow", "4"),
    ("HeldAliased", "4"),
    ("HeldApart", "4"),
    ("HeldSlot", "4"),
    ("ChoiceAbove", "true"),
    ("ChoiceBelow", "true"),
    ("ChoiceAliased", "true"),
    ("ChoiceApart", "true"),
];

/// A holder over a model whose draft is never `null`.
const PLAIN_HOLDERS: [&str; 2] = ["HeldPlainAbove", "HeldPlainBelow"];

fn bundle() -> String {
    [
        PREAMBLE.to_owned(),
        ReadingAbove::ts_definition(),
        ReadingAbove::zod_schema(),
        ReadingBelow::ts_definition(),
        ReadingBelow::zod_schema(),
        ReadingApart::ts_definition(),
        ReadingApart::zod_schema(),
        reading_alias_schema::Schema::ts_definition(),
        reading_alias_schema::Schema::zod_schema(),
        Slot::<u32>::ts_definition(),
        Slot::<u32>::zod_schema(),
        Boxed::<u32>::ts_definition(),
        Boxed::<u32>::zod_schema(),
        HeldAbove::ts_definition(),
        HeldAbove::zod_schema(),
        HeldBelow::ts_definition(),
        HeldBelow::zod_schema(),
        HeldAliased::ts_definition(),
        HeldAliased::zod_schema(),
        HeldApart::ts_definition(),
        HeldApart::zod_schema(),
        HeldSlot::ts_definition(),
        HeldSlot::zod_schema(),
        ChoiceAbove::ts_definition(),
        ChoiceAbove::zod_schema(),
        ChoiceBelow::ts_definition(),
        ChoiceBelow::zod_schema(),
        ChoiceAliased::ts_definition(),
        ChoiceAliased::zod_schema(),
        ChoiceApart::ts_definition(),
        ChoiceApart::zod_schema(),
        Plain::ts_definition(),
        Plain::zod_schema(),
        PlainBelow::ts_definition(),
        PlainBelow::zod_schema(),
        HeldPlainAbove::ts_definition(),
        HeldPlainAbove::zod_schema(),
        HeldPlainBelow::ts_definition(),
        HeldPlainBelow::zod_schema(),
    ]
    .join("\n\n")
}

/// Every holder is the valid type `tsc --strict` takes `null` as, and a value built by hand is
/// refused; a holder over a model that is never `null` refuses `null`.
#[test]
fn type_check_the_valid_type_of_a_holder_admits_the_null_its_member_parses_to() {
    let Some(at) = built_from("null-holders", &bundle()) else {
        return;
    };
    let names = NULLABLE_HOLDERS
        .iter()
        .map(|(name, _)| *name)
        .chain(PLAIN_HOLDERS)
        .collect::<Vec<_>>();
    let schemas = names
        .iter()
        .map(|name| format!("{name}$Schema"))
        .collect::<Vec<_>>()
        .join(", ");
    let held_types = names.join(", ");
    let mut accepted = format!(
        "import {{ z }} from \"zod\";\nimport {{ {schemas}, Slot$SchemaFactory, Boxed$SchemaFactory }} from \"./lib/index.js\";\nimport type {{ {held_types}, Slot, Boxed }} from \"./lib/index.js\";\n"
    );
    for (name, _) in NULLABLE_HOLDERS {
        let _ = write!(
            accepted,
            "export const {name}Typed: {name} = null;\nexport const {name}Parsed: {name} = {name}$Schema.parse(null);\n"
        );
    }
    accepted.push_str(
        "export const slotTyped: Slot<number | null> = null;\n\
         export const slotParsed: Slot<number | null> = Slot$SchemaFactory(z.number().nullable()).parse(null);\n\
         export const slotCount: Slot<number | null> = Slot$SchemaFactory(z.number().nullable()).parse(3);\n\
         export const boxedTyped: Boxed<string | null> = null;\n\
         export const boxedParsed: Boxed<string | null> = Boxed$SchemaFactory(z.string().nullable()).parse(null);\n\
         export const plainAbove: HeldPlainAbove = HeldPlainAbove$Schema.parse({ label: \"a\" });\n\
         export const plainBelow: HeldPlainBelow = HeldPlainBelow$Schema.parse({ label: \"a\" });\n",
    );
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &accepted).unwrap();
    let (compiles, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    assert!(
        compiles,
        "TypeScript refuses a value the schema parses to:\n{said}\n{accepted}"
    );
    let mut refused = NULLABLE_HOLDERS
        .iter()
        .map(|&(name, value)| {
            (
                format!("import type {{ {name} }} from \"./lib/index.js\";\nexport const built: {name} = {value};\n"),
                format!("{value} built by hand as a {name}"),
            )
        })
        .collect::<Vec<_>>();
    refused.extend([
        (
            "import type { Slot } from \"./lib/index.js\";\nexport const built: Slot<number | null> = 4;\n".to_owned(),
            "a number built by hand as a slot".to_owned(),
        ),
        (
            "import type { Slot } from \"./lib/index.js\";\nexport const built: Slot<number> = null;\n".to_owned(),
            "`null` as a slot of numbers".to_owned(),
        ),
    ]);
    for name in PLAIN_HOLDERS {
        refused.push((
            format!("import type {{ {name} }} from \"./lib/index.js\";\nexport const built: {name} = null;\n"),
            format!("`null` as a {name}"),
        ));
        refused.push((
            format!("import type {{ {name} }} from \"./lib/index.js\";\nexport const built: {name} = {{ label: \"a\" }};\n"),
            format!("an object built by hand as a {name}"),
        ));
    }
    for (attempt, why) in refused {
        fs::write(at.join("consumer.ts"), &attempt).unwrap();
        let (taken, answer) = compiled(&at, "tsconfig.consumer.json").unwrap();
        assert!(!taken, "TypeScript takes {why} as a valid value:\n{answer}");
        assert!(
            answer.contains("TS2322"),
            "{why} is refused otherwise:\n{answer}"
        );
    }
    fs::remove_dir_all(&at).unwrap();
}

/// The schemas read `null` as before: a holder of a model that admits it returns `null`, and a
/// holder of one that never does refuses it.
#[test]
fn the_schema_of_a_holder_reads_null_as_its_member_does() {
    let driver = r#"import * as lib from "./lib/index.js";
for (const name of ["HeldAbove", "HeldBelow", "HeldAliased", "HeldApart", "ChoiceAbove", "ChoiceBelow", "ChoiceAliased", "ChoiceApart"]) {
  console.log(name, lib[name + "$Schema"].parse(null) === null);
}
for (const name of ["HeldPlainAbove", "HeldPlainBelow"]) {
  console.log(name, lib[name + "$Schema"].safeParse(null).success);
}
"#;
    let Some((printed, failed)) = loaded_from("null-holders-read", &bundle(), driver) else {
        return;
    };
    let mut expected = String::new();
    for name in [
        "HeldAbove",
        "HeldBelow",
        "HeldAliased",
        "HeldApart",
        "ChoiceAbove",
        "ChoiceBelow",
        "ChoiceAliased",
        "ChoiceApart",
    ] {
        let _ = writeln!(expected, "{name} true");
    }
    for name in PLAIN_HOLDERS {
        let _ = writeln!(expected, "{name} false");
    }
    assert_eq!(
        printed,
        expected.trim_end(),
        "the schemas read `null` otherwise:\n{failed}"
    );
}
