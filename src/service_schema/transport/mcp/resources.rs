//! Resources, resource templates and completion in the Rust dispatcher: what the catalog lists, the
//! arms of `route` that read, list and complete, and the listing `serve` pages across services.

use crate::service_schema::parse::{
    OperationDef, OperationInputs, OperationOutcome, PathSegment, ServiceDef, wire_key, written,
};
use crate::service_schema::support::{message_reader_ident, message_validator_ident};
use crate::service_schema::transport::amqp_rpc::call_arguments;
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Ident;

const COMPLETION_LIMIT: usize = 100;
const DEFAULT_TTL_MS: u64 = 0;
const DEFAULT_SCOPE: &str = "private";

/// The operations of a service that are not tools.
pub(super) struct Resourced<'service> {
    pub completions: Vec<&'service OperationDef>,
    pub fixed: Vec<&'service OperationDef>,
    pub listing: Option<&'service OperationDef>,
    pub templates: Vec<&'service OperationDef>,
}

impl<'service> Resourced<'service> {
    pub const fn any(&self) -> bool {
        !self.completions.is_empty()
            || !self.fixed.is_empty()
            || self.listing.is_some()
            || !self.templates.is_empty()
    }

    pub const fn is_served(&self) -> bool {
        !self.fixed.is_empty() || !self.templates.is_empty() || self.listing.is_some()
    }

    pub fn of(service: &'service ServiceDef) -> Self {
        let mut held = Self {
            completions: Vec::new(),
            fixed: Vec::new(),
            listing: None,
            templates: Vec::new(),
        };
        for operation in service
            .operations
            .iter()
            .filter(|operation| !operation.is_mcp_tool())
        {
            if let Some(resource) = &operation.mcp.resource {
                if resource.is_template() {
                    held.templates.push(operation);
                } else {
                    held.fixed.push(operation);
                }
            } else if operation.mcp.list_resources {
                held.listing = held.listing.or(Some(operation));
            } else {
                held.completions.push(operation);
            }
        }
        held
    }

    /// The operations that answer a `resources/read`, a fixed URI ahead of a template.
    fn reads(&self) -> impl Iterator<Item = &'service OperationDef> {
        self.fixed.iter().chain(self.templates.iter()).copied()
    }
}

/// The members of `catalog()` beyond its tools.
pub(super) struct CatalogParts {
    pub capabilities: TokenStream,
    pub resources: Vec<TokenStream>,
    pub templates: Vec<TokenStream>,
}

