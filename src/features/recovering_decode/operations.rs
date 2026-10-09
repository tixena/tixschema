//! The MongoDB operations `#[model_schema(decode_with)]` adds under `mongodb`.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

/// The `IntoFuture` of a `Read` that answers `answer`: the driver's method `asked` over the
/// filter, under each option in `sent`, and what `answered` makes of what the driver answers.
fn awaited(
    answer: &TokenStream,
    bound: &TokenStream,
    asked: &str,
    sent: &[TokenStream],
    answered: &TokenStream,
) -> TokenStream {
    let action = Ident::new(asked, Span::call_site());
    quote! {
        impl<'c, T> ::core::future::IntoFuture for Read<'c, T, #answer>
        where
            #bound
        {
            type Output = ::core::result::Result<#answer, OperationError>;
            type IntoFuture = ::core::pin::Pin<::std::boxed::Box<dyn ::core::future::Future<Output = Self::Output> + ::core::marker::Send + 'c>>;

            fn into_future(self) -> Self::IntoFuture {
                ::std::boxed::Box::pin(async move {
                    let mut asked = self.collection.#action(self.filter);
                    #(#sent)*
                    #answered
                })
            }
        }
    }
}

/// `OperationError`, with what makes it an error and what turns the error a typed filter or
/// update fails with into it.
pub fn error_items() -> TokenStream {
    quote! {
        /// Why an operation on a flagged type failed.
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum OperationError {
            /// MongoDB refused the operation or could not be reached.
            Database(mongodb::error::Error),
            /// A row did not read as the type: its `_id`, and every issue no resolver settled.
            Unreadable { row: ::std::string::String, issues: ::std::vec::Vec<Issue<bson::Bson>> },
            /// A value given to a filter, an update or an insert could not be written as BSON.
            Unwritable(WriteError),
        }

        impl ::core::fmt::Display for OperationError {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    Self::Database(refused) => write!(f, "MongoDB refused the operation or could not be reached: {refused}"),
                    Self::Unreadable { row, issues } => {
                        write!(f, "the row {row} does not read as expected: ")?;
                        ::core::fmt::Display::fmt(&Unrecovered { issues: issues.clone() }, f)
                    }
                    Self::Unwritable(refused) => write!(f, "a value could not be written as BSON: {refused}"),
                }
            }
        }

        impl ::std::error::Error for OperationError {
            fn source(&self) -> ::core::option::Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    Self::Database(refused) => ::core::option::Option::Some(refused),
                    Self::Unreadable { .. } => ::core::option::Option::None,
                    Self::Unwritable(refused) => ::core::option::Option::Some(refused),
                }
            }
        }

        // `E` is `WriteError`, named by a bound: an `impl` for the alias itself is refused beside
        // the standard `From<T> for T`, and the first bound is what tells this one from it.
        impl<E> ::core::convert::From<E> for OperationError
        where
            E: ::serde::ser::Error,
            bson::Serializer: ::serde::Serializer<Error = E>,
        {
            fn from(refused: E) -> Self {
                Self::Unwritable(refused)
            }
        }
    }
}

/// Every operation `mongodb` adds to the type: the reads, the writes, and the read and the write
/// of one stored row they share. `filter` and `update` name the type parameters an operation
/// takes its filter and its update as.
pub fn methods(module: &Ident, filter: &Ident, update: &Ident) -> TokenStream {
    let reads = read_methods(module, filter);
    let writes = write_methods(module, filter, update);
    let rows = row_methods(module);
    quote! {
        #reads
        #writes
        #rows
    }
}

/// The bound a filter is taken under: any filter over the rows of the type.
fn over_these_rows(filter: &Ident) -> TokenStream {
    quote! {
        #filter: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<Self>>,
    }
}

