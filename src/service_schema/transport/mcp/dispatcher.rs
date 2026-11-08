//! The dispatcher half of the `mcp` transport: `serve` and `serve_http` around one `route` per
//! service, and the `answer` and `answer_http` of a service that stands alone.

use super::prompts;
use super::resources::{self, Resourced};
use super::{Carried, error_carried, success_carried, writer};
use super::{content, interview, listen, skills, tasks};
use crate::features::service_schema::registry_ident;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationOutcome, ServiceDef, argument_local, written,
};
use crate::service_schema::support::{message_reader_ident, message_validator_ident, module_ident};
use crate::service_schema::transport::amqp_rpc::{
    call_arguments, declares_a_reply, panic_guard, placement_doc,
};
use crate::service_schema::transport::{Transport, dispatcher_macro_ident};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

pub(super) fn macro_tokens(service: &ServiceDef, transport: Transport) -> TokenStream {
    let contract = &service.ident;
    let module = module_ident(service);
    let schema = registry_ident(service);
    let macro_name = dispatcher_macro_ident(service, transport);
    let placement = placement_doc(&macro_name, "mcp_transport", "the_contract_crate::");
    let macro_doc = format!(
        "The `{contract}` dispatcher for the `{}` transport, held as tokens rather than compiled \
         here.\n\n\
         It takes no arguments and emits bare items - `SERVER_NAME`, `INSTRUCTIONS`, `catalog`, \
         `route`, `McpEndpoint`, `serve`, `answer`, `McpHttpRequest`, `McpHttpResponse`, \
         `serve_http`, `answer_http`, `serve_listening`, `answer_listening`, `serve_http_listening`, \
         `answer_http_listening`, `McpChange`, `McpServer`, `McpCall`, `McpInFlight`, `McpEventWriter` and `McpHttpStream` - so the caller supplies the module they land in and two \
         transports in one crate cannot collide. The invoking crate names `serde`, `serde_json` \
         and `tracing` in its own manifest, because the items below call them. The service's \
         `{schema}` is reached as `$crate::{schema}`, so a service declared in a submodule \
         re-exports it at the crate root.\n\n\
         {placement}",
        transport.name()
    );
    let contract_name = contract.to_string();
    let constants = constants(&schema, &contract_name);
    let types = types();
    let endpoint_fns = endpoint_fns();
    let held = Resourced::of(service);
    let catalog = catalog(service, &schema, &held, skills::any(service));
    let route = route(service, &module, &schema, contract, &held);
    let listing = resources::listing_fns();
    let protocol = protocol();
    let answers = answers();
    let discovery = discovery();
    let serving = serving(&module);
    let routing = routing();
    let answer = answer(contract);
    let serve_stream = serve_fns();
    let answer_http = answer_http_fns(contract);
    let http_fns = http_fns();
    let http_rules = http_rules();
    let http_authorization = http_authorization();
    let http_entry = http_entry();
    let http_serving = http_serving();
    let writer_types = writer::types();
    let writer_functions = writer::functions();
    let listen_types = listen::types();
    let listen_functions = listen::functions();
    let listen_streaming = listen::streaming();
    quote! {
        #[doc = #macro_doc]
        #[macro_export]
        macro_rules! #macro_name {
            () => {
                #constants
                #types
                #writer_types
                #listen_types
                #catalog
                #route
                #endpoint_fns
                #protocol
                #listing
                #answers
                #discovery
                #serving
                #routing
                #serve_stream
                #answer
                #answer_http
                #http_fns
                #http_rules
                #http_authorization
                #http_entry
                #http_serving
                #writer_functions
                #listen_functions
                #listen_streaming
            };
        }
    }
}

