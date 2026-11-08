//! The structs a dispatcher publishes keep their fields private: each is built through its
//! constructor and read back through its accessors, in every module a dispatcher is expanded in.

macro_rules! built_and_read_back {
    ($($module:ident),* $(,)?) => {$(
        mod $module {
            use crate::$module as dispatcher;

            #[test]
            fn the_published_structs_hold_what_they_were_built_with() {
                let oauth = dispatcher::McpOAuth::new(
                    &["https://auth.example.com"],
                    "https://mcp.example.com/mcp",
                    "https://mcp.example.com/.well-known/oauth-protected-resource/mcp",
                    &["documents:read"],
                );
                assert_eq!(oauth.authorization_servers(), ["https://auth.example.com"]);
                assert_eq!(oauth.resource(), "https://mcp.example.com/mcp");
                assert_eq!(
                    oauth.resource_metadata_url(),
                    "https://mcp.example.com/.well-known/oauth-protected-resource/mcp"
                );
                assert_eq!(oauth.scopes_supported(), ["documents:read"]);

                let settings =
                    dispatcher::McpHttpSettings::new(&["https://app.example.com"], Some(oauth));
                assert_eq!(settings.allowed_origins(), ["https://app.example.com"]);
                assert_eq!(
                    settings.oauth().map(|held| held.resource()),
                    Some("https://mcp.example.com/mcp")
                );

                let request = dispatcher::McpHttpRequest::new(
                    "POST".to_owned(),
                    "/mcp?probe=1".to_owned(),
                    vec![("Mcp-Name".to_owned(), "get-version".to_owned())],
                    b"{}".to_vec(),
                );
                assert_eq!(request.method(), "POST");
                assert_eq!(request.path(), "/mcp?probe=1");
                assert_eq!(request.header("mcp-name"), Some("get-version"));
                assert_eq!(
                    request.headers(),
                    [("Mcp-Name".to_owned(), "get-version".to_owned())]
                );
                assert_eq!(request.body(), b"{}");

                let response = dispatcher::McpHttpResponse::new(
                    202,
                    vec![("Content-Type".to_owned(), "text/plain".to_owned())],
                    b"ok".to_vec(),
                );
                assert_eq!(response.status(), 202);
                assert_eq!(
                    response.headers(),
                    [("Content-Type".to_owned(), "text/plain".to_owned())]
                );
                assert_eq!(response.body(), b"ok");

                let in_flight = dispatcher::McpInFlight::default();
                let writer = String::new();
                let host = dispatcher::McpHost::new(&in_flight, &writer);
                assert!(core::ptr::eq(host.in_flight(), &in_flight));
                assert!(core::ptr::eq(host.writer(), &writer));
            }
        }
    )*};
}
