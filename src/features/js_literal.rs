//! The writer a Zod source takes the text of an example or a default from. A `u64` or `i64` is a
//! bigint literal there, as the bounds of its schema are, because a number literal above 2^53
//! loses its value. The writer is generated into the consumer's crate, beside the code that calls
//! it: tixschema has no runtime library.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

/// The name of the module the writer is declared in, inside the function that uses it.
const MODULE: &str = "tixschema_bigint_writer";

/// The statements that declare the writer, to put at the head of a function body.
pub fn writer() -> TokenStream {
    let module = module();
    let header = header();
    let compounds = compounds();
    let serializer = serializer();
    quote! {
        mod #module {
            #header
            #compounds
            #serializer
        }
    }
}

fn header() -> TokenStream {
    quote! {
        use ::serde::ser::{Error as _, Serialize, Serializer};
        use ::serde_json::Value;

        type Failure = ::serde_json::Error;
        type Written = ::core::result::Result<Value, Failure>;

        const MARK: char = '\u{1}';
        const OPENING: &str = "\"\\u0001bigint:";
        const CLOSING: &str = "\\u0001\"";
        const POINTERS: [&str; 3] = ["alloc::boxed::Box<", "alloc::rc::Rc<", "alloc::sync::Arc<"];
        const SAFE: u64 = 9_007_199_254_740_991;

        pub(super) fn top() -> Writer {
            Writer { wide: false }
        }

        pub(super) fn text<T: ?::core::marker::Sized + Serialize>(
            value: &T,
        ) -> ::std::string::String {
            rendered(&value.serialize(Writer::declared::<T>()).unwrap())
        }

        pub(super) fn rendered(value: &Value) -> ::std::string::String {
            let json = ::serde_json::to_string(value).unwrap();
            let mut replaced = ::std::string::String::new();
            let mut rest = json.as_str();
            while let ::core::option::Option::Some((before, after)) = rest.split_once(OPENING) {
                let ::core::option::Option::Some((digits, tail)) = after.split_once(CLOSING)
                else {
                    break;
                };
                replaced.push_str(before);
                replaced.push_str(digits);
                replaced.push('n');
                rest = tail;
            }
            replaced.push_str(rest);
            replaced
        }

        fn is_wide(mut name: &str) -> bool {
            loop {
                if let ::core::option::Option::Some(inner) = name
                    .strip_prefix("&mut ")
                    .or_else(|| name.strip_prefix('&'))
                    .or_else(|| {
                        POINTERS
                            .iter()
                            .find_map(|pointer| name.strip_prefix(pointer)?.strip_suffix('>'))
                    })
                {
                    name = inner;
                } else {
                    return name == "i64" || name == "u64";
                }
            }
        }

        fn marked(digits: ::std::string::String) -> Written {
            ::core::result::Result::Ok(Value::String(::std::format!("{MARK}bigint:{digits}{MARK}")))
        }

        #[derive(Clone, Copy)]
        pub(super) struct Writer {
            wide: bool,
        }

        impl Writer {
            fn declared<T: ?::core::marker::Sized>() -> Self {
                Self {
                    wide: is_wide(::core::any::type_name::<T>()),
                }
            }
        }

        struct Child<'a, T: ?::core::marker::Sized>(&'a T);

        impl<T: ?::core::marker::Sized + Serialize> Serialize for Child<'_, T> {
            fn serialize<S: Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                let written = self
                    .0
                    .serialize(Writer::declared::<T>())
                    .map_err(<S::Error as ::serde::ser::Error>::custom)?;
                written.serialize(serializer)
            }
        }
    }
}

