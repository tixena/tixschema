//! The dispatcher half of the `mcp` transport: `serve` and `serve_http` around one `route` per
//! service, and the `answer` and `answer_http` of a service that stands alone.

use super::resources::{self, Resourced};
use super::{Carried, error_carried, success_carried};
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
         `serve_http` and `answer_http` - so the caller supplies the module they land in and two \
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
    let catalog = catalog(&schema, &held);
    let route = route(service, &module, &schema, contract, &held);
    let listing = resources::listing_fns();
    let protocol = protocol();
    let answers = answers();
    let serving = serving(&module);
    let serve_fn = serve_fn();
    let answer = answer(contract);
    let http_fns = http_fns();
    let http_rules = http_rules();
    let http_entry = http_entry();
    quote! {
        #[doc = #macro_doc]
        #[macro_export]
        macro_rules! #macro_name {
            () => {
                #constants
                #types
                #catalog
                #route
                #endpoint_fns
                #protocol
                #listing
                #answers
                #serving
                #serve_fn
                #answer
                #http_fns
                #http_rules
                #http_entry
            };
        }
    }
}

fn catalog(schema: &Ident, held: &Resourced<'_>) -> TokenStream {
    let parts = resources::catalog_parts(held);
    let capabilities = parts.capabilities;
    let (resources, templates) = (parts.resources, parts.templates);
    quote! {
        /// Everything the list methods answer for this service, fixed at compile time.
        pub fn catalog() -> ::serde_json::Value {
            ::serde_json::json!({
                "capabilities": { "tools": {} #capabilities },
                "tools": $crate::#schema::mcp_tools(),
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
    let resource_helpers = held.any().then(|| resources::helpers(held));
    let reads = resources::read_arm(held, module);
    let listing = resources::list_arm(held, module);
    let completing = resources::complete_arm(held, service, module);
    quote! {
        #guard
        #(#replies)*
        #tool_results
        #resource_helpers

        /// One request this service owns: `tools/call`, `resources/read`, its own page of
        /// `resources/list` and `completion/complete`. `None` for a request it does not own,
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
    let settled = settled(operation, &called, tool);
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
fn settled(operation: &OperationDef, called: &TokenStream, tool: &str) -> TokenStream {
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
    let (success_pattern, success_arm) =
        carried_arm(&success_carried(operation, success), tool, false);
    let (error_pattern, error_arm) = carried_arm(&error_carried(operation, error), tool, true);
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
            pub catalogs: &'static [fn() -> ::serde_json::Value],
        }

        impl McpEndpoint {
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

        /// A JSON-RPC error response.
        fn failed(id: &::serde_json::Value, error: ::serde_json::Value) -> ::serde_json::Value {
            ::serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": error })
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
                _ => params["argument"]["name"]
                    .as_str()
                    .and(params["ref"]["uri"].as_str()),
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

        fn discovered(instructions: ::core::option::Option<&str>, endpoint: &McpEndpoint) -> ::serde_json::Value {
            let mut capabilities = ::serde_json::Map::new();
            let mut extensions = ::serde_json::Map::new();
            for catalog in endpoint.catalogs {
                let catalog = catalog();
                for (held, into) in [("capabilities", &mut capabilities), ("extensions", &mut extensions)] {
                    if let ::core::option::Option::Some(::serde_json::Value::Object(offered)) = catalog.get(held) {
                        for (key, value) in offered {
                            into.entry(key.clone()).or_insert_with(|| value.clone());
                        }
                    }
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
            for catalog in endpoint.catalogs {
                if let ::core::option::Option::Some(::serde_json::Value::Array(offered)) = catalog().get(key) {
                    items.extend(offered.iter().cloned());
                }
            }
            ::serde_json::json!({
                "resultType": "complete",
                key: items,
                "ttlMs": LISTED_FOR_MS,
                "cacheScope": "public",
            })
        }
    }
}

/// `serve`, and what it does ahead of asking a service.
fn serving(module: &Ident) -> TokenStream {
    quote! {
        /// Everything `serve` does ahead of asking a service.
        fn begun(text: &str, instructions: ::core::option::Option<&str>, endpoint: &McpEndpoint, server_name: &str) -> Begun {
            let request = match read_request(text) {
                ::core::result::Result::Ok(request) => request,
                ::core::result::Result::Err(refused) => return Begun::Answered(::core::option::Option::Some(canonical(&refused))),
            };
            let ::core::option::Option::Some(id) = request.get("id") else {
                return Begun::Answered(::core::option::Option::None);
            };
            if let ::core::result::Result::Err(refused) = checked(&request) {
                return Begun::Answered(::core::option::Option::Some(canonical(&refused)));
            }
            let answered = match request["method"].as_str().unwrap_or_default() {
                "server/discover" => ::core::result::Result::Ok(discovered(instructions, endpoint)),
                "tools/list" => ::core::result::Result::Ok(listed(endpoint, "tools")),
                "resources/templates/list" => {
                    ::core::result::Result::Ok(listed(endpoint, "resourceTemplates"))
                }
                "resources/list" => return Begun::Routed(request),
                method @ ("tools/call" | "resources/read" | "completion/complete") => {
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

        /// What a routed `tools/call` answers once its services have been asked.
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

/// `serve`, the loop over the services of an endpoint.
fn serve_fn() -> TokenStream {
    quote! {
        /// One JSON-RPC message in, one out, for the services of the endpoint handed in. `route`
        /// answers for the service at that place in the endpoint. A notification answers `None`.
        pub async fn serve<R>(
            text: &str,
            server_name: &str,
            instructions: ::core::option::Option<&str>,
            endpoint: &McpEndpoint,
            route: R,
        ) -> ::core::option::Option<::std::string::String>
        where
            R: ::core::ops::AsyncFn(usize, &str, &::serde_json::Value) -> ::core::option::Option<
                ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
            >,
        {
            let request = match begun(text, instructions, endpoint, server_name) {
                Begun::Answered(text) => return text,
                Begun::Routed(request) => request,
            };
            let method = request["method"].as_str().unwrap_or_default();
            if method == "resources/list" {
                let (start, own, first) = match listing_start(&request, endpoint.catalogs.len()) {
                    ::core::result::Result::Ok(start) => start,
                    ::core::result::Result::Err(refused) => {
                        return ::core::option::Option::Some(response_text(&request["id"], ::core::result::Result::Err(refused), server_name));
                    }
                };
                let mut answered = ::core::option::Option::None;
                for part in start..endpoint.catalogs.len() {
                    let params = listing_params(if part == start { own.as_ref() } else { ::core::option::Option::None });
                    if let ::core::option::Option::Some(page) = route(part, "resources/list", &params).await {
                        answered = ::core::option::Option::Some((part, page));
                        break;
                    }
                }
                return ::core::option::Option::Some(response_text(
                    &request["id"],
                    listed_resources(first, endpoint, answered),
                    server_name,
                ));
            }
            let mut answered = ::core::option::Option::None;
            for part in 0..endpoint.catalogs.len() {
                answered = route(part, method, &request["params"]).await;
                if answered.is_some() {
                    break;
                }
            }
            routed_text(&request, answered, server_name)
        }
    }
}

fn answer(contract: &Ident) -> TokenStream {
    let schema = format_ident!("{contract}Schema", span = contract.span());
    quote! {
        /// `serve` for this service alone, with `SERVER_NAME`, `INSTRUCTIONS`, `catalog()` and
        /// `route`. Usable as is for stdio, and its future is `Send` wherever `Svc` and `Ctx`
        /// allow it.
        pub async fn answer<Svc, Ctx>(
            text: &str,
            svc: &Svc,
            ctx: &Ctx,
        ) -> ::core::option::Option<::std::string::String>
        where
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Sync,
        {
            const ALONE: McpEndpoint = McpEndpoint::new(&[$crate::#schema::MCP_NAMES], &[catalog]);
            let request = match begun(text, INSTRUCTIONS, &ALONE, SERVER_NAME) {
                Begun::Answered(text) => return text,
                Begun::Routed(request) => request,
            };
            let method = request["method"].as_str().unwrap_or_default();
            if method == "resources/list" {
                let (_start, own, first) = match listing_start(&request, ALONE.catalogs.len()) {
                    ::core::result::Result::Ok(start) => start,
                    ::core::result::Result::Err(refused) => {
                        return ::core::option::Option::Some(response_text(&request["id"], ::core::result::Result::Err(refused), SERVER_NAME));
                    }
                };
                let answered = route("resources/list", &listing_params(own.as_ref()), svc, ctx)
                    .await
                    .map(|page| (0, page));
                return ::core::option::Option::Some(response_text(
                    &request["id"],
                    listed_resources(first, &ALONE, answered),
                    SERVER_NAME,
                ));
            }
            let answered = route(method, &request["params"], svc, ctx).await;
            routed_text(&request, answered, SERVER_NAME)
        }

        /// `serve_http` for this service alone.
        pub async fn answer_http<Svc, Ctx>(
            request: &McpHttpRequest,
            allowed_origins: &[&str],
            svc: &Svc,
            ctx: &Ctx,
        ) -> McpHttpResponse
        where
            Svc: $crate::#contract<Ctx> + ::core::marker::Sync,
            Ctx: ::core::marker::Sync,
        {
            match admitted(request, allowed_origins) {
                ::core::result::Result::Err(refused) => refused,
                ::core::result::Result::Ok(text) => {
                    http_response(answer(&text, svc, ctx).await)
                }
            }
        }
    }
}

fn http_fns() -> TokenStream {
    quote! {
        fn without_body(status: u16) -> McpHttpResponse {
            McpHttpResponse { status, headers: ::std::vec::Vec::new(), body: ::std::vec::Vec::new() }
        }

        fn json_response(status: u16, body: &::serde_json::Value) -> McpHttpResponse {
            McpHttpResponse {
                status,
                headers: ::std::vec![("Content-Type".to_owned(), "application/json".to_owned())],
                body: canonical(body).into_bytes(),
            }
        }
    }
}

/// The enum and structs the dispatcher publishes or reads, with their impls.
fn types() -> TokenStream {
    let endpoint = endpoint_type();
    quote! {
        #endpoint

        /// What a message needs before a service is asked: its answer, or the `tools/call` that
        /// `route` is to answer.
        enum Begun {
            Answered(::core::option::Option<::std::string::String>),
            Routed(::serde_json::Value),
        }

        /// An HTTP request in plain terms, the way `http_rest` publishes its own.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct McpHttpRequest {
            pub body: ::std::vec::Vec<u8>,
            pub headers: ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
            pub method: ::std::string::String,
        }

        /// An HTTP response in plain terms.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct McpHttpResponse {
            pub body: ::std::vec::Vec<u8>,
            pub headers: ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
            pub status: u16,
        }

        impl McpHttpRequest {
            /// The value of the header called `name`, whatever its case.
            pub fn header(&self, name: &str) -> ::core::option::Option<&str> {
                self.headers
                    .iter()
                    .find(|(held, _)| held.eq_ignore_ascii_case(name))
                    .map(|(_, value)| value.as_str())
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

        const CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
        const LISTED_FOR_MS: u64 = 300_000;
        /// The methods `serve` answers itself or asks `route` for.
        const METHODS: [&str; 7] = [
            "completion/complete",
            "resources/list",
            "resources/read",
            "resources/templates/list",
            "server/discover",
            "tools/call",
            "tools/list",
        ];
        const PROTOCOL_KEY: &str = "io.modelcontextprotocol/protocolVersion";
        const PROTOCOL_VERSION: &str = "2026-07-28";
        const SERVER_INFO_KEY: &str = "io.modelcontextprotocol/serverInfo";
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

/// The rules a request meets before any service is asked, and the entry that applies them.
fn http_entry() -> TokenStream {
    quote! {
        /// The rules a request meets before any service is asked: the text of its body where
        /// they let it through, and the response that refuses it where they do not.
        fn admitted<'request>(
            request: &'request McpHttpRequest,
            allowed_origins: &[&str],
        ) -> ::core::result::Result<&'request str, McpHttpResponse> {
            if request.method != "POST" {
                let mut refused = without_body(405);
                refused.headers.push(("Allow".to_owned(), "POST".to_owned()));
                return ::core::result::Result::Err(refused);
            }
            if let ::core::option::Option::Some(origin) = request.header("origin") {
                if !allowed_origins.contains(&origin) {
                    return ::core::result::Result::Err(without_body(403));
                }
            }
            let ::core::result::Result::Ok(text) = ::core::str::from_utf8(&request.body) else {
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
            if !METHODS.contains(&method) {
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
                ::core::option::Option::Some(body) => McpHttpResponse {
                    status: 200,
                    headers: ::std::vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: body.into_bytes(),
                },
                ::core::option::Option::None => without_body(202),
            }
        }

        /// The Streamable HTTP rules around `serve`.
        pub async fn serve_http<R>(
            request: &McpHttpRequest,
            allowed_origins: &[&str],
            server_name: &str,
            instructions: ::core::option::Option<&str>,
            endpoint: &McpEndpoint,
            route: R,
        ) -> McpHttpResponse
        where
            R: ::core::ops::AsyncFn(usize, &str, &::serde_json::Value) -> ::core::option::Option<
                ::core::result::Result<::serde_json::Value, ::serde_json::Value>,
            >,
        {
            match admitted(request, allowed_origins) {
                ::core::result::Result::Err(refused) => refused,
                ::core::result::Result::Ok(text) => {
                    http_response(serve(text, server_name, instructions, endpoint, route).await)
                }
            }
        }
    }
}
