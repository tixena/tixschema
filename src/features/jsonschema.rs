//! JSON Schema generation: `json_schema()` and `json_schema_within()` for a type that lists
//! `enable = ["jsonschema"]`.

use crate::utils::is_nonstrict;

const DEFS_PREFIX: &str = "#/$defs/";

/// The associated const an untagged enum with a member serde writes as `()` publishes on its
/// schema module's `Schema`, which a holder that flattens the enum reads.
pub const UNIT_MEMBER_RECORD: &str = "WRITES_A_UNIT_MEMBER";

/// How a merge that cannot proceed names itself in the diagnostic it raises: `subject` is the
/// frame that reads the merge, `edge` what the merged schema was reached through, and each remedy
/// the way out in the spelling that applies where that edge was written. `beside_tag` is the
/// `Enum::Variant` serde names where the merged schema is the content of a newtype variant under a
/// tag alone, and `None` where it is flattened.
pub struct MergeDiagnostic<'msg> {
    pub beside_tag: Option<&'msg str>,
    pub cycle_remedy: &'msg str,
    pub edge: &'msg str,
    pub non_object_remedy: &'msg str,
    pub subject: &'msg str,
}

/// One schema merged into a base, together with how the author named it. A merged schema is a
/// `serde_json` expression by the time it reaches the merge and no longer carries the name it came
/// from, so the label travels beside it — it is what a diagnostic points the author at. `map` says
/// serde hands it every key no member takes, where its description closes it to some, as a map
/// keyed by a plain enum's does. `entries` says serde reads it as the entries of a map: an absent
/// one, which serde reads wherever one entry does not read, leaves every key in the object.
pub struct MergedSource {
    pub entries: bool,
    pub label: String,
    pub map: bool,
    pub optional: bool,
    /// A `bool` expression: whether a `null` at the source's own level is a member over `()`,
    /// which serde writes as no key and does not read back, rather than an `Option`'s absence.
    pub unit_member: proc_macro2::TokenStream,
    pub value: proc_macro2::TokenStream,
}

/// Check if we should generate JSON schema methods.
#[cfg(test)]
pub const fn should_generate_json_schema() -> bool {
    true
}

/// One type parameter of a generic item, as that item's own schema module reads it.
pub struct SchemaParameter {
    pub binding: proc_macro2::Ident,
    pub default: proc_macro2::TokenStream,
}

