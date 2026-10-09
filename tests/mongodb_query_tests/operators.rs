//! One document per operator of each kind of path, asserted as text.

use bson::doc;
use bson::oid::ObjectId;

use super::{Address, Invoice, InvoiceId, InvoiceStatus, LineItem, Point, invoice_schema, shown};

fn line_item() -> LineItem {
    LineItem {
        article: "C-3".to_owned(),
        price: 4.0_f64,
        quantity: 9_u32,
    }
}

#[test]
fn a_comparison_writes_its_operator_over_a_value_of_the_fields_own_type() {
    let invoice = Invoice::MONGO_FIELDS;
    let id = InvoiceId(ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap());
    assert_eq!(
        shown(invoice.id.eq(id).unwrap()),
        r#"{ "_id": { "$eq": ObjectId(507f1f77bcf86cd799439011) } }"#
    );
    assert_eq!(
        shown(invoice.status.ne(InvoiceStatus::Draft).unwrap()),
        r#"{ "status": { "$ne": "draft" } }"#
    );
    assert_eq!(
        shown(invoice.total.lt(50.0_f64).unwrap()),
        r#"{ "total": { "$lt": Double(50.0) } }"#
    );
    assert_eq!(
        shown(invoice.customer.open_invoices.gte(2_u32).unwrap()),
        r#"{ "customer.openInvoices": { "$gte": Int64(2) } }"#
    );
    assert_eq!(
        shown(
            invoice
                .number
                .is_in(["INV-0042".to_owned(), "INV-0043".to_owned()])
                .unwrap()
        ),
        r#"{ "number": { "$in": ["INV-0042", "INV-0043"] } }"#
    );
    assert_eq!(
        shown(
            invoice
                .status
                .not_in([InvoiceStatus::Draft, InvoiceStatus::Paid])
                .unwrap()
        ),
        r#"{ "status": { "$nin": ["draft", "paid"] } }"#
    );
}

#[test]
fn a_pattern_is_matched_on_a_path_of_text() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.number.regex("^inv-", "i")),
        r#"{ "number": { "$regex": "^inv-", "$options": "i" } }"#
    );
    assert_eq!(
        shown(invoice.paid_at.regex("^2026-", "")),
        r#"{ "paidAt": { "$regex": "^2026-", "$options": "" } }"#
    );
    assert_eq!(
        shown(invoice.customer.name.regex("ltd$", "i")),
        r#"{ "customer.name": { "$regex": "ltd$", "$options": "i" } }"#
    );
}

#[test]
fn a_list_of_plain_values_is_matched_through_its_elements() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(
            invoice.scores.elem_match(
                invoice
                    .scores
                    .element()
                    .gte(80_u32)
                    .unwrap()
                    .lt(85_u32)
                    .unwrap()
            )
        ),
        r#"{ "scores": { "$elemMatch": { "$gte": Int64(80), "$lt": Int64(85) } } }"#
    );
    assert_eq!(
        shown(invoice.tags.size(0)),
        r#"{ "tags": { "$size": Int64(0) } }"#
    );
    assert_eq!(
        shown(invoice.tags.contains_none(["void".to_owned()]).unwrap()),
        r#"{ "tags": { "$nin": ["void"] } }"#
    );
}

#[test]
fn a_list_of_models_is_matched_by_a_filter_over_its_element() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.items.price.gt(100.0_f64).unwrap()),
        r#"{ "items.price": { "$gt": Double(100.0) } }"#
    );
    assert_eq!(
        shown(
            invoice.items.elem_match(
                LineItem::MONGO_FIELDS
                    .price
                    .gt(100.0_f64)
                    .unwrap()
                    .and(LineItem::MONGO_FIELDS.article.eq("B-7".to_owned()).unwrap())
            )
        ),
        r#"{ "items": { "$elemMatch": { "$and": [{ "price": { "$gt": Double(100.0) } }, { "sku": { "$eq": "B-7" } }] } } }"#
    );
    assert_eq!(
        shown(
            invoice
                .items
                .elem_match(LineItem::MONGO_FIELDS.quantity.gte(2_u32).unwrap())
        ),
        r#"{ "items": { "$elemMatch": { "quantity": { "$gte": Int64(2) } } } }"#
    );
    assert_eq!(
        shown(invoice.items.size(2)),
        r#"{ "items": { "$size": Int64(2) } }"#
    );
    assert_eq!(
        shown(invoice.items.push(line_item()).unwrap()),
        r#"{ "$push": { "items": { "sku": "C-3", "price": Double(4.0), "quantity": Int64(9) } } }"#
    );
    assert_eq!(
        shown(
            invoice
                .items
                .pull(LineItem::MONGO_FIELDS.quantity.lt(1_u32).unwrap())
        ),
        r#"{ "$pull": { "items": { "quantity": { "$lt": Int64(1) } } } }"#
    );
}

