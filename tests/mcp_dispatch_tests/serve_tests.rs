//! `serve` and `answer`: the samples of the wire, the order of the checks, the routing of a call,
//! and an endpoint shared by two services.

use crate::billing_mcp;
use crate::cases::FAULT_CASES;
use crate::document_mcp::{self, McpEndpoint};
use crate::http_tests::{ORIGIN, post};
use crate::support::{
    CENTS, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, LISTED_FOR_MS, Lines, METHOD_NOT_FOUND,
    Recorded, UNSUPPORTED_VERSION, VERSION_NUMBER, WORKSPACE, answer, answer_workspace,
    assert_error, assert_result, assert_valid, block_on, call, id, request, server_info, through,
};
use crate::tests::Archive;
use alloc::sync::Arc;
use core::future::Future;
use serde_json::{Value, json};

/// The output of every tool that answers a version.
fn version_output() -> Value {
    json!({ "additionalProperties": false,
            "properties": { "version": { "type": "integer" } },
            "required": ["version"], "type": "object" })
}

/// The tools of `DocumentService`, as the document writes them.
fn expected_tools() -> Value {
    json!([
        { "name": "check-drafts",
          "title": "Count Of Drafts",
          "description": "Answers the number of drafts it was handed.",
          "inputSchema": { "additionalProperties": false,
                           "properties": { "drafts": { "items": { "additionalProperties": false,
                                                       "properties": { "references": { "items": { "additionalProperties": false,
                                                                                         "properties": { "name": { "type": "string" } },
                                                                                         "required": ["name"], "type": "object" },
                                                                                       "type": "array" },
                                                                       "title": { "minLength": 1_u8, "type": "string" } },
                                                       "required": ["references", "title"], "type": "object" },
                                                     "type": "array" } },
                           "required": ["drafts"], "type": "object" },
          "outputSchema": version_output() },
        { "name": "check-many",
          "title": "Versions Of Many",
          "description": "Answers the versions of several documents, one request for each.",
          "inputSchema": { "additionalProperties": false,
                           "properties": { "value": { "items": { "additionalProperties": false,
                                                      "properties": { "document_id": { "minLength": 1_u8, "type": "string" } },
                                                      "required": ["document_id"], "type": "object" }, "type": "array" } },
                           "required": ["value"], "type": "object" },
          "outputSchema": version_output() },
        { "name": "check-maybe",
          "title": "Version Of Maybe One",
          "description": "Answers the version of a document, if one is asked for.",
          "inputSchema": { "additionalProperties": false,
                           "properties": { "value": { "anyOf": [{ "additionalProperties": false,
                                                      "properties": { "document_id": { "minLength": 1_u8, "type": "string" } },
                                                      "required": ["document_id"], "type": "object" }, { "type": "null" }] } },
                           "required": ["value"], "type": "object" },
          "outputSchema": version_output() },
        { "name": "check-named",
          "title": "Versions By Name",
          "description": "Answers the versions of documents asked for by name.",
          "inputSchema": { "additionalProperties": { "additionalProperties": false,
                                                      "properties": { "document_id": { "minLength": 1_u8, "type": "string" } },
                                                      "required": ["document_id"], "type": "object" }, "type": "object" },
          "outputSchema": version_output() },
        { "name": "check-pair",
          "title": "Versions Of A Pair",
          "description": "Answers the version of the second of two documents.",
          "inputSchema": { "$defs": { "GetVersionRequest": { "additionalProperties": false,
                                                             "properties": { "document_id": { "minLength": 1_u8, "type": "string" } },
                                                             "required": ["document_id"], "type": "object" } },
                           "additionalProperties": false,
                           "properties": { "first": { "$ref": "#/$defs/GetVersionRequest" },
                                           "second": { "$ref": "#/$defs/GetVersionRequest" } },
                           "required": ["first", "second"], "type": "object" },
          "outputSchema": version_output() },
        { "name": "get-tagged",
          "title": "Get Tagged",
          "description": "Answers the current version of one document, unless it still carries the tag the caller knows.",
          "inputSchema": { "additionalProperties": false,
                           "properties": { "if-none-match": { "anyOf": [{ "type": "string" }, { "type": "null" }] },
                                           "value": { "type": "string" } },
                           "required": ["value"], "type": "object" },
          "outputSchema": { "additionalProperties": false,
                            "properties": { "etag": { "type": "string" },
                                            "value": { "additionalProperties": false,
                                                       "properties": { "version": { "type": "integer" } },
                                                       "required": ["version"], "type": "object" } },
                            "required": ["value", "etag"], "type": "object" } },
        { "name": "get-version",
          "title": "Current Document Version",
          "description": "Answers the current version of one document.",
          "inputSchema": { "additionalProperties": false,
                           "properties": { "document_id": { "minLength": 1_u8, "type": "string" } },
                           "required": ["document_id"], "type": "object" },
          "outputSchema": version_output() },
        { "name": "lookup-document",
          "title": "Lookup",
          "description": "Answers the current version of the document the identifier names.",
          "inputSchema": { "additionalProperties": false,
                           "properties": { "value": { "type": "string" } },
                           "required": ["value"], "type": "object" },
          "outputSchema": version_output() },
        { "name": "rename-document",
          "title": "Rename Document",
          "description": "Renames one document.",
          "inputSchema": { "additionalProperties": false,
                           "properties": { "documentId": { "type": "string" }, "title": { "type": "string" } },
                           "required": ["documentId", "title"], "type": "object" } },
        { "name": "touch-document",
          "title": "Touch Document",
          "description": "Marks one document as read.",
          "inputSchema": { "additionalProperties": false,
                           "properties": { "documentId": { "type": "string" }, "reader": { "type": "string" } },
                           "required": ["documentId", "reader"], "type": "object" } }
    ])
}

