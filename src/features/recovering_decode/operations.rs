//! The MongoDB operations `#[model_schema(decode_with)]` adds under `mongodb`.
//!
//! `OperationError` goes into the flagged type's own `{type}_schema` module, so two flagged types
//! share none of it. `find_one` and `find_one_with` go on the type, beside its BSON entry points
//! and under their bounds. The collection stays one of `bson::Document`: a row is read through
//! `from_bson_piped`, never through plain serde. Of the driver the emission names
//! `mongodb::Collection` and `mongodb::error::Error`, each one path whichever major version of
//! the `bson` library the driver is built for.

use proc_macro2::{Ident, TokenStream};
use quote::quote;

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
        // the standard `From<T> for T`, and the first bound is what tells this one from it. No
        // `impl` from the driver's error can stand beside this one, so `Database` is built by name.
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

/// `find_one`, `find_one_with`, and the read of one stored row they end with. `filter` names the
/// type parameter each read takes its filter as: any filter over the rows of the type, whichever
/// module's `Filter` it is, so one that starts from a nested model's path is taken as it stands.
/// This builds:
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
pub fn read_methods(module: &Ident, filter: &Ident) -> TokenStream {
    let over_these_rows = quote! {
        #filter: ::core::convert::Into<bson::Document> + ::core::convert::AsRef<::core::marker::PhantomData<Self>>,
    };
    quote! {
        /// The row `filter` matches, read as this type, and `None` where it matches none. It is
        /// `find_one_with` with no resolvers: any issue in the row refuses it.
        pub async fn find_one<#filter>(
            collection: &mongodb::Collection<bson::Document>,
            filter: #filter,
        ) -> ::core::result::Result<::core::option::Option<Self>, #module::OperationError>
        where
            #over_these_rows
        {
            Self::find_one_with(collection, filter, &[]).await
        }

        /// The row `filter` matches, read as this type with `resolvers` run in order over every
        /// issue found in it, once, and `None` where it matches none.
        pub async fn find_one_with<#filter>(
            collection: &mongodb::Collection<bson::Document>,
            filter: #filter,
            resolvers: &[#module::Resolver<'_, bson::Document, bson::Bson>],
        ) -> ::core::result::Result<::core::option::Option<Self>, #module::OperationError>
        where
            #over_these_rows
        {
            let found = collection
                .find_one(::core::convert::Into::<bson::Document>::into(filter))
                .await
                .map_err(#module::OperationError::Database)?;
            found.map(|row| Self::mongo_read_row(row, resolvers)).transpose()
        }

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
    }
}