/// What awaits a `Read`, per answer: every row, the first row, and how many rows there are. Each
/// hands the driver the options its own method of that kind has.
fn read_futures() -> TokenStream {
    let every_row = awaited(
        &quote! { ::std::vec::Vec<T> },
        &quote! { T: ::core::marker::Send + 'c, },
        "find",
        &["sort", "limit", "skip", "hint", "collation"].map(sent),
        &quote! {
            let mut cursor = asked.await.map_err(OperationError::Database)?;
            let mut rows = ::std::vec::Vec::new();
            while cursor.advance().await.map_err(OperationError::Database)? {
                let row = cursor.deserialize_current().map_err(OperationError::Database)?;
                rows.push((self.read)(row, self.resolvers)?);
            }
            ::core::result::Result::Ok(rows)
        },
    );
    let first_row = awaited(
        &quote! { ::core::option::Option<T> },
        &quote! { T: 'c, },
        "find_one",
        &["sort", "skip", "hint", "collation"].map(sent),
        &quote! {
            let found = asked.await.map_err(OperationError::Database)?;
            found.map(|row| (self.read)(row, self.resolvers)).transpose()
        },
    );
    // A count takes its limit unsigned, and refuses the `0` a read takes for every row.
    let counted_limit = quote! {
        if let ::core::option::Option::Some(limit) = self.limit.map(::core::primitive::i64::unsigned_abs).filter(|limit| *limit != 0) {
            asked = asked.limit(limit);
        }
    };
    let how_many = awaited(
        &quote! { u64 },
        &quote! { T: 'c, },
        "count_documents",
        &[counted_limit, sent("skip"), sent("hint"), sent("collation")],
        &quote! { asked.await.map_err(OperationError::Database) },
    );
    quote! {
        #every_row
        #first_row
        #how_many
    }
}

/// `Read`: the read `find_one`, `find`, `count` and the twins that take resolvers answer, with
/// the options it is run under and what awaits it.
///
/// A read is awaited as it stands, or under options first. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use bson::{Document, doc};
/// use mongodb::Collection;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::OperationError;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub number: String,
///     pub paid: bool,
///     pub total: f64,
/// }
///
/// pub async fn largest_unpaid(
///     invoices: &Collection<Document>,
/// ) -> Result<(Vec<Invoice>, u64), OperationError> {
///     let paths = Invoice::MONGO_FIELDS;
///     let largest = Invoice::find(invoices, paths.paid.eq(false)?)
///         .sort(doc! { "total": -1 })
///         .limit(2)
///         .await?;
///     let unpaid = Invoice::count(invoices, paths.paid.eq(false)?).await?;
///     Ok((largest, unpaid))
/// }
///
/// fn main() {}
/// ```
pub fn read_items() -> TokenStream {
    let options = read_options();
    let awaited = read_futures();
    quote! {
        /// A read of the rows of `T` a filter matches, which asks nothing of MongoDB until it is
        /// awaited. `O` is what it then answers: every row, the first row or `None`, or how many
        /// rows there are.
        #[must_use = "a read asks nothing of MongoDB until it is awaited"]
        #[non_exhaustive]
        pub struct Read<'c, T, O> {
            collection: &'c mongodb::Collection<bson::Document>,
            filter: bson::Document,
            resolvers: &'c [Resolver<'c, bson::Document, bson::Bson>],
            read: fn(bson::Document, &[Resolver<'_, bson::Document, bson::Bson>]) -> ::core::result::Result<T, OperationError>,
            sort: ::core::option::Option<bson::Document>,
            limit: ::core::option::Option<i64>,
            skip: ::core::option::Option<u64>,
            hint: ::core::option::Option<mongodb::options::Hint>,
            collation: ::core::option::Option<mongodb::options::Collation>,
            answers: ::core::marker::PhantomData<fn() -> O>,
        }

        impl<'c, T, O> Read<'c, T, O> {
            /// A read of the rows `filter` matches in `collection`, under no option yet. `read`
            /// reads one stored row, with `resolvers` run over its issues.
            pub fn new(
                collection: &'c mongodb::Collection<bson::Document>,
                filter: bson::Document,
                resolvers: &'c [Resolver<'c, bson::Document, bson::Bson>],
                read: fn(bson::Document, &[Resolver<'_, bson::Document, bson::Bson>]) -> ::core::result::Result<T, OperationError>,
            ) -> Self {
                Self {
                    collection,
                    filter,
                    resolvers,
                    read,
                    sort: ::core::option::Option::None,
                    limit: ::core::option::Option::None,
                    skip: ::core::option::Option::None,
                    hint: ::core::option::Option::None,
                    collation: ::core::option::Option::None,
                    answers: ::core::marker::PhantomData,
                }
            }

            #options
        }

        impl<T, O> ::core::fmt::Debug for Read<'_, T, O> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct("Read")
                    .field("filter", &self.filter)
                    .field("sort", &self.sort)
                    .field("limit", &self.limit)
                    .field("skip", &self.skip)
                    .field("hint", &self.hint)
                    .field("collation", &self.collation)
                    .field("resolvers", &self.resolvers.len())
                    .finish_non_exhaustive()
            }
        }

        #awaited
    }
}

/// `find_one`, `find`, `count` and the twins that take resolvers, each answering a `Read`.
/// `filter` names the type parameter each takes its filter as: any filter over the rows of the
/// type, whichever module's `Filter` it is, so one that starts from a nested model's path is
/// taken as it stands. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use bson::Document;
/// use mongodb::Collection;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::OperationError;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Customer {
///     pub name: String,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub customer: Customer,
///     pub number: String,
/// }
///
/// pub async fn of_acme(
///     invoices: &Collection<Document>,
/// ) -> Result<Option<Invoice>, OperationError> {
///     let paths = Invoice::MONGO_FIELDS;
///     Invoice::find_one(invoices, paths.customer.name.eq("Acme".to_owned())?).await
/// }
///
/// fn main() {}
/// ```
fn read_methods(module: &Ident, filter: &Ident) -> TokenStream {
    let over_these_rows = over_these_rows(filter);
    let filtered = quote! { ::core::convert::Into::<bson::Document>::into(filter) };
    let resolved = quote! { &'c [#module::Resolver<'c, bson::Document, bson::Bson>] };
    quote! {
        /// The first row `filter` matches, read as this type, and `None` where it matches none.
        /// It is `find_one_with` with no resolvers: any issue in the row refuses it.
        pub fn find_one<#filter>(
            collection: &mongodb::Collection<bson::Document>,
            filter: #filter,
        ) -> #module::Read<'_, Self, ::core::option::Option<Self>>
        where
            #over_these_rows
        {
            Self::find_one_with(collection, filter, &[])
        }

        /// The first row `filter` matches, read as this type with `resolvers` run in order over
        /// every issue found in it, once, and `None` where it matches none.
        pub fn find_one_with<'c, #filter>(
            collection: &'c mongodb::Collection<bson::Document>,
            filter: #filter,
            resolvers: #resolved,
        ) -> #module::Read<'c, Self, ::core::option::Option<Self>>
        where
            #over_these_rows
        {
            #module::Read::new(collection, #filtered, resolvers, Self::mongo_read_row)
        }

        /// Every row `filter` matches, read as this type. It is `find_with` with no resolvers:
        /// any issue in any row refuses them all.
        pub fn find<#filter>(
            collection: &mongodb::Collection<bson::Document>,
            filter: #filter,
        ) -> #module::Read<'_, Self, ::std::vec::Vec<Self>>
        where
            #over_these_rows
        {
            Self::find_with(collection, filter, &[])
        }

        /// Every row `filter` matches, read as this type with `resolvers` run in order over every
        /// issue found in each, once. The first row that does not read refuses them all, by its
        /// `_id`.
        pub fn find_with<'c, #filter>(
            collection: &'c mongodb::Collection<bson::Document>,
            filter: #filter,
            resolvers: #resolved,
        ) -> #module::Read<'c, Self, ::std::vec::Vec<Self>>
        where
            #over_these_rows
        {
            #module::Read::new(collection, #filtered, resolvers, Self::mongo_read_row)
        }

        /// How many rows `filter` matches. No row is read.
        pub fn count<#filter>(
            collection: &mongodb::Collection<bson::Document>,
            filter: #filter,
        ) -> #module::Read<'_, Self, u64>
        where
            #over_these_rows
        {
            #module::Read::new(collection, #filtered, &[], Self::mongo_read_row)
        }
    }
}

