//! The `OpenAPI` 3.1 document of an `http_rest` service, written as the body of
//! `<Service>Schema::openapi()`: each operation's `HttpShape` read into paths, parameters, bodies,
//! statuses and headers, and every type they name described once under `components.schemas`.

use crate::features::jsonschema::in_flight_type;
use crate::model_schema::{named_json_schema_modules, type_json_schema_value};
use crate::service_schema::parse::{
    BodyKind, DEFAULT_BINDING_ERROR_STATUS, HttpShape, OperationDef, OperationInputs,
    OperationOutcome, ServiceDef, error_declared_type, is_scalar_named_type, is_unit_type,
    option_inner, tuple_elements, vec_inner, wire_key, written,
};
use crate::service_schema::support::{
    fault_fields_typescript_name, message_wire_key_ident, module_ident,
};
use crate::service_schema::transport::OpenApiArguments;
use crate::utils::ident_schema_module_name;
use core::iter;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::Type;

const COMPONENTS_PREFIX: &str = "#/components/schemas/";
const JSON: &str = "application/json";
const OCTET_STREAM: &str = "application/octet-stream";
const MULTIPART: &str = "multipart/form-data";
const PARTIAL_STATUS: u16 = 206;

/// The statuses `dispatch`'s default `FaultHandler` answers a fault at, each with what it means.
const FAULTS: [(u16, &str); 3] = [
    (
        400,
        "A fault: a payload that does not decode or fails validation.",
    ),
    (404, "A fault: no route answers the request."),
    (500, "A fault: the handler panicked."),
];

