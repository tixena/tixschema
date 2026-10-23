//! A service that sends and answers a map keyed by an `i64`: each method and each arm of a result
//! that writes one carries a `@deprecated` note, as a member does.

use super::{HeaderProbeError, poll_once};
use core::future::ready;
use std::collections::HashMap;
use tixschema::service_schema;

/// The line of a `JSDoc` block that carries the note, at a member's indent.
#[cfg(all(feature = "typescript", feature = "zod"))]
const NOTED: &str = "   * @deprecated A map key above 2^53 - 1 (9007199254740991) in size is not \
                     held exactly as a number.";

/// The note as a `JSDoc` block of one line.
#[cfg(all(feature = "typescript", feature = "zod"))]
const NOTE_BLOCK: &str = "/** @deprecated A map key above 2^53 - 1 (9007199254740991) in size is \
                          not held exactly as a number. */";

/// A service that sends and answers a map keyed by an `i64`, one keyed by a `u32`, and a message
/// it declares from two arguments, one of which is a map keyed by an `i64`.
#[service_schema(transports = [])]
pub trait KeyedProbeService<Ctx> {
    async fn narrow(
        &self,
        ctx: &Ctx,
        req: HashMap<u32, bool>,
    ) -> Result<HashMap<u32, bool>, HeaderProbeError>;

    async fn stow(
        &self,
        ctx: &Ctx,
        kept: HashMap<i64, bool>,
        note: String,
    ) -> Result<(), HeaderProbeError>;

    async fn tally(
        &self,
        ctx: &Ctx,
        req: HashMap<i64, bool>,
    ) -> Result<HashMap<i64, bool>, HeaderProbeError>;
}

/// The implementation of `KeyedProbeService` these tests call.
pub struct KeyedProbeBackEnd;

impl KeyedProbeService<()> for KeyedProbeBackEnd {
    async fn narrow(
        &self,
        _ctx: &(),
        req: HashMap<u32, bool>,
    ) -> Result<HashMap<u32, bool>, HeaderProbeError> {
        ready(()).await;
        Ok(req)
    }

    async fn stow(
        &self,
        _ctx: &(),
        kept: HashMap<i64, bool>,
        _note: String,
    ) -> Result<(), HeaderProbeError> {
        ready(()).await;
        if kept.is_empty() {
            Err(HeaderProbeError::Missing)
        } else {
            Ok(())
        }
    }

    async fn tally(
        &self,
        _ctx: &(),
        req: HashMap<i64, bool>,
    ) -> Result<HashMap<i64, bool>, HeaderProbeError> {
        ready(()).await;
        Ok(req)
    }
}

#[test]
fn the_keyed_probe_service_is_still_implementable_and_callable() {
    let held = HashMap::from([(1_i64 << 60_i32, true)]);
    let tallied = poll_once(KeyedProbeBackEnd.tally(&(), held.clone())).unwrap();
    assert_eq!(tallied.ok(), Some(held.clone()), "the tally answered");
    let narrowed = poll_once(KeyedProbeBackEnd.narrow(&(), HashMap::from([(7_u32, false)])))
        .unwrap()
        .ok();
    assert_eq!(
        narrowed,
        Some(HashMap::from([(7_u32, false)])),
        "the narrow map"
    );
    let stowed = poll_once(KeyedProbeBackEnd.stow(&(), HashMap::new(), "n".to_owned())).unwrap();
    assert!(
        matches!(stowed, Err(HeaderProbeError::Missing)),
        "got: {stowed:?}"
    );
    assert!(
        poll_once(KeyedProbeBackEnd.stow(&(), held, "n".to_owned()))
            .unwrap()
            .is_ok(),
        "a map that holds an entry is stowed"
    );
}

/// Asserts `written` holds each of `expected`.
#[cfg(all(feature = "typescript", feature = "zod"))]
fn holds_each(written: &str, expected: &[String]) {
    for held in expected {
        assert!(written.contains(held), "missing `{held}` in:\n{written}");
    }
}

/// A method carries the note where its message writes a map keyed by a 64-bit integer, and a
/// result's and an outcome's arm where its value writes one; a message the macro declares carries
/// it on its member, and nothing that writes a map keyed by a `u32` carries it.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn a_method_and_an_arm_that_write_a_map_keyed_by_a_64_bit_integer_carry_the_note() {
    let arm = format!(
        "  | {{\n      ok: true;\n      {NOTE_BLOCK}\n      value: Partial<Record<number, \
         boolean>>;\n    }}\n"
    );
    holds_each(
        &KeyedProbeServiceSchema::ts_definition(),
        &[
            format!("export type KeyedProbeServiceTallyResult =\n{arm}"),
            "export type KeyedProbeServiceNarrowResult =\n  | { ok: true; value: \
             Partial<Record<number, boolean>> }\n"
                .to_owned(),
            format!("   * kept\n   * \n{NOTED}\n   */\n  kept: Partial<Record<number, boolean>>;"),
        ],
    );
    holds_each(
        &KeyedProbeServiceSchema::ts_service(),
        &[
            format!("export type KeyedProbeServiceTallyOutcome =\n{arm}"),
            format!(
                "  /**\n   * Handles `tally` on `KeyedProbeService` and answers it.\n   *\n\
                 {NOTED}\n   */\n  tally(ctx: Ctx, req: Partial<Record<number, boolean>>): \
                 Promise<KeyedProbeServiceTallyOutcome>;"
            ),
            "  /** Handles `narrow` on `KeyedProbeService` and answers it. */\n".to_owned(),
        ],
    );
    holds_each(
        &KeyedProbeServiceSchema::ts_client(),
        &[
            format!(
                "  /**\n   * Calls `tally` on `KeyedProbeService` and waits for the answer.\n   \
                 *\n{NOTED}\n   */\n  tally(req: Partial<Record<number, boolean>>): \
                 Promise<KeyedProbeServiceTallyResult>;"
            ),
            "  /** Calls `narrow` on `KeyedProbeService` and waits for the answer. */\n".to_owned(),
        ],
    );
    holds_each(
        &KeyedProbeServiceSchema::ts_http_client(),
        &[
            format!(
                "  /**\n   * Calls `tally` over `POST /tally`.\n   *\n{NOTED}\n   */\n  tally(req: \
                 Partial<Record<number, boolean>>): Promise<KeyedProbeServiceTallyResult>;"
            ),
            "  /** Calls `narrow` over `POST /narrow`. */\n".to_owned(),
        ],
    );
}
