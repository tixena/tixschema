//! The query types of the BSON and MongoDB library: `mongodb_foundation!()` writes them once per
//! crate under the `mongodb` feature, and each type that lists `"mongodb"` in `enable` re-exports
//! them from its `{type}_schema` module.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

/// The comparisons a path of plain values has: the method, its operator, and what it asks.
const COMPARISONS: [(&str, &str, &str); 6] = [
    ("eq", "$eq", "equals"),
    ("ne", "$ne", "does not equal"),
    ("gt", "$gt", "is greater than"),
    ("gte", "$gte", "is at least"),
    ("lt", "$lt", "is less than"),
    ("lte", "$lte", "is at most"),
];

/// Everything `mongodb` adds to a flagged type's schema module.
pub fn query_items() -> TokenStream {
    let path = path_items();
    let filter = filter_items();
    let arm_filter = arm_filter_items();
    let update = update_items();
    let arm_update = arm_update_items();
    let nests = nests_items();
    let field = field_items();
    let optional_field = optional_field_items();
    let element = element_items();
    let list_field = list_field_items();
    let model = model_items();
    let optional_model = optional_model_items();
    let model_list = model_list_items();
    let nested = nested_items();
    quote! {
        #path
        #filter
        #arm_filter
        #update
        #arm_update
        #nests
        #field
        #optional_field
        #element
        #list_field
        #model
        #optional_model
        #model_list
        #nested
    }
}

/// `ArmFilter`: the rows of one arm of an enum serde writes under a tag, with the conditions on
/// the model that arm holds.
///
/// An arm filter is a value a `const` item holds, and its `and` takes a filter over the rows of
/// the model the arm holds. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Courier {
///     pub name: String,
///     pub region: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(tag = "type")]
/// pub enum Transport {
///     #[serde(rename = "by-courier")]
///     ByCourier(Courier),
///     Collected,
/// }
///
/// const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
///     Transport::MONGO_FIELDS.type__by_courier();
///
/// fn main() -> Result<(), transport_schema::WriteError> {
///     assert_eq!(
///         BY_COURIER.into_document().to_string(),
///         r#"{ "type": { "$eq": "by-courier" } }"#
///     );
///     let named = BY_COURIER.and(Courier::MONGO_FIELDS.name().eq("Acme".to_owned())?);
///     assert_eq!(
///         named.into_document().to_string(),
///         r#"{ "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme" } }] }"#
///     );
///     let others = transport_schema::Filter::from(BY_COURIER).negated();
///     assert_eq!(
///         others.into_document().to_string(),
///         r#"{ "$nor": [{ "type": { "$eq": "by-courier" } }] }"#
///     );
///     Ok(())
/// }
/// ```
///
/// The run below hands `and` a filter over the rows of `Wagon`, a type the arm does not hold. A
/// `compile_fail` doctest asserts only that some error was raised, so it was compiled standalone
/// as an ordinary test file, and the text under it is the only error it earned, verbatim:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Courier {
///     pub name: String,
///     pub region: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(tag = "type")]
/// pub enum Transport {
///     #[serde(rename = "by-courier")]
///     ByCourier(Courier),
///     Collected,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Wagon {
///     pub number: String,
/// }
///
/// const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
///     Transport::MONGO_FIELDS.type__by_courier();
///
/// fn main() -> Result<(), transport_schema::WriteError> {
///     let _ = BY_COURIER.and(Wagon::MONGO_FIELDS.number().eq("W-9".to_owned())?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Filter<Wagon>: AsRef<PhantomData<Courier>>` is not satisfied
///   --> tests/zz_probe.rs:34:28
///    |
/// 34 |     let _ = BY_COURIER.and(Wagon::MONGO_FIELDS.number().eq("W-9".to_owned())?);
///    |                        --- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                        |
///    |                        required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<Courier>>` is not implemented for `__tixschema_mongodb::Filter<Wagon>`
///       but trait `AsRef<PhantomData<Wagon>>` is implemented for it
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `Wagon`, found `Courier`
/// note: required by a bound in `ArmFilter::<Root, M>::and`
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `ArmFilter::<Root, M>::and`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn arm_filter_items() -> TokenStream {
    quote! {
        /// Rows of one arm of an enum serde writes under a tag: the arm's tag, and the conditions
        /// `and` adds on the model `M` the arm holds.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct ArmFilter<Root, M: ?::core::marker::Sized> {
            tag: &'static str,
            value: &'static str,
            place: MongoPath,
            conditions: ::std::vec::Vec<bson::Document>,
            of: ::core::marker::PhantomData<fn(&M) -> Root>,
        }

        impl<Root, M> ArmFilter<Root, M>
        where
            M: ?::core::marker::Sized,
        {
            /// The rows whose `tag` is `written`, with what the arm holds written at `place`.
            #[must_use]
            pub const fn of(tag: &'static str, written: &'static str, place: MongoPath) -> Self {
                Self { tag, value: written, place, conditions: ::std::vec::Vec::new(), of: ::core::marker::PhantomData }
            }

            /// `$and`: the arm's rows that `condition` matches too. `condition` is a filter over the
            /// rows of `M`, from any module, and each of its keys is written at the arm's place.
            #[must_use]
            pub fn and<F>(mut self, condition: F) -> Self
            where
                F: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<M>>,
            {
                self.conditions.push(nested_filter(self.place.key(), condition.into()));
                self
            }

            /// The arm's model changed by `change`, an update of the rows of `M`, from any module,
            /// each of its keys written at the arm's place. An update selects no row: the
            /// conditions `and` added are left behind, and the operation's own filter carries them.
            #[must_use]
            pub fn update<U>(self, change: U) -> ArmUpdate<Root, M>
            where
                U: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<fn(M) -> M>>,
            {
                ArmUpdate {
                    tag: self.tag,
                    value: self.value,
                    place: self.place,
                    document: nested_update(self.place.key(), change.into()),
                    of: ::core::marker::PhantomData,
                }
            }

            /// The arm's tag written on insert, and nothing else: the update of an arm that holds
            /// no model.
            #[must_use]
            pub fn tag_on_insert(self) -> ArmUpdate<Root, M> {
                ArmUpdate {
                    tag: self.tag,
                    value: self.value,
                    place: self.place,
                    document: bson::Document::new(),
                    of: ::core::marker::PhantomData,
                }
                .tag_on_insert()
            }

            /// The document the driver takes.
            #[must_use]
            pub fn into_document(self) -> bson::Document {
                self.into()
            }
        }

        impl<Root, M: ?::core::marker::Sized> ::core::convert::From<ArmFilter<Root, M>> for bson::Document {
            fn from(arm: ArmFilter<Root, M>) -> Self {
                let mut asked = Self::new();
                asked.insert("$eq", bson::Bson::String(arm.value.to_owned()));
                let mut tagged = Self::new();
                tagged.insert(arm.tag, bson::Bson::Document(asked));
                if arm.conditions.is_empty() {
                    return tagged;
                }
                let members = ::core::iter::once(tagged)
                    .chain(arm.conditions)
                    .map(bson::Bson::Document)
                    .collect::<::std::vec::Vec<bson::Bson>>();
                let mut joined = Self::new();
                joined.insert("$and", bson::Bson::Array(members));
                joined
            }
        }

        impl<Root, M: ?::core::marker::Sized> ::core::convert::From<ArmFilter<Root, M>> for Filter<Root> {
            fn from(arm: ArmFilter<Root, M>) -> Self {
                Self::raw(arm.into())
            }
        }

        impl<Root, M: ?::core::marker::Sized> ::core::convert::AsRef<::core::marker::PhantomData<Root>> for ArmFilter<Root, M> {
            fn as_ref(&self) -> &::core::marker::PhantomData<Root> {
                &::core::marker::PhantomData
            }
        }
    }
}