/// The answer of `DocumentService` to a call of `tool`.
fn ask(tool: &str, arguments: &Value) -> Value {
    answer(&Archive::default(), &call(&id(1), tool, arguments)).unwrap()
}

/// The text of the one block a failed call answers.
fn failure_text(answered: &Value) -> &str {
    assert_eq!(answered["result"]["isError"], true);
    answered["result"]["content"][0]["text"].as_str().unwrap()
}

#[test]
fn server_discover_answers_the_name_the_instructions_and_the_capabilities() {
    let asked = request(&json!("discover-1"), "server/discover", json!({}));
    assert_valid("DiscoverRequest", &asked);
    let answered = answer(&Archive::default(), &asked).unwrap();
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": "discover-1",
            "result": {
                "resultType": "complete",
                "supportedVersions": ["2026-07-28"],
                "capabilities": { "completions": {}, "resources": {}, "tools": {} },
                "instructions": "Reads and renames documents in the caller's workspace.",
                "_meta": server_info("DocumentService"),
                "ttlMs": LISTED_FOR_MS,
                "cacheScope": "public",
            },
        })
    );
    assert_result("DiscoverResult", &answered);
    assert_eq!(
        document_mcp::SERVER_NAME,
        "DocumentService",
        "the trait names the server"
    );
    assert_eq!(
        document_mcp::INSTRUCTIONS,
        Some("Reads and renames documents in the caller's workspace.")
    );
}

#[test]
fn tools_list_answers_every_tool_in_declaration_order() {
    let asked = request(&id(1), "tools/list", json!({}));
    assert_valid("ListToolsRequest", &asked);
    let answered = answer(&Archive::default(), &asked).unwrap();
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "complete",
                "tools": expected_tools(),
                "_meta": server_info("DocumentService"),
                "ttlMs": LISTED_FOR_MS,
                "cacheScope": "public",
            },
        })
    );
    assert_result("ListToolsResult", &answered);
}

