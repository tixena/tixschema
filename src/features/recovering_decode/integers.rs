//! `BsonIntegers`, the serializer a BSON value is written through wherever the support libraries
//! write one: what the BSON serializer it wraps writes, with each `u64` and `i64` the macro writes
//! as the string of its decimal digits written as the integer, as a stored row holds it. The
//! macro writes the string inside a newtype of its own name where the serializer is
//! human-readable, which the BSON serializer of both major versions of the `bson` library reports
//! itself to be.

use proc_macro2::TokenStream;
use quote::quote;

use crate::model_schema::digits::DIGITS_NEWTYPE;

/// The items of `BsonIntegers`, in the `decode_with` library.
pub fn bson_integer_items() -> TokenStream {
    let serializer = serializer_impl();
    let compounds = compound_impls();
    quote! {
        /// A serializer that writes what `S` writes, with each `u64` and `i64` the macro writes as
        /// the string of its decimal digits written as the integer, and a value written through it.
        #[non_exhaustive]
        pub struct BsonIntegers<S>(pub S);

        impl<T> ::serde::Serialize for BsonIntegers<&T>
        where
            T: ?::core::marker::Sized + ::serde::Serialize,
        {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                self.0.serialize(BsonIntegers(serializer))
            }
        }

        #serializer
        #compounds
    }
}

/// `impl Serializer for BsonIntegers<S>`: every method forwarded to `S`, each value it holds
/// written through `BsonIntegers`, and a digit string's newtype written as its integer.
fn serializer_impl() -> TokenStream {
    let capture = capture_items();
    let compounds = compound_methods();
    let scalars = forwarded_scalars();
    quote! {
        impl<S> ::serde::Serializer for BsonIntegers<S>
        where
            S: ::serde::Serializer,
        {
            type Ok = S::Ok;
            type Error = S::Error;
            type SerializeSeq = BsonIntegers<S::SerializeSeq>;
            type SerializeTuple = BsonIntegers<S::SerializeTuple>;
            type SerializeTupleStruct = BsonIntegers<S::SerializeTupleStruct>;
            type SerializeTupleVariant = BsonIntegers<S::SerializeTupleVariant>;
            type SerializeMap = BsonIntegers<S::SerializeMap>;
            type SerializeStruct = BsonIntegers<S::SerializeStruct>;
            type SerializeStructVariant = BsonIntegers<S::SerializeStructVariant>;

            fn is_human_readable(&self) -> bool {
                self.0.is_human_readable()
            }

            #(#scalars)*

            fn serialize_none(self) -> ::core::result::Result<S::Ok, S::Error> {
                self.0.serialize_none()
            }

            fn serialize_some<T>(self, value: &T) -> ::core::result::Result<S::Ok, S::Error>
            where
                T: ?::core::marker::Sized + ::serde::Serialize,
            {
                self.0.serialize_some(&BsonIntegers(value))
            }

            fn serialize_unit(self) -> ::core::result::Result<S::Ok, S::Error> {
                self.0.serialize_unit()
            }

            fn serialize_unit_struct(self, name: &'static str) -> ::core::result::Result<S::Ok, S::Error> {
                self.0.serialize_unit_struct(name)
            }

            fn serialize_unit_variant(
                self,
                name: &'static str,
                variant_index: u32,
                variant: &'static str,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                self.0.serialize_unit_variant(name, variant_index, variant)
            }

            fn serialize_newtype_struct<T>(
                self,
                name: &'static str,
                value: &T,
            ) -> ::core::result::Result<S::Ok, S::Error>
            where
                T: ?::core::marker::Sized + ::serde::Serialize,
            {
                if name == #DIGITS_NEWTYPE {
                    #capture
                    if let ::core::result::Result::Ok(digits) = value.serialize(Captured) {
                        if let ::core::result::Result::Ok(number) = <u64 as ::core::str::FromStr>::from_str(&digits) {
                            return self.0.serialize_u64(number);
                        }
                        if let ::core::result::Result::Ok(number) = <i64 as ::core::str::FromStr>::from_str(&digits) {
                            return self.0.serialize_i64(number);
                        }
                    }
                }
                self.0.serialize_newtype_struct(name, &BsonIntegers(value))
            }

            fn serialize_newtype_variant<T>(
                self,
                name: &'static str,
                variant_index: u32,
                variant: &'static str,
                value: &T,
            ) -> ::core::result::Result<S::Ok, S::Error>
            where
                T: ?::core::marker::Sized + ::serde::Serialize,
            {
                self.0.serialize_newtype_variant(name, variant_index, variant, &BsonIntegers(value))
            }

            #compounds
        }
    }
}

