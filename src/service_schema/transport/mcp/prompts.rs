//! Prompts in the Rust dispatcher: what the catalog lists, the arms of `route` that get a prompt
//! and complete one of its arguments.

use crate::service_schema::parse::{
    McpCompletesKind, OperationDef, OperationInputs, OperationOutcome, ServiceDef, option_inner,
    wire_key, written,
};
use crate::service_schema::support::{message_reader_ident, message_validator_ident};
use crate::service_schema::transport::amqp_rpc::call_arguments;
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Ident;

const COMPLETION_LIMIT: usize = 100;

pub(super) fn any(service: &ServiceDef) -> bool {
    service.operations.iter().any(OperationDef::is_mcp_prompt)
}

fn prompts(service: &ServiceDef) -> impl Iterator<Item = &OperationDef> {
    service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_prompt())
}

/// Each prompt as the members of its entry in `prompts/list`.
pub fn prompt_entries(service: &ServiceDef) -> Vec<TokenStream> {
    prompts(service).map(entry).collect()
}

fn entry(operation: &OperationDef) -> TokenStream {
    let name = operation.mcp_tool_name();
    let title = operation.mcp_tool_title();
    let description = operation.doc.clone().unwrap_or_default();
    let mut members = quote! { "name": #name, "title": #title, "description": #description };
    let arguments: Vec<TokenStream> = operation
        .mcp
        .prompt
        .iter()
        .flat_map(|prompt| prompt.arguments.iter())
        .map(|(argument, ty)| {
            let key = wire_key(argument);
            let required = option_inner(ty).is_none();
            quote! { { "name": #key, "required": #required } }
        })
        .collect();
    if !arguments.is_empty() {
        members.extend(quote! { , "arguments": [#(#arguments),*] });
    }
    if let Some(source) = &operation.mcp.icon {
        members.extend(quote! { , "icons": [{ "src": #source }] });
    }
    quote! { { #members } }
}

/// The member of `catalog()` that holds the prompts.
pub(super) fn catalog_prompts(service: &ServiceDef) -> TokenStream {
    let entries = prompt_entries(service);
    quote! { "prompts": [#(#entries),*], }
}

/// The capability `server/discover` adds for the prompts.
pub(super) fn capability(service: &ServiceDef) -> TokenStream {
    if any(service) {
        quote! { , "prompts": {} }
    } else {
        TokenStream::new()
    }
}

/// The `route` arm of `prompts/get`.
pub(super) fn get_arm(service: &ServiceDef, module: &Ident) -> Option<TokenStream> {
    if !any(service) {
        return None;
    }
    let blocks = prompts(service).map(|operation| get_block(operation, module));
    Some(quote! {
        "prompts/get" => {
            let name = params.get("name")?.as_str()?;
            #(#blocks)*
            ::core::option::Option::None
        }
    })
}

/// The `route` arm of `completion/complete` for the arguments of prompts, when no resource
/// completion shares the method.
pub(super) fn complete_blocks(service: &ServiceDef, module: &Ident) -> Vec<TokenStream> {
    service
        .operations
        .iter()
        .filter_map(|operation| complete_block(operation, service, module))
        .collect()
}

fn message_source(operation: &OperationDef) -> TokenStream {
    let arguments = quote! {
        match params.get("arguments") {
            ::core::option::Option::Some(::serde_json::Value::Object(held)) => held.clone(),
            _ => ::serde_json::Map::new(),
        }
    };
    match &operation.inputs {
        OperationInputs::Empty => quote! { ::serde_json::json!({}) },
        OperationInputs::Generated(_) => quote! { ::serde_json::Value::Object(#arguments) },
        OperationInputs::Named(_) => {
            let key = operation
                .mcp
                .prompt
                .as_ref()
                .and_then(|prompt| prompt.arguments.first())
                .map(|(argument, _)| wire_key(argument))
                .unwrap_or_default();
            quote! {
                {
                    let mut held = #arguments;
                    held.remove(#key).unwrap_or(::serde_json::Value::Null)
                }
            }
        }
    }
}

fn get_block(operation: &OperationDef, module: &Ident) -> TokenStream {
    let prompt = operation.mcp_tool_name();
    let wire = &operation.wire_name;
    let method = &operation.ident;
    let reader = message_reader_ident(operation);
    let validator = message_validator_ident(operation);
    let call = call_arguments(operation);
    let source = message_source(operation);
    let reading = super::resources::reading(operation, module, &source, &reader, &validator);
    let description = operation.doc.clone().unwrap_or_default();
    let OperationOutcome::Reply {
        success,
        error: _error,
    } = &operation.outcome
    else {
        return TokenStream::new();
    };
    let text = quote::ToTokens::to_token_stream(success.as_ref())
        .to_string()
        .replace(' ', "");
    let messages = if text == "String" {
        quote! {
            ::std::vec![::serde_json::json!({
                "role": "user",
                "content": { "type": "text", "text": answered },
            })]
        }
    } else {
        quote! {
            answered
                .iter()
                .map(|message| ::serde_json::json!({
                    "role": match message.role {
                        $crate::#module::McpRole::User => "user",
                        $crate::#module::McpRole::Assistant => "assistant",
                    },
                    "content": content_block(&message.content),
                }))
                .collect::<::std::vec::Vec<_>>()
        }
    };
    quote! {
        if name == #prompt {
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
                                &format!("{}: the service refused the prompt", #prompt),
                                ::core::option::Option::Some(data),
                            )),
                            ::core::result::Result::Err(refused) => ::core::result::Result::Err(refused),
                        }
                    }
                    ::core::result::Result::Ok(::core::result::Result::Ok(answered)) => {
                        ::core::result::Result::Ok(::serde_json::json!({
                            "resultType": "complete",
                            "description": #description,
                            "messages": #messages,
                        }))
                    }
                },
            );
        }
    }
}

fn complete_block(
    operation: &OperationDef,
    service: &ServiceDef,
    module: &Ident,
) -> Option<TokenStream> {
    let completes = operation.mcp.completes.as_ref()?;
    if completes.kind != McpCompletesKind::Prompt {
        return None;
    }
    let target = service
        .operations
        .iter()
        .find(|candidate| written(&candidate.ident) == completes.operation)?;
    let argument = target
        .mcp
        .prompt
        .as_ref()?
        .arguments
        .iter()
        .find(|(argument, _)| written(argument) == completes.parameter)
        .map(|(argument, _)| wire_key(argument))?;
    let prompt = target.mcp_tool_name();
    let wire = &operation.wire_name;
    let method = &operation.ident;
    let reader = message_reader_ident(operation);
    let validator = message_validator_ident(operation);
    let call = call_arguments(operation);
    let OperationInputs::Generated(fields) = &operation.inputs else {
        return None;
    };
    let typed_key = wire_key(&fields.first()?.0);
    let chosen_key = wire_key(&fields.get(1)?.0);
    let source = quote! { ::serde_json::json!({ #typed_key: typed, #chosen_key: chosen }) };
    let reading = super::resources::reading(operation, module, &source, &reader, &validator);
    let limit = Literal::usize_unsuffixed(COMPLETION_LIMIT);
    Some(quote! {
        if prompt == #prompt && parameter == #argument {
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
                                "Completion refused",
                                ::core::option::Option::Some(data),
                            )),
                            ::core::result::Result::Err(refused) => ::core::result::Result::Err(refused),
                        }
                    }
                    ::core::result::Result::Ok(::core::result::Result::Ok(suggested)) => {
                        let total = suggested.len();
                        let values: ::std::vec::Vec<::std::string::String> =
                            suggested.into_iter().take(#limit).collect();
                        let mut completion = ::serde_json::json!({ "values": values, "hasMore": total > #limit });
                        if total > #limit {
                            completion["total"] = ::serde_json::json!(total);
                        }
                        ::core::result::Result::Ok(::serde_json::json!({
                            "resultType": "complete",
                            "completion": completion,
                        }))
                    }
                },
            );
        }
    })
}
