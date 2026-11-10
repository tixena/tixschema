//! The TypeScript `mcp` client, read off the emitted text.

use super::ts_mcp_client_of;

const MESSAGES_ONLY: &str = "
    pub trait GreetingService<Ctx> {
        /// Greets one person.
        #[service_schema_op(mcp(prompt))]
        async fn greet(&self, ctx: &Ctx, name: String)
            -> Result<Vec<greeting_service_schema::McpPromptMessage>, GreetingError>;
    }
";

const A_STRING_PROMPT: &str = "
    pub trait GreetingService<Ctx> {
        /// Greets one person in a line.
        #[service_schema_op(mcp(prompt))]
        async fn greet_in_a_line(&self, ctx: &Ctx, name: String) -> Result<String, GreetingError>;
    }
";

#[test]
fn the_prompt_text_reader_is_written_only_where_a_prompt_answers_a_string() {
    let messages = ts_mcp_client_of(MESSAGES_ONLY);
    assert!(!messages.contains("McpClientPromptText"), "got: {messages}");
    let text = ts_mcp_client_of(A_STRING_PROMPT);
    assert!(
        text.contains("function greetingServiceMcpClientPromptText("),
        "got: {text}"
    );
}