fn compounds() -> TokenStream {
    let elements = [
        ("SerializeSeq", "serialize_element"),
        ("SerializeTuple", "serialize_element"),
        ("SerializeTupleStruct", "serialize_field"),
        ("SerializeTupleVariant", "serialize_field"),
    ]
    .map(|(held, method)| compound(&format_ident!("{held}"), &format_ident!("{method}")));
    let members = ["SerializeStruct", "SerializeStructVariant"].map(|named| {
        let held = format_ident!("{named}");
        quote! {
            impl<I: ::serde::ser::#held> ::serde::ser::#held for Compound<I> {
                type Ok = I::Ok;
                type Error = I::Error;

                fn serialize_field<T: ?::core::marker::Sized + Serialize>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> ::core::result::Result<(), I::Error> {
                    self.inner.serialize_field(key, &Child(value))
                }

                fn end(self) -> ::core::result::Result<I::Ok, I::Error> {
                    self.inner.end()
                }
            }
        }
    });
    quote! {
        pub(super) struct Compound<I> {
            inner: I,
        }

        #(#elements)*
        #(#members)*

        impl<I: ::serde::ser::SerializeMap> ::serde::ser::SerializeMap
            for Compound<I>
        {
            type Ok = I::Ok;
            type Error = I::Error;

            fn serialize_key<T: ?::core::marker::Sized + Serialize>(
                &mut self,
                key: &T,
            ) -> ::core::result::Result<(), I::Error> {
                self.inner.serialize_key(key)
            }

            fn serialize_value<T: ?::core::marker::Sized + Serialize>(
                &mut self,
                value: &T,
            ) -> ::core::result::Result<(), I::Error> {
                self.inner.serialize_value(&Child(value))
            }

            fn end(self) -> ::core::result::Result<I::Ok, I::Error> {
                self.inner.end()
            }
        }
    }
}

fn primitives() -> Vec<TokenStream> {
    [
        ("serialize_bool", quote! { bool }),
        ("serialize_i8", quote! { i8 }),
        ("serialize_i16", quote! { i16 }),
        ("serialize_i32", quote! { i32 }),
        ("serialize_i128", quote! { i128 }),
        ("serialize_u8", quote! { u8 }),
        ("serialize_u16", quote! { u16 }),
        ("serialize_u32", quote! { u32 }),
        ("serialize_u128", quote! { u128 }),
        ("serialize_f32", quote! { f32 }),
        ("serialize_f64", quote! { f64 }),
        ("serialize_char", quote! { char }),
        ("serialize_str", quote! { &str }),
        ("serialize_bytes", quote! { &[u8] }),
    ]
    .into_iter()
    .map(|(named, held)| {
        let method = format_ident!("{named}");
        quote! {
            fn #method(self, value: #held) -> Written {
                ::serde_json::value::Serializer.#method(value)
            }
        }
    })
    .collect()
}

fn serializer() -> TokenStream {
    let primitives = primitives();
    let leaves = leaves();
    let constructors = constructors();
    quote! {
        impl Serializer for Writer {
            type Ok = Value;
            type Error = Failure;
            type SerializeSeq = Compound<<::serde_json::value::Serializer as Serializer>::SerializeSeq>;
            type SerializeTuple = Compound<<::serde_json::value::Serializer as Serializer>::SerializeTuple>;
            type SerializeTupleStruct = Compound<<::serde_json::value::Serializer as Serializer>::SerializeTupleStruct>;
            type SerializeTupleVariant = Compound<<::serde_json::value::Serializer as Serializer>::SerializeTupleVariant>;
            type SerializeMap = Compound<<::serde_json::value::Serializer as Serializer>::SerializeMap>;
            type SerializeStruct = Compound<<::serde_json::value::Serializer as Serializer>::SerializeStruct>;
            type SerializeStructVariant = Compound<<::serde_json::value::Serializer as Serializer>::SerializeStructVariant>;

            #(#primitives)*
            #leaves
            #constructors
        }
    }
}

