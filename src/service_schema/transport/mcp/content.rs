//! Result content in the dispatcher: a tool that answers blocks, not a structured value.

use crate::service_schema::parse::{OperationDef, ServiceDef};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::service_schema::mcp_tools::is_prompt_messages;
pub(super) use crate::service_schema::mcp_tools::{
    is_content_list as is_list, is_content_pair as is_pair,
};

pub(super) fn any(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| operation.mcp.content.is_some())
}

/// Whether a prompt answers a list of messages, which hold blocks.
pub(super) fn prompt_messages(service: &ServiceDef) -> bool {
    service.operations.iter().any(is_prompt_messages)
}

/// The result of blocks, and what writes one block.
pub(super) fn helpers(service: &ServiceDef, module: &Ident) -> TokenStream {
    let pair = service.operations.iter().any(is_pair);
    let list = service.operations.iter().any(is_list) || prompt_messages(service);
    let media = pair.then(|| {
        quote! {
            /// An image or audio block for `bytes`, where `content_type` says they are one.
            fn media_block(bytes: &[u8], content_type: &str) -> ::core::option::Option<::serde_json::Value> {
                let kind = if content_type.starts_with("image/") {
                    "image"
                } else if content_type.starts_with("audio/") {
                    "audio"
                } else {
                    return ::core::option::Option::None;
                };
                ::core::option::Option::Some(::serde_json::json!({
                    "type": kind,
                    "data": base64_encoded(bytes),
                    "mimeType": content_type,
                }))
            }
        }
    });
    let blocks = list.then(|| {
        quote! {
            /// One block of `content`, as the result writes it.
            fn content_block(block: &$crate::#module::McpContent) -> ::serde_json::Value {
                match block {
                    $crate::#module::McpContent::Text { text } => {
                        ::serde_json::json!({ "type": "text", "text": text })
                    }
                    $crate::#module::McpContent::Image { data, mime_type } => {
                        ::serde_json::json!({ "type": "image", "data": base64_encoded(data), "mimeType": mime_type })
                    }
                    $crate::#module::McpContent::Audio { data, mime_type } => {
                        ::serde_json::json!({ "type": "audio", "data": base64_encoded(data), "mimeType": mime_type })
                    }
                    $crate::#module::McpContent::ResourceLink { uri, name, title, description, mime_type } => {
                        let mut written = ::serde_json::json!({ "type": "resource_link", "uri": uri, "name": name });
                        if let ::core::option::Option::Some(title) = title {
                            written["title"] = ::serde_json::Value::String(title.clone());
                        }
                        if let ::core::option::Option::Some(description) = description {
                            written["description"] = ::serde_json::Value::String(description.clone());
                        }
                        if let ::core::option::Option::Some(mime_type) = mime_type {
                            written["mimeType"] = ::serde_json::Value::String(mime_type.clone());
                        }
                        written
                    }
                    $crate::#module::McpContent::Resource { uri, mime_type, contents } => {
                        let mut resource = ::serde_json::json!({ "uri": uri });
                        if let ::core::option::Option::Some(mime_type) = mime_type {
                            resource["mimeType"] = ::serde_json::Value::String(mime_type.clone());
                        }
                        match contents {
                            $crate::#module::McpResourceContents::Text(text) => {
                                resource["text"] = ::serde_json::Value::String(text.clone());
                            }
                            $crate::#module::McpResourceContents::Blob(bytes) => {
                                resource["blob"] = ::serde_json::Value::String(base64_encoded(bytes));
                            }
                        }
                        ::serde_json::json!({ "type": "resource", "resource": resource })
                    }
                }
            }
        }
    });
    let result = any(service).then(|| {
        quote! {
            /// The result of a tool that answers blocks.
            fn blocks_result(content: ::std::vec::Vec<::serde_json::Value>) -> ::serde_json::Value {
                ::serde_json::json!({ "resultType": "complete", "content": content, "isError": false })
            }
        }
    });
    quote! {
        #result
        #media
        #blocks
    }
}

/// The arm for a success that is blocks. A pair that is no image and no audio is answered by
/// `ordinary`, as an operation with no flag answers it.
pub(super) fn success_arm(operation: &OperationDef, ordinary: &TokenStream) -> TokenStream {
    if is_pair(operation) {
        return quote! {
            match media_block(&carried_value_0.0, &carried_value_0.1) {
                ::core::option::Option::Some(block) => ::core::option::Option::Some(
                    ::core::result::Result::Ok(blocks_result(::std::vec![block])),
                ),
                ::core::option::Option::None => #ordinary,
            }
        };
    }
    quote! {
        ::core::option::Option::Some(::core::result::Result::Ok(blocks_result(
            carried_value_0.iter().map(content_block).collect(),
        )))
    }
}
