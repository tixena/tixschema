//! What the Dart emitter writes for a `Vec<u8>` and for each sequence that is not one.

use super::models::{
    bytes_counted_dart, bytes_either_dart, bytes_holder_dart, bytes_image_dart,
    bytes_lookalikes_dart, bytes_optional_dart, bytes_payload_dart,
};

/// The Dart read of a `Vec<u8>` member: the items held to a `u8`, then built as a `Uint8List`.
const READ: &str = "Uint8List.fromList((json['data'] as List<dynamic>).map((e) => (switch (e as int) { final number when number >= 0 && number <= 255 => number, final number => throw FormatException('expected an integer from 0 to 255, found $number') })).toList())";

#[test]
fn a_vec_of_u8_is_a_uint8list() {
    let emitted = bytes_image_dart::dart_definition();
    assert!(emitted.contains("final Uint8List data;"), "{emitted}");
    assert!(emitted.contains(&format!("data: {READ},")), "{emitted}");
    assert!(
        emitted.contains("'data': data.map((e) => (switch (e) {"),
        "{emitted}"
    );
    assert!(!emitted.contains("List<int>"), "{emitted}");
}

#[test]
fn a_vec_of_u8_is_a_uint8list_in_every_place_a_member_holds_one() {
    let emitted = bytes_holder_dart::dart_definition();
    for text in [
        "final BytesPayloadType aliased;",
        "final Uint8List data;",
        "final BytesBox<Uint8List> filled;",
        "final Map<String, Uint8List> keyed;",
        "final DartResult1<Uint8List, String> outcome;",
        "final (Uint8List, String) paired;",
        "final List<Uint8List> rows;",
    ] {
        assert!(emitted.contains(text), "{text}\n{emitted}");
    }
    assert!(
        bytes_payload_dart::dart_definition().contains("final Uint8List value;"),
        "{}",
        bytes_payload_dart::dart_definition()
    );
    assert!(bytes_either_dart::dart_definition().contains("final Uint8List value;"));
    assert!(bytes_optional_dart::dart_definition().contains("final Uint8List? maybe;"));
}

#[test]
fn every_other_sequence_stays_a_list_of_int() {
    let emitted = bytes_lookalikes_dart::dart_definition();
    for text in [
        "final List<int> boxed;",
        "final List<int> digest;",
        "final List<int> distinct;",
        "final List<int?> gaps;",
        "final List<int> queued;",
        "final List<int> signed;",
        "final List<int> wide;",
    ] {
        assert!(emitted.contains(text), "{text}\n{emitted}");
    }
    assert!(!emitted.contains("final Uint8List boxed"), "{emitted}");
    assert!(emitted.contains("final List<int> sevens;"), "{emitted}");
}

#[test]
fn validate_counts_a_uint8list_as_it_counts_a_list() {
    let emitted = bytes_counted_dart::dart_definition();
    assert!(
        emitted.contains(
            "if (data.length < 2) \"'data': too short: minimum length is 2, got ${data.length}\""
        ),
        "{emitted}"
    );
}
