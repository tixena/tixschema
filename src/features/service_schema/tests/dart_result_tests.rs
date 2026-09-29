//! The `{Service}{Operation}Result` sealed class pair, read off the emitted text — the same way
//! `dart_http_client_tests` reads the client's own text.

use super::{
    DART_BYTES_HEADER_OUT_SERVICE, DART_SINGLE_PLACEHOLDER_HTTP_SERVICE, DART_STREAM_HTTP_SERVICE,
    DART_UNIT_SUCCESS_HTTP_SERVICE, dart_result_of,
};

#[test]
fn a_reply_operation_gets_a_sealed_base_and_three_final_subclasses() {
    let written = dart_result_of(DART_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    let joined = written.join("\n\n");
    assert!(
        joined.contains(
            "/// What `window` answers: the success, the error the operation declared, or a fault \
             it never\n\
             /// declared.\n\
             sealed class ConversationClientServiceWindowResult {\n  \
             const ConversationClientServiceWindowResult();\n}"
        ),
        "the sealed base carries a doc line naming the operation it answers for. Got: {joined}"
    );
    assert!(
        joined.contains(
            "final class ConversationClientServiceWindowResultOk extends \
             ConversationClientServiceWindowResult {\n  \
             const ConversationClientServiceWindowResultOk(this.value);\n  \
             final WindowPage value;\n}"
        ),
        "the success arm carries the operation's own declared success type. Got: {joined}"
    );
    assert!(
        joined.contains(
            "final class ConversationClientServiceWindowResultOperation extends \
             ConversationClientServiceWindowResult {\n  \
             const ConversationClientServiceWindowResultOperation(this.error);\n  \
             final WindowError error;\n}"
        ),
        "the declared-error arm carries the operation's own declared error type. Got: {joined}"
    );
    assert!(
        joined.contains(
            "final class ConversationClientServiceWindowResultFault extends \
             ConversationClientServiceWindowResult {\n  \
             const ConversationClientServiceWindowResultFault(this.fault);\n  \
             final ConversationClientServiceFaultFields fault;\n}"
        ),
        "the fault arm carries the same fault fields every other Dart surface answers faults \
         through. Got: {joined}"
    );
}

#[test]
fn the_one_way_operation_gets_no_pair() {
    let written = dart_result_of(DART_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert_eq!(
        written.len(),
        1,
        "`purge_conversation` is one-way and declared no reply to join into a pair. Got: \
         {written:?}"
    );
    assert!(
        !written
            .iter()
            .any(|pair| pair.contains("PurgeConversation")),
        "got: {written:?}"
    );
}

#[test]
fn the_pair_carries_no_codec() {
    let written = dart_result_of(DART_SINGLE_PLACEHOLDER_HTTP_SERVICE).join("\n\n");
    for named in ["fromJson", "toJson"] {
        assert!(
            !written.contains(named),
            "the pair is caller-shaped, built by the client, never a wire shape a codec reads or \
             writes. Got: {written}"
        );
    }
}

#[test]
fn a_stream_operations_ok_carries_the_same_record_the_rest_client_answers_with() {
    let written = dart_result_of(DART_STREAM_HTTP_SERVICE).join("\n\n");
    assert!(
        written.contains(
            "final ({String? contentRange, String contentType, Stream<List<int>> body}) value;"
        ),
        "a bare streamed success shares its spelling with `dart_http_client`'s own return type, \
         through the shared `dart_success_type`. Got: {written}"
    );
    assert!(
        written.contains(
            "final (({String? contentRange, String contentType, Stream<List<int>> body}), String) \
             value;"
        ),
        "a `header_out` operation's own success wraps the streamed record in a tuple, exactly as \
         the client's own return type does. Got: {written}"
    );
}

#[test]
fn a_bytes_operation_with_header_out_shares_its_success_type_with_the_client() {
    let written = dart_result_of(DART_BYTES_HEADER_OUT_SERVICE).join("\n\n");
    assert!(
        written.contains("final (List<int>, String, String) value;"),
        "got: {written}"
    );
}

#[test]
fn a_unit_success_gets_a_field_less_ok_member() {
    let written = dart_result_of(DART_UNIT_SUCCESS_HTTP_SERVICE).join("\n\n");
    assert!(
        written.contains(
            "final class PingClientServicePingResultOk extends PingClientServicePingResult {\n  \
             const PingClientServicePingResultOk();\n}"
        ),
        "got: {written}"
    );
    assert!(!written.contains("void value"), "got: {written}");
}
