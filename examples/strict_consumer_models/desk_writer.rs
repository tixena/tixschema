//! A writer for the `mcp` dispatcher of `DeskService`, written outside the module that expands it.

use crate::desk_dispatcher::{McpClientGone, McpEventWriter, McpHttpStream};
use core::future::{Future, ready};
use std::sync::Mutex;

/// Keeps the events written to it, and fails once its client has gone.
pub struct DeskWriter {
    events: Mutex<Vec<String>>,
    gone: bool,
}

impl DeskWriter {
    fn keep(&self, text: String) -> Result<(), McpClientGone> {
        if self.gone {
            return Err(McpClientGone::new());
        }
        let Ok(mut events) = self.events.lock() else {
            return Err(McpClientGone::new());
        };
        events.push(text);
        Ok(())
    }

    /// A writer whose client has gone where `gone` is true.
    #[must_use = "a writer records nothing until it is handed to a dispatcher"]
    pub const fn new(gone: bool) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            gone,
        }
    }
}

impl McpEventWriter for DeskWriter {
    fn write(&self, message: String) -> impl Future<Output = Result<(), McpClientGone>> + Send {
        ready(self.keep(message))
    }
}

impl McpHttpStream for DeskWriter {
    fn begin(
        &self,
        status: u16,
        _headers: &[(String, String)],
    ) -> impl Future<Output = Result<(), McpClientGone>> + Send {
        ready(self.keep(status.to_string()))
    }
}
