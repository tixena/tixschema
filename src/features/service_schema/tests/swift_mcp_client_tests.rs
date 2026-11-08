//! The Swift `mcp` client, read off the emitted text. The runtime group in
//! `tests/service_schema_emitted_client_tests/run_swift_mcp.rs` compiles and runs it.

use super::{MCP_SHADOWED_SERVICE, swift_mcp_client_of};

#[test]
fn an_argument_named_after_a_local_keeps_its_label_and_is_read_by_another_name() {
    let written = swift_mcp_client_of(MCP_SHADOWED_SERVICE);
    assert!(
        written.contains("arguments arguments_: String")
            && written.contains("carried carried_: String"),
        "the label a caller writes stays, and the method reads the argument by a name of its own. \
         Got: {written}"
    );
    assert!(
        written.contains("headers[\"x-arguments\"] = ShadowServiceMcpValue(arguments_)"),
        "got: {written}"
    );
}

#[test]
fn an_optional_header_is_left_out_when_it_holds_nothing() {
    let written = swift_mcp_client_of(MCP_SHADOWED_SERVICE);
    assert!(
        written.contains(
            "if let sentHeader = trace { headers[\"x-trace\"] = ShadowServiceMcpValue(sentHeader) }"
        ),
        "got: {written}"
    );
}

#[test]
fn the_client_is_an_actor_that_numbers_its_requests_in_its_own_state() {
    let written = swift_mcp_client_of(MCP_SHADOWED_SERVICE);
    assert!(
        written.contains("public actor ShadowServiceMcpClient")
            && written.contains("private var nextId = 0")
            && written.contains("nextId += 1"),
        "got: {written}"
    );
}

#[test]
fn a_one_way_method_throws_the_refusal_both_swift_clients_share() {
    let written = swift_mcp_client_of(MCP_SHADOWED_SERVICE);
    assert!(
        written.contains("public func touch(_ req: ShadowTouchRequest) async throws")
            || written.contains("public func touch("),
        "got: {written}"
    );
    assert!(
        written.contains("throw ShadowServiceRefusal(fault: fault)"),
        "got: {written}"
    );
}