pub(super) fn catalog_parts(held: &Resourced<'_>) -> CatalogParts {
    let mut capabilities = TokenStream::new();
    if held.is_served() {
        capabilities.extend(quote! { , "resources": {} });
    }
    if !held.completions.is_empty() {
        capabilities.extend(quote! { , "completions": {} });
    }
    let entry = |operation: &OperationDef, key: &str| {
        let uri = operation
            .mcp
            .resource
            .as_ref()
            .map(|resource| resource.uri.as_str())
            .unwrap_or_default();
        let name = &operation.wire_name;
        let mut members = quote! { #key: #uri, "name": #name };
        if let Some(title) = &operation.mcp.title {
            members.extend(quote! { , "title": #title });
        }
        if let Some(description) = &operation.doc {
            members.extend(quote! { , "description": #description });
        }
        if let Some(mime) = &operation.mcp.mime_type {
            members.extend(quote! { , "mimeType": #mime });
        }
        quote! { { #members } }
    };
    CatalogParts {
        capabilities,
        resources: held
            .fixed
            .iter()
            .map(|operation| entry(operation, "uri"))
            .collect(),
        templates: held
            .templates
            .iter()
            .map(|operation| entry(operation, "uriTemplate"))
            .collect(),
    }
}

/// What a resource-backed operation answers a request with when its message would not read.
pub(super) fn helpers(held: &Resourced<'_>) -> TokenStream {
    let matching = (!held.templates.is_empty()).then(|| {
        quote! {
            /// What each placeholder of `template` captures in `uri`: one non-empty run of text up
            /// to the next `/`, with the literal parts compared as written.
            fn match_uri<'uri>(
                template: &[(&str, bool)],
                uri: &'uri str,
            ) -> ::core::option::Option<::std::vec::Vec<&'uri str>> {
                let mut rest = uri;
                let mut captured = ::std::vec::Vec::new();
                for (text, placeholder) in template {
                    if *placeholder {
                        let end = rest.find('/').unwrap_or(rest.len());
                        if end == 0 {
                            return ::core::option::Option::None;
                        }
                        let (value, remainder) = rest.split_at(end);
                        captured.push(value);
                        rest = remainder;
                    } else {
                        rest = rest.strip_prefix(text)?;
                    }
                }
                rest.is_empty().then_some(captured)
            }
        }
    });
    let blobs = held.reads().any(is_blob).then(|| {
        quote! {
            /// `bytes` in base64, with the standard alphabet and padding.
            fn base64_encoded(bytes: &[u8]) -> ::std::string::String {
                const ALPHABET: &[u8] =
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                let letter = |group: u32, shift: u32| -> char {
                    let at = usize::try_from((group >> shift) & 63).unwrap_or(0);
                    ALPHABET.get(at).copied().map_or('=', char::from)
                };
                let mut text = ::std::string::String::new();
                for chunk in bytes.chunks(3) {
                    let byte = |at: usize| u32::from(chunk.get(at).copied().unwrap_or(0));
                    let group = (byte(0) << 16_u32) | (byte(1) << 8_u32) | byte(2);
                    text.push(letter(group, 18));
                    text.push(letter(group, 12));
                    text.push(if chunk.len() > 1 { letter(group, 6) } else { '=' });
                    text.push(if chunk.len() > 2 { letter(group, 0) } else { '=' });
                }
                text
            }
        }
    });
    quote! {
        #matching
        #blobs

        /// A fault as the `-32602` error of a request that would not read.
        fn faulted(fault: impl ::serde::Serialize) -> ::serde_json::Value {
            invalid_params(
                "Invalid params",
                ::core::option::Option::Some(::serde_json::json!({ "isServiceFault": true, "fault": fault })),
            )
        }
    }
}

fn is_blob(operation: &OperationDef) -> bool {
    matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error }
            if matches!(success.as_ref(), syn::Type::Tuple(_))
    )
}

/// The `route` arm of `resources/read`.
pub(super) fn read_arm(held: &Resourced<'_>, module: &Ident) -> Option<TokenStream> {
    if held.fixed.is_empty() && held.templates.is_empty() {
        return None;
    }
    let reads = held.reads().map(|operation| read_block(operation, module));
    Some(quote! {
        "resources/read" => {
            let uri = params.get("uri")?.as_str()?;
            #(#reads)*
            ::core::option::Option::None
        }
    })
}

/// The `route` arm of `resources/list`.
pub(super) fn list_arm(held: &Resourced<'_>, module: &Ident) -> Option<TokenStream> {
    let operation = held.listing?;
    let wire = &operation.wire_name;
    let method = &operation.ident;
    let reader = message_reader_ident(operation);
    let validator = message_validator_ident(operation);
    let call = call_arguments(operation);
    let ttl = ttl_of(operation);
    let scope = scope_of(operation);
    let source = match &operation.inputs {
        OperationInputs::Named(_) => quote! {
            match params.get("cursor") {
                ::core::option::Option::Some(::serde_json::Value::String(held)) => {
                    ::serde_json::Value::String(held.clone())
                }
                _ => ::serde_json::Value::Null,
            }
        },
        OperationInputs::Empty | OperationInputs::Generated(_) => {
            quote! { ::serde_json::json!({}) }
        }
    };
    let reading = reading(operation, module, &source, &reader, &validator);
    Some(quote! {
        "resources/list" => {
            #reading
            ::core::option::Option::Some(
                match caught(move || svc.#method(ctx #(, #call)*)).await {
                    ::core::result::Result::Err(panicked) => {
                        record_panic(#wire, &panicked);
                        ::core::result::Result::Err(internal_error(#wire, &panicked))
                    }
                    ::core::result::Result::Ok(::core::result::Result::Err(error)) => {
                        match encoded(#wire, &error) {
                            ::core::result::Result::Ok(data) => ::core::result::Result::Err(invalid_params(
                                "Resources could not be listed",
                                ::core::option::Option::Some(data),
                            )),
                            ::core::result::Result::Err(refused) => ::core::result::Result::Err(refused),
                        }
                    }
                    ::core::result::Result::Ok(::core::result::Result::Ok(page)) => {
                        match encoded(#wire, &page) {
                            ::core::result::Result::Ok(mut result) => {
                                result["resultType"] = ::serde_json::json!("complete");
                                result["ttlMs"] = ::serde_json::json!(#ttl);
                                result["cacheScope"] = ::serde_json::json!(#scope);
                                ::core::result::Result::Ok(result)
                            }
                            ::core::result::Result::Err(refused) => ::core::result::Result::Err(refused),
                        }
                    }
                },
            )
        }
    })
}

/// The `route` arm of `completion/complete`.
pub(super) fn complete_arm(
    held: &Resourced<'_>,
    service: &ServiceDef,
    module: &Ident,
) -> Option<TokenStream> {
    if held.completions.is_empty() {
        return None;
    }
    let blocks = held
        .completions
        .iter()
        .filter_map(|operation| complete_block(operation, service, module));
    Some(quote! {
        "completion/complete" => {
            let reference = params.get("ref")?;
            if reference["type"] != "ref/resource" {
                return ::core::option::Option::None;
            }
            let uri = reference["uri"].as_str()?;
            let parameter = params.get("argument")?["name"].as_str()?;
            let typed = params["argument"]["value"].as_str().unwrap_or_default();
            let chosen = match params["context"].get("arguments") {
                ::core::option::Option::Some(held) => held.clone(),
                ::core::option::Option::None => ::serde_json::json!({}),
            };
            #(#blocks)*
            ::core::option::Option::None
        }
    })
}

fn complete_block(
    operation: &OperationDef,
    service: &ServiceDef,
    module: &Ident,
) -> Option<TokenStream> {
    let completes = operation.mcp.completes.as_ref()?;
    let target = service
        .operations
        .iter()
        .find(|candidate| written(&candidate.ident) == completes.resource)?;
    let uri = &target.mcp.resource.as_ref()?.uri;
    let parameter = &completes.parameter;
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
    let reading = reading(operation, module, &source, &reader, &validator);
    let limit = Literal::usize_unsuffixed(COMPLETION_LIMIT);
    Some(quote! {
        if uri == #uri && parameter == #parameter {
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

/// The statements that read and judge the message of `operation` from `source`, answering the
/// request with a fault where either refuses it.
fn reading(
    operation: &OperationDef,
    module: &Ident,
    source: &TokenStream,
    reader: &Ident,
    validator: &Ident,
) -> TokenStream {
    let wire = &operation.wire_name;
    quote! {
        let received = match $crate::#module::#reader(#source) {
            ::core::result::Result::Ok(received) => received,
            ::core::result::Result::Err(rejected) => {
                return ::core::option::Option::Some(::core::result::Result::Err(faulted(
                    $crate::#module::ServiceFault::undeserializable_payload(
                        #wire,
                        &::std::string::ToString::to_string(&rejected),
                    ),
                )));
            }
        };
        if let ::core::result::Result::Err(violations) = $crate::#module::#validator(&received) {
            return ::core::option::Option::Some(::core::result::Result::Err(faulted(
                $crate::#module::ServiceFault::failed_validation(
                    #wire,
                    $crate::#module::violated_field(&violations),
                    &$crate::#module::violation_detail(&violations),
                ),
            )));
        }
    }
}

fn ttl_of(operation: &OperationDef) -> TokenStream {
    let ttl = operation.mcp.ttl_ms.unwrap_or(DEFAULT_TTL_MS);
    format!("{ttl}_u64").parse().unwrap_or_default()
}

fn scope_of(operation: &OperationDef) -> &str {
    operation
        .mcp
        .cache_scope
        .as_deref()
        .unwrap_or(DEFAULT_SCOPE)
}

fn read_block(operation: &OperationDef, module: &Ident) -> TokenStream {
    let Some(resource) = operation.mcp.resource.as_ref() else {
        return TokenStream::new();
    };
    let wire = &operation.wire_name;
    let method = &operation.ident;
    let reader = message_reader_ident(operation);
    let validator = message_validator_ident(operation);
    let call = call_arguments(operation);
    let names = resource.placeholder_names();
    let source = message_source(operation, &names);
    let reading = reading(operation, module, &source, &reader, &validator);
    let ttl = ttl_of(operation);
    let scope = scope_of(operation);
    let mime = operation
        .mcp
        .mime_type
        .as_ref()
        .map(|mime| quote! { "mimeType": #mime, });
    let (pattern, content) = if is_blob(operation) {
        (
            quote! { (bytes, content_type) },
            quote! { { "uri": uri, "mimeType": content_type, "blob": base64_encoded(&bytes) } },
        )
    } else {
        (
            quote! { text },
            quote! { { "uri": uri, #mime "text": text } },
        )
    };
    let body = quote! {
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
                            "Resource not found",
                            ::core::option::Option::Some(data),
                        )),
                        ::core::result::Result::Err(refused) => ::core::result::Result::Err(refused),
                    }
                }
                ::core::result::Result::Ok(::core::result::Result::Ok(#pattern)) => {
                    ::core::result::Result::Ok(::serde_json::json!({
                        "resultType": "complete",
                        "contents": [::serde_json::json!(#content)],
                        "ttlMs": #ttl,
                        "cacheScope": #scope,
                    }))
                }
            },
        );
    };
    if resource.is_template() {
        let parts = resource.segments.iter().map(|segment| match segment {
            PathSegment::Literal(text) => quote! { (#text, false) },
            PathSegment::Placeholder(name) => quote! { (#name, true) },
        });
        quote! {
            if let ::core::option::Option::Some(captured) = match_uri(&[#(#parts),*], uri) {
                #body
            }
        }
    } else {
        let fixed = &resource.uri;
        quote! {
            if uri == #fixed {
                #body
            }
        }
    }
}

/// The value the message is read from: the placeholders the URI captured, under the keys the
/// operation's arguments write.
fn message_source(operation: &OperationDef, placeholders: &[&str]) -> TokenStream {
    match &operation.inputs {
        OperationInputs::Empty => quote! { ::serde_json::json!({}) },
        OperationInputs::Named(_) => {
            if placeholders.is_empty() {
                quote! { ::serde_json::Value::Null }
            } else {
                quote! { ::serde_json::Value::String(captured[0].to_owned()) }
            }
        }
        OperationInputs::Generated(fields) => {
            let members = placeholders.iter().enumerate().filter_map(|(at, name)| {
                let (field, _) = fields.iter().find(|(field, _)| written(field) == *name)?;
                let key = wire_key(field);
                Some(quote! { #key: captured[#at] })
            });
            quote! { ::serde_json::json!({ #(#members),* }) }
        }
    }
}

/// How `serve` pages `resources/list` across the services of an endpoint.
pub(super) fn listing_fns() -> TokenStream {
    quote! {
        /// A JSON-RPC `-32602` error, with `data` where there is some.
        fn invalid_params(message: &str, data: ::core::option::Option<::serde_json::Value>) -> ::serde_json::Value {
            error_object(-32602, message, data)
        }

        /// Where a `resources/list` starts: the service, its own cursor, and whether this is the
        /// first page. A cursor that names no service is the `-32602` it asks for.
        fn listing_start(
            request: &::serde_json::Value,
            services: usize,
        ) -> ::core::result::Result<(usize, ::core::option::Option<::std::string::String>, bool), ::serde_json::Value> {
            let invalid = || invalid_params("Invalid cursor", ::core::option::Option::None);
            match request["params"].get("cursor") {
                ::core::option::Option::None | ::core::option::Option::Some(::serde_json::Value::Null) => {
                    ::core::result::Result::Ok((0, ::core::option::Option::None, true))
                }
                ::core::option::Option::Some(::serde_json::Value::String(written)) => {
                    let (part, own) = written.split_once(':').ok_or_else(invalid)?;
                    let part: usize = part.parse().map_err(|_| invalid())?;
                    if part >= services {
                        return ::core::result::Result::Err(invalid());
                    }
                    let own = if own.is_empty() {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(own.to_owned())
                    };
                    ::core::result::Result::Ok((part, own, false))
                }
                ::core::option::Option::Some(_) => ::core::result::Result::Err(invalid()),
            }
        }

        /// The params a service is asked its own page with.
        fn listing_params(own: ::core::option::Option<&::std::string::String>) -> ::serde_json::Value {
            match own {
                ::core::option::Option::Some(cursor) => ::serde_json::json!({ "cursor": cursor }),
                ::core::option::Option::None => ::serde_json::json!({}),
            }
        }

        /// One page of `resources/list`: the resources with a fixed URI on the first page, and the
        /// page the first service that lists answered, its cursor naming its place.
        fn listed_resources(
            first: bool,
            endpoint: &McpEndpoint,
            answered: ::core::option::Option<(usize, ::core::result::Result<::serde_json::Value, ::serde_json::Value>)>,
        ) -> ::core::result::Result<::serde_json::Value, ::serde_json::Value> {
            let mut resources = ::std::vec::Vec::new();
            if first {
                for catalog in endpoint.catalogs {
                    if let ::core::option::Option::Some(::serde_json::Value::Array(offered)) = catalog().get("resources") {
                        resources.extend(offered.iter().cloned());
                    }
                }
            }
            let mut result = ::serde_json::json!({
                "resultType": "complete",
                "ttlMs": 0_u64,
                "cacheScope": "private",
            });
            if let ::core::option::Option::Some((part, page)) = answered {
                let page = page?;
                if let ::core::option::Option::Some(::serde_json::Value::Array(listed)) = page.get("resources") {
                    resources.extend(listed.iter().cloned());
                }
                for key in ["ttlMs", "cacheScope"] {
                    if let ::core::option::Option::Some(held) = page.get(key) {
                        result[key] = held.clone();
                    }
                }
                let own = page.get("nextCursor").and_then(::serde_json::Value::as_str).filter(|cursor| !cursor.is_empty());
                let next = match own {
                    ::core::option::Option::Some(cursor) => ::core::option::Option::Some(format!("{part}:{cursor}")),
                    ::core::option::Option::None => (part + 1 < endpoint.catalogs.len()).then(|| format!("{}:", part + 1)),
                };
                if let ::core::option::Option::Some(next) = next {
                    result["nextCursor"] = ::serde_json::Value::String(next);
                }
            }
            result["resources"] = ::serde_json::Value::Array(resources);
            ::core::result::Result::Ok(result)
        }
    }
}
