## Chrono Date/Time Types

Enable the `chrono` feature for chrono date/time type support. All chrono types map to `string` in TypeScript, with appropriate Zod validation for the specific date/time format.

```toml
tixschema = { version = "0.4", features = ["chrono"] }
```

A consumer enabling `chrono` on tixschema must also enable `serde` on its own `chrono` dependency — `as_number` (below) injects a `chrono::serde::ts_milliseconds` hook that only exists there.

Supported types and mappings:

| Rust Type | TypeScript | Zod Schema | JSON Schema |
|-----------|------------|------------|-------------|
| `NaiveDate` | `string` | `z.iso.date()` | `{"type": "string", "format": "date"}` |
| `NaiveTime` | `string` | `z.preprocess(<millis and seconds arrow>, z.iso.time())` | `{"type": "string", "pattern": <the time serde writes>}` |
| `NaiveDateTime` | `string` | `z.preprocess(<zone and seconds arrow>, z.iso.datetime({ local: true }))` | `{"type": "string", "pattern": <the date and time serde writes>}` |
| `DateTime<Tz>` (default) | `Date` | `z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())` | `{"type": "string", "format": "date-time"}` |
| `DateTime<Tz>` + `#[model_schema_prop(as_number)]` | `number` | `z.preprocess(<epoch arrow>, z.number().int())` | `{"type": "integer"}` |

`DateTime<Tz>` renders as a native TypeScript `Date` by default, which is what MongoDB needs to expire a BSON `Date` via TTL. A `Date`, an ISO string and epoch milliseconds are coerced to one; anything else is refused before the coercion, since `z.coerce.date()` alone reads `null` as `1970-01-01T00:00:00.000Z` and `true` as one millisecond later. An `Option<DateTime<Tz>>` still reads `null` as `undefined`, through the `z.null()` arm every `Option` has. A `nullable` one reads `null` as `null`, a `null` among the items of a `Vec<Option<DateTime<Tz>>>` stays `null`, and in an untagged enum an input that is no date reaches the member written after the date. An object that is no `Date` is refused even where `new Date` would convert it, as it would a dayjs or a moment value: hand the schema the `Date` itself. The bare `as_number` flag opts a single `DateTime<Tz>` field into an epoch-milliseconds `number` instead, validated by a self-contained inline coercer (no imported helper). `as_number` is only valid on a `DateTime<Tz>` field — using it elsewhere is a compile error.

On the Rust side, `as_number` also injects `#[serde(with = "chrono::serde::ts_milliseconds")]` (or `ts_milliseconds_option` for an `Option<DateTime<Utc>>`), so the field serializes to, and deserializes from, the same epoch-milliseconds number every surface describes — no hand-written serde attribute needed. It is injected wherever the field is: a struct's field, a tuple struct's slot, and a member of an enum's variant under every tagging, a member of an `#[serde(untagged)]` enum's variant among them. Where the field already carries its own `with`, or a `serialize_with` beside a `deserialize_with`, the injection is held back and the author's hooks run instead. A `serialize_with` or a `deserialize_with` of the author's alone keeps its direction, and the injected hook takes the other: `deserialize_with = "chrono::serde::ts_milliseconds::deserialize"` beside the author's `serialize_with`, `serialize_with = "chrono::serde::ts_milliseconds::serialize"` beside the author's `deserialize_with`. An `Option` under a key that the injected hook reads takes `#[serde(default)]` too, so an absent key is read as `None`, as serde reads an `Option` read by no hook: `#[model_schema_prop(as_number)] #[serde(skip_serializing_if = "Option::is_none")] pub later: Option<DateTime<Utc>>` reads back the `{}` serde writes for `None`.

chrono's hook reads a `DateTime<Utc>`, and an `Option` of one, alone. `as_number` on any other date a hook would be injected on stops the build, a list of dates, a date in a `Box` and a date in another zone among them: `pub struct Stamped { #[model_schema_prop(as_number)] pub stamp: Vec<DateTime<Utc>> }` answers:

```text
error: model_schema: field `stamp`: `#[model_schema_prop(as_number)]` writes and reads the field through chrono's `ts_milliseconds` hook, which takes a `DateTime<Utc>`, or an `Option<DateTime<Utc>>` through `ts_milliseconds_option`, and the field is a `Vec<DateTime<Utc>>`. Declare the field as one of those two, or write and read it through functions of your own: a `with`, or a `serialize_with` beside a `deserialize_with`.
```

A date under `as_number` is the integer of its epoch milliseconds wherever a refusal names what serde writes. Held by the lone slot of a variant under a tag alone, it is refused as `an integer`, quoting serde's refusal of the write, `cannot serialize tagged newtype variant Stamping::Stamp containing an integer` for `#[serde(tag = "kind")] pub enum Stamping { Stamp(#[model_schema_prop(as_number)] DateTime<Utc>) }`; flattened, it is refused as an `integer`, which serde refuses to flatten (`can only flatten structs and maps (got an integer)`); and a struct of one slot, an alias or an untagged enum's member over one, declared above, is refused there as that integer. A bound on it is refused, as on any date: `#[model_schema_prop(as_number, minimum = 0)]` answers ``error: model_schema: field `stamp`: `minimum` cannot apply to `chrono::DateTime` under `as_number` — no surface reads a bound beside a date, the epoch milliseconds `as_number` writes it as among them: the constraint would reach neither Zod, nor the JSON schema, nor the generated validator. Drop it, or carry the milliseconds in an `i64` field, which `minimum` and `maximum` bound.``

The JSON Schema describes the form serde writes. A `NaiveDate` is RFC 3339's `date` format and a `DateTime<Tz>` its `date-time`. A `NaiveTime` and a `NaiveDateTime` carry no zone, which RFC 3339's `time` and `date-time` formats require and serde refuses, so each is held to a pattern of the text serde writes, its seconds written: `^(?:(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\.[0-9]+)?)$` for a `NaiveTime`, which reads `10:20:30` and `10:20:30.125` and refuses `10:20:30Z`, and the pattern of the date, a `T` and that time for a `NaiveDateTime`. Under `as_number` a `DateTime<Tz>` is `{"type": "integer"}`, the epoch milliseconds that hook writes and reads, and a date's text is refused, as the hook refuses it.

`NaiveTime` stays a TypeScript `string`, but its Zod schema also accepts millis-since-start-of-day, converting them to an `HH:MM:SS` string before validation, and writes the seconds an `HH:MM` text leaves out: `10:20` is `10:20:00`. `NaiveDateTime`'s Zod schema writes a text in the form serde reads the same way: `2024-01-05T10:20:30.000Z`, the text `toISOString()` writes, is `2024-01-05T10:20:30.000`, the `Z` serde refuses on a date and time with no zone dropped and the clock digits kept, and `2024-01-05T10:20` is `2024-01-05T10:20:00`. A client sends what the schema gives back, so each is sent in the form serde reads ([The Form Each Client Sends](client-forms.md#the-form-each-client-sends)). The `as_number` coercer ends on `z.number().int()`: serde reads no fraction of a millisecond.

Example:

```rust
use tixschema::{model_schema, model_schema_prop};
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, NaiveTime, NaiveDateTime, DateTime, Utc};

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub name: String,
    pub date: NaiveDate,
    pub time: NaiveTime,
    pub local_datetime: NaiveDateTime,
    pub created_at: DateTime<Utc>,
    #[model_schema_prop(as_number)]
    pub epoch_ms: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}
```

Generated TypeScript:

```typescript
export type Event = {
  name: string;
  date: string;
  time: string;
  local_datetime: string;
  created_at: Date;
  epoch_ms: number;
  updated_at: Date | undefined;
};
```

Generated Zod:

```typescript
export const Event$Schema: ZodType<Event> = z.strictObject({
  name: z.string(),
  date: z.iso.date(),
  time: z.preprocess((arg) => { if (typeof arg === "number") { const s = Math.floor(arg / 1000); const hh = String(Math.floor(s / 3600)).padStart(2, "0"); const mm = String(Math.floor((s % 3600) / 60)).padStart(2, "0"); const ss = String(s % 60).padStart(2, "0"); return `${hh}:${mm}:${ss}`; } if (typeof arg === "string" && /^\d{2}:\d{2}$/.test(arg)) return `${arg}:00`; return arg; }, z.iso.time()),
  local_datetime: z.preprocess((arg) => { if (typeof arg !== "string") return arg; const read = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2})(:\d{2}(?:\.\d+)?)?Z?$/.exec(arg); return read ? `${read[1]}${read[2] ?? ":00"}` : arg; }, z.iso.datetime({ local: true })),
  created_at: z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()),
  epoch_ms: z.preprocess((arg) => { if (arg instanceof Date) return arg.getTime(); if (typeof arg === "string") return Date.parse(arg); return arg; }, z.number().int()),
  updated_at: z.union([z.null().transform(() => undefined), z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()), z.undefined()]).prefault(undefined),
}).meta({
  examples: []
});
```

Chrono types also work in collections (`Vec<NaiveDate>` generates `z.array(z.iso.date())`) and in enums as tuple variant elements (`as_number` is honored on a tuple-variant `DateTime<Tz>` payload).