/// The options a `Read` takes: one method each, taking what the driver's own option takes.
fn read_options() -> TokenStream {
    [
        (
            "sort",
            quote! { bson::Document },
            "The order the rows are read in: each key with `1` or `-1`. A count is the same in \
             any order.",
        ),
        (
            "limit",
            quote! { i64 },
            "At most this many rows, as the driver's `find` takes it: `0` is every row. A count \
             is held to the same number, and a read of one row answers the first whatever it is.",
        ),
        ("skip", quote! { u64 }, "Passes over this many rows first."),
        (
            "hint",
            quote! { mongodb::options::Hint },
            "The index MongoDB is to use.",
        ),
        (
            "collation",
            quote! { mongodb::options::Collation },
            "The collation text is compared under.",
        ),
    ]
    .into_iter()
    .map(|(option, taken, asks)| {
        let name = Ident::new(option, Span::call_site());
        quote! {
            #[doc = #asks]
            pub fn #name(mut self, #name: #taken) -> Self {
                self.#name = ::core::option::Option::Some(#name);
                self
            }
        }
    })
    .collect()
}

/// The read of one stored row every read ends with, and the write of one every insert opens with.
fn row_methods(module: &Ident) -> TokenStream {
    quote! {
        /// One stored row read as this type. A refused read answers the row's `_id` as
        /// `bson::Bson` displays it, which keeps its BSON type in the text.
        fn mongo_read_row(
            row: bson::Document,
            resolvers: &[#module::Resolver<'_, bson::Document, bson::Bson>],
        ) -> ::core::result::Result<Self, #module::OperationError> {
            let id = row.get("_id").map_or_else(|| ::std::string::String::from("without an _id"), ::std::string::ToString::to_string);
            Self::from_bson_piped(row, resolvers)
                .map_err(|refused| #module::OperationError::Unreadable { row: id, issues: refused.issues })
        }

        /// This value as the row MongoDB stores for it. A value serde writes as anything but a
        /// document is refused: a row is one.
        fn mongo_written_row(&self) -> ::core::result::Result<bson::Document, #module::OperationError> {
            match ::serde::Serialize::serialize(self, bson::Serializer::new()).map_err(#module::OperationError::Unwritable)? {
                bson::Bson::Document(row) => ::core::result::Result::Ok(row),
                _ => ::core::result::Result::Err(#module::OperationError::Unwritable(
                    <#module::WriteError as ::serde::ser::Error>::custom(
                        "a row is stored as a document, and this value is not written as one",
                    ),
                )),
            }
        }
    }
}