fn catalog(
    service: &ServiceDef,
    schema: &Ident,
    held: &Resourced<'_>,
    skilled: bool,
) -> TokenStream {
    let parts = resources::catalog_parts(held, skilled);
    let extensions: Vec<TokenStream> = skilled
        .then(|| quote! { "io.modelcontextprotocol/skills": {} })
        .into_iter()
        .chain(tasks::extension(service))
        .collect();
    let capabilities = parts.capabilities;
    let prompt_capability = prompts::capability(service);
    let listed_prompts = prompts::catalog_prompts(service);
    let (resources, templates) = (parts.resources, parts.templates);
    quote! {
        /// Everything the list methods answer for this service, fixed at compile time.
        pub fn catalog() -> ::serde_json::Value {
            ::serde_json::json!({
                "capabilities": { "tools": {} #capabilities #prompt_capability },
                "extensions": { #(#extensions),* },
                "skills": $crate::#schema::mcp_skills(),
                "tools": $crate::#schema::mcp_tools(),
                #listed_prompts
                "resources": [#(#resources),*],
                "resourceTemplates": [#(#templates),*],
            })
        }
    }
}

fn route(
    service: &ServiceDef,
    module: &Ident,
    schema: &Ident,
    contract: &Ident,
    held: &Resourced<'_>,
) -> TokenStream {
    let tools: Vec<&OperationDef> = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
        .collect();
    let arms = tools
        .iter()
        .map(|operation| route_arm(module, schema, operation));
    let (implementation, context, guard) = if service.operations.is_empty() {
        (quote!(_), quote!(_), TokenStream::new())
    } else {
        (quote!(svc), quote!(ctx), panic_guard())
    };
    let replies = declares_a_reply(service)
        .then(|| replying_helpers(module))
        .into_iter();
    let tool_results = (!tools.is_empty()).then(|| tool_results(module));
    let asking = interview::any(service).then(interview::helpers);
    let blocks = (content::any(service) || content::prompt_messages(service))
        .then(|| content::helpers(service, module));
    let base64 = (resources::reads_bytes(held)
        || skills::reads_bytes(service)
        || content::any(service)
        || content::prompt_messages(service))
    .then(resources::base64_helper);
    let tool_call = (!tools.is_empty()).then(|| {
        quote! {
            "tools/call" => {
                let name = params.get("name")?.as_str()?;
                match name {
                    #(#arms)*
                    _ => ::core::option::Option::None,
                }
            }
        }
    });
    let resource_helpers = (held.any()
        || prompts::any(service)
        || tasks::any(service)
        || !skills::dynamic(service).is_empty())
    .then(|| resources::helpers(held));
    let task_helpers = tasks::any(service).then(tasks::helpers);
    let task_arms = tasks::arms(service, module);
    let skill_reads = skills::read_blocks(service, schema, module);
    let skill_get = skills::get_arm(service, schema);
    let reads = resources::read_arm(held, module, &skill_reads);
    let getting = prompts::get_arm(service, module);
    let listing = resources::list_arm(held, module);
    let completing = resources::complete_arm(held, service, module);
    quote! {
        #guard
        #(#replies)*
        #tool_results
        #asking
        #base64
        #blocks
        #resource_helpers
        #task_helpers

        /// One request this service owns: `tools/call`, `resources/read`, `prompts/get`, its own
        /// page of `resources/list` and `completion/complete`. `None` for a request it does not own,
        /// otherwise the `result` object of the answer (`Ok`) or its JSON-RPC `error` object
        /// (`Err`).
        pub async fn route<Svc, Ctx>(
            method: &str,
            params: &::serde_json::Value,
            #implementation: &Svc,
            #context: &Ctx,
        ) -> ::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>
        where
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Sync,
        {
            match method {
                #tool_call
                #reads
                #getting
                #task_arms
                #skill_get
                #listing
                #completing
                _ => ::core::option::Option::None,
            }
        }
    }
}

/// The result of a tool call, and a fault as the text of a tool execution error.
fn tool_results(module: &Ident) -> TokenStream {
    quote! {
        /// The result of a tool call: its structured value, or the text of its failure.
        fn completed(structured: ::core::option::Option<::serde_json::Value>, failure: ::core::option::Option<::std::string::String>) -> ::serde_json::Value {
            let (content, is_error) = match failure {
                ::core::option::Option::Some(text) => (::std::vec![::serde_json::json!({ "type": "text", "text": text })], true),
                ::core::option::Option::None => match &structured {
                    ::core::option::Option::Some(value) => (::std::vec![::serde_json::json!({ "type": "text", "text": canonical(value) })], false),
                    ::core::option::Option::None => (::std::vec::Vec::new(), false),
                },
            };
            let mut result = ::serde_json::json!({
                "resultType": "complete",
                "content": content,
                "isError": is_error,
            });
            if let ::core::option::Option::Some(value) = structured {
                result["structuredContent"] = value;
            }
            result
        }

        /// A fault as the text of a tool execution error.
        fn fault_result(fault: $crate::#module::ServiceFault) -> ::serde_json::Value {
            completed(
                ::core::option::Option::None,
                ::core::option::Option::Some(canonical(&::serde_json::json!({ "isServiceFault": true, "fault": fault }))),
            )
        }
    }
}

/// What only an operation that answers writes: a JSON-RPC internal error and the encoding of a
/// value, both ending in the `handler-panic` fault.
fn replying_helpers(module: &Ident) -> TokenStream {
    quote! {
        /// The JSON-RPC `-32603` error of a handler that came apart.
        fn internal_error(tool: &str, detail: &str) -> ::serde_json::Value {
            ::serde_json::json!({
                "code": -32603_i64,
                "message": format!("{tool}: {detail}"),
                "data": {
                    "isServiceFault": true,
                    "fault": $crate::#module::ServiceFault::handler_panic(tool, detail),
                },
            })
        }

        /// A value written as JSON, or the error of the call that could not write it.
        fn encoded<T>(tool: &str, value: &T) -> ::core::result::Result<::serde_json::Value, ::serde_json::Value>
        where
            T: ::serde::Serialize,
        {
            ::serde_json::to_value(value).map_err(|rejected| {
                internal_error(tool, &format!("the answer would not serialize: {rejected}"))
            })
        }
    }
}

fn route_arm(module: &Ident, schema: &Ident, operation: &OperationDef) -> TokenStream {
    let tool = operation.mcp_tool_name();
    let reader = message_reader_ident(operation);
    let validator = message_validator_ident(operation);
    let method = &operation.ident;
    let wrapped = format_ident!(
        "wrapped_{}",
        written(&operation.ident),
        span = operation.ident.span()
    );
    let bound = HttpShape::of(operation).header_in;
    let call = call_arguments(operation);
    let take_headers = bound.iter().enumerate().map(|(position, header)| {
        let raw = format_ident!("raw_header_{position}");
        let name = &header.name;
        quote! { let #raw = arguments.remove(#name); }
    });
    let read_headers = bound.iter().enumerate().map(|(position, header)| {
        let raw = format_ident!("raw_header_{position}");
        let held = argument_local(&header.parameter);
        let name = &header.name;
        quote! {
            let #held = match ::serde_json::from_value(#raw.unwrap_or(::serde_json::Value::Null)) {
                ::core::result::Result::Ok(decoded) => decoded,
                ::core::result::Result::Err(detail) => {
                    return ::core::option::Option::Some(::core::result::Result::Ok(fault_result(
                        $crate::#module::ServiceFault::failed_validation(
                            #tool,
                            ::core::option::Option::Some(#name),
                            &::std::string::ToString::to_string(&detail),
                        ),
                    )));
                }
            };
        }
    });
    let called = quote! { caught(move || svc.#method(ctx #(, #call)*)).await };
    let settled = settled(operation, schema, &called, tool);
    let prepared = interview::preparation(operation, module, schema, tool);
    quote! {
        #tool => {
            let mut arguments = match params.get("arguments") {
                ::core::option::Option::Some(::serde_json::Value::Object(held)) => held.clone(),
                ::core::option::Option::None | ::core::option::Option::Some(::serde_json::Value::Null) => ::serde_json::Map::new(),
                ::core::option::Option::Some(_) => {
                    return ::core::option::Option::Some(::core::result::Result::Ok(fault_result(
                        $crate::#module::ServiceFault::undeserializable_payload(
                            #tool,
                            "the arguments are not a JSON object",
                        ),
                    )));
                }
            };
            #(#take_headers)*
            #prepared
            let source = if $crate::#schema::#wrapped() {
                match arguments.remove("value") {
                    ::core::option::Option::Some(held) => held,
                    ::core::option::Option::None => {
                        return ::core::option::Option::Some(::core::result::Result::Ok(fault_result(
                            $crate::#module::ServiceFault::undeserializable_payload(
                                #tool,
                                "missing field `value`",
                            ),
                        )));
                    }
                }
            } else {
                ::serde_json::Value::Object(arguments)
            };
            let received = match $crate::#module::#reader(source) {
                ::core::result::Result::Ok(received) => received,
                ::core::result::Result::Err(rejected) => {
                    return ::core::option::Option::Some(::core::result::Result::Ok(fault_result(
                        $crate::#module::ServiceFault::undeserializable_payload(
                            #tool,
                            &::std::string::ToString::to_string(&rejected),
                        ),
                    )));
                }
            };
            if let ::core::result::Result::Err(violations) = $crate::#module::#validator(&received) {
                return ::core::option::Option::Some(::core::result::Result::Ok(fault_result(
                    $crate::#module::ServiceFault::failed_validation(
                        #tool,
                        $crate::#module::violated_field(&violations),
                        &$crate::#module::violation_detail(&violations),
                    ),
                )));
            }
            #(#read_headers)*
            #settled
        }
    }
}

