//! What the `mcp` transport reads of a trait, and what it refuses at expansion.

#![cfg(feature = "serde")]

use super::mcp_refusals::refusals;
use super::parse::{OperationDef, ServiceDef, parse_service};
use super::transport::Transport;
use super::{exec_service_schema, stream_envelope_refusal};
use proc_macro2::TokenStream;
use quote::{ToTokens as _, quote};
use syn::ItemTrait;

const DOCUMENTED: &str = r#"
    /// Reads and renames documents.
    ///
    /// Every call is made on behalf of one caller.
    pub trait DocumentService<Ctx> {
        /// Answers the current version of one document.
        #[service_schema_op(mcp(title = "Current Document Version", name = "lookup-document"))]
        async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
            -> Result<VersionResponse, VersionError>;

        /// Renames one document.
        async fn rename_document(&self, ctx: &Ctx, document_id: String, title: String)
            -> Result<(), VersionError>;
    }
"#;

const INTERVIEWS: &str = r#"
    pub trait DocumentService<Ctx> {
        /// Deletes one document.
        #[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = confirmation)))]
        async fn delete_document(&self, ctx: &Ctx, document_id: String, confirmation: Option<DeleteConfirmation>)
            -> Result<(), DeleteError>;

        /// Imports one file.
        #[service_schema_op(mcp(interview(arm = "NeedsDriveConnection", url)))]
        async fn import_from_drive(&self, ctx: &Ctx, file_id: String)
            -> Result<DocumentId, ImportError>;

        /// The preview of one document.
        #[service_schema_op(mcp(content))]
        async fn document_preview(&self, ctx: &Ctx, document_id: String)
            -> Result<(Vec<u8>, String), VersionError>;
    }
"#;

const RESOURCES: &str = r#"
    pub trait DocumentService<Ctx> {
        /// The guide.
        #[service_schema_op(mcp(resource = "guide://configure", mime_type = "text/markdown", ttl_ms = 5, cache_scope = "public"))]
        async fn configuration_guide(&self, ctx: &Ctx) -> Result<String, VersionError>;

        /// Lists the documents.
        #[service_schema_op(mcp(list_resources))]
        async fn list_documents(&self, ctx: &Ctx, cursor: Option<String>)
            -> Result<McpResourcePage, VersionError>;

        /// The file of one document.
        #[service_schema_op(mcp(resource = "workspace://documents/{document_id}"))]
        async fn document_file(&self, ctx: &Ctx, document_id: String)
            -> Result<(Vec<u8>, String), VersionError>;

        /// Suggests documents.
        #[service_schema_op(mcp(completes(resource = "document_file", parameter = "document_id")))]
        async fn suggest_documents(&self, ctx: &Ctx, typed: String, chosen: HashMap<String, String>)
            -> Result<Vec<String>, VersionError>;

        /// Answers the version.
        async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
            -> Result<VersionResponse, VersionError>;
    }
"#;

const PROMPTS: &str = r#"
    pub trait DocumentService<Ctx> {
        /// Guides the model.
        #[service_schema_op(mcp(prompt, title = "Guide", scopes = ["prompts:read"]))]
        async fn configure_organization(&self, ctx: &Ctx) -> Result<String, SetupError>;

        /// Reviews one document type.
        #[service_schema_op(mcp(prompt, name = "review", icon = "https://example.com/r.png"))]
        async fn review_document_type(&self, ctx: &Ctx, document_type: String, focus: Option<String>)
            -> Result<Vec<McpPromptMessage>, SetupError>;

        /// Suggests types.
        #[service_schema_op(mcp(completes(prompt = "review_document_type", parameter = "document_type")))]
        async fn suggest_types(&self, ctx: &Ctx, typed: String, chosen: HashMap<String, String>)
            -> Result<Vec<String>, SetupError>;

        /// The version.
        async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
            -> Result<VersionResponse, VersionError>;
    }
"#;

fn declared(source: &str) -> ItemTrait {
    syn::parse_str::<ItemTrait>(source).unwrap()
}

fn service(source: &str) -> ServiceDef {
    parse_service(&declared(source)).unwrap()
}

fn parse_refusals(source: &str) -> Vec<String> {
    parse_service(&declared(source))
        .err()
        .map_or_else(Vec::new, |refusal| {
            refusal.into_iter().map(|one| one.to_string()).collect()
        })
}

fn mcp_refusals_of(source: &str) -> Vec<String> {
    refusals(&service(source)).map_or_else(Vec::new, |refusal| {
        refusal.into_iter().map(|one| one.to_string()).collect()
    })
}

fn expansion(arguments: TokenStream, source: &str) -> String {
    exec_service_schema(arguments, declared(source).to_token_stream()).to_string()
}

#[test]
fn the_rustdoc_of_the_trait_and_of_each_method_is_captured() {
    let read = service(DOCUMENTED);
    assert_eq!(
        read.doc.as_deref(),
        Some("Reads and renames documents.\n\nEvery call is made on behalf of one caller.")
    );
    assert_eq!(
        read.operations[0].doc.as_deref(),
        Some("Answers the current version of one document.")
    );
    assert_eq!(
        read.operations[1].doc.as_deref(),
        Some("Renames one document.")
    );
}

