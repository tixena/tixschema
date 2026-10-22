//! A member serde reads and never writes: the Dart read takes it, and `toJson` writes no key for
//! it, in a struct of its own and beside a flattened member.

use super::held_numbers::u32_held;
use super::unwritten_models::furling_dart;
#[cfg(feature = "serde")]
use super::unwritten_models::{flemishing_dart, ranging_dart, reefing_dart, splaying_dart};

/// The member is read, and `toJson` writes no key for it.
#[test]
fn a_member_serde_never_writes_is_read_and_has_no_entry() {
    let furling = furling_dart::dart_definition();
    let spare = format!(
        "spare: (json['spare'] == null ? null : {}),",
        u32_held("json['spare'] as int")
    );
    assert!(furling.contains(&spare), "got: {furling}");
    assert!(
        furling.contains("Map<String, dynamic> toJson() => { 'label': label, };"),
        "got: {furling}"
    );
}

/// Beside a flattened member, the member is read, and `toJson` writes no key for it.
#[cfg(feature = "serde")]
#[test]
fn a_member_serde_never_writes_beside_a_flattened_one_has_no_entry() {
    let reefing = reefing_dart::dart_definition();
    let spare = format!(
        "spare: (json['spare'] == null ? null : {}),",
        u32_held("json['spare'] as int")
    );
    assert!(reefing.contains(&spare), "got: {reefing}");
    assert!(
        reefing.contains(
            "Map<String, dynamic> toJson() => { ...((braiding).toJson() as Map<String, \
             dynamic>), };"
        ),
        "got: {reefing}"
    );
}

/// A flattened struct and a flattened map serde never writes are read, and `toJson` spreads
/// nothing of them. One that is no `Option` is read as a flattened member is, its key being none
/// to leave out.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_member_serde_never_writes_is_read_and_spread_nowhere() {
    let flemishing = flemishing_dart::dart_definition();
    for held in [
        "braiding: (json.keys.any(const ['label'].contains) ? Braiding.fromJsonUnjudged(json) \
         : null)",
        "Map<String, dynamic> toJson() => { 'note': note, };",
    ] {
        assert!(flemishing.contains(held), "no `{held}` in: {flemishing}");
    }
    let splaying = splaying_dart::dart_definition();
    for held in [
        "const Splaying({required this.braiding,required this.note,});",
        "Splaying(braiding: Braiding.fromJsonUnjudged(json),",
        "Map<String, dynamic> toJson() => { 'note': note, };",
    ] {
        assert!(splaying.contains(held), "no `{held}` in: {splaying}");
    }
    let ranging = ranging_dart::dart_definition();
    for held in [
        "const Ranging({required this.counts,}); final Map<String, int> counts;",
        "Map<String, dynamic> toJson() => {  };",
    ] {
        assert!(ranging.contains(held), "no `{held}` in: {ranging}");
    }
}
