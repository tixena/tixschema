## MongoDB ObjectId Support

Enable the `mongodb` feature for first-class MongoDB ObjectId support with proper serialization and validation.

```rust
use tixschema::model_schema;
use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Document {
    pub id: ObjectId,
    pub title: String,
    pub author_id: ObjectId,
    pub tags: Vec<ObjectId>,
    pub metadata: HashMap<String, ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ObjectId>,
    pub related_docs: HashMap<String, Vec<ObjectId>>,
}
```

Generated TypeScript:

```typescript
export type Document = {
  id: ObjectId;
  title: string;
  author_id: ObjectId;
  tags: Array<ObjectId>;
  metadata: Partial<Record<string, ObjectId>>;
  parent_id: ObjectId | undefined;
  related_docs: Partial<Record<string, Array<ObjectId>>>;
};

export const Document$Schema = z.strictObject({
  id: z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  title: z.string(),
  author_id: z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  tags: z.array(z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  metadata: z.record(z.string(), z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  parent_id: z.union([z.null().transform(() => undefined), z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }), z.undefined()]).prefault(undefined),
  related_docs: z.record(z.string(), z.array(z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }))),
}).meta({
  examples: []
});
```

ObjectIds serialize to MongoDB's standard JSON format:

```json
{
  "id": { "$oid": "507f1f77bcf86cd799439011" },
  "title": "My Document",
  "author_id": { "$oid": "507f1f77bcf86cd799439012" },
  "tags": [
    { "$oid": "507f1f77bcf86cd799439013" },
    { "$oid": "507f1f77bcf86cd799439014" }
  ]
}
```

Key details:

- Uses `{ "$oid": "hex_string" }` format matching MongoDB's native serialization.
- Validates 24-character hexadecimal ObjectId strings via regex.
- Refuses a key beside `$oid`, as Serde refuses it: Zod writes the `$oid` object as a `z.strictObject`, and the JSON Schema writes `"additionalProperties": false`. A flattened `ObjectId` is joined as a flattened struct is, so its holder refuses a key none of its members reads, where Serde reads past it.
- Supports ObjectIds in arrays, HashMaps, optional fields, and deeply nested structures.
- The MongoDB crate is a dev-dependency only -- zero production overhead.

Every non-Rust target treats `ObjectId` as a bare reference rather than publishing it -- `ts_definition()` writes `ObjectId` unresolved (see "Missing TypeScript Types" below), and `dart_definition()`, `swift_definition()` and `kotlin_definition()` do the same. The consuming project supplies a type in its own language that reads and writes the `{ "$oid": "hex_string" }` wire form, and refuses what Serde refuses: an object that holds a key beside `$oid`, and a `$oid` that is not 24 hexadecimal digits. Serde reads the digits in upper case too; these types, the Zod schema and the JSON Schema read them in lower case alone, as Serde writes them. The Swift type is public and `Sendable`, as the types `swift_definition()` writes are, and the Kotlin type refuses another key under the default `Json`, as every class the plugin reads does:

```dart
class ObjectId {
  final String oid;
  ObjectId(this.oid);
  factory ObjectId.fromJson(Map<String, dynamic> json) => switch (json) {
        {'\$oid': final String oid}
            when json.length == 1 && RegExp(r'^[a-f0-9]{24}$').hasMatch(oid) =>
          ObjectId(oid),
        _ => throw FormatException('Invalid ObjectId: $json'),
      };
  Map<String, dynamic> toJson() => {'\$oid': oid};
}
```

```swift
public struct ObjectId: Codable, Sendable {
  public let oid: String
  enum CodingKeys: String, CodingKey { case oid = "$oid" }

  public init(oid: String) { self.oid = oid }

  public init(from decoder: Decoder) throws {
    let keys = try decoder.container(keyedBy: AnyKey.self).allKeys
    oid = try decoder.container(keyedBy: CodingKeys.self).decode(String.self, forKey: .oid)
    guard keys.count == 1, oid.count == 24, oid.allSatisfy({ "0123456789abcdef".contains($0) })
    else {
      throw DecodingError.dataCorrupted(
        DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Invalid ObjectId"))
    }
  }

  private struct AnyKey: CodingKey {
    let stringValue: String
    var intValue: Int? { nil }
    init?(stringValue: String) { self.stringValue = stringValue }
    init?(intValue: Int) { nil }
  }
}
```

```kotlin
@Serializable
data class ObjectId(@SerialName("\$oid") val oid: String) {
    init {
        if (!Regex("[a-f0-9]{24}").matches(oid)) throw SerializationException("Invalid ObjectId: $oid")
    }
}
```

Where an `ObjectId` is flattened, Serde reads past a key beside `$oid`. So the Dart and Swift reads hand a flattened `ObjectId`, and a brand, a struct of one slot or an alias over one declared above its holder, the entry of its `$oid` key alone, and the Kotlin read reads it with `ignoreUnknownKeys`. An optional one is read as no value where the object holds no `$oid` key, and is refused where the key does not read.

Elsewhere the `ObjectId` is handed the object any flattened member is handed, and the keys of a member flattened after it, and of one flattened ahead of it whose keys the client does not know, reach it and refuse the payload, where Serde reads it:

- in Dart and in Swift, for a brand or a struct of one slot over an `ObjectId` declared below its holder, and in Dart for an alias of one declared there: a type the emitter has not written. In Swift an alias declared below its holder is a type tixschema has not written, which the `ObjectId` above does not declare itself as (see [`validate()` in the Swift client](swift.md#validate-in-the-swift-client)), so its holder does not compile;
