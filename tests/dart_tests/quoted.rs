//! A name on the wire that holds a `$`, a quote or a backslash, in the Dart backend: each is
//! written between the quotes of a Dart string with the character escaped, so the file compiles and
//! the read and the write take the name serde writes. A double quote needs no escape there.

use super::quoted_models::{
    Dutied, Excised, Leveed, Levied, Mulcted, Tariffed, Tithed, Tolled, UNDER_AN_ALIAS, WRITTEN,
    dutied_dart, excised_dart, levied_dart, mulcted_dart, serde_written, tariffed_dart,
    tithed_dart, tolled_dart,
};

/// Asserts that `definition` holds each of `pieces`.
fn holds_each(definition: &str, pieces: &[&str]) {
    for piece in pieces {
        assert!(definition.contains(piece), "no `{piece}` in: {definition}");
    }
}

/// serde writes each model under the names the rows state, and reads each alias as the name it
/// writes.
#[test]
fn serde_writes_each_name_the_rows_state() {
    let leveed = Leveed {
        cost: "c".to_owned(),
    };
    let written = [
        serde_json::to_string(&Tariffed {
            backslash: "b".to_owned(),
            cost: "c".to_owned(),
            quoted: "s".to_owned(),
        }),
        serde_json::to_string(&Tolled::Bid { lot: 1 }),
        serde_json::to_string(&Tolled::Held("p".to_owned())),
        serde_json::to_string(&Tolled::Still),
        serde_json::to_string(&Levied::Bid { lot: 1 }),
        serde_json::to_string(&Levied::Held(leveed.clone())),
        serde_json::to_string(&Levied::Still),
        serde_json::to_string(&Dutied::Bid { lot: 1 }),
        serde_json::to_string(&Dutied::Held("h".to_owned())),
        serde_json::to_string(&Dutied::Still),
        serde_json::to_string(&Tithed {
            leveed,
            note: "n".to_owned(),
        }),
        serde_json::to_string(&Mulcted {
            note: "n".to_owned(),
            tolled: Tolled::Held("p".to_owned()),
        }),
        serde_json::to_string(&Excised::High),
        serde_json::to_string(&Excised::Low),
    ]
    .map(Result::unwrap);
    let rows = WRITTEN.map(|(_, json)| json);
    assert_eq!(
        written, rows,
        "serde no longer writes the names the rows state"
    );
    for (read_as, json) in WRITTEN {
        assert_eq!(
            serde_written(read_as, json),
            json,
            "serde no longer writes back {json} as it read it"
        );
    }
    for (read_as, json, back) in UNDER_AN_ALIAS {
        assert_eq!(
            serde_written(read_as, json),
            back,
            "serde no longer reads {json} as {back}"
        );
    }
}

/// A struct's members, its own tag key and its name, and a member's alias are each written with a
/// `$`, a quote and a backslash escaped, in the read, in the refusal of another tag and in `toJson`.
#[test]
fn a_structs_names_are_escaped_in_its_dart_text() {
    holds_each(
        &tariffed_dart::dart_definition(),
        &[
            r"final key = switch (entry.key) { 'cost\'s' => 'cost\$usd', final other => other };",
            r"if (!named.containsKey('ki\$nd')) throw FormatException('missing field `ki\$nd`');",
            r#"if (named['ki\$nd'] != 'Price\'s\$Tag\\') throw FormatException('expected tag "Price\'s\$Tag\\", found ' + jsonEncode(named['ki\$nd']));"#,
            r#"return Tariffed(backslash: named['back\\slash'] as String,cost: named['cost\$usd'] as String,quoted: named['say "hi"'] as String,);"#,
            r#"Map<String, dynamic> toJson() => { 'ki\$nd': 'Price\'s\$Tag\\','back\\slash': backslash,'cost\$usd': cost,'say "hi"': quoted, };"#,
        ],
    );
}

/// An enum's variants, under each tagging, its tag key, its content key and a variant's alias are
/// each written with a `$`, a quote and a backslash escaped, in the read and in `toJson`, and so are
/// a plain enum's values.
#[test]
fn an_enums_names_are_escaped_in_its_dart_text() {
    holds_each(
        &tolled_dart::dart_definition(),
        &[
            r#"case 'Calm"': return TolledStill();"#,
            r"case 'Bid\$Ask' || 'O\'Neil': return TolledBid.fromJsonUnjudged(map[tag]);",
            r"case 'C:\\Path': return TolledHeld(map['C:\\Path'] as String);",
            r"TolledBid(lot: json['lo\$t'] as int,);",
            r"@override dynamic toJson() => { 'Bid\$Ask': { 'lo\$t': lot, } };",
            r"@override dynamic toJson() => { 'C:\\Path': value };",
            r#"@override dynamic toJson() => 'Calm"';"#,
        ],
    );
    holds_each(
        &levied_dart::dart_definition(),
        &[
            r"final tag = json['ki\'nd'] as String;",
            r#"case 'He"ld': return LeviedHeld(Leveed.fromJsonUnjudged(json));"#,
            r"Map<String, dynamic> toJson() => { 'ki\'nd': 'Bid\$Ask','lo\$t': lot, };",
            r#"{ 'ki\'nd': 'He"ld', ...((value).toJson() as Map<String, dynamic>) }"#,
            r"@override dynamic toJson() => { 'ki\'nd': 'Calm\\' };",
        ],
    );
    holds_each(
        &dutied_dart::dart_definition(),
        &[
            r"final tag = json['ki\$nd'] as String;",
            r"case 'Bid\$Ask': return DutiedBid.fromJsonUnjudged(json['bo\'dy']);",
            r"case 'Calm\\': return json.containsKey('bo\'dy') ? throw",
            r"@override dynamic toJson() => { 'ki\$nd': 'Bid\$Ask', 'bo\'dy': { 'lot': lot, } };",
            r#"@override dynamic toJson() => { 'ki\$nd': 'He"ld', 'bo\'dy': value };"#,
        ],
    );
    holds_each(
        &excised_dart::dart_definition(),
        &[
            r"enum Excised { high('hi\'gh'), low('lo\$w');",
            r#"'hi\'gh' || 'to"p' => Excised.high,'lo\$w' => Excised.low,"#,
        ],
    );
}

/// A struct and an externally tagged enum flattened beside a member of their holder's own are
/// handed the object without that member's name, and the enum the one entry under one of its
/// names, each written escaped.
#[test]
fn a_flattened_members_names_are_escaped_in_its_holders_dart_text() {
    holds_each(
        &tithed_dart::dart_definition(),
        &[r"Tithed(leveed: Leveed.fromJsonUnjudged(({...json}..remove('no\'te'))),"],
    );
    holds_each(
        &mulcted_dart::dart_definition(),
        &[
            r#"whole.keys.where(const ['Bid\$Ask', 'O\'Neil', 'C:\\Path', 'Calm"'].contains)"#,
            r"Map<String, dynamic> toJson() => { 'no\'te': note,",
        ],
    );
}
