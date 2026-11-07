//! `<Service>Schema::mcp_tools()`, `wrapped_{operation}()` and `MCP_NAMES`: the tool list of an
//! `mcp` service, each tool built from its operation's message, success, headers and rustdoc.

use super::parse::{
    BodyKind, HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, is_unit_type,
    option_inner, tuple_elements, wire_key, written,
};
use super::transport::http_rest::openapi::message_type;
use crate::features::jsonschema::in_flight_type;
use crate::model_schema::needs_wrapper;
use crate::model_schema::type_json_schema_value;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Type;

const DEFS_PREFIX: &str = "#/$defs/";

/// The members of `<Service>Schema` the `mcp` transport reads.
pub fn registry_items(service: &ServiceDef) -> TokenStream {
    let wrapped = service.operations.iter().map(wrapped_method);
    let tools = tools_method(service);
    let instructions = service.doc.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |text| quote! { ::core::option::Option::Some(#text) },
    );
    let names = service
        .operations
        .iter()
        .map(|operation| operation.mcp_tool_name().to_owned());
    quote! {
        #tools

        #(#wrapped)*

        #[doc = " The trait's rustdoc, which the server answers as its `instructions`."]
        pub const MCP_INSTRUCTIONS: ::core::option::Option<&'static str> = #instructions;

        #[doc = " Every tool the service lists, as `(\"tool\", <name>)`."]
        pub const MCP_NAMES: &'static [(&'static str, &'static str)] = &[#(("tool", #names)),*];
    }
}

fn wrapped_ident(operation: &OperationDef) -> syn::Ident {
    format_ident!(
        "wrapped_{}",
        written(&operation.ident),
        span = operation.ident.span()
    )
}

/// `wrapped_{operation}()`: whether the tool's arguments object holds the message under `value`.
fn wrapped_method(operation: &OperationDef) -> TokenStream {
    let name = wrapped_ident(operation);
    let binds_a_header = !HttpShape::of(operation).header_in.is_empty();
    let answer = match &operation.inputs {
        OperationInputs::Named(declared) if binds_a_header => {
            let _: &Type = declared;
            quote! { true }
        }
        OperationInputs::Named(declared) => needs_wrapper::of_type(declared),
        OperationInputs::Empty | OperationInputs::Generated(_) => quote! { false },
    };
    let doc = format!(
        " Whether the tool `{}` holds its message under `value`.",
        operation.mcp_tool_name()
    );
    quote! {
        #[doc = #doc]
        pub const fn #name() -> bool {
            #answer
        }
    }
}

