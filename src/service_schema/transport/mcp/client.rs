//! The client half of the `mcp` transport: one `tools/call` per operation, sent through the
//! caller's `Transport`.

use super::{Carried, error_carried, success_carried};
use crate::features::service_schema::registry_ident;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationOutcome, ServiceDef, argument_local, is_unit_type, written,
};
use crate::service_schema::support::{message_validator_ident, module_ident};
use crate::service_schema::transport::amqp_rpc::{
    Generated, answers, call_message, fault_mirror_readers, fault_wire_types, outbound_refusal,
    placement_doc,
};
use crate::service_schema::transport::{Transport, client_macro_ident};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

pub(super) fn macro_tokens(service: &ServiceDef, transport: Transport) -> TokenStream {
    let contract = &service.ident;
    let client = format_ident!("{contract}Client", span = contract.span());
    let published = client_macro_ident(service, transport);
    let generated = Generated::of(module_ident(service));
    let schema = registry_ident(service);
    let methods = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
        .map(|operation| method(operation, &generated, &schema));
    let placement = placement_doc(
        &published,
        "mcp_client",
        "use the_contract_crate::{GetVersionRequest, VersionError, VersionResponse};\n\n\
         the_contract_crate::",
    );
    let macro_doc = format!(
        "The `{contract}` client for the `{}` transport, held as tokens rather than compiled \
         here.\n\n\
         It takes no arguments and emits bare items - the `Transport` seam, `McpMessage`, the \
         client type and one method per operation - so the caller supplies the module they land \
         in and two transports in one crate cannot collide. The invoking crate names `serde` and \
         `serde_json` in its own manifest, because the items below call them. The service's \
         `{schema}` is reached as `$crate::{schema}`, so a service declared in a submodule \
         re-exports it at the crate root.\n\n\
         {placement}\n\n\
         The `use` names the types the author declared one by one: the messages, the successes \
         and the errors a method signature spells.",
        transport.name()
    );
    let client_doc = format!(
        "A `{contract}` caller over any `Transport` that can carry a JSON-RPC text to a server and \
         bring its answer back.\n\n\
         Every operation on the trait has a method here, taking that operation's arguments and \
         nothing else: the context is the implementation's, and a caller has none to hand."
    );
    let fault = &generated.fault;
    let wire = fault_wire_types();
    let readers = fault_mirror_readers(&generated);
    let canonical = super::canonical_json();
    let helpers = helpers(fault);
    let exchange = exchange(fault);
    quote! {
        #[doc = #macro_doc]
        #[macro_export]
        macro_rules! #published {
            () => {
                const PROTOCOL_VERSION: &str = "2026-07-28";

                /// One JSON-RPC text for the seam to carry, and the values an HTTP seam writes as
                /// headers. A stdio seam writes the text alone.
                #[derive(Debug)]
                pub struct McpMessage {
                    pub method: &'static str,
                    pub name: ::core::option::Option<::std::string::String>,
                    pub protocol_version: &'static str,
                    pub text: ::std::string::String,
                }

                /// What binds a client to a server: it carries one message and answers the text
                /// that came back. `Err` is a call that did not travel, in words.
                pub trait Transport {
                    /// Sends one message and answers the server's text.
                    fn send(&self, message: McpMessage) -> impl ::core::future::Future<
                        Output = ::core::result::Result<::std::string::String, ::std::string::String>,
                    > + ::core::marker::Send;
                }

                #wire
                #readers

                #[doc = #client_doc]
                pub struct #client<T: Transport> {
                    next: ::core::sync::atomic::AtomicU64,
                    transport: T,
                }

                impl<T: Transport> #client<T> {
                    /// Binds a client to a transport.
                    pub const fn new(transport: T) -> Self {
                        Self { transport, next: ::core::sync::atomic::AtomicU64::new(0) }
                    }

                    /// The transport this client was bound to.
                    pub const fn transport(&self) -> &T {
                        &self.transport
                    }
                }

                impl<T: Transport + ::core::marker::Sync> #client<T> {
                    #(#methods)*
                }

                #canonical
                #helpers
                #exchange
            };
        }
    }
}

