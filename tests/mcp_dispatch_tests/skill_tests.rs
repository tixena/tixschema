//! Skills: the folder a service ships and the skills its operations write at run time, through
//! `skills/list`, `skills/get` and `resources/read`. Every read passes the official schema; the
//! extension publishes none for the two skills methods.

use crate::ExportServiceSchema;
use crate::export_mcp;
use crate::support::{
    INTERNAL_ERROR, INVALID_PARAMS, LISTED_FOR_MS, Lines, assert_error, assert_result,
    assert_valid, block_on, id, request, server_info,
};
use crate::tests::{Archive, Reporting};
use serde_json::{Value, json};
use std::fs::read_to_string;
use std::panic::catch_unwind;

const CHECKLIST_DIGEST: &str =
    "sha256:31cd46b42fd3c2fc1b1ecc9f462fab419bd69901a67b214fca0b4e62ad36c02e";
const MANIFEST_DIGEST: &str =
    "sha256:58e2de02d4cda1450548bf7137c2ff9f1f9c5c5280fa9946179cd3e806d1943d";
const PACK_DIGEST: &str = "sha256:c99452aa05b9764f6630d127bc93dddf7917b7bd42e0acf95ee9cae931211c34";
const SEAL_DIGEST: &str = "sha256:b48a58f432cdfa61d362e41c68bb346b0b3eb3f763ebfdc2cb0047bb30b1ae0e";

const RELEASE_NOTES: &str = "Writes the release notes of the caller's workspace: \"what changed\", from its latest changes.";

fn ask(method: &str, params: Value) -> Value {
    let asked = request(&id(1), method, params);
    block_on(export_mcp::answer(
        &asked.to_string(),
        &Archive::default(),
        |_call| Reporting::new(|_done, _total, _message| {}, || false),
        &Lines::default(),
        &export_mcp::McpInFlight::default(),
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
    .unwrap()
}

fn read(uri: &str) -> Value {
    let asked = request(&id(1), "resources/read", json!({ "uri": uri }));
    assert_valid("ReadResourceRequest", &asked);
    ask("resources/read", json!({ "uri": uri }))
}

fn code_review() -> Value {
    json!({
        "uri": "skill://code-review/SKILL.md",
        "frontmatter": {
            "name": "code-review",
            "description": "Review code using the team's checklist.",
        },
        "resources": [
            { "uri": "skill://code-review/SKILL.md", "digest": MANIFEST_DIGEST, "size": 164_u32 },
            { "uri": "skill://code-review/assets/seal.bin", "digest": SEAL_DIGEST, "size": 9_u32 },
            { "uri": "skill://code-review/references/checklist.md", "digest": CHECKLIST_DIGEST, "size": 76_u32 },
        ],
    })
}

fn pack_parcels() -> Value {
    json!({
        "uri": "skill://pack-parcels/SKILL.md",
        "frontmatter": {
            "name": "pack-parcels",
            "description": "Pack a parcel for the carrier: weigh it, wrap it, then label it.\n",
            "license": "MIT",
        },
        "resources": [
            { "uri": "skill://pack-parcels/SKILL.md", "digest": PACK_DIGEST, "size": 151_u32 },
        ],
    })
}

fn release_notes() -> Value {
    json!({
        "uri": "skill://release-notes/SKILL.md",
        "frontmatter": { "name": "release-notes", "description": RELEASE_NOTES },
        "resources": "dynamic",
    })
}

fn workspace_figures() -> Value {
    json!({
        "uri": "skill://workspace-figures/SKILL.md",
        "frontmatter": {
            "name": "workspace-figures",
            "description": "Writes the figures of the caller's workspace.",
        },
        "resources": "dynamic",
    })
}

/// The frontmatter at the head of `text`, read as YAML.
fn frontmatter_of(text: &str) -> Value {
    let body = text.strip_prefix("---\n").unwrap();
    let (front, _) = body.split_once("\n---\n").unwrap();
    yaml_serde::from_str(front).unwrap()
}

#[test]
fn discover_names_the_resources_capability_and_the_skills_extension_together() {
    let answered = ask("server/discover", json!({}));
    assert_eq!(
        answered["result"]["capabilities"],
        json!({
            "resources": {},
            "tools": {},
            "extensions": { "io.modelcontextprotocol/skills": {} },
        })
    );
    assert_result("DiscoverResult", &answered);
}

#[test]
fn skills_list_answers_the_folder_then_the_operations_and_is_not_cached() {
    let answered = ask("skills/list", json!({}));
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "complete",
                "skills": [code_review(), pack_parcels(), workspace_figures(), release_notes()],
                "ttlMs": 0_u8,
                "cacheScope": "private",
                "_meta": server_info("ExportService"),
            },
        })
    );
    assert_valid("JSONRPCResultResponse", &answered);
}

#[test]
fn skills_get_answers_an_entry_by_name_or_by_uri() {
    let by_name = ask("skills/get", json!({ "name": "code-review" }));
    assert_eq!(by_name["result"]["skill"], code_review());
    assert_eq!(by_name["result"]["ttlMs"], LISTED_FOR_MS);
    assert_eq!(by_name["result"]["cacheScope"], "public");
    assert_valid("JSONRPCResultResponse", &by_name);

    let by_uri = ask(
        "skills/get",
        json!({ "uri": "skill://release-notes/SKILL.md" }),
    );
    assert_eq!(by_uri["result"]["skill"], release_notes());
    assert_eq!(by_uri["result"]["ttlMs"], 0_u8);
    assert_eq!(by_uri["result"]["cacheScope"], "private");
}

