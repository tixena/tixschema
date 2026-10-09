//! A dispatcher written by hand, for a service that named no transport.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "service_schema_hand_written_dispatcher_tests/declarations.rs"]
mod declarations;

#[cfg(test)]
#[path = "service_schema_hand_written_dispatcher_tests/dispatcher.rs"]
mod dispatcher;