/// What the handler's outcome becomes: the guard's `Err` is a panic, and an `Ok` is the outcome the
/// operation declared.
fn settled(
    operation: &OperationDef,
    schema: &Ident,
    called: &TokenStream,
    tool: &str,
) -> TokenStream {
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return quote! {
            if let ::core::result::Result::Err(panicked) = #called {
                record_panic(#tool, &panicked);
            }
            ::core::option::Option::Some(::core::result::Result::Ok(completed(
                ::core::option::Option::None,
                ::core::option::Option::None,
            )))
        };
    };
    let (success_pattern, ordinary_success) =
        carried_arm(&success_carried(operation, success), tool, false);
    let success_arm = if operation.mcp.task.is_some() {
        tasks::success_arm(tool)
    } else if operation.mcp.content.is_some() {
        content::success_arm(operation, &ordinary_success)
    } else {
        ordinary_success
    };
    let (error_pattern, ordinary_error) = carried_arm(&error_carried(operation, error), tool, true);
    let error_arm = interview::asking(operation, schema, tool, ordinary_error);
    quote! {
        match #called {
            ::core::result::Result::Err(panicked) => {
                record_panic(#tool, &panicked);
                ::core::option::Option::Some(::core::result::Result::Err(internal_error(#tool, &panicked)))
            }
            ::core::result::Result::Ok(::core::result::Result::Ok(#success_pattern)) => #success_arm,
            ::core::result::Result::Ok(::core::result::Result::Err(#error_pattern)) => #error_arm,
        }
    }
}

/// The pattern an outcome is destructured by, and the arm that answers it.
fn carried_arm(carried: &Carried, tool: &str, is_error: bool) -> (TokenStream, TokenStream) {
    let values: Vec<Ident> = (0..carried.value_types.len())
        .map(|position| format_ident!("carried_value_{position}"))
        .collect();
    let headers: Vec<Ident> = (0..carried.header_names.len())
        .map(|position| format_ident!("carried_header_{position}"))
        .collect();
    let finish = |body: TokenStream| {
        if is_error {
            quote! {
                ::core::option::Option::Some(::core::result::Result::Ok(completed(
                    ::core::option::Option::None,
                    ::core::option::Option::Some(canonical(&#body)),
                )))
            }
        } else {
            quote! {
                ::core::option::Option::Some(::core::result::Result::Ok(completed(
                    ::core::option::Option::Some(#body),
                    ::core::option::Option::None,
                )))
            }
        }
    };
    let value_ref = if values.len() == 1 {
        let only = &values[0];
        quote! { &#only }
    } else {
        quote! { &(#(#values),*) }
    };
    if carried.header_names.is_empty() {
        if carried.unit {
            return (
                quote! { _ },
                quote! {
                    ::core::option::Option::Some(::core::result::Result::Ok(completed(
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                    )))
                },
            );
        }
        let only = &values[0];
        let body = quote! { value };
        let answer = finish(body);
        return (
            quote! { #only },
            quote! {
                match encoded(#tool, &#only) {
                    ::core::result::Result::Ok(value) => #answer,
                    ::core::result::Result::Err(refused) => ::core::option::Option::Some(::core::result::Result::Err(refused)),
                }
            },
        );
    }
    let names = &carried.header_names;
    let answer = finish(quote! { ::serde_json::Value::Object(members) });
    (
        quote! { (#(#values),* , #(#headers),*) },
        quote! {
            {
                let mut members = ::serde_json::Map::new();
                match encoded(#tool, #value_ref) {
                    ::core::result::Result::Ok(value) => members.insert("value".to_owned(), value),
                    ::core::result::Result::Err(refused) => return ::core::option::Option::Some(::core::result::Result::Err(refused)),
                };
                #(
                    match encoded(#tool, &#headers) {
                        ::core::result::Result::Ok(value) => {
                            if !value.is_null() {
                                members.insert(#names.to_owned(), value);
                            }
                        }
                        ::core::result::Result::Err(refused) => return ::core::option::Option::Some(::core::result::Result::Err(refused)),
                    }
                )*
                #answer
            }
        },
    )
}

fn endpoint_type() -> TokenStream {
    quote! {
        /// The services of one endpoint, in order.
        pub struct McpEndpoint {
            catalogs: &'static [fn() -> ::serde_json::Value],
        }

        impl McpEndpoint {
            /// The `catalog` of each service, in order.
            pub const fn catalogs(&self) -> &'static [fn() -> ::serde_json::Value] {
                self.catalogs
            }

            /// Panics on a name two services share, so a `const` endpoint with one fails the build.
            pub const fn new(
                names: &'static [&'static [(&'static str, &'static str)]],
                catalogs: &'static [fn() -> ::serde_json::Value],
            ) -> Self {
                let mut earlier = 0;
                while earlier < names.len() {
                    let mut later = earlier + 1;
                    while later < names.len() {
                        let mut first = 0;
                        while first < names[earlier].len() {
                            let mut second = 0;
                            while second < names[later].len() {
                                let (kind, name) = names[earlier][first];
                                let (other_kind, other_name) = names[later][second];
                                if same_text(kind, other_kind) && same_text(name, other_name) {
                                    panic!("{}", name);
                                }
                                second += 1;
                            }
                            first += 1;
                        }
                        later += 1;
                    }
                    earlier += 1;
                }
                Self { catalogs }
            }
        }

    }
}

/// The comparison `McpEndpoint::new` makes of two names.
fn endpoint_fns() -> TokenStream {
    quote! {
        /// This service as the only one of its endpoint.
        const fn alone() -> McpServer<'static> {
            McpServer::new(&ALONE, INSTRUCTIONS, SERVER_NAME)
        }

        const fn same_text(left: &str, right: &str) -> bool {
            let (left, right) = (left.as_bytes(), right.as_bytes());
            if left.len() != right.len() {
                return false;
            }
            let mut at = 0;
            while at < left.len() {
                if left[at] != right[at] {
                    return false;
                }
                at += 1;
            }
            true
        }
    }
}

/// `serve`, and the pieces of the protocol it answers with.
fn protocol() -> TokenStream {
    let canonical = super::canonical_json();
    quote! {
        #canonical

        /// A JSON-RPC error response; a `null` id is left out, which the official schema requires.
        fn failed(id: &::serde_json::Value, error: ::serde_json::Value) -> ::serde_json::Value {
            if id.is_null() {
                ::serde_json::json!({ "jsonrpc": "2.0", "error": error })
            } else {
                ::serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": error })
            }
        }

        /// The error object of a JSON-RPC error response.
        fn error_object(code: i64, message: &str, data: ::core::option::Option<::serde_json::Value>) -> ::serde_json::Value {
            let mut error = ::serde_json::json!({ "code": code, "message": message });
            if let ::core::option::Option::Some(data) = data {
                error["data"] = data;
            }
            error
        }

        /// The message in `text`, or the `-32600` response that refuses it.
        fn read_request(text: &str) -> ::core::result::Result<::serde_json::Value, ::serde_json::Value> {
            let invalid = |detail: &str| {
                failed(
                    &::serde_json::Value::Null,
                    error_object(-32600, &format!("Invalid request: {detail}"), ::core::option::Option::None),
                )
            };
            let request: ::serde_json::Value =
                ::serde_json::from_str(text).map_err(|rejected| invalid(&rejected.to_string()))?;
            if !request.is_object() {
                return ::core::result::Result::Err(invalid("not a JSON object"));
            }
            if request["jsonrpc"] != "2.0" {
                return ::core::result::Result::Err(invalid("`jsonrpc` is not \"2.0\""));
            }
            if !request["method"].is_string() {
                return ::core::result::Result::Err(invalid("no `method`"));
            }
            if let ::core::option::Option::Some(id) = request.get("id") {
                if !(id.is_string() || id.is_number()) {
                    return ::core::result::Result::Err(invalid("`id` is neither a string nor a number"));
                }
            }
            ::core::result::Result::Ok(request)
        }

        /// The `-32602` or `-32022` response of a request that names no protocol version this
        /// server answers.
        fn checked(request: &::serde_json::Value) -> ::core::result::Result<(), ::serde_json::Value> {
            let id = &request["id"];
            let meta = &request["params"]["_meta"];
            for key in [PROTOCOL_KEY, CAPABILITIES_KEY] {
                if meta.get(key).is_none() {
                    return ::core::result::Result::Err(failed(
                        id,
                        error_object(-32602, &format!("Missing required _meta key: {key}"), ::core::option::Option::None),
                    ));
                }
            }
            let requested = &meta[PROTOCOL_KEY];
            if *requested != PROTOCOL_VERSION {
                return ::core::result::Result::Err(failed(
                    id,
                    error_object(
                        -32022,
                        "Unsupported protocol version",
                        ::core::option::Option::Some(::serde_json::json!({
                            "supported": [PROTOCOL_VERSION],
                            "requested": requested,
                        })),
                    ),
                ));
            }
            ::core::result::Result::Ok(())
        }

    }
}

/// How a message is answered by `serve` itself: the capabilities, the list of tools and the name
/// of the server beside each result.
fn answers() -> TokenStream {
    quote! {
        /// What a routed request asks for: the tool, the URI, or the URI of the template a
        /// completion is for.
        fn named<'params>(params: &'params ::serde_json::Value, method: &str) -> ::core::option::Option<&'params str> {
            match method {
                "tools/call" => params["name"].as_str(),
                "resources/read" => params["uri"].as_str(),
                "prompts/get" => params["name"].as_str(),
                "skills/get" => params["name"].as_str().or(params["uri"].as_str()),
                "tasks/get" | "tasks/cancel" | "tasks/update" => params["taskId"].as_str(),
                _ => params["argument"]["name"].as_str().and(
                    if params["ref"]["type"] == "ref/prompt" {
                        params["ref"]["name"].as_str()
                    } else {
                        params["ref"]["uri"].as_str()
                    },
                ),
            }
        }

        /// The text of the response to `id`: the result with the server's name beside it, or the
        /// error.
        fn response_text(
            id: &::serde_json::Value,
            answered: ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
            server_name: &str,
        ) -> ::std::string::String {
            canonical(&match answered {
                ::core::result::Result::Ok(result) => ::serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": with_server_info(result, server_name),
                }),
                ::core::result::Result::Err(error) => failed(id, error),
            })
        }

        fn with_server_info(mut result: ::serde_json::Value, server_name: &str) -> ::serde_json::Value {
            if let ::serde_json::Value::Object(members) = &mut result {
                members
                    .entry("resultType")
                    .or_insert_with(|| ::serde_json::Value::String("complete".to_owned()));
                let meta = members
                    .entry("_meta")
                    .or_insert_with(|| ::serde_json::Value::Object(::serde_json::Map::new()));
                if let ::serde_json::Value::Object(meta) = meta {
                    meta.insert(
                        SERVER_INFO_KEY.to_owned(),
                        ::serde_json::json!({ "name": server_name, "version": env!("CARGO_PKG_VERSION") }),
                    );
                }
            }
            result
        }
    }
}

/// What `server/discover` and the list methods answer from the catalogs of an endpoint.
fn discovery() -> TokenStream {
    quote! {
        /// Whether any service of the endpoint offers the tasks extension.
        fn tasked(endpoint: &McpEndpoint) -> bool {
            endpoint
                .catalogs
                .iter()
                .any(|catalog| catalog()["extensions"].get("io.modelcontextprotocol/tasks").is_some())
        }

        fn discovered(instructions: ::core::option::Option<&str>, endpoint: &McpEndpoint, listens: bool) -> ::serde_json::Value {
            let mut capabilities = ::serde_json::Map::new();
            let mut extensions = ::serde_json::Map::new();
            for catalog in endpoint.catalogs() {
                let catalog = catalog();
                for (held, into) in [("capabilities", &mut capabilities), ("extensions", &mut extensions)] {
                    if let ::core::option::Option::Some(::serde_json::Value::Object(offered)) = catalog.get(held) {
                        for (key, value) in offered {
                            into.entry(key.clone()).or_insert_with(|| value.clone());
                        }
                    }
                }
            }
            if listens {
                if let ::core::option::Option::Some(::serde_json::Value::Object(resources)) = capabilities.get_mut("resources") {
                    resources.insert("listChanged".to_owned(), ::serde_json::Value::Bool(true));
                    resources.insert("subscribe".to_owned(), ::serde_json::Value::Bool(true));
                }
            }
            if !extensions.is_empty() {
                capabilities.insert("extensions".to_owned(), ::serde_json::Value::Object(extensions));
            }
            let mut result = ::serde_json::json!({
                "resultType": "complete",
                "supportedVersions": [PROTOCOL_VERSION],
                "capabilities": capabilities,
                "ttlMs": LISTED_FOR_MS,
                "cacheScope": "public",
            });
            if let ::core::option::Option::Some(text) = instructions {
                result["instructions"] = ::serde_json::Value::String(text.to_owned());
            }
            result
        }

        /// What the catalogs hold under `key`, one after the other, as a list result.
        fn listed(endpoint: &McpEndpoint, key: &str) -> ::serde_json::Value {
            let mut items = ::std::vec::Vec::new();
            for catalog in endpoint.catalogs() {
                if let ::core::option::Option::Some(::serde_json::Value::Array(offered)) = catalog().get(key) {
                    items.extend(offered.iter().cloned());
                }
            }
            let dynamic = items.iter().any(|item| item["resources"] == "dynamic");
            ::serde_json::json!({
                "resultType": "complete",
                key: items,
                "ttlMs": if dynamic { 0_u64 } else { LISTED_FOR_MS },
                "cacheScope": if dynamic { "private" } else { "public" },
            })
        }
    }
}

/// `serve`, and what it does ahead of asking a service.
fn serving(module: &Ident) -> TokenStream {
    quote! {
        /// Everything `serve` does ahead of asking a service.
        fn begun(text: &str, instructions: ::core::option::Option<&str>, endpoint: &McpEndpoint, server_name: &str, listens: bool) -> Begun {
            let request = match read_request(text) {
                ::core::result::Result::Ok(request) => request,
                ::core::result::Result::Err(refused) => return Begun::Answered(::core::option::Option::Some(canonical(&refused))),
            };
            let ::core::option::Option::Some(id) = request.get("id") else {
                return Begun::Notified(request);
            };
            if let ::core::result::Result::Err(refused) = checked(&request) {
                return Begun::Answered(::core::option::Option::Some(canonical(&refused)));
            }
            let answered = match request["method"].as_str().unwrap_or_default() {
                "server/discover" => ::core::result::Result::Ok(discovered(instructions, endpoint, listens)),
                "tools/list" => ::core::result::Result::Ok(listed(endpoint, "tools")),
                "resources/templates/list" => {
                    ::core::result::Result::Ok(listed(endpoint, "resourceTemplates"))
                }
                "prompts/list" => ::core::result::Result::Ok(listed(endpoint, "prompts")),
                "skills/list" => ::core::result::Result::Ok(listed(endpoint, "skills")),
                "resources/list" => return Begun::Routed(request),
                "subscriptions/listen" if listens => return Begun::Routed(request),
                method @ ("tasks/get" | "tasks/cancel" | "tasks/update") if tasked(endpoint) => {
                    if named(&request["params"], method).is_some() {
                        return Begun::Routed(request);
                    }
                    ::core::result::Result::Err(error_object(
                        -32602,
                        &format!("Invalid params: {method} names nothing to answer"),
                        ::core::option::Option::None,
                    ))
                }
                method @ ("tools/call" | "resources/read" | "prompts/get" | "completion/complete"
                | "skills/get") => {
                    if named(&request["params"], method).is_some() {
                        return Begun::Routed(request);
                    }
                    ::core::result::Result::Err(error_object(
                        -32602,
                        &format!("Invalid params: {method} names nothing to answer"),
                        ::core::option::Option::None,
                    ))
                }
                other => ::core::result::Result::Err(error_object(
                    -32601,
                    &format!("Method not found: {other}"),
                    ::core::option::Option::None,
                )),
            };
            Begun::Answered(::core::option::Option::Some(response_text(id, answered, server_name)))
        }

        /// What a routed request answers once its services have been asked.
        fn routed_text(
            request: &::serde_json::Value,
            answered: ::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>,
            server_name: &str,
        ) -> ::core::option::Option<::std::string::String> {
            let method = request["method"].as_str().unwrap_or_default();
            let name = named(&request["params"], method).unwrap_or_default();
            let answered = answered.unwrap_or_else(|| {
                let unknown = match method {
                    "tools/call" => "Unknown tool",
                    "resources/read" => "Resource not found",
                    "prompts/get" => "Unknown prompt",
                    "skills/get" => "Skill not found",
                    "tasks/get" | "tasks/cancel" | "tasks/update" => "Unknown task",
                    _ => "Unknown completion",
                };
                ::core::result::Result::Err(error_object(
                    -32602,
                    &format!("{unknown}: {name}"),
                    ::core::option::Option::Some(::serde_json::json!({
                        "isServiceFault": true,
                        "fault": $crate::#module::ServiceFault::unknown_operation(name),
                    })),
                ))
            });
            ::core::option::Option::Some(response_text(&request["id"], answered, server_name))
        }
    }
}

/// The type of the `route` that `serve` and `serve_http` take.
fn route_type() -> TokenStream {
    quote! {
        &(dyn for<'request> ::core::ops::Fn(&'request Ctx, usize, &'request str, &'request ::serde_json::Value) -> ::core::pin::Pin<
            ::std::boxed::Box<dyn ::core::future::Future<Output = ::core::option::Option<
                ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
            >> + ::core::marker::Send + 'request>,
        > + ::core::marker::Sync)
    }
}

/// `serve`, and what it does around the services it asks.
fn routing() -> TokenStream {
    quote! {
        /// What a message needs before a service is asked: `Ok` with the request that `route`
        /// is to answer, or `Err` with the answer it has already. A `notifications/cancelled`
        /// marks the request it names in `in_flight`.
        fn started(
            text: &str,
            server: &McpServer<'_>,
            in_flight: ::core::option::Option<&McpInFlight>,
            listens: bool,
        ) -> ::core::result::Result<::serde_json::Value, ::core::option::Option<::std::string::String>> {
            match begun(text, server.instructions(), server.endpoint(), server.name(), listens) {
                Begun::Answered(answered) => ::core::result::Result::Err(answered),
                Begun::Notified(notice) => {
                    if let ::core::option::Option::Some(table) = in_flight {
                        if notice["method"] == "notifications/cancelled" {
                            table.cancel(&notice["params"]["requestId"]);
                        }
                    }
                    ::core::result::Result::Err(::core::option::Option::None)
                }
                Begun::Routed(request) => ::core::result::Result::Ok(request),
            }
        }

        /// Asks the services of the endpoint for what the routed request wants. A `resources/list`
        /// pages across them, and any other method goes to the first service that answers.
        async fn put_to_services<Ctx>(
            request: &::serde_json::Value,
            endpoint: &McpEndpoint,
            ctx: &Ctx,
            route: &(dyn for<'request> ::core::ops::Fn(&'request Ctx, usize, &'request str, &'request ::serde_json::Value) -> ::core::pin::Pin<
                ::std::boxed::Box<dyn ::core::future::Future<Output = ::core::option::Option<
                    ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
                >> + ::core::marker::Send + 'request>,
            > + ::core::marker::Sync),
        ) -> ::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>
        where
            Ctx: ::core::marker::Sync,
        {
            let method = request["method"].as_str().unwrap_or_default();
            if method == "resources/list" {
                let (start, own, first) = match listing_start(request, endpoint.catalogs().len()) {
                    ::core::result::Result::Ok(start) => start,
                    ::core::result::Result::Err(refused) => return ::core::option::Option::Some(::core::result::Result::Err(refused)),
                };
                let mut page = ::core::option::Option::None;
                for part in start..endpoint.catalogs().len() {
                    let params = listing_params(if part == start { own.as_ref() } else { ::core::option::Option::None });
                    if let ::core::option::Option::Some(answered) = route(ctx, part, "resources/list", &params).await {
                        page = ::core::option::Option::Some((part, answered));
                        break;
                    }
                }
                return ::core::option::Option::Some(listed_resources(first, endpoint, page));
            }
            for part in 0..endpoint.catalogs().len() {
                let answered = route(ctx, part, method, &request["params"]).await;
                if answered.is_some() {
                    return answered;
                }
            }
            ::core::option::Option::None
        }

        /// The answer to a routed request once its services have been asked: dropped where the
        /// call was cancelled.
        fn finished(
            request: &::serde_json::Value,
            call: &McpCall,
            answered: ::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>,
            server_name: &str,
        ) -> ::core::option::Option<::std::string::String> {
            if call.cancelled() {
                return ::core::option::Option::None;
            }
            routed_text(request, answered, server_name)
        }

    }
}

/// `serve` over a stream of lines.
fn serve_fns() -> TokenStream {
    let route_type = route_type();
    quote! {
        /// One JSON-RPC message in, one out, for the services of the endpoint handed in. `route`
        /// answers for the service at that place in the endpoint, and receives the `Ctx` that
        /// `ctx` built from the request in flight. A notification answers `None`, and
        /// `notifications/cancelled` marks the request it names in `in_flight`. `route` boxes a
        /// `Send` future that owns what it routes to, so the future of `serve` can be spawned.
        pub async fn serve<Ctx, W, MakeCtx>(
            text: &str,
            server_name: &str,
            instructions: ::core::option::Option<&str>,
            endpoint: &McpEndpoint,
            ctx: MakeCtx,
            host: McpHost<'_, W>,
            route: #route_type,
        ) -> ::core::option::Option<::std::string::String>
        where
            MakeCtx: ::core::ops::FnOnce(McpCall) -> Ctx + ::core::marker::Send,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            W: McpEventWriter + ::core::marker::Sync,
        {
            let server = McpServer::new(endpoint, instructions, server_name);
            serve_core(text, &server, ctx, host, route, &mut idle, false).await
        }

        /// `serve` for an application that hands in its source of changes: `subscriptions/listen`
        /// is answered, and `server/discover` says so.
        pub async fn serve_listening<Ctx, W, Changes, MakeCtx>(
            text: &str,
            server: &McpServer<'_>,
            ctx: MakeCtx,
            host: McpHost<'_, W>,
            route: #route_type,
            changes: &mut Changes,
        ) -> ::core::option::Option<::std::string::String>
        where
            MakeCtx: ::core::ops::FnOnce(McpCall) -> Ctx + ::core::marker::Send,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            W: McpEventWriter + ::core::marker::Sync,
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            serve_core(text, server, ctx, host, route, changes, true).await
        }

        async fn serve_core<Ctx, W, Changes, MakeCtx>(
            text: &str,
            server: &McpServer<'_>,
            ctx: MakeCtx,
            host: McpHost<'_, W>,
            route: #route_type,
            changes: &mut Changes,
            listens: bool,
        ) -> ::core::option::Option<::std::string::String>
        where
            MakeCtx: ::core::ops::FnOnce(McpCall) -> Ctx + ::core::marker::Send,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            W: McpEventWriter + ::core::marker::Sync,
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            let request = match started(text, server, ::core::option::Option::Some(host.in_flight()), listens) {
                ::core::result::Result::Ok(request) => request,
                ::core::result::Result::Err(answered) => return answered,
            };
            let call = McpCall::of(&request);
            let ctx = ctx(call.clone());
            let _entered = host.in_flight().enter(&request["id"], &call);
            let answered = if request["method"] == "subscriptions/listen" {
                Driven::new(&call, host.writer(), listening(&request, &call, server.endpoint(), changes)).await
            } else {
                Driven::new(&call, host.writer(), put_to_services(&request, server.endpoint(), &ctx, route)).await
            };
            finished(&request, &call, answered, server.name())
        }
    }
}

fn answer(contract: &Ident) -> TokenStream {
    quote! {
        /// `serve` for this service alone, with `SERVER_NAME`, `INSTRUCTIONS`, `catalog()` and
        /// `route`. Usable as is for stdio, and its future is `Send` wherever `Svc`, `Ctx` and
        /// `writer` allow it.
        pub async fn answer<Svc, Ctx, W, MakeCtx>(
            text: &str,
            svc: &Svc,
            ctx: MakeCtx,
            writer: &W,
            in_flight: &McpInFlight,
        ) -> ::core::option::Option<::std::string::String>
        where
            MakeCtx: ::core::ops::FnOnce(McpCall) -> Ctx + ::core::marker::Send,
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            W: McpEventWriter + ::core::marker::Sync,
        {
            answer_core(text, svc, ctx, writer, in_flight, &mut idle, false).await
        }

        /// `answer` for an application that hands in its source of changes: `subscriptions/listen`
        /// is answered, and `server/discover` says so.
        pub async fn answer_listening<Svc, Ctx, W, Changes, MakeCtx>(
            text: &str,
            svc: &Svc,
            ctx: MakeCtx,
            writer: &W,
            in_flight: &McpInFlight,
            changes: &mut Changes,
        ) -> ::core::option::Option<::std::string::String>
        where
            MakeCtx: ::core::ops::FnOnce(McpCall) -> Ctx + ::core::marker::Send,
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            W: McpEventWriter + ::core::marker::Sync,
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            answer_core(text, svc, ctx, writer, in_flight, changes, true).await
        }

        async fn answer_core<Svc, Ctx, W, Changes, MakeCtx>(
            text: &str,
            svc: &Svc,
            ctx: MakeCtx,
            writer: &W,
            in_flight: &McpInFlight,
            changes: &mut Changes,
            listens: bool,
        ) -> ::core::option::Option<::std::string::String>
        where
            MakeCtx: ::core::ops::FnOnce(McpCall) -> Ctx + ::core::marker::Send,
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            W: McpEventWriter + ::core::marker::Sync,
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            let request = match started(text, &alone(), ::core::option::Option::Some(in_flight), listens) {
                ::core::result::Result::Ok(request) => request,
                ::core::result::Result::Err(answered) => return answered,
            };
            let call = McpCall::of(&request);
            let ctx = ctx(call.clone());
            let _entered = in_flight.enter(&request["id"], &call);
            let answered = if request["method"] == "subscriptions/listen" {
                Driven::new(&call, writer, listening(&request, &call, &ALONE, changes)).await
            } else {
                Driven::new(&call, writer, put_to_service(&request, svc, &ctx)).await
            };
            finished(&request, &call, answered, SERVER_NAME)
        }

        /// `put_to_services` for this service alone.
        async fn put_to_service<Svc, Ctx>(
            request: &::serde_json::Value,
            svc: &Svc,
            ctx: &Ctx,
        ) -> ::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>
        where
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
        {
            let method = request["method"].as_str().unwrap_or_default();
            if method == "resources/list" {
                let (_start, own, first) = match listing_start(request, ALONE.catalogs().len()) {
                    ::core::result::Result::Ok(start) => start,
                    ::core::result::Result::Err(refused) => return ::core::option::Option::Some(::core::result::Result::Err(refused)),
                };
                let page = route("resources/list", &listing_params(own.as_ref()), svc, ctx)
                    .await
                    .map(|page| (0, page));
                return ::core::option::Option::Some(listed_resources(first, &ALONE, page));
            }
            route(method, &request["params"], svc, ctx).await
        }

    }
}

/// `answer_http` and its core.
fn answer_http_fns(contract: &Ident) -> TokenStream {
    quote! {
        /// `serve_http` for this service alone.
        pub async fn answer_http<Svc, Ctx, Admit, Stream>(
            request: &McpHttpRequest,
            settings: &McpHttpSettings<'_>,
            stream: &Stream,
            svc: &Svc,
            admit: Admit,
        ) -> ::core::option::Option<McpHttpResponse>
        where
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            Admit: ::core::ops::AsyncFn(&McpHttpRequest, McpCall) -> ::core::result::Result<Ctx, McpDenied>,
            Stream: McpHttpStream + ::core::marker::Sync,
        {
            answer_http_core(&McpHttpAsk { request, settings, stream }, svc, admit, &mut idle, false).await
        }

        /// `answer_http` for an application that hands in its source of changes. A
        /// `subscriptions/listen` answers an event stream that ends when `changes` yields `None`.
        pub async fn answer_http_listening<Svc, Ctx, Admit, Stream, Changes>(
            request: &McpHttpRequest,
            settings: &McpHttpSettings<'_>,
            stream: &Stream,
            svc: &Svc,
            admit: Admit,
            changes: &mut Changes,
        ) -> ::core::option::Option<McpHttpResponse>
        where
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            Admit: ::core::ops::AsyncFn(&McpHttpRequest, McpCall) -> ::core::result::Result<Ctx, McpDenied>,
            Stream: McpHttpStream + ::core::marker::Sync,
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            answer_http_core(&McpHttpAsk { request, settings, stream }, svc, admit, changes, true).await
        }

        async fn answer_http_core<Svc, Ctx, Admit, Stream, Changes>(
            ask: &McpHttpAsk<'_, Stream>,
            svc: &Svc,
            admit: Admit,
            changes: &mut Changes,
            listens: bool,
        ) -> ::core::option::Option<McpHttpResponse>
        where
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            Admit: ::core::ops::AsyncFn(&McpHttpRequest, McpCall) -> ::core::result::Result<Ctx, McpDenied>,
            Stream: McpHttpStream + ::core::marker::Sync,
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            let McpHttpAsk { request, settings, stream } = *ask;
            if let ::core::result::Result::Err(refused) = before_admit(request, settings) {
                return ::core::option::Option::Some(refused);
            }
            let call = McpCall::of_body(request);
            let ctx = match admit(request, call.clone()).await {
                ::core::result::Result::Ok(ctx) => ctx,
                ::core::result::Result::Err(denied) => return ::core::option::Option::Some(denial_response(&denied, settings, request)),
            };
            match admitted(request, listens, &ALONE) {
                ::core::result::Result::Err(refused) => ::core::option::Option::Some(refused),
                ::core::result::Result::Ok(text) => {
                    match started(text, &alone(), ::core::option::Option::None, listens) {
                        ::core::result::Result::Err(answered) => ::core::option::Option::Some(http_response(answered)),
                        ::core::result::Result::Ok(asked_request) => {
                            let work = async {
                                if asked_request["method"] == "subscriptions/listen" {
                                    listening(&asked_request, &call, &ALONE, changes).await
                                } else {
                                    put_to_service(&asked_request, svc, &ctx).await
                                }
                            };
                            http_answer(&asked_request, &call, stream, SERVER_NAME, work).await
                        }
                    }
                }
            }
        }
    }
}