/// `ArmUpdate`: what `update` and `tag_on_insert` on an arm filter answer, an update of the rows
/// of the arm's enum.
///
/// An arm's `update` takes an update of the rows of the model the arm holds, and `tag_on_insert`
/// writes the arm's tag beside it. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Courier {
///     pub name: String,
///     pub region: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(tag = "type")]
/// pub enum Transport {
///     #[serde(rename = "by-courier")]
///     ByCourier(Courier),
///     Collected,
/// }
///
/// const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
///     Transport::MONGO_FIELDS.type__by_courier();
///
/// fn main() -> Result<(), transport_schema::WriteError> {
///     let renamed = BY_COURIER.update(Courier::MONGO_FIELDS.name().set("Acme".to_owned())?);
///     assert_eq!(
///         renamed.tag_on_insert().into_document().to_string(),
///         r#"{ "$set": { "name": "Acme" }, "$setOnInsert": { "type": "by-courier" } }"#
///     );
///     let collected = Transport::MONGO_FIELDS.type__collected().tag_on_insert();
///     assert_eq!(
///         transport_schema::Update::from(collected).into_document().to_string(),
///         r#"{ "$setOnInsert": { "type": "Collected" } }"#
///     );
///     Ok(())
/// }
/// ```
///
/// Each run below hands `update` something that is no update of the rows of `Courier`. A
/// `compile_fail` doctest asserts only that some error was raised, so each was compiled
/// standalone as an ordinary test file, and the text under it is the only error it earned,
/// verbatim.
///
/// A filter over the rows of `Courier`, which carries a filter's marker and not an update's:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Courier {
///     pub name: String,
///     pub region: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(tag = "type")]
/// pub enum Transport {
///     #[serde(rename = "by-courier")]
///     ByCourier(Courier),
///     Collected,
/// }
///
/// const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
///     Transport::MONGO_FIELDS.type__by_courier();
///
/// fn main() -> Result<(), transport_schema::WriteError> {
///     let _ = BY_COURIER.update(Courier::MONGO_FIELDS.name().eq("Acme".to_owned())?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Filter<Courier>: AsRef<PhantomData<fn(Courier) -> Courier>>` is not satisfied
///   --> tests/zz_probe.rs:28:31
///    |
/// 28 |     let _ = BY_COURIER.update(Courier::MONGO_FIELDS.name().eq("Acme".to_owned())?);
///    |                        ------ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                        |
///    |                        required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<fn(Courier) -> Courier>>` is not implemented for `__tixschema_mongodb::Filter<Courier>`
///       but trait `AsRef<PhantomData<Courier>>` is implemented for it
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `Courier`, found `fn(Courier) -> Courier`
/// note: required by a bound in `ArmFilter::<Root, M>::update`
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `ArmFilter::<Root, M>::update`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// An update of the rows of `Wagon`, a type the arm does not hold:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Courier {
///     pub name: String,
///     pub region: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(tag = "type")]
/// pub enum Transport {
///     #[serde(rename = "by-courier")]
///     ByCourier(Courier),
///     Collected,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Wagon {
///     pub number: String,
/// }
///
/// const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
///     Transport::MONGO_FIELDS.type__by_courier();
///
/// fn main() -> Result<(), transport_schema::WriteError> {
///     let _ = BY_COURIER.update(Wagon::MONGO_FIELDS.number().set("W-1".to_owned())?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Update<Wagon>: AsRef<PhantomData<fn(Courier) -> Courier>>` is not satisfied
///   --> tests/zz_probe.rs:34:31
///    |
/// 34 |     let _ = BY_COURIER.update(Wagon::MONGO_FIELDS.number().set("W-1".to_owned())?);
///    |                        ------ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                        |
///    |                        required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<fn(Courier) -> Courier>>` is not implemented for `__tixschema_mongodb::Update<Wagon>`
///       but trait `AsRef<PhantomData<fn(Wagon) -> Wagon>>` is implemented for it
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// note: required by a bound in `ArmFilter::<Root, M>::update`
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `ArmFilter::<Root, M>::update`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn arm_update_items() -> TokenStream {
    quote! {
        /// An update of the rows of one arm of an enum serde writes under a tag: what `update` and
        /// `and` change of the model `M` the arm holds, and the arm's tag where it is written on
        /// insert.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct ArmUpdate<Root, M: ?::core::marker::Sized> {
            tag: &'static str,
            value: &'static str,
            place: MongoPath,
            document: bson::Document,
            of: ::core::marker::PhantomData<fn(&M) -> Root>,
        }

        impl<Root, M> ArmUpdate<Root, M>
        where
            M: ?::core::marker::Sized,
        {
            /// Both updates as one. `change` is an update of the rows of `M`, from any module, and
            /// each of its keys is written at the arm's place.
            #[must_use]
            pub fn and<U>(mut self, change: U) -> Self
            where
                U: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<fn(M) -> M>>,
            {
                self.document = Update::<Root>::merged_update(self.document, nested_update(self.place.key(), change.into()));
                self
            }

            /// `$setOnInsert` of the arm's tag: what an upsert writes where it inserts the row.
            #[must_use]
            pub fn tag_on_insert(mut self) -> Self {
                let mut tagged = bson::Document::new();
                tagged.insert(self.tag, bson::Bson::String(self.value.to_owned()));
                let mut written = bson::Document::new();
                written.insert("$setOnInsert", bson::Bson::Document(tagged));
                self.document = Update::<Root>::merged_update(self.document, written);
                self
            }

            /// The document the driver takes.
            #[must_use]
            pub fn into_document(self) -> bson::Document {
                self.document
            }
        }

        impl<Root, M: ?::core::marker::Sized> ::core::convert::From<ArmUpdate<Root, M>> for bson::Document {
            fn from(arm: ArmUpdate<Root, M>) -> Self {
                arm.document
            }
        }

        impl<Root, M: ?::core::marker::Sized> ::core::convert::From<ArmUpdate<Root, M>> for Update<Root> {
            fn from(arm: ArmUpdate<Root, M>) -> Self {
                Self::raw(arm.document)
            }
        }

        impl<Root, M: ?::core::marker::Sized> ::core::convert::AsRef<::core::marker::PhantomData<fn(Root) -> Root>> for ArmUpdate<Root, M> {
            fn as_ref(&self) -> &::core::marker::PhantomData<fn(Root) -> Root> {
                &::core::marker::PhantomData
            }
        }
    }
}

/// `Element`: the conditions one element of a list of plain values is held to.
fn element_items() -> TokenStream {
    let comparisons = COMPARISONS.iter().map(|&(method, operator, asks)| {
        let name = Ident::new(method, Span::call_site());
        let told = format!("`{operator}`: the element {asks} `value`.");
        quote! {
            #[doc = #told]
            pub fn #name(self, value: V) -> ::core::result::Result<Self, WriteError> {
                self.held_to(#operator, value)
            }
        }
    });
    quote! {
        /// The conditions one element of a list of `V` is held to, for `$elemMatch`.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct Element<V> {
            conditions: bson::Document,
            write: fn(V) -> ::core::result::Result<bson::Bson, WriteError>,
        }

        impl<V> Element<V> {
            fn held_to(mut self, operator: &str, value: V) -> ::core::result::Result<Self, WriteError> {
                self.conditions.insert(operator, (self.write)(value)?);
                ::core::result::Result::Ok(self)
            }

            #(#comparisons)*
        }
    }
}

