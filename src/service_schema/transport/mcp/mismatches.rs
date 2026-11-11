//! What a `tools/call` refused for its arguments answers: serde's own problem, and beside it every
//! other place the tool's input schema refuses, each at its dotted path.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// What a service's refused reads call: the walk and `listed_refusal` wherever one of them lists,
/// `refused_arguments` for its tools, `prompt_input` for a prompt of several arguments and
/// `read_input` for a resource template.
pub(super) struct Listing {
    pub prompts: bool,
    pub reads: bool,
    pub tools: bool,
}

/// The helpers `listing` asks for, none where nothing lists.
pub(super) fn helpers(module: &Ident, schema: &Ident, listing: &Listing) -> TokenStream {
    if !(listing.tools || listing.prompts || listing.reads) {
        return TokenStream::new();
    }
    let walk = walk();
    let object = object_walk();
    let listed = listed(module);
    let tools = listing.tools.then(|| refused(module, schema));
    let prompts = listing.prompts.then(|| prompt_input(schema));
    let reads = listing.reads.then(|| read_input(schema));
    quote! {
        #walk
        #object
        #listed
        #tools
        #prompts
        #reads
    }
}

/// `mismatches`, which follows a reference, an `anyOf`, an `enum` and a type.
fn walk() -> TokenStream {
    quote! {
        /// One place the arguments do not read: its dotted path, and what is wrong there.
        fn refuse(
            found: &mut ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
            at: &[::std::string::String],
            said: &str,
        ) {
            found.push((at.join("."), said.to_owned()));
        }

        /// Every place `value` does not read as `schema` says, below the steps `at`, in the order
        /// they are met. A key of `passed` is read past: it is read apart from the message.
        fn mismatches(
            schema: &::serde_json::Value,
            value: &::serde_json::Value,
            defs: &::serde_json::Value,
            passed: &[&str],
            at: &[::std::string::String],
            found: &mut ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
        ) {
            let ::serde_json::Value::Object(members) = schema else {
                return;
            };
            if let ::core::option::Option::Some(reference) =
                members.get("$ref").and_then(::serde_json::Value::as_str)
            {
                let key = reference.rsplit('/').next().unwrap_or_default();
                if let ::core::option::Option::Some(defined) = defs.get(key) {
                    mismatches(defined, value, defs, passed, at, found);
                }
                return;
            }
            if let ::core::option::Option::Some(::serde_json::Value::Array(alternatives)) =
                members.get("anyOf")
            {
                if value.is_null() {
                    return;
                }
                // A value one alternative reads has no problem; otherwise the first alternative's
                // are named, as the other servers name them.
                let mut named = ::core::option::Option::None;
                for alternative in alternatives.iter().filter(|alternative| {
                    alternative.get("type").and_then(::serde_json::Value::as_str)
                        != ::core::option::Option::Some("null")
                }) {
                    let mut own = ::std::vec::Vec::new();
                    mismatches(alternative, value, defs, passed, at, &mut own);
                    if own.is_empty() {
                        return;
                    }
                    if named.is_none() {
                        named = ::core::option::Option::Some(own);
                    }
                }
                found.extend(named.unwrap_or_default());
                return;
            }
            if let ::core::option::Option::Some(::serde_json::Value::Array(allowed)) =
                members.get("enum")
                && !allowed.contains(value)
            {
                refuse(found, at, "invalid value, expected one of the listed values");
                return;
            }
            match members.get("type").and_then(::serde_json::Value::as_str) {
                ::core::option::Option::Some("object") => {
                    object_mismatches(members, value, defs, passed, at, found);
                }
                ::core::option::Option::Some("array") => {
                    let ::serde_json::Value::Array(items) = value else {
                        refuse(found, at, "invalid type, expected an array");
                        return;
                    };
                    if let ::core::option::Option::Some(each) = members.get("items") {
                        for (index, item) in items.iter().enumerate() {
                            let below = [at, &[index.to_string()]].concat();
                            mismatches(each, item, defs, &[], &below, found);
                        }
                    }
                }
                ::core::option::Option::Some("string") if !value.is_string() => {
                    refuse(found, at, "invalid type, expected a string");
                }
                ::core::option::Option::Some("integer") if !value.is_i64() && !value.is_u64() => {
                    refuse(found, at, "invalid type, expected an integer");
                }
                ::core::option::Option::Some("number") if !value.is_number() => {
                    refuse(found, at, "invalid type, expected a number");
                }
                ::core::option::Option::Some("boolean") if !value.is_boolean() => {
                    refuse(found, at, "invalid type, expected a boolean");
                }
                _ => {}
            }
        }
    }
}