#[test]
fn a_trait_and_a_method_without_rustdoc_hold_none() {
    let read = service(
        "pub trait DocumentService<Ctx> {
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }",
    );
    assert_eq!(read.doc, None);
    assert_eq!(read.operations[0].doc, None);
}

#[test]
fn the_tool_name_and_title_are_the_ones_written_or_the_defaults() {
    let read = service(DOCUMENTED);
    assert_eq!(read.operations[0].mcp_tool_name(), "lookup-document");
    assert_eq!(
        read.operations[0].mcp_tool_title(),
        "Current Document Version"
    );
    assert_eq!(read.operations[1].mcp_tool_name(), "rename-document");
    assert_eq!(read.operations[1].mcp_tool_title(), "Rename Document");
}

#[test]
fn the_tool_name_defaults_to_the_wire_name_the_message_key_moves() {
    let read = service(
        r#"pub trait DocumentService<Ctx> {
            /// Answers the version.
            #[service_schema_op(message = "version-of")]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }"#,
    );
    assert_eq!(read.operations[0].mcp_tool_name(), "version-of");
}

#[test]
fn every_key_of_the_group_is_read() {
    let read = service(
        r#"pub trait DocumentService<Ctx> {
            #[service_schema_op(mcp(
                title = "Rename",
                name = "rename",
                icon = "https://example.com/rename.png",
                read_only,
                destructive = false,
                idempotent,
                open_world = true,
                header(document_id = "Document-Id")
            ))]
            async fn rename_document(&self, ctx: &Ctx, document_id: String, title: String)
                -> Result<(), VersionError>;
        }"#,
    );
    let mcp = &read.operations[0].mcp;
    assert_eq!(mcp.title.as_deref(), Some("Rename"));
    assert_eq!(mcp.name.as_deref(), Some("rename"));
    assert_eq!(mcp.icon.as_deref(), Some("https://example.com/rename.png"));
    assert!(mcp.read_only);
    assert_eq!(mcp.destructive, Some(false));
    assert!(mcp.idempotent);
    assert_eq!(mcp.open_world, Some(true));
    assert_eq!(mcp.header.len(), 1);
    assert_eq!(mcp.header[0].0.to_string(), "document_id");
    assert_eq!(mcp.header[0].1, "Document-Id");
}

#[test]
fn an_empty_title_name_or_icon_is_refused_naming_the_operation() {
    for key in ["title", "name", "icon"] {
        let reported = parse_refusals(&format!(
            r#"pub trait DocumentService<Ctx> {{
                #[service_schema_op(mcp({key} = ""))]
                async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                    -> Result<VersionResponse, VersionError>;
            }}"#
        ));
        assert_eq!(
            reported,
            [format!(
                "service_schema: operation `get_version` writes an empty `{key}` in `mcp(...)`\n       \
                 write the text, or leave `{key}` out"
            )]
        );
    }
}

#[test]
fn a_key_the_group_does_not_take_is_refused_naming_the_operation() {
    let reported = parse_refusals(
        "pub trait DocumentService<Ctx> {
            #[service_schema_op(mcp(task))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }",
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `get_version` writes a key `mcp(...)` does not take\n       \
             the keys are `title`, `name`, `icon`, `read_only`, `destructive`, `idempotent`, \
             `open_world`, `scopes = [\"...\"]`, `header(<argument> = \"<Header-Name>\")`, `resource`, \
             `mime_type`, `list_resources`, `prompt`, \
             `completes(resource = \"<operation>\", parameter = \"<argument>\")` or \
             `completes(prompt = \"<operation>\", parameter = \"<argument>\")`, `ttl_ms`, \
             `cache_scope`, `skill`, `content` and `interview(arm = \"<Arm>\", answers = <argument>)` or \
             `interview(arm = \"<Arm>\", url)`"
                .to_owned()
        ]
    );
}

#[test]
fn the_unknown_directive_message_lists_the_mcp_group() {
    let reported = parse_refusals(
        "pub trait DocumentService<Ctx> {
            #[service_schema_op(fire_and_forget)]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }",
    );
    assert!(
        reported[0].contains("`http(...)` and `mcp(...)`"),
        "{reported:?}"
    );
}

#[test]
fn a_flag_written_with_the_wrong_shape_is_refused() {
    let flag = parse_refusals(
        "pub trait DocumentService<Ctx> {
            #[service_schema_op(mcp(read_only = true))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }",
    );
    assert_eq!(
        flag,
        [
            "service_schema: operation `get_version` writes `read_only = ...` in `mcp(...)`\n       \
             `read_only` is a bare flag and takes no value; write `read_only`"
        ]
    );
    let valued = parse_refusals(
        "pub trait DocumentService<Ctx> {
            #[service_schema_op(mcp(destructive))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }",
    );
    assert_eq!(
        valued,
        [
            "service_schema: operation `get_version` writes `destructive` in `mcp(...)` with no \
             value\n       write `destructive = true` or `destructive = false`"
        ]
    );
}

