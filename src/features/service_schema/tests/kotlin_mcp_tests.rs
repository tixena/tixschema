//! The Kotlin `mcp` server and client, read off the emitted text. The runtime groups in
//! `tests/service_schema_emitted_client_tests/run_kotlin_mcp.rs` compile and run them.

use super::{MCP_SHADOWED_SERVICE, kotlin_mcp_client_of, kotlin_mcp_service_of};

/// The text from `marker` through the end of the line that ends the block it opens.
fn from_marker<'written>(written: &'written str, marker: &str) -> &'written str {
    let start = written.find(marker);
    assert!(start.is_some(), "no `{marker}` in: {written}");
    let rest = &written[start.unwrap()..];
    let end = rest.find("\n    }\n").map_or(rest.len(), |at| at + 7);
    &rest[..end]
}

#[test]
fn a_header_is_read_into_a_local_moved_off_the_names_the_route_writes() {
    let written = kotlin_mcp_service_of(MCP_SHADOWED_SERVICE);
    let arm = from_marker(&written, "\"shadow\" ->");
    for held in ["arguments_", "outcome_"] {
        assert!(
            arm.contains(&format!("val {held} = shadowServiceMcpHeader(")),
            "the header read into `{held}`, because the arm writes `{}` itself. Got: {arm}",
            held.trim_end_matches('_')
        );
    }
    assert!(
        arm.contains("handlers.shadow(decoded, arguments_, outcome_, carried, trace)"),
        "the handler is called with the locals the headers were read into. Got: {arm}"
    );
}

#[test]
fn an_optional_header_is_read_as_absent_when_it_is_missing() {
    let written = kotlin_mcp_service_of(MCP_SHADOWED_SERVICE);
    let arm = from_marker(&written, "\"shadow\" ->");
    assert!(
        arm.contains("val trace = shadowServiceMcpOptionalHeader(\"shadow\", \"x-trace\","),
        "got: {arm}"
    );
}

#[test]
fn a_one_way_operation_answers_the_empty_success_whatever_its_handler_did() {
    let written = kotlin_mcp_service_of(MCP_SHADOWED_SERVICE);
    let arm = from_marker(&written, "\"touch\" ->");
    assert!(
        arm.contains("if (unexpected is CancellationException) throw unexpected")
            && arm.contains("shadowServiceMcpDone(null, null)"),
        "a panic of a one-way handler is swallowed and a cancellation is not. Got: {arm}"
    );
}

#[test]
fn an_endpoint_that_holds_a_name_twice_throws_when_it_is_built() {
    let written = kotlin_mcp_service_of(MCP_SHADOWED_SERVICE);
    assert!(
        written
            .contains("class ShadowServiceMcpEndpoint(vararg catalogs: Map<String, List<String>>)")
            && written
                .contains("check(name !in earlier) { \"two services share the MCP name $name\" }"),
        "got: {written}"
    );
}

#[test]
fn the_catalog_and_the_wrapping_are_left_for_the_accessor() {
    let written = kotlin_mcp_service_of(MCP_SHADOWED_SERVICE);
    for marker in [
        "@MCP_TOOLS@",
        "@MCP_NAMES@",
        "@MCP_INSTRUCTIONS@",
        "@MCP_VERSION@",
    ] {
        assert_eq!(
            written.matches(marker).count(),
            1,
            "`{marker}` in: {written}"
        );
    }
    assert!(
        written.contains("\"shadow\" to @MCP_WRAPPED_0@,")
            && written.contains("\"touch\" to @MCP_WRAPPED_1@,"),
        "one wrapping marker per tool, in declaration order. Got: {written}"
    );
}

#[test]
fn the_client_copies_an_argument_named_after_a_local_and_reads_the_copy() {
    let written = kotlin_mcp_client_of(MCP_SHADOWED_SERVICE);
    assert!(
        written.contains("val arguments_ = arguments")
            && written.contains("val outcome_ = outcome"),
        "got: {written}"
    );
    assert!(
        written.contains(
            "sent[\"x-arguments\"] = Json.encodeToJsonElement(serializer<String>(), arguments_)"
        ),
        "got: {written}"
    );
}

#[test]
fn a_one_way_method_throws_the_refusal_the_client_declares() {
    let written = kotlin_mcp_client_of(MCP_SHADOWED_SERVICE);
    assert!(
        written.contains("class ShadowServiceMcpRefusal(val fault: ShadowServiceFaultFields)")
            && written.contains("throw ShadowServiceMcpRefusal(outcome.fault)"),
        "got: {written}"
    );
}