/// The JSON-schema methods a schema module publishes, whose standalone documents also hold the
/// `examples` keyword, `examples` being the expression that answers its values as an
/// `Option<serde_json::Value>`, `None` leaving the keyword out. `rust_ident` is the declared
/// type's own name, which with the module it is declared in tells it apart from every other type.
pub fn json_schema_methods(
    def_name: &str,
    rust_ident: &str,
    body: &proc_macro2::TokenStream,
    parameters: &[SchemaParameter],
    examples: Option<&proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let in_flight_type = in_flight_type();
    let defs_prefix = DEFS_PREFIX;
    if parameters.is_empty() {
        let described = guarded_description(
            def_name,
            rust_ident,
            body,
            &quote::quote! { ::std::vec::Vec::new() },
            false,
        );
        let rooted = rooted_document(
            &quote::quote! { Self::json_schema_within(&mut in_flight, &mut hoisted_defs, #defs_prefix) },
            examples,
        );
        return quote::quote! {
            pub const JSON_SCHEMA_NAME: &str = #def_name;

            pub fn json_schema() -> serde_json::Value {
                #rooted
            }

            pub fn json_schema_within(
                in_flight: &mut #in_flight_type,
                hoisted_defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
                pointer_prefix: &str,
            ) -> serde_json::Value {
                #described
            }
        };
    }
    let described =
        guarded_description(def_name, rust_ident, body, &bound_filling(parameters), true);
    let rooted = rooted_document(
        &quote::quote! { Self::json_schema_within(&mut in_flight, &mut hoisted_defs, #defs_prefix) },
        examples,
    );
    let rooted_at_arguments = rooted_document(
        &quote::quote! {
            Self::json_schema_within_with(&mut in_flight, &mut hoisted_defs, #defs_prefix, args)
        },
        examples,
    );
    let declared = declared_delegation(parameters);
    let bindings = argument_bindings(parameters);
    quote::quote! {
        pub const JSON_SCHEMA_NAME: &str = #def_name;

        pub fn json_schema() -> serde_json::Value {
            #rooted
        }

        pub fn json_schema_with(args: &[serde_json::Value]) -> serde_json::Value {
            #rooted_at_arguments
        }

        pub fn json_schema_within(
            in_flight: &mut #in_flight_type,
            hoisted_defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            pointer_prefix: &str,
        ) -> serde_json::Value {
            #declared
        }

        pub fn json_schema_within_with(
            in_flight: &mut #in_flight_type,
            hoisted_defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            pointer_prefix: &str,
            args: &[serde_json::Value],
        ) -> serde_json::Value {
            #bindings
            #described
        }
    }
}

/// The names whose descriptions are still being written, as the type every `within` form carries
/// them in. Each name travels with the documents its parameters were filled with — what says which
/// body the definition the name is deferred to will hold; a name declaring no parameter carries an
/// empty list.
pub fn in_flight_type() -> proc_macro2::TokenStream {
    quote::quote! { ::std::vec::Vec<(&'static str, ::std::vec::Vec<serde_json::Value>)> }
}

/// The filling this frame is writing its body at, read back off the locals the arguments were
/// bound to — so what is compared is one document per declared parameter, whatever the caller
/// passed and however short its list was.
fn bound_filling(parameters: &[SchemaParameter]) -> proc_macro2::TokenStream {
    let bound = parameters.iter().map(|parameter| &parameter.binding);
    quote::quote! { vec![#(#bound.clone()),*] }
}

/// The description guarded against re-entering a type still being written, as the tokens the
/// `within` form's body is. A type is in flight under its declaration's path, so two types of one
/// published name are told apart, and its references to itself point at a provisional key until its
/// body settles the key: its own, or, where another body holds that key once this one is written,
/// the key and a fingerprint of its own body. `filling` is the documents this frame's parameters
/// were filled with, which the guard compares against and the hoisted key is built from. A generic
/// filled with a type in flight keeps a provisional key, and a body that holds a reference to
/// another type in flight is written under its own provisional key; each keeps a record until the
/// types it waits on settle their keys and the definition is keyed again.
fn guarded_description(
    def_name: &str,
    rust_ident: &str,
    body: &proc_macro2::TokenStream,
    filling: &proc_macro2::TokenStream,
    discriminate_by_filling: bool,
) -> proc_macro2::TokenStream {
    let refusal = refilled_cycle_refusal(def_name, discriminate_by_filling);
    let key_binding = hoisted_key(def_name, discriminate_by_filling);
    let helpers = frame_helpers(discriminate_by_filling);
    let (popped, recorded) = if discriminate_by_filling {
        (
            quote::quote! { let (_, filling) = in_flight.pop().unwrap_or_default(); },
            quote::quote! {
                if key.contains('~') {
                    let labels = primitive_labels(&filling);
                    recorded(hoisted_defs, &key, #def_name, IDENTITY, filling, labels);
                }
            },
        )
    } else {
        (
            quote::quote! { in_flight.pop(); },
            quote::quote! {
                if key.contains('~') {
                    recorded(
                        hoisted_defs,
                        &key,
                        #def_name,
                        IDENTITY,
                        ::std::vec::Vec::new(),
                        ::std::vec::Vec::new(),
                    );
                }
            },
        )
    };
    quote::quote! {
        const IDENTITY: &str = ::core::concat!(::core::module_path!(), "::", #rust_ident);
        #helpers
        let filling: ::std::vec::Vec<serde_json::Value> = #filling;
        #key_binding
        let provisional = format!("{key}~{IDENTITY}");
        if let ::core::option::Option::Some((_, in_flight_filling)) =
            in_flight.iter().find(|(named, _)| *named == IDENTITY)
        {
            #refusal
            // Reserved rather than written: the frame that put this type in flight is still
            // writing the body, and settles the key once it has one.
            hoisted_defs.entry(key).or_insert(serde_json::Value::Null);
            hoisted_defs.entry(provisional.clone()).or_insert(serde_json::Value::Null);
            return serde_json::json!({ "$ref": format!("{pointer_prefix}{provisional}") });
        }
        in_flight.push((IDENTITY, filling));
        let mut described = #body;
        #popped
        let flying: ::std::vec::Vec<&str> = in_flight.iter().map(|(named, _)| *named).collect();
        let key = if !key.contains('~')
            && waits(&described, hoisted_defs, pointer_prefix, &flying, &mut ::std::vec::Vec::new())
        {
            provisional.clone()
        } else {
            // Checked as the key stands now: another type of this name written during this frame
            // keeps the key, and this body takes the fingerprint. A text under the key is the key
            // an earlier definition there settled at, not a body.
            let taken = hoisted_defs.get(&key).is_some_and(|stored| {
                !stored.is_null()
                    && !stored.is_string()
                    && *stored != described
                    && keyed(stored) != keyed(&described)
            });
            if taken {
                let canonical = serde_json::to_string(&keyed(&described)).unwrap_or_default();
                format!("{key}.{:016x}", digest(canonical.as_bytes()))
            } else {
                key
            }
        };
        let pointer = format!("{}{key}", pointer_prefix);
        let reached = key != provisional && hoisted_defs.contains_key(&provisional);
        if reached {
            repointed(&mut described, &format!("{pointer_prefix}{provisional}"), &pointer);
        }
        hoisted_defs.insert(key.clone(), described);
        #recorded
        if reached {
            settled(hoisted_defs, pointer_prefix, &provisional, &key, &flying);
        }
        serde_json::json!({ "$ref": pointer })
    }
}

/// The statements that bind `key`, what this frame's definition is hoisted under before its body
/// is known: the published name, and for a generic type the key `filled_key` builds from its
/// filling.
fn hoisted_key(def_name: &str, discriminate_by_filling: bool) -> proc_macro2::TokenStream {
    if discriminate_by_filling {
        quote::quote! {
            let key = filled_key(#def_name, &filling, &primitive_labels(&filling));
        }
    } else {
        quote::quote! {
            let key = #def_name.to_string();
        }
    }
}

/// The functions a frame reads its references through and settles its key with, as the tokens that
/// declare them; `generic` adds the ones only a generic type's frame calls.
fn frame_helpers(generic: bool) -> proc_macro2::TokenStream {
    let settling = settling_helpers();
    let filled = generic.then(filled_helpers);
    quote::quote! {
        #settling
        #filled
        // The FNV-1a offset basis and prime, spelled out: the internals of
        // `std::hash::Hasher` may change between compilers.
        fn digest(bytes: &[u8]) -> u64 {
            let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            hash
        }
        fn repointed(schema: &mut serde_json::Value, from: &str, to: &str) {
            match schema {
                serde_json::Value::Array(items) => {
                    for item in items {
                        repointed(item, from, to);
                    }
                }
                serde_json::Value::Object(members) => {
                    for (keyword, held) in members.iter_mut() {
                        if keyword == "$ref" && held.as_str() == ::core::option::Option::Some(from) {
                            *held = serde_json::Value::String(to.to_string());
                        } else {
                            repointed(held, from, to);
                        }
                    }
                }
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::String(_) => {}
            }
        }
        // A reference to a type still in flight, or to a definition that waits on one, points at
        // `<key>~<identity>` or `<key>~<fingerprint>` until that type settles its key; bodies are
        // compared and fingerprinted with each such pointer at `<key>`.
        fn keyed(schema: &serde_json::Value) -> serde_json::Value {
            match schema {
                serde_json::Value::Array(items) => {
                    serde_json::Value::Array(items.iter().map(keyed).collect())
                }
                serde_json::Value::Object(members) => serde_json::Value::Object(
                    members
                        .iter()
                        .map(|(keyword, held)| {
                            let held = held
                                .as_str()
                                .filter(|_| keyword == "$ref")
                                .and_then(|pointer| pointer.split_once('~'))
                                .map_or_else(
                                    || keyed(held),
                                    |(pointer, _)| serde_json::Value::String(pointer.to_string()),
                                );
                            (keyword.clone(), held)
                        })
                        .collect(),
                ),
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::String(_) => schema.clone(),
            }
        }
    }
}

/// The functions a frame settles its key with, as the tokens that declare them. Beside the
/// definitions, the hoisted map holds entries no reference reaches, which `defs_settled` drops:
/// under a provisional key whose frame or definition has settled, the key it settled at; and under
/// `RECORDS`, the name, identity, filling and primitive labels of each definition keyed while a type
/// it names is in flight.
fn settling_helpers() -> proc_macro2::TokenStream {
    let keying = filled_keying();
    let relocated = relocation();
    let waiting = waiting_helpers();
    let rekeying = record_rekeying();
    quote::quote! {
        const RECORDS: &str = "~";
        #keying
        #relocated
        #waiting
        #rekeying
        // Points every reference at `provisional` to `key`, then keys again each recorded
        // definition that held a pointer just moved, until none is left to key.
        fn settled(
            defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            prefix: &str,
            provisional: &str,
            key: &str,
            flying: &[&str],
        ) {
            defs.insert(provisional.to_string(), serde_json::Value::String(key.to_string()));
            let mut pending = match defs.get_mut(RECORDS) {
                ::core::option::Option::Some(serde_json::Value::Array(records)) => ::core::mem::take(records),
                _ => ::std::vec::Vec::new(),
            };
            defs.retain(|held, _| held != RECORDS);
            let mut queue = ::std::vec::Vec::new();
            moved(defs, &mut pending, &mut queue, &format!("{prefix}{provisional}"), &format!("{prefix}{key}"));
            while !queue.is_empty() {
                let mut record = queue.remove(0);
                let ::core::option::Option::Some(held) = record["key"].as_str().map(str::to_string) else {
                    continue;
                };
                let rekeyed = rekeyed(defs, prefix, &record, flying);
                if rekeyed != held {
                    let body = defs.get(&held).cloned().unwrap_or(serde_json::Value::Null);
                    relocated(defs, &held, &rekeyed, &body);
                    for written in defs.values_mut() {
                        if written.as_str() == ::core::option::Option::Some(held.as_str()) {
                            *written = serde_json::Value::String(rekeyed.clone());
                        }
                    }
                    defs.insert(held.clone(), serde_json::Value::String(rekeyed.clone()));
                    moved(defs, &mut pending, &mut queue, &format!("{prefix}{held}"), &format!("{prefix}{rekeyed}"));
                }
                if rekeyed.contains('~') {
                    record["key"] = serde_json::Value::String(rekeyed);
                    pending.push(record);
                }
            }
            if !pending.is_empty() {
                defs.insert(RECORDS.to_string(), serde_json::Value::Array(pending));
            }
        }
        fn recorded(
            defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            key: &str,
            name: &str,
            identity: &str,
            filling: ::std::vec::Vec<serde_json::Value>,
            labels: ::std::vec::Vec<::core::option::Option<::std::string::String>>,
        ) {
            let record = serde_json::json!({
                "filling": filling,
                "identity": identity,
                "key": key,
                "labels": labels,
                "name": name,
            });
            if let ::core::option::Option::Some(serde_json::Value::Array(records)) = defs.get_mut(RECORDS) {
                records.push(record);
            } else {
                defs.insert(RECORDS.to_string(), serde_json::Value::Array(vec![record]));
            }
        }
    }
}

/// The functions that say what a definition waits on, as the tokens that declare them: `waits`
/// says whether a schema reaches the provisional key of a type in `flying`, the types in flight
/// around the frame, directly or through a definition keyed provisionally, `seen` holding the keys
/// already read; `holds` whether it holds a reference to `pointer`; and `moved` points every
/// reference at `from` to `to` and queues each pending record whose filling or body held it.
fn waiting_helpers() -> proc_macro2::TokenStream {
    quote::quote! {
        fn waits(
            schema: &serde_json::Value,
            defs: &serde_json::Map<::std::string::String, serde_json::Value>,
            prefix: &str,
            flying: &[&str],
            seen: &mut ::std::vec::Vec<::std::string::String>,
        ) -> bool {
            match schema {
                serde_json::Value::Array(items) => {
                    items.iter().any(|item| waits(item, defs, prefix, flying, seen))
                }
                serde_json::Value::Object(members) => members.iter().any(|(keyword, held)| {
                    if keyword != "$ref" {
                        return waits(held, defs, prefix, flying, seen);
                    }
                    let ::core::option::Option::Some(key) = held
                        .as_str()
                        .and_then(|pointer| pointer.strip_prefix(prefix))
                        .filter(|key| key.contains('~'))
                    else {
                        return false;
                    };
                    if flying.iter().any(|identity| {
                        key.strip_suffix(identity).is_some_and(|head| head.ends_with('~'))
                    }) {
                        return true;
                    }
                    if seen.iter().any(|visited| visited == key) {
                        return false;
                    }
                    seen.push(key.to_string());
                    defs.get(key).is_some_and(|body| waits(body, defs, prefix, flying, seen))
                }),
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::String(_) => false,
            }
        }
        fn holds(schema: &serde_json::Value, pointer: &str) -> bool {
            match schema {
                serde_json::Value::Array(items) => items.iter().any(|item| holds(item, pointer)),
                serde_json::Value::Object(members) => members.iter().any(|(keyword, held)| {
                    if keyword == "$ref" {
                        held.as_str() == ::core::option::Option::Some(pointer)
                    } else {
                        holds(held, pointer)
                    }
                }),
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::String(_) => false,
            }
        }
        fn moved(
            defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            pending: &mut ::std::vec::Vec<serde_json::Value>,
            queue: &mut ::std::vec::Vec<serde_json::Value>,
            from: &str,
            to: &str,
        ) {
            for record in ::core::mem::take(pending) {
                let body = record["key"].as_str().and_then(|held| defs.get(held));
                if holds(&record["filling"], from) || body.is_some_and(|body| holds(body, from)) {
                    queue.push(record);
                } else {
                    pending.push(record);
                }
            }
            for written in defs.values_mut() {
                repointed(written, from, to);
            }
            for record in pending.iter_mut().chain(queue.iter_mut()) {
                repointed(&mut record["filling"], from, to);
            }
        }
    }
}

/// The function that keys a recorded definition again, as the tokens that declare it:
/// `rekeyed(defs, prefix, record, flying)` answers the key its filling gives it, which holds `~`
/// while the filling names a type in flight; while its body still waits on a type in `flying`, that
/// key, `~` and its identity; and otherwise the key or, where another body holds it, the key and a
/// fingerprint.
fn record_rekeying() -> proc_macro2::TokenStream {
    quote::quote! {
        fn rekeyed(
            defs: &serde_json::Map<::std::string::String, serde_json::Value>,
            prefix: &str,
            record: &serde_json::Value,
            flying: &[&str],
        ) -> ::std::string::String {
            let (
                ::core::option::Option::Some(held),
                ::core::option::Option::Some(name),
                ::core::option::Option::Some(identity),
                ::core::option::Option::Some(filling),
            ) = (
                record["key"].as_str(),
                record["name"].as_str(),
                record["identity"].as_str(),
                record["filling"].as_array(),
            )
            else {
                return record["key"].as_str().unwrap_or_default().to_string();
            };
            let labels: ::std::vec::Vec<::core::option::Option<::std::string::String>> = record["labels"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|label| label.as_str().map(str::to_string))
                .collect();
            // A type that declares no parameter is recorded with no filling.
            let key = if filling.is_empty() {
                name.to_string()
            } else {
                filled_key(name, filling, &labels)
            };
            if key.contains('~') {
                return key;
            }
            let body = defs.get(held).cloned().unwrap_or(serde_json::Value::Null);
            if waits(&body, defs, prefix, flying, &mut vec![held.to_string()]) {
                return format!("{key}~{identity}");
            }
            let taken = defs.get(&key).is_some_and(|stored| {
                !stored.is_null() && !stored.is_string() && *stored != body && keyed(stored) != keyed(&body)
            });
            if taken {
                let canonical = serde_json::to_string(&keyed(&body)).unwrap_or_default();
                format!("{key}.{:016x}", digest(canonical.as_bytes()))
            } else {
                key
            }
        }
    }
}

/// The function a generic type's key is built from its filling with, as the tokens that declare
/// it: `filled_key(name, filling, primitive_labels)` writes the published name, the labels and a
/// fingerprint of the filling read with each provisional pointer at its key, and, while a pointer is
/// still provisional, `~` and a fingerprint of the filling as it is. An argument that is a
/// reference is labelled by the key it points at, and any other by the label `primitive_labels`
/// holds at its position.
fn filled_keying() -> proc_macro2::TokenStream {
    quote::quote! {
        fn filled_key(
            name: &str,
            filling: &[serde_json::Value],
            primitive_labels: &[::core::option::Option<::std::string::String>],
        ) -> ::std::string::String {
            let read: ::std::vec::Vec<serde_json::Value> = filling.iter().map(keyed).collect();
            let canonical = serde_json::to_string(&read).unwrap_or_default();
            let fingerprint = digest(canonical.as_bytes());
            // An argument's own recognizable name: another hoisted key (off its `$ref`), or a
            // primitive's label.
            let labels: ::std::vec::Vec<::std::string::String> = read
                .iter()
                .zip(primitive_labels)
                .filter_map(|(argument, primitive)| {
                    argument.get("$ref").and_then(serde_json::Value::as_str).map_or_else(
                        || primitive.clone(),
                        |reference| reference.rsplit('/').next().map(str::to_string),
                    )
                })
                .collect();
            let key = if labels.is_empty() {
                format!("{name}.{fingerprint:016x}")
            } else {
                format!("{name}.{}-{fingerprint:016x}", labels.join("_"))
            };
            if read.as_slice() == filling {
                return key;
            }
            let held = serde_json::to_string(filling).unwrap_or_default();
            format!("{key}~{:016x}", digest(held.as_bytes()))
        }
    }
}

/// The function that moves a definition to another key, as the tokens that declare it:
/// `relocated(defs, held, key, body)` writes `body` where `held` stood, or over `key` where `key`
/// already stands, so every other definition keeps its place.
fn relocation() -> proc_macro2::TokenStream {
    quote::quote! {
        fn relocated(
            defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            held: &str,
            key: &str,
            body: &serde_json::Value,
        ) {
            let standing = defs.contains_key(key);
            for (written_at, written) in ::core::mem::take(defs) {
                if written_at == held {
                    if !standing {
                        defs.insert(key.to_string(), body.clone());
                    }
                } else if written_at == key {
                    defs.insert(written_at, body.clone());
                } else {
                    defs.insert(written_at, written);
                }
            }
        }
    }
}

/// The two functions only a generic type's frame calls, as the tokens that declare them:
/// `primitive_labels` reads the `"type"` keyword of each argument, which labels one that is no
/// reference, and `identified` reads each pointer at a type that settled before in this document at
/// the key it settled at, so two fillings are compared by the types they name.
fn filled_helpers() -> proc_macro2::TokenStream {
    quote::quote! {
        fn primitive_labels(
            filling: &[serde_json::Value],
        ) -> ::std::vec::Vec<::core::option::Option<::std::string::String>> {
            filling
                .iter()
                .map(|argument| argument.get("type").and_then(serde_json::Value::as_str).map(str::to_string))
                .collect()
        }
        fn identified(
            filling: &[serde_json::Value],
            defs: &serde_json::Map<::std::string::String, serde_json::Value>,
            prefix: &str,
        ) -> ::std::vec::Vec<serde_json::Value> {
            fn read(
                schema: &serde_json::Value,
                defs: &serde_json::Map<::std::string::String, serde_json::Value>,
                prefix: &str,
            ) -> serde_json::Value {
                match schema {
                    serde_json::Value::Array(items) => serde_json::Value::Array(
                        items.iter().map(|item| read(item, defs, prefix)).collect(),
                    ),
                    serde_json::Value::Object(members) => serde_json::Value::Object(
                        members
                            .iter()
                            .map(|(keyword, held)| {
                                let marked = held
                                    .as_str()
                                    .filter(|_| keyword == "$ref")
                                    .and_then(|pointer| pointer.strip_prefix(prefix))
                                    .and_then(|key| defs.get(key))
                                    .and_then(serde_json::Value::as_str)
                                    .map(|key| serde_json::Value::String(format!("{prefix}{key}")));
                                (keyword.clone(), marked.unwrap_or_else(|| read(held, defs, prefix)))
                            })
                            .collect(),
                    ),
                    serde_json::Value::Null
                    | serde_json::Value::Bool(_)
                    | serde_json::Value::Number(_)
                    | serde_json::Value::String(_) => schema.clone(),
                }
            }
            filling.iter().map(|argument| read(argument, defs, prefix)).collect()
        }
    }
}

/// The function a finished document's definitions are settled by, as the tokens that declare it.
/// `defs_settled(roots, defs, prefix, kept)` drops each definition nothing reaches from `roots` or
/// from `kept`, and writes back in place, beside whatever keywords its reference carried, each one
/// referenced once that `kept` does not name. A cycle keeps the definition it is entered at, which
/// is referenced from outside the cycle and from inside it.
pub fn settled_definitions() -> proc_macro2::TokenStream {
    let walkers = reference_walkers();
    quote::quote! {
        fn defs_settled(
            roots: &mut [&mut serde_json::Value],
            defs: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            prefix: &str,
            kept: &[&str],
        ) {
            #walkers
            let mut outside: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
            for root in roots.iter() {
                let mut found = ::std::vec::Vec::new();
                pointed(root, prefix, &mut found);
                outside.extend(found.into_iter().map(str::to_string));
            }
            defs.retain(|_, body| !body.is_null());
            let mut reached: ::std::collections::BTreeSet<::std::string::String> =
                ::std::collections::BTreeSet::new();
            let mut pending: ::std::vec::Vec<::std::string::String> = kept
                .iter()
                .map(|key| (*key).to_string())
                .chain(outside.iter().cloned())
                .collect();
            while let ::core::option::Option::Some(key) = pending.pop() {
                let ::core::option::Option::Some(body) =
                    defs.get(&key).filter(|_| !reached.contains(&key))
                else {
                    continue;
                };
                let mut found = ::std::vec::Vec::new();
                pointed(body, prefix, &mut found);
                pending.extend(found.into_iter().map(str::to_string));
                reached.insert(key);
            }
            defs.retain(|key, _| reached.contains(key));

            let mut counts: ::std::collections::BTreeMap<::std::string::String, usize> =
                ::std::collections::BTreeMap::new();
            for key in outside {
                *counts.entry(key).or_insert(0) += 1;
            }
            for body in defs.values() {
                let mut found = ::std::vec::Vec::new();
                pointed(body, prefix, &mut found);
                for key in found {
                    *counts.entry(key.to_string()).or_insert(0) += 1;
                }
            }
            let mut bodies = serde_json::Map::new();
            for (key, body) in ::core::mem::take(defs) {
                if counts.get(&key) == ::core::option::Option::Some(&1) && !kept.contains(&key.as_str()) {
                    bodies.insert(key, body);
                } else {
                    defs.insert(key, body);
                }
            }
            for root in roots.iter_mut() {
                placed(root, &mut bodies, prefix);
            }
            for body in defs.values_mut() {
                placed(body, &mut bodies, prefix);
            }
        }
    }
}

/// The two walks `defs_settled` makes, as the tokens that declare them: `pointed` finds the keys
/// a schema points at, and `placed` writes a definition in place of its one reference.
fn reference_walkers() -> proc_macro2::TokenStream {
    quote::quote! {
        // The keys `schema` points at, one per reference; the keywords that hold values
        // rather than schemas are not read.
        fn pointed<'schema>(
            schema: &'schema serde_json::Value,
            prefix: &str,
            found: &mut ::std::vec::Vec<&'schema str>,
        ) {
            match schema {
                serde_json::Value::Array(items) => {
                    for item in items {
                        pointed(item, prefix, found);
                    }
                }
                serde_json::Value::Object(members) => {
                    for (keyword, held) in members {
                        match keyword.as_str() {
                            "$ref" => found.extend(
                                held.as_str().and_then(|pointer| pointer.strip_prefix(prefix)),
                            ),
                            "const" | "default" | "enum" | "examples" => {}
                            "properties" | "patternProperties" | "dependentSchemas" => {
                                for named in held.as_object().into_iter().flat_map(serde_json::Map::values) {
                                    pointed(named, prefix, found);
                                }
                            }
                            _ => pointed(held, prefix, found),
                        }
                    }
                }
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::String(_) => {}
            }
        }

        fn placed(
            schema: &mut serde_json::Value,
            bodies: &mut serde_json::Map<::std::string::String, serde_json::Value>,
            prefix: &str,
        ) {
            let body = schema
                .get("$ref")
                .and_then(serde_json::Value::as_str)
                .and_then(|pointer| pointer.strip_prefix(prefix))
                .and_then(|key| bodies.remove(key));
            if let ::core::option::Option::Some(mut body) = body {
                placed(&mut body, bodies, prefix);
                if let (serde_json::Value::Object(written), serde_json::Value::Object(beside)) =
                    (&mut body, &mut *schema)
                {
                    for (keyword, held) in ::core::mem::take(beside) {
                        if keyword != "$ref" {
                            written.insert(keyword, held);
                        }
                    }
                }
                *schema = body;
                return;
            }
            match schema {
                serde_json::Value::Array(items) => {
                    for item in items {
                        placed(item, bodies, prefix);
                    }
                }
                serde_json::Value::Object(members) => {
                    for (keyword, held) in members.iter_mut() {
                        match keyword.as_str() {
                            "const" | "default" | "enum" | "examples" => {}
                            "properties" | "patternProperties" | "dependentSchemas" => {
                                if let ::core::option::Option::Some(named) = held.as_object_mut() {
                                    for member in named.values_mut() {
                                        placed(member, bodies, prefix);
                                    }
                                }
                            }
                            _ => placed(held, bodies, prefix),
                        }
                    }
                }
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::String(_) => {}
            }
        }
    }
}

/// How a reference coming back around to a name at another filling refuses, as the tokens the
/// guard raises it with. A generic type's fillings are compared by the types they name, and one
/// that names the same types points at the frame in flight, keyed off that frame's own filling.
fn refilled_cycle_refusal(def_name: &str, generic: bool) -> proc_macro2::TokenStream {
    let message = format!(
        "`{def_name}`: a reference closes a cycle at a filling the document is not being written \
         at — in flight at {{}}, and this reference names {{}}; a document holds one definition per \
         name, so a cycle cannot change filling partway through it; write the reference at the \
         filling already in flight, or key the definitions by name and filling so each filling gets \
         a definition of its own."
    );
    if !generic {
        return quote::quote! {
            if *in_flight_filling != filling {
                panic!(
                    #message,
                    serde_json::Value::Array(in_flight_filling.clone()),
                    serde_json::Value::Array(filling),
                );
            }
        };
    }
    quote::quote! {
        let key = filled_key(#def_name, in_flight_filling, &primitive_labels(in_flight_filling));
        let provisional = format!("{key}~{IDENTITY}");
        let in_flight_filling = identified(in_flight_filling, hoisted_defs, pointer_prefix);
        let filling = identified(&filling, hoisted_defs, pointer_prefix);
        if in_flight_filling != filling {
            panic!(
                #message,
                serde_json::Value::Array(in_flight_filling),
                serde_json::Value::Array(filling),
            );
        }
    }
}

/// A document standing on its own: `within` run against a fresh document, its definitions settled
/// and whatever stays joined to the root, and the `examples` keyword where the type has any.
fn rooted_document(
    described: &proc_macro2::TokenStream,
    examples: Option<&proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let in_flight_type = in_flight_type();
    let settled = settled_definitions();
    let defs_prefix = DEFS_PREFIX;
    let example_keyword = examples.map(|values| {
        quote::quote! {
            if let (
                serde_json::Value::Object(members),
                ::core::option::Option::Some(values),
            ) = (&mut described, #values)
            {
                members.insert("examples".to_string(), values);
            }
        }
    });
    quote::quote! {
        #settled
        let mut in_flight: #in_flight_type = ::std::vec::Vec::new();
        let mut hoisted_defs = serde_json::Map::new();
        let mut described = #described;
        defs_settled(&mut [&mut described], &mut hoisted_defs, #defs_prefix, &[]);
        #example_keyword
        if hoisted_defs.is_empty() {
            return described;
        }
        // The pointers into them are from the root, so the definitions join it — ahead of the
        // description, which is the rest of the document.
        let mut rooted = serde_json::Map::new();
        rooted.insert("$defs".to_string(), serde_json::Value::Object(hoisted_defs));
        if let serde_json::Value::Object(members) = described {
            rooted.extend(members);
        }
        serde_json::Value::Object(rooted)
    }
}

/// The filling an item declared for itself, handed on to the form that takes fillings — the body
/// of the argumentless `within`, which is the one frame that owns the recursion state.
fn declared_delegation(parameters: &[SchemaParameter]) -> proc_macro2::TokenStream {
    let declared = parameters.iter().map(|parameter| &parameter.default);
    quote::quote! {
        let arguments = [#(#declared),*];
        Self::json_schema_within_with(in_flight, hoisted_defs, pointer_prefix, &arguments)
    }
}

/// The locals the body reads its parameters through, bound off the argument list positionally.
fn argument_bindings(parameters: &[SchemaParameter]) -> proc_macro2::TokenStream {
    let bound = parameters.iter().enumerate().map(|(position, parameter)| {
        let binding = &parameter.binding;
        quote::quote! {
            let #binding: serde_json::Value = args
                .get(#position)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
        }
    });
    quote::quote! { #(#bound)* }
}

/// When the struct has `#[serde(flatten)]` fields, the base properties are distributed into each
/// branch of the flattened types' schemas (cross-product over any `oneOf`), producing a strict
/// closed schema that validates the base fields and the flattened union together.
pub fn generate_struct_json_schema_method(
    json_schema_fields: &[proc_macro2::TokenStream],
    flatten_json_schemas: &[MergedSource],
    def_name: &str,
    rust_ident: &str,
    parameters: &[SchemaParameter],
    examples: Option<&proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let body = if flatten_json_schemas.is_empty() {
        closed_object_body(json_schema_fields)
    } else {
        flattened_object_body(json_schema_fields, flatten_json_schemas, def_name)
    };
    json_schema_methods(def_name, rust_ident, &body, parameters, examples)
}

/// The struct's own fields as one closed object.
fn closed_object_body(json_schema_fields: &[proc_macro2::TokenStream]) -> proc_macro2::TokenStream {
    let open = is_nonstrict();
    quote::quote! {
        {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            schema_obj.insert("additionalProperties".to_string(), serde_json::Value::Bool(#open));
            let mut properties = serde_json::Map::new();
            let mut required = ::std::vec::Vec::new();

            #(#json_schema_fields)*

            schema_obj.insert(
                "properties".to_string(),
                serde_json::Value::Object(properties),
            );

            schema_obj.insert("required".to_string(), serde_json::Value::Array(required));

            serde_json::Value::Object(schema_obj)
        }
    }
}

/// The four things the merge asks of a schema it is handed, as the tokens the merging block opens
/// with: how two objects join, whether a schema is a deferred name, what type it commits its value
/// to, and which branches it offers under which spelling.
fn merge_readers() -> proc_macro2::TokenStream {
    let open = is_nonstrict();
    quote::quote! {
        fn merge_object_schemas(
            a: &serde_json::Map<::std::string::String, serde_json::Value>,
            b: &serde_json::Map<::std::string::String, serde_json::Value>,
        ) -> serde_json::Map<::std::string::String, serde_json::Value> {
            let mut out = serde_json::Map::new();
            out.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            let mut properties = serde_json::Map::new();
            for src in [a, b] {
                if let ::core::option::Option::Some(p) = src.get("properties").and_then(serde_json::Value::as_object) {
                    for (k, v) in p {
                        properties.insert(k.clone(), v.clone());
                    }
                }
            }
            out.insert("properties".to_string(), serde_json::Value::Object(properties));
            let mut required: ::std::vec::Vec<serde_json::Value> = ::std::vec::Vec::new();
            for src in [a, b] {
                if let ::core::option::Option::Some(r) = src.get("required").and_then(serde_json::Value::as_array) {
                    for item in r {
                        if !required.contains(item) {
                            required.push(item.clone());
                        }
                    }
                }
            }
            out.insert("required".to_string(), serde_json::Value::Array(required));
            out.insert("additionalProperties".to_string(), serde_json::Value::Bool(#open));
            out
        }

        // A merged schema that names itself is a reference into the definitions being hoisted, keyed
        // by what follows the pointer's last `/` whichever prefix the run writes: the frame that
        // deferred the name fills the entry in before it returns.
        fn deferred_name(schema: &serde_json::Value) -> ::core::option::Option<&str> {
            schema.get("$ref")?.as_str()?.rsplit_once('/').map(|(_, name)| name)
        }

        // What a description commits its value to on the wire, when it commits to anything.
        fn described_type(schema: &serde_json::Value) -> ::core::option::Option<&str> {
            schema.get("type")?.as_str()
        }

        // What serde picked one of: `oneOf` for a discriminated enum, `anyOf` for an untagged one.
        // `None` means the schema offers no choice.
        fn union_branches(schema: &serde_json::Value) -> ::core::option::Option<(&'static str, &[serde_json::Value])> {
            for keyword in ["oneOf", "anyOf"] {
                if let ::core::option::Option::Some(union) = schema.get(keyword).and_then(serde_json::Value::as_array) {
                    return ::core::option::Option::Some((keyword, union.as_slice()));
                }
            }
            ::core::option::Option::None
        }

        // The variant name a branch of an externally tagged enum's `oneOf` pins, or `None` for
        // every branch that pins none.
        fn tagged_unit_variant(schema: &serde_json::Value) -> ::core::option::Option<&str> {
            (described_type(schema)? == "string").then_some(())?;
            schema.get("const")?.as_str()
        }
    }
}

/// How a merged object holds the keys serde hands the map merged into it, as the tokens that declare
/// it: every key the object does not name is held to the map's values.
fn map_handing() -> proc_macro2::TokenStream {
    quote::quote! {
        // The maps merged so far, and `members` among them where it is a map or admits keys
        // beyond its own: serde hands such a source every key no member takes. Each is its
        // values, and the keys it names where it names some. A nonstrict struct, which names
        // members and admits other keys in its own object, is handed none.
        fn handed_beside(
            handed: &[serde_json::Value],
            members: &serde_json::Map<::std::string::String, serde_json::Value>,
            map: bool,
        ) -> ::std::vec::Vec<serde_json::Value> {
            let mut maps = handed.to_vec();
            if let ::core::option::Option::Some(value) = members
                .get("additionalProperties")
                .filter(|value| {
                    map || (value.as_bool() != ::core::option::Option::Some(false)
                        && !(value.as_bool() == ::core::option::Option::Some(true)
                            && members.contains_key("properties")))
                })
            {
                let mut held = serde_json::Map::new();
                held.insert("additionalProperties".to_string(), value.clone());
                if let ::core::option::Option::Some(names) = members.get("propertyNames") {
                    held.insert("propertyNames".to_string(), names.clone());
                }
                maps.push(serde_json::Value::Object(held));
            }
            maps
        }

        fn handed_out(
            mut members: serde_json::Map<::std::string::String, serde_json::Value>,
            handed: &[serde_json::Value],
        ) -> serde_json::Map<::std::string::String, serde_json::Value> {
            // A map flattened beside another member or a tag is refused, so the object holds one.
            if let ::core::option::Option::Some(first) =
                handed.first().and_then(serde_json::Value::as_object)
            {
                for (keyword, described) in first {
                    members.insert(keyword.clone(), described.clone());
                }
            }
            members
        }
    }
}

/// What a merged source offers where serde reads it as absent, as the tokens that declare it.
fn absences() -> proc_macro2::TokenStream {
    quote::quote! {
        impl Branches<'_> {
            // What an `Option` makes of what it wraps: one object cannot say a group of keys is
            // written together or not at all, so the choice is an `anyOf` between two key sets.
            fn or_absent(self) -> Self {
                Self::Union("anyOf", vec![self, Self::Absent])
            }

            // What the absence of a source serde reads as the entries of a map leaves: the keys.
            fn entries_left(self) -> Self {
                match self {
                    Self::Absent => Self::Dropped,
                    Self::Union(spelling, branches) => Self::Union(
                        spelling,
                        branches.into_iter().map(Self::entries_left).collect(),
                    ),
                    other => other,
                }
            }
        }

        // One base with no members merged in, and every key it was handed held to nothing.
        fn dropped_into(
            base: &serde_json::Map<::std::string::String, serde_json::Value>,
            handed: &[serde_json::Value],
        ) -> Merged {
            let mut maps = handed.to_vec();
            maps.push(serde_json::json!({ "additionalProperties": true }));
            Merged::Object(merge_object_schemas(base, &serde_json::Map::new()), maps)
        }
    }
}

/// What the merge holds while it multiplies, as the tokens that declare it.
fn merged_tree() -> proc_macro2::TokenStream {
    let handing = map_handing();
    let absences = absences();
    quote::quote! {
        #handing
        #absences

        enum Branches<'defs> {
            // What a source contributes when it is not there, through an `Option` or an item whose
            // surface offers a `null`: no members.
            Absent,
            // What a map contributes when serde reads it as absent: no members, and every key it
            // was handed, with any value, which serde drops with the map.
            Dropped,
            Object(&'defs serde_json::Map<::std::string::String, serde_json::Value>),
            // An externally tagged enum's unit variant, which the description pins as the bare name
            // serde writes for it standing alone.
            Tagged(&'defs str),
            Union(&'static str, ::std::vec::Vec<Branches<'defs>>),
        }

        enum Merged {
            Object(serde_json::Map<::std::string::String, serde_json::Value>, ::std::vec::Vec<serde_json::Value>),
            Union(&'static str, ::std::vec::Vec<Merged>),
        }

        impl Branches<'_> {
            // What one base becomes once this source's choices are written into it: every leaf
            // contributes its members to a copy of the base.
            fn merged_into(
                &self,
                base: &serde_json::Map<::std::string::String, serde_json::Value>,
                handed: &[serde_json::Value],
                map: bool,
            ) -> Merged {
                match *self {
                    // Merged, not copied: an absent source contributes no members, and the branch
                    // keeps the base's keys in the base's order.
                    Self::Absent => Merged::Object(
                        merge_object_schemas(base, &serde_json::Map::new()),
                        handed.to_vec(),
                    ),
                    Self::Dropped => dropped_into(base, handed),
                    Self::Object(members) => Merged::Object(
                        merge_object_schemas(base, members),
                        handed_beside(handed, members, map),
                    ),
                    Self::Tagged(name) => {
                        let mut properties = serde_json::Map::new();
                        properties
                            .insert(name.to_string(), serde_json::json!({ "type": "null" }));
                        let mut written = serde_json::Map::new();
                        written.insert(
                            "properties".to_string(),
                            serde_json::Value::Object(properties),
                        );
                        written.insert(
                            "required".to_string(),
                            serde_json::Value::Array(vec![serde_json::Value::String(
                                name.to_string(),
                            )]),
                        );
                        Merged::Object(merge_object_schemas(base, &written), handed.to_vec())
                    }
                    Self::Union(spelling, ref branches) => {
                        let mut merged: ::std::vec::Vec<Merged> = branches
                            .iter()
                            .map(|branch| branch.merged_into(base, handed, map))
                            .collect();
                        // One key set is an object rather than a choice between objects, so a level
                        // offering a single branch writes no wrapper and its spelling goes unread.
                        if merged.len() == 1 {
                            merged.swap_remove(0)
                        } else {
                            Merged::Union(spelling, merged)
                        }
                    }
                }
            }
        }

        impl Merged {
            // Every leaf gains the members of one leaf of the source, so a source reaches the
            // branches an earlier source left behind rather than only the object it started from.
            fn multiplied(self, source: &Branches<'_>, map: bool) -> Self {
                match self {
                    Self::Union(keyword, branches) => Self::Union(
                        keyword,
                        branches
                            .into_iter()
                            .map(|branch| branch.multiplied(source, map))
                            .collect(),
                    ),
                    Self::Object(base, handed) => source.merged_into(&base, &handed, map),
                }
            }

            fn into_value(self) -> serde_json::Value {
                match self {
                    Self::Object(members, handed) => {
                        serde_json::Value::Object(handed_out(members, &handed))
                    }
                    Self::Union(keyword, branches) => {
                        let mut out = serde_json::Map::new();
                        out.insert(
                            "type".to_string(),
                            serde_json::Value::String("object".to_string()),
                        );
                        out.insert(
                            keyword.to_string(),
                            serde_json::Value::Array(
                                branches.into_iter().map(Self::into_value).collect(),
                            ),
                        );
                        serde_json::Value::Object(out)
                    }
                }
            }
        }
    }
}

/// How the expansion refuses a schema it cannot merge, as the tokens that declare the refusals.
fn expansion_refusals(diagnostic: &MergeDiagnostic<'_>) -> proc_macro2::TokenStream {
    let MergeDiagnostic {
        beside_tag,
        cycle_remedy,
        edge,
        non_object_remedy,
        subject,
    } = *diagnostic;
    let (named_written, unit_written) = if beside_tag.is_some() {
        (
            "beside the tag as a key holding null, which a schema closed around the tag rejects",
            "beside the tag serde writes the variant as the tag alone, and reads it back, while \
             the TypeScript type and the Zod schema of the variant intersect the tag's object \
             with `null`, which admits no value",
        )
    } else {
        (
            "as a key holding null, which a schema closed around the remaining fields rejects",
            "serde writes no key for it, and reads the object back, while the TypeScript type and \
             the Zod schema intersect the object with `null`, which admits no value",
        )
    };
    let null_refusal = null_refusal(diagnostic);
    quote::quote! {
        #null_refusal

        // `whose` names whose variant name the schema describes: a plain enum's, or the unit
        // variant's of an externally tagged enum a union member holds.
        fn refuse_variant_name(label: &str, position: &[usize], whose: &str) -> ! {
            if position.is_empty() {
                panic!(
                    "`{}`: {} `{}` is written as {} — its schema describes a `string`, and serde writes that name {}; {}",
                    #subject, #edge, label, whose, #named_written, #non_object_remedy,
                );
            }
            panic!(
                "`{}`: {} `{}` writes a union member as {} — its branch {} describes a `string`, and serde writes that name {}; {}",
                #subject, #edge, label, whose, branch_path(position), #named_written, #non_object_remedy,
            );
        }

        fn refuse_unit(label: &str) -> ! {
            panic!(
                "`{}`: {} `{}` is written as `()` — its schema describes a `null`: {}; {}",
                #subject, #edge, label, #unit_written, #non_object_remedy,
            );
        }

        fn branch_path(position: &[usize]) -> ::std::string::String {
            position
                .iter()
                .map(usize::to_string)
                .collect::<::std::vec::Vec<::std::string::String>>()
                .join(".")
        }

        // An entry still only reserved is this merge coming back around to a name whose body is
        // still being written: there is nothing to merge. That frame's references to itself point
        // at `<key>~<identity>`, and the message names the key.
        fn refuse_missing_body(label: &str, position: &[usize], name: &str) -> ! {
            let name = name.split_once('~').map_or(name, |(key, _)| key);
            if position.is_empty() {
                panic!(
                    "`{}`: {} `{}` closes a flatten cycle — the flattened body does not exist to merge, and no finite value inhabits the type; {}",
                    #subject, #edge, name, #cycle_remedy,
                );
            }
            panic!(
                "`{}`: {} `{}` closes a flatten cycle through a union member — its branch {} is `{}`, whose body does not exist to merge, and no finite value inhabits the type; {}",
                #subject, #edge, label, branch_path(position), name, #cycle_remedy,
            );
        }

        // A name whose body does exist but stands on the path the expansion is already descending:
        // the same cycle, closed through unions rather than through the value itself.
        fn refuse_repeated_name(
            label: &str,
            position: &[usize],
            name: &str,
            expanding: &[&str],
        ) -> ! {
            let path = expanding
                .iter()
                .map(|resolved| format!("`{resolved}`"))
                .collect::<::std::vec::Vec<::std::string::String>>()
                .join(" → ");
            panic!(
                "`{}`: {} `{}` closes a flatten cycle through nested unions — its branch {} names `{}`, already expanding on the path {}, and no finite value inhabits the type; {}",
                #subject, #edge, label, branch_path(position), name, path, #cycle_remedy,
            );
        }

        fn refuse_non_object(label: &str, position: &[usize], named: &str) -> ! {
            if position.is_empty() {
                panic!(
                    "`{}`: {} `{}` is not written as an object — its schema describes a `{}`, which has no members to merge, and what serde writes for it does not join the object being written; {}",
                    #subject, #edge, label, named, #non_object_remedy,
                );
            }
            panic!(
                "`{}`: {} `{}` writes a union member that is not an object — its branch {} describes a `{}`, which has no members to merge, and what serde writes for that member does not join the object being written; {}",
                #subject, #edge, label, branch_path(position), named, #non_object_remedy,
            );
        }
    }
}

/// How the expansion refuses a `null` among the choices of what it merges, as the tokens that
/// declare the refusal: beside a tag every such `null`, and flattened a member over `()` of a
/// union declared below, whose `null` its record tells apart from an `Option`'s absence.
fn null_refusal(diagnostic: &MergeDiagnostic<'_>) -> proc_macro2::TokenStream {
    let (edge, non_object_remedy, subject) = (
        diagnostic.edge,
        diagnostic.non_object_remedy,
        diagnostic.subject,
    );
    let Some(tagged) = diagnostic.beside_tag else {
        return quote::quote! {
            fn refuse_unit_member(label: &str, position: &[usize]) -> ! {
                panic!(
                    "`{}`: {} `{}` writes a union member as `()` — its branch {} describes a `null`, for which serde writes no key, and which it does not read back as that member; {}",
                    #subject, #edge, label, branch_path(position), #non_object_remedy,
                );
            }
        };
    };
    quote::quote! {
        // serde's tag refuses an `Option` for `Some` and `None` alike, and `()` it writes as
        // the tag alone, which it does not read back as a union member.
        fn refuse_tagged_null(label: &str, position: &[usize]) -> ! {
            panic!(
                "`{}`: {} `{}` writes a union member as `null` — its branch {} describes a `null`, which cannot sit beside the tag: where that `null` is an optional's, serde refuses every write of it at run time — `cannot serialize tagged newtype variant {} containing an optional` — and where it is `()`'s, serde writes the tag alone, which it does not read back as a union member; {}",
                #subject, #edge, label, branch_path(position), #tagged, #non_object_remedy,
            );
        }
    }
}

/// The branch tree one merged schema contributes, as the tokens that declare how it is read.
/// `beside_tag` says the schema is the content of a newtype variant under a tag alone, where serde
/// refuses the `None` a flattened `Option` writes as no key.
fn branch_expansion(beside_tag: bool) -> proc_macro2::TokenStream {
    let offered_null = if beside_tag {
        quote::quote! { refuse_tagged_null(label, position); }
    } else {
        quote::quote! {
            if unit_member {
                refuse_unit_member(label, position);
            }
            return ::core::option::Option::Some(Branches::Absent);
        }
    };
    // Beside a tag every `null` of the content is refused, so no record is asked there.
    let (unit_member, handed_on) = if beside_tag {
        (quote::quote! {}, quote::quote! {})
    } else {
        (
            quote::quote! { , unit_member: bool },
            quote::quote! { , unit_member },
        )
    };
    // The `null` of the content's own choice is refused ahead of every other branch, as serde's
    // tag refuses an `Option` before it reads what the `Option` holds.
    let null_first = beside_tag.then(|| {
        quote::quote! {
            let offered_null = if position.is_empty() {
                branches
                    .iter()
                    .position(|branch| described_type(branch) == ::core::option::Option::Some("null"))
            } else {
                ::core::option::Option::None
            };
            if let ::core::option::Option::Some(index) = offered_null {
                position.push(index + 1);
                refuse_tagged_null(label, position);
            }
        }
    });
    let resolution = named_body_resolution();
    quote::quote! {
        fn expanded_branches<'defs>(
            schema: &'defs serde_json::Value,
            hoisted_defs: &'defs serde_json::Map<::std::string::String, serde_json::Value>,
            expanding: &mut ::std::vec::Vec<&'defs str>,
            position: &mut ::std::vec::Vec<usize>,
            label: &str
            #unit_member
        ) -> ::core::option::Option<Branches<'defs>> {
            #resolution

            if let ::core::option::Option::Some(named) = described_type(body) {
                // A `null` among the choices the flatten edge itself offers is the absence, not a
                // refusal: the source is nullable. Beside a tag, and below that level, it is
                // refused.
                if named == "null" && position.len() == 1 {
                    #offered_null
                }
                if named == "null" && position.is_empty() {
                    refuse_unit(label);
                }
                if named == "string" && body.get("enum").is_some() {
                    refuse_variant_name(label, position, "a plain enum's variant name");
                }
                if named != "object" {
                    refuse_non_object(label, position, named);
                }
            }

            let ::core::option::Option::Some((spelling, branches)) = union_branches(body) else {
                return body.as_object().map(Branches::Object);
            };
            #null_first

            // The name guards what is below it and nothing else, so it joins the path only for the
            // descent and leaves it before the level that resolved it answers.
            if let ::core::option::Option::Some(name) = resolved {
                expanding.push(name);
            }
            let mut expanded: ::std::vec::Vec<Branches<'defs>> = ::std::vec::Vec::new();
            for (index, branch) in branches.iter().enumerate() {
                position.push(index + 1);
                // A unit variant of the choice the flatten edge itself offers is a key set like any
                // other; one level down no surface describes the key it writes, and it is refused.
                let tagged = (spelling == "oneOf")
                    .then(|| tagged_unit_variant(branch))
                    .flatten();
                let below = match tagged {
                    ::core::option::Option::Some(name) if position.len() == 1 => ::core::option::Option::Some(Branches::Tagged(name)),
                    ::core::option::Option::Some(_) => refuse_variant_name(label, position, "the name of an externally tagged enum's unit variant"),
                    ::core::option::Option::None => expanded_branches(
                        branch, hoisted_defs, expanding, position, label #handed_on
                    ),
                };
                position.pop();
                expanded.extend(below);
            }
            if resolved.is_some() {
                expanding.pop();
            }
            (!expanded.is_empty()).then_some(Branches::Union(spelling, expanded))
        }
    }
}

/// The statements that bind `body`, what the schema a merge is handed stands for, and `resolved`,
/// the last name it was reached through: a definition may be a reference itself, as a brand or an
/// alias of a named type is, and a chain longer than the definitions are many closes on itself.
fn named_body_resolution() -> proc_macro2::TokenStream {
    quote::quote! {
        let mut resolved = ::core::option::Option::None;
        let mut body = schema;
        let mut steps = 0;
        while let ::core::option::Option::Some(name) = deferred_name(body) {
            let ::core::option::Option::Some(named_body) = hoisted_defs
                .get(name)
                .filter(|body| body.is_object() && steps < hoisted_defs.len())
            else {
                refuse_missing_body(label, position, name);
            };
            // The path holds only names already descended through, so the first frame never
            // finds itself on it and this refusal always has a branch to name.
            if expanding.contains(&name) {
                refuse_repeated_name(label, position, name, expanding);
            }
            resolved = ::core::option::Option::Some(name);
            body = named_body;
            steps += 1;
        }
    }
}

/// A base object's members with the members of every schema merged beside them.
pub fn merged_object_value(
    base: &proc_macro2::TokenStream,
    merged: &[MergedSource],
    diagnostic: &MergeDiagnostic<'_>,
) -> proc_macro2::TokenStream {
    let refusals = expansion_refusals(diagnostic);
    let expansion = branch_expansion(diagnostic.beside_tag.is_some());
    let labels = merged.iter().map(|source| source.label.as_str());
    let optionals = merged.iter().map(|source| source.optional);
    let entries = merged.iter().map(|source| source.entries);
    let maps = merged.iter().map(|source| source.map);
    let values = merged.iter().map(|source| &source.value);
    let readers = merge_readers();
    let tree = merged_tree();
    // Beside a tag no record is asked, so none is carried.
    let (flattened, read_as, unit_member) = if diagnostic.beside_tag.is_some() {
        (
            quote::quote! {
                let flattened: ::std::vec::Vec<
                    (&'static str, bool, bool, bool, serde_json::Value),
                > = vec![ #((#labels, #optionals, #entries, #maps, #values)),* ];
            },
            quote::quote! { (label, optional, entries, map, fs) },
            quote::quote! {},
        )
    } else {
        let units = merged.iter().map(|source| &source.unit_member);
        (
            quote::quote! {
                let flattened: ::std::vec::Vec<
                    (&'static str, bool, bool, bool, bool, serde_json::Value),
                > = vec![ #((#labels, #optionals, #entries, #maps, #units, #values)),* ];
            },
            quote::quote! { (label, optional, entries, map, unit_member, fs) },
            quote::quote! { , *unit_member },
        )
    };
    quote::quote! {
        {
            #tree
            #readers
            #refusals
            #expansion

            #flattened

            let mut described = Merged::Object(#base, ::std::vec::Vec::new());
            for #read_as in &flattened {
                let mut expanding: ::std::vec::Vec<&str> = ::std::vec::Vec::new();
                let mut position: ::std::vec::Vec<usize> = ::std::vec::Vec::new();
                if let ::core::option::Option::Some(source) = expanded_branches(
                    fs,
                    hoisted_defs,
                    &mut expanding,
                    &mut position,
                    label
                    #unit_member
                ) {
                    // The absence is offered around whatever the source described as, so a union
                    // reached through an `Option` keeps its own spelling.
                    let offered = if *optional { source.or_absent() } else { source };
                    let offered = if *entries { offered.entries_left() } else { offered };
                    described = described.multiplied(&offered, *map);
                }
            }

            described.into_value()
        }
    }
}

/// The struct's own fields distributed into each branch of the flattened types' schemas. A flatten
/// edge that closes a cycle is rejected where it is read rather than merged: `def_name` names one
/// end of the closing edge in the diagnostic.
fn flattened_object_body(
    json_schema_fields: &[proc_macro2::TokenStream],
    flatten_json_schemas: &[MergedSource],
    def_name: &str,
) -> proc_macro2::TokenStream {
    let open = is_nonstrict();
    let base = quote::quote! {
        {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            schema_obj.insert("additionalProperties".to_string(), serde_json::Value::Bool(#open));
            let mut properties = serde_json::Map::new();
            let mut required = ::std::vec::Vec::new();

            #(#json_schema_fields)*

            schema_obj.insert("properties".to_string(), serde_json::Value::Object(properties));
            schema_obj.insert("required".to_string(), serde_json::Value::Array(required));
            schema_obj
        }
    };
    merged_object_value(
        &base,
        flatten_json_schemas,
        &MergeDiagnostic {
            beside_tag: None,
            cycle_remedy: "write the field as a named member so the cycle defers through a reference",
            edge: "`#[serde(flatten)]` of",
            non_object_remedy: "write the field as a named member so the value gets a key of its own",
            subject: def_name,
        },
    )
}

/// Generates the JSON schema method implementation for plain enums.
pub fn generate_plain_enum_json_schema_method(
    enumerated: &[proc_macro2::TokenStream],
    def_name: &str,
    rust_ident: &str,
    parameters: &[SchemaParameter],
    examples: Option<&proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let body = quote::quote! {
        {
            let mut schema_obj = serde_json::Map::new();
            schema_obj.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            schema_obj.insert("enum".to_string(), serde_json::Value::Array(
                [#(#enumerated),*].into_iter().map(|v: &str| serde_json::Value::String(v.to_string())).collect()
            ));

            serde_json::Value::Object(schema_obj)
        }
    };
    json_schema_methods(def_name, rust_ident, &body, parameters, examples)
}

#[cfg(test)]
mod tests;
