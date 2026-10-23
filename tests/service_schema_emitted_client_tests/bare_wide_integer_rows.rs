//! What each `http_rest` client of [`TallyingBareService`](super::run_node_wide_integers) sends
//! of a bare `i64` and reads of one: the request each call hands its transport, which is what the
//! Rust client sends of the same call, and what the client answers of the response the Rust
//! dispatcher answers it with.

#![cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]

/// Each call of a run, in the order each run makes it: `shift(9223372036854775806, 1)`,
/// `shift(4611686018427387903, none)`, `total([9223372036854775807, 1])`,
/// `record(-9223372036854775808)` and `total([1])`, answered beyond an `i64`.
pub const BARE_EXCHANGES: [BareExchange; 5] = [
    BareExchange {
        answered: "ok 9223372036854775807 null",
        body: "9223372036854775807",
        headers: &[],
        request: "GET /bare/9223372036854775806 x-offset=1 ",
        status: 200,
    },
    BareExchange {
        answered: "ok 4611686018427387903 9223372036854775806",
        body: "4611686018427387903",
        headers: &[("x-doubled", "9223372036854775806")],
        request: "GET /bare/4611686018427387903  ",
        status: 200,
    },
    BareExchange {
        answered: "declared -9223372036854775808",
        body: "-9223372036854775808",
        headers: &[],
        request: "POST /bare  [9223372036854775807,1]",
        status: 422,
    },
    BareExchange {
        answered: "sent",
        body: "",
        headers: &[],
        request: "PUT /bare  -9223372036854775808",
        status: 204,
    },
    BareExchange {
        answered: "fault undeserializable-payload",
        body: "9223372036854775808",
        headers: &[],
        request: "POST /bare  [1]",
        status: 200,
    },
];

/// One call: the request it hands the transport, as a run prints it, the response the transport
/// answers with, and what the client answers.
pub struct BareExchange {
    /// What the client answers, as a run prints it.
    pub answered: &'static str,
    /// The response's body.
    pub body: &'static str,
    /// The response's headers.
    pub headers: &'static [(&'static str, &'static str)],
    /// The request: its method, its path, each header as `name=value` and its body.
    pub request: &'static str,
    /// The response's status.
    pub status: u16,
}

/// What a run prints of [`BARE_EXCHANGES`]: each request, then what the client answered.
pub fn bare_printed() -> String {
    BARE_EXCHANGES
        .iter()
        .fold(String::new(), |mut printed, exchange| {
            printed.push_str(exchange.request);
            printed.push('\n');
            printed.push_str(exchange.answered);
            printed.push('\n');
            printed
        })
}
