//! The private functions the Kotlin emitter writes into a serializer that refuses a value it took
//! off the decoder, as the emitter writes them.

/// The functions that rebase a refusal onto the path the decoder states.
pub const PATHED: &str = concat!(
    "private fun kotlinSchemaRebased(refused: SerializationException, at: String): \
     SerializationException { if (refused.cause?.message == \"validate()\") return if \
     (refused.suppressed.isNotEmpty()) refused else kotlinSchemaPlaced(refused, at, true); val said = \
     refused.message.orEmpty().replaceFirst(Regex(\"^Unexpected JSON token at offset \\\\d+: \"), \
     \"\"); return \
     SerializationException(if (\" at path: \\$\" in said) said.replaceFirst(\" at path: \\$\", \" at \
     path: \" + at) else said + \" at path: \" + at, refused) }; @OptIn(ExperimentalSerializationApi::class) \
     private inline fun <KotlinSchemaRead> kotlinSchemaAt(input: Decoder, read: () -> \
     KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) \
     { val at = try { (input as? JsonDecoder)?.decodeJsonElement(); null } catch (probe: \
     JsonDecodingException) { probe.path } ?: throw refused; throw kotlinSchemaRebased(refused, \
     at) }; private inline fun \
     <KotlinSchemaRead> kotlinSchemaUnder(step: String, read: () -> \
     KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) { throw \
     if (refused.cause?.message == \"validate()\") kotlinSchemaPlaced(refused, \"\\$\" + step, false) \
     else kotlinSchemaRebased(refused, \"\\$\" + step) }; ",
    r#"private fun kotlinSchemaPlaced(refused: SerializationException, at: String, held: Boolean): SerializationException { val place = Regex("""\.([^.\[]+)|\[(\d+)]|\['(.*?)']""").findAll(at).joinToString(".") { step -> step.groupValues.drop(1).joinToString("") }; val issues = (refused.javaClass.getMethod("getIssues").invoke(refused) as List<*>).map { said -> said as String }; val rebuilt = refused.javaClass.getConstructor(List::class.java).newInstance(if (place.isEmpty()) issues else issues.map { said -> if (said.startsWith("'") && said.indexOf("'", 1) > 0) "'" + place + "." + said.substring(1) else "'" + place + "': " + said }) as SerializationException; if (held) rebuilt.addSuppressed(IllegalStateException("placed")); return rebuilt }; "#,
);
