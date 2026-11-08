//! `subscriptions/listen` on the Rust dispatcher: the application's source of changes, the
//! acknowledgment, and the notifications written on the stream.

use proc_macro2::TokenStream;
use quote::quote;

/// The type the application's source of changes yields.
pub(super) fn types() -> TokenStream {
    quote! {
        /// What the application's source of changes hands to a `subscriptions/listen` stream.
        #[derive(Clone, Debug, Eq, PartialEq)]
        #[non_exhaustive]
        pub enum McpChange {
            /// The list of resources changed.
            ResourcesListChanged,
            /// One resource changed.
            ResourceUpdated { uri: ::std::string::String },
            /// One task moved.
            Task { task_id: ::std::string::String },
            /// Keeps an event stream open: the comment line `:` on HTTP, nothing on stdio.
            KeepAlive,
        }
    }
}

pub(super) fn functions() -> TokenStream {
    quote! {
        /// A source of changes that never yields, for the entry points that offer none.
        async fn idle() -> ::core::option::Option<McpChange> {
            ::core::option::Option::None
        }

        /// The filter a `subscriptions/listen` asks for, or the `-32602` error that refuses it.
        fn requested_filter(params: &::serde_json::Value) -> ::core::result::Result<::serde_json::Value, ::serde_json::Value> {
            let invalid = |detail: &str| error_object(-32602, &format!("Invalid params: {detail}"), ::core::option::Option::None);
            let ::core::option::Option::Some(filter) = params.get("notifications").filter(|held| held.is_object()) else {
                return ::core::result::Result::Err(invalid("subscriptions/listen needs `notifications`"));
            };
            if filter.get("resourcesListChanged").is_some_and(|held| !held.is_boolean()) {
                return ::core::result::Result::Err(invalid("`resourcesListChanged` is not a boolean"));
            }
            if let ::core::option::Option::Some(uris) = filter.get("resourceSubscriptions") {
                let strings = uris.as_array().is_some_and(|held| held.iter().all(::serde_json::Value::is_string));
                if !strings {
                    return ::core::result::Result::Err(invalid("`resourceSubscriptions` is not an array of strings"));
                }
            }
            ::core::result::Result::Ok(filter.clone())
        }

        /// The subset of `filter` this endpoint honors: resources only, because the tool list is
        /// fixed at compile time and no prompts are offered.
        fn honored(filter: &::serde_json::Value, endpoint: &McpEndpoint) -> ::serde_json::Value {
            let mut agreed = ::serde_json::Map::new();
            let offers_resources = endpoint
                .catalogs()
                .iter()
                .any(|catalog| catalog()["capabilities"].get("resources").is_some());
            if offers_resources {
                if filter["resourcesListChanged"] == true {
                    agreed.insert("resourcesListChanged".to_owned(), ::serde_json::Value::Bool(true));
                }
                if let ::core::option::Option::Some(::serde_json::Value::Array(uris)) = filter.get("resourceSubscriptions") {
                    if !uris.is_empty() {
                        agreed.insert("resourceSubscriptions".to_owned(), ::serde_json::Value::Array(uris.clone()));
                    }
                }
            }
            ::serde_json::Value::Object(agreed)
        }

        /// A notification of a subscription stream.
        fn subscribed(id: &::serde_json::Value, method: &str, mut params: ::serde_json::Value) -> Queued {
            params["_meta"] = ::serde_json::json!({ SUBSCRIPTION_KEY: id });
            Queued::Message(canonical(&::serde_json::json!({ "jsonrpc": "2.0", "method": method, "params": params })))
        }

        /// The work of a `subscriptions/listen`: the acknowledgment, then what `changes` yields that
        /// the client asked for, until `changes` yields `None`, which ends the stream with the
        /// graceful answer, or the call is cancelled, which drops it.
        async fn listening<Changes>(
            request: &::serde_json::Value,
            call: &McpCall,
            endpoint: &McpEndpoint,
            changes: &mut Changes,
        ) -> ::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>
        where
            Changes: ::core::ops::AsyncFnMut() -> ::core::option::Option<McpChange>,
        {
            let id = &request["id"];
            let agreed = match requested_filter(&request["params"]) {
                ::core::result::Result::Ok(filter) => honored(&filter, endpoint),
                ::core::result::Result::Err(refused) => return ::core::option::Option::Some(::core::result::Result::Err(refused)),
            };
            call.queue(subscribed(id, "notifications/subscriptions/acknowledged", ::serde_json::json!({ "notifications": agreed })));
            let lists = agreed["resourcesListChanged"] == true;
            let watched: ::std::vec::Vec<::std::string::String> = agreed["resourceSubscriptions"]
                .as_array()
                .map(|uris| uris.iter().filter_map(|uri| uri.as_str().map(::std::borrow::ToOwned::to_owned)).collect())
                .unwrap_or_default();
            loop {
                let mut next = ::core::pin::pin!(changes());
                let change = ::core::future::poll_fn(|cx| {
                    if call.cancelled() {
                        return ::core::task::Poll::Ready(::core::option::Option::None);
                    }
                    ::core::future::Future::poll(next.as_mut(), cx).map(::core::option::Option::Some)
                })
                .await;
                match change {
                    ::core::option::Option::None => return ::core::option::Option::None,
                    ::core::option::Option::Some(::core::option::Option::None) => break,
                    ::core::option::Option::Some(::core::option::Option::Some(McpChange::ResourcesListChanged)) => {
                        if lists {
                            call.queue(subscribed(id, "notifications/resources/list_changed", ::serde_json::json!({})));
                        }
                    }
                    ::core::option::Option::Some(::core::option::Option::Some(McpChange::ResourceUpdated { uri })) => {
                        if watched.contains(&uri) {
                            call.queue(subscribed(id, "notifications/resources/updated", ::serde_json::json!({ "uri": uri })));
                        }
                    }
                    ::core::option::Option::Some(::core::option::Option::Some(McpChange::Task { .. })) => {}
                    ::core::option::Option::Some(::core::option::Option::Some(McpChange::KeepAlive)) => call.queue(Queued::KeepAlive),
                }
            }
            ::core::option::Option::Some(::core::result::Result::Ok(::serde_json::json!({ "_meta": { SUBSCRIPTION_KEY: id } })))
        }
    }
}
