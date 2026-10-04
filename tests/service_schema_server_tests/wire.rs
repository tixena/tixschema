//! What the server macro puts on the queue: the `type` property and the body of every reply, and
//! the headers beside them, built from `serde_json::json!` literals rather than from any one
//! service's own message types, since the framing reads structurally and names none.

#[cfg(test)]
mod tests {
    use lapin::types::{AMQPValue, FieldTable, LongString, ShortString};
    use serde_json::{Value, json};

    use crate::amqp_server::{framed_fault, outgoing_headers, reply};

    fn faulted(fault: &Value) -> (&'static str, Value) {
        reply(&framed_fault(fault))
    }

    #[test]
    fn a_success_crosses_as_the_value_alone() {
        assert_eq!(
            reply(&json!({ "ok": true, "value": { "creditId": "64de3d95ff45b119e5b53ad1" } })),
            (
                "response",
                json!({ "creditId": "64de3d95ff45b119e5b53ad1" })
            ),
        );
    }

    #[test]
    fn a_value_crosses_with_every_field_the_caller_reads() {
        let value = json!({
            "type": "response",
            "organizationId": "acme",
            "availableCredits": [{
                "creditId": "64de3d95ff45b119e5b53ad1",
                "isPostPaid": false,
                "priority": 0_i32,
                "remainingCredits": 4_250_i32,
            }],
        });
        assert_eq!(
            reply(&json!({ "ok": true, "value": value })),
            ("response", value),
        );
    }

    #[test]
    fn a_unit_success_crosses_as_null() {
        assert_eq!(
            reply(&json!({ "ok": true, "value": null })),
            ("response", Value::Null)
        );
        assert_eq!(reply(&json!({ "ok": true })), ("response", Value::Null));
    }

    #[test]
    fn a_declared_error_crosses_with_every_field_it_carries() {
        let declared = json!({
            "errorCode": "resource-not-found",
            "errorMessage": "no guide is served at \"guide://data-dictionary\"",
            "uri": "guide://data-dictionary",
        });
        assert_eq!(
            reply(&json!({ "ok": false, "error": declared })),
            ("error", declared),
        );
    }

    #[test]
    fn a_declared_error_crosses_with_nothing_added() {
        for declared in [
            json!({ "errorCode": "not-found" }),
            json!({ "errorMessage": "Forbidden" }),
        ] {
            assert_eq!(
                reply(&json!({ "ok": false, "error": declared })),
                ("error", declared.clone()),
            );
        }
    }

    #[test]
    fn a_declared_error_that_is_not_an_object_crosses_as_it_was_written() {
        assert_eq!(
            reply(&json!({ "ok": false, "error": "Missing" })),
            ("error", json!("Missing")),
        );
    }

    #[test]
    fn a_declared_error_tagged_type_keeps_its_own_tag() {
        let declared = json!({ "type": "quota-exceeded", "limit": 3_i32 });
        assert_eq!(
            reply(&json!({ "ok": false, "error": declared })),
            ("error", declared),
        );
    }

    #[test]
    fn a_fault_crosses_as_the_fault_alone() {
        let fault = json!({
            "detail": "expected number, received string",
            "field": "creditCount",
            "kind": "failed-validation",
            "operation": "usage-generation-request",
        });
        assert_eq!(faulted(&fault), ("fault", fault));
    }

    #[test]
    fn a_fault_is_never_mistaken_for_an_error_the_operation_declared() {
        let (kind, _) = faulted(&json!({
            "detail": "the service answers to no operation by that name",
            "kind": "unknown-operation",
            "operation": "expire-generation-credit",
        }));
        assert_eq!(kind, "fault");
    }

    #[test]
    fn an_answer_that_is_no_message_is_reported_as_a_fault() {
        assert_eq!(
            reply(&json!("not a message")),
            (
                "fault",
                json!({
                    "detail": "the service answered with no message",
                    "kind": "undeserializable-payload",
                }),
            ),
        );
    }

    #[test]
    fn a_failure_arm_carrying_no_error_is_reported_as_a_fault() {
        assert_eq!(
            reply(&json!({ "ok": false })),
            (
                "fault",
                json!({
                    "detail": "the service answered with no error",
                    "kind": "undeserializable-payload",
                }),
            ),
        );
    }

    fn table(entries: Vec<(&str, AMQPValue)>) -> FieldTable {
        let mut table = FieldTable::default();
        for (name, value) in entries {
            table.insert(ShortString::from(name), value);
        }
        table
    }

    fn text(value: &str) -> AMQPValue {
        AMQPValue::LongString(LongString::from(value))
    }

    #[test]
    fn a_success_envelope_carries_no_is_error_header() {
        assert_eq!(
            outgoing_headers(&json!({ "ok": true, "value": {} }), Vec::new()),
            FieldTable::default(),
        );
    }

    #[test]
    fn a_declared_error_envelope_carries_the_is_error_header_as_a_boolean() {
        assert_eq!(
            outgoing_headers(
                &json!({ "ok": false, "error": { "errorCode": "db-error" } }),
                Vec::new(),
            ),
            table(vec![("is_error", AMQPValue::Boolean(true))]),
        );
    }

    #[test]
    fn a_framed_fault_carries_the_is_error_header_as_a_boolean() {
        let framed = framed_fault(&json!({
            "detail": "invalid type: string, expected u32",
            "kind": "undeserializable-payload",
            "operation": "add-generation-credit",
        }));
        assert_eq!(
            outgoing_headers(&framed, Vec::new()),
            table(vec![("is_error", AMQPValue::Boolean(true))]),
        );
    }

    #[test]
    fn a_declared_header_out_still_arrives_as_text_beside_is_error_on_an_error_envelope() {
        assert_eq!(
            outgoing_headers(
                &json!({ "ok": false, "error": { "errorCode": "db-error" } }),
                vec![("etag".to_owned(), "\"v1\"".to_owned())],
            ),
            table(vec![
                ("etag", text("\"v1\"")),
                ("is_error", AMQPValue::Boolean(true)),
            ]),
        );
    }

    #[test]
    fn a_declared_header_out_is_alone_on_a_success_envelope() {
        assert_eq!(
            outgoing_headers(
                &json!({ "ok": true, "value": {} }),
                vec![("etag".to_owned(), "\"v1\"".to_owned())],
            ),
            table(vec![("etag", text("\"v1\""))]),
        );
    }
}
