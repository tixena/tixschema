//! The client half of the `mcp` transport: one `tools/call` per operation, sent through the
//! caller's `Transport`.

use super::{Carried, error_carried, success_carried};
use crate::features::service_schema::registry_ident;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, argument_local,
    is_unit_type, wire_key, written,
};
use crate::service_schema::support::{message_validator_ident, module_ident};
use crate::service_schema::transport::amqp_rpc::{
    Generated, answers, call_message, fault_mirror_readers, fault_wire_types, outbound_refusal,
    placement_doc,
};
use crate::service_schema::transport::{Transport, client_macro_ident};
use proc_macro2::TokenStream;
use quote::ToTokens as _;
use quote::{format_ident, quote};
use syn::Ident;
use syn::Type;

/// The message a seam carries.
fn message_type() -> TokenStream {
    quote! {
        /// One JSON-RPC text for the seam to carry, and the values an HTTP seam writes as
        /// headers. A stdio seam writes the text alone.
        #[derive(Debug)]
        pub struct McpMessage {
            method: &'static str,
            name: ::core::option::Option<::std::string::String>,
            protocol_version: &'static str,
            text: ::std::string::String,
        }

        impl McpMessage {
            /// The JSON-RPC method, for the `Mcp-Method` header.
            pub const fn method(&self) -> &'static str {
                self.method
            }

            /// The tool, prompt or resource name, for the `Mcp-Name` header, where there is one.
            pub fn name(&self) -> ::core::option::Option<&str> {
                self.name.as_deref()
            }

            /// Binds the text to send and the values an HTTP seam writes as headers.
            pub const fn new(
                method: &'static str,
                name: ::core::option::Option<::std::string::String>,
                protocol_version: &'static str,
                text: ::std::string::String,
            ) -> Self {
                Self { method, name, protocol_version, text }
            }

            /// The protocol revision, for the `MCP-Protocol-Version` header.
            pub const fn protocol_version(&self) -> &'static str {
                self.protocol_version
            }

            /// The JSON-RPC text.
            pub fn text(&self) -> &str {
                &self.text
            }
        }
    }
}

pub(super) fn macro_tokens(service: &ServiceDef, transport: Transport) -> TokenStream {
    let contract = &service.ident;
    let client = format_ident!("{contract}Client", span = contract.span());
    let published = client_macro_ident(service, transport);
    let generated = Generated::of(module_ident(service));
    let schema = registry_ident(service);
    let methods = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool() || operation.is_mcp_prompt())
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
         in and two transports in one crate cannot collide. The invoking crate names `serde`, \
         `serde_json` and `serde_path_to_error` in its own manifest, because the items below call \
         them. The service's \
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
    let message = message_type();
    let wire = fault_wire_types();
    let readers = fault_mirror_readers(&generated);
    let canonical = super::canonical_json();
    let tools = service.operations.iter().any(OperationDef::is_mcp_tool);
    let prompting = service.operations.iter().any(OperationDef::is_mcp_prompt);
    let helpers = helpers(fault);
    let reading_tools = tools.then(|| tool_reader(fault));
    let reading_prompts = prompting.then(|| prompt_reader(fault));
    let texting = service
        .operations
        .iter()
        .any(|operation| operation.is_mcp_prompt() && answers_a_string(operation));
    let reading_text = texting.then(|| text_reader(fault));
    let exchange = exchange(fault, tools, prompting);
    let blocks = content_helpers(service, &generated);
    quote! {
        #[doc = #macro_doc]
        #[macro_export]
        macro_rules! #published {
            () => {
                const PROTOCOL_VERSION: &str = "2026-07-28";

                #message

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
                #reading_tools
                #reading_prompts
                #reading_text
                #blocks
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
            ::serde_path_to_error::deserialize(value).map_err(|rejected| {
                let steps: ::std::vec::Vec<::std::string::String> = rejected
                    .path()
                    .iter()
                    .filter_map(|step| match step {
                        ::serde_path_to_error::Segment::Seq { index } => {
                            ::core::option::Option::Some(index.to_string())
                        }
                        ::serde_path_to_error::Segment::Map { key } => {
                            ::core::option::Option::Some(key.clone())
                        }
                        ::serde_path_to_error::Segment::Enum { variant } => {
                            ::core::option::Option::Some(variant.clone())
                        }
                        ::serde_path_to_error::Segment::Unknown => ::core::option::Option::None,
                    })
                    .collect();
                if steps.is_empty() {
                    unreadable(tool, &rejected.inner().to_string())
                } else {
                    unreadable(tool, &format!("'{}': {}", steps.join("."), rejected.inner()))
                }
            })
        }

        /// The fault a value tagged `isServiceFault` carries.
        fn reported_fault(tool: &str, tagged: ::serde_json::Value) -> #fault {
            match ::serde_json::from_value::<TaggedFault>(tagged) {
                ::core::result::Result::Ok(tagged) => tagged.reported(tool),
                ::core::result::Result::Err(rejected) => unreadable(tool, &rejected.to_string()),
            }
        }
    }
}

