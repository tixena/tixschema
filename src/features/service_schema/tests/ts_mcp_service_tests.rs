//! What the TypeScript surfaces name for an untagged enum no variant of which read, off the
//! emitted text.

use super::{
    client_of, http_client_of, http_service_of, service_of, ts_mcp_client_of, ts_mcp_server_of,
    ws_client_of, ws_server_of, ws_service_of,
};

const A_TOOL: &str = "
    pub trait GreetingService<Ctx> {
        /// Greets one person.
        #[service_schema_op(mcp(read_only))]
        async fn greet(&self, ctx: &Ctx, name: String) -> Result<String, GreetingError>;
    }
";

#[test]
fn the_mcp_server_names_an_issue_by_its_own_path_and_reads_no_alternative() {
    let server = ts_mcp_server_of(A_TOOL);
    assert!(
        server.contains("function greetingServiceMcpSteps("),
        "got: {server}"
    );
    assert!(!server.contains("issue.errors"), "got: {server}");
    assert!(
        server.contains("issue.path.slice(0, issue.path.length - issue.params.union)"),
        "got: {server}"
    );
}

#[test]
fn every_typescript_surface_that_folds_issues_names_the_union_an_issue_is_marked_with() {
    let folded = [
        ("ws_rpc client", client_of(A_TOOL)),
        ("http_rest client", http_client_of(A_TOOL)),
        ("http_rest dispatcher", http_service_of(A_TOOL)),
        ("ws_rpc websocket client", ws_client_of(A_TOOL)),
        ("ws_rpc dispatcher", service_of(A_TOOL)),
        ("ws_rpc websocket dispatcher", ws_service_of(A_TOOL)),
        ("ws_rpc websocket server", ws_server_of(A_TOOL)),
        ("mcp client", ts_mcp_client_of(A_TOOL)),
        ("mcp server", ts_mcp_server_of(A_TOOL)),
    ];
    for (surface, text) in folded {
        assert!(
            text.contains("params?.union") || !text.contains("unrecognized_keys"),
            "{surface} folds an unrecognized key and does not read the mark: {text}"
        );
    }
}