/// `object_mismatches`: an unknown key, a member that does not read, and a missing key.
fn object_walk() -> TokenStream {
    quote! {
        /// Every place the object `value` does not read as the object schema `members` says.
        fn object_mismatches(
            members: &::serde_json::Map<::std::string::String, ::serde_json::Value>,
            value: &::serde_json::Value,
            defs: &::serde_json::Value,
            passed: &[&str],
            at: &[::std::string::String],
            found: &mut ::std::vec::Vec<(::std::string::String, ::std::string::String)>,
        ) {
            let ::serde_json::Value::Object(held) = value else {
                refuse(found, at, "invalid type, expected an object");
                return;
            };
            let properties = members.get("properties");
            let others = members.get("additionalProperties");
            for (key, entry) in held {
                if passed.contains(&key.as_str()) {
                    continue;
                }
                let below = [at, ::core::slice::from_ref(key)].concat();
                match properties.and_then(|declared| declared.get(key)).or(others) {
                    ::core::option::Option::Some(::serde_json::Value::Bool(false)) => {
                        refuse(found, &below, &format!("unknown field `{key}`"));
                    }
                    ::core::option::Option::Some(read_as) => {
                        mismatches(read_as, entry, defs, &[], &below, found);
                    }
                    ::core::option::Option::None => {}
                }
            }
            let required = members
                .get("required")
                .and_then(::serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(::serde_json::Value::as_str);
            for key in required {
                if !held.contains_key(key) && !passed.contains(&key) {
                    let below = [at, &[key.to_owned()]].concat();
                    refuse(found, &below, &format!("missing field `{key}`"));
                }
            }
        }
    }
}

/// `listed_refusal`, the fault of a refused read with every problem the walk finds beside serde's.
fn listed(module: &Ident) -> TokenStream {
    quote! {
        /// The fault of a message of `operation` that serde refused, read from `source`: serde's
        /// own problem, then every other place `input` refuses, read past the keys `passed`.
        /// Called only once a read has refused, so a message that reads is never walked.
        fn listed_refusal(
            operation: &str,
            input: &::serde_json::Value,
            source: &::serde_json::Value,
            passed: &[&str],
            refusal: &Refusal,
        ) -> $crate::#module::ServiceFault {
            let first = refused_payload(operation, refusal);
            if first.kind() != $crate::#module::ServiceFaultKind::FailedValidation {
                return first;
            }
            let mut found = ::std::vec::Vec::new();
            mismatches(input, source, &input["$defs"], passed, &[], &mut found);
            let named = first.field().unwrap_or_default();
            found.retain(|(path, _said)| path != named);
            if found.is_empty() {
                return first;
            }
            let own = first
                .field()
                .and_then(|field| first.detail().strip_prefix(&format!("'{field}': ")))
                .unwrap_or_else(|| first.detail());
            let mut detail = first.detail().to_owned();
            let mut issues =
                ::std::vec![$crate::#module::ServiceFaultIssue::new(first.field(), own)];
            for (path, said) in &found {
                let field = (!path.is_empty()).then_some(path.as_str());
                detail.push_str("; ");
                if let ::core::option::Option::Some(named_at) = field {
                    detail.push('\'');
                    detail.push_str(named_at);
                    detail.push_str("': ");
                }
                detail.push_str(said);
                issues.push($crate::#module::ServiceFaultIssue::new(field, said));
            }
            $crate::#module::ServiceFault::failed_validation(operation, first.field(), &detail)
                .with_issues(issues)
        }
    }
}

/// `refused_arguments`, the fault a refused `tools/call` answers.
fn refused(module: &Ident, schema: &Ident) -> TokenStream {
    quote! {
        /// The fault of the arguments of `tool` that serde refused, walked against the tool's
        /// input schema: the message under `value` where the tool wraps it, and otherwise the
        /// arguments, read past the keys `passed`.
        fn refused_arguments(
            tool: &str,
            wrapped: bool,
            passed: &[&str],
            params: &::serde_json::Value,
            refusal: &Refusal,
        ) -> $crate::#module::ServiceFault {
            let tools = $crate::#schema::mcp_tools();
            let ::core::option::Option::Some(input) = tools
                .iter()
                .find(|listed| listed["name"] == tool)
                .map(|listed| &listed["inputSchema"])
            else {
                return refused_payload(tool, refusal);
            };
            let empty = ::serde_json::Value::Object(::serde_json::Map::new());
            let arguments = params
                .get("arguments")
                .filter(|given| given.is_object())
                .unwrap_or(&empty);
            if !wrapped {
                return listed_refusal(tool, input, arguments, passed, refusal);
            }
            let mut message = input["properties"]["value"].clone();
            if let ::serde_json::Value::Object(members) = &mut message {
                members.insert("$defs".to_owned(), input["$defs"].clone());
            }
            listed_refusal(tool, &message, &arguments["value"], &[], refusal)
        }
    }
}

/// `prompt_input`, the schema a prompt's arguments are walked with.
fn prompt_input(schema: &Ident) -> TokenStream {
    quote! {
        /// The schema the arguments of `prompt` are walked with: an object of the arguments
        /// `prompts/list` gives it, each a string, those it marks required required.
        fn prompt_input(prompt: &str) -> ::serde_json::Value {
            let mut properties = ::serde_json::Map::new();
            let mut required = ::std::vec::Vec::new();
            for listed in $crate::#schema::mcp_prompts() {
                if listed["name"] != prompt {
                    continue;
                }
                for argument in listed["arguments"].as_array().into_iter().flatten() {
                    let ::core::option::Option::Some(name) = argument["name"].as_str() else {
                        continue;
                    };
                    properties.insert(name.to_owned(), ::serde_json::json!({ "type": "string" }));
                    if argument["required"] == true {
                        required.push(::serde_json::Value::String(name.to_owned()));
                    }
                }
            }
            ::serde_json::json!({ "properties": properties, "required": required, "type": "object" })
        }
    }
}

/// `read_input`, the schema a resource template's message is walked with.
fn read_input(schema: &Ident) -> TokenStream {
    quote! {
        /// The schema the message of the resource read `operation` is walked with, `null`, which
        /// names no problem, for one `mcp_read_inputs()` does not hold.
        fn read_input(operation: &str) -> ::serde_json::Value {
            $crate::#schema::mcp_read_inputs()
                .into_iter()
                .find(|(wire, _input)| *wire == operation)
                .map(|(_wire, input)| input)
                .unwrap_or_default()
        }
    }
}