fn http_fns() -> TokenStream {
    quote! {
        fn without_body(status: u16) -> McpHttpResponse {
            McpHttpResponse::new(status, ::std::vec::Vec::new(), ::std::vec::Vec::new())
        }

        fn json_response(status: u16, body: &::serde_json::Value) -> McpHttpResponse {
            McpHttpResponse::new(
                status,
                ::std::vec![("Content-Type".to_owned(), "application/json".to_owned())],
                canonical(body).into_bytes(),
            )
        }
    }
}

/// The server an endpoint answers as, and what an HTTP server of it is configured with.
fn server_and_settings_types() -> TokenStream {
    quote! {
        /// The server the services of an endpoint answer as.
        #[derive(Clone, Copy)]
        pub struct McpServer<'server> {
            endpoint: &'server McpEndpoint,
            instructions: ::core::option::Option<&'server str>,
            name: &'server str,
        }

        impl<'server> McpServer<'server> {
            /// The endpoint it answers for.
            pub const fn endpoint(&self) -> &'server McpEndpoint {
                self.endpoint
            }

            /// The text reported as `instructions`, where there is one.
            pub const fn instructions(&self) -> ::core::option::Option<&'server str> {
                self.instructions
            }

            /// The name reported as `serverInfo.name`.
            pub const fn name(&self) -> &'server str {
                self.name
            }

            /// Binds a server to its endpoint, instructions and name.
            pub const fn new(
                endpoint: &'server McpEndpoint,
                instructions: ::core::option::Option<&'server str>,
                name: &'server str,
            ) -> Self {
                Self { endpoint, instructions, name }
            }
        }

        /// An HTTP request in plain terms, the way `http_rest` publishes its own.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct McpHttpRequest {
            body: ::std::vec::Vec<u8>,
            headers: ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
            method: ::std::string::String,
            path: ::std::string::String,
        }

        /// What an HTTP server of this endpoint is configured with.
        #[derive(Clone, Copy, Debug)]
        pub struct McpHttpSettings<'settings> {
            allowed_origins: &'settings [&'settings str],
            oauth: ::core::option::Option<McpOAuth<'settings>>,
        }

        impl<'settings> McpHttpSettings<'settings> {
            /// The origins a request may come from.
            pub const fn allowed_origins(&self) -> &'settings [&'settings str] {
                self.allowed_origins
            }

            /// Binds the origins a request may come from and the OAuth metadata. `oauth` is `None`
            /// where the server authorizes by other means: no metadata document and no challenge.
            pub const fn new(
                allowed_origins: &'settings [&'settings str],
                oauth: ::core::option::Option<McpOAuth<'settings>>,
            ) -> Self {
                Self { allowed_origins, oauth }
            }

            /// The OAuth metadata, where the server authorizes by OAuth.
            pub const fn oauth(&self) -> ::core::option::Option<McpOAuth<'settings>> {
                self.oauth
            }
        }

        /// The protected-resource metadata of a server that authorizes by OAuth.
        #[derive(Clone, Copy, Debug)]
        pub struct McpOAuth<'oauth> {
            authorization_servers: &'oauth [&'oauth str],
            resource: &'oauth str,
            resource_metadata_url: &'oauth str,
            scopes_supported: &'oauth [&'oauth str],
        }

        impl<'oauth> McpOAuth<'oauth> {
            /// The authorization servers the resource trusts.
            pub const fn authorization_servers(&self) -> &'oauth [&'oauth str] {
                self.authorization_servers
            }

            /// Binds the metadata a protected resource publishes.
            pub const fn new(
                authorization_servers: &'oauth [&'oauth str],
                resource: &'oauth str,
                resource_metadata_url: &'oauth str,
                scopes_supported: &'oauth [&'oauth str],
            ) -> Self {
                Self { authorization_servers, resource, resource_metadata_url, scopes_supported }
            }

            /// The identifier of the protected resource.
            pub const fn resource(&self) -> &'oauth str {
                self.resource
            }

            /// Where the metadata document is served.
            pub const fn resource_metadata_url(&self) -> &'oauth str {
                self.resource_metadata_url
            }

            /// The scopes the resource understands.
            pub const fn scopes_supported(&self) -> &'oauth [&'oauth str] {
                self.scopes_supported
            }
        }
    }
}