fn leaves() -> TokenStream {
    quote! {
        fn serialize_i64(self, value: i64) -> Written {
            if self.wide || value.unsigned_abs() > SAFE {
                marked(value.to_string())
            } else {
                ::serde_json::value::Serializer.serialize_i64(value)
            }
        }

        fn serialize_u64(self, value: u64) -> Written {
            if self.wide || value > SAFE {
                marked(value.to_string())
            } else {
                ::serde_json::value::Serializer.serialize_u64(value)
            }
        }

        fn serialize_none(self) -> Written {
            ::serde_json::value::Serializer.serialize_none()
        }

        fn serialize_some<T: ?::core::marker::Sized + Serialize>(self, value: &T) -> Written {
            value.serialize(Writer::declared::<T>())
        }

        fn serialize_unit(self) -> Written {
            ::serde_json::value::Serializer.serialize_unit()
        }

        fn serialize_unit_struct(self, name: &'static str) -> Written {
            ::serde_json::value::Serializer.serialize_unit_struct(name)
        }

        fn serialize_unit_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
        ) -> Written {
            ::serde_json::value::Serializer.serialize_unit_variant(name, index, variant)
        }

        fn serialize_newtype_struct<T: ?::core::marker::Sized + Serialize>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Written {
            value.serialize(Writer::declared::<T>())
        }

        fn serialize_newtype_variant<T: ?::core::marker::Sized + Serialize>(
            self,
            _name: &'static str,
            _index: u32,
            variant: &'static str,
            value: &T,
        ) -> Written {
            let written = value.serialize(Writer::declared::<T>())?;
            let mut object = ::serde_json::Map::new();
            object.insert(variant.to_owned(), written);
            ::core::result::Result::Ok(Value::Object(object))
        }
    }
}

fn constructors() -> TokenStream {
    quote! {
        fn serialize_seq(
            self,
            len: ::core::option::Option<usize>,
        ) -> ::core::result::Result<Self::SerializeSeq, Failure> {
            ::serde_json::value::Serializer
                .serialize_seq(len)
                .map(|inner| Compound { inner })
        }

        fn serialize_tuple(
            self,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeTuple, Failure> {
            ::serde_json::value::Serializer
                .serialize_tuple(len)
                .map(|inner| Compound { inner })
        }

        fn serialize_tuple_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeTupleStruct, Failure> {
            ::serde_json::value::Serializer
                .serialize_tuple_struct(name, len)
                .map(|inner| Compound { inner })
        }

        fn serialize_tuple_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeTupleVariant, Failure> {
            ::serde_json::value::Serializer
                .serialize_tuple_variant(name, index, variant, len)
                .map(|inner| Compound { inner })
        }

        fn serialize_map(
            self,
            len: ::core::option::Option<usize>,
        ) -> ::core::result::Result<Self::SerializeMap, Failure> {
            ::serde_json::value::Serializer
                .serialize_map(len)
                .map(|inner| Compound { inner })
        }

        fn serialize_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeStruct, Failure> {
            ::serde_json::value::Serializer
                .serialize_struct(name, len)
                .map(|inner| Compound { inner })
        }

        fn serialize_struct_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
            len: usize,
        ) -> ::core::result::Result<Self::SerializeStructVariant, Failure> {
            ::serde_json::value::Serializer
                .serialize_struct_variant(name, index, variant, len)
                .map(|inner| Compound { inner })
        }
    }
}

/// The text of the value `held` borrows.
pub fn text(held: &TokenStream) -> TokenStream {
    let module = module();
    quote! { #module::text(#held) }
}

/// The text of the value the function `through` writes into the serializer it is handed, a
/// `serialize_with`, a `with` module's or the date's `as_number`.
#[cfg(feature = "serde")]
pub fn text_through(through: &TokenStream, held: &TokenStream) -> TokenStream {
    let module = module();
    quote! { #module::rendered(&#through(#held, #module::top()).unwrap()) }
}

fn module() -> Ident {
    format_ident!("{MODULE}")
}

fn compound(is: &Ident, method: &Ident) -> TokenStream {
    quote! {
        impl<I: ::serde::ser::#is> ::serde::ser::#is for Compound<I> {
            type Ok = I::Ok;
            type Error = I::Error;

            fn #method<T: ?::core::marker::Sized + Serialize>(
                &mut self,
                value: &T,
            ) -> ::core::result::Result<(), I::Error> {
                self.inner.#method(&Child(value))
            }

            fn end(self) -> ::core::result::Result<I::Ok, I::Error> {
                self.inner.end()
            }
        }
    }
}