/// `Field`: the path of one value every row holds, with the operators that compare and set it.
///
/// A path takes a value of its field's own type and nothing else. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{Field, MongoPath, OptionalField, WriteError};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "kebab-case")]
/// pub enum InvoiceStatus {
///     Draft,
///     PastDue,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Invoice {
///     pub number: String,
///     pub open_invoices: u32,
///     #[serde(default, skip_serializing_if = "Option::is_none")]
///     pub paid_at: Option<String>,
///     pub status: InvoiceStatus,
///     pub total: f64,
/// }
///
/// const OPEN_INVOICES: Field<Invoice, u32> = Field::plain(MongoPath::at("openInvoices"));
/// const PAID_AT: OptionalField<Invoice, String> = OptionalField::plain(MongoPath::at("paidAt"));
/// const STATUS: Field<Invoice, InvoiceStatus> = Field::plain(MongoPath::at("status"));
/// const TOTAL: Field<Invoice, f64> = Field::plain(MongoPath::at("total"));
///
/// fn main() -> Result<(), WriteError> {
///     let _ = STATUS.eq(InvoiceStatus::PastDue)?;
///     let _ = OPEN_INVOICES.lte(3)?;
///     let _ = TOTAL.gt(1000.0)?;
///     let _ = PAID_AT.unset();
///     Ok(())
/// }
/// ```
///
/// Each run below declares one such path alone and asks of it what its kind refuses. A
/// `compile_fail` doctest asserts only that some error was raised, so each was compiled
/// standalone as an ordinary test file, and the text under it is the only error it earned,
/// verbatim.
///
/// `$unset` on a field every row holds:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{Field, MongoPath};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub number: String,
/// }
///
/// const NUMBER: Field<Invoice, String> = Field::plain(MongoPath::at("number"));
///
/// fn main() {
///     let _ = NUMBER.unset();
/// }
/// ```
///
/// ```text
/// error[E0599]: no method named `unset` found for struct `__tixschema_mongodb::Field<Root, V, N>` in the current scope
///   --> tests/zz_probe.rs:19:20
///    |
///  8 | tixschema::mongodb_foundation!();
///    | -------------------------------- method `unset` not found for this struct
/// ...
/// 19 |     let _ = NUMBER.unset();
///    |                    ^^^^^
///    |
/// help: there is a method `set` with a similar name, but with different arguments
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// Text for a field typed with an enum:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{Field, MongoPath};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "kebab-case")]
/// pub enum InvoiceStatus {
///     Draft,
///     PastDue,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub status: InvoiceStatus,
/// }
///
/// const STATUS: Field<Invoice, InvoiceStatus> = Field::plain(MongoPath::at("status"));
///
/// fn main() {
///     let _ = STATUS.eq("past-due");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:27:23
///    |
/// 27 |     let _ = STATUS.eq("past-due");
///    |                    -- ^^^^^^^^^^ expected `InvoiceStatus`, found `&str`
///    |                    |
///    |                    arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// Text for a `u32` field:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{Field, MongoPath};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Invoice {
///     pub open_invoices: u32,
/// }
///
/// const OPEN_INVOICES: Field<Invoice, u32> = Field::plain(MongoPath::at("openInvoices"));
///
/// fn main() {
///     let _ = OPEN_INVOICES.lte("3");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:20:31
///    |
/// 20 |     let _ = OPEN_INVOICES.lte("3");
///    |                           --- ^^^ expected `u32`, found `&str`
///    |                           |
///    |                           arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// Text for an `f64` field:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{Field, MongoPath};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// const TOTAL: Field<Invoice, f64> = Field::plain(MongoPath::at("total"));
///
/// fn main() {
///     let _ = TOTAL.gt("1000");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:19:22
///    |
/// 19 |     let _ = TOTAL.gt("1000");
///    |                   -- ^^^^^^ expected `f64`, found `&str`
///    |                   |
///    |                   arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn field_items() -> TokenStream {
    let operators = field_operators();
    let nest = nest_method(&quote! { self.path.key() });
    quote! {
        /// The path of a value of type `V` every row of `Root` holds. `N` is the model `nest`
        /// reaches under it: `V`, or what a `Box` or its like written as `V` holds.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct Field<Root, V, N: ?::core::marker::Sized = V> {
            path: MongoPath,
            write: fn(V) -> ::core::result::Result<bson::Bson, WriteError>,
            of: ::core::marker::PhantomData<fn(&N) -> Root>,
        }

        impl<Root, V, N> Field<Root, V, N>
        where
            N: ?::core::marker::Sized,
        {
            /// A path whose values the type's own `Serialize` writes.
            #[must_use]
            pub const fn plain(path: MongoPath) -> Self
            where
                V: ::serde::Serialize,
            {
                Self { path, write: write_plain::<V>, of: ::core::marker::PhantomData }
            }

            /// A path whose values `write` writes: the field's own serde hook.
            #[must_use]
            pub const fn hooked(path: MongoPath, write: fn(V) -> ::core::result::Result<bson::Bson, WriteError>) -> Self {
                Self { path, write, of: ::core::marker::PhantomData }
            }

            /// The key, which the typed paths a type of the crate writes read.
            pub(crate) const fn held_path(&self) -> MongoPath {
                self.path
            }

            fn compared(&self, operator: &str, value: V) -> ::core::result::Result<Filter<Root>, WriteError> {
                ::core::result::Result::Ok(Filter::held(self.path.key(), operator, (self.write)(value)?))
            }

            fn listed<I>(&self, operator: &str, values: I) -> ::core::result::Result<Filter<Root>, WriteError>
            where
                I: ::core::iter::IntoIterator<Item = V>,
            {
                let written = values
                    .into_iter()
                    .map(self.write)
                    .collect::<::core::result::Result<::std::vec::Vec<bson::Bson>, WriteError>>()?;
                ::core::result::Result::Ok(Filter::held(self.path.key(), operator, bson::Bson::Array(written)))
            }

            #operators

            #nest
        }

        impl<Root, N> Field<Root, ::std::string::String, N>
        where
            N: ?::core::marker::Sized,
        {
            /// `$regex` with `$options`: rows whose text at this path matches `pattern`.
            #[must_use]
            pub fn regex(&self, pattern: &str, options: &str) -> Filter<Root> {
                let mut condition = bson::Document::new();
                condition.insert("$regex", pattern);
                condition.insert("$options", options);
                let mut document = bson::Document::new();
                document.insert(self.path.key(), bson::Bson::Document(condition));
                Filter::raw(document)
            }
        }
    }
}