#[test]
fn skills_get_of_a_skill_nobody_has_is_32602_with_the_unknown_operation_fault() {
    let answered = ask("skills/get", json!({ "name": "no-such-skill" }));
    assert_eq!(answered["error"]["code"], INVALID_PARAMS);
    assert_eq!(answered["error"]["data"]["isServiceFault"], true);
    assert_error("JSONRPCErrorResponse", &answered);

    let nameless = ask("skills/get", json!({}));
    assert_eq!(nameless["error"]["code"], INVALID_PARAMS);
}

#[test]
fn a_file_of_the_folder_is_read_as_text_with_its_type() {
    let answered = read("skill://code-review/references/checklist.md");
    assert_eq!(
        answered["result"]["contents"],
        json!([{
            "uri": "skill://code-review/references/checklist.md",
            "mimeType": "text/markdown",
            "text": "# Checklist\n\n- Does each function do one thing?\n- Is every failure handled?\n",
        }])
    );
    assert_eq!(answered["result"]["ttlMs"], LISTED_FOR_MS);
    assert_result("ReadResourceResult", &answered);
}

#[test]
fn a_binary_file_of_the_folder_is_read_as_a_blob() {
    let answered = read("skill://code-review/assets/seal.bin");
    assert_eq!(
        answered["result"]["contents"],
        json!([{
            "uri": "skill://code-review/assets/seal.bin",
            "mimeType": "application/octet-stream",
            "blob": "AP+AAXNlYWz+",
        }])
    );
    assert_result("ReadResourceResult", &answered);
}

#[test]
fn a_file_the_folder_does_not_hold_is_not_found() {
    let answered = read("skill://code-review/references/missing.md");
    assert_eq!(answered["error"]["code"], INVALID_PARAMS);
    assert_error("JSONRPCErrorResponse", &answered);
}

#[test]
fn the_manifest_a_read_answers_is_the_file_the_digest_names() {
    let answered = read("skill://code-review/SKILL.md");
    let text = answered["result"]["contents"][0]["text"].as_str().unwrap();
    assert_eq!(text.len(), 164);
    assert_eq!(
        frontmatter_of(text),
        code_review()["frontmatter"],
        "the SKILL.md a host reads says what the entry says"
    );
    assert_eq!(
        frontmatter_of(&read_to_string("tests/mcp_skills/pack-parcels/SKILL.md").unwrap()),
        pack_parcels()["frontmatter"]
    );
}

#[test]
fn a_dynamic_skill_writes_its_frontmatter_ahead_of_the_handlers_text() {
    let answered = read("skill://release-notes/SKILL.md");
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "complete",
                "contents": [{
                    "uri": "skill://release-notes/SKILL.md",
                    "mimeType": "text/markdown",
                    "text": "---\nname: release-notes\ndescription: \"Writes the release notes of the caller's workspace: \\\"what changed\\\", from its latest changes.\"\n---\n\n# Release notes\n\nRead the changes since 2026-11-01, then write one line per change.\n",
                }],
                "ttlMs": 0_u8,
                "cacheScope": "private",
                "_meta": server_info("ExportService"),
            },
        })
    );
    assert_result("ReadResourceResult", &answered);
    let text = answered["result"]["contents"][0]["text"].as_str().unwrap();
    assert_eq!(
        frontmatter_of(text),
        release_notes()["frontmatter"],
        "the frontmatter read back as YAML is the entry's"
    );
}

#[test]
fn a_supporting_file_of_a_dynamic_skill_is_the_handlers_text_alone() {
    let answered = read("skill://release-notes/references/changes.md");
    assert_eq!(
        answered["result"]["contents"],
        json!([{
            "uri": "skill://release-notes/references/changes.md",
            "mimeType": "text/markdown",
            "text": "- renamed a document\n",
        }])
    );
    assert_result("ReadResourceResult", &answered);
}

#[test]
fn a_dynamic_skill_that_answers_bytes_reads_as_a_blob() {
    let answered = read("skill://workspace-figures/figures.bin");
    assert_eq!(
        answered["result"]["contents"],
        json!([{
            "uri": "skill://workspace-figures/figures.bin",
            "mimeType": "application/octet-stream",
            "blob": "AP+AAQ==",
        }])
    );
    assert_result("ReadResourceResult", &answered);
}

#[test]
fn a_declared_error_is_32602_with_the_errors_json_and_a_panic_is_32603() {
    let missing = read("skill://release-notes/references/missing.md");
    assert_eq!(missing["error"]["code"], INVALID_PARAMS);
    assert_eq!(
        missing["error"]["data"],
        json!({ "errorCode": "not-found" })
    );
    assert_error("JSONRPCErrorResponse", &missing);

    let panicked = read("skill://release-notes/panic");
    assert_eq!(panicked["error"]["code"], INTERNAL_ERROR);
    assert_error("JSONRPCErrorResponse", &panicked);
}

#[test]
fn the_names_hold_each_skill_so_two_services_cannot_share_one() {
    let names = ExportServiceSchema::MCP_NAMES;
    for skill in [
        "code-review",
        "pack-parcels",
        "release-notes",
        "workspace-figures",
    ] {
        assert!(names.contains(&("skill", skill)), "{skill} in {names:?}");
    }
    let shared = catch_unwind(|| {
        export_mcp::McpEndpoint::new(
            &[
                ExportServiceSchema::MCP_NAMES,
                ExportServiceSchema::MCP_NAMES,
            ],
            &[export_mcp::catalog, export_mcp::catalog],
        )
    });
    assert!(shared.is_err());
}