/// What reads a tool's answer.
fn tool_reader(fault: &TokenStream) -> TokenStream {
    quote! {
        /// One answer read: the success's structured value (with `blocks`, its `content` beside
        /// it), the error the operation declared, or the fault the answer reports.
        fn read_answer(
            tool: &str,
            text: &str,
            blocks: bool,
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
                let found = if blocks {
                    ::serde_json::json!({
                        "content": result.get("content").cloned().unwrap_or(::serde_json::Value::Null),
                        "structuredContent": result.get("structuredContent").cloned().unwrap_or(::serde_json::Value::Null),
                    })
                } else {
                    result.get("structuredContent").cloned().unwrap_or(::serde_json::Value::Null)
                };
                return ::core::result::Result::Ok(::core::result::Result::Ok(found));
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

/// What reads a prompt's answer.
fn prompt_reader(fault: &TokenStream) -> TokenStream {
    quote! {
        /// One prompt answer read: its result, the error the operation declared (a `-32602` whose
        /// `data` holds no fault), or the fault the answer reports.
        fn read_prompt(
            prompt: &str,
            text: &str,
        ) -> ::core::result::Result<
            ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
            #fault,
        > {
            let answered: ::serde_json::Value = match ::serde_json::from_str(text) {
                ::core::result::Result::Ok(answered) => answered,
                ::core::result::Result::Err(rejected) => {
                    return ::core::result::Result::Err(unreadable(prompt, &rejected.to_string()));
                }
            };
            if let ::core::option::Option::Some(error) = answered.get("error") {
                let data = &error["data"];
                if data.get("isServiceFault").is_some() {
                    return ::core::result::Result::Err(reported_fault(prompt, data.clone()));
                }
                if error["code"] == -32602_i64 && !data.is_null() {
                    return ::core::result::Result::Ok(::core::result::Result::Err(data.clone()));
                }
                let said = format!("{}: {}", error["code"], error["message"].as_str().unwrap_or_default());
                return ::core::result::Result::Err(#fault::transport_failure(prompt, &said));
            }
            match answered.get("result") {
                ::core::option::Option::Some(result) => {
                    ::core::result::Result::Ok(::core::result::Result::Ok(result.clone()))
                }
                ::core::option::Option::None => ::core::result::Result::Err(unreadable(
                    prompt,
                    "the answer holds neither a result nor an error",
                )),
            }
        }
    }
}

/// What reads the text of a prompt's first message, written where a prompt answers a `String`.
fn text_reader(fault: &TokenStream) -> TokenStream {
    quote! {
        /// The text of the first message of a prompt's result.
        fn first_text(prompt: &str, result: &::serde_json::Value) -> ::core::result::Result<::std::string::String, #fault> {
            result["messages"][0]["content"]["text"]
                .as_str()
                .map(::std::borrow::ToOwned::to_owned)
                .ok_or_else(|| unreadable(prompt, "the first message holds no text"))
        }
    }
}

fn is_a_string(success: &Type) -> bool {
    success.to_token_stream().to_string().replace(' ', "") == "String"
}

fn answers_a_string(operation: &OperationDef) -> bool {
    match &operation.outcome {
        OperationOutcome::Reply {
            error: _error,
            success,
        } => is_a_string(success),
        OperationOutcome::OneWay => false,
    }
}

/// What reads the blocks of a content tool's answer, written where one is declared.
fn content_helpers(service: &ServiceDef, generated: &Generated) -> TokenStream {
    let pair = service.operations.iter().any(super::content::is_pair);
    let list = service.operations.iter().any(super::content::is_list);
    let messages = super::content::prompt_messages(service);
    if !pair && !list && !messages {
        return TokenStream::new();
    }
    let fault = &generated.fault;
    let module = &generated.module;
    let decoder = super::resources::base64_decoder();
    let reading_pair = pair.then(|| pair_reader(fault));
    let reading_list = (list || messages).then(|| list_reader(fault, module, list));
    let reading_messages = messages.then(|| message_reader(fault, module));
    quote! {
        #decoder

        #reading_pair
        #reading_list
        #reading_messages
    }
}

fn pair_reader(fault: &TokenStream) -> TokenStream {
    quote! {
        /// The bytes and the content type of an answer: its structured pair, or its one image
        /// or audio block.
        fn read_pair(
            tool: &str,
            found: ::serde_json::Value,
        ) -> ::core::result::Result<(::std::vec::Vec<u8>, ::std::string::String), #fault> {
            let structured = &found["structuredContent"];
            if !structured.is_null() {
                return decoded(tool, structured.clone());
            }
            let block = &found["content"][0];
            let (::core::option::Option::Some(data), ::core::option::Option::Some(mime_type)) =
                (block["data"].as_str(), block["mimeType"].as_str())
            else {
                return ::core::result::Result::Err(unreadable(tool, "the answer holds no image or audio block"));
            };
            match base64_decoded(data) {
                ::core::option::Option::Some(bytes) => ::core::result::Result::Ok((bytes, mime_type.to_owned())),
                ::core::option::Option::None => ::core::result::Result::Err(unreadable(tool, "the block holds bytes that are not base64")),
            }
        }
    }
}

fn list_reader(fault: &TokenStream, module: &Ident, blocks: bool) -> TokenStream {
    let listing = blocks.then(|| {
        quote! {
            /// The blocks of `content`, each read into the type the service module declares.
            fn read_blocks(
                tool: &str,
                content: &::serde_json::Value,
            ) -> ::core::result::Result<::std::vec::Vec<$crate::#module::McpContent>, #fault> {
                let ::core::option::Option::Some(blocks) = content.as_array() else {
                    return ::core::result::Result::Err(unreadable(tool, "the answer holds no content"));
                };
                blocks.iter().map(|block| read_block(tool, block)).collect()
            }
        }
    });
    quote! {
        /// The text a block holds under `key`.
        fn block_text(tool: &str, block: &::serde_json::Value, key: &str) -> ::core::result::Result<::std::string::String, #fault> {
            block[key]
                .as_str()
                .map(::std::borrow::ToOwned::to_owned)
                .ok_or_else(|| unreadable(tool, &format!("a block holds no `{key}`")))
        }

        /// The bytes a block holds under `data`.
        fn block_bytes(tool: &str, block: &::serde_json::Value, key: &str) -> ::core::result::Result<::std::vec::Vec<u8>, #fault> {
            base64_decoded(&block_text(tool, block, key)?)
                .ok_or_else(|| unreadable(tool, &format!("`{key}` is not base64")))
        }

        /// One block, read into the type the service module declares.
        fn read_block(
            tool: &str,
            block: &::serde_json::Value,
        ) -> ::core::result::Result<$crate::#module::McpContent, #fault> {
            let optional = |key: &str| block[key].as_str().map(::std::borrow::ToOwned::to_owned);
            ::core::result::Result::Ok(match block["type"].as_str() {
                ::core::option::Option::Some("text") => $crate::#module::McpContent::Text {
                    text: block_text(tool, block, "text")?,
                },
                ::core::option::Option::Some("image") => $crate::#module::McpContent::Image {
                    data: block_bytes(tool, block, "data")?,
                    mime_type: block_text(tool, block, "mimeType")?,
                },
                ::core::option::Option::Some("audio") => $crate::#module::McpContent::Audio {
                    data: block_bytes(tool, block, "data")?,
                    mime_type: block_text(tool, block, "mimeType")?,
                },
                ::core::option::Option::Some("resource_link") => $crate::#module::McpContent::ResourceLink {
                    uri: block_text(tool, block, "uri")?,
                    name: block_text(tool, block, "name")?,
                    title: optional("title"),
                    description: optional("description"),
                    mime_type: optional("mimeType"),
                },
                ::core::option::Option::Some("resource") => {
                    let resource = &block["resource"];
                    $crate::#module::McpContent::Resource {
                        uri: block_text(tool, resource, "uri")?,
                        mime_type: resource["mimeType"].as_str().map(::std::borrow::ToOwned::to_owned),
                        contents: if resource.get("blob").is_some() {
                            $crate::#module::McpResourceContents::Blob(block_bytes(tool, resource, "blob")?)
                        } else {
                            $crate::#module::McpResourceContents::Text(block_text(tool, resource, "text")?)
                        },
                    }
                }
                _ => return ::core::result::Result::Err(unreadable(tool, "a block of a type this client does not read")),
            })
        }

        #listing
    }
}