/// The operators of a `Field`: each takes a value of the field's own type, by value.
fn field_operators() -> TokenStream {
    let comparisons = COMPARISONS.iter().map(|&(method, operator, asks)| {
        let name = Ident::new(method, Span::call_site());
        let told = format!("`{operator}`: rows whose value at this path {asks} `value`.");
        quote! {
            #[doc = #told]
            pub fn #name(&self, value: V) -> ::core::result::Result<Filter<Root>, WriteError> {
                self.compared(#operator, value)
            }
        }
    });
    quote! {
        #(#comparisons)*

        /// `$in`: rows whose value at this path is one of `values`.
        pub fn is_in<I>(&self, values: I) -> ::core::result::Result<Filter<Root>, WriteError>
        where
            I: ::core::iter::IntoIterator<Item = V>,
        {
            self.listed("$in", values)
        }

        /// `$nin`: rows whose value at this path is none of `values`.
        pub fn not_in<I>(&self, values: I) -> ::core::result::Result<Filter<Root>, WriteError>
        where
            I: ::core::iter::IntoIterator<Item = V>,
        {
            self.listed("$nin", values)
        }

        /// `$set`: puts `value` at this path.
        pub fn set(&self, value: V) -> ::core::result::Result<Update<Root>, WriteError> {
            ::core::result::Result::Ok(Update::held("$set", self.path.key(), (self.write)(value)?))
        }

        /// `$setOnInsert`: puts `value` at this path in a row the update inserts.
        pub fn set_on_insert(&self, value: V) -> ::core::result::Result<Update<Root>, WriteError> {
            ::core::result::Result::Ok(Update::held("$setOnInsert", self.path.key(), (self.write)(value)?))
        }
    }
}

/// `Filter`: the document a read takes, over the rows of one type.
///
/// A filter joins a filter over the same rows, whichever module's `Filter` it is. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use bson::doc;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Order {
///     pub placed: bool,
/// }
///
/// const PLACED: order_schema::Field<Invoice, bool> =
///     order_schema::Field::plain(order_schema::MongoPath::at("placed"));
/// const TOTAL: invoice_schema::Field<Invoice, f64> =
///     invoice_schema::Field::plain(invoice_schema::MongoPath::at("total"));
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let _ = TOTAL
///         .gt(1000.0)?
///         .and(PLACED.eq(true)?)
///         .and(invoice_schema::Filter::<Invoice>::raw(doc! { "details.po": "7781" }));
///     Ok(())
/// }
/// ```
///
/// Each run below hands `and` something that is no filter over the rows of `Invoice`. A
/// `compile_fail` doctest asserts only that some error was raised, so each was compiled
/// standalone as an ordinary test file, and the text under it is the only error it earned,
/// verbatim.
///
/// `PLACED` declared over the rows of `Order`, so its filter is one over another type's rows:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Order {
///     pub placed: bool,
/// }
///
/// const PLACED: order_schema::Field<Order, bool> =
///     order_schema::Field::plain(order_schema::MongoPath::at("placed"));
/// const TOTAL: invoice_schema::Field<Invoice, f64> =
///     invoice_schema::Field::plain(invoice_schema::MongoPath::at("total"));
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let _ = TOTAL.gt(1000.0)?.and(PLACED.eq(true)?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Filter<Order>: AsRef<PhantomData<Invoice>>` is not satisfied
///   --> tests/zz_probe.rs:26:35
///    |
/// 26 |     let _ = TOTAL.gt(1000.0)?.and(PLACED.eq(true)?);
///    |                               --- ^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                               |
///    |                               required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<Invoice>>` is not implemented for `__tixschema_mongodb::Filter<Order>`
///       but trait `AsRef<PhantomData<Order>>` is implemented for it
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `Order`, found `Invoice`
/// note: required by a bound in `__tixschema_mongodb::Filter::<Root>::and`
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Filter::<Root>::and`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// A document handed over as it stands, without `Filter::raw`:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use bson::doc;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// const TOTAL: invoice_schema::Field<Invoice, f64> =
///     invoice_schema::Field::plain(invoice_schema::MongoPath::at("total"));
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let _ = TOTAL.gt(1000.0)?.and(doc! { "details.po": "7781" });
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `bson::Document: AsRef<PhantomData<Invoice>>` is not satisfied
///   --> tests/zz_probe.rs:19:35
///    |
/// 19 |     let _ = TOTAL.gt(1000.0)?.and(doc! { "details.po": "7781" });
///    |                               --- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `AsRef<PhantomData<Invoice>>` is not implemented for `bson::Document`
///    |                               |
///    |                               required by a bound introduced by this call
///    |
/// note: required by a bound in `__tixschema_mongodb::Filter::<Root>::and`
///   --> tests/zz_probe.rs:7:1
///    |
///  7 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Filter::<Root>::and`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// An update over the same rows, which carries an update's marker and not a filter's:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// const TOTAL: invoice_schema::Field<Invoice, f64> =
///     invoice_schema::Field::plain(invoice_schema::MongoPath::at("total"));
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let _ = TOTAL.gt(1000.0)?.and(TOTAL.set(0.0)?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Update<Invoice>: AsRef<PhantomData<Invoice>>` is not satisfied
///   --> tests/zz_probe.rs:18:35
///    |
/// 18 |     let _ = TOTAL.gt(1000.0)?.and(TOTAL.set(0.0)?);
///    |                               --- ^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                               |
///    |                               required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<Invoice>>` is not implemented for `__tixschema_mongodb::Update<Invoice>`
///       but trait `AsRef<PhantomData<fn(Invoice) -> Invoice>>` is implemented for it
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `fn(Invoice) -> Invoice`, found `Invoice`
/// note: required by a bound in `__tixschema_mongodb::Filter::<Root>::and`
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Filter::<Root>::and`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn filter_items() -> TokenStream {
    quote! {
        /// A filter over the rows of `Root`.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct Filter<Root> {
            document: bson::Document,
            root: ::core::marker::PhantomData<fn() -> Root>,
        }

        impl<Root> Filter<Root> {
            /// A filter written as a document, for a path that has no typed form.
            #[must_use]
            pub fn raw(document: bson::Document) -> Self {
                Self { document, root: ::core::marker::PhantomData }
            }

            /// `{ key: { operator: value } }`, which the typed paths a type of the crate writes
            /// build an arm's filter with.
            pub(crate) fn held(key: &str, operator: &str, value: bson::Bson) -> Self {
                let mut condition = bson::Document::new();
                condition.insert(operator, value);
                let mut document = bson::Document::new();
                document.insert(key, bson::Bson::Document(condition));
                Self::raw(document)
            }

            fn joined<F>(self, operator: &str, other: F) -> Self
            where
                F: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<Root>>,
            {
                let mut document = self.document;
                if document.len() == 1
                    && let ::core::option::Option::Some(bson::Bson::Array(members)) = document.get_mut(operator)
                {
                    members.push(bson::Bson::Document(other.into()));
                    return Self::raw(document);
                }
                let mut joined = bson::Document::new();
                joined.insert(operator, bson::Bson::Array(::std::vec![bson::Bson::Document(document), bson::Bson::Document(other.into())]));
                Self::raw(joined)
            }

            /// `$and`: rows both filters match. `other` is a filter over the same rows, from any
            /// module.
            #[must_use]
            pub fn and<F>(self, other: F) -> Self
            where
                F: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<Root>>,
            {
                self.joined("$and", other)
            }

            /// `$or`: rows either filter matches.
            #[must_use]
            pub fn or<F>(self, other: F) -> Self
            where
                F: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<Root>>,
            {
                self.joined("$or", other)
            }

            /// `$nor` of this one filter: rows it does not match.
            #[must_use]
            pub fn negated(self) -> Self {
                let mut document = bson::Document::new();
                document.insert("$nor", bson::Bson::Array(::std::vec![bson::Bson::Document(self.document)]));
                Self::raw(document)
            }

            /// The document the driver takes.
            #[must_use]
            pub fn into_document(self) -> bson::Document {
                self.document
            }
        }

        impl<Root> ::core::convert::From<Filter<Root>> for bson::Document {
            fn from(filter: Filter<Root>) -> Self {
                filter.document
            }
        }

        impl<Root> ::core::convert::AsRef<::core::marker::PhantomData<Root>> for Filter<Root> {
            fn as_ref(&self) -> &::core::marker::PhantomData<Root> {
                &::core::marker::PhantomData
            }
        }
    }
}

/// `ListField`: the path of a list of plain values, with the operators over its elements.
///
/// A list is matched through its elements. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{ListField, MongoPath, WriteError};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub tags: Vec<String>,
/// }
///
/// const TAGS: ListField<Invoice, String> = ListField::plain(MongoPath::at("tags"));
///
/// fn main() -> Result<(), WriteError> {
///     let _ = TAGS.contains("export".to_owned())?;
///     Ok(())
/// }
/// ```
///
/// The run below is that one with `contains` written `gt`, and nothing else changed. A
/// `compile_fail` doctest asserts only that some error was raised, so it was compiled standalone
/// as an ordinary test file, and the text under it is the only error it earned. It is verbatim
/// but for one note, which quotes the standard library's own `Iterator` at a path of the machine
/// that compiled it: the second `...` stands for that note.
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{ListField, MongoPath, WriteError};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub tags: Vec<String>,
/// }
///
/// const TAGS: ListField<Invoice, String> = ListField::plain(MongoPath::at("tags"));
///
/// fn main() -> Result<(), WriteError> {
///     let _ = TAGS.gt("export".to_owned())?;
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0599]: `ListField<Invoice, std::string::String>` is not an iterator
///   --> tests/zz_probe.rs:19:18
///    |
///  8 | tixschema::mongodb_foundation!();
///    | -------------------------------- method `gt` not found for this struct because it doesn't satisfy `ListField<Invoice, std::string::String>: Iterator`
/// ...
/// 19 |     let _ = TAGS.gt("export".to_owned())?;
///    |                  ^^ `ListField<Invoice, std::string::String>` is not an iterator
///    |
///    = note: the following trait bounds were not satisfied:
///            `ListField<Invoice, std::string::String>: Iterator`
///            which is required by `&mut ListField<Invoice, std::string::String>: Iterator`
/// note: the trait `Iterator` must be implemented
///   --> $RUST/core/src/iter/traits/iterator.rs:42:1
///    |
/// 42 | pub const trait Iterator {
///    | ^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: items from traits can only be used if the trait is implemented and in scope
///    = note: the following traits define an item `gt`, perhaps you need to implement one of them:
///            candidate #1: `Iterator`
///            candidate #2: `PartialOrd`
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn list_field_items() -> TokenStream {
    let operators = list_field_operators();
    let nest = nest_method(&quote! { self.path.key() });
    quote! {
        /// The path of a list of plain values of type `V`. `N` is the model `nest` reaches under
        /// it: `V`, or what `V` wraps.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct ListField<Root, V, N: ?::core::marker::Sized = V> {
            path: MongoPath,
            write: fn(V) -> ::core::result::Result<bson::Bson, WriteError>,
            of: ::core::marker::PhantomData<fn(&N) -> Root>,
        }

        impl<Root, V, N> ListField<Root, V, N>
        where
            N: ?::core::marker::Sized,
        {
            /// A list whose elements the type's own `Serialize` writes.
            #[must_use]
            pub const fn plain(path: MongoPath) -> Self
            where
                V: ::serde::Serialize,
            {
                Self { path, write: write_plain::<V>, of: ::core::marker::PhantomData }
            }

            /// A list whose elements `write` writes: the field's own serde hook.
            #[must_use]
            pub const fn hooked(path: MongoPath, write: fn(V) -> ::core::result::Result<bson::Bson, WriteError>) -> Self {
                Self { path, write, of: ::core::marker::PhantomData }
            }

            fn written<I>(&self, values: I) -> ::core::result::Result<bson::Bson, WriteError>
            where
                I: ::core::iter::IntoIterator<Item = V>,
            {
                ::core::result::Result::Ok(bson::Bson::Array(
                    values
                        .into_iter()
                        .map(self.write)
                        .collect::<::core::result::Result<::std::vec::Vec<bson::Bson>, WriteError>>()?,
                ))
            }

            #operators

            #nest
        }
    }
}

