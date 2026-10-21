//! A member serde reads and never writes: the Dart read takes it, and `toJson` writes no key for
//! it, in a struct of its own and beside a flattened member.

use super::unwritten_models::furling_dart;
#[cfg(feature = "serde")]
use super::unwritten_models::reefing_dart;

/// The member is read, and `toJson` writes no key for it.
#[test]
fn a_member_serde_never_writes_is_read_and_has_no_entry() {
    let furling = furling_dart::dart_definition();
    assert!(
        furling.contains("spare: (json['spare'] == null ? null : json['spare'] as int),"),
        "got: {furling}"
    );
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
    assert!(
        reefing.contains("spare: (json['spare'] == null ? null : json['spare'] as int),"),
        "got: {reefing}"
    );
    assert!(
        reefing.contains(
            "Map<String, dynamic> toJson() => { ...((braiding).toJson() as Map<String, \
             dynamic>), };"
        ),
        "got: {reefing}"
    );
}