/// What reads the messages of a prompt's result.
fn message_reader(fault: &TokenStream, module: &Ident) -> TokenStream {
    quote! {
        /// The messages of a prompt's result, each read into the type the service module declares.
        fn read_messages(
            prompt: &str,
            messages: &::serde_json::Value,
        ) -> ::core::result::Result<::std::vec::Vec<$crate::#module::McpPromptMessage>, #fault> {
            let ::core::option::Option::Some(held) = messages.as_array() else {
                return ::core::result::Result::Err(unreadable(prompt, "the answer holds no messages"));
            };
            held.iter()
                .map(|message| {
                    let role = match message["role"].as_str() {
                        ::core::option::Option::Some("user") => $crate::#module::McpRole::User,
                        ::core::option::Option::Some("assistant") => $crate::#module::McpRole::Assistant,
                        _ => return ::core::result::Result::Err(unreadable(prompt, "a message of a role this client does not read")),
                    };
                    ::core::result::Result::Ok($crate::#module::McpPromptMessage {
                        role,
                        content: read_block(prompt, &message["content"])?,
                    })
                })
                .collect()
        }
    }
}

/// The calls every method makes through the seam.
fn exchange(fault: &TokenStream, tools: bool, prompting: bool) -> TokenStream {
    let calling = tools.then(|| {
        quote! {
            /// Sends one `tools/call` and reads the answer.
            async fn exchange<T>(
                transport: &T,
                next: &::core::sync::atomic::AtomicU64,
                tool: &'static str,
                arguments: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
                blocks: bool,
            ) -> ::core::result::Result<
                ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
                #fault,
            >
            where
                T: Transport + ::core::marker::Sync,
            {
                match sent(transport, next, "tools/call", tool, arguments).await {
                    ::core::result::Result::Ok(answer) => read_answer(tool, &answer, blocks),
                    ::core::result::Result::Err(uncarried) => {
                        ::core::result::Result::Err(#fault::transport_failure(tool, &uncarried))
                    }
                }
            }
        }
    });
    let prompted = prompting.then(|| {
        quote! {
            /// Sends one `prompts/get` and reads the answer.
            async fn prompted<T>(
                transport: &T,
                next: &::core::sync::atomic::AtomicU64,
                prompt: &'static str,
                arguments: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
            ) -> ::core::result::Result<
                ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
                #fault,
            >
            where
                T: Transport + ::core::marker::Sync,
            {
                match sent(transport, next, "prompts/get", prompt, arguments).await {
                    ::core::result::Result::Ok(answer) => read_prompt(prompt, &answer),
                    ::core::result::Result::Err(uncarried) => {
                        ::core::result::Result::Err(#fault::transport_failure(prompt, &uncarried))
                    }
                }
            }
        }
    });
    quote! {
        /// Sends one request through the seam and answers the text that came back.
        async fn sent<T>(
            transport: &T,
            next: &::core::sync::atomic::AtomicU64,
            method: &'static str,
            name: &'static str,
            arguments: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
        ) -> ::core::result::Result<::std::string::String, ::std::string::String>
        where
            T: Transport + ::core::marker::Sync,
        {
            let id = next.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed) + 1;
            let text = canonical(&::serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": {
                    "name": name,
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
            let message = McpMessage::new(
                method,
                ::core::option::Option::Some(name.to_owned()),
                PROTOCOL_VERSION,
                text,
            );
            transport.send(message).await
        }

        #calling
        #prompted
    }
}

/// One prompt's client method: validate, write the arguments as strings, send `prompts/get` and
/// read the messages.
fn prompt_method(operation: &OperationDef, generated: &Generated) -> TokenStream {
    let module = &generated.module;
    let named = &operation.ident;
    let prompt = operation.mcp_tool_name();
    let check = message_validator_ident(operation);
    let (taken, packed) = call_message(operation, module);
    let invalid = outbound_refusal(operation, generated);
    let answers = answers(operation, generated);
    let doc = method_doc(operation);
    let refused = refusal(
        operation,
        generated,
        &quote! { &rejected.to_string() },
        &quote! { ::core::option::Option::None },
    );
    let not_strings = refusal(
        operation,
        generated,
        &quote! { "the arguments are not text" },
        &quote! { ::core::option::Option::None },
    );
    let single = matches!(operation.inputs, OperationInputs::Named(_)).then(|| {
        let key = operation
            .mcp
            .prompt
            .iter()
            .filter_map(|held| held.arguments.first())
            .map(|(argument, _)| wire_key(argument))
            .next()
            .unwrap_or_default();
        quote! {
            ::serde_json::Value::String(held) => {
                arguments.insert(#key.to_owned(), ::serde_json::Value::String(held));
            }
        }
    });
    let settled = settled(operation, prompt, generated);
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
                        #refused
                    }
                };
                let mut arguments = ::serde_json::Map::new();
                match message {
                    ::serde_json::Value::Object(members) => {
                        arguments.extend(members.into_iter().filter(|(_, held)| held.is_string()));
                    }
                    #single
                    ::serde_json::Value::Null => {}
                    _ => {
                        #not_strings
                    }
                }
                let settled = prompted(&self.transport, &self.next, #prompt, arguments).await;
                #settled
            }
        }
    }
}