#[test]
fn a_success_answers_its_value_as_structured_content_and_as_text() {
    let answered = ask("get-version", &json!({ "document_id": "doc-1" }));
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "complete",
                "content": [{ "type": "text", "text": "{\"version\":7}" }],
                "structuredContent": { "version": VERSION_NUMBER },
                "isError": false,
                "_meta": server_info("DocumentService"),
            },
        })
    );
    assert_result("CallToolResult", &answered);
}

#[test]
fn an_answer_is_one_line_of_compact_json_with_its_keys_in_order() {
    let asked = call(&id(2), "get-version", &json!({ "document_id": "doc-1" }));
    let text = block_on(document_mcp::answer(
        &asked.to_string(),
        &Archive::default(),
        |_call| (),
        &Lines::default(),
        &document_mcp::McpInFlight::default(),
    ))
    .unwrap();
    assert_eq!(
        text,
        format!(
            "{{\"id\":2,\"jsonrpc\":\"2.0\",\"result\":{{\"_meta\":{{\"io.modelcontextprotocol/serverInfo\":{{\"name\":\"DocumentService\",\"version\":\"{}\"}}}},\"content\":[{{\"text\":\"{{\\\"version\\\":7}}\",\"type\":\"text\"}}],\"isError\":false,\"resultType\":\"complete\",\"structuredContent\":{{\"version\":7}}}}}}",
            env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn a_unit_success_answers_an_empty_content_list_and_no_structured_content() {
    let answered = ask(
        "rename-document",
        &json!({ "documentId": "doc-1", "title": "Plan" }),
    );
    assert_eq!(
        answered["result"],
        json!({
            "resultType": "complete",
            "content": [],
            "isError": false,
            "_meta": server_info("DocumentService"),
        })
    );
    assert_result("CallToolResult", &answered);
}

#[test]
fn a_declared_error_answers_its_json_as_text_and_no_structured_content() {
    let answered = ask("get-version", &json!({ "document_id": "missing" }));
    assert_eq!(
        answered["result"],
        json!({
            "resultType": "complete",
            "content": [{ "type": "text", "text": "{\"errorCode\":\"not-found\"}" }],
            "isError": true,
            "_meta": server_info("DocumentService"),
        })
    );
    assert_result("CallToolResult", &answered);
}

#[test]
fn arguments_that_do_not_read_answer_the_failed_validation_fault_naming_the_path() {
    let answered = ask("get-version", &json!({ "document_id": 5_u8 }));
    assert_eq!(
        failure_text(&answered),
        "{\"fault\":{\"detail\":\"'document_id': invalid type: integer `5`, expected a string\",\"field\":\"document_id\",\"kind\":\"failed-validation\",\"operation\":\"get-version\"},\"isServiceFault\":true}"
    );
    assert_result("CallToolResult", &answered);
}

#[test]
fn arguments_the_validator_refuses_answer_the_failed_validation_fault_naming_the_field() {
    let archive = Archive::default();
    let answered = answer(
        &archive,
        &call(&id(6), "get-version", &json!({ "document_id": "" })),
    )
    .unwrap();
    assert_eq!(
        failure_text(&answered),
        "{\"fault\":{\"detail\":\"'document_id': too short: minimum length is 1, got 0\",\"field\":\"document_id\",\"kind\":\"failed-validation\",\"operation\":\"get-version\"},\"isServiceFault\":true}"
    );
    assert_result("CallToolResult", &answered);
    assert_eq!(*archive.reached.lock().unwrap(), Vec::<String>::new());
}

#[test]
fn an_unknown_tool_answers_32602_with_the_unknown_operation_fault() {
    let answered = ask("delete-everything", &json!({}));
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "error": {
                "code": INVALID_PARAMS,
                "message": "Unknown tool: delete-everything",
                "data": { "isServiceFault": true, "fault": {
                    "detail": "the service answers to no operation by that name",
                    "kind": "unknown-operation",
                    "operation": "delete-everything",
                } },
            },
        })
    );
    assert_error("JSONRPCErrorResponse", &answered);
}