#[test]
fn a_model_a_row_may_leave_out_is_tested_for_removed_and_set_whole() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.billing.city.eq("Santo Domingo".to_owned()).unwrap()),
        r#"{ "billing.city": { "$eq": "Santo Domingo" } }"#
    );
    assert_eq!(
        shown(invoice.billing.exists(false)),
        r#"{ "billing": { "$exists": false } }"#
    );
    assert_eq!(
        shown(invoice.billing.unset()),
        r#"{ "$unset": { "billing": "" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .billing
                .set(Address {
                    city: "Moca".to_owned(),
                    postal_code: "56000".to_owned(),
                })
                .unwrap()
        ),
        r#"{ "$set": { "billing": { "city": "Moca", "postalCode": "56000" } } }"#
    );
}

#[test]
fn a_nested_model_is_compared_whole_and_by_each_path_below_it() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.origin.0.gt(18.0_f64).unwrap()),
        r#"{ "origin.0": { "$gt": Double(18.0) } }"#
    );
    assert_eq!(
        shown(invoice.origin.1.lt(-69.0_f64).unwrap()),
        r#"{ "origin.1": { "$lt": Double(-69.0) } }"#
    );
    assert_eq!(
        shown(invoice.origin.eq(Point(18.47_f64, -69.9_f64)).unwrap()),
        r#"{ "origin": { "$eq": [Double(18.47), Double(-69.9)] } }"#
    );
}

#[test]
fn an_update_writes_its_operator_and_several_merge_into_one() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.tags.push("reviewed".to_owned()).unwrap()),
        r#"{ "$push": { "tags": "reviewed" } }"#
    );
    assert_eq!(
        shown(invoice.tags.pull("draft".to_owned()).unwrap()),
        r#"{ "$pull": { "tags": "draft" } }"#
    );
    assert_eq!(
        shown(invoice.number.set_on_insert("INV-0099".to_owned()).unwrap()),
        r#"{ "$setOnInsert": { "number": "INV-0099" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .status
                .set(InvoiceStatus::Paid)
                .unwrap()
                .and(invoice.total.set(0.0_f64).unwrap())
                .and(invoice.customer.open_invoices.set(2_u32).unwrap())
                .and(invoice.paid_at.unset())
                .and(invoice.tags.push("settled".to_owned()).unwrap())
                .and(invoice.number.set_on_insert("INV-0099".to_owned()).unwrap())
        ),
        r#"{ "$set": { "status": "paid", "total": Double(0.0), "customer.openInvoices": Int64(2) }, "$unset": { "paidAt": "" }, "$push": { "tags": "settled" }, "$setOnInsert": { "number": "INV-0099" } }"#
    );
}

#[test]
fn a_path_a_row_may_leave_out_compares_and_sets_as_any_path_does() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.customer.open_invoices.lte(3_u32).unwrap()),
        r#"{ "customer.openInvoices": { "$lte": Int64(3) } }"#
    );
    assert_eq!(
        shown(invoice.paid_at.eq("2026-10-01".to_owned()).unwrap()),
        r#"{ "paidAt": { "$eq": "2026-10-01" } }"#
    );
    assert_eq!(
        shown(invoice.paid_at.set("2026-10-01".to_owned()).unwrap()),
        r#"{ "$set": { "paidAt": "2026-10-01" } }"#
    );
    assert_eq!(
        shown(invoice.paid_at.exists(false)),
        r#"{ "paidAt": { "$exists": false } }"#
    );
    assert_eq!(
        shown(invoice.tags.size(0).negated()),
        r#"{ "$nor": [{ "tags": { "$size": Int64(0) } }] }"#
    );
    assert_eq!(
        invoice.tags.size(0).into_document(),
        doc! { "tags": { "$size": 0_i64 } }
    );
    assert_eq!(
        invoice.paid_at.unset().into_document(),
        doc! { "$unset": { "paidAt": "" } }
    );
}