#[test]
fn a_header_that_marks_no_property_of_the_input_is_refused() {
    let reported = parse_refusals(
        r#"pub trait DocumentService<Ctx> {
            #[service_schema_op(mcp(header(missing = "Document-Id")))]
            async fn rename_document(&self, ctx: &Ctx, document_id: String, title: String)
                -> Result<(), VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `rename_document` marks `missing` with \
             `header(missing = \"Document-Id\")` in `mcp(...)`, and `missing` is no property of \
             the tool's input\n       \
             name an argument of a message the macro declares, or one a `header_in` binds"
        ]
    );
}

#[test]
fn an_operation_without_rustdoc_is_refused_on_an_mcp_service_only() {
    let source = "pub trait DocumentService<Ctx> {
        async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
            -> Result<VersionResponse, VersionError>;
    }";
    assert_eq!(
        mcp_refusals_of(source),
        [
            "service_schema: operation `get_version` has no rustdoc, and this service asks for \
             `mcp`\n       \
             a tool's description is its method's rustdoc; write a `///` comment on the method"
        ]
    );
    assert!(
        !expansion(quote! { transports = ["ws_rpc"] }, source).contains("compile_error"),
        "a service that does not ask for mcp needs no rustdoc"
    );
    assert!(expansion(quote! { transports = ["mcp"] }, source).contains("compile_error"));
}

#[test]
fn a_tool_name_outside_the_set_or_over_the_limit_is_refused() {
    let outside = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Answers the version.
            #[service_schema_op(mcp(name = "get version"))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }"#,
    );
    let message = "service_schema: operation `get_version` is the tool `get version`, and this \
                   service asks for `mcp`\n       \
                   a tool name is at most 128 characters of `A-Z`, `a-z`, `0-9`, `_`, `-` and \
                   `.`; write `mcp(name = \"...\")` with one";
    assert_eq!(outside, [message]);
    let long = "a".repeat(129);
    let over = mcp_refusals_of(&format!(
        r#"pub trait DocumentService<Ctx> {{
            /// Answers the version.
            #[service_schema_op(mcp(name = "{long}"))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }}"#
    ));
    assert_eq!(over.len(), 1);
    assert!(over[0].contains(&format!("the tool `{long}`")), "{over:?}");
    let within = mcp_refusals_of(&format!(
        r#"pub trait DocumentService<Ctx> {{
            /// Answers the version.
            #[service_schema_op(mcp(name = "{}"))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }}"#,
        "a".repeat(128)
    ));
    assert!(within.is_empty(), "{within:?}");
}

#[test]
fn a_wire_name_override_is_held_to_the_same_set() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Answers the version.
            #[service_schema_op(message = "version/of")]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }"#,
    );
    assert_eq!(reported.len(), 1);
    assert!(
        reported[0].contains("the tool `version/of`"),
        "{reported:?}"
    );
}

#[test]
fn two_operations_with_one_tool_name_are_refused_naming_both() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Answers the version.
            #[service_schema_op(mcp(name = "version"))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;

            /// Answers the version again.
            #[service_schema_op(mcp(name = "version"))]
            async fn lookup(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        [
            "service_schema: operations `get_version` and `lookup` are both the tool `version`, \
             and this service asks for `mcp`\n       \
             a server lists each tool name once; write `mcp(name = \"...\")` on one of them"
        ]
    );
}

#[test]
fn a_header_named_value_beside_a_success_an_error_or_a_message_is_refused() {
    let out = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Answers the version.
            #[service_schema_op(http(method = "POST", path = "/v", header_out("value")))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<(VersionResponse, String), VersionError>;
        }"#,
    );
    assert_eq!(
        out,
        [
            "service_schema: operation `get_version` binds the name \"value\" on `header_out`, and \
             this service asks for `mcp`\n       \
             the success is written under \"value\" there; bind the header under another name"
        ]
    );
    let error = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Answers the version.
            #[service_schema_op(http(method = "POST", path = "/v", error_header_out("value")))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, (VersionError, String)>;
        }"#,
    );
    assert_eq!(error.len(), 1);
    assert!(
        error[0].contains("the error is written under \"value\""),
        "{error:?}"
    );
    let inbound = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Answers the version.
            #[service_schema_op(http(method = "POST", path = "/v", header_in("value" = known_tag)))]
            async fn get_version(&self, ctx: &Ctx, document_id: String, known_tag: String)
                -> Result<VersionResponse, VersionError>;
        }"#,
    );
    assert_eq!(inbound.len(), 1);
    assert!(
        inbound[0].contains("the message is written under \"value\""),
        "{inbound:?}"
    );
}

#[test]
fn a_header_in_named_like_a_key_of_the_generated_message_is_refused() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Renames one document.
            #[service_schema_op(http(method = "POST", path = "/r", header_in("documentId" = tag)))]
            async fn rename_document(&self, ctx: &Ctx, document_id: String, title: String, tag: String)
                -> Result<(), VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `rename_document` binds the `header_in` name \
             \"documentId\", and this service asks for `mcp`\n       \
             the tool's input already holds the key \"documentId\" for the argument \
             `document_id`; bind the header under another name"
        ]
    );
}

