//! A name serde reads beside the one it writes, in the Dart read: a variant is read under each of
//! its aliases, under each tagging, flattened and in a plain enum, a member under each of its
//! aliases, in a struct, flattened and in a variant, and each is written under the name serde
//! writes.

use super::aliased_models::{
    careening_dart, fairleading_dart, fothering_dart, gybing_dart, hawsing_dart, lulling_dart,
    scending_dart, surging_dart, swell_dart, veering_dart,
};
use super::flattened_aliased_models::riding_dart;

/// What copies a payload's entries into `named` with the key `name` read as `label`, and refuses
/// the two together.
const NAME_AS_LABEL: &str = "final named = <String, dynamic>{}; for (final entry in \
     json.entries) { final key = switch (entry.key) { 'name' => 'label', final other => other }; \
     if (named.containsKey(key)) throw FormatException('duplicate field `$key`'); named[key] = \
     entry.value; }";

/// What copies a payload's entries into `named` with the keys `name` and `title` read as `label`,
/// and `remark` as `note`.
const HAWSING_NAMES: &str = "final named = <String, dynamic>{}; for (final entry in \
     json.entries) { final key = switch (entry.key) { 'name' || 'title' => 'label', 'remark' => \
     'note', final other => other }; if (named.containsKey(key)) throw FormatException('duplicate \
     field `$key`'); named[key] = entry.value; }";

/// Asserts that `definition` holds each of `pieces`.
fn holds_each(definition: &str, pieces: &[String]) {
    for piece in pieces {
        assert!(definition.contains(piece), "no `{piece}` in: {definition}");
    }
}

/// An enum takes a variant under each of its aliases, under each tagging, and an externally
/// tagged one reads the value of a variant read under an alias too from the entry it took, a unit
/// variant from its entry over `null`, and one under a content key from an object with no content.
/// Each variant is written under the name serde writes, and a member of a variant is read under
/// its alias.
#[test]
fn a_tagged_enum_reads_a_variant_under_each_alias() {
    holds_each(
        &surging_dart::dart_definition(),
        &[
            "switch (json) { case 'Calm' || 'Still' || 'Slack': return SurgingCalm(); default: \
             throw ArgumentError('Unknown Surging: ' + json); }"
                .to_owned(),
            "switch (tag) { case 'Berth' || 'Pier': return \
             SurgingBerth.fromJsonUnjudged(map[tag]); case 'Calm' || 'Still' || 'Slack': return \
             map[tag] == null ? SurgingCalm() : throw ArgumentError('Surging: ' + tag + ' takes no \
             value'); case 'Held' || 'Tied': return SurgingHeld(map[tag] as String); default: \
             throw ArgumentError('Unknown Surging: ' + tag); }"
                .to_owned(),
            format!(
                "factory SurgingBerth.fromJsonUnjudged(Map<String, dynamic> json) {{ \
                 {NAME_AS_LABEL} return SurgingBerth(label: named['label'] as String,); }} \
                 @override dynamic toJson() => {{ 'Berth': {{ 'label': label, }} }};"
            ),
            "@override dynamic toJson() => 'Calm';".to_owned(),
            "@override dynamic toJson() => { 'Held': value };".to_owned(),
        ],
    );
    holds_each(
        &careening_dart::dart_definition(),
        &[
            "final tag = json['kind'] as String; switch (tag) { case 'Berth' || 'Pier': return \
             CareeningBerth.fromJsonUnjudged(json); case 'Calm' || 'Still': return \
             CareeningCalm(); default:"
                .to_owned(),
            format!(
                "factory CareeningBerth.fromJsonUnjudged(Map<String, dynamic> json) {{ \
                 {NAME_AS_LABEL} return CareeningBerth(label: named['label'] as String,); }} \
                 Map<String, dynamic> toJson() => {{ 'kind': 'Berth','label': label, }};"
            ),
            "@override dynamic toJson() => { 'kind': 'Calm' };".to_owned(),
        ],
    );
    holds_each(
        &scending_dart::dart_definition(),
        &[
            "switch (tag) { case 'Berth' || 'Pier': return \
             ScendingBerth.fromJsonUnjudged(json['data']); case 'Calm' || 'Still': return \
             json.containsKey('data') ? throw FormatException('Scending: ' + tag + ' takes no \
             value') : ScendingCalm(); case 'Held' || 'Tied': return ScendingHeld(json['data'] as \
             String); default:"
                .to_owned(),
            "@override dynamic toJson() => { 'kind': 'Held', 'data': value };".to_owned(),
        ],
    );
}