/// The operators of a `ListField`: over one element, over several, and over the list itself.
fn list_field_operators() -> TokenStream {
    quote! {
        /// `$eq`: rows whose list at this path holds `value`.
        pub fn contains(&self, value: V) -> ::core::result::Result<Filter<Root>, WriteError> {
            ::core::result::Result::Ok(Filter::held(self.path.key(), "$eq", (self.write)(value)?))
        }

        /// `$in`: rows whose list at this path holds one of `values`.
        pub fn contains_any<I>(&self, values: I) -> ::core::result::Result<Filter<Root>, WriteError>
        where
            I: ::core::iter::IntoIterator<Item = V>,
        {
            ::core::result::Result::Ok(Filter::held(self.path.key(), "$in", self.written(values)?))
        }

        /// `$nin`: rows whose list at this path holds none of `values`.
        pub fn contains_none<I>(&self, values: I) -> ::core::result::Result<Filter<Root>, WriteError>
        where
            I: ::core::iter::IntoIterator<Item = V>,
        {
            ::core::result::Result::Ok(Filter::held(self.path.key(), "$nin", self.written(values)?))
        }

        /// `$size`: rows whose list at this path holds `length` elements.
        #[must_use]
        pub fn size(&self, length: u32) -> Filter<Root> {
            Filter::held(self.path.key(), "$size", bson::Bson::Int64(::core::primitive::i64::from(length)))
        }

        /// No condition yet on one element; `elem_match` takes what it is then held to.
        #[must_use]
        pub fn element(&self) -> Element<V> {
            Element { conditions: bson::Document::new(), write: self.write }
        }

        /// `$elemMatch`: rows where one element meets every condition of `element`.
        #[must_use]
        pub fn elem_match(&self, element: Element<V>) -> Filter<Root> {
            Filter::held(self.path.key(), "$elemMatch", bson::Bson::Document(element.conditions))
        }

        /// `$push`: adds `value` to the list at this path.
        pub fn push(&self, value: V) -> ::core::result::Result<Update<Root>, WriteError> {
            ::core::result::Result::Ok(Update::held("$push", self.path.key(), (self.write)(value)?))
        }

        /// `$pull`: takes every element equal to `value` out of the list at this path.
        pub fn pull(&self, value: V) -> ::core::result::Result<Update<Root>, WriteError> {
            ::core::result::Result::Ok(Update::held("$pull", self.path.key(), (self.write)(value)?))
        }

        /// `$set`: puts `values` at this path as the whole list.
        pub fn set<I>(&self, values: I) -> ::core::result::Result<Update<Root>, WriteError>
        where
            I: ::core::iter::IntoIterator<Item = V>,
        {
            ::core::result::Result::Ok(Update::held("$set", self.path.key(), self.written(values)?))
        }

        /// `$setOnInsert`: puts `values` at this path as the whole list, in a row the update
        /// inserts.
        pub fn set_on_insert<I>(&self, values: I) -> ::core::result::Result<Update<Root>, WriteError>
        where
            I: ::core::iter::IntoIterator<Item = V>,
        {
            ::core::result::Result::Ok(Update::held("$setOnInsert", self.path.key(), self.written(values)?))
        }
    }
}