#[test]
fn a_streamed_body_and_a_file_part_are_refused_beside_mcp_as_beside_ws_rpc() {
    let streamed = service(
        r#"pub trait ContentService<Ctx> {
            /// Answers the content.
            #[service_schema_op(http(method = "GET", path = "/c/{document_id}", body = "stream"))]
            async fn get_content(&self, ctx: &Ctx, document_id: String)
                -> Result<content_service_schema::StreamedAnswer, ContentError>;
        }"#,
    );
    let refused = stream_envelope_refusal(&streamed, &[Transport::Mcp]).unwrap();
    assert!(
        refused.to_string().contains(
            "operation `get_content` declares a streamed body, and this service also declares `mcp`"
        ),
        "{refused}"
    );
    let expanded = expansion(
        quote! { transports = ["mcp"] },
        r#"pub trait ContentService<Ctx> {
            /// Answers the content.
            #[service_schema_op(http(method = "POST", path = "/u", body = "multipart", part("file" = attachment)))]
            async fn upload_document(&self, ctx: &Ctx, title: String, category: String, attachment: Box<dyn upload_service_schema::BodySource + Send>)
                -> Result<(), ContentError>;
        }"#,
    );
    assert!(
        expanded.contains("declares a multipart file part"),
        "{expanded}"
    );
}

#[test]
fn a_service_that_asks_for_mcp_expands_its_tool_list_and_one_that_does_not_has_none() {
    let source = "
        /// Reads documents.
        pub trait DocumentService<Ctx> {
            /// Answers the version.
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;

            /// Renames one document.
            async fn rename_document(&self, ctx: &Ctx, document_id: String, title: String)
                -> Result<(), VersionError>;
        }";
    for transports in [
        quote! { transports = ["mcp"] },
        quote! { transports = ["mcp", "ws_rpc"] },
    ] {
        let asked = expansion(transports, source);
        assert!(!asked.contains("compile_error"), "{asked}");
        assert!(asked.contains("pub fn mcp_tools"), "{asked}");
        assert!(
            asked.contains("pub const fn wrapped_get_version"),
            "{asked}"
        );
        assert!(asked.contains("pub const MCP_NAMES"), "{asked}");
    }
    let plain = expansion(quote! { transports = ["ws_rpc"] }, source);
    assert!(!plain.contains("mcp_tools"), "{plain}");
    assert!(!plain.contains("MCP_NAMES"), "{plain}");
}

#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn the_typescript_accessors_are_published_only_on_a_service_that_asks_for_mcp() {
    let source = "
        /// Reads and renames documents.
        pub trait DocumentService<Ctx> {
            /// Answers the current version of one document.
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }";
    for transports in [
        quote! { transports = ["mcp"] },
        quote! { transports = ["mcp", "ws_rpc"] },
    ] {
        let asked = expansion(transports, source);
        assert!(asked.contains("pub fn ts_mcp_service"), "{asked}");
        assert!(asked.contains("pub fn ts_mcp_client"), "{asked}");
    }
    let plain = expansion(quote! { transports = ["ws_rpc"] }, source);
    assert!(!plain.contains("ts_mcp_service"), "{plain}");
    assert!(!plain.contains("ts_mcp_client"), "{plain}");
    assert!(plain.contains("ts_http_service"), "{plain}");
}

#[test]
fn a_message_lists_jsonschema_on_a_service_that_asks_for_mcp_as_it_does_for_openapi() {
    let source = "
        pub trait DocumentService<Ctx> {
            /// Renames one document.
            async fn rename_document(&self, ctx: &Ctx, document_id: String, title: String)
                -> Result<(), VersionError>;
        }";
    let listed = "enable = [\"jsonschema\"]";
    assert!(expansion(quote! { transports = ["mcp"] }, source).contains(listed));
    assert!(!expansion(quote! { transports = ["ws_rpc"] }, source).contains(listed));
}

#[test]
fn the_mcp_transport_is_known_by_name() {
    assert_eq!(Transport::Mcp.name(), "mcp");
    assert!(Transport::KNOWN.contains(&Transport::Mcp));
}

#[test]
fn the_resource_keys_are_read() {
    let read = service(RESOURCES);
    let guide = &read.operations[0].mcp;
    let resource = guide.resource.as_ref().unwrap();
    assert_eq!(resource.uri, "guide://configure");
    assert!(!resource.is_template());
    assert_eq!(guide.mime_type.as_deref(), Some("text/markdown"));
    assert_eq!(guide.ttl_ms, Some(5));
    assert_eq!(guide.cache_scope.as_deref(), Some("public"));
    assert!(read.operations[1].mcp.list_resources);
    let file = read.operations[2].mcp.resource.as_ref().unwrap();
    assert!(file.is_template());
    assert_eq!(file.placeholder_names(), ["document_id"]);
    let completes = read.operations[3].mcp.completes.as_ref().unwrap();
    assert_eq!(completes.operation, "document_file");
    assert_eq!(completes.parameter, "document_id");
    let tools: Vec<_> = read
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
        .map(OperationDef::mcp_tool_name)
        .collect();
    assert_eq!(tools, ["get-version"]);
    assert_eq!(mcp_refusals_of(RESOURCES), Vec::<String>::new());
}

