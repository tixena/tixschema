//! Resources, resource templates and completion: the listing across services, a read, a
//! completion, and the rules each applies, every answer checked against the official schema.

use crate::document_mcp::{self, McpHttpRequest, McpHttpSettings};
use crate::support::{
    INTERNAL_ERROR, INVALID_PARAMS, LISTED_FOR_MS, answer, answer_workspace, assert_error,
    assert_result, assert_valid, block_on, id, request, schema_errors, server_info, through,
};
use crate::tests::Archive;
use serde_json::{Value, json};

const SETTINGS: McpHttpSettings<'static> = McpHttpSettings {
    allowed_origins: &[],
    oauth: None,
};

const GUIDE: &str = "guide://configure-workspace";

/// An answer that is a JSON-RPC error whose body is the definition `error`.
fn refused(error: &str, answered: &Value) {
    assert_error("JSONRPCErrorResponse", answered);
    assert_valid(error, &answered["error"]);
}

fn guide() -> Value {
    json!({
        "uri": GUIDE,
        "name": "configuration-guide",
        "description": "The guide to the configuration of a workspace.",
        "mimeType": "text/markdown",
    })
}

fn listed(cursor: Option<&str>) -> Value {
    let params = cursor.map_or_else(|| json!({}), |held| json!({ "cursor": held }));
    let asked = request(&id(1), "resources/list", params);
    assert_valid("ListResourcesRequest", &asked);
    answer_workspace(&Archive::default(), &asked).unwrap()
}

fn read(uri: &str) -> Value {
    read_with(&Archive::default(), uri)
}

fn read_with(archive: &Archive, uri: &str) -> Value {
    let asked = request(&id(1), "resources/read", json!({ "uri": uri }));
    assert_valid("ReadResourceRequest", &asked);
    answer_workspace(archive, &asked).unwrap()
}

fn completion_request(parameter: &str, typed: &str, chosen: &Value) -> Value {
    request(
        &id(1),
        "completion/complete",
        json!({
            "ref": { "type": "ref/resource", "uri": "workspace://documents/{document_id}" },
            "argument": { "name": parameter, "value": typed },
            "context": { "arguments": chosen },
        }),
    )
}

fn complete(archive: &Archive, parameter: &str, typed: &str, chosen: &Value) -> Value {
    let asked = completion_request(parameter, typed, chosen);
    assert_valid("CompleteRequest", &asked);
    answer_workspace(archive, &asked).unwrap()
}

#[test]
fn the_first_page_holds_the_fixed_resources_then_the_first_listing() {
    let answered = listed(None);
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "complete",
                "resources": [
                    guide(),
                    { "uri": "workspace://documents/doc-1", "name": "id-card.pdf", "mimeType": "application/pdf" },
                ],
                "nextCursor": "0:page-2",
                "ttlMs": 0_u8,
                "cacheScope": "private",
                "_meta": server_info("Workspace"),
            },
        })
    );
    assert_result("ListResourcesResult", &answered);
}

#[test]
fn a_cursor_resumes_the_service_it_names_and_moves_on_when_that_service_is_done() {
    let second = listed(Some("0:page-2"));
    assert_eq!(
        second["result"]["resources"],
        json!([{ "uri": "workspace://documents/doc-2", "name": "passport.pdf", "mimeType": "application/pdf" }])
    );
    assert_eq!(second["result"]["nextCursor"], "1:");
    assert_result("ListResourcesResult", &second);

    let last = listed(Some("1:"));
    assert_eq!(
        last["result"]["resources"],
        json!([{ "uri": "billing://invoices/inv-1", "name": "inv-1" }])
    );
    assert_eq!(last["result"].get("nextCursor"), None);
    assert_result("ListResourcesResult", &last);
}

#[test]
fn a_listing_answers_the_ttl_and_the_scope_its_operation_wrote() {
    let last = listed(Some("1:"));
    assert_eq!(last["result"]["ttlMs"], 60_000_u32);
    assert_eq!(last["result"]["cacheScope"], "public");
}

#[test]
fn a_cursor_that_names_no_service_is_refused_with_32602() {
    for cursor in ["7:", "x", "0", "-1:a", ":a"] {
        let answered = listed(Some(cursor));
        assert_eq!(answered["error"]["code"], INVALID_PARAMS, "{cursor}");
        assert_eq!(answered["error"]["message"], "Invalid cursor");
        refused("InvalidParamsError", &answered);
    }
    let asked = request(&id(1), "resources/list", json!({ "cursor": 4_u8 }));
    let answered = answer_workspace(&Archive::default(), &asked).unwrap();
    assert_eq!(answered["error"]["code"], INVALID_PARAMS);
}