fn tools_method(service: &ServiceDef) -> TokenStream {
    let tools = service.operations.iter().map(tool_value);
    let helpers = helpers();
    quote! {
        #[doc = " The service's tools, one per operation in the order the trait declares them."]
        pub fn mcp_tools() -> ::std::vec::Vec<::serde_json::Value> {
            #helpers
            ::std::vec![#(#tools),*]
        }
    }
}

/// One tool as the `Tool` of the MCP schema writes it.
fn tool_value(operation: &OperationDef) -> TokenStream {
    let name = operation.mcp_tool_name();
    let title = operation.mcp_tool_title();
    let description = operation.doc.clone().unwrap_or_default();
    let input = input_schema(operation);
    let output = output_schema(operation).map(|schema| {
        quote! { tool.insert("outputSchema".to_owned(), #schema); }
    });
    let annotations = annotations(operation);
    let icons = operation.mcp.icon.as_ref().map(|source| {
        quote! {
            tool.insert(
                "icons".to_owned(),
                ::serde_json::json!([{ "src": #source }]),
            );
        }
    });
    quote! {
        {
            let mut tool = ::serde_json::Map::new();
            tool.insert("name".to_owned(), ::serde_json::Value::String(#name.to_owned()));
            tool.insert("title".to_owned(), ::serde_json::Value::String(#title.to_owned()));
            tool.insert(
                "description".to_owned(),
                ::serde_json::Value::String(#description.to_owned()),
            );
            tool.insert("inputSchema".to_owned(), #input);
            #output
            #annotations
            #icons
            ::serde_json::Value::Object(tool)
        }
    }
}

fn annotations(operation: &OperationDef) -> Option<TokenStream> {
    let mcp = &operation.mcp;
    let mut hints: Vec<TokenStream> = Vec::new();
    if mcp.read_only {
        hints.push(quote! { "readOnlyHint": true });
    }
    if let Some(destructive) = mcp.destructive {
        hints.push(quote! { "destructiveHint": #destructive });
    }
    if mcp.idempotent {
        hints.push(quote! { "idempotentHint": true });
    }
    if let Some(open_world) = mcp.open_world {
        hints.push(quote! { "openWorldHint": #open_world });
    }
    (!hints.is_empty()).then(|| {
        quote! {
            tool.insert("annotations".to_owned(), ::serde_json::json!({ #(#hints),* }));
        }
    })
}

/// The statements that bind the run a schema is described in: the names a rendering of a type
/// reads.
fn run() -> TokenStream {
    let in_flight_type = in_flight_type();
    quote! {
        let mut in_flight: #in_flight_type = ::std::vec::Vec::new();
        let mut hoisted_defs = ::serde_json::Map::new();
        let pointer_prefix: &str = #DEFS_PREFIX;
    }
}

/// `ty` described in the run [`run`] binds.
fn described(subject: &str, ty: &Type) -> TokenStream {
    let schema = type_json_schema_value(subject, ty);
    quote! {
        {
            let in_flight = &mut in_flight;
            let hoisted_defs = &mut hoisted_defs;
            #schema
        }
    }
}

/// One header, as the name it is a property under, its schema, and whether it is required.
fn header_property(subject: &str, name: &str, ty: &Type) -> TokenStream {
    let schema = described(subject, ty);
    let required = option_inner(ty).is_none();
    quote! { (#name, #schema, #required) }
}

fn input_schema(operation: &OperationDef) -> TokenStream {
    let subject = operation.ident.to_string();
    let shape = HttpShape::of(operation);
    let run = run();
    let message = message_type(operation).map_or_else(
        || quote! { ::serde_json::json!({ "type": "object" }) },
        |declared| described(&subject, &declared),
    );
    let headers = shape
        .header_in
        .iter()
        .map(|bound| header_property(&subject, &bound.name, &bound.ty));
    let wrapped = match &operation.inputs {
        OperationInputs::Named(_) => {
            let method = wrapped_ident(operation);
            quote! { Self::#method() }
        }
        OperationInputs::Empty | OperationInputs::Generated(_) => quote! { false },
    };
    let marks = header_marks(operation, &shape);
    quote! {
        {
            #run
            let message = #message;
            let headers = ::std::vec![#(#headers),*];
            arguments_object(message, #wrapped, headers, &[#(#marks),*], hoisted_defs)
        }
    }
}

/// Each `mcp(header(...))` as the property it marks and the header it is mirrored into.
fn header_marks(operation: &OperationDef, shape: &HttpShape) -> Vec<TokenStream> {
    operation
        .mcp
        .header
        .iter()
        .filter_map(|(argument, header)| {
            let name = written(argument);
            let key = match &operation.inputs {
                OperationInputs::Generated(fields) => fields
                    .iter()
                    .find(|(field, _)| written(field) == name)
                    .map(|(field, _)| wire_key(field)),
                OperationInputs::Empty | OperationInputs::Named(_) => None,
            }
            .or_else(|| {
                shape
                    .header_in
                    .iter()
                    .find(|bound| written(&bound.parameter) == name)
                    .map(|bound| bound.name.clone())
            })?;
            Some(quote! { (#key, #header) })
        })
        .collect()
}

fn output_schema(operation: &OperationDef) -> Option<TokenStream> {
    let OperationOutcome::Reply {
        success,
        error: _error,
    } = &operation.outcome
    else {
        return None;
    };
    let shape = HttpShape::of(operation);
    let subject = operation.ident.to_string();
    let elements: Vec<Type> = tuple_elements(success)
        .map(|elements| elements.iter().cloned().collect())
        .unwrap_or_default();
    let (body, first_header): (Type, usize) = if shape.header_out.is_empty() {
        if is_unit_type(success) {
            return None;
        }
        (success.as_ref().clone(), 0)
    } else if matches!(shape.body_kind, BodyKind::Bytes) {
        let (bytes, content_type) = (elements.first()?, elements.get(1)?);
        (syn::parse_quote! { (#bytes, #content_type) }, 2)
    } else {
        (elements.first()?.clone(), 1)
    };
    let run = run();
    let body_schema = described(&subject, &body);
    let headers = shape
        .header_out
        .iter()
        .zip(elements.iter().skip(first_header))
        .map(|(name, ty)| header_property(&subject, name, ty));
    Some(quote! {
        {
            #run
            let success = #body_schema;
            let headers = ::std::vec![#(#headers),*];
            output_object(success, headers, hoisted_defs)
        }
    })
}

/// The two functions every tool's schemas are finished by, written once inside `mcp_tools()`:
/// the arguments object of an input, and the object of an output.
fn helpers() -> TokenStream {
    quote! {
        type Members = ::serde_json::Map<::std::string::String, ::serde_json::Value>;
        type Headers = ::std::vec::Vec<(&'static str, ::serde_json::Value, bool)>;

        fn text(written: &str) -> ::serde_json::Value {
            ::serde_json::Value::String(written.to_owned())
        }

        fn members_of(schema: ::serde_json::Value) -> Members {
            match schema {
                ::serde_json::Value::Object(members) => members,
                other => {
                    let mut members = Members::new();
                    members.insert("allOf".to_owned(), ::serde_json::Value::Array(::std::vec![other]));
                    members
                }
            }
        }

        fn beside_value(message: ::serde_json::Value, headers: Headers) -> Members {
            let mut properties = Members::new();
            properties.insert("value".to_owned(), message);
            let mut required = ::std::vec![text("value")];
            for (name, schema, needed) in headers {
                properties.insert(name.to_owned(), schema);
                if needed {
                    required.push(text(name));
                }
            }
            let mut root = Members::new();
            root.insert("additionalProperties".to_owned(), ::serde_json::Value::Bool(false));
            root.insert("properties".to_owned(), ::serde_json::Value::Object(properties));
            root.insert("required".to_owned(), ::serde_json::Value::Array(required));
            root
        }

        fn arguments_object(
            message: ::serde_json::Value,
            wrapped: bool,
            headers: Headers,
            marks: &[(&str, &str)],
            defs: Members,
        ) -> ::serde_json::Value {
            let mut root = if wrapped {
                beside_value(message, headers)
            } else {
                let mut root = members_of(message);
                if !headers.is_empty() {
                    let mut properties = match root.remove("properties") {
                        ::core::option::Option::Some(::serde_json::Value::Object(held)) => held,
                        _ => Members::new(),
                    };
                    let mut required = match root.remove("required") {
                        ::core::option::Option::Some(::serde_json::Value::Array(held)) => held,
                        _ => ::std::vec::Vec::new(),
                    };
                    for (name, schema, needed) in headers {
                        properties.insert(name.to_owned(), schema);
                        if needed {
                            required.push(text(name));
                        }
                    }
                    root.insert("properties".to_owned(), ::serde_json::Value::Object(properties));
                    root.insert("required".to_owned(), ::serde_json::Value::Array(required));
                }
                root
            };
            for (key, header) in marks {
                if let ::core::option::Option::Some(::serde_json::Value::Object(properties)) =
                    root.get_mut("properties")
                    && let ::core::option::Option::Some(::serde_json::Value::Object(property)) =
                        properties.get_mut(*key)
                {
                    property.insert("x-mcp-header".to_owned(), text(header));
                }
            }
            root.entry("type".to_owned()).or_insert_with(|| text("object"));
            defs_joined(root, defs)
        }

        fn output_object(
            success: ::serde_json::Value,
            headers: Headers,
            defs: Members,
        ) -> ::serde_json::Value {
            let root = if headers.is_empty() {
                members_of(success)
            } else {
                let mut root = beside_value(success, headers);
                root.insert("type".to_owned(), text("object"));
                root
            };
            defs_joined(root, defs)
        }

        fn defs_joined(mut root: Members, mut defs: Members) -> ::serde_json::Value {
            defs.retain(|_, described| !described.is_null());
            if !defs.is_empty() {
                root.insert("$defs".to_owned(), ::serde_json::Value::Object(defs));
            }
            ::serde_json::Value::Object(root)
        }
    }
}
