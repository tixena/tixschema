//! The private function the Kotlin emitter writes into a serializer that reads a flattened member,
//! as the emitter writes it.

/// The function that reads a flattened member and places a refusal inside it at the holder.
pub const FLATTENED: &str = "private inline fun <KotlinSchemaRead> kotlinSchemaFlattened(read: () -> \
     KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) \
     { if (refused.cause?.message == \"validate()\") throw refused; throw \
     SerializationException(refused.message.orEmpty().replaceFirst(Regex(\"^Unexpected JSON token \
     at offset \\\\d+: \"), \"\").replaceFirst(Regex(\"^'[^']*': \"), \"\").replace(Regex(\" at \
     path: \\\\$[^\\\\n]*\"), \" at path: \\\\$\"), refused) }; ";