/// The methods of `impl Serializer for BsonIntegers<S>` that write a scalar, each forwarded to `S`.
fn forwarded_scalars() -> impl Iterator<Item = TokenStream> {
    [
        (quote! { serialize_bool }, quote! { bool }),
        (quote! { serialize_i8 }, quote! { i8 }),
        (quote! { serialize_i16 }, quote! { i16 }),
        (quote! { serialize_i32 }, quote! { i32 }),
        (quote! { serialize_i64 }, quote! { i64 }),
        (quote! { serialize_u8 }, quote! { u8 }),
        (quote! { serialize_u16 }, quote! { u16 }),
        (quote! { serialize_u32 }, quote! { u32 }),
        (quote! { serialize_u64 }, quote! { u64 }),
        (quote! { serialize_f32 }, quote! { f32 }),
        (quote! { serialize_f64 }, quote! { f64 }),
        (quote! { serialize_char }, quote! { char }),
        (quote! { serialize_str }, quote! { &str }),
        (quote! { serialize_bytes }, quote! { &[u8] }),
    ]
    .into_iter()
    .map(|(method, held)| {
        quote! {
            fn #method(self, value: #held) -> ::core::result::Result<S::Ok, S::Error> {
                self.0.#method(value)
            }
        }
    })
}

/// The methods of `impl Serializer for BsonIntegers<S>` that start a compound value, each one
/// `S` answers wrapped in `BsonIntegers`.
fn compound_methods() -> TokenStream {
    quote! {
        fn serialize_seq(
            self,
            len: ::core::option::Option<usize>,
        ) -> ::core::result::Result<Self::SerializeSeq, S::Error> {
            self.0.serialize_seq(len).map(BsonIntegers)
        }

        fn serialize_tuple(self, len: usize) -> ::core::result::Result<Self::SerializeTuple, S::Error> {
            self.0.serialize_tuple(len).map(BsonIntegers)
        }

        fn serialize_tuple_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeTupleStruct, S::Error> {
            self.0.serialize_tuple_struct(name, len).map(BsonIntegers)
        }

        fn serialize_tuple_variant(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeTupleVariant, S::Error> {
            self.0
                .serialize_tuple_variant(name, variant_index, variant, len)
                .map(BsonIntegers)
        }

        fn serialize_map(
            self,
            len: ::core::option::Option<usize>,
        ) -> ::core::result::Result<Self::SerializeMap, S::Error> {
            self.0.serialize_map(len).map(BsonIntegers)
        }

        fn serialize_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeStruct, S::Error> {
            self.0.serialize_struct(name, len).map(BsonIntegers)
        }

        fn serialize_struct_variant(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeStructVariant, S::Error> {
            self.0
                .serialize_struct_variant(name, variant_index, variant, len)
                .map(BsonIntegers)
        }
    }
}

/// `Captured`, declared where it is used: a serializer that answers the text a digit string's
/// newtype holds, and refuses every other value.
fn capture_items() -> TokenStream {
    let compounds = uncaptured_compounds();
    let refused = uncaptured_scalars();
    quote! {
        #[derive(Debug)]
        struct Uncaptured;
        impl ::core::fmt::Display for Uncaptured {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str("no digit string")
            }
        }
        impl ::std::error::Error for Uncaptured {}
        impl ::serde::ser::Error for Uncaptured {
            fn custom<M>(_: M) -> Self
            where
                M: ::core::fmt::Display,
            {
                Self
            }
        }
        struct Captured;
        impl ::serde::Serializer for Captured {
            type Ok = ::std::string::String;
            type Error = Uncaptured;
            type SerializeSeq = ::serde::ser::Impossible<::std::string::String, Uncaptured>;
            type SerializeTuple = ::serde::ser::Impossible<::std::string::String, Uncaptured>;
            type SerializeTupleStruct = ::serde::ser::Impossible<::std::string::String, Uncaptured>;
            type SerializeTupleVariant = ::serde::ser::Impossible<::std::string::String, Uncaptured>;
            type SerializeMap = ::serde::ser::Impossible<::std::string::String, Uncaptured>;
            type SerializeStruct = ::serde::ser::Impossible<::std::string::String, Uncaptured>;
            type SerializeStructVariant = ::serde::ser::Impossible<::std::string::String, Uncaptured>;

            #(#refused)*

            fn serialize_str(self, value: &str) -> ::core::result::Result<::std::string::String, Uncaptured> {
                ::core::result::Result::Ok(::std::borrow::ToOwned::to_owned(value))
            }

            fn serialize_none(self) -> ::core::result::Result<::std::string::String, Uncaptured> {
                ::core::result::Result::Err(Uncaptured)
            }

            fn serialize_some<T>(self, _: &T) -> ::core::result::Result<::std::string::String, Uncaptured>
            where
                T: ?::core::marker::Sized + ::serde::Serialize,
            {
                ::core::result::Result::Err(Uncaptured)
            }

            fn serialize_unit(self) -> ::core::result::Result<::std::string::String, Uncaptured> {
                ::core::result::Result::Err(Uncaptured)
            }

            fn serialize_unit_struct(self, _: &'static str) -> ::core::result::Result<::std::string::String, Uncaptured> {
                ::core::result::Result::Err(Uncaptured)
            }

            fn serialize_unit_variant(
                self,
                _: &'static str,
                _: u32,
                _: &'static str,
            ) -> ::core::result::Result<::std::string::String, Uncaptured> {
                ::core::result::Result::Err(Uncaptured)
            }

            fn serialize_newtype_struct<T>(
                self,
                _: &'static str,
                value: &T,
            ) -> ::core::result::Result<::std::string::String, Uncaptured>
            where
                T: ?::core::marker::Sized + ::serde::Serialize,
            {
                value.serialize(self)
            }

            fn serialize_newtype_variant<T>(
                self,
                _: &'static str,
                _: u32,
                _: &'static str,
                _: &T,
            ) -> ::core::result::Result<::std::string::String, Uncaptured>
            where
                T: ?::core::marker::Sized + ::serde::Serialize,
            {
                ::core::result::Result::Err(Uncaptured)
            }

            #compounds
        }
    }
}

/// The methods of `impl Serializer for Captured` that write a scalar but a text, each refused.
fn uncaptured_scalars() -> impl Iterator<Item = TokenStream> {
    [
        (quote! { serialize_bool }, quote! { bool }),
        (quote! { serialize_i8 }, quote! { i8 }),
        (quote! { serialize_i16 }, quote! { i16 }),
        (quote! { serialize_i32 }, quote! { i32 }),
        (quote! { serialize_i64 }, quote! { i64 }),
        (quote! { serialize_u8 }, quote! { u8 }),
        (quote! { serialize_u16 }, quote! { u16 }),
        (quote! { serialize_u32 }, quote! { u32 }),
        (quote! { serialize_u64 }, quote! { u64 }),
        (quote! { serialize_f32 }, quote! { f32 }),
        (quote! { serialize_f64 }, quote! { f64 }),
        (quote! { serialize_char }, quote! { char }),
        (quote! { serialize_bytes }, quote! { &[u8] }),
    ]
    .into_iter()
    .map(|(method, held)| {
        quote! {
            fn #method(self, _: #held) -> ::core::result::Result<::std::string::String, Uncaptured> {
                ::core::result::Result::Err(Uncaptured)
            }
        }
    })
}