/// The functions every method reads an answer and makes a call through.
fn helpers(fault: &TokenStream) -> TokenStream {
    quote! {
        fn unreadable(tool: &str, detail: &str) -> #fault {
            #fault::undeserializable_payload(tool, detail)
        }

        /// A value of the declared type, or the fault of the answer that holds none.
        fn decoded<V>(tool: &str, value: ::serde_json::Value) -> ::core::result::Result<V, #fault>
        where
            V: ::serde::de::DeserializeOwned,
        {
            ::serde_json::from_value(value)
                .map_err(|rejected| unreadable(tool, &rejected.to_string()))
        }

        /// The fault a value tagged `isServiceFault` carries.
        fn reported_fault(tool: &str, tagged: ::serde_json::Value) -> #fault {
            match ::serde_json::from_value::<TaggedFault>(tagged) {
                ::core::result::Result::Ok(tagged) => tagged.reported(tool),
                ::core::result::Result::Err(rejected) => unreadable(tool, &rejected.to_string()),
            }
        }

        /// One answer read: the success's structured value, the error the operation
        /// declared, or the fault the answer reports.
        fn read_answer(
            tool: &str,
            text: &str,
        ) -> ::core::result::Result<
            ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
            #fault,
        > {
            let answered: ::serde_json::Value = match ::serde_json::from_str(text) {
                ::core::result::Result::Ok(answered) => answered,
                ::core::result::Result::Err(rejected) => {
                    return ::core::result::Result::Err(unreadable(tool, &rejected.to_string()));
                }
            };
            if let ::core::option::Option::Some(error) = answered.get("error") {
                if error["data"].get("isServiceFault").is_some() {
                    return ::core::result::Result::Err(reported_fault(tool, error["data"].clone()));
                }
                let said = format!("{}: {}", error["code"], error["message"].as_str().unwrap_or_default());
                return ::core::result::Result::Err(#fault::transport_failure(tool, &said));
            }
            let ::core::option::Option::Some(result) = answered.get("result") else {
                return ::core::result::Result::Err(unreadable(tool, "the answer holds neither a result nor an error"));
            };
            if result["isError"] != true {
                return ::core::result::Result::Ok(::core::result::Result::Ok(
                    result.get("structuredContent").cloned().unwrap_or(::serde_json::Value::Null),
                ));
            }
            let ::core::option::Option::Some(text) = result["content"][0]["text"].as_str() else {
                return ::core::result::Result::Err(unreadable(tool, "the error holds no text"));
            };
            let carried: ::serde_json::Value = match ::serde_json::from_str(text) {
                ::core::result::Result::Ok(carried) => carried,
                ::core::result::Result::Err(rejected) => {
                    return ::core::result::Result::Err(unreadable(tool, &rejected.to_string()));
                }
            };
            if carried.get("isServiceFault").is_some() {
                return ::core::result::Result::Err(reported_fault(tool, carried));
            }
            ::core::result::Result::Ok(::core::result::Result::Err(carried))
        }
    }
}

/// The call every method makes through the seam.
fn exchange(fault: &TokenStream) -> TokenStream {
    quote! {
        /// Sends one `tools/call` and reads the answer.
        async fn exchange<T>(
            transport: &T,
            next: &::core::sync::atomic::AtomicU64,
            tool: &'static str,
            arguments: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
        ) -> ::core::result::Result<
            ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
            #fault,
        >
        where
            T: Transport + ::core::marker::Sync,
        {
            let id = next.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed) + 1;
            let text = canonical(&::serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {
                    "name": tool,
                    "arguments": arguments,
                    "_meta": {
                        "io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
                        "io.modelcontextprotocol/clientCapabilities": {},
                        "io.modelcontextprotocol/clientInfo": {
                            "name": env!("CARGO_PKG_NAME"),
                            "version": env!("CARGO_PKG_VERSION"),
                        },
                    },
                },
            }));
            let message = McpMessage {
                text,
                protocol_version: PROTOCOL_VERSION,
                method: "tools/call",
                name: ::core::option::Option::Some(tool.to_owned()),
            };
            match transport.send(message).await {
                ::core::result::Result::Ok(answer) => read_answer(tool, &answer),
                ::core::result::Result::Err(uncarried) => {
                    ::core::result::Result::Err(#fault::transport_failure(tool, &uncarried))
                }
            }
        }
    }
}

