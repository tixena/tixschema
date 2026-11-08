//! Tasks in the Rust dispatcher: the handle a task tool answers to a client that declared the
//! extension, and the arms of `route` that answer `tasks/get`, `tasks/cancel` and `tasks/update`
//! by calling the operations the tool names.

use crate::service_schema::mcp_tools::has_tasks;
use crate::service_schema::parse::{
    McpTask, OperationDef, OperationInputs, ServiceDef, wire_key, written,
};
use crate::service_schema::support::{message_reader_ident, message_validator_ident};
use crate::service_schema::transport::amqp_rpc::call_arguments;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub(super) fn any(service: &ServiceDef) -> bool {
    has_tasks(service)
}

/// The member of `extensions` the catalog adds for the tasks.
pub(super) fn extension(service: &ServiceDef) -> Option<TokenStream> {
    any(service).then(|| quote! { "io.modelcontextprotocol/tasks": {} })
}

/// What the request says of the client, and the view of a task state.
pub(super) fn helpers() -> TokenStream {
    quote! {
        /// Whether the client declared the tasks extension.
        fn declared_tasks(params: &::serde_json::Value) -> bool {
            params["_meta"]["io.modelcontextprotocol/clientCapabilities"]["extensions"]["io.modelcontextprotocol/tasks"].is_object()
        }
    }
}

/// The arm for a task tool's success: the task result for a client that declared the extension,
/// and the handle as a structured value for any other.
pub(super) fn success_arm(tool: &str) -> TokenStream {
    quote! {
        match encoded(#tool, &carried_value_0) {
            ::core::result::Result::Ok(mut handle) => {
                handle["status"] = ::serde_json::Value::String("working".to_owned());
                if declared_tasks(params) {
                    let held = handle["taskId"].as_str().unwrap_or_default().to_owned();
                    handle["taskId"] = ::serde_json::Value::String(format!("{}:{}", #tool, held));
                    handle["resultType"] = ::serde_json::Value::String("task".to_owned());
                    ::core::option::Option::Some(::core::result::Result::Ok(handle))
                } else {
                    ::core::option::Option::Some(::core::result::Result::Ok(completed(
                        ::core::option::Option::Some(handle),
                        ::core::option::Option::None,
                    )))
                }
            }
            ::core::result::Result::Err(refused) => ::core::option::Option::Some(::core::result::Result::Err(refused)),
        }
    }
}

fn tasks(service: &ServiceDef) -> impl Iterator<Item = (&OperationDef, &McpTask)> {
    service
        .operations
        .iter()
        .filter_map(|operation| Some((operation, operation.mcp.task.as_ref()?)))
}

fn named<'service>(service: &'service ServiceDef, name: &str) -> Option<&'service OperationDef> {
    service
        .operations
        .iter()
        .find(|candidate| written(&candidate.ident) == name)
}

