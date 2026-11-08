//! The test of every entry point that answers `subscriptions/listen`, run on one dispatcher.

/// One module per dispatcher, one test per entry point, so that every function the dispatcher
/// emits is used.
macro_rules! listens_through_every_entry_point_of {
    ($name:ident, $module:ident, $schema:ident, $service:expr, $ctx:expr) => {
        mod $name {
            use super::*;

            static ENDPOINT: $module::McpEndpoint =
                $module::McpEndpoint::new(&[$schema::MCP_NAMES], &[$module::catalog]);
            const SETTINGS: $module::McpHttpSettings<'static> = $module::McpHttpSettings::new(&[], None);

            fn script() -> VecDeque<$module::McpChange> {
                VecDeque::from([
                    $module::McpChange::ResourcesListChanged,
                    $module::McpChange::ResourceUpdated {
                        uri: WATCHED.to_owned(),
                    },
                    $module::McpChange::Task {
                        task_id: "export:1".to_owned(),
                        state: serde_json::json!({}),
                    },
                    $module::McpChange::KeepAlive,
                ])
            }

            fn server() -> $module::McpServer<'static> {
                $module::McpServer::new(&ENDPOINT, None, $module::SERVER_NAME)
            }

            fn asked() -> Value {
                listen(&serde_json::json!({ "resourcesListChanged": true }))
            }

            fn posted() -> $module::McpHttpRequest {
                $module::McpHttpRequest::new("POST".to_owned(), "/mcp".to_owned(), vec![
                        ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
                        ("Mcp-Method".to_owned(), "subscriptions/listen".to_owned()),
                    ], asked().to_string().into_bytes())
            }

            fn closes(answered: &str) {
                assert_valid(
                    "SubscriptionsListenResultResponse",
                    &serde_json::from_str(answered).unwrap(),
                );
            }

            #[test]
            fn answer_listening() {
                let service = $service;
                let lines = Lines::default();
                let mut queue = script();
                let mut source = async || queue.pop_front();
                closes(
                    &block_on($module::answer_listening(
                        &asked().to_string(),
                        &service,
                        $ctx,
                        &lines,
                        &$module::McpInFlight::default(),
                        &mut source,
                    ))
                    .unwrap(),
                );
                assert_eq!(
                    written(&lines)[0]["method"],
                    "notifications/subscriptions/acknowledged"
                );
            }

            #[test]
            fn serve_listening() {
                let service = $service;
                let lines = Lines::default();
                let mut queue = script();
                let mut source = async || queue.pop_front();
                closes(
                    &block_on($module::serve_listening(
                        &asked().to_string(),
                        &server(),
                        $ctx,
                        $module::McpHost::new(&$module::McpInFlight::default(), &lines),
                        &|ctx: &_, part, method: &str, params: &Value| {
                            let held = service.clone();
                            Box::pin(async move {
                                match part {
                                    0 => $module::route(method, params, &held, ctx).await,
                                    _ => None,
                                }
                            })
                        },
                        &mut source,
                    ))
                    .unwrap(),
                );
                assert_eq!(
                    written(&lines)[0]["method"],
                    "notifications/subscriptions/acknowledged"
                );
            }

            #[test]
            fn answer_http_listening() {
                let service = $service;
                let mut queue = script();
                let mut source = async || queue.pop_front();
                let response = through(|stream| block_on($module::answer_http_listening(
                    &posted(),
                    &SETTINGS, stream,
                    &service,
                    async |_request: &$module::McpHttpRequest, call: $module::McpCall| {
                        Ok::<_, $module::McpDenied>(($ctx)(call))
                    },
                    &mut source,
                )));
                assert_eq!(response.status(), 200);
                let streamed = String::from_utf8(response.body().to_vec()).unwrap();
                assert_eq!(streamed.matches(":\n\n").count(), 1, "{streamed}");
            }

            #[test]
            fn serve_http_listening() {
                let service = $service;
                let mut queue = script();
                let mut source = async || queue.pop_front();
                let response = through(|stream| block_on($module::serve_http_listening(
                    &posted(),
                    &SETTINGS, stream,
                    &server(),
                    async |_request: &$module::McpHttpRequest, call: $module::McpCall| {
                        Ok::<_, $module::McpDenied>(($ctx)(call))
                    },
                    &|ctx: &_, part, method: &str, params: &Value| {
                        let held = service.clone();
                        Box::pin(async move {
                            match part {
                                0 => $module::route(method, params, &held, ctx).await,
                                _ => None,
                            }
                        })
                    },
                    &mut source,
                )));
                assert_eq!(response.status(), 200);
                let streamed = String::from_utf8(response.body().to_vec()).unwrap();
                assert_eq!(streamed.matches(":\n\n").count(), 1, "{streamed}");
            }
        }
    };
}
