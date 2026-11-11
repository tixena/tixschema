//! The private functions the Kotlin emitter writes into a serializer that refuses a value it took
//! off the decoder, as the emitter writes them.

/// The functions that rebase a refusal onto the path the decoder states.
pub const PATHED: &str = "private fun kotlinSchemaRebased(refused: SerializationException, at: \
     String): SerializationException { if (refused.cause?.message == \"validate()\") return \
     refused; val said = \
     refused.message.orEmpty().replaceFirst(Regex(\"^Unexpected JSON token at offset \\\\d+: \"), \
     \"\"); return \
     SerializationException(if (\" at path: \\$\" in said) said.replaceFirst(\" at path: \\$\", \" at \
     path: \" + at) else said + \" at path: \" + at, refused) }; @OptIn(ExperimentalSerializationApi::class) \
     private inline fun <KotlinSchemaRead> kotlinSchemaAt(input: Decoder, read: () -> \
     KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) \
     { val at = try { (input as? JsonDecoder)?.decodeJsonElement(); null } catch (probe: \
     JsonDecodingException) { probe.path } ?: throw refused; throw kotlinSchemaRebased(refused, \
     at) }; private inline fun \
     <KotlinSchemaRead> kotlinSchemaUnder(step: String, read: () -> KotlinSchemaRead): \
     KotlinSchemaRead = try { read() } catch (refused: SerializationException) { throw \
     kotlinSchemaRebased(refused, \"\\$\" + step) }; ";