/// A list is tested for one value and for any of several, changed by element, and set whole.
#[test]
fn a_list_of_plain_values_is_tested_by_value_and_changed_by_element_or_whole() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.scores.contains(5_u32).unwrap()),
        r#"{ "scores": { "$eq": Int64(5) } }"#
    );
    assert_eq!(
        shown(
            invoice
                .tags
                .contains_any(["export".to_owned(), "priority".to_owned()])
                .unwrap()
        ),
        r#"{ "tags": { "$in": ["export", "priority"] } }"#
    );
    assert_eq!(
        shown(invoice.scores.pull(72_u32).unwrap()),
        r#"{ "$pull": { "scores": Int64(72) } }"#
    );
    assert_eq!(
        shown(invoice.scores.set([72_u32, 84_u32]).unwrap()),
        r#"{ "$set": { "scores": [Int64(72), Int64(84)] } }"#
    );
    assert_eq!(
        shown(invoice.tags.set_on_insert(["new".to_owned()]).unwrap()),
        r#"{ "$setOnInsert": { "tags": ["new"] } }"#
    );
    assert_eq!(
        shown(
            invoice
                .scores
                .elem_match(invoice.scores.element().eq(72_u32).unwrap())
        ),
        r#"{ "scores": { "$elemMatch": { "$eq": Int64(72) } } }"#
    );
    assert_eq!(
        shown(
            invoice.scores.elem_match(
                invoice
                    .scores
                    .element()
                    .ne(72_u32)
                    .unwrap()
                    .gt(10_u32)
                    .unwrap()
                    .lte(90_u32)
                    .unwrap()
            )
        ),
        r#"{ "scores": { "$elemMatch": { "$ne": Int64(72), "$gt": Int64(10), "$lte": Int64(90) } } }"#
    );
}

/// Every operator over a nested model's whole value, and a list of models set whole.
#[test]
fn a_nested_model_is_tested_and_set_as_one_whole_value() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.origin.ne(Point(0.0_f64, 0.0_f64)).unwrap()),
        r#"{ "origin": { "$ne": [Double(0.0), Double(0.0)] } }"#
    );
    assert_eq!(
        shown(
            invoice
                .origin
                .is_in([Point(18.47_f64, -69.9_f64), Point(0.0_f64, 0.0_f64)])
                .unwrap()
        ),
        r#"{ "origin": { "$in": [[Double(18.47), Double(-69.9)], [Double(0.0), Double(0.0)]] } }"#
    );
    assert_eq!(
        shown(invoice.origin.not_in([Point(0.0_f64, 0.0_f64)]).unwrap()),
        r#"{ "origin": { "$nin": [[Double(0.0), Double(0.0)]] } }"#
    );
    assert_eq!(
        shown(invoice.origin.set(Point(1.5_f64, 2.5_f64)).unwrap()),
        r#"{ "$set": { "origin": [Double(1.5), Double(2.5)] } }"#
    );
    assert_eq!(
        shown(
            invoice
                .origin
                .set_on_insert(Point(1.5_f64, 2.5_f64))
                .unwrap()
        ),
        r#"{ "$setOnInsert": { "origin": [Double(1.5), Double(2.5)] } }"#
    );
    assert_eq!(
        shown(invoice.items.set([line_item()]).unwrap()),
        r#"{ "$set": { "items": [{ "sku": "C-3", "price": Double(4.0), "quantity": Int64(9) }] } }"#
    );
}

/// BSON has no unsigned 64-bit integer.
#[test]
fn a_value_bson_cannot_hold_is_answered_as_an_error() {
    use invoice_schema::{Field, ListField, MongoPath};

    let sequence: Field<Invoice, u64> = Field::plain(MongoPath::under(MongoPath::ROOT, "sequence"));
    let marks: ListField<Invoice, u64> =
        ListField::plain(MongoPath::under(MongoPath::ROOT, "marks"));
    let refused = [
        sequence.eq(u64::MAX).unwrap_err(),
        sequence.is_in([1_u64, u64::MAX]).unwrap_err(),
        sequence.set(u64::MAX).unwrap_err(),
        marks.contains(u64::MAX).unwrap_err(),
        marks.contains_any([u64::MAX]).unwrap_err(),
        marks.element().gt(u64::MAX).unwrap_err(),
        marks.push(u64::MAX).unwrap_err(),
        marks.set([u64::MAX]).unwrap_err(),
    ];
    for refusal in refused {
        let told = refusal.to_string();
        assert!(told.contains("18446744073709551615"), "got: {told}");
    }
    assert_eq!(
        shown(sequence.eq(7_u64).unwrap()),
        r#"{ "sequence": { "$eq": Int64(7) } }"#
    );
}
