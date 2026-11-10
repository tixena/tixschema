//! A client placed in a crate other than the one that declared the service, which is how a caller
//! of a server uses it: the declaring crate and the calling crate are built apart, and the caller
//! prints what each kind of answer makes of a call.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "mcp_client_tests/tests.rs"]
mod tests;