/// What hands the driver's action the option `option`, where the read was given one.
fn sent(option: &str) -> TokenStream {
    let name = Ident::new(option, Span::call_site());
    quote! {
        if let ::core::option::Option::Some(#name) = self.#name {
            asked = asked.#name(#name);
        }
    }
}

/// `insert_one`, and `update_one`, `update_many`, `delete_one` and `delete_many`, each answering
/// what the driver's own method of that name answers. `filter` and `update` name the type
/// parameters a filter and an update are taken as, each by the marker its kind carries. This
/// builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use bson::Document;
/// use mongodb::Collection;
/// use mongodb::results::UpdateResult;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::OperationError;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub number: String,
///     pub paid: bool,
/// }
///
/// pub async fn settle(invoices: &Collection<Document>) -> Result<UpdateResult, OperationError> {
///     let paths = Invoice::MONGO_FIELDS;
///     Invoice::update_one(
///         invoices,
///         paths.number.eq("INV-0042".to_owned())?,
///         paths.paid.set(true)?,
///     )
///     .await
/// }
///
/// fn main() {}
/// ```
///
/// The run below is that one with the filter and the update in each other's place, and nothing
/// else changed. A `compile_fail` doctest asserts only that some error was raised, so it was
/// compiled standalone as an ordinary test file. It earned six errors: the two under it, one per
/// argument, verbatim, and each of them twice more in the same words, against the call as a whole
/// and against its `.await`.
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use bson::Document;
/// use mongodb::Collection;
/// use mongodb::results::UpdateResult;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// use invoice_schema::OperationError;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub number: String,
///     pub paid: bool,
/// }
///
/// pub async fn settle(invoices: &Collection<Document>) -> Result<UpdateResult, OperationError> {
///     let paths = Invoice::MONGO_FIELDS;
///     Invoice::update_one(
///         invoices,
///         paths.paid.set(true)?,
///         paths.number.eq("INV-0042".to_owned())?,
///     )
///     .await
/// }
///
/// fn main() {}
/// ```
///
/// ```text
/// error[E0277]: the trait bound `invoice_schema::Update<Invoice>: AsRef<PhantomData<Invoice>>` is not satisfied
///   --> tests/zz_probe.rs:21:9
///    |
/// 19 |     Invoice::update_one(
///    |     ------------------- required by a bound introduced by this call
/// 20 |         invoices,
/// 21 |         paths.paid.set(true)?,
///    |         ^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |
/// help: the trait `AsRef<PhantomData<Invoice>>` is not implemented for `invoice_schema::Update<Invoice>`
///       but trait `AsRef<PhantomData<fn(Invoice) -> Invoice>>` is implemented for it
///   --> tests/zz_probe.rs:10:1
///    |
/// 10 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `fn(Invoice) -> Invoice`, found `Invoice`
/// note: required by a bound in `Invoice::update_one`
///   --> tests/zz_probe.rs:10:1
///    |
/// 10 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Invoice::update_one`
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error[E0277]: the trait bound `invoice_schema::Filter<Invoice>: AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not satisfied
///   --> tests/zz_probe.rs:22:9
///    |
/// 19 |     Invoice::update_one(
///    |     ------------------- required by a bound introduced by this call
/// ...
/// 22 |         paths.number.eq("INV-0042".to_owned())?,
///    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |
/// help: the trait `AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not implemented for `invoice_schema::Filter<Invoice>`
///       but trait `AsRef<PhantomData<Invoice>>` is implemented for it
///   --> tests/zz_probe.rs:10:1
///    |
/// 10 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `Invoice`, found `fn(Invoice) -> Invoice`
/// note: required by a bound in `Invoice::update_one`
///   --> tests/zz_probe.rs:10:1
///    |
/// 10 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Invoice::update_one`
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 6 previous errors
/// ```
fn write_methods(module: &Ident, filter: &Ident, update: &Ident) -> TokenStream {
    let over_these_rows = over_these_rows(filter);
    let refused = quote! { map_err(#module::OperationError::Database) };
    let updates = [
        ("update_one", "the first row `filter` matches"),
        ("update_many", "every row `filter` matches"),
    ]
    .map(|(method, rows)| {
        let name = Ident::new(method, Span::call_site());
        let told = format!("Applies `update` to {rows}.");
        quote! {
            #[doc = #told]
            pub async fn #name<#filter, #update>(
                collection: &mongodb::Collection<bson::Document>,
                filter: #filter,
                update: #update,
            ) -> ::core::result::Result<mongodb::results::UpdateResult, #module::OperationError>
            where
                #over_these_rows
                #update: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<fn(Self) -> Self>>,
            {
                collection
                    .#name(::core::convert::Into::<bson::Document>::into(filter), ::core::convert::Into::<bson::Document>::into(update))
                    .await
                    .#refused
            }
        }
    });
    let deletes = [
        ("delete_one", "the first row `filter` matches"),
        ("delete_many", "every row `filter` matches"),
    ]
    .map(|(method, rows)| {
        let name = Ident::new(method, Span::call_site());
        let told = format!("Deletes {rows}.");
        quote! {
            #[doc = #told]
            pub async fn #name<#filter>(
                collection: &mongodb::Collection<bson::Document>,
                filter: #filter,
            ) -> ::core::result::Result<mongodb::results::DeleteResult, #module::OperationError>
            where
                #over_these_rows
            {
                collection
                    .#name(::core::convert::Into::<bson::Document>::into(filter))
                    .await
                    .#refused
            }
        }
    });
    quote! {
        /// Stores this value as one row. It is written as a BSON document before this answers,
        /// so what is awaited holds no borrow of it, and a value that cannot be written is
        /// refused without MongoDB being asked anything.
        pub fn insert_one<'c>(
            &self,
            collection: &'c mongodb::Collection<bson::Document>,
        ) -> ::core::pin::Pin<::std::boxed::Box<dyn ::core::future::Future<Output = ::core::result::Result<mongodb::results::InsertOneResult, #module::OperationError>> + ::core::marker::Send + 'c>> {
            let written = self.mongo_written_row();
            ::std::boxed::Box::pin(async move { collection.insert_one(written?).await.#refused })
        }

        #(#updates)*
        #(#deletes)*
    }
}