/// The methods of `impl Serializer for Captured` that start a compound value, each refused.
fn uncaptured_compounds() -> TokenStream {
    quote! {
        fn serialize_seq(
            self,
            _: ::core::option::Option<usize>,
        ) -> ::core::result::Result<Self::SerializeSeq, Uncaptured> {
            ::core::result::Result::Err(Uncaptured)
        }

        fn serialize_tuple(self, _: usize) -> ::core::result::Result<Self::SerializeTuple, Uncaptured> {
            ::core::result::Result::Err(Uncaptured)
        }

        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> ::core::result::Result<Self::SerializeTupleStruct, Uncaptured> {
            ::core::result::Result::Err(Uncaptured)
        }

        fn serialize_tuple_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> ::core::result::Result<Self::SerializeTupleVariant, Uncaptured> {
            ::core::result::Result::Err(Uncaptured)
        }

        fn serialize_map(
            self,
            _: ::core::option::Option<usize>,
        ) -> ::core::result::Result<Self::SerializeMap, Uncaptured> {
            ::core::result::Result::Err(Uncaptured)
        }

        fn serialize_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> ::core::result::Result<Self::SerializeStruct, Uncaptured> {
            ::core::result::Result::Err(Uncaptured)
        }

        fn serialize_struct_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> ::core::result::Result<Self::SerializeStructVariant, Uncaptured> {
            ::core::result::Result::Err(Uncaptured)
        }
    }
}