#[test]
fn another_protocol_version_answers_32022_naming_both() {
    let mut asked = call(&id(8), "get-version", &json!({ "document_id": "doc-1" }));
    asked["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2025-11-25");
    let answered = answer(&Archive::default(), &asked).unwrap();
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 8_u64,
            "error": {
                "code": UNSUPPORTED_VERSION,
                "message": "Unsupported protocol version",
                "data": { "supported": ["2026-07-28"], "requested": "2025-11-25" },
            },
        })
    );
    assert_error("UnsupportedProtocolVersionError", &answered);
}

#[test]
fn a_handler_that_panicked_answers_32603_with_the_handler_panic_fault() {
    let answered = ask("get-version", &json!({ "document_id": "panic" }));
    assert_eq!(
        answered["error"],
        json!({
            "code": INTERNAL_ERROR,
            "message": "get-version: the handler came apart",
            "data": { "isServiceFault": true, "fault": {
                "detail": "the handler came apart",
                "kind": "handler-panic",
                "operation": "get-version",
            } },
        })
    );
    assert_error("JSONRPCErrorResponse", &answered);
}

#[test]
fn a_one_way_call_answers_the_empty_success_after_its_handler_has_returned() {
    let archive = Archive::default();
    let answered = answer(
        &archive,
        &call(
            &id(11),
            "touch-document",
            &json!({ "documentId": "doc-1", "reader": "reviewer-7" }),
        ),
    )
    .unwrap();
    assert_eq!(
        answered["result"],
        json!({
            "resultType": "complete",
            "content": [],
            "isError": false,
            "_meta": server_info("DocumentService"),
        })
    );
    assert_result("CallToolResult", &answered);
    assert_eq!(
        *archive.reached.lock().unwrap(),
        ["touch-document doc-1 reviewer-7"]
    );
}

#[test]
fn a_one_way_handler_that_panicked_answers_the_empty_success() {
    let answered = ask(
        "touch-document",
        &json!({ "documentId": "doc-1", "reader": "panic" }),
    );
    assert_eq!(answered["result"]["content"], json!([]));
    assert_eq!(answered["result"]["isError"], false);
}

#[test]
fn a_one_way_call_still_judges_its_arguments() {
    let archive = Archive::default();
    let answered = answer(
        &archive,
        &call(&id(11), "touch-document", &json!({ "documentId": 4_u8 })),
    )
    .unwrap();
    assert_eq!(answered["result"]["isError"], true);
    assert_eq!(*archive.reached.lock().unwrap(), Vec::<String>::new());
}

#[test]
fn a_header_property_beside_a_wrapped_message_binds_both_ways() {
    let archive = Archive::default();
    let answered = answer(
        &archive,
        &call(
            &id(12),
            "get-tagged",
            &json!({ "value": "doc-1", "if-none-match": "\"v6\"" }),
        ),
    )
    .unwrap();
    assert_eq!(
        answered["result"],
        json!({
            "resultType": "complete",
            "content": [{ "type": "text", "text": "{\"etag\":\"\\\"v7\\\"\",\"value\":{\"version\":7}}" }],
            "structuredContent": { "etag": "\"v7\"", "value": { "version": VERSION_NUMBER } },
            "isError": false,
            "_meta": server_info("DocumentService"),
        })
    );
    assert_result("CallToolResult", &answered);
    assert_eq!(
        *archive.reached.lock().unwrap(),
        ["get-tagged doc-1 Some(\"\\\"v6\\\"\")"]
    );
}

#[test]
fn an_optional_header_property_may_be_left_out() {
    let archive = Archive::default();
    answer(
        &archive,
        &call(&id(12), "get-tagged", &json!({ "value": "doc-1" })),
    )
    .unwrap();
    assert_eq!(*archive.reached.lock().unwrap(), ["get-tagged doc-1 None"]);
}