#[test]
fn a_placeholder_that_names_no_argument_is_refused() {
    let reported = parse_refusals(
        r#"pub trait DocumentService<Ctx> {
            /// The file of one document.
            #[service_schema_op(mcp(resource = "workspace://documents/{missing}"))]
            async fn document_file(&self, ctx: &Ctx, document_id: String)
                -> Result<String, VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `document_file` writes the placeholder `{missing}` in \
             `mcp(resource = \"workspace://documents/{missing}\")`, and `missing` is no argument \
             of it\n       name an argument of the operation between the braces"
        ]
    );
}

#[test]
fn an_unterminated_placeholder_in_a_uri_is_refused() {
    let reported = parse_refusals(
        r#"pub trait DocumentService<Ctx> {
            /// The file of one document.
            #[service_schema_op(mcp(resource = "workspace://documents/{document_id"))]
            async fn document_file(&self, ctx: &Ctx, document_id: String)
                -> Result<String, VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        [
            "service_schema: `mcp(resource = ...)`'s URI opens `{` with no matching `}`\n       \
             write `{field}`, closed before the URI ends"
        ]
    );
}

#[test]
fn two_operations_with_one_uri_are_refused_whatever_the_placeholders_are_called() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// The file of one document.
            #[service_schema_op(mcp(resource = "workspace://documents/{document_id}"))]
            async fn document_file(&self, ctx: &Ctx, document_id: String)
                -> Result<String, VersionError>;

            /// The text of one document.
            #[service_schema_op(mcp(resource = "workspace://documents/{id}"))]
            async fn document_text(&self, ctx: &Ctx, id: String)
                -> Result<String, VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        ["service_schema: operation `document_text` is the resource \
             `workspace://documents/{id}`, which `document_file` already is, and this service \
             asks for `mcp`\n       \
             a server reads each URI through one operation; write another URI on one of them"]
    );
}

#[test]
fn a_success_a_resource_cannot_answer_is_refused() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// The guide.
            #[service_schema_op(mcp(resource = "guide://configure"))]
            async fn configuration_guide(&self, ctx: &Ctx) -> Result<VersionResponse, VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `configuration_guide` is the resource `guide://configure` \
             and answers a success a resource cannot, and this service asks for `mcp`\n       \
             answer a `String` for text, or a `(Vec<u8>, String)` of bytes and their content type"
        ]
    );
}

#[test]
fn a_completes_that_names_no_operation_or_no_placeholder_is_refused() {
    let source = |resource: &str, parameter: &str| {
        format!(
            r#"pub trait DocumentService<Ctx> {{
                /// The file of one document.
                #[service_schema_op(mcp(resource = "workspace://documents/{{document_id}}"))]
                async fn document_file(&self, ctx: &Ctx, document_id: String)
                    -> Result<String, VersionError>;

                /// Suggests documents.
                #[service_schema_op(mcp(completes(resource = "{resource}", parameter = "{parameter}")))]
                async fn suggest_documents(&self, ctx: &Ctx, typed: String, chosen: HashMap<String, String>)
                    -> Result<Vec<String>, VersionError>;
            }}"#
        )
    };
    assert_eq!(
        mcp_refusals_of(&source("nothing", "document_id")),
        [
            "service_schema: operation `suggest_documents` completes `nothing`, which is no \
             resource template of the service, and this service asks for `mcp`\n       \
             name an operation of this service that writes `mcp(resource = \"...{placeholder}...\")`"
        ]
    );
    assert_eq!(
        mcp_refusals_of(&source("document_file", "page")),
        [
            "service_schema: operation `suggest_documents` completes `document_file` for `page`, \
             and the URI `workspace://documents/{document_id}` holds no such placeholder, and \
             this service asks for `mcp`\n       name a placeholder of the template"
        ]
    );
}

#[test]
fn a_listing_and_a_completion_are_held_to_their_shapes() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// The file of one document.
            #[service_schema_op(mcp(resource = "workspace://documents/{document_id}"))]
            async fn document_file(&self, ctx: &Ctx, document_id: String)
                -> Result<String, VersionError>;

            /// Lists the documents.
            #[service_schema_op(mcp(list_resources))]
            async fn list_documents(&self, ctx: &Ctx, cursor: String, limit: u32)
                -> Result<Vec<String>, VersionError>;

            /// Suggests documents.
            #[service_schema_op(mcp(completes(resource = "document_file", parameter = "document_id")))]
            async fn suggest_documents(&self, ctx: &Ctx, typed: String)
                -> Result<String, VersionError>;
        }"#,
    );
    assert_eq!(reported.len(), 4, "{reported:?}");
    assert!(reported[0].contains("lists resources and answers a success a listing cannot"));
    assert!(reported[1].contains("lists resources and takes more than a cursor"));
    assert!(reported[2].contains("answers a success a completion cannot"));
    assert!(reported[3].contains("does not take two arguments"));
}

