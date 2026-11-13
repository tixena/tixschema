//! A brand over a `usize` answered in a reply header, read back by the emitted Swift and Kotlin
//! `http_rest` clients compiled with warnings as errors, at each value the dispatcher writes.

#![cfg(any(feature = "swift", feature = "kotlin"))]

use core::fmt::Write as _;

use super::usize_header_service::{QuotaClientServiceSchema, WRITTEN, quota_client_service_schema};

#[cfg(feature = "kotlin")]
const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

#[cfg(feature = "swift")]
const SWIFT_DRIVER: &str = r##"
struct QuotaRecorder: QuotaClientServiceHttpTransport {
  let quota: String
  func send(_ request: QuotaClientServiceHttpRequest) async throws -> QuotaClientServiceHttpResponse {
    QuotaClientServiceHttpResponse(status: 200, headers: [("x-quota", quota)], body: Data(#"{"label":"a"}"#.utf8))
  }
}

for quota in QUOTAS {
  let client = QuotaClientServiceHttpClient(transport: QuotaRecorder(quota: quota))
  switch await client.ask(QuotaAsk(label: "a")) {
  case .success(let answered): print("\(quota): read \(answered.1.value)")
  case .failure(.fault(let fault)): print("\(quota): fault \(fault.kind.rawValue)")
  case .failure(.declared): print("\(quota): declared")
  }
}
"##;

#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
class QuotaRecorder(private val quota: String) : QuotaClientServiceHttpTransport {
    override suspend fun send(request: QuotaClientServiceHttpRequest): QuotaClientServiceHttpResponse =
        QuotaClientServiceHttpResponse(200, listOf("x-quota" to quota), """{"label":"a"}""".encodeToByteArray())
}

fun main() = runBlocking {
    for (quota in QUOTAS) {
        val answered = QuotaClientServiceHttpClient(QuotaRecorder(quota)).ask(QuotaAskDraft(label = "a").validate())
        val said = when (answered) {
            is QuotaClientServiceAskResult.Ok -> "read ${answered.value.headerOut0.value}"
            is QuotaClientServiceAskResult.Fault -> "fault"
            else -> "declared"
        }
        println("$quota: $said")
    }
}
"#;

fn listed() -> String {
    WRITTEN.map(|value| format!("\"{value}\"")).join(", ")
}

#[cfg(feature = "swift")]
fn quotas() -> String {
    format!("let QUOTAS = [{}]", listed())
}

#[cfg(feature = "kotlin")]
fn quotas_kotlin() -> String {
    format!("val QUOTAS = listOf({})", listed())
}

fn read_each() -> String {
    let mut printed = String::new();
    for value in WRITTEN {
        writeln!(printed, "{value}: read {value}").unwrap();
    }
    printed
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_reads_a_branded_usize_header_at_each_value_the_dispatcher_writes() {
    use super::runtime::judged;
    use super::usize_header_service::{quota_ask_swift, quota_error_swift, quota_swift};
    let source = [
        "import Foundation".to_owned(),
        quota_swift::swift_definition(),
        quota_ask_swift::swift_definition(),
        quota_error_swift::swift_definition(),
        quota_client_service_schema::quota_client_service_fault_fields_swift::swift_definition(),
        quota_client_service_schema::quota_client_service_fault_kind_swift::swift_definition(),
        QuotaClientServiceSchema::swift_http_client(),
        quotas(),
        SWIFT_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-usize-header",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(
        run.stdout,
        read_each(),
        "the Swift client does not read each value"
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_client_reads_a_branded_usize_header_at_each_value_the_dispatcher_writes() {
    use super::runtime::ran_kotlin;
    use super::usize_header_service::{quota_ask_kotlin, quota_error_kotlin, quota_kotlin};
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        quota_kotlin::kotlin_definition(),
        quota_ask_kotlin::kotlin_definition(),
        quota_error_kotlin::kotlin_definition(),
        quota_client_service_schema::quota_client_service_fault_fields_kotlin::kotlin_definition(),
        quota_client_service_schema::quota_client_service_fault_kind_kotlin::kotlin_definition(),
        QuotaClientServiceSchema::kotlin_http_client(),
        quotas_kotlin(),
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(
        printed,
        read_each(),
        "the Kotlin client does not read each value"
    );
}