#[test]
fn a_wrapped_message_is_read_from_its_value_property() {
    let found = ask("lookup-document", &json!({ "value": "doc-1" }));
    assert_eq!(
        found["result"]["structuredContent"],
        json!({ "version": VERSION_NUMBER })
    );
    let unfound = ask("lookup-document", &json!({}));
    assert_eq!(
        failure_text(&unfound),
        "{\"fault\":{\"detail\":\"'value': missing field `value`\",\"field\":\"value\",\"kind\":\"failed-validation\",\"operation\":\"lookup-document\"},\"isServiceFault\":true}"
    );
}

#[test]
fn a_key_that_is_missing_and_a_value_of_the_wrong_type_name_their_full_path() {
    let missing = ask(
        "check-drafts",
        &json!({ "drafts": [{ "references": [], "title": "Plan" }, { "references": [] }] }),
    );
    assert_eq!(
        failure_text(&missing),
        "{\"fault\":{\"detail\":\"'drafts.1.title': missing field `title`\",\"field\":\"drafts.1.title\",\"kind\":\"failed-validation\",\"operation\":\"check-drafts\"},\"isServiceFault\":true}"
    );
    let mistyped = ask(
        "check-drafts",
        &json!({ "drafts": [{ "references": [{ "name": 5_u8 }], "title": "Plan" }] }),
    );
    assert_eq!(
        failure_text(&mistyped),
        "{\"fault\":{\"detail\":\"'drafts.0.references.0.name': invalid type: integer `5`, expected a string\",\"field\":\"drafts.0.references.0.name\",\"kind\":\"failed-validation\",\"operation\":\"check-drafts\"},\"isServiceFault\":true}"
    );
    let broken = ask(
        "check-drafts",
        &json!({ "drafts": [{ "references": [], "title": "" }] }),
    );
    assert_eq!(
        failure_text(&broken),
        "{\"fault\":{\"detail\":\"'drafts.0.title': too short: minimum length is 1, got 0\",\"field\":\"drafts.0.title\",\"kind\":\"failed-validation\",\"operation\":\"check-drafts\"},\"isServiceFault\":true}"
    );
}

#[test]
fn an_unknown_key_inside_a_list_names_the_key_once() {
    let unknown = ask(
        "check-drafts",
        &json!({ "drafts": [
            { "references": [], "title": "Plan" },
            { "references": [], "subtitle": "Draft", "title": "Plan" },
        ] }),
    );
    assert_eq!(
        failure_text(&unknown),
        "{\"fault\":{\"detail\":\"'drafts.1.subtitle': unknown field `subtitle`, expected `references` or `title`\",\"field\":\"drafts.1.subtitle\",\"kind\":\"failed-validation\",\"operation\":\"check-drafts\"},\"isServiceFault\":true}"
    );
}

#[test]
fn a_fault_names_the_tool_the_caller_called_and_the_kind_the_read_or_the_validator_gives() {
    for case in FAULT_CASES {
        case.holds(&ask(case.tool, &case.arguments()));
    }
}

#[test]
fn arguments_that_are_no_object_answer_the_undeserializable_payload_fault() {
    let mut asked = call(&id(13), "get-version", &json!({}));
    asked["params"]["arguments"] = json!([1_u8]);
    let answered = answer(&Archive::default(), &asked).unwrap();
    assert!(failure_text(&answered).contains("\"kind\":\"undeserializable-payload\""));
}

#[test]
fn text_that_is_no_message_answers_32600_with_no_id() {
    for text in [
        "",
        "not json",
        "[]",
        "{\"jsonrpc\":\"1.0\",\"id\":1,\"method\":\"x\"}",
        "{\"jsonrpc\":\"2.0\",\"id\":1}",
        "{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"x\"}",
    ] {
        let answered: Value = serde_json::from_str(
            &block_on(document_mcp::answer(
                text,
                &Archive::default(),
                |_call| (),
                &Lines::default(),
                &document_mcp::McpInFlight::default(),
            ))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(answered["error"]["code"], INVALID_REQUEST, "{text}");
        assert!(answered.get("id").is_none(), "{text}");
        assert_valid("JSONRPCErrorResponse", &answered);
    }
}

#[test]
fn a_notification_answers_nothing() {
    let notification =
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} });
    assert_eq!(answer(&Archive::default(), &notification), None);
}