/// One operation's client method: validate, write the arguments, send and read the answer.
fn method(operation: &OperationDef, generated: &Generated, schema: &Ident) -> TokenStream {
    let module = &generated.module;
    let named = &operation.ident;
    let tool = operation.mcp_tool_name();
    let check = message_validator_ident(operation);
    let wrapped = format_ident!(
        "wrapped_{}",
        written(&operation.ident),
        span = operation.ident.span()
    );
    let (message_taken, packed) = call_message(operation, module);
    let bound = HttpShape::of(operation).header_in;
    let header_taken = bound.iter().map(|header| {
        let parameter = &header.parameter;
        let ty = &header.ty;
        quote! { #parameter: #ty }
    });
    let taken: Vec<TokenStream> = message_taken.into_iter().chain(header_taken).collect();
    let invalid = outbound_refusal(operation, generated);
    let answers = answers(operation, generated);
    let doc = method_doc(operation);
    let message_refused = refusal(
        operation,
        generated,
        &quote! { &rejected.to_string() },
        &quote! { ::core::option::Option::None },
    );
    let not_an_object = refusal(
        operation,
        generated,
        &quote! { "the message is not a JSON object" },
        &quote! { ::core::option::Option::None },
    );
    let header_writes = header_writes(operation, generated);
    let settled = settled(operation, tool, generated);
    quote! {
        #[doc = #doc]
        pub fn #named(
            &self #(, #taken)*
        ) -> impl ::core::future::Future<Output = #answers> + ::core::marker::Send {
            async move {
                #packed
                if let ::core::result::Result::Err(violations) = $crate::#module::#check(&sending) {
                    #invalid
                }
                let message = match ::serde_json::to_value(&sending) {
                    ::core::result::Result::Ok(message) => message,
                    ::core::result::Result::Err(rejected) => {
                        #message_refused
                    }
                };
                let mut arguments = ::serde_json::Map::new();
                if $crate::#schema::#wrapped() {
                    arguments.insert("value".to_owned(), message);
                } else if let ::serde_json::Value::Object(members) = message {
                    arguments = members;
                } else {
                    #not_an_object
                }
                #(#header_writes)*
                let settled = exchange(&self.transport, &self.next, #tool, arguments).await;
                #settled
            }
        }
    }
}

