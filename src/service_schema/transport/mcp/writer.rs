//! The writer half of the dispatcher: the request in flight, the progress it queues, the
//! cancellation it carries, and the event stream an HTTP answer becomes.

use proc_macro2::TokenStream;
use quote::quote;

/// The types of the writer: the call, the table of calls in flight, and the event stream.
pub(super) fn types() -> TokenStream {
    let call = call();
    let driven = driven();
    let in_flight = in_flight();
    let event_stream = event_stream_type();
    quote! {
        /// The client has gone: a write to it failed.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct McpClientGone;

        /// Where the transport writes a message beyond the answer: one line on stdio, one event of
        /// an event-stream answer on HTTP. A failed write means the client has gone.
        pub trait McpEventWriter {
            fn write(&self, message: ::std::string::String) -> impl ::core::future::Future<Output = ::core::result::Result<(), McpClientGone>> + ::core::marker::Send;
        }

        /// What `serve` writes to, and the requests it has in flight.
        pub struct McpHost<'host, W> {
            pub in_flight: &'host McpInFlight,
            pub writer: &'host W,
        }

        #call
        #driven
        #in_flight
        #event_stream
    }
}

/// The functions of the writer.
pub(super) fn functions() -> TokenStream {
    quote! {
        fn held<T>(lock: &::std::sync::Mutex<T>) -> ::std::sync::MutexGuard<'_, T> {
            lock.lock().unwrap_or_else(::std::sync::PoisonError::into_inner)
        }

        /// One event of an event stream, carrying one JSON-RPC message.
        fn event_text(message: &str) -> ::std::string::String {
            format!("event: message\ndata: {message}\n\n")
        }

        /// The `200` event-stream answer: each event written, then the answer.
        fn event_stream_response(stream: &EventStream, answered: ::core::option::Option<::std::string::String>) -> McpHttpResponse {
            let mut body = ::core::mem::take(&mut *held(&stream.events));
            if let ::core::option::Option::Some(text) = answered {
                body.push_str(&event_text(&text));
            }
            McpHttpResponse {
                status: 200,
                headers: ::std::vec![
                    ("Content-Type".to_owned(), "text/event-stream".to_owned()),
                    ("X-Accel-Buffering".to_owned(), "no".to_owned()),
                ],
                body: body.into_bytes(),
            }
        }
    }
}

fn call() -> TokenStream {
    quote! {
        /// The request in flight, handed to `admit` and to the closure that builds `Ctx`, so that a
        /// handler reaches it.
        #[derive(Clone, Debug)]
        pub struct McpCall {
            state: ::std::sync::Arc<McpCallState>,
        }

        #[derive(Debug)]
        struct McpCallState {
            cancelled: ::core::sync::atomic::AtomicBool,
            queued: ::std::sync::Mutex<::std::collections::VecDeque<::std::string::String>>,
            token: ::core::option::Option<::serde_json::Value>,
            waiting: ::std::sync::Mutex<::core::option::Option<::core::task::Waker>>,
        }

        impl McpCall {
            fn cancel(&self) {
                self.state.cancelled.store(true, ::core::sync::atomic::Ordering::Release);
                self.wake();
            }

            /// Whether the client cancelled this request, or went away.
            pub fn cancelled(&self) -> bool {
                self.state.cancelled.load(::core::sync::atomic::Ordering::Acquire)
            }

            fn discard(&self) {
                held(&self.state.queued).clear();
            }

            fn has_queued(&self) -> bool {
                !held(&self.state.queued).is_empty()
            }

            fn new(token: ::core::option::Option<::serde_json::Value>) -> Self {
                Self {
                    state: ::std::sync::Arc::new(McpCallState {
                        cancelled: ::core::sync::atomic::AtomicBool::new(false),
                        queued: ::std::sync::Mutex::new(::std::collections::VecDeque::new()),
                        token,
                        waiting: ::std::sync::Mutex::new(::core::option::Option::None),
                    }),
                }
            }

            fn next_queued(&self) -> ::core::option::Option<::std::string::String> {
                held(&self.state.queued).pop_front()
            }

            /// The call of `message`, which reports progress where it carries a `progressToken`.
            fn of(message: &::serde_json::Value) -> Self {
                let token = &message["params"]["_meta"]["progressToken"];
                Self::new((token.is_string() || token.is_number()).then(|| token.clone()))
            }

            /// The call of an HTTP request, read before its body is judged.
            fn of_body(request: &McpHttpRequest) -> Self {
                match ::serde_json::from_slice::<::serde_json::Value>(&request.body) {
                    ::core::result::Result::Ok(message) => Self::of(&message),
                    ::core::result::Result::Err(_) => Self::new(::core::option::Option::None),
                }
            }

            /// Queues `notifications/progress` where the request carries a `progressToken`, and does
            /// nothing otherwise.
            pub fn progress(&self, done: f64, total: ::core::option::Option<f64>, message: ::core::option::Option<&str>) {
                let ::core::option::Option::Some(token) = &self.state.token else {
                    return;
                };
                if self.cancelled() {
                    return;
                }
                let mut params = ::serde_json::json!({ "progressToken": token, "progress": done });
                if let ::core::option::Option::Some(total) = total {
                    params["total"] = ::serde_json::Value::from(total);
                }
                if let ::core::option::Option::Some(message) = message {
                    params["message"] = ::serde_json::Value::from(message);
                }
                held(&self.state.queued).push_back(canonical(&::serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/progress",
                    "params": params,
                })));
                self.wake();
            }

            fn reports_progress(&self) -> bool {
                self.state.token.is_some()
            }

            fn wake(&self) {
                let waiting = held(&self.state.waiting).take();
                if let ::core::option::Option::Some(waker) = waiting {
                    waker.wake();
                }
            }

            fn watch(&self, waker: &::core::task::Waker) {
                *held(&self.state.waiting) = ::core::option::Option::Some(waker.clone());
            }
        }
    }
}