#[test]
fn a_service_alone_pages_its_own_listing() {
    let first = answer(
        &Archive::default(),
        &request(&id(1), "resources/list", json!({})),
    )
    .unwrap();
    assert_eq!(first["result"]["resources"][0], guide());
    assert_eq!(first["result"]["nextCursor"], "0:page-2");
    assert_result("ListResourcesResult", &first);
    let archive = Archive::default();
    let second = answer(
        &archive,
        &request(&id(2), "resources/list", json!({ "cursor": "0:page-2" })),
    )
    .unwrap();
    assert_eq!(second["result"]["resources"][0]["name"], "passport.pdf");
    assert_eq!(second["result"].get("nextCursor"), None);
    assert_eq!(
        *archive.reached.lock().unwrap(),
        ["list-documents Some(\"page-2\")"]
    );
}

#[test]
fn the_templates_are_fixed_and_answer_what_tools_list_answers() {
    let asked = request(&id(1), "resources/templates/list", json!({}));
    assert_valid("ListResourceTemplatesRequest", &asked);
    let answered = answer_workspace(&Archive::default(), &asked).unwrap();
    assert_eq!(
        answered["result"],
        json!({
            "resultType": "complete",
            "resourceTemplates": [
                { "uriTemplate": "workspace://documents/{document_id}",
                  "name": "document-file",
                  "description": "The file of one document." },
                { "uriTemplate": "billing://invoices/{invoice_id}",
                  "name": "get-invoice",
                  "description": "One invoice.",
                  "mimeType": "application/json" },
                { "uriTemplate": "billing://ledgers/{ledger}/invoices/{invoice_id}",
                  "name": "get-ledger-invoice",
                  "description": "One invoice of one ledger." },
            ],
            "ttlMs": LISTED_FOR_MS,
            "cacheScope": "public",
            "_meta": server_info("Workspace"),
        })
    );
    assert_result("ListResourceTemplatesResult", &answered);
}

#[test]
fn a_fixed_resource_reads_as_text_with_its_mime_type() {
    let answered = read(GUIDE);
    assert_eq!(
        answered["result"],
        json!({
            "resultType": "complete",
            "contents": [{ "uri": GUIDE, "mimeType": "text/markdown", "text": "# Configure the workspace" }],
            "ttlMs": 0_u8,
            "cacheScope": "private",
            "_meta": server_info("Workspace"),
        })
    );
    assert_result("ReadResourceResult", &answered);
}

#[test]
fn a_template_reads_bytes_as_base64_with_the_content_type_the_pair_holds() {
    let archive = Archive::default();
    let answered = read_with(&archive, "workspace://documents/doc-1");
    assert_eq!(
        answered["result"]["contents"],
        json!([{ "uri": "workspace://documents/doc-1", "mimeType": "application/pdf", "blob": "JVBERi0=" }])
    );
    assert_result("ReadResourceResult", &answered);
    assert_eq!(*archive.reached.lock().unwrap(), ["document-file doc-1"]);
}

#[test]
fn several_placeholders_fill_the_fields_of_the_message_and_the_operation_sets_the_ttl() {
    let archive = Archive::default();
    let answered = read_with(&archive, "billing://ledgers/main/invoices/inv-1");
    assert_eq!(
        answered["result"]["contents"],
        json!([{ "uri": "billing://ledgers/main/invoices/inv-1", "text": "main/inv-1" }])
    );
    assert_eq!(
        *archive.reached.lock().unwrap(),
        ["get-ledger-invoice main inv-1"]
    );
    let invoice = read("billing://invoices/inv-1");
    assert_eq!(invoice["result"]["ttlMs"], 1_000_u32);
    assert_eq!(invoice["result"]["cacheScope"], "public");
    assert_eq!(
        invoice["result"]["contents"][0]["mimeType"],
        "application/json"
    );
    assert_result("ReadResourceResult", &invoice);
}

#[test]
fn a_placeholder_takes_one_non_empty_run_up_to_the_next_slash() {
    for uri in [
        "workspace://documents/",
        "workspace://documents/doc-1/extra",
        "workspace://documents",
        "billing://ledgers//invoices/inv-1",
    ] {
        let answered = read(uri);
        assert_eq!(answered["error"]["code"], INVALID_PARAMS, "{uri}");
        assert_eq!(
            answered["error"]["data"]["fault"]["kind"],
            "unknown-operation"
        );
        refused("InvalidParamsError", &answered);
    }
}

#[test]
fn a_declared_error_answers_32602_with_the_error_in_data() {
    let answered = read("workspace://documents/missing");
    assert_eq!(
        answered["error"],
        json!({
            "code": INVALID_PARAMS,
            "message": "Resource not found",
            "data": { "errorCode": "not-found" },
        })
    );
    refused("InvalidParamsError", &answered);
}

#[test]
fn a_panic_answers_32603_with_the_fault() {
    let answered = read("workspace://documents/panic");
    assert_eq!(answered["error"]["code"], INTERNAL_ERROR);
    assert_eq!(answered["error"]["data"]["fault"]["kind"], "handler-panic");
    refused("InternalError", &answered);
}

#[test]
fn a_resource_is_no_tool() {
    let asked = request(
        &id(1),
        "tools/call",
        json!({ "name": "configuration-guide", "arguments": {} }),
    );
    let answered = answer_workspace(&Archive::default(), &asked).unwrap();
    assert_eq!(answered["error"]["code"], INVALID_PARAMS);
}

