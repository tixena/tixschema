//! Skills in the Rust dispatcher: what the catalog lists, `skills/get`, and the reads of a skill's
//! files, from the folder the service embeds and from the operations that write a skill.

use super::resources::reading;
use crate::service_schema::parse::{OperationDef, OperationOutcome, ServiceDef};
use crate::service_schema::skills::written_frontmatter;
use crate::service_schema::support::{message_reader_ident, message_validator_ident};
use crate::service_schema::transport::amqp_rpc::call_arguments;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

const MANIFEST: &str = "SKILL.md";

/// The operations that write a skill at run time.
pub(super) fn dynamic(service: &ServiceDef) -> Vec<&OperationDef> {
    service
        .operations
        .iter()
        .filter(|operation| operation.mcp.skill.is_some())
        .collect()
}

/// Whether the service ships a folder or has an operation that writes a skill.
pub(super) fn any(service: &ServiceDef) -> bool {
    service.skills.is_some()
        || service
            .operations
            .iter()
            .any(|operation| operation.mcp.skill.is_some())
}

/// Whether a skill an operation writes answers bytes.
pub(super) fn reads_bytes(service: &ServiceDef) -> bool {
    dynamic(service).into_iter().any(is_blob)
}

fn is_blob(operation: &OperationDef) -> bool {
    matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error }
            if matches!(success.as_ref(), syn::Type::Tuple(_))
    )
}

/// The `route` arm of `skills/get`.
pub(super) fn get_arm(service: &ServiceDef, schema: &Ident) -> Option<TokenStream> {
    any(service).then(|| {
        quote! {
            "skills/get" => {
                let name = params.get("name").and_then(::serde_json::Value::as_str);
                let uri = params.get("uri").and_then(::serde_json::Value::as_str);
                let entry = $crate::#schema::mcp_skills().into_iter().find(|entry| {
                    name.is_some_and(|name| entry["frontmatter"]["name"] == name)
                        || uri.is_some_and(|uri| entry["uri"] == uri)
                })?;
                let dynamic = entry["resources"] == "dynamic";
                ::core::option::Option::Some(::core::result::Result::Ok(::serde_json::json!({
                    "resultType": "complete",
                    "skill": entry,
                    "ttlMs": if dynamic { 0_u64 } else { LISTED_FOR_MS },
                    "cacheScope": if dynamic { "private" } else { "public" },
                })))
            }
        }
    })
}

/// The blocks of the `resources/read` arm that answer a `skill://` URI.
pub(super) fn read_blocks(
    service: &ServiceDef,
    schema: &Ident,
    module: &Ident,
) -> Vec<TokenStream> {
    let mut blocks = Vec::new();
    if service.skills.is_some() {
        blocks.push(quote! {
            if let ::core::option::Option::Some(content) = $crate::#schema::mcp_skill_file(uri) {
                return ::core::option::Option::Some(::core::result::Result::Ok(::serde_json::json!({
                    "resultType": "complete",
                    "contents": [content],
                    "ttlMs": LISTED_FOR_MS,
                    "cacheScope": "public",
                })));
            }
        });
    }
    blocks.extend(
        dynamic(service)
            .into_iter()
            .filter_map(|operation| read_block(operation, module)),
    );
    blocks
}

fn read_block(operation: &OperationDef, module: &Ident) -> Option<TokenStream> {
    let (name, _) = operation.mcp.skill.as_ref()?;
    let root = format!("skill://{name}/");
    let frontmatter = written_frontmatter(name, operation.doc.as_deref().unwrap_or_default());
    let wire = &operation.wire_name;
    let method = &operation.ident;
    let reader = message_reader_ident(operation);
    let validator = message_validator_ident(operation);
    let call = call_arguments(operation);
    let source = quote! { ::serde_json::Value::String(path.to_owned()) };
    let reading = reading(operation, module, &source, &reader, &validator, None);
    let (pattern, content) = if is_blob(operation) {
        (
            quote! { (bytes, content_type) },
            quote! { { "uri": uri, "mimeType": content_type, "blob": base64_encoded(&bytes) } },
        )
    } else {
        (
            quote! { text },
            quote! { { "uri": uri, "mimeType": "text/markdown", "text": written } },
        )
    };
    let written = if is_blob(operation) {
        TokenStream::new()
    } else {
        quote! {
            let written = if path == #MANIFEST {
                ::std::format!("{}{}", #frontmatter, text)
            } else {
                text
            };
        }
    };
    Some(quote! {
        if let ::core::option::Option::Some(path) = uri.strip_prefix(#root).filter(|path| !path.is_empty()) {
            #reading
            return ::core::option::Option::Some(
                match caught(move || svc.#method(ctx #(, #call)*)).await {
                    ::core::result::Result::Err(panicked) => {
                        record_panic(#wire, &panicked);
                        ::core::result::Result::Err(internal_error(#wire, &panicked))
                    }
                    ::core::result::Result::Ok(::core::result::Result::Err(error)) => {
                        match encoded(#wire, &error) {
                            ::core::result::Result::Ok(data) => ::core::result::Result::Err(invalid_params(
                                "Skill file not found",
                                ::core::option::Option::Some(data),
                            )),
                            ::core::result::Result::Err(refused) => ::core::result::Result::Err(refused),
                        }
                    }
                    ::core::result::Result::Ok(::core::result::Result::Ok(#pattern)) => {
                        #written
                        ::core::result::Result::Ok(::serde_json::json!({
                            "resultType": "complete",
                            "contents": [::serde_json::json!(#content)],
                            "ttlMs": 0_u64,
                            "cacheScope": "private",
                        }))
                    }
                },
            );
        }
    })
}