#[test]
fn a_request_missing_a_meta_key_answers_32602() {
    for key in [
        "io.modelcontextprotocol/protocolVersion",
        "io.modelcontextprotocol/clientCapabilities",
    ] {
        let mut asked = call(&id(1), "get-version", &json!({ "document_id": "doc-1" }));
        asked["params"]["_meta"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        let answered = answer(&Archive::default(), &asked).unwrap();
        assert_eq!(answered["error"]["code"], INVALID_PARAMS);
        assert_eq!(
            answered["error"]["message"],
            format!("Missing required _meta key: {key}")
        );
        assert_error("JSONRPCErrorResponse", &answered);
    }
    let bare = json!({ "jsonrpc": "2.0", "id": 1_u64, "method": "tools/list" });
    assert_eq!(
        answer(&Archive::default(), &bare).unwrap()["error"]["code"],
        INVALID_PARAMS
    );
}

#[test]
fn a_method_serve_does_not_know_answers_32601() {
    let answered = answer(
        &Archive::default(),
        &request(&id(1), "sampling/createMessage", json!({})),
    )
    .unwrap();
    assert_eq!(answered["error"]["code"], METHOD_NOT_FOUND);
    assert_error("JSONRPCErrorResponse", &answered);
}

#[test]
fn a_call_that_names_no_tool_answers_32602() {
    let answered = answer(
        &Archive::default(),
        &request(&id(1), "tools/call", json!({})),
    )
    .unwrap();
    assert_eq!(answered["error"]["code"], INVALID_PARAMS);
    assert_eq!(answered["error"].get("data"), None);
}

#[test]
fn route_answers_only_a_call_of_a_tool_its_service_owns() {
    let archive = Archive::default();
    let owned = json!({ "name": "get-version", "arguments": { "document_id": "doc-1" } });
    assert!(block_on(document_mcp::route("tools/call", &owned, &archive, &())).is_some());
    assert!(block_on(document_mcp::route("tools/list", &owned, &archive, &())).is_none());
    let strange = json!({ "name": "get-balance", "arguments": {} });
    assert!(block_on(document_mcp::route("tools/call", &strange, &archive, &())).is_none());
    assert!(block_on(document_mcp::route("tools/call", &json!({}), &archive, &())).is_none());
}

#[test]
fn an_endpoint_of_two_services_lists_their_tools_one_after_the_other() {
    let answered = answer_workspace(
        &Archive::default(),
        &request(&id(1), "tools/list", json!({})),
    )
    .unwrap();
    let names: Vec<&str> = answered["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "check-drafts",
            "check-many",
            "check-maybe",
            "check-named",
            "check-pair",
            "get-tagged",
            "get-version",
            "lookup-document",
            "rename-document",
            "touch-document",
            "get-balance"
        ]
    );
    assert_eq!(
        answered["result"]["_meta"],
        server_info("Workspace"),
        "the server is named by whoever hands the endpoint in"
    );
    assert_result("ListToolsResult", &answered);
}

#[test]
fn an_endpoint_routes_a_call_to_the_service_that_owns_the_tool() {
    let archive = Archive::default();
    let billed = answer_workspace(
        &archive,
        &call(
            &id(1),
            "get-balance",
            &json!({ "value": "acct-1", "x-ledger": "main" }),
        ),
    )
    .unwrap();
    assert_eq!(
        billed["result"]["structuredContent"],
        json!({ "cents": CENTS })
    );
    assert_result("CallToolResult", &billed);
    let documented = answer_workspace(
        &archive,
        &call(&id(2), "get-version", &json!({ "document_id": "doc-1" })),
    )
    .unwrap();
    assert_eq!(
        documented["result"]["structuredContent"],
        json!({ "version": VERSION_NUMBER })
    );
    let unknown =
        answer_workspace(&archive, &call(&id(3), "delete-everything", &json!({}))).unwrap();
    assert_eq!(unknown["error"]["code"], INVALID_PARAMS);
    assert_eq!(
        unknown["error"]["data"]["fault"]["kind"],
        "unknown-operation"
    );
}