fn in_flight() -> TokenStream {
    quote! {
        /// The requests a stdio host has in flight, so that `notifications/cancelled` reaches the
        /// right one.
        #[derive(Clone, Debug, Default)]
        pub struct McpInFlight {
            calls: ::std::sync::Arc<::std::sync::Mutex<::std::collections::HashMap<::std::string::String, McpCall>>>,
        }

        impl McpInFlight {
            fn cancel(&self, id: &::serde_json::Value) {
                let found = held(&self.calls).get(&id.to_string()).cloned();
                if let ::core::option::Option::Some(call) = found {
                    call.cancel();
                }
            }

            fn enter<'table>(&'table self, id: &::serde_json::Value, call: &McpCall) -> Entered<'table> {
                let key = id.to_string();
                held(&self.calls).insert(key.clone(), call.clone());
                Entered { table: self, key, call: call.clone() }
            }
        }

        /// A request listed in a table until it is dropped.
        struct Entered<'table> {
            call: McpCall,
            key: ::std::string::String,
            table: &'table McpInFlight,
        }

        impl ::core::ops::Drop for Entered<'_> {
            fn drop(&mut self) {
                let mut calls = held(&self.table.calls);
                let ours = calls
                    .get(&self.key)
                    .is_some_and(|listed| ::std::sync::Arc::ptr_eq(&listed.state, &self.call.state));
                if ours {
                    calls.remove(&self.key);
                }
            }
        }
    }
}

fn event_stream_type() -> TokenStream {
    quote! {
        /// The events an HTTP answer writes before its final one.
        #[derive(Default)]
        struct EventStream {
            events: ::std::sync::Mutex<::std::string::String>,
        }

        impl McpEventWriter for EventStream {
            fn write(&self, message: ::std::string::String) -> impl ::core::future::Future<Output = ::core::result::Result<(), McpClientGone>> + ::core::marker::Send {
                held(&self.events).push_str(&event_text(&message));
                ::core::future::ready(::core::result::Result::Ok(()))
            }
        }
    }
}

fn driven() -> TokenStream {
    quote! {
        /// The handler's work, polled with the progress it queues written between two polls. A
        /// failed write cancels the call and drops what is still queued.
        struct Driven<'call, W, F> {
            call: &'call McpCall,
            finished: ::core::option::Option<::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>>,
            work: ::core::pin::Pin<::std::boxed::Box<F>>,
            writer: &'call W,
            writing: ::core::option::Option<::core::pin::Pin<::std::boxed::Box<dyn ::core::future::Future<Output = ::core::result::Result<(), McpClientGone>> + ::core::marker::Send + 'call>>>,
        }

        impl<'call, W, F> Driven<'call, W, F> {
            fn new(call: &'call McpCall, writer: &'call W, work: F) -> Self {
                Self { call, finished: ::core::option::Option::None, work: ::std::boxed::Box::pin(work), writer, writing: ::core::option::Option::None }
            }
        }

        impl<W, F> ::core::future::Future for Driven<'_, W, F>
        where
            W: McpEventWriter,
            F: ::core::future::Future<Output = ::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>>,
        {
            type Output = ::core::option::Option<::core::result::Result<::serde_json::Value, ::serde_json::Value>>;

            fn poll(mut self: ::core::pin::Pin<&mut Self>, cx: &mut ::core::task::Context<'_>) -> ::core::task::Poll<Self::Output> {
                let this = &mut *self;
                this.call.watch(cx.waker());
                loop {
                    if let ::core::option::Option::Some(writing) = this.writing.as_mut() {
                        match ::core::future::Future::poll(writing.as_mut(), cx) {
                            ::core::task::Poll::Pending => return ::core::task::Poll::Pending,
                            ::core::task::Poll::Ready(::core::result::Result::Ok(())) => {}
                            ::core::task::Poll::Ready(::core::result::Result::Err(McpClientGone)) => this.call.cancel(),
                        }
                        this.writing = ::core::option::Option::None;
                    }
                    if this.call.cancelled() {
                        this.call.discard();
                    }
                    if let ::core::option::Option::Some(message) = this.call.next_queued() {
                        this.writing = ::core::option::Option::Some(::std::boxed::Box::pin(this.writer.write(message)));
                        continue;
                    }
                    if let ::core::option::Option::Some(answered) = this.finished.take() {
                        return ::core::task::Poll::Ready(answered);
                    }
                    match this.work.as_mut().poll(cx) {
                        ::core::task::Poll::Ready(answered) => this.finished = ::core::option::Option::Some(answered),
                        ::core::task::Poll::Pending if this.call.has_queued() => {}
                        ::core::task::Poll::Pending => return ::core::task::Poll::Pending,
                    }
                }
            }
        }
    }
}
