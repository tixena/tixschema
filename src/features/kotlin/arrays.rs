//! A fixed array `[T; N]`, which the Kotlin client holds as a `List`: serde and the Zod schema read
//! it from an array of `N` items alone, at each level. Each level is read through a class of the
//! item's own over the serializer of its items, which throws `SerializationException` for an array
//! of another length, in the words the read of a tuple throws it with.
//!
//! The plugin reads a declared level by the class its annotation names, handing it the serializer
//! of the items, and a serializer the emitter writes composes it over the one it reads them with.

use core::cell::RefCell;

use super::{KOTLIN_TUPLE_AUX, KOTLIN_TUPLE_COUNTER, texts};
use crate::field_type::FieldDef;

thread_local! {
    /// The class each length the item being expanded reads is read through, so every array of
    /// one length names one. Cleared with the tuple wrappers.
    static KOTLIN_ARRAYS: RefCell<Vec<(usize, String)>> = const { RefCell::new(Vec::new()) };
}

/// Drops the classes of the item just written.
pub fn clear() {
    KOTLIN_ARRAYS.with(|arrays| arrays.borrow_mut().clear());
}

/// The class `named` that reads a list of `length` items, and refuses one of any other.
fn declaration(named: &str, length: usize) -> String {
    format!(
        "class {named}<Item>(item: KSerializer<Item>) : KSerializer<List<Item>> {{ private val \
         listed = ListSerializer(item); override val descriptor: SerialDescriptor = \
         listed.descriptor; override fun serialize(encoder: Encoder, value: List<Item>) = \
         listed.serialize(encoder, value); override fun deserialize(decoder: Decoder): \
         List<Item> {{ val read = listed.deserialize(decoder); if (read.size != {length}) throw \
         SerializationException(\"invalid length \" + read.size + \", expected {length}\"); \
         return read }} }}"
    )
}

/// Whether a value typed `field` holds an array of one length where the plugin reads the
/// annotation the emitter writes on it, the places a text of one form names its serializer.
pub fn declared(field: &FieldDef) -> bool {
    texts::found(field, false, has_a_length)
}

/// Whether `field` itself holds an array of one length at one of its own levels.
const fn has_a_length(field: &FieldDef) -> bool {
    !field.array_lengths.is_empty()
}

/// Whether a serializer the emitter writes reads a value typed `field` through the class of a
/// length it holds, which it composes, where it composes the serializer of a text of one form.
pub fn reaches(field: &FieldDef) -> bool {
    texts::found(field, true, has_a_length)
}

/// The class an array of `length` items is read through, declared once per item.
pub fn serializer(length: usize) -> String {
    KOTLIN_ARRAYS.with(|arrays| {
        let mut written = arrays.borrow_mut();
        if let Some((_, named)) = written.iter().find(|(declared, _)| *declared == length) {
            return named.clone();
        }
        let index = KOTLIN_TUPLE_COUNTER.with(|counter| {
            let next = counter.get() + 1;
            counter.set(next);
            next
        });
        let named = format!("KotlinArray{index}");
        KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(declaration(&named, length)));
        written.push((length, named.clone()));
        named
    })
}