/// What the read answer becomes: the success the operation declared, its declared error, or the
/// fault.
fn settled(operation: &OperationDef, tool: &str, generated: &Generated) -> TokenStream {
    let Generated {
        call_error,
        fault,
        module: _module,
    } = generated;
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return quote! {
            match settled {
                ::core::result::Result::Ok(::core::result::Result::Ok(_)) => ::core::result::Result::Ok(()),
                ::core::result::Result::Ok(::core::result::Result::Err(_)) => ::core::result::Result::Err(
                    #fault::undeserializable_payload(#tool, "a one-way operation answered an error"),
                ),
                ::core::result::Result::Err(reported) => ::core::result::Result::Err(reported),
            }
        };
    };
    let success_carried = success_carried(operation, success);
    let error_carried = error_carried(operation, error);
    let (found, success_read) = if success_carried.unit && is_unit_type(success) {
        (quote!(_), quote! { ::core::result::Result::Ok(#success) })
    } else {
        (
            quote!(found),
            carried_read(&success_carried, tool, call_error, &quote!(found)),
        )
    };
    let error_read = carried_read(&error_carried, tool, call_error, &quote!(declared));
    quote! {
        match settled {
            ::core::result::Result::Ok(::core::result::Result::Ok(#found)) => {
                #success_read
            }
            ::core::result::Result::Ok(::core::result::Result::Err(declared)) => {
                match { #error_read } {
                    ::core::result::Result::Ok(read) => ::core::result::Result::Err(#call_error::Operation(read)),
                    ::core::result::Result::Err(refused) => ::core::result::Result::Err(refused),
                }
            }
            ::core::result::Result::Err(reported) => ::core::result::Result::Err(#call_error::Fault(reported)),
        }
    }
}

/// An expression that reads the declared value, and the headers beside it, out of `source`: the
/// value itself, or the object `{ "value": ..., "<header>": ... }`. It answers `Ok` with the
/// declared type, or `Err` with the `CallError` of the fault that read refused.
fn carried_read(
    carried: &Carried,
    tool: &str,
    call_error: &TokenStream,
    source: &TokenStream,
) -> TokenStream {
    let refusing =
        |fault: TokenStream| quote! { ::core::result::Result::Err(#call_error::Fault(#fault)) };
    let value_types = &carried.value_types;
    if carried.header_names.is_empty() {
        let value_type = &value_types[0];
        let refused = refusing(quote! { reported });
        return quote! {
            match decoded::<#value_type>(#tool, #source) {
                ::core::result::Result::Ok(read) => ::core::result::Result::Ok(read),
                ::core::result::Result::Err(reported) => #refused,
            }
        };
    }
    let names = &carried.header_names;
    let header_types = &carried.header_types;
    let values: Vec<Ident> = (0..value_types.len())
        .map(|position| format_ident!("carried_value_{position}"))
        .collect();
    let headers: Vec<Ident> = (0..names.len())
        .map(|position| format_ident!("carried_header_{position}"))
        .collect();
    let refused = refusing(quote! { reported });
    let not_an_object = refusing(quote! { unreadable(#tool, "the answer holds no object") });
    let value_read = if value_types.len() == 1 {
        let (only, only_type) = (&values[0], &value_types[0]);
        quote! {
            let #only: #only_type = match decoded(#tool, members.remove("value").unwrap_or(::serde_json::Value::Null)) {
                ::core::result::Result::Ok(read) => read,
                ::core::result::Result::Err(reported) => return #refused,
            };
        }
    } else {
        quote! {
            let (#(#values),*): (#(#value_types),*) = match decoded(#tool, members.remove("value").unwrap_or(::serde_json::Value::Null)) {
                ::core::result::Result::Ok(read) => read,
                ::core::result::Result::Err(reported) => return #refused,
            };
        }
    };
    quote! {
        (|| {
            let ::serde_json::Value::Object(mut members) = #source else {
                return #not_an_object;
            };
            #value_read
            #(
                let #headers: #header_types = match decoded(#tool, members.remove(#names).unwrap_or(::serde_json::Value::Null)) {
                    ::core::result::Result::Ok(read) => read,
                    ::core::result::Result::Err(reported) => return #refused,
                };
            )*
            ::core::result::Result::Ok((#(#values,)* #(#headers),*))
        })()
    }
}

fn method_doc(operation: &OperationDef) -> String {
    let named = &operation.ident;
    let tool = operation.mcp_tool_name();
    let context = " No context is taken. A context is what an implementation needs and a caller \
                    has nothing to hand one to, so a call carries the message and nothing else.";
    match &operation.outcome {
        OperationOutcome::OneWay => format!(
            " Calls the tool `{tool}` and waits for its empty answer.\n\n\
            {context}\n\n\
             # Errors\n\n\
             A message that fails its own validator never reaches the transport, and the fault \
             names the field. A call the transport could not carry, and a fault in the answer, \
             come back as the fault they are."
        ),
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => format!(
            " Calls the tool `{tool}` and waits for the answer.\n\n\
            {context}\n\n\
             # Errors\n\n\
             `Err(CallError::Operation(…))` is the error `{named}` declared. \
             `Err(CallError::Fault(…))` is a defect: the server reported one, this client refused \
             the message it was about to send, in which case the transport was never reached, or \
             the call never landed."
        ),
    }
}

/// What a message or a header that cannot be written answers: the fault, and no call of `send`.
fn refusal(
    operation: &OperationDef,
    generated: &Generated,
    detail: &TokenStream,
    field: &TokenStream,
) -> TokenStream {
    let Generated {
        call_error,
        fault,
        module: _module,
    } = generated;
    let tool = operation.mcp_tool_name();
    let built = quote! { #fault::failed_validation(#tool, #field, #detail) };
    match &operation.outcome {
        OperationOutcome::OneWay => quote! { return ::core::result::Result::Err(#built); },
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => quote! { return ::core::result::Result::Err(#call_error::Fault(#built)); },
    }
}

/// Each header argument written beside the message, under its header's name. A `None` is left out.
fn header_writes(operation: &OperationDef, generated: &Generated) -> Vec<TokenStream> {
    HttpShape::of(operation)
        .header_in
        .iter()
        .map(|header| {
            let name = &header.name;
            let held = argument_local(&header.parameter);
            let refused = refusal(
                operation,
                generated,
                &quote! { &rejected.to_string() },
                &quote! { ::core::option::Option::Some(#name) },
            );
            quote! {
                match ::serde_json::to_value(&#held) {
                    ::core::result::Result::Ok(written) => {
                        if !written.is_null() {
                            arguments.insert(#name.to_owned(), written);
                        }
                    }
                    ::core::result::Result::Err(rejected) => {
                        #refused
                    }
                }
            }
        })
        .collect()
}