/// A flattened externally tagged enum takes the first entry under a variant's name or an alias of
/// one, and reads the variant from it. The holder writes a unit variant, which the enum writes as
/// its name, as an entry of that name over `null`.
#[test]
fn a_flattened_enum_takes_the_entry_of_a_variant_under_an_alias() {
    holds_each(
        &riding_dart::dart_definition(),
        &[
            "surging: Surging.fromJsonUnjudged((switch (({...json}..remove('note'))) { final \
           whole => { for (final key in whole.keys.where(const ['Berth', 'Pier', 'Calm', 'Still', \
           'Slack', 'Held', 'Tied'].contains).take(1)) key: whole[key] } })),"
                .to_owned(),
            "Map<String, dynamic> toJson() => { 'note': note,...(switch ((surging).toJson()) { \
             final String name => {name: null}, final written => written as Map<String, dynamic> \
             }), };"
                .to_owned(),
        ],
    );
}

/// A plain enum takes a variant under each of its aliases, a function it names called first, and
/// writes it under the name serde writes.
#[test]
fn a_plain_enum_reads_a_variant_under_each_alias() {
    assert_eq!(
        swell_dart::dart_definition(),
        "enum Swell { ebb('Ebb'), flood('Flood'); const Swell(this.wireValue); final String \
         wireValue; static Swell fromJson(String json) => switch (json) { 'Ebb' || 'Low' || \
         'Neap' => Swell.ebb,'Flood' => Swell.flood, _ => throw ArgumentError('Unknown Swell: ' \
         + json) }; static Swell fromJsonUnjudged(String json) => fromJson(json); String toJson() \
         => wireValue; List<String> validate() => const []; }"
    );
    assert_eq!(
        lulling_dart::dart_definition(),
        "enum Lulling { ebb('Ebb'), flood('Flood'); const Lulling(this.wireValue); final String \
         wireValue; static Lulling fromJson(String json) => fromJsonUnjudged(json); static \
         Lulling fromJsonUnjudged(String payload) { final json = unpadded(payload) as String; \
         return switch (json) { 'Ebb' || 'Low' => Lulling.ebb,'Flood' => Lulling.flood, _ => \
         throw ArgumentError('Unknown Lulling: ' + json) }; } String toJson() => wireValue; \
         List<String> validate() => const []; }"
    );
}

/// A struct reads a member under each of its aliases, a parameter's converter handed what was
/// read under one, and writes each under the name serde writes.
#[test]
fn a_struct_reads_a_member_under_each_alias() {
    holds_each(
        &hawsing_dart::dart_definition(),
        &[format!(
            "factory Hawsing.fromJsonUnjudged(Map<String, dynamic> json) {{ {HAWSING_NAMES} \
             return Hawsing(label: named['label'] as String,note: (named['note'] == null ? null \
             : named['note'] as String),); }} Map<String, dynamic> toJson() => {{ 'label': \
             label,if (note != null) 'note': (note == null ? null : note), }};"
        )],
    );
    holds_each(
        &gybing_dart::dart_definition(),
        &[format!(
            "factory Gybing.fromJsonUnjudged(Map<String, dynamic> json, LabelType \
             Function(dynamic) labelTypeFromJson) {{ {HAWSING_NAMES} return Gybing(label: \
             labelTypeFromJson(named['label']),note: (named['note'] == null ? null : \
             named['note'] as String),); }}"
        )],
    );
}

/// A member flattened after a member under an alias is handed the object without the member's
/// key, and a flattened struct takes each name serde reads its members under off its holder's
/// object.
#[test]
fn a_member_flattened_beside_an_alias_is_handed_the_keys_serde_hands_it() {
    holds_each(
        &fairleading_dart::dart_definition(),
        &[format!(
            "factory Fairleading.fromJsonUnjudged(Map<String, dynamic> json) {{ {NAME_AS_LABEL} \
             return Fairleading(label: named['label'] as String,tally: \
             Pennon.fromJsonUnjudged(withItsKeys(({{...named}}..remove('label')))),); }}"
        )],
    );
    holds_each(
        &fothering_dart::dart_definition(),
        &["Fothering(hawsing: Hawsing.fromJsonUnjudged(json),tally: \
           Pennon.fromJsonUnjudged(withItsKeys(({...json}..remove('label')..remove('name')\
           ..remove('title')..remove('note')..remove('remark')))),);"
            .to_owned()],
    );
}

/// An untagged enum tries a variant with a member under an alias from the payload with the alias
/// read as the member's name.
#[test]
fn an_untagged_enum_reads_a_member_under_its_alias() {
    holds_each(
        &veering_dart::dart_definition(),
        &[format!(
            "factory VeeringBerth.fromJsonUnjudged(Map<String, dynamic> json) {{ \
             {NAME_AS_LABEL} return VeeringBerth(label: named['label'] as String,); }}"
        )],
    );
}