/// The enum and structs the dispatcher publishes or reads, with their impls.
fn types() -> TokenStream {
    let endpoint = endpoint_type();
    let settings = server_and_settings_types();
    quote! {
        #endpoint

        /// What a message needs before a service is asked: its answer, the notification it is, or
        /// the request that `route` is to answer.
        enum Begun {
            Answered(::core::option::Option<::std::string::String>),
            Notified(::serde_json::Value),
            Routed(::serde_json::Value),
        }

        #settings

        /// Why `admit` refused a request.
        #[derive(Clone, Debug, Eq, PartialEq)]
        #[non_exhaustive]
        pub enum McpDenied {
            /// `401`: no credentials, or credentials that do not hold. `scope` names what the
            /// called operation needs, where it is known.
            NoValidCredentials { scope: ::core::option::Option<::std::string::String> },
            /// `403` with a challenge: the credentials hold, but not the scopes `scope` names.
            InsufficientScope { scope: ::std::string::String },
            /// `403` with no challenge.
            Forbidden,
        }

        /// An HTTP response in plain terms.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct McpHttpResponse {
            body: ::std::vec::Vec<u8>,
            headers: ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
            status: u16,
        }

        impl McpHttpResponse {
            /// The response body.
            pub fn body(&self) -> &[u8] {
                &self.body
            }

            /// Every header this response carries, in the order they were written.
            pub fn headers(&self) -> &[(::std::string::String, ::std::string::String)] {
                &self.headers
            }

            /// Binds one response, for an adapter to write back however its own framework answers.
            pub const fn new(
                status: u16,
                headers: ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
                body: ::std::vec::Vec<u8>,
            ) -> Self {
                Self { body, headers, status }
            }

            /// The status code.
            pub const fn status(&self) -> u16 {
                self.status
            }
        }

        impl McpHttpRequest {
            /// The token of an `Authorization: Bearer` header, whatever the case of the scheme.
            pub fn bearer_token(&self) -> ::core::option::Option<&str> {
                let value = self.header("authorization")?;
                let (scheme, token) = value.split_once(' ')?;
                let token = token.trim();
                (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
            }

            /// The request body, undecoded.
            pub fn body(&self) -> &[u8] {
                &self.body
            }

            /// The value of the header called `name`, whatever its case.
            pub fn header(&self, name: &str) -> ::core::option::Option<&str> {
                self.headers()
                    .iter()
                    .find(|(held, _)| held.eq_ignore_ascii_case(name))
                    .map(|(_, value)| value.as_str())
            }

            /// Every header this request carried, in the order it carried them.
            pub fn headers(&self) -> &[(::std::string::String, ::std::string::String)] {
                &self.headers
            }

            /// The `Mcp-Name` header, decoded from `=?base64?...?=` where it is written so.
            pub fn mcp_name(&self) -> ::core::option::Option<::std::string::String> {
                decoded_header(self.header("mcp-name")?)
            }

            /// The method exactly as the request line carried it.
            pub fn method(&self) -> &str {
                &self.method
            }

            /// Binds one request as an adapter read it off the wire. `path` is the request's path,
            /// with or without its query.
            pub const fn new(
                method: ::std::string::String,
                path: ::std::string::String,
                headers: ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
                body: ::std::vec::Vec<u8>,
            ) -> Self {
                Self { body, headers, method, path }
            }

            /// The path, with or without its query.
            pub fn path(&self) -> &str {
                &self.path
            }
        }
    }
}