/// `Model`: the path of a nested model every row holds. It carries the operators over the whole
/// value, and `nest` writes what is asked of the model's own paths under its key.
fn model_items() -> TokenStream {
    let nest = nest_method(&quote! { self.whole.path.key() });
    quote! {
        /// The path of a nested model `M` every row of `Root` holds, with the operators over the
        /// whole value. `N` is the model `nest` reaches under it: `M`, or what `M` wraps.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct Model<Root, M, N: ?::core::marker::Sized = M> {
            whole: Field<Root, M, N>,
        }

        impl<Root, M, N> Model<Root, M, N>
        where
            N: ?::core::marker::Sized,
        {
            /// A nested model whose own `Serialize` writes it.
            #[must_use]
            pub const fn plain(path: MongoPath) -> Self
            where
                M: ::serde::Serialize,
            {
                Self { whole: Field::plain(path) }
            }

            /// `$eq`: rows whose whole value at this path equals `value`.
            pub fn eq(&self, value: M) -> ::core::result::Result<Filter<Root>, WriteError> {
                self.whole.eq(value)
            }

            /// `$ne`: rows whose whole value at this path does not equal `value`.
            pub fn ne(&self, value: M) -> ::core::result::Result<Filter<Root>, WriteError> {
                self.whole.ne(value)
            }

            /// `$in`: rows whose whole value at this path is one of `values`.
            pub fn is_in<I>(&self, values: I) -> ::core::result::Result<Filter<Root>, WriteError>
            where
                I: ::core::iter::IntoIterator<Item = M>,
            {
                self.whole.is_in(values)
            }

            /// `$nin`: rows whose whole value at this path is none of `values`.
            pub fn not_in<I>(&self, values: I) -> ::core::result::Result<Filter<Root>, WriteError>
            where
                I: ::core::iter::IntoIterator<Item = M>,
            {
                self.whole.not_in(values)
            }

            /// `$set`: puts `value` at this path, whole.
            pub fn set(&self, value: M) -> ::core::result::Result<Update<Root>, WriteError> {
                self.whole.set(value)
            }

            /// `$setOnInsert`: puts `value` at this path, whole, in a row the update inserts.
            pub fn set_on_insert(&self, value: M) -> ::core::result::Result<Update<Root>, WriteError> {
                self.whole.set_on_insert(value)
            }

            #nest
        }
    }
}

/// `ModelList`: the path of a list of nested models. Its own operators are over the list and its
/// elements, and what `nest` writes under its key matches where any element does.
///
/// One element is held to a filter over the rows of the element's own type. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{ModelList, MongoPath, WriteError};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct LineItem {
///     pub quantity: u32,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub items: Vec<LineItem>,
/// }
///
/// const ITEMS: ModelList<Invoice, LineItem> = ModelList::plain(MongoPath::at("items"));
/// const QUANTITY: line_item_schema::Field<LineItem, u32> =
///     line_item_schema::Field::plain(line_item_schema::MongoPath::at("quantity"));
///
/// fn main() -> Result<(), WriteError> {
///     let _ = ITEMS.elem_match(QUANTITY.gte(2)?);
///     Ok(())
/// }
/// ```
///
/// The run below is that one with `gte` written `set`, an update over the element where a filter
/// is asked, and nothing else changed. A `compile_fail` doctest asserts only that some error was
/// raised, so it was compiled standalone as an ordinary test file, and the text under it is the
/// only error it earned, verbatim:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{ModelList, MongoPath, WriteError};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct LineItem {
///     pub quantity: u32,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub items: Vec<LineItem>,
/// }
///
/// const ITEMS: ModelList<Invoice, LineItem> = ModelList::plain(MongoPath::at("items"));
/// const QUANTITY: line_item_schema::Field<LineItem, u32> =
///     line_item_schema::Field::plain(line_item_schema::MongoPath::at("quantity"));
///
/// fn main() -> Result<(), WriteError> {
///     let _ = ITEMS.elem_match(QUANTITY.set(2)?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Update<LineItem>: AsRef<PhantomData<LineItem>>` is not satisfied
///   --> tests/zz_probe.rs:27:30
///    |
/// 27 |     let _ = ITEMS.elem_match(QUANTITY.set(2)?);
///    |                   ---------- ^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                   |
///    |                   required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<LineItem>>` is not implemented for `__tixschema_mongodb::Update<LineItem>`
///       but trait `AsRef<PhantomData<fn(LineItem) -> LineItem>>` is implemented for it
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `fn(LineItem) -> LineItem`, found `LineItem`
/// note: required by a bound in `ModelList::<Root, M, N>::elem_match`
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `ModelList::<Root, M, N>::elem_match`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn model_list_items() -> TokenStream {
    let nest = nest_method(&quote! { self.whole.path.key() });
    quote! {
        /// The path of a list of nested models `M`. `N` is the model `nest` reaches under it,
        /// where any element matches, and the one `elem_match` and `pull` take a filter over:
        /// `M`, or what `M` wraps.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct ModelList<Root, M, N: ?::core::marker::Sized = M> {
            whole: ListField<Root, M, N>,
        }

        impl<Root, M, N> ModelList<Root, M, N>
        where
            N: ?::core::marker::Sized,
        {
            /// A list of nested models whose own `Serialize` writes each.
            #[must_use]
            pub const fn plain(path: MongoPath) -> Self
            where
                M: ::serde::Serialize,
            {
                Self { whole: ListField::plain(path) }
            }

            /// `$elemMatch`: rows where one element matches `matching`, a filter over the rows
            /// of `N`.
            #[must_use]
            pub fn elem_match<E>(&self, matching: E) -> Filter<Root>
            where
                E: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<N>>,
            {
                Filter::held(self.whole.path.key(), "$elemMatch", bson::Bson::Document(matching.into()))
            }

            /// `$size`: rows whose list at this path holds `length` elements.
            #[must_use]
            pub fn size(&self, length: u32) -> Filter<Root> {
                self.whole.size(length)
            }

            /// `$push`: adds `value` to the list at this path.
            pub fn push(&self, value: M) -> ::core::result::Result<Update<Root>, WriteError> {
                self.whole.push(value)
            }

            /// `$pull`: takes every element `matching` matches out of the list at this path.
            /// `matching` is a filter over the rows of `N`.
            #[must_use]
            pub fn pull<E>(&self, matching: E) -> Update<Root>
            where
                E: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<N>>,
            {
                Update::held("$pull", self.whole.path.key(), bson::Bson::Document(matching.into()))
            }

            /// `$set`: puts `values` at this path as the whole list.
            pub fn set<I>(&self, values: I) -> ::core::result::Result<Update<Root>, WriteError>
            where
                I: ::core::iter::IntoIterator<Item = M>,
            {
                self.whole.set(values)
            }

            #nest
        }
    }
}

/// `nest`, as each path kind that reaches a nested model declares it over the key `key` reads.
fn nest_method(key: &TokenStream) -> TokenStream {
    quote! {
        /// What `inner` asks of an `N`, asked of the `N` this path holds: `inner` is a filter
        /// over the rows of `N` or an update of them, from any module, and each of its keys is
        /// written under this path's.
        #[must_use]
        pub fn nest<Kind, E>(&self, inner: E) -> E::Nested
        where
            E: Nests<Root, N, Kind>,
        {
            inner.nested_under(#key)
        }
    }
}

/// `Nested`: where a row writes a nested model under no path of a kind of its own, the key
/// `nest` writes that model's under.
fn nested_items() -> TokenStream {
    let nest = nest_method(&quote! { self.path.key() });
    quote! {
        /// Where a row of `Root` writes what a nested model `N` holds: `nest` reaches it, and
        /// nothing is asked of the whole value. The key is empty where the model's keys are
        /// written among those of the type that holds it.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct Nested<Root, N: ?::core::marker::Sized> {
            path: MongoPath,
            of: ::core::marker::PhantomData<fn(&N) -> Root>,
        }

        impl<Root, N> Nested<Root, N>
        where
            N: ?::core::marker::Sized,
        {
            /// What a nested model holds, written under `path`.
            #[must_use]
            pub const fn at(path: MongoPath) -> Self {
                Self { path, of: ::core::marker::PhantomData }
            }

            #nest
        }
    }
}

