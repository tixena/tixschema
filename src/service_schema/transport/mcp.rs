//! The `mcp` transport: two inert macros, `{service}_mcp_dispatcher!` and `{service}_mcp_client!`,
//! and nothing compiled where the service is declared.

mod client;
mod content;
mod dispatcher;
mod interview;
mod resources;

use super::Transport;
use crate::service_schema::parse::{
    BodyKind, HttpShape, OperationDef, ServiceDef, is_unit_type, tuple_elements,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

/// How a success or a declared error crosses: the value, and the headers written beside it under
/// their own names.
pub(super) struct Carried {
    pub header_names: Vec<String>,
    pub header_types: Vec<Type>,
    /// The value is the unit type and no header rides beside it.
    pub unit: bool,
    /// The types whose serialized tuple is `value`: one, or the bytes and their content type.
    pub value_types: Vec<Type>,
}

pub fn emit(service: &ServiceDef, transport: Transport) -> TokenStream {
    let dispatcher = dispatcher::macro_tokens(service, transport);
    let client = client::macro_tokens(service, transport);
    quote! {
        #dispatcher
        #client
    }
}

/// The success of a request-and-reply operation, split into its value and its headers.
pub(super) fn success_carried(operation: &OperationDef, success: &Type) -> Carried {
    let shape = HttpShape::of(operation);
    let plain = Carried {
        header_names: Vec::new(),
        header_types: Vec::new(),
        unit: is_unit_type(success),
        value_types: vec![success.clone()],
    };
    if shape.header_out.is_empty() {
        return plain;
    }
    let Some(written) = tuple_elements(success) else {
        return plain;
    };
    let elements: Vec<Type> = written.iter().cloned().collect();
    let first_header = if matches!(shape.body_kind, BodyKind::Bytes) {
        2
    } else {
        1
    };
    Carried {
        header_names: shape.header_out,
        header_types: elements.iter().skip(first_header).cloned().collect(),
        unit: false,
        value_types: elements.iter().take(first_header).cloned().collect(),
    }
}

/// The declared error of a request-and-reply operation, split into its value and its headers.
pub(super) fn error_carried(operation: &OperationDef, error: &Type) -> Carried {
    let shape = HttpShape::of(operation);
    let plain = Carried {
        header_names: Vec::new(),
        header_types: Vec::new(),
        unit: false,
        value_types: vec![error.clone()],
    };
    if shape.error_header_out.is_empty() {
        return plain;
    }
    let Some(written) = tuple_elements(error) else {
        return plain;
    };
    let elements: Vec<Type> = written.iter().cloned().collect();
    Carried {
        header_names: shape.error_header_out,
        header_types: elements.iter().skip(1).cloned().collect(),
        unit: false,
        value_types: elements.iter().take(1).cloned().collect(),
    }
}

/// `canonical`, the compact JSON text of a value with the members of every object in key order,
/// whatever order the `serde_json` of the invoking crate keeps them in.
fn canonical_json() -> TokenStream {
    quote! {
        /// `value` as compact JSON with the members of every object in key order, whatever order
        /// the `serde_json` of the invoking crate keeps them in.
        fn canonical(value: &::serde_json::Value) -> ::std::string::String {
            let mut text = ::std::string::String::new();
            write_canonical(value, &mut text);
            text
        }

        fn write_canonical(value: &::serde_json::Value, into: &mut ::std::string::String) {
            match value {
                ::serde_json::Value::Object(members) => {
                    let mut keys: ::std::vec::Vec<&::std::string::String> = members.keys().collect();
                    keys.sort();
                    into.push('{');
                    for (at, key) in keys.into_iter().enumerate() {
                        if at > 0 {
                            into.push(',');
                        }
                        into.push_str(&::serde_json::Value::String(key.clone()).to_string());
                        into.push(':');
                        write_canonical(&members[key.as_str()], into);
                    }
                    into.push('}');
                }
                ::serde_json::Value::Array(items) => {
                    into.push('[');
                    for (at, item) in items.iter().enumerate() {
                        if at > 0 {
                            into.push(',');
                        }
                        write_canonical(item, into);
                    }
                    into.push(']');
                }
                other => into.push_str(&other.to_string()),
            }
        }
    }
}