#[test]
fn a_listing_that_fails_or_panics_is_answered_like_a_read() {
    for (cursor, code) in [("0:nowhere", INVALID_PARAMS), ("0:panic", INTERNAL_ERROR)] {
        let answered = listed(Some(cursor));
        assert_eq!(answered["error"]["code"], code, "{cursor}");
        refused(
            if code == INTERNAL_ERROR {
                "InternalError"
            } else {
                "InvalidParamsError"
            },
            &answered,
        );
    }
}

#[test]
fn a_completion_calls_the_operation_with_the_typed_text_and_the_chosen_values() {
    let archive = Archive::default();
    let answered = complete(&archive, "document_id", "doc", &json!({ "kind": "pdf" }));
    assert_eq!(
        answered["result"],
        json!({
            "resultType": "complete",
            "completion": { "values": ["doc-1", "doc-2"], "hasMore": false },
            "_meta": server_info("Workspace"),
        })
    );
    assert_result("CompleteResult", &answered);
    assert_eq!(
        *archive.reached.lock().unwrap(),
        ["suggest-documents doc Some(\"pdf\")"]
    );
}

#[test]
fn a_completion_holds_at_most_100_values_and_says_how_many_there_were() {
    let answered = complete(&Archive::default(), "document_id", "many", &json!({}));
    let completion = &answered["result"]["completion"];
    assert_eq!(completion["values"].as_array().unwrap().len(), 100);
    assert_eq!(completion["values"][99], "many-99");
    assert_eq!(completion["total"], 150_u32);
    assert_eq!(completion["hasMore"], true);
    assert_result("CompleteResult", &answered);
}

#[test]
fn a_completion_of_a_parameter_nothing_completes_is_refused() {
    let answered = complete(&Archive::default(), "page", "doc", &json!({}));
    assert_eq!(answered["error"]["code"], INVALID_PARAMS);
    refused("InvalidParamsError", &answered);
}

#[test]
fn a_completion_that_fails_or_panics_is_answered_like_a_read() {
    let declared = complete(&Archive::default(), "document_id", "none", &json!({}));
    assert_eq!(declared["error"]["code"], INVALID_PARAMS);
    assert_eq!(
        declared["error"]["data"],
        json!({ "errorCode": "not-found" })
    );
    let panicked = complete(&Archive::default(), "document_id", "panic", &json!({}));
    assert_eq!(panicked["error"]["code"], INTERNAL_ERROR);
}

#[test]
fn a_completion_whose_context_is_not_strings_answers_a_fault_and_calls_nothing() {
    let archive = Archive::default();
    let unread = completion_request("document_id", "doc", &json!({ "kind": 4_u8 }));
    assert_ne!(
        schema_errors("CompleteRequest", &unread),
        Vec::<String>::new()
    );
    let answered = answer_workspace(&archive, &unread).unwrap();
    assert_eq!(answered["error"]["code"], INVALID_PARAMS);
    assert_eq!(
        answered["error"]["data"]["fault"]["kind"],
        "undeserializable-payload"
    );
    assert_eq!(*archive.reached.lock().unwrap(), Vec::<String>::new());
}

#[test]
fn a_resource_is_read_over_http_by_the_uri_in_its_name_header() {
    let message = request(
        &id(1),
        "resources/read",
        json!({ "uri": "workspace://documents/doc-1" }),
    );
    let mut asked = McpHttpRequest {
        body: message.to_string().into_bytes(),
        headers: vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "resources/read".to_owned()),
            (
                "Mcp-Name".to_owned(),
                "workspace://documents/doc-1".to_owned(),
            ),
        ],
        method: "POST".to_owned(),
        path: "/mcp".to_owned(),
    };
    let response = through(|stream| {
        block_on(document_mcp::answer_http(
            &asked,
            &SETTINGS,
            stream,
            &Archive::default(),
            async |_request, _call| Ok(()),
        ))
    });
    assert_eq!(response.status, 200);
    let sent: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(sent["result"]["contents"][0]["blob"], "JVBERi0=");
    asked.headers.pop();
    asked.headers.push((
        "Mcp-Name".to_owned(),
        "workspace://documents/doc-2".to_owned(),
    ));
    let mismatched = through(|stream| {
        block_on(document_mcp::answer_http(
            &asked,
            &SETTINGS,
            stream,
            &Archive::default(),
            async |_request, _call| Ok(()),
        ))
    });
    assert_eq!(mismatched.status, 400);
}

#[test]
fn the_names_an_endpoint_checks_hold_the_resources_and_the_templates() {
    use crate::{BillingServiceSchema, DocumentServiceSchema};
    assert!(DocumentServiceSchema::MCP_NAMES.contains(&("resource", GUIDE)));
    assert!(
        DocumentServiceSchema::MCP_NAMES
            .contains(&("resource template", "workspace://documents/{document_id}"))
    );
    assert!(BillingServiceSchema::MCP_NAMES.contains(&(
        "resource template",
        "billing://ledgers/{ledger}/invoices/{invoice_id}"
    )));
}