/// `Nests`, the bound `nest` takes a filter or an update by, with the two markers that tell
/// them apart and the functions that write each under a key.
///
/// `nest` takes a filter over the rows of the model its path holds, or an update of them, built
/// from that model's own paths. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{Filter, Update, WriteError};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Customer {
///     pub name: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub customer: Customer,
/// }
///
/// fn main() -> Result<(), WriteError> {
///     let (invoice, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
///     let named: Filter<Invoice> = invoice
///         .customer()
///         .nest(customer.name().eq("Acme".to_owned())?);
///     let renamed: Update<Invoice> = invoice
///         .customer()
///         .nest(customer.name().set("John".to_owned())?);
///     assert_eq!(
///         named.into_document().to_string(),
///         r#"{ "customer.name": { "$eq": "Acme" } }"#
///     );
///     assert_eq!(
///         renamed.into_document().to_string(),
///         r#"{ "$set": { "customer.name": "John" } }"#
///     );
///     Ok(())
/// }
/// ```
///
/// The run below hands `nest` a filter over the rows of `Order`, a type the path does not hold,
/// and nothing else changed. A `compile_fail` doctest asserts only that some error was raised,
/// so it was compiled standalone as an ordinary test file, and the text under it is the only
/// error it earned, verbatim:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::WriteError;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Customer {
///     pub name: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub customer: Customer,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Order {
///     pub placed: bool,
/// }
///
/// fn main() -> Result<(), WriteError> {
///     let _ = Invoice::MONGO_FIELDS
///         .customer()
///         .nest(Order::MONGO_FIELDS.placed().eq(true)?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Filter<Order>: Nests<Invoice, Customer, _>` is not satisfied
///   --> tests/zz_probe.rs:31:15
///    |
/// 31 |         .nest(Order::MONGO_FIELDS.placed().eq(true)?);
///    |          ---- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |          |
///    |          required by a bound introduced by this call
///    |
/// help: the trait `Nests<Invoice, Customer, _>` is not implemented for `__tixschema_mongodb::Filter<Order>`
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// note: required by a bound in `Model::<Root, M, N>::nest`
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Model::<Root, M, N>::nest`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn nests_items() -> TokenStream {
    quote! {
        /// Marks what `nest` is handed as a filter.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct AsFilter;

        /// Marks what `nest` is handed as an update.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct AsUpdate;

        /// What `nest` takes on a path of a row of `Root` that holds an `N`: a filter over the
        /// rows of `N`, or an update of them, from any module. `Kind` is `AsFilter` or
        /// `AsUpdate`, and is read off what is handed over.
        pub trait Nests<Root, N: ?::core::marker::Sized, Kind> {
            /// The same filter or update, over the rows of `Root`.
            type Nested;

            /// This filter or update with each of its keys under `key`.
            fn nested_under(self, key: &str) -> Self::Nested;
        }

        impl<Root, N, E> Nests<Root, N, AsFilter> for E
        where
            N: ?::core::marker::Sized,
            E: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<N>>,
        {
            type Nested = Filter<Root>;

            fn nested_under(self, key: &str) -> Filter<Root> {
                Filter::raw(nested_filter(key, self.into()))
            }
        }

        impl<Root, N, E> Nests<Root, N, AsUpdate> for E
        where
            E: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<fn(N) -> N>>,
        {
            type Nested = Update<Root>;

            fn nested_under(self, key: &str) -> Update<Root> {
                Update::raw(nested_update(key, self.into()))
            }
        }

        /// `key` under `holder`, joined by a dot: an empty side is left out with its dot.
        fn nested_key(holder: &str, key: &str) -> ::std::string::String {
            if holder.is_empty() {
                key.to_owned()
            } else if key.is_empty() {
                holder.to_owned()
            } else {
                ::std::format!("{holder}.{key}")
            }
        }

        /// The filter `document` with each key under `holder`: those inside `$and`, `$or` and
        /// `$nor` too, and any other operator kept as it is written.
        fn nested_filter(holder: &str, document: bson::Document) -> bson::Document {
            let mut nested = bson::Document::new();
            for (key, value) in document {
                let joins = ::core::matches!(key.as_str(), "$and" | "$or" | "$nor");
                match value {
                    held if !key.starts_with('$') => {
                        nested.insert(nested_key(holder, &key), held);
                    }
                    bson::Bson::Array(members) if joins => {
                        let under = members
                            .into_iter()
                            .map(|member| match member {
                                bson::Bson::Document(inner) => bson::Bson::Document(nested_filter(holder, inner)),
                                other => other,
                            })
                            .collect::<::std::vec::Vec<bson::Bson>>();
                        nested.insert(key, bson::Bson::Array(under));
                    }
                    kept => {
                        nested.insert(key, kept);
                    }
                }
            }
            nested
        }

        /// The update `document` with each key of each operator under `holder`, its value kept.
        fn nested_update(holder: &str, document: bson::Document) -> bson::Document {
            let mut nested = bson::Document::new();
            for (operator, changed) in document {
                let under = match changed {
                    bson::Bson::Document(keys) => {
                        let mut joined = bson::Document::new();
                        for (key, value) in keys {
                            joined.insert(nested_key(holder, &key), value);
                        }
                        bson::Bson::Document(joined)
                    }
                    other => other,
                };
                nested.insert(operator, under);
            }
            nested
        }
    }
}

/// `OptionalField`: a `Field` a row may leave out, which adds `$exists` and `$unset` to it.
fn optional_field_items() -> TokenStream {
    quote! {
        /// The path of a value of type `V` a row of `Root` may leave out. It dereferences to the
        /// `Field` of the value where it is there.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct OptionalField<Root, V, N: ?::core::marker::Sized = V> {
            present: Field<Root, V, N>,
        }

        impl<Root, V, N> OptionalField<Root, V, N>
        where
            N: ?::core::marker::Sized,
        {
            /// A path whose values the type's own `Serialize` writes.
            #[must_use]
            pub const fn plain(path: MongoPath) -> Self
            where
                V: ::serde::Serialize,
            {
                Self { present: Field::plain(path) }
            }

            /// A path whose values `write` writes: the field's own serde hook, handed `Some`.
            #[must_use]
            pub const fn hooked(path: MongoPath, write: fn(V) -> ::core::result::Result<bson::Bson, WriteError>) -> Self {
                Self { present: Field::hooked(path, write) }
            }

            /// `$exists`: rows that hold this path, or with `false` rows that leave it out.
            #[must_use]
            pub fn exists(&self, present: bool) -> Filter<Root> {
                Filter::held(self.present.path.key(), "$exists", bson::Bson::Boolean(present))
            }

            /// `$unset`: takes this path out of the row.
            #[must_use]
            pub fn unset(&self) -> Update<Root> {
                Update::held("$unset", self.present.path.key(), bson::Bson::String(::std::string::String::new()))
            }
        }

        impl<Root, V, N> ::core::ops::Deref for OptionalField<Root, V, N>
        where
            N: ?::core::marker::Sized,
        {
            type Target = Field<Root, V, N>;

            fn deref(&self) -> &Self::Target {
                &self.present
            }
        }
    }
}

/// `OptionalModel`: a `Model` a row may leave out, with `$exists`, `$unset` and `$set` over the
/// whole value.
fn optional_model_items() -> TokenStream {
    quote! {
        /// The path of a nested model `M` a row of `Root` may leave out. It dereferences to the
        /// `Model` of the value where it is there.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct OptionalModel<Root, M, N: ?::core::marker::Sized = M> {
            present: Model<Root, M, N>,
        }

        impl<Root, M, N> OptionalModel<Root, M, N>
        where
            N: ?::core::marker::Sized,
        {
            /// A nested model whose own `Serialize` writes it.
            #[must_use]
            pub const fn plain(path: MongoPath) -> Self
            where
                M: ::serde::Serialize,
            {
                Self { present: Model::plain(path) }
            }

            /// `$exists`: rows that hold this path, or with `false` rows that leave it out.
            #[must_use]
            pub fn exists(&self, present: bool) -> Filter<Root> {
                Filter::held(self.present.whole.path.key(), "$exists", bson::Bson::Boolean(present))
            }

            /// `$unset`: takes this path out of the row.
            #[must_use]
            pub fn unset(&self) -> Update<Root> {
                Update::held("$unset", self.present.whole.path.key(), bson::Bson::String(::std::string::String::new()))
            }
        }

        impl<Root, M, N> ::core::ops::Deref for OptionalModel<Root, M, N>
        where
            N: ?::core::marker::Sized,
        {
            type Target = Model<Root, M, N>;

            fn deref(&self) -> &Self::Target {
                &self.present
            }
        }
    }
}

/// `MongoPath`, `WriteError`, and the function a path with no hook writes its values through.
///
/// A path is one key, as the type that declares it writes it. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::MongoPath;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// const TOTAL: MongoPath = MongoPath::at("total");
///
/// fn main() {
///     assert_eq!(TOTAL.key(), "total");
///     assert_eq!(MongoPath::ROOT.key(), "");
/// }
/// ```
fn path_items() -> TokenStream {
    quote! {
        /// What a value that cannot be written as BSON fails with: `bson::ser::Error` in version
        /// 2 of the library, `bson::error::Error` in version 3.
        pub type WriteError = <bson::Serializer as ::serde::Serializer>::Error;

        /// The key of a value a filter or an update names, as the type that declares the path
        /// writes it: under no key of whatever holds that type, and empty for the value itself.
        #[derive(Clone, Copy, Debug)]
        #[non_exhaustive]
        pub struct MongoPath {
            key: &'static str,
        }

        impl MongoPath {
            /// The value itself: the empty key.
            pub const ROOT: Self = Self { key: "" };

            /// The path of `key`.
            #[must_use]
            pub const fn at(key: &'static str) -> Self {
                Self { key }
            }

            /// The dotted key MongoDB reads.
            #[must_use]
            pub const fn key(&self) -> &'static str {
                self.key
            }
        }

        /// Writes a value the way its type's own `Serialize` writes it.
        fn write_plain<V>(value: V) -> ::core::result::Result<bson::Bson, WriteError>
        where
            V: ::serde::Serialize,
        {
            ::serde::Serialize::serialize(&value, bson::Serializer::new())
        }
    }
}