/// The statements that call `target` with the message in `source`, answering what `settle` makes of
/// a success.
fn call_block(
    target: &OperationDef,
    module: &Ident,
    source: &TokenStream,
    refused: &str,
    settle: &TokenStream,
) -> TokenStream {
    let wire = &target.wire_name;
    let method = &target.ident;
    let reader = message_reader_ident(target);
    let validator = message_validator_ident(target);
    let call = call_arguments(target);
    let reading = super::resources::reading(target, module, source, &reader, &validator);
    quote! {
        #reading
        match caught(move || svc.#method(ctx #(, #call)*)).await {
            ::core::result::Result::Err(panicked) => {
                record_panic(#wire, &panicked);
                return ::core::option::Option::Some(::core::result::Result::Err(internal_error(#wire, &panicked)));
            }
            ::core::result::Result::Ok(::core::result::Result::Err(error)) => {
                return ::core::option::Option::Some(match encoded(#wire, &error) {
                    ::core::result::Result::Ok(data) => ::core::result::Result::Err(invalid_params(
                        #refused,
                        ::core::option::Option::Some(data),
                    )),
                    ::core::result::Result::Err(refused) => ::core::result::Result::Err(refused),
                });
            }
            ::core::result::Result::Ok(::core::result::Result::Ok(answered)) => {
                #settle
            }
        }
    }
}

/// The block of `tasks/get` for the task tool `tool`, calling `target`.
fn get_block(tool: &str, target: &OperationDef, module: &Ident) -> TokenStream {
    let refused = format!("{tool}: the service refused the task");
    let wire = &target.wire_name;
    let settle = quote! {
        match encoded(#wire, &answered) {
            ::core::result::Result::Ok(state) => {
                return ::core::option::Option::Some(::core::result::Result::Ok(task_view(task_id, state)));
            }
            ::core::result::Result::Err(refused) => {
                return ::core::option::Option::Some(::core::result::Result::Err(refused));
            }
        }
    };
    let source = quote! { ::serde_json::Value::String(held.to_owned()) };
    let block = call_block(target, module, &source, &refused, &settle);
    quote! { if tool == #tool { #block } }
}

/// The block of `tasks/cancel` for the task tool `tool`, calling `target`.
fn cancel_block(tool: &str, target: &OperationDef, module: &Ident) -> TokenStream {
    let refused = format!("{tool}: the service refused to cancel the task");
    let settle = quote! {
        let _unit = answered;
        return ::core::option::Option::Some(::core::result::Result::Ok(::serde_json::json!({})));
    };
    let source = quote! { ::serde_json::Value::String(held.to_owned()) };
    let block = call_block(target, module, &source, &refused, &settle);
    quote! { if tool == #tool { #block } }
}

/// The block of `tasks/update` for the task tool `tool`, calling `target` once for each answer.
fn update_block(tool: &str, target: &OperationDef, module: &Ident) -> Option<TokenStream> {
    let refused = format!("{tool}: the service refused the answer");
    let OperationInputs::Generated(fields) = &target.inputs else {
        return None;
    };
    let keys: Vec<String> = fields.iter().map(|(field, _)| wire_key(field)).collect();
    let [id_key, key_key, action_key, content_key] = keys.as_slice() else {
        return None;
    };
    let source = quote! {
        ::serde_json::json!({
            #id_key: held,
            #key_key: key,
            #action_key: response["action"].as_str().unwrap_or_default(),
            #content_key: response.get("content").map(canonical),
        })
    };
    let settle = quote! { let _unit = answered; };
    let block = call_block(target, module, &source, &refused, &settle);
    Some(quote! {
        if tool == #tool {
            let mut answers: ::std::vec::Vec<_> = responses.iter().collect();
            answers.sort_by(|first, second| first.0.cmp(second.0));
            for (key, response) in answers {
                #block
            }
            return ::core::option::Option::Some(::core::result::Result::Ok(::serde_json::json!({})));
        }
    })
}

/// The `route` arm of `tasks/get`, `tasks/cancel` and `tasks/update`.
pub(super) fn arms(service: &ServiceDef, module: &Ident) -> Option<TokenStream> {
    if !any(service) {
        return None;
    }
    let mut get = Vec::new();
    let mut cancel = Vec::new();
    let mut update = Vec::new();
    for (start, task) in tasks(service) {
        let tool = start.mcp_tool_name();
        if let Some(target) = task.get.as_deref().and_then(|name| named(service, name)) {
            get.push(get_block(tool, target, module));
        }
        if let Some(target) = task.cancel.as_deref().and_then(|name| named(service, name)) {
            cancel.push(cancel_block(tool, target, module));
        }
        if let Some(target) = task.update.as_deref().and_then(|name| named(service, name)) {
            update.extend(update_block(tool, target, module));
        }
    }
    let getting = (!get.is_empty()).then(|| {
        quote! { "tasks/get" => { #(#get)* ::core::option::Option::None } }
    });
    let cancelling = (!cancel.is_empty()).then(|| {
        quote! { "tasks/cancel" => { #(#cancel)* ::core::option::Option::None } }
    });
    let updating = (!update.is_empty()).then(|| {
        quote! {
            "tasks/update" => {
                let ::core::option::Option::Some(responses) = params.get("inputResponses").and_then(::serde_json::Value::as_object) else {
                    return ::core::option::Option::Some(::core::result::Result::Err(invalid_params(
                        "Invalid params: tasks/update needs `inputResponses`",
                        ::core::option::Option::None,
                    )));
                };
                #(#update)*
                ::core::option::Option::None
            }
        }
    });
    Some(quote! {
        "tasks/get" | "tasks/cancel" | "tasks/update" => {
            let task_id = params.get("taskId")?.as_str()?;
            let (tool, held) = task_id.split_once(':')?;
            match method {
                #getting
                #cancelling
                #updating
                _ => ::core::option::Option::None,
            }
        }
    })
}