#[test]
fn an_operation_that_is_two_things_is_refused() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// The guide.
            #[service_schema_op(mcp(resource = "guide://configure", list_resources))]
            async fn configuration_guide(&self, ctx: &Ctx) -> Result<String, VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `configuration_guide` is more than one of `resource`, \
             `list_resources`, `prompt`, `completes` and `skill` in `mcp(...)`, and this service \
             asks for `mcp`\n       an operation is one of them; split it into one operation each"
        ]
    );
}

#[test]
fn a_service_lists_its_resources_through_one_operation() {
    let reported = mcp_refusals_of(
        "pub trait DocumentService<Ctx> {
            /// Lists the documents.
            #[service_schema_op(mcp(list_resources))]
            async fn list_documents(&self, ctx: &Ctx) -> Result<McpResourcePage, VersionError>;

            /// Lists the drafts.
            #[service_schema_op(mcp(list_resources))]
            async fn list_drafts(&self, ctx: &Ctx) -> Result<McpResourcePage, VersionError>;
        }",
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `list_drafts` lists resources, which `list_documents` \
             already does, and this service asks for `mcp`\n       \
             a service lists its resources through one operation"
        ]
    );
}

#[test]
fn a_cache_scope_other_than_public_or_private_is_refused() {
    let reported = parse_refusals(
        r#"pub trait DocumentService<Ctx> {
            #[service_schema_op(mcp(cache_scope = "shared"))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }"#,
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `get_version` writes `cache_scope = \"shared\"` in \
             `mcp(...)`\n       the scope is \"public\" or \"private\""
        ]
    );
}
fn scoped(scopes: &str) -> String {
    format!(
        "pub trait DocumentService<Ctx> {{
            /// Answers the current version of one document.
            #[service_schema_op(mcp(scopes = {scopes}))]
            async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                -> Result<VersionResponse, VersionError>;
        }}"
    )
}

#[test]
fn scopes_are_read_in_order() {
    let read = service(&scoped(r#"["documents:read", "documents:write"]"#));
    let names: Vec<&str> = read.operations[0]
        .mcp
        .scopes
        .iter()
        .map(|(scope, _)| scope.as_str())
        .collect();
    assert_eq!(names, ["documents:read", "documents:write"]);
    assert_eq!(
        mcp_refusals_of(&scoped(r#"["documents:read"]"#)),
        Vec::<String>::new()
    );
}

#[test]
fn a_scope_that_is_empty_spaced_quoted_escaped_controlled_or_repeated_is_refused() {
    for (scope, problem) in [
        (r#""""#, "an empty scope"),
        (
            r#""documents read""#,
            "the scope \"documents read\", which holds a space, a quote, a backslash or a control character",
        ),
        (
            r#""a\"b""#,
            "the scope \"a\\\"b\", which holds a space, a quote, a backslash or a control character",
        ),
        (
            r#""a\\b""#,
            "the scope \"a\\\\b\", which holds a space, a quote, a backslash or a control character",
        ),
        (
            r#""a\tb""#,
            "the scope \"a\\tb\", which holds a space, a quote, a backslash or a control character",
        ),
    ] {
        let reported = mcp_refusals_of(&scoped(&format!("[{scope}]")));
        assert_eq!(reported.len(), 1, "{scope}: {reported:?}");
        assert!(
            reported[0].starts_with(&format!(
                "service_schema: operation `get_version` writes {problem} in `mcp(scopes = [...])`"
            )),
            "{scope}: {reported:?}"
        );
    }
    let repeated = mcp_refusals_of(&scoped(r#"["documents:read", "documents:read"]"#));
    assert_eq!(repeated.len(), 1, "{repeated:?}");
    assert!(
        repeated[0].contains("the scope \"documents:read\" twice"),
        "{repeated:?}"
    );
}

#[test]
fn scopes_written_as_anything_but_a_list_of_text_are_refused() {
    for written in ["scopes", "scopes = \"a\"", "scopes = [1]"] {
        let reported = parse_refusals(&format!(
            "pub trait DocumentService<Ctx> {{
                #[service_schema_op(mcp({written}))]
                async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest)
                    -> Result<VersionResponse, VersionError>;
            }}"
        ));
        assert!(!reported.is_empty(), "{written}");
    }
}

#[test]
fn an_interview_and_a_content_flag_are_read() {
    let read = service(INTERVIEWS);
    let form = read.operations[0].mcp.interview.as_ref().unwrap();
    assert_eq!(form.arm.to_string(), "NeedsConfirmation");
    assert_eq!(form.answers.as_ref().unwrap().to_string(), "confirmation");
    let link = read.operations[1].mcp.interview.as_ref().unwrap();
    assert_eq!(link.arm.to_string(), "NeedsDriveConnection");
    assert!(link.answers.is_none());
    assert!(read.operations[2].mcp.content.is_some());
    assert!(read.operations.iter().all(OperationDef::is_mcp_tool));
    assert_eq!(mcp_refusals_of(INTERVIEWS), Vec::<String>::new());
}

#[test]
fn an_interview_needs_its_arm_and_exactly_one_of_answers_and_url() {
    for written in [
        r#"interview(arm = "NeedsConfirmation", answers = confirmation, url)"#,
        r#"interview(arm = "NeedsConfirmation")"#,
        "interview(answers = confirmation)",
        "interview(url)",
    ] {
        let reported = parse_refusals(&format!(
            "pub trait DocumentService<Ctx> {{
                /// Deletes one document.
                #[service_schema_op(mcp({written}))]
                async fn delete_document(&self, ctx: &Ctx, document_id: String, confirmation: Option<DeleteConfirmation>)
                    -> Result<(), DeleteError>;
            }}"
        ));
        assert_eq!(reported.len(), 1, "{written}: {reported:?}");
        assert!(
            reported[0].contains("without its arm and exactly one of `answers` and `url`"),
            "{written}: {reported:?}"
        );
    }
}

#[test]
fn an_interview_is_held_to_the_operation_it_is_written_on() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Names no argument.
            #[service_schema_op(mcp(interview(arm = "Asks", answers = missing)))]
            async fn first(&self, ctx: &Ctx, document_id: String, other: String) -> Result<(), E>;

            /// Takes a bare form.
            #[service_schema_op(mcp(interview(arm = "Asks", answers = form)))]
            async fn second(&self, ctx: &Ctx, document_id: String, form: Form) -> Result<(), E>;

            /// Takes the message alone.
            #[service_schema_op(mcp(interview(arm = "Asks", answers = form)))]
            async fn third(&self, ctx: &Ctx, form: Option<Form>) -> Result<(), E>;

            /// Is one way.
            #[service_schema_op(one_way, mcp(interview(arm = "Asks", url)))]
            async fn fourth(&self, ctx: &Ctx, document_id: String);

            /// Binds a header beside the arm.
            #[service_schema_op(http(method = "POST", path = "/fifth", error_header_out("retry-after")), mcp(interview(arm = "Asks", url)))]
            async fn fifth(&self, ctx: &Ctx, document_id: String) -> Result<(), (E, String)>;
        }"#,
    );
    assert_eq!(reported.len(), 5, "{reported:?}");
    assert!(reported[0].contains("`first` writes `interview(answers = missing)`"));
    assert!(reported[1].contains("`second` asks with `form` and its type `Form` is no `Option`"));
    assert!(reported[2].contains("`third` writes `interview(answers = form)`"));
    assert!(reported[3].contains("`fourth` is one way"));
    assert!(
        reported[4]
            .contains("`fifth` writes `interview(...)` in `mcp(...)` and binds `error_header_out`")
    );
}

