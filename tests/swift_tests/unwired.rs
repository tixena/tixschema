//! No Swift type declares a member, or a variant, serde neither writes nor reads.

use super::unwired_models::{
    assert_serde_neither_writes_nor_reads, ballast_load_swift, ballast_swift,
};

/// The struct declares its one member on the wire, and the enum its one variant on the wire.
#[test]
fn no_swift_type_declares_a_member_serde_neither_writes_nor_reads() {
    assert_serde_neither_writes_nor_reads();
    let ballast = ballast_swift::swift_definition();
    assert!(
        ballast.contains("public let note: String;"),
        "got: {ballast}"
    );
    let load = ballast_load_swift::swift_definition();
    assert!(load.contains("case open("), "got: {load}");
    for text in [&ballast, &load] {
        for absent in ["weight", "tonnage", "Shifted", "shifted", "UInt64", "Int64"] {
            assert!(!text.contains(absent), "`{absent}` in: {text}");
        }
    }
}
