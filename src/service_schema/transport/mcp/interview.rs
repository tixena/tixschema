//! Elicitation in the dispatcher: an arm of an operation's declared error that asks the user, with
//! a form or with a link, and the retried call that carries the answer.

use crate::rename_rule::RenameRule;
use crate::service_schema::mcp_tools::answers_key;
use crate::service_schema::parse::{OperationDef, ServiceDef, written};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

pub(super) fn any(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| operation.mcp.interview.is_some())
}

/// What the request says of the client, and the result that asks.
pub(super) fn helpers() -> TokenStream {
    quote! {
        /// Whether the client declared the elicitation `mode`: an empty `elicitation` is a form.
        fn declared_elicitation(params: &::serde_json::Value, mode: &str) -> bool {
            let declared = &params["_meta"]["io.modelcontextprotocol/clientCapabilities"]["elicitation"];
            if !declared.is_object() {
                return false;
            }
            if mode == "url" {
                return declared.get("url").is_some_and(::serde_json::Value::is_object);
            }
            declared.get("form").is_some_and(::serde_json::Value::is_object)
                || declared.get("url").is_none()
        }

        /// Whether the user declined or dismissed the request keyed `key`.
        fn dismissed(params: &::serde_json::Value, key: &str) -> bool {
            let action = params["inputResponses"][key]["action"].as_str();
            action == ::core::option::Option::Some("decline")
                || action == ::core::option::Option::Some("cancel")
        }

        /// The result that asks the client for the elicitation `request`, keyed `key`.
        fn input_required(key: &str, request: ::serde_json::Value) -> ::serde_json::Value {
            let mut requests = ::serde_json::Map::new();
            requests.insert(
                key.to_owned(),
                ::serde_json::json!({ "method": "elicitation/create", "params": request }),
            );
            ::serde_json::json!({ "resultType": "input_required", "inputRequests": requests })
        }
    }
}

/// The answers of a retried form call, read into the arguments ahead of the handler.
pub(super) fn preparation(
    operation: &OperationDef,
    module: &Ident,
    schema: &Ident,
    tool: &str,
) -> Option<TokenStream> {
    let interview = operation.mcp.interview.as_ref()?;
    interview.answers.as_ref()?;
    let key = answers_key(operation)?;
    let read = format_ident!(
        "mcp_read_form_{}",
        written(&operation.ident),
        span = operation.ident.span()
    );
    Some(quote! {
        let mut declined = false;
        if let ::core::option::Option::Some(response) = params
            .get("inputResponses")
            .and_then(|held| held.get(#key))
        {
            if response.get("action").and_then(::serde_json::Value::as_str)
                == ::core::option::Option::Some("accept")
            {
                let content = response
                    .get("content")
                    .cloned()
                    .unwrap_or_else(|| ::serde_json::Value::Object(::serde_json::Map::new()));
                if let ::core::result::Result::Err(detail) = $crate::#schema::#read(&content) {
                    return ::core::option::Option::Some(::core::result::Result::Ok(fault_result(
                        $crate::#module::ServiceFault::failed_validation(
                            #tool,
                            ::core::option::Option::Some(#key),
                            &detail,
                        ),
                    )));
                }
                arguments.insert(#key.to_owned(), content);
            } else {
                arguments.remove(#key);
                declined = true;
            }
        }
    })
}

/// `answered`, the arm for the declared error, preceded by the result that asks where the error is
/// the arm that asks and the client can be asked.
pub(super) fn asking(
    operation: &OperationDef,
    schema: &Ident,
    tool: &str,
    answered: TokenStream,
) -> TokenStream {
    let Some(interview) = operation.mcp.interview.as_ref() else {
        return answered;
    };
    let name = written(&operation.ident);
    let asks = format_ident!("mcp_asks_{name}", span = operation.ident.span());
    let ask = if interview.answers.is_some() {
        let Some(key) = answers_key(operation) else {
            return answered;
        };
        let form = format_ident!("mcp_form_{name}", span = operation.ident.span());
        let rustdoc = operation.doc.clone().unwrap_or_default();
        quote! {
            if !declined
                && $crate::#schema::#asks(&carried_value_0)
                && declared_elicitation(params, "form")
            {
                let said = match encoded(#tool, &carried_value_0) {
                    ::core::result::Result::Ok(said) => said,
                    ::core::result::Result::Err(refused) => {
                        return ::core::option::Option::Some(::core::result::Result::Err(refused));
                    }
                };
                let message = said
                    .get("message")
                    .and_then(::serde_json::Value::as_str)
                    .unwrap_or(#rustdoc);
                return ::core::option::Option::Some(::core::result::Result::Ok(input_required(
                    #key,
                    ::serde_json::json!({
                        "mode": "form",
                        "message": message,
                        "requestedSchema": $crate::#schema::#form(),
                    }),
                )));
            }
        }
    } else {
        let fallback = RenameRule::KebabCase.apply_to_variant(&interview.arm.to_string());
        quote! {
            if let ::core::option::Option::Some((url, message)) = $crate::#schema::#asks(&carried_value_0) {
                let said = match encoded(#tool, &carried_value_0) {
                    ::core::result::Result::Ok(said) => said,
                    ::core::result::Result::Err(refused) => {
                        return ::core::option::Option::Some(::core::result::Result::Err(refused));
                    }
                };
                let key = said
                    .get("errorCode")
                    .and_then(::serde_json::Value::as_str)
                    .unwrap_or(#fallback);
                if !dismissed(params, key) && declared_elicitation(params, "url") {
                    return ::core::option::Option::Some(::core::result::Result::Ok(input_required(
                        key,
                        ::serde_json::json!({ "mode": "url", "url": url, "message": message }),
                    )));
                }
            }
        }
    };
    quote! {
        {
            #ask
            #answered
        }
    }
}