#[test]
fn a_content_tool_answers_blocks_and_nothing_beside_them() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Answers a string.
            #[service_schema_op(mcp(content))]
            async fn first(&self, ctx: &Ctx, document_id: String) -> Result<String, E>;

            /// Is one way.
            #[service_schema_op(one_way, mcp(content))]
            async fn second(&self, ctx: &Ctx, document_id: String);

            /// Is a resource.
            #[service_schema_op(mcp(content, resource = "guide://configure"))]
            async fn third(&self, ctx: &Ctx) -> Result<String, E>;

            /// Answers blocks beside a header.
            #[service_schema_op(http(method = "POST", path = "/fourth", header_out("etag")), mcp(content))]
            async fn fourth(&self, ctx: &Ctx, document_id: String)
                -> Result<(Vec<u8>, String), E>;
        }"#,
    );
    assert!(
        reported[0].contains(
            "`first` writes `content` in `mcp(...)` and answers a success it cannot write as blocks"
        ),
        "{reported:?}"
    );
    assert!(
        reported
            .iter()
            .any(|one| one.contains("`second` is one way and writes `content`"))
    );
    assert!(
        reported
            .iter()
            .any(|one| one.contains("`third` is a resource, a listing or a completion"))
    );
    assert!(
        reported
            .iter()
            .any(|one| one
                .contains("`fourth` writes `content` in `mcp(...)` and binds `header_out`"))
    );
}

#[test]
fn a_prompt_is_read_with_its_arguments_and_is_no_tool() {
    let read = service(PROMPTS);
    let review = &read.operations[1];
    assert!(review.is_mcp_prompt());
    assert!(!review.is_mcp_tool());
    assert_eq!(review.mcp_tool_name(), "review");
    let arguments: Vec<String> = review
        .mcp
        .prompt
        .as_ref()
        .unwrap()
        .arguments
        .iter()
        .map(|(argument, _)| argument.to_string())
        .collect();
    assert_eq!(arguments, ["document_type", "focus"]);
    let completes = read.operations[2].mcp.completes.as_ref().unwrap();
    assert_eq!(completes.operation, "review_document_type");
    assert_eq!(completes.parameter, "document_type");
    let tools: Vec<_> = read
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
        .map(OperationDef::mcp_tool_name)
        .collect();
    assert_eq!(tools, ["get-version"]);
    assert_eq!(mcp_refusals_of(PROMPTS), Vec::<String>::new());
}

