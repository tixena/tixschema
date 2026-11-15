//! The private functions the Kotlin emitter writes into the judged serializer of a type, as the
//! emitter writes them.

/// What the judged serializer of a type holds, which throws its refusal placed at the path it read
/// the value at.
pub const THROWN: &str = r#"@OptIn(ExperimentalSerializationApi::class) private fun kotlinSchemaThrown(input: Decoder, refused: SerializationException): SerializationException { val at = try { (input as? JsonDecoder)?.decodeJsonElement(); null } catch (probe: JsonDecodingException) { probe.path } ?: return refused; val place = Regex("""\.([^.\[]+)|\[(\d+)]|\['(.*?)']""").findAll(at).joinToString(".") { step -> step.groupValues.drop(1).joinToString("") }; if (place.isEmpty()) return refused; val issues = (refused.javaClass.getMethod("getIssues").invoke(refused) as List<*>).map { said -> said as String }; val rebuilt = refused.javaClass.getConstructor(List::class.java).newInstance(issues.map { said -> if (said.startsWith("'") && said.indexOf("'", 1) > 0) "'" + place + "." + said.substring(1) else "'" + place + "': " + said }) as SerializationException; rebuilt.addSuppressed(IllegalStateException("placed")); return rebuilt }; "#;

/// What the judged serializer of a type holds that answers a refusal a value it holds threw as the
/// refusal of the type read, `refusal`.
pub fn held_refusal(refusal: &str) -> String {
    format!(
        "private fun kotlinSchemaHeld(refused: SerializationException): SerializationException {{ \
         if (refused.cause?.message != \"validate()\") return refused; val issues = \
         (refused.javaClass.getMethod(\"getIssues\").invoke(refused) as List<*>).map {{ said -> \
         said as String }}; val rebuilt = {refusal}(issues); refused.suppressed.forEach {{ marked -> \
         rebuilt.addSuppressed(marked) }}; return rebuilt }}; "
    )
}