#[test]
fn a_required_header_property_that_is_missing_answers_the_failed_validation_fault_naming_it() {
    let archive = Archive::default();
    let missing = answer_workspace(
        &archive,
        &call(&id(1), "get-balance", &json!({ "value": "acct-1" })),
    )
    .unwrap();
    assert_eq!(
        failure_text(&missing),
        "{\"fault\":{\"detail\":\"invalid type: null, expected a string\",\"field\":\"x-ledger\",\"kind\":\"failed-validation\",\"operation\":\"get-balance\"},\"isServiceFault\":true}"
    );
    let unreadable = answer_workspace(
        &archive,
        &call(
            &id(2),
            "get-balance",
            &json!({ "value": "acct-1", "x-ledger": 4_u8 }),
        ),
    )
    .unwrap();
    assert!(failure_text(&unreadable).contains("\"field\":\"x-ledger\""));
    assert_eq!(*archive.reached.lock().unwrap(), Vec::<String>::new());
}

fn extras() -> Value {
    json!({
        "capabilities": { "completions": {}, "prompts": {}, "resources": {} },
        "extensions": { "io.modelcontextprotocol/tasks": {} },
    })
}

#[test]
fn discover_answers_the_union_of_the_capabilities_with_the_extensions_inside_them() {
    static ENDPOINT: McpEndpoint = McpEndpoint::new(&[], &[document_mcp::catalog, extras]);
    let asked = request(&id(1), "server/discover", json!({}));
    let text = block_on(document_mcp::serve(
        &asked.to_string(),
        "Workspace",
        Some("Everything."),
        &ENDPOINT,
        |_call| (),
        document_mcp::McpHost::new(&document_mcp::McpInFlight::default(), &Lines::default()),
        &|_ctx: &(), _part, _method: &str, _params: &Value| Box::pin(async { None }),
    ))
    .unwrap();
    let answered: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        answered["result"]["capabilities"],
        json!({
            "completions": {},
            "extensions": { "io.modelcontextprotocol/tasks": {} },
            "prompts": {},
            "resources": {},
            "tools": {},
        })
    );
    assert_eq!(answered["result"]["instructions"], "Everything.");
    assert_result("DiscoverResult", &answered);
}

#[test]
fn a_tool_and_a_prompt_of_one_name_share_an_endpoint() {
    static ENDPOINT: McpEndpoint = McpEndpoint::new(
        &[&[("tool", "get-version")], &[("prompt", "get-version")]],
        &[],
    );
    assert_eq!(ENDPOINT.catalogs().len(), 0);
    assert_eq!(WORKSPACE.catalogs().len(), 2);
}