/// `Update`: the document a write takes, over the rows of one type.
///
/// An update merges an update of the same rows, whichever module's `Update` it is. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{Field, MongoPath, WriteError};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// const TOTAL: Field<Invoice, f64> = Field::plain(MongoPath::at("total"));
///
/// fn main() -> Result<(), WriteError> {
///     let _ = TOTAL.set(0.0)?.and(TOTAL.set_on_insert(1000.0)?);
///     Ok(())
/// }
/// ```
///
/// The run below is that one with `set_on_insert` written `gt`, a filter over the same rows where
/// an update is asked, and nothing else changed. A `compile_fail` doctest asserts only that some
/// error was raised, so it was compiled standalone as an ordinary test file, and the text under
/// it is the only error it earned, verbatim:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::{Field, MongoPath, WriteError};
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// const TOTAL: Field<Invoice, f64> = Field::plain(MongoPath::at("total"));
///
/// fn main() -> Result<(), WriteError> {
///     let _ = TOTAL.set(0.0)?.and(TOTAL.gt(1000.0)?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Filter<Invoice>: AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not satisfied
///   --> tests/zz_probe.rs:19:33
///    |
/// 19 |     let _ = TOTAL.set(0.0)?.and(TOTAL.gt(1000.0)?);
///    |                             --- ^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                             |
///    |                             required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not implemented for `__tixschema_mongodb::Filter<Invoice>`
///       but trait `AsRef<PhantomData<Invoice>>` is implemented for it
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `Invoice`, found `fn(Invoice) -> Invoice`
/// note: required by a bound in `__tixschema_mongodb::Update::<Root>::and`
///   --> tests/zz_probe.rs:8:1
///    |
///  8 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Update::<Root>::and`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
fn update_items() -> TokenStream {
    quote! {
        /// An update of the rows of `Root`.
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct Update<Root> {
            document: bson::Document,
            root: ::core::marker::PhantomData<fn() -> Root>,
        }

        impl<Root> Update<Root> {
            /// An update written as a document, for a path that has no typed form.
            #[must_use]
            pub fn raw(document: bson::Document) -> Self {
                Self { document, root: ::core::marker::PhantomData }
            }

            fn held(operator: &str, key: &str, value: bson::Bson) -> Self {
                let mut changed = bson::Document::new();
                changed.insert(key, value);
                let mut document = bson::Document::new();
                document.insert(operator, bson::Bson::Document(changed));
                Self::raw(document)
            }

            /// Both updates as one document: the keys of each operator of `added` join that
            /// operator's own in `document`.
            fn merged_update(mut document: bson::Document, added: bson::Document) -> bson::Document {
                for (operator, changed) in added {
                    match (document.get_mut(&operator), changed) {
                        (::core::option::Option::Some(bson::Bson::Document(existing)), bson::Bson::Document(keys)) => {
                            for (key, value) in keys {
                                existing.insert(key, value);
                            }
                        }
                        (_, whole) => {
                            document.insert(operator, whole);
                        }
                    }
                }
                document
            }

            /// Both updates as one: the keys of each operator are merged. `other` is an update of
            /// the same rows, from any module.
            #[must_use]
            pub fn and<U>(self, other: U) -> Self
            where
                U: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<fn(Root) -> Root>>,
            {
                Self::raw(Self::merged_update(self.document, other.into()))
            }

            /// The document the driver takes.
            #[must_use]
            pub fn into_document(self) -> bson::Document {
                self.document
            }
        }

        impl<Root> ::core::convert::From<Update<Root>> for bson::Document {
            fn from(update: Update<Root>) -> Self {
                update.document
            }
        }

        // The rows it changes, where a filter carries the rows it is over: neither stands where
        // the other is asked.
        impl<Root> ::core::convert::AsRef<::core::marker::PhantomData<fn(Root) -> Root>> for Update<Root> {
            fn as_ref(&self) -> &::core::marker::PhantomData<fn(Root) -> Root> {
                &::core::marker::PhantomData
            }
        }
    }
}