/// The compound serializers of `BsonIntegers`, each forwarded to the one `S` answers, every value
/// written through `BsonIntegers`.
fn compound_impls() -> TokenStream {
    let element = |trait_name: TokenStream, method: TokenStream| {
        quote! {
            impl<C> ::serde::ser::#trait_name for BsonIntegers<C>
            where
                C: ::serde::ser::#trait_name,
            {
                type Ok = C::Ok;
                type Error = C::Error;

                fn #method<T>(&mut self, value: &T) -> ::core::result::Result<(), C::Error>
                where
                    T: ?::core::marker::Sized + ::serde::Serialize,
                {
                    self.0.#method(&BsonIntegers(value))
                }

                fn end(self) -> ::core::result::Result<C::Ok, C::Error> {
                    self.0.end()
                }
            }
        }
    };
    let field = |trait_name: TokenStream| {
        quote! {
            impl<C> ::serde::ser::#trait_name for BsonIntegers<C>
            where
                C: ::serde::ser::#trait_name,
            {
                type Ok = C::Ok;
                type Error = C::Error;

                fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> ::core::result::Result<(), C::Error>
                where
                    T: ?::core::marker::Sized + ::serde::Serialize,
                {
                    self.0.serialize_field(key, &BsonIntegers(value))
                }

                fn skip_field(&mut self, key: &'static str) -> ::core::result::Result<(), C::Error> {
                    self.0.skip_field(key)
                }

                fn end(self) -> ::core::result::Result<C::Ok, C::Error> {
                    self.0.end()
                }
            }
        }
    };
    let seq = element(quote! { SerializeSeq }, quote! { serialize_element });
    let tuple = element(quote! { SerializeTuple }, quote! { serialize_element });
    let tuple_struct = element(quote! { SerializeTupleStruct }, quote! { serialize_field });
    let tuple_variant = element(quote! { SerializeTupleVariant }, quote! { serialize_field });
    let struct_fields = field(quote! { SerializeStruct });
    let struct_variant = field(quote! { SerializeStructVariant });
    quote! {
        #seq
        #tuple
        #tuple_struct
        #tuple_variant
        #struct_fields
        #struct_variant

        impl<C> ::serde::ser::SerializeMap for BsonIntegers<C>
        where
            C: ::serde::ser::SerializeMap,
        {
            type Ok = C::Ok;
            type Error = C::Error;

            fn serialize_key<T>(&mut self, key: &T) -> ::core::result::Result<(), C::Error>
            where
                T: ?::core::marker::Sized + ::serde::Serialize,
            {
                self.0.serialize_key(&BsonIntegers(key))
            }

            fn serialize_value<T>(&mut self, value: &T) -> ::core::result::Result<(), C::Error>
            where
                T: ?::core::marker::Sized + ::serde::Serialize,
            {
                self.0.serialize_value(&BsonIntegers(value))
            }

            fn end(self) -> ::core::result::Result<C::Ok, C::Error> {
                self.0.end()
            }
        }
    }
}
