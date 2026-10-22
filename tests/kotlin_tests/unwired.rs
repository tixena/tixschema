//! No Kotlin type declares a member, or a variant, serde neither writes nor reads.

use super::unwired_models::{
    assert_serde_neither_writes_nor_reads, ballast_kotlin, ballast_load_kotlin,
};

/// The struct declares its one member on the wire, and the enum its one variant on the wire.
#[test]
fn no_kotlin_type_declares_a_member_serde_neither_writes_nor_reads() {
    assert_serde_neither_writes_nor_reads();
    let ballast = ballast_kotlin::kotlin_definition();
    assert!(ballast.contains("val note: String"), "got: {ballast}");
    let load = ballast_load_kotlin::kotlin_definition();
    assert!(load.contains("Open"), "got: {load}");
    for text in [&ballast, &load] {
        for absent in ["weight", "tonnage", "Shifted", "ULong", "Long"] {
            assert!(!text.contains(absent), "`{absent}` in: {text}");
        }
    }
}
