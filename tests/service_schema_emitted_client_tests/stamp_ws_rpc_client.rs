//! The `ws_rpc` client for `StampClientService`, expanded out of the transport's macro into a
//! module the headers groups name. The `use` resolves the types the author declared, which the
//! macro spells exactly as they were written.

use crate::tests::{StampError, StampReceipt};

stamp_client_service_ws_rpc_client!();
