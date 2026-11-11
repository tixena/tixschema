//! The private function the Kotlin emitter writes into a serializer that reads a part of a value
//! it took off the decoder apart, as the emitter writes it.

/// The function that reads a part of a value apart, from its text.
pub const INNER: &str = "private fun <KotlinSchemaRead> kotlinSchemaInner(json: Json, step: String, \
     serializer: DeserializationStrategy<KotlinSchemaRead>, element: JsonElement): \
     KotlinSchemaRead = kotlinSchemaUnder(step) { json.decodeFromString(serializer, \
     element.toString()) }; ";