/// The names the dispatcher reads.
fn constants(schema: &Ident, contract_name: &str) -> TokenStream {
    quote! {
        /// The trait's rustdoc, answered as `instructions` by `server/discover`.
        pub const INSTRUCTIONS: ::core::option::Option<&str> = $crate::#schema::MCP_INSTRUCTIONS;

        /// The trait's own name, reported as `serverInfo.name`.
        pub const SERVER_NAME: &str = #contract_name;

        const ALONE: McpEndpoint = McpEndpoint::new(&[$crate::#schema::MCP_NAMES], &[catalog]);
        const CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
        const LISTED_FOR_MS: u64 = 300_000;
        /// The methods `serve` answers itself or asks `route` for.
        const METHODS: [&str; 11] = [
            "completion/complete",
            "prompts/get",
            "prompts/list",
            "resources/list",
            "resources/read",
            "resources/templates/list",
            "server/discover",
            "skills/get",
            "skills/list",
            "tools/call",
            "tools/list",
        ];
        const PROTOCOL_KEY: &str = "io.modelcontextprotocol/protocolVersion";
        const PROTOCOL_VERSION: &str = "2026-07-28";
        const SERVER_INFO_KEY: &str = "io.modelcontextprotocol/serverInfo";
        const SUBSCRIPTION_KEY: &str = "io.modelcontextprotocol/subscriptionId";
    }
}

/// The checks of a request's headers, and the reading of one.
fn http_rules() -> TokenStream {
    quote! {
        /// The text a header holds, decoded from `=?base64?...?=` where it is written so.
        fn decoded_header(value: &str) -> ::core::option::Option<::std::string::String> {
            let ::core::option::Option::Some(encoded) = value
                .strip_prefix("=?base64?")
                .and_then(|rest| rest.strip_suffix("?="))
            else {
                return ::core::option::Option::Some(value.to_owned());
            };
            let mut bytes = ::std::vec::Vec::new();
            let (mut bits, mut held) = (0_u32, 0_u32);
            for letter in encoded.trim_end_matches('=').bytes() {
                let sextet = match letter {
                    b'A'..=b'Z' => letter - b'A',
                    b'a'..=b'z' => letter - b'a' + 26,
                    b'0'..=b'9' => letter - b'0' + 52,
                    b'+' => 62,
                    b'/' => 63,
                    _ => return ::core::option::Option::None,
                };
                bits = (bits << 6_u32) | u32::from(sextet);
                held += 6;
                if held >= 8 {
                    held -= 8;
                    bytes.push(u8::try_from((bits >> held) & 0xff).ok()?);
                    bits &= (1 << held) - 1;
                }
            }
            ::std::string::String::from_utf8(bytes).ok()
        }

        /// The `-32020` response of a request whose headers do not say what its body says.
        fn header_mismatch(request: &McpHttpRequest, message: &::serde_json::Value) -> ::core::option::Option<McpHttpResponse> {
            let method = message["method"].as_str().unwrap_or_default();
            let named = match method {
                "tools/call" | "prompts/get" => ::core::option::Option::Some(&message["params"]["name"]),
                "resources/read" => ::core::option::Option::Some(&message["params"]["uri"]),
                _ => ::core::option::Option::None,
            };
            let mismatched = |header: &str| {
                json_response(
                    400,
                    &failed(
                        &message["id"],
                        error_object(-32020, &format!("Header mismatch: {header}"), ::core::option::Option::None),
                    ),
                )
            };
            let body_version = &message["params"]["_meta"][PROTOCOL_KEY];
            let version = request.header("mcp-protocol-version");
            if version.is_none() || (!body_version.is_null() && ::core::option::Option::Some(body_version.as_str().unwrap_or_default()) != version) {
                return ::core::option::Option::Some(mismatched("MCP-Protocol-Version"));
            }
            if request.header("mcp-method") != ::core::option::Option::Some(method) {
                return ::core::option::Option::Some(mismatched("Mcp-Method"));
            }
            if let ::core::option::Option::Some(body_name) = named {
                let header = request.header("mcp-name").and_then(decoded_header);
                if header.as_deref() != body_name.as_str() || header.is_none() {
                    return ::core::option::Option::Some(mismatched("Mcp-Name"));
                }
            }
            ::core::option::Option::None
        }
    }
}

/// The rules a request meets before `admit`, and the denials `admit` answers with.
fn http_authorization() -> TokenStream {
    quote! {
        /// The path of `url`, without its query or fragment.
        fn url_path(url: &str) -> &str {
            let after = url.split_once("://").map_or(url, |(_, rest)| rest);
            let from_path = after.find('/').and_then(|at| after.get(at..)).unwrap_or("/");
            from_path.split(['?', '#']).next().unwrap_or(from_path)
        }

        /// The response to a denial: its status, and the challenge where the server uses OAuth.
        fn denial_response(
            denied: &McpDenied,
            settings: &McpHttpSettings<'_>,
            request: &McpHttpRequest,
        ) -> McpHttpResponse {
            let (status, challenge) = match (denied, settings.oauth()) {
                (McpDenied::NoValidCredentials { scope }, oauth) => {
                    let challenge = oauth.map(|oauth| {
                        let mut parts = ::std::vec![format!("resource_metadata=\"{}\"", oauth.resource_metadata_url())];
                        if let ::core::option::Option::Some(scope) = scope {
                            parts.push(format!("scope=\"{scope}\""));
                        }
                        if request.bearer_token().is_some() {
                            parts.push("error=\"invalid_token\"".to_owned());
                        }
                        format!("Bearer {}", parts.join(", "))
                    });
                    (401, challenge)
                }
                (McpDenied::InsufficientScope { scope }, oauth) => (
                    403,
                    oauth.map(|oauth| {
                        format!(
                            "Bearer error=\"insufficient_scope\", scope=\"{scope}\", resource_metadata=\"{}\"",
                            oauth.resource_metadata_url()
                        )
                    }),
                ),
                (McpDenied::Forbidden, _) => (403, ::core::option::Option::None),
            };
            let mut response = without_body(status);
            if let ::core::option::Option::Some(challenge) = challenge {
                response.headers.push(("WWW-Authenticate".to_owned(), challenge));
            }
            response
        }

        /// The protected-resource metadata document of `oauth`.
        fn metadata_response(oauth: &McpOAuth<'_>) -> McpHttpResponse {
            let mut document = ::serde_json::json!({
                "authorization_servers": oauth.authorization_servers(),
                "bearer_methods_supported": ["header"],
                "resource": oauth.resource(),
            });
            if !oauth.scopes_supported().is_empty() {
                document["scopes_supported"] = ::serde_json::json!(oauth.scopes_supported());
            }
            json_response(200, &document)
        }

        /// The rules a request meets before `admit`: the metadata document, the method and the
        /// `Origin`.
        fn before_admit(
            request: &McpHttpRequest,
            settings: &McpHttpSettings<'_>,
        ) -> ::core::result::Result<(), McpHttpResponse> {
            if let ::core::option::Option::Some(oauth) = settings.oauth() {
                let path = request.path().split(['?', '#']).next().unwrap_or_default();
                if request.method() == "GET" && path == url_path(oauth.resource_metadata_url()) {
                    return ::core::result::Result::Err(metadata_response(&oauth));
                }
            }
            if request.method() != "POST" {
                let mut refused = without_body(405);
                refused.headers.push(("Allow".to_owned(), "POST".to_owned()));
                return ::core::result::Result::Err(refused);
            }
            if let ::core::option::Option::Some(origin) = request.header("origin") {
                if !settings.allowed_origins().contains(&origin) {
                    return ::core::result::Result::Err(without_body(403));
                }
            }
            ::core::result::Result::Ok(())
        }
    }
}

/// The rules a request meets after `admit`, and the entry that applies them all.
fn http_entry() -> TokenStream {
    quote! {
        /// The rules a request meets once `admit` let it through: the text of its body where they
        /// let it through, and the response that refuses it where they do not.
        fn admitted<'request>(request: &'request McpHttpRequest, listens: bool, endpoint: &McpEndpoint) -> ::core::result::Result<&'request str, McpHttpResponse> {
            let ::core::result::Result::Ok(text) = ::core::str::from_utf8(request.body()) else {
                return ::core::result::Result::Err(json_response(
                    400,
                    &failed(
                        &::serde_json::Value::Null,
                        error_object(-32600, "Invalid request: the body is not UTF-8 text", ::core::option::Option::None),
                    ),
                ));
            };
            let message = match read_request(text) {
                ::core::result::Result::Ok(message) => message,
                ::core::result::Result::Err(refused) => return ::core::result::Result::Err(json_response(400, &refused)),
            };
            if message.get("id").is_none() {
                return ::core::result::Result::Err(without_body(202));
            }
            if let ::core::option::Option::Some(refused) = header_mismatch(request, &message) {
                return ::core::result::Result::Err(refused);
            }
            if let ::core::result::Result::Err(refused) = checked(&message) {
                return ::core::result::Result::Err(json_response(400, &refused));
            }
            let method = message["method"].as_str().unwrap_or_default();
            let task_method = matches!(method, "tasks/get" | "tasks/cancel" | "tasks/update");
            let known = if task_method { tasked(endpoint) } else { METHODS.contains(&method) };
            if !(known || (listens && method == "subscriptions/listen")) {
                return ::core::result::Result::Err(json_response(
                    404,
                    &failed(
                        &message["id"],
                        error_object(-32601, &format!("Method not found: {method}"), ::core::option::Option::None),
                    ),
                ));
            }
            ::core::result::Result::Ok(text)
        }

        /// The response that carries the text `serve` answered.
        fn http_response(answered: ::core::option::Option<::std::string::String>) -> McpHttpResponse {
            match answered {
                ::core::option::Option::Some(body) => McpHttpResponse::new(
                    200,
                    ::std::vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body.into_bytes(),
                ),
                ::core::option::Option::None => without_body(202),
            }
        }

    }
}