#[test]
fn a_second_service_answers_alone_over_stdio_and_over_http() {
    use crate::billing_mcp::{
        self, McpEndpoint as BillingEndpoint, McpHttpRequest, McpHttpSettings,
    };
    use crate::{BillingServiceSchema, DocumentServiceSchema};
    static ALONE: BillingEndpoint =
        BillingEndpoint::new(&[BillingServiceSchema::MCP_NAMES], &[billing_mcp::catalog]);
    let archive = Archive::default();
    let asked = call(
        &id(1),
        "get-balance",
        &json!({ "value": "acct-1", "x-ledger": "main" }),
    );
    let by_answer: Value = serde_json::from_str(
        &block_on(billing_mcp::answer(
            &asked.to_string(),
            &archive,
            |_call| (),
            &Lines::default(),
            &billing_mcp::McpInFlight::default(),
        ))
        .unwrap(),
    )
    .unwrap();
    let by_serve: Value = serde_json::from_str(
        &block_on(billing_mcp::serve(
            &asked.to_string(),
            billing_mcp::SERVER_NAME,
            billing_mcp::INSTRUCTIONS,
            &ALONE,
            |_call| (),
            billing_mcp::McpHost::new(&billing_mcp::McpInFlight::default(), &Lines::default()),
            &|ctx: &(), _part, method: &str, params: &Value| {
                let held = archive.clone();
                Box::pin(async move { billing_mcp::route(method, params, &held, ctx).await })
            },
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(by_answer, by_serve);
    assert_eq!(by_answer["result"]["_meta"], server_info("BillingService"));
    assert_eq!(
        by_answer["result"]["structuredContent"],
        json!({ "cents": CENTS })
    );
    let over_http = McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tools/call".to_owned()),
            ("Mcp-Name".to_owned(), "get-balance".to_owned()),
        ],
        asked.to_string().into_bytes(),
    );
    let settings = McpHttpSettings::new(&[], None);
    let response = through(|stream| {
        block_on(billing_mcp::answer_http(
            &over_http,
            &settings,
            stream,
            &archive,
            async |_request, _call| Ok(billing_mcp::McpAdmitted::new(())),
        ))
    });
    assert_eq!(response.status(), 200);
    let served = through(|stream| {
        block_on(billing_mcp::serve_http(
            &over_http,
            &settings,
            stream,
            &billing_mcp::McpServer::new(&ALONE, None, billing_mcp::SERVER_NAME),
            async |_request, _call| Ok(billing_mcp::McpAdmitted::new(())),
            &|ctx: &(), _part, method: &str, params: &Value| {
                let held = archive.clone();
                Box::pin(async move { billing_mcp::route(method, params, &held, ctx).await })
            },
        ))
    });
    assert_eq!(served, response);
    assert_eq!(
        DocumentServiceSchema::MCP_NAMES.len(),
        12,
        "ten tools, one resource and one template"
    );
}

fn spawned<Task>(task: Task) -> Task
where
    Task: Future + Send + 'static,
{
    task
}

#[test]
fn the_futures_of_serve_and_serve_http_are_send() {
    let text = request(&id(1), "tools/list", json!({})).to_string();
    drop(spawned(async move {
        let documents = Arc::new(Archive::default());
        let billing = Arc::new(Archive::default());
        let in_flight = document_mcp::McpInFlight::default();
        let lines = Lines::default();
        document_mcp::serve(
            &text,
            "Workspace",
            None,
            &WORKSPACE,
            |_call| (),
            document_mcp::McpHost::new(&in_flight, &lines),
            &|ctx: &(), part, method: &str, params: &Value| {
                let held_documents = Arc::clone(&documents);
                let held_billing = Arc::clone(&billing);
                Box::pin(async move {
                    match part {
                        0 => document_mcp::route(method, params, &*held_documents, ctx).await,
                        1 => billing_mcp::route(method, params, &*held_billing, ctx).await,
                        _ => None,
                    }
                })
            },
        )
        .await
    }));
    let asked = post(&request(&id(1), "tools/list", json!({})));
    drop(spawned(async move {
        let documents = Arc::new(Archive::default());
        let billing = Arc::new(Archive::default());
        let settings = document_mcp::McpHttpSettings::new(&[ORIGIN], None);
        let stream = Recorded::default();
        document_mcp::serve_http(
            &asked,
            &settings,
            &stream,
            &document_mcp::McpServer::new(&WORKSPACE, None, "Workspace"),
            async |_request: &document_mcp::McpHttpRequest, _call: document_mcp::McpCall| {
                Ok::<_, document_mcp::McpDenied>(document_mcp::McpAdmitted::new(()))
            },
            &|ctx: &(), part, method: &str, params: &Value| {
                let held_documents = Arc::clone(&documents);
                let held_billing = Arc::clone(&billing);
                Box::pin(async move {
                    match part {
                        0 => document_mcp::route(method, params, &*held_documents, ctx).await,
                        1 => billing_mcp::route(method, params, &*held_billing, ctx).await,
                        _ => None,
                    }
                })
            },
        )
        .await
    }));
}