/// One operation's client method: validate, write the arguments, send and read the answer.
fn method(operation: &OperationDef, generated: &Generated, schema: &Ident) -> TokenStream {
    if operation.is_mcp_prompt() {
        return prompt_method(operation, generated);
    }
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
    let blocks = operation.mcp.content.is_some();
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
                let settled = exchange(&self.transport, &self.next, #tool, arguments, #blocks).await;
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
    let (found, success_read) = if operation.is_mcp_prompt() {
        (
            quote!(found),
            prompt_read(success, tool, call_error, &quote!(found)),
        )
    } else if operation.mcp.content.is_some() {
        (
            quote!(found),
            content_read(operation, tool, call_error, &quote!(found)),
        )
    } else if success_carried.unit && is_unit_type(success) {
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

/// An expression that reads the messages of a prompt's result `source` into the success the
/// operation declared: the text of the first message, or every message.
fn prompt_read(
    success: &Type,
    tool: &str,
    call_error: &TokenStream,
    source: &TokenStream,
) -> TokenStream {
    let reader = if is_a_string(success) {
        quote! { first_text(#tool, &#source) }
    } else {
        quote! { read_messages(#tool, &#source["messages"]) }
    };
    quote! {
        match #reader {
            ::core::result::Result::Ok(read) => ::core::result::Result::Ok(read),
            ::core::result::Result::Err(reported) => ::core::result::Result::Err(#call_error::Fault(reported)),
        }
    }
}

/// An expression that reads the blocks of `source`, the `content` and `structuredContent` of the
/// answer, into the success the operation declared.
fn content_read(
    operation: &OperationDef,
    tool: &str,
    call_error: &TokenStream,
    source: &TokenStream,
) -> TokenStream {
    let refused = quote! { ::core::result::Result::Err(#call_error::Fault(reported)) };
    if super::content::is_pair(operation) {
        return quote! {
            match read_pair(#tool, #source) {
                ::core::result::Result::Ok(read) => ::core::result::Result::Ok(read),
                ::core::result::Result::Err(reported) => #refused,
            }
        };
    }
    quote! {
        match read_blocks(#tool, &#source["content"]) {
            ::core::result::Result::Ok(read) => ::core::result::Result::Ok(read),
            ::core::result::Result::Err(reported) => #refused,
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