/// `serve_http`, and the answer it writes around the services it asks.
fn http_serving() -> TokenStream {
    let route_type = route_type();
    quote! {
        /// The Streamable HTTP rules around `serve`. `route` also receives the `Ctx` that
        /// `admit` answered. An event-stream answer is written to `stream` and answers `None`.
        pub async fn serve_http<Ctx, Admit, Stream>(
            request: &McpHttpRequest,
            settings: &McpHttpSettings<'_>,
            stream: &Stream,
            server: &McpServer<'_>,
            admit: Admit,
            route: #route_type,
        ) -> ::core::option::Option<McpHttpResponse>
        where
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            Admit: ::core::ops::AsyncFn(&McpHttpRequest, McpCall) -> ::core::result::Result<Ctx, McpDenied>,
            Admit: ::core::marker::Send + ::core::marker::Sync,
            Stream: McpHttpStream + ::core::marker::Sync,
        {
            serve_http_core(&McpHttpAsk { request, settings, stream }, server, admit, route, &mut idle, false).await
        }

        /// `serve_http` for an application that hands in its source of changes. A
        /// `subscriptions/listen` answers an event stream that ends when `changes` yields `None`.
        pub async fn serve_http_listening<Ctx, Admit, Stream, Changes>(
            request: &McpHttpRequest,
            settings: &McpHttpSettings<'_>,
            stream: &Stream,
            server: &McpServer<'_>,
            admit: Admit,
            route: #route_type,
            changes: &mut Changes,
        ) -> ::core::option::Option<McpHttpResponse>
        where
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            Admit: ::core::ops::AsyncFn(&McpHttpRequest, McpCall) -> ::core::result::Result<Ctx, McpDenied>,
            Admit: ::core::marker::Send + ::core::marker::Sync,
            Stream: McpHttpStream + ::core::marker::Sync,
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            serve_http_core(&McpHttpAsk { request, settings, stream }, server, admit, route, changes, true).await
        }

        async fn serve_http_core<Ctx, Admit, Stream, Changes>(
            ask: &McpHttpAsk<'_, Stream>,
            server: &McpServer<'_>,
            admit: Admit,
            route: #route_type,
            changes: &mut Changes,
            listens: bool,
        ) -> ::core::option::Option<McpHttpResponse>
        where
            Ctx: ::core::marker::Send + ::core::marker::Sync,
            Admit: ::core::ops::AsyncFn(&McpHttpRequest, McpCall) -> ::core::result::Result<Ctx, McpDenied>,
            Admit: ::core::marker::Send + ::core::marker::Sync,
            Stream: McpHttpStream + ::core::marker::Sync,
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            let McpHttpAsk { request, settings, stream } = *ask;
            if let ::core::result::Result::Err(refused) = before_admit(request, settings) {
                return ::core::option::Option::Some(refused);
            }
            let call = McpCall::of_body(request);
            let ctx = match admit(request, call.clone()).await {
                ::core::result::Result::Ok(ctx) => ctx,
                ::core::result::Result::Err(denied) => return ::core::option::Option::Some(denial_response(&denied, settings, request)),
            };
            match admitted(request, listens, server.endpoint) {
                ::core::result::Result::Err(refused) => ::core::option::Option::Some(refused),
                ::core::result::Result::Ok(text) => {
                    match started(text, server, ::core::option::Option::None, listens) {
                        ::core::result::Result::Err(answered) => ::core::option::Option::Some(http_response(answered)),
                        ::core::result::Result::Ok(asked_request) => {
                            let work = async {
                                if asked_request["method"] == "subscriptions/listen" {
                                    listening(&asked_request, &call, server.endpoint(), changes).await
                                } else {
                                    put_to_services(&asked_request, server.endpoint(), &ctx, route).await
                                }
                            };
                            http_answer(&asked_request, &call, stream, server.name(), work).await
                        }
                    }
                }
            }
        }
    }
}