/// One answer a status gives: its media type and schema, where it has a body, and its headers.
/// Answers that share a status are joined into one response.
struct Answer {
    content: Option<(&'static str, Option<TokenStream>)>,
    description: String,
    headers: Vec<Header>,
}

/// One `header_out` or `error_header_out` entry, and the schema of the element it is written from.
#[derive(Clone)]
struct Header {
    name: String,
    required: bool,
    schema: TokenStream,
}

/// What the document reads across operations: the name of every type to be written under
/// `components.schemas`, and whether an operation reads a member of a declared message's schema.
struct Named {
    module: syn::Ident,
    names: Vec<TokenStream>,
    reads_a_member: bool,
}

impl Named {
    /// `ty`'s schema as an expression of the run, with each model type it names kept for
    /// `components.schemas`.
    fn described(&mut self, subject: &str, ty: &Type) -> TokenStream {
        self.names.extend(
            named_json_schema_modules(ty)
                .into_iter()
                .map(|(module, span)| {
                    quote_spanned! {span=> #module::Schema::JSON_SCHEMA_NAME }
                }),
        );
        type_json_schema_value(subject, ty)
    }

    /// The schema of a value read from text, an `Option` read as the value it holds where present.
    fn described_text(&mut self, subject: &str, ty: &Type) -> TokenStream {
        self.described(subject, option_inner(ty).unwrap_or(ty))
    }
}

/// `<Service>Schema::openapi()`. Every model type an operation names is reserved under its name
/// in `components.schemas` before any is described, so each is written once and every reference
/// to it is a `$ref`; a reservation no definition filled is dropped.
pub fn openapi_method(service: &ServiceDef, asked: &OpenApiArguments) -> TokenStream {
    let title = asked
        .title
        .clone()
        .unwrap_or_else(|| service.ident.to_string());
    let module = module_ident(service);
    let fault_module = format_ident!(
        "{}",
        ident_schema_module_name(&fault_fields_typescript_name(&service.ident.to_string()))
    );
    let fault = quote! { #module::#fault_module };
    let mut named = Named {
        module,
        names: vec![quote! { #fault::Schema::JSON_SCHEMA_NAME }],
        reads_a_member: false,
    };
    let statements = path_statements(service, &mut named);
    let mut reserved: Vec<String> = Vec::new();
    let reservations: Vec<&TokenStream> = named
        .names
        .iter()
        .filter(|name| {
            let spelled = name.to_string();
            let first = !reserved.contains(&spelled);
            reserved.push(spelled);
            first
        })
        .collect();
    let member_schema = named.reads_a_member.then(member_schema_fn);
    let in_flight_type = in_flight_type();
    quote! {
        #[doc = " The service's OpenAPI 3.1 document: every `http_rest` operation, parameter, body,"]
        #[doc = " status and header it declares, and the schema of every type they name."]
        pub fn openapi() -> serde_json::Value {
            #member_schema
            let mut in_flight: #in_flight_type = ::std::vec::Vec::new();
            let mut components = serde_json::Map::new();
            let paths = {
                let in_flight = &mut in_flight;
                let hoisted_defs = &mut components;
                let pointer_prefix: &str = #COMPONENTS_PREFIX;
                for named in [#(#reservations),*] {
                    hoisted_defs.entry(named).or_insert(serde_json::Value::Null);
                }
                let fault = #fault::Schema::json_schema_within(in_flight, hoisted_defs, pointer_prefix);
                let mut paths = serde_json::Map::new();
                #(#statements)*
                paths
            };
            components.retain(|_, described| !described.is_null());
            serde_json::json!({
                "openapi": "3.1.0",
                "info": { "title": #title, "version": ::core::env!("CARGO_PKG_VERSION") },
                "paths": paths,
                "components": { "schemas": components },
            })
        }
    }
}

/// The schema a named message's own member is described by, read off the message's definition
/// where the message is a `$ref`, and a string where the definition names no such member.
fn member_schema_fn() -> TokenStream {
    quote! {
        fn member_schema(
            message: &serde_json::Value,
            components: &serde_json::Map<::std::string::String, serde_json::Value>,
            member: &str,
        ) -> serde_json::Value {
            let definition = message
                .get("$ref")
                .and_then(serde_json::Value::as_str)
                .and_then(|pointer| pointer.rsplit_once('/'))
                .and_then(|(_, name)| components.get(name))
                .unwrap_or(message);
            definition
                .get("properties")
                .and_then(|members| members.get(member))
                .cloned()
                .unwrap_or_else(|| serde_json::json!({ "type": "string" }))
        }
    }
}

/// One statement per path template, in the order the service first declares each: the template
/// and every method an operation answers at it.
fn path_statements(service: &ServiceDef, named: &mut Named) -> Vec<TokenStream> {
    let mut templates: Vec<(String, Vec<&OperationDef>)> = Vec::new();
    for operation in &service.operations {
        let template = HttpShape::of(operation).path_template();
        match templates.iter_mut().find(|(held, _)| *held == template) {
            Some((_, operations)) => operations.push(operation),
            None => templates.push((template, vec![operation])),
        }
    }
    templates
        .iter()
        .map(|(template, operations)| {
            let methods = operations.iter().map(|operation| {
                let method = HttpShape::of(operation).method.name().to_lowercase();
                let built = operation_value(operation, named);
                quote! { methods.insert(#method.to_owned(), #built); }
            });
            quote! {
                {
                    let mut methods = serde_json::Map::new();
                    #(#methods)*
                    paths.insert(#template.to_owned(), serde_json::Value::Object(methods));
                }
            }
        })
        .collect()
}

/// The type of the message an operation receives: the one it names, or the one the macro declared
/// for it.
pub fn message_type(operation: &OperationDef) -> Option<Type> {
    match &operation.inputs {
        OperationInputs::Named(declared) => Some(declared.as_ref().clone()),
        OperationInputs::Empty | OperationInputs::Generated(_) => operation
            .generated_message_ident()
            .map(|declared| syn::parse_quote! { #declared }),
    }
}

/// One operation's object: its `operationId`, its parameters, its request body and its responses.
fn operation_value(operation: &OperationDef, named: &mut Named) -> TokenStream {
    let shape = HttpShape::of(operation);
    let subject = operation.ident.to_string();
    let wire = &operation.wire_name;
    let placeholders = shape.placeholder_names();
    let scalar_whole = matches!(
        &operation.inputs,
        OperationInputs::Named(declared)
            if placeholders.len() == 1 && is_scalar_named_type(declared)
    );
    let reads_members = matches!(operation.inputs, OperationInputs::Named(_))
        && !placeholders.is_empty()
        && !scalar_whole;
    let multipart = matches!(shape.body_kind, BodyKind::Multipart);
    let json_body = shape.method.carries_a_body() && !multipart && !scalar_whole;
    let message_local = (json_body || reads_members)
        .then(|| message_type(operation))
        .flatten()
        .map(|message| {
            let described = named.described(&subject, &message);
            quote! { let message = #described; }
        });
    named.reads_a_member |= reads_members;

    let parameters = parameters(operation, &shape, &placeholders, scalar_whole, named);
    let parameters_insert = (!parameters.is_empty()).then(|| {
        quote! {
            operation.insert(
                "parameters".to_owned(),
                serde_json::Value::Array(::std::vec![#(#parameters),*]),
            );
        }
    });
    let request_body = if multipart {
        Some(multipart_body(operation, &shape, &placeholders, named))
    } else if json_body && message_local.is_some() {
        Some(quote! {
            serde_json::json!({ "required": true, "content": { #JSON: { "schema": message } } })
        })
    } else {
        None
    };
    let request_body_insert = request_body.map(|body| {
        quote! { operation.insert("requestBody".to_owned(), #body); }
    });

    let (error_local, answers) = answers(operation, &shape, &subject, named);
    let responses = answers.iter().map(|(status, joined)| {
        let code = status.to_string();
        let response = response_value(joined);
        quote! { responses.insert(#code.to_owned(), #response); }
    });
    quote! {
        {
            #message_local
            #error_local
            let mut operation = serde_json::Map::new();
            operation.insert("operationId".to_owned(), serde_json::Value::String(#wire.to_owned()));
            #parameters_insert
            #request_body_insert
            let mut responses = serde_json::Map::new();
            #(#responses)*
            operation.insert("responses".to_owned(), serde_json::Value::Object(responses));
            serde_json::Value::Object(operation)
        }
    }
}

/// Each path placeholder, each query field and each `header_in`, in that order.
fn parameters(
    operation: &OperationDef,
    shape: &HttpShape,
    placeholders: &[String],
    scalar_whole: bool,
    named: &mut Named,
) -> Vec<TokenStream> {
    let subject = operation.ident.to_string();
    let mut written_out = Vec::new();
    for placeholder in placeholders {
        let schema = match &operation.inputs {
            OperationInputs::Generated(fields) => fields
                .iter()
                .find(|(field, _)| written(field) == *placeholder)
                .map(|(_, ty)| named.described(&subject, ty)),
            OperationInputs::Named(declared) if scalar_whole => {
                Some(named.described(&subject, declared))
            }
            OperationInputs::Named(_) => {
                let key_of = message_wire_key_ident(operation);
                let module = &named.module;
                Some(quote! {
                    member_schema(&message, hoisted_defs, #module::#key_of(#placeholder))
                })
            }
            OperationInputs::Empty => None,
        }
        .unwrap_or_else(|| quote! { serde_json::json!({ "type": "string" }) });
        written_out.push(parameter(placeholder, "path", true, &schema, false));
    }
    let reads_query =
        !shape.method.carries_a_body() && !matches!(shape.body_kind, BodyKind::Multipart);
    if let (true, OperationInputs::Generated(fields)) = (reads_query, &operation.inputs) {
        for (field, ty) in fields {
            if placeholders.contains(&written(field)) {
                continue;
            }
            let schema = named.described_text(&subject, ty);
            let listed = vec_inner(option_inner(ty).unwrap_or(ty)).is_some();
            written_out.push(parameter(
                &wire_key(field),
                "query",
                option_inner(ty).is_none(),
                &schema,
                listed,
            ));
        }
    }
    for header in &shape.header_in {
        let schema = named.described_text(&subject, &header.ty);
        written_out.push(parameter(
            &header.name,
            "header",
            option_inner(&header.ty).is_none(),
            &schema,
            false,
        ));
    }
    written_out
}

/// One Parameter Object. A list in the query is read off one comma-separated value, which the
/// document spells `style: form` with `explode: false`.
fn parameter(
    name: &str,
    location: &str,
    required: bool,
    schema: &TokenStream,
    comma_separated: bool,
) -> TokenStream {
    let style = comma_separated.then(|| quote! { "style": "form", "explode": false, });
    quote! {
        {
            let schema = #schema;
            serde_json::json!({
                "name": #name,
                "in": #location,
                "required": #required,
                #style
                "schema": schema,
            })
        }
    }
}

/// A `body = "multipart"` request: one property per part, each field the path does not bind read
/// from a text part of its wire key, and each `part(...)` binding a file.
fn multipart_body(
    operation: &OperationDef,
    shape: &HttpShape,
    placeholders: &[String],
    named: &mut Named,
) -> TokenStream {
    let subject = operation.ident.to_string();
    let mut inserts = Vec::new();
    let mut required = Vec::new();
    if let OperationInputs::Generated(fields) = &operation.inputs {
        for (field, ty) in fields {
            if placeholders.contains(&written(field)) {
                continue;
            }
            let key = wire_key(field);
            let schema = named.described_text(&subject, ty);
            inserts.push(quote! { properties.insert(#key.to_owned(), #schema); });
            if option_inner(ty).is_none() {
                required.push(key);
            }
        }
    }
    for part in &shape.multipart_parts {
        let key = &part.name;
        inserts.push(quote! {
            properties.insert(
                #key.to_owned(),
                serde_json::json!({ "type": "string", "contentMediaType": #OCTET_STREAM }),
            );
        });
        required.push(key.clone());
    }
    quote! {
        {
            let mut properties = serde_json::Map::new();
            #(#inserts)*
            serde_json::json!({
                "required": true,
                "content": {
                    #MULTIPART: {
                        "schema": {
                            "type": "object",
                            "properties": properties,
                            "required": [#(#required),*],
                        },
                    },
                },
            })
        }
    }
}

/// Every status the operation answers at, in the order first met, with the answers that share it:
/// the success, each declared error's status, and the faults. Beside them, the statement that
/// describes the declared error once where more than one status reads it.
fn answers(
    operation: &OperationDef,
    shape: &HttpShape,
    subject: &str,
    named: &mut Named,
) -> (Option<TokenStream>, Vec<(u16, Vec<Answer>)>) {
    let mut statuses: Vec<(u16, Vec<Answer>)> = Vec::new();
    let mut error_local = None;
    match &operation.outcome {
        OperationOutcome::OneWay => answered(
            &mut statuses,
            shape.ok_status,
            Answer {
                content: None,
                description: "The operation was accepted, and answers no body.".to_owned(),
                headers: Vec::new(),
            },
        ),
        OperationOutcome::Reply { error, success } => {
            for (status, success_answer) in success_answers(shape, success, subject, named) {
                answered(&mut statuses, status, success_answer);
            }
            let head = error_declared_type(shape.error_header_out.len(), error);
            let described = named.described(subject, head);
            error_local = Some(quote! { let error = #described; });
            let headers = headers(
                &shape.error_header_out,
                &following(error, 1),
                subject,
                named,
            );
            for (status, description) in error_statuses(shape) {
                answered(
                    &mut statuses,
                    status,
                    Answer {
                        content: Some((JSON, Some(quote! { error.clone() }))),
                        description,
                        headers: headers.clone(),
                    },
                );
            }
        }
    }
    for (status, description) in FAULTS {
        answered(
            &mut statuses,
            status,
            Answer {
                content: Some((JSON, Some(quote! { fault.clone() }))),
                description: description.to_owned(),
                headers: Vec::new(),
            },
        );
    }
    (error_local, statuses)
}

/// Joins `answer` to the answers already met at `status`.
fn answered(statuses: &mut Vec<(u16, Vec<Answer>)>, status: u16, answer: Answer) {
    match statuses.iter_mut().find(|(held, _)| *held == status) {
        Some((_, joined)) => joined.push(answer),
        None => statuses.push((status, vec![answer])),
    }
}

/// The success at `ok_status`: no body for an empty reply, raw bytes for `body = "bytes"` and
/// `body = "stream"`, and the success type as JSON otherwise, with each `header_out` element. A
/// `body = "stream"` operation also answers `206`, which `StreamedAnswer::Partial` is open to on
/// every one: the same content and headers, beside a required `content-range`.
fn success_answers(
    shape: &HttpShape,
    success: &Type,
    subject: &str,
    named: &mut Named,
) -> Vec<(u16, Answer)> {
    let (content, first_header) = match shape.body_kind {
        BodyKind::Stream => (Some((OCTET_STREAM, None)), 1),
        BodyKind::Bytes => (Some((OCTET_STREAM, None)), 2),
        BodyKind::Json | BodyKind::Multipart => {
            if shape.header_out.is_empty() {
                let content = (!is_unit_type(success))
                    .then(|| (JSON, Some(named.described(subject, success))));
                (content, 1)
            } else {
                let body = tuple_elements(success)
                    .and_then(|elements| elements.first())
                    .unwrap_or(success);
                (Some((JSON, Some(named.described(subject, body)))), 1)
            }
        }
    };
    let headers = headers(
        &shape.header_out,
        &following(success, first_header),
        subject,
        named,
    );
    let description = if content.is_some() {
        "The operation's answer."
    } else {
        "The operation answered, with no body."
    };
    let partial = matches!(shape.body_kind, BodyKind::Stream).then(|| {
        let content_range = Header {
            name: "content-range".to_owned(),
            required: true,
            schema: quote! { serde_json::json!({ "type": "string" }) },
        };
        (
            PARTIAL_STATUS,
            Answer {
                content: content.clone(),
                description: "A partial body: the range the implementation answered.".to_owned(),
                headers: iter::once(content_range)
                    .chain(headers.iter().cloned())
                    .collect(),
            },
        )
    });
    let whole = Answer {
        content,
        description: description.to_owned(),
        headers,
    };
    iter::once((shape.ok_status, whole))
        .chain(partial)
        .collect()
}

/// The elements of `ty`'s tuple from `first` on: the values its declared headers are written from.
fn following(ty: &Type, first: usize) -> Vec<Type> {
    tuple_elements(ty)
        .map(|elements| elements.iter().skip(first).cloned().collect())
        .unwrap_or_default()
}

/// Each declared header beside the element it is written from, required where the element is not
/// an `Option`.
fn headers(
    declared: &[String],
    elements: &[Type],
    subject: &str,
    named: &mut Named,
) -> Vec<Header> {
    declared
        .iter()
        .zip(elements)
        .map(|(name, ty)| Header {
            name: name.clone(),
            required: option_inner(ty).is_none(),
            schema: named.described_text(subject, ty),
        })
        .collect()
}

/// Each status a declared error answers at, with the variants it answers for: the `error_status`
/// table, or every declared error at the one fixed status where the operation wrote none.
fn error_statuses(shape: &HttpShape) -> Vec<(u16, String)> {
    if shape.error_status.is_empty() {
        return vec![(DEFAULT_BINDING_ERROR_STATUS, "A declared error.".to_owned())];
    }
    let mut statuses: Vec<(u16, Vec<String>)> = Vec::new();
    for (variant, code) in &shape.error_status {
        let spelled = format!("`{variant}`");
        match statuses.iter_mut().find(|(held, _)| held == code) {
            Some((_, variants)) => variants.push(spelled),
            None => statuses.push((*code, vec![spelled])),
        }
    }
    statuses
        .into_iter()
        .map(|(code, variants)| (code, format!("A declared error: {}.", variants.join(", "))))
        .collect()
}

/// One Response Object from every answer its status gives: the descriptions in order, one entry
/// per media type whose schemas are joined under `anyOf`, and each header once.
fn response_value(joined: &[Answer]) -> TokenStream {
    let description = joined
        .iter()
        .map(|answer| answer.description.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let mut media: Vec<(&str, Vec<&TokenStream>)> = Vec::new();
    for (media_type, schema) in joined.iter().filter_map(|answer| answer.content.as_ref()) {
        if let Some((_, schemas)) = media.iter_mut().find(|(held, _)| held == media_type) {
            schemas.extend(schema);
        } else {
            media.push((media_type, schema.iter().collect()));
        }
    }
    let content = (!media.is_empty()).then(|| {
        let entries = media.iter().map(|(media_type, schemas)| {
            let value = match schemas.as_slice() {
                [] => quote! { serde_json::json!({}) },
                [only] => quote! { serde_json::json!({ "schema": (#only) }) },
                several => {
                    quote! { serde_json::json!({ "schema": { "anyOf": [#((#several)),*] } }) }
                }
            };
            quote! { content.insert(#media_type.to_owned(), #value); }
        });
        quote! {
            let mut content = serde_json::Map::new();
            #(#entries)*
            response.insert("content".to_owned(), serde_json::Value::Object(content));
        }
    });
    let mut listed: Vec<&Header> = Vec::new();
    for header in joined.iter().flat_map(|answer| &answer.headers) {
        if !listed.iter().any(|held| held.name == header.name) {
            listed.push(header);
        }
    }
    let headers = (!listed.is_empty()).then(|| {
        let entries = listed.iter().map(|header| {
            let name = &header.name;
            let required = header.required;
            let schema = &header.schema;
            quote! {
                headers.insert(#name.to_owned(), {
                    let schema = #schema;
                    serde_json::json!({ "required": #required, "schema": schema })
                });
            }
        });
        quote! {
            let mut headers = serde_json::Map::new();
            #(#entries)*
            response.insert("headers".to_owned(), serde_json::Value::Object(headers));
        }
    });
    quote! {
        {
            let mut response = serde_json::Map::new();
            response.insert("description".to_owned(), serde_json::Value::String(#description.to_owned()));
            #content
            #headers
            serde_json::Value::Object(response)
        }
    }
}