#[test]
fn a_prompt_argument_that_is_not_text_is_refused() {
    let reported = mcp_refusals_of(
        "pub trait DocumentService<Ctx> {
            /// Reviews.
            #[service_schema_op(mcp(prompt))]
            async fn review(&self, ctx: &Ctx, page: u32) -> Result<String, VersionError>;
        }",
    );
    assert_eq!(
        reported,
        [
            "service_schema: operation `review` is a prompt and takes `page: u32`, and this \
             service asks for `mcp`\n       \
             MCP sends a prompt's arguments as strings only; take a `String`, or an \
             `Option<String>` for one that may be left out"
        ]
    );
}

#[test]
fn a_prompt_is_held_to_its_success_its_outcome_and_its_keys() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Answers a number.
            #[service_schema_op(mcp(prompt))]
            async fn number(&self, ctx: &Ctx) -> Result<u32, VersionError>;

            /// Fires and forgets.
            #[service_schema_op(one_way, mcp(prompt))]
            async fn forget(&self, ctx: &Ctx);

            /// Binds a header.
            #[service_schema_op(http(method = "POST", path = "/tagged", header_in("x-tag" = tag)), mcp(prompt))]
            async fn tagged(&self, ctx: &Ctx, tag: String) -> Result<String, VersionError>;

            /// Reads only.
            #[service_schema_op(mcp(prompt, read_only, ttl_ms = 5, content))]
            async fn lonely(&self, ctx: &Ctx) -> Result<String, VersionError>;
        }"#,
    );
    assert!(
        reported.iter().any(|text| text.contains(
            "`number` is a prompt and answers a success a prompt cannot write as messages"
        )),
        "{reported:#?}"
    );
    assert!(
        reported
            .iter()
            .any(|text| text.contains("`forget` is one way and writes `prompt`")),
        "{reported:#?}"
    );
    assert!(
        reported
            .iter()
            .any(|text| text.contains("`tagged` is a prompt and binds a header")),
        "{reported:#?}"
    );
    for key in ["read_only", "ttl_ms", "content"] {
        assert!(
            reported.iter().any(|text| text.contains(&format!(
                "`lonely` is a prompt and writes `{key}` in `mcp(...)`"
            ))),
            "{key}: {reported:#?}"
        );
    }
}

#[test]
fn a_prompt_name_is_held_to_the_rules_of_a_tool_name_and_is_listed_once() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Reviews.
            #[service_schema_op(mcp(prompt, name = "bad name"))]
            async fn review(&self, ctx: &Ctx) -> Result<String, VersionError>;

            /// Reviews once.
            #[service_schema_op(mcp(prompt, name = "again"))]
            async fn review_once(&self, ctx: &Ctx) -> Result<String, VersionError>;

            /// Reviews twice.
            #[service_schema_op(mcp(prompt, name = "again"))]
            async fn review_twice(&self, ctx: &Ctx) -> Result<String, VersionError>;
        }"#,
    );
    assert_eq!(reported.len(), 2, "{reported:#?}");
    assert!(reported[0].contains("`review` is the prompt `bad name`"));
    assert!(
        reported[1]
            .contains("operations `review_once` and `review_twice` are both the prompt `again`")
    );
}

#[test]
fn a_prompt_and_a_tool_of_one_name_are_both_listed() {
    let reported = mcp_refusals_of(
        r#"pub trait DocumentService<Ctx> {
            /// Reviews.
            #[service_schema_op(mcp(prompt, name = "review"))]
            async fn review_prompt(&self, ctx: &Ctx) -> Result<String, VersionError>;

            /// Reviews.
            #[service_schema_op(mcp(name = "review"))]
            async fn review_tool(&self, ctx: &Ctx) -> Result<(), VersionError>;
        }"#,
    );
    assert_eq!(reported, Vec::<String>::new());
}

#[test]
fn a_prompt_completion_that_names_no_prompt_or_no_argument_is_refused() {
    let source = |prompt: &str, parameter: &str| {
        format!(
            r#"pub trait DocumentService<Ctx> {{
                /// Reviews.
                #[service_schema_op(mcp(prompt))]
                async fn review(&self, ctx: &Ctx, document_type: String)
                    -> Result<String, VersionError>;

                /// Suggests types.
                #[service_schema_op(mcp(completes(prompt = "{prompt}", parameter = "{parameter}")))]
                async fn suggest(&self, ctx: &Ctx, typed: String, chosen: HashMap<String, String>)
                    -> Result<Vec<String>, VersionError>;
            }}"#
        )
    };
    assert_eq!(
        mcp_refusals_of(&source("nothing", "document_type")),
        [
            "service_schema: operation `suggest` completes `nothing`, which is no prompt of the \
             service, and this service asks for `mcp`\n       \
             name an operation of this service that writes `mcp(prompt)`"
        ]
    );
    assert_eq!(
        mcp_refusals_of(&source("review", "page")),
        [
            "service_schema: operation `suggest` completes `review` for `page`, and the prompt \
             takes no such argument, and this service asks for `mcp`\n       \
             name an argument of the prompt, as the operation spells it"
        ]
    );
}
