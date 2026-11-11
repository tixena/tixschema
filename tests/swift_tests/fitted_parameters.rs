//! What the Swift backend writes for a type parameter: each read of one is read again through
//! `SwiftSchemaFit` where Foundation refuses it with no coding path, as a read of a fixed-width
//! integer is, and a type that declares no parameter reads what fills one as it did.
//! `service_schema_emitted_client_tests/run_swift_fitted_parameters.rs` compiles the models of
//! `fitted_parameters_models.rs` under a Swift toolchain and reads a payload of each.

use super::fitted_parameters_models::{
    bollard_swift, cleat_swift, fender_swift, hawser_swift, knot_mooring_swift, mooring_swift,
    quay_swift, tether_swift, wharf_swift, wide_mooring_swift,
};
use super::swift_fitted::FITTED;

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for text in written {
        assert!(swift.contains(text), "{text}\nin: {swift}");
    }
}

#[test]
fn a_member_of_a_type_parameter_is_read_through_the_fitting_read() {
    holds(
        &tether_swift::swift_definition(),
        &[
            FITTED,
            "self.held = try Self.swiftSchemaFitted({ try container.decode(Held.self, forKey: \
             .held) }, again: { _ = try container.decode(SwiftSchemaFit<Held>.self, forKey: \
             .held) }) }; ",
        ],
    );
}

#[test]
fn a_type_parameter_in_a_list_as_a_map_value_and_under_an_option_is_read_through_it() {
    holds(
        &hawser_swift::swift_definition(),
        &[
            FITTED,
            "self.keyed = try Self.swiftSchemaFitted({ try container.decode([String: Held].self, \
             forKey: .keyed) }, again: { _ = try container.decode([String: \
             SwiftSchemaFit<Held>].self, forKey: .keyed) }); ",
            "self.listed = try Self.swiftSchemaFitted({ try container.decode([Held].self, forKey: \
             .listed) }, again: { _ = try container.decode([SwiftSchemaFit<Held>].self, forKey: \
             .listed) }); ",
            "self.maybe = try Self.swiftSchemaFitted({ try container.decodeIfPresent(Held.self, \
             forKey: .maybe) }, again: { _ = try \
             container.decodeIfPresent(SwiftSchemaFit<Held>.self, forKey: .maybe) }) }; ",
        ],
    );
}

#[test]
fn a_type_parameter_in_a_variant_and_in_a_tuple_struct_is_read_through_it() {
    holds(
        &cleat_swift::swift_definition(),
        &[
            FITTED,
            "case \"Bare\": self = .bare(try Self.swiftSchemaFitted({ try container.decode(\
             Held.self, forKey: key) }, again: { _ = try \
             container.decode(SwiftSchemaFit<Held>.self, forKey: key) })); ",
            "slot0 = try Self.swiftSchemaFitted({ try container.decode(Held.self) }, again: { _ = \
             try container.decode(SwiftSchemaFit<Held>.self) }); slot1 = try \
             container.decode(String.self); ",
        ],
    );
    holds(
        &fender_swift::swift_definition(),
        &[
            FITTED,
            "self.value = try Self.swiftSchemaFitted({ try container.decode(Held.self) }, again: \
             { _ = try container.decode(SwiftSchemaFit<Held>.self) }) }; ",
        ],
    );
}

#[test]
fn a_type_that_declares_no_parameter_reads_what_fills_one_as_it_did() {
    for (swift, read) in [
        (
            mooring_swift::swift_definition(),
            "self.inner = try container.decode(Tether<UInt32>.self, forKey: .inner) }; ",
        ),
        (
            wide_mooring_swift::swift_definition(),
            "self.inner = try container.decode(Tether<Int64>.self, forKey: .inner) }; ",
        ),
        (
            knot_mooring_swift::swift_definition(),
            "self.inner = try container.decode(Tether<Knot>.self, forKey: .inner) }; ",
        ),
        (
            bollard_swift::swift_definition(),
            "self.lines = try container.decode(Hawser<UInt8>.self, forKey: .lines) }; ",
        ),
        (
            wharf_swift::swift_definition(),
            "self.cleat = try container.decode(Cleat<UInt16>.self, forKey: .cleat) }; ",
        ),
        (
            quay_swift::swift_definition(),
            "self.fender = try container.decode(Fender<Int8>.self, forKey: .fender) }; ",
        ),
    ] {
        holds(&swift, &[read]);
        assert!(!swift.contains("SwiftSchemaFit"), "{swift}");
    }
}
