## jiff Date/Time Types

The `jiff` feature is on by default and reads jiff's date and time types. `jiff` and `chrono` are never on together: a build with both stops at a `compile_error!`, and a crate that keeps chrono turns the default features off ([Chrono dates and times](chrono.md#chrono-datetime-types)).

```toml
tixschema = "0.7"
```

A consumer must also enable `serde` on its own `jiff` dependency, named `jiff`: `as_number` (below) injects a `jiff::fmt::serde::timestamp::millisecond::required` hook that only exists there.

Supported types and mappings:

| Rust Type | TypeScript | Zod Schema | JSON Schema |
|-----------|------------|------------|-------------|
| `jiff::civil::Date` | `string` | `z.iso.date()` | `{"type": "string", "format": "date"}` |
| `jiff::civil::Time` | `string` | `z.preprocess(<millis, seconds and fraction arrow>, z.iso.time())` | `{"type": "string", "pattern": <the time jiff writes>}` |
| `jiff::civil::DateTime` | `string` | `z.preprocess(<separator, seconds and fraction arrow>, z.string().regex(<the date and time jiff writes>))` | `{"type": "string", "pattern": <the date and time jiff writes>}` |
| `jiff::Timestamp` (default) | `Date` | `z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())` | `{"type": "string", "format": "date-time"}` |
| `jiff::Timestamp` + `#[model_schema_prop(as_number)]` | `number` | `z.preprocess(<epoch arrow>, z.number().int())` | `{"type": "integer"}` |
| `jiff::ZonedDateTime` (`jiff::Zoned` in jiff 0.2) | `string` | `z.string().regex(<the text jiff writes>)` | `{"type": "string", "pattern": <the text jiff writes>}` |

A bare `Date`, `Time`, `DateTime`, `Timestamp`, `ZonedDateTime` or `Zoned` is jiff's, unless the crate declared a `#[model_schema]` type of its own under that name above the type that holds it: then the name is that type. A `bson::DateTime` field is read as a `jiff::civil::DateTime`: declare the BSON date as a `jiff::Timestamp` stored through `bson::serde_helpers::datetime::FromJiff02Timestamp` instead.

A `Timestamp` takes the place a chrono `DateTime<Utc>` has on every surface: TypeScript holds it as a `Date`, the Zod schema coerces a `Date`, an ISO string or epoch milliseconds to one and refuses anything else, and the JSON Schema is RFC 3339's `date-time`. jiff writes a `Timestamp` in UTC with the fraction in the fewest digits it needs, `2024-01-05T10:20:30.5Z` for half a second past and `2024-01-05T10:20:30Z` for whole seconds, and every client sends it that way.

A `ZonedDateTime` is a date and time with its time zone, and crosses as the one text jiff writes for it, zone named: `2026-11-20T08:00:00-04:00[America/Santo_Domingo]`. Every surface reads that text and writes the same text back, so the instant and the zone go round unchanged. Each client holds it as a string. tixschema writes no other form: a consumer who stores zoned values and needs to sort or index them by instant stores the `Timestamp` beside them.

`as_number` opts a single `Timestamp` field into an epoch-milliseconds `number`, validated by a self-contained inline coercer. `as_number` is only valid on a `Timestamp` field: using it elsewhere is a compile error. On the Rust side it injects `#[serde(with = "jiff::fmt::serde::timestamp::millisecond::required")]` (or `millisecond::optional` for an `Option<Timestamp>`), so the field serializes to, and deserializes from, the epoch milliseconds every surface describes. Where the field already carries its own `with`, or a `serialize_with` beside a `deserialize_with`, the injection is held back; a `serialize_with` or a `deserialize_with` of the author's alone keeps its direction, and the injected hook takes the other. An `Option` under a key the injected hook reads takes `#[serde(default)]` too, so an absent key is read as `None`.

jiff's hook reads a `Timestamp`, and an `Option` of one, alone. `as_number` on any other date a hook would be injected on stops the build: `pub struct Stamped { #[model_schema_prop(as_number)] pub stamp: Vec<Timestamp> }` answers:

```text
error: model_schema: field `stamp`: `#[model_schema_prop(as_number)]` writes and reads the field through jiff's `millisecond::required` hook, which takes a `jiff::Timestamp`, or an `Option<jiff::Timestamp>` through `millisecond::optional`, and the field is a `Vec<Timestamp>`. Declare the field as one of those two, or write and read it through functions of your own: a `with`, or a `serialize_with` beside a `deserialize_with`.
```

A `Timestamp` under `as_number` is the integer of its epoch milliseconds wherever a refusal names what serde writes. Held by the lone slot of a variant under a tag alone, it is refused as `an integer`, quoting serde's refusal of the write, `cannot serialize tagged newtype variant Stamping::Stamp containing an integer`; flattened, it is refused as an `integer`, which serde refuses to flatten (`can only flatten structs and maps (got an integer)`). A bound on it is refused, as on any date: `#[model_schema_prop(as_number, minimum = 0)]` answers ``error: model_schema: field `stamp`: `minimum` cannot apply to `jiff::Timestamp` under `as_number` — no surface reads a bound beside a date, the epoch milliseconds `as_number` writes it as among them: the constraint would reach neither Zod, nor the JSON schema, nor the generated validator. Drop it, or carry the milliseconds in an `i64` field, which `minimum` and `maximum` bound.``

The JSON Schema describes the form jiff writes. A `civil::Time` and a `civil::DateTime` carry no zone, so each is held to a pattern of the text jiff writes, its seconds written and its fraction in the fewest digits it needs: `^(?:(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\.[0-9]*[1-9])?)$` for a `civil::Time`, which reads `10:20:30` and `10:20:30.125` and refuses `10:20:30.500` and `10:20:30Z`, and the pattern of the date, a `T` and that time for a `civil::DateTime`.

jiff reads more than it writes, and each surface reads as Zod reads and sends what jiff writes ([The Form Each Client Sends](client-forms.md#the-form-each-client-sends)). `civil::Time`'s Zod schema accepts millis-since-start-of-day, writes the seconds an `HH:MM` text leaves out, and drops the trailing zeros of a fraction: `10:20` is `10:20:00`, and `10:20:30.500` is `10:20:30.5`. `civil::DateTime`'s Zod schema writes the `T` jiff writes where jiff reads a space, the seconds, and the fraction the same way: `2024-01-05 10:20` is `2024-01-05T10:20:00`. The text is then held to the form jiff writes, so a `Z`, which jiff refuses on a civil date and time, is refused: `2024-01-05T10:20:30.000Z` is no `civil::DateTime`, where `z.iso.datetime({ local: true })` reads it. `2026-11-20T00:00:00`, which jiff reads as a `civil::Date`, is refused where `z.iso.date()` refuses it.

Example:

```rust
use tixschema::{model_schema, model_schema_prop};
use serde::{Deserialize, Serialize};
use jiff::{Timestamp, civil::{Date, Time, DateTime}};

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub created_at: Timestamp,
    pub date: Date,
    #[model_schema_prop(as_number)]
    pub epoch_ms: Timestamp,
    pub local_datetime: DateTime,
    pub name: String,
    pub time: Time,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<Timestamp>,
}
```

Generated TypeScript:

```typescript
export type EventDraft = {
  created_at: Date;
  date: string;
  epoch_ms: number;
  local_datetime: string;
  name: string;
  time: string;
  updated_at: Date | undefined;
};

export type Event = Readonly<EventDraft> & $brand<"Event">;
```

With `created_at` at `2026-11-20T12:00:00.5Z` and `epoch_ms` at `2026-11-20T12:00:00.123Z`, `serde_json::to_string(&event)` writes `"created_at":"2026-11-20T12:00:00.5Z"` and `"epoch_ms":1795176000123`.

jiff types also work in collections (`Vec<Date>` generates `z.array(z.iso.date())`) and in enums as tuple variant elements (`as_number` is honored on a tuple-variant `Timestamp` payload).
