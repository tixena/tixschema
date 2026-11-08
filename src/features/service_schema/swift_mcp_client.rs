//! The Swift `mcp` client: one `tools/call` per operation, sent through the caller's one-method
//! transport and read back as the `Result<Success, Failure>` `swift_http_client()` declares. An
//! actor numbers its requests in its own state.

use super::swift_http_client::{
    message_swift_typename, swift_call, swift_parameter, swift_sent_value,
};
use super::swift_type::swift_typename_of;
use super::swift_ws_client::{header_in_params, sent_forms, sent_payload};
use super::{left_to_http_rest, on_one_line};
use crate::features::swift::{forms, swift_quoted};
use crate::rename_rule::RenameRule;
use crate::service_schema::mcp_tools::{is_content_list, is_content_pair, wrapped_ident};
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationOutcome, ServiceDef, is_unit_type, option_inner,
    tuple_elements, written,
};
use core::fmt::Write as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

const TRANSPORT: &str = "/// One JSON-RPC text for the transport to carry, and the values an HTTP transport writes as headers.
public struct @N@McpMessage: Sendable {
  public let text: String
  public let protocolVersion: String
  public let method: String
  public let name: String?

  public init(text: String, protocolVersion: String, method: String, name: String?) {
    self.text = text
    self.protocolVersion = protocolVersion
    self.method = method
    self.name = name
  }
}

/// What binds a `@N@` Swift client to a server: it carries one message and answers the text that came back.
public protocol @N@McpTransport: Sendable {
  func send(_ message: @N@McpMessage) async throws -> String
}";

const SUPPORT: &str = r#"func @P@McpFault(_ kind: @N@FaultKind, _ operation: String, _ detail: String) -> @N@Fault {
  @N@Fault(detail: detail, field: nil, kind: kind, operation: operation)
}

func @P@McpDatedEncoder() -> JSONEncoder {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
  encoder.dateEncodingStrategy = .custom { date, encoder in
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    var container = encoder.singleValueContainer()
    try container.encode(formatter.string(from: date).replacingOccurrences(of: ".000Z", with: "Z"))
  }
  return encoder
}

func @P@McpDatedDecoder() -> JSONDecoder {
  let decoder = JSONDecoder()
  decoder.dateDecodingStrategy = .custom { decoder in
    let container = try decoder.singleValueContainer()
    let text = try container.decode(String.self)
    let fractional = ISO8601DateFormatter()
    fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    let whole = ISO8601DateFormatter()
    whole.formatOptions = [.withInternetDateTime]
    guard let date = fractional.date(from: text) ?? whole.date(from: text) else {
      throw DecodingError.dataCorruptedError(in: container, debugDescription: "expected a date and time with its zone, found \(text)")
    }
    return date
  }
  return decoder
}

struct @N@McpValue: Encodable {
  private let write: (Encoder) throws -> Void

  init<Value: Encodable>(_ value: Value) {
    write = { encoder in
      var container = encoder.singleValueContainer()
      try container.encode(value)
    }
  }

  func encode(to encoder: Encoder) throws {
    try write(encoder)
  }
}

struct @N@McpArguments: Encodable {
  private struct Key: CodingKey {
    let stringValue: String
    var intValue: Int? { nil }
    init(_ text: String) { stringValue = text }
    init?(stringValue: String) { self.stringValue = stringValue }
    init?(intValue: Int) { return nil }
  }

  let wrapped: Bool
  let message: @N@McpValue
  let headers: [String: @N@McpValue]

  func encode(to encoder: Encoder) throws {
    if !wrapped { try message.encode(to: encoder) }
    if wrapped || !headers.isEmpty {
      var container = encoder.container(keyedBy: Key.self)
      if wrapped { try container.encode(message, forKey: Key("value")) }
      for (name, value) in headers { try container.encode(value, forKey: Key(name)) }
    }
  }
}

struct @N@McpRequest: Encodable {
  struct Params: Encodable {
    struct Meta: Encodable {
      struct Capabilities: Encodable {}
      struct Info: Encodable {
        let name: String
        let version: String
      }
      let capabilities = Capabilities()
      let info = Info(name: "@N@McpClient", version: "@MCP_VERSION@")
      let protocolVersion = "2026-07-28"
      private enum CodingKeys: String, CodingKey {
        case capabilities = "io.modelcontextprotocol/clientCapabilities"
        case info = "io.modelcontextprotocol/clientInfo"
        case protocolVersion = "io.modelcontextprotocol/protocolVersion"
      }
    }
    let arguments: @N@McpArguments
    let meta = Meta()
    let name: String
    private enum CodingKeys: String, CodingKey {
      case arguments
      case meta = "_meta"
      case name
    }
  }
  let id: Int
  let jsonrpc = "2.0"
  let method = "tools/call"
  let params: Params
}

struct @N@McpProbe: Decodable {
  struct Failure: Decodable {
    struct Marker: Decodable { let isServiceFault: Bool? }
    let code: Int?
    let message: String?
    let data: Marker?
    private enum CodingKeys: String, CodingKey { case code, message, data }
    init(from decoder: Decoder) throws {
      let container = try decoder.container(keyedBy: CodingKeys.self)
      code = try? container.decode(Int.self, forKey: .code)
      message = try? container.decode(String.self, forKey: .message)
      data = try? container.decode(Marker.self, forKey: .data)
    }
  }
  struct Outcome: Decodable {
    struct Block: Decodable { let text: String? }
    let isError: Bool?
    let content: [Block]?
  }
  let error: Failure?
  let result: Outcome?
}

struct @N@McpTagged: Decodable {
  let isServiceFault: Bool?
  let fault: @N@Fault?
}

struct @N@McpInError: Decodable {
  struct Failure: Decodable { let data: @N@McpTagged }
  let error: Failure
}

struct @N@McpInText: Decodable {
  let text: @N@McpTagged
}

struct @N@McpResult<Payload: Decodable>: Decodable {
  struct Inner: Decodable { let structuredContent: Payload }
  let result: Inner
}

struct @N@McpDeclared<Payload: Decodable>: Decodable {
  let error: Payload
}

enum @N@McpExchanged {
  case answered(Data)
  case failed(@N@Fault)
}

enum @N@McpReading {
  case succeeded
  case declared(Data)
  case faulted(@N@Fault)
}

func @P@McpClassify(_ tool: String, _ raw: Data) -> @N@McpReading {
  let probe: @N@McpProbe
  do {
    probe = try JSONDecoder().decode(@N@McpProbe.self, from: raw)
  } catch {
    return .faulted(@P@McpFault(.undeserializablePayload, tool, "\(error)"))
  }
  if let failure = probe.error {
    if failure.data?.isServiceFault != nil {
      if let tagged = try? JSONDecoder().decode(@N@McpInError.self, from: raw), let fault = tagged.error.data.fault {
        return .faulted(fault)
      }
      return .faulted(@P@McpFault(.undeserializablePayload, tool, "the answer carries no readable fault"))
    }
    let code = failure.code.map { String($0) } ?? "null"
    return .faulted(@P@McpFault(.transportFailure, tool, "\(code): \(failure.message ?? "")"))
  }
  guard let result = probe.result else {
    return .faulted(@P@McpFault(.undeserializablePayload, tool, "the answer holds neither a result nor an error"))
  }
  if result.isError != true { return .succeeded }
  guard let text = result.content?.first?.text else {
    return .faulted(@P@McpFault(.undeserializablePayload, tool, "the error holds no text"))
  }
  if let tagged = try? JSONDecoder().decode(@N@McpInText.self, from: Data("{\"text\":\(text)}".utf8)),
    tagged.text.isServiceFault != nil
  {
    if let fault = tagged.text.fault { return .faulted(fault) }
    return .faulted(@P@McpFault(.undeserializablePayload, tool, "the answer carries no readable fault"))
  }
  return .declared(Data("{\"error\":\(text)}".utf8))
}"#;

const ACTOR_HEAD: &str = r#"public actor @N@McpClient {
  private let transport: any @N@McpTransport
  private var nextId = 0

  public init(transport: any @N@McpTransport) {
    self.transport = transport
  }

  private func exchange(_ tool: String, _ arguments: @N@McpArguments) async -> @N@McpExchanged {
    nextId += 1
    let text: String
    do {
      let data = try @P@McpDatedEncoder().encode(@N@McpRequest(id: nextId, params: .init(arguments: arguments, name: tool)))
      text = String(decoding: data, as: UTF8.self)
    } catch {
      return .failed(@P@McpFault(.failedValidation, tool, "\(error)"))
    }
    do {
      let answered = try await transport.send(
        @N@McpMessage(text: text, protocolVersion: "2026-07-28", method: "tools/call", name: tool))
      return .answered(Data(answered.utf8))
    } catch {
      return .failed(@P@McpFault(.transportFailure, tool, "\(error)"))
    }
  }

"#;

const BLOCKS: &str = "struct @N@McpBlocks<Payload: Decodable>: Decodable {
  struct Inner: Decodable { let content: Payload }
  let result: Inner
}";

const PAIR: &str = "struct @N@McpPair: Decodable {
  struct Inner: Decodable {
    struct Block: Decodable {
      let data: Data?
      let mimeType: String?
    }
    struct Structured: Decodable {
      let bytes: [UInt8]
      let contentType: String

      init(from decoder: Decoder) throws {
        var container = try decoder.unkeyedContainer()
        bytes = try container.decode([UInt8].self)
        contentType = try container.decode(String.self)
      }
    }
    let structuredContent: Structured?
    let content: [Block]?
  }
  let result: Inner
}";

/// The type an `mcp(content)` operation answers a list of, as the service module declares it in
/// Rust: one block of a tool's `content`, read and written as the specification writes it.
pub const CONTENT_TYPE: &str = r#"/// One block of a tool's `content`: what an `mcp(content)` operation answers a list of.
public enum McpContent: Sendable, Equatable, Codable {
  /// Text.
  case text(String)
  /// An image: its bytes and its content type.
  case image(data: [UInt8], mimeType: String)
  /// Audio: its bytes and its content type.
  case audio(data: [UInt8], mimeType: String)
  /// A link to a resource the client can read.
  case resourceLink(uri: String, name: String, title: String?, description: String?, mimeType: String?)
  /// A resource carried in the result.
  case resource(uri: String, mimeType: String?, contents: McpResourceContents)

  private enum Keys: String, CodingKey {
    case type, text, data, mimeType, uri, name, title, description, resource, blob
  }

  private enum ResourceKeys: String, CodingKey {
    case uri, mimeType, text, blob
  }

  private static func bytes(_ text: String, _ container: KeyedDecodingContainer<some CodingKey>) throws -> [UInt8] {
    guard let decoded = Data(base64Encoded: text) else {
      throw DecodingError.dataCorrupted(
        .init(codingPath: container.codingPath, debugDescription: "expected base64, found \(text)"))
    }
    return [UInt8](decoded)
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: Keys.self)
    switch try container.decode(String.self, forKey: .type) {
    case "text":
      self = .text(try container.decode(String.self, forKey: .text))
    case "image":
      self = .image(
        data: try Self.bytes(try container.decode(String.self, forKey: .data), container),
        mimeType: try container.decode(String.self, forKey: .mimeType))
    case "audio":
      self = .audio(
        data: try Self.bytes(try container.decode(String.self, forKey: .data), container),
        mimeType: try container.decode(String.self, forKey: .mimeType))
    case "resource_link":
      self = .resourceLink(
        uri: try container.decode(String.self, forKey: .uri),
        name: try container.decode(String.self, forKey: .name),
        title: try container.decodeIfPresent(String.self, forKey: .title),
        description: try container.decodeIfPresent(String.self, forKey: .description),
        mimeType: try container.decodeIfPresent(String.self, forKey: .mimeType))
    case "resource":
      let resource = try container.nestedContainer(keyedBy: ResourceKeys.self, forKey: .resource)
      let contents: McpResourceContents
      if let blob = try resource.decodeIfPresent(String.self, forKey: .blob) {
        contents = .blob(try Self.bytes(blob, resource))
      } else {
        contents = .text(try resource.decode(String.self, forKey: .text))
      }
      self = .resource(
        uri: try resource.decode(String.self, forKey: .uri),
        mimeType: try resource.decodeIfPresent(String.self, forKey: .mimeType),
        contents: contents)
    default:
      throw DecodingError.dataCorruptedError(
        forKey: .type, in: container, debugDescription: "a block of a type this client does not read")
    }
  }

  public func encode(to encoder: Encoder) throws {
    var container = encoder.container(keyedBy: Keys.self)
    switch self {
    case .text(let text):
      try container.encode("text", forKey: .type)
      try container.encode(text, forKey: .text)
    case .image(let data, let mimeType):
      try container.encode("image", forKey: .type)
      try container.encode(Data(data).base64EncodedString(), forKey: .data)
      try container.encode(mimeType, forKey: .mimeType)
    case .audio(let data, let mimeType):
      try container.encode("audio", forKey: .type)
      try container.encode(Data(data).base64EncodedString(), forKey: .data)
      try container.encode(mimeType, forKey: .mimeType)
    case .resourceLink(let uri, let name, let title, let description, let mimeType):
      try container.encode("resource_link", forKey: .type)
      try container.encode(uri, forKey: .uri)
      try container.encode(name, forKey: .name)
      try container.encodeIfPresent(title, forKey: .title)
      try container.encodeIfPresent(description, forKey: .description)
      try container.encodeIfPresent(mimeType, forKey: .mimeType)
    case .resource(let uri, let mimeType, let contents):
      try container.encode("resource", forKey: .type)
      var resource = container.nestedContainer(keyedBy: ResourceKeys.self, forKey: .resource)
      try resource.encode(uri, forKey: .uri)
      try resource.encodeIfPresent(mimeType, forKey: .mimeType)
      switch contents {
      case .text(let text): try resource.encode(text, forKey: .text)
      case .blob(let data): try resource.encode(Data(data).base64EncodedString(), forKey: .blob)
      }
    }
  }
}

/// What an embedded resource holds.
public enum McpResourceContents: Sendable, Equatable {
  /// Text.
  case text(String)
  /// Bytes.
  case blob([UInt8])
}"#;

/// Where the accessor writes the version of the crate that declares the service.
const VERSION_MARKER: &str = "@MCP_VERSION@";

pub fn emit(declared: &ServiceDef) -> Vec<String> {
    let service = &declared.enveloped();
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let mut published = vec![
        fill(TRANSPORT, &prefix, &named),
        fill(SUPPORT, &prefix, &named),
    ];
    if service.operations.iter().any(is_content_list) {
        published.push(fill(BLOCKS, &prefix, &named));
    }
    if service.operations.iter().any(is_content_pair) {
        published.push(fill(PAIR, &prefix, &named));
    }
    for operation in &service.operations {
        published.extend(decoders(operation, &named));
    }
    published.push(actor(service, &prefix, &named));
    published
}

/// `kotlin_mcp_client()`'s twin: the text above with the wrapping of each message and the crate's
/// version, which only the compiled service can give, written over their markers when it runs in
/// the consuming program.
pub fn accessor(service: &ServiceDef) -> TokenStream {
    let text = emit(service).join("\n\n");
    let wrapped = service
        .enveloped()
        .operations
        .iter()
        .enumerate()
        .map(|(at, operation)| {
            let marker = wrapped_marker(at);
            let wrapped = wrapped_ident(operation);
            quote! { text = text.replace(#marker, &::std::string::ToString::to_string(&Self::#wrapped())); }
        })
        .collect::<Vec<_>>();
    quote! {
        #[doc = " The service's generated Swift `mcp` client: the one-method transport and the"]
        #[doc = " actor that numbers its requests and answers `swift_http_client()`'s own results."]
        pub fn swift_mcp_client() -> ::std::string::String {
            let mut text = #text.replace(#VERSION_MARKER, env!("CARGO_PKG_VERSION"));
            #(#wrapped)*
            text
        }
    }
}

fn wrapped_marker(at: usize) -> String {
    format!("@MCP_WRAPPED_{at}@")
}

fn fill(template: &str, prefix: &str, named: &str) -> String {
    template.replace("@P@", prefix).replace("@N@", named)
}

fn pascal(operation: &OperationDef) -> String {
    RenameRule::PascalCase.apply_to_field(&written(&operation.ident))
}

fn failure_name(named: &str, operation: &OperationDef) -> Option<String> {
    match &operation.outcome {
        OperationOutcome::OneWay => None,
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => Some(format!("{named}{}Failure", pascal(operation))),
    }
}

/// The structure that reads a value and the headers written beside it, all under one object:
/// `value`, then one member per header, an absent optional one read as `nil`.
fn header_struct(
    type_name: &str,
    body_ty: &str,
    names: &[String],
    elements: &[&Type],
    ident_prefix: &str,
) -> String {
    let mut properties = String::new();
    let mut keys = String::new();
    let mut reads = String::new();
    for (index, (name, ty)) in names.iter().zip(elements.iter().skip(1)).enumerate() {
        let ident = format!("{ident_prefix}{index}");
        let full = swift_typename_of(ty);
        let _ = writeln!(properties, "  let {ident}: {full}");
        let _ = writeln!(keys, "    case {ident} = {}", swift_quoted(name));
        if let Some(inner) = option_inner(ty) {
            let _ = writeln!(
                reads,
                "    {ident} = try container.decodeIfPresent({}.self, forKey: .{ident})",
                swift_typename_of(inner)
            );
        } else {
            let _ = writeln!(
                reads,
                "    {ident} = try container.decode({full}.self, forKey: .{ident})"
            );
        }
    }
    format!(
        "struct {type_name}: Decodable {{\n  let value: {body_ty}\n{properties}\n  \
         private enum Keys: String, CodingKey {{\n    case value\n{keys}  }}\n\n  \
         init(from decoder: Decoder) throws {{\n    \
         let container = try decoder.container(keyedBy: Keys.self)\n    \
         value = try container.decode({body_ty}.self, forKey: .value)\n{reads}  }}\n}}"
    )
}

/// The structures that read the headers of one operation's success and of its declared error.
fn decoders(operation: &OperationDef, named: &str) -> Vec<String> {
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return Vec::new();
    };
    let shape = HttpShape::of(operation);
    let mut found = Vec::new();
    if !shape.header_out.is_empty() {
        let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
        let body = elements
            .first()
            .map_or_else(|| swift_typename_of(success), |ty| swift_typename_of(ty));
        found.push(header_struct(
            &format!("{named}Mcp{}Success", pascal(operation)),
            &body,
            &shape.header_out,
            &elements,
            "headerOut",
        ));
    }
    if !shape.error_header_out.is_empty() {
        let elements: Vec<&Type> = tuple_elements(error).into_iter().flatten().collect();
        let head = elements
            .first()
            .map_or_else(|| swift_typename_of(error), |ty| swift_typename_of(ty));
        found.push(header_struct(
            &format!("{named}Mcp{}Declared", pascal(operation)),
            &head,
            &shape.error_header_out,
            &elements,
            "errorHeaderOut",
        ));
    }
    found
}

fn actor(service: &ServiceDef, prefix: &str, named: &str) -> String {
    let note = left_to_http_rest(service, "method", on_one_line)
        .map_or_else(String::new, |said| format!("/// {said}\n"));
    let forms = forms::helpers(&sent_forms(service));
    let texts = if forms.is_empty() {
        String::new()
    } else {
        format!("  {}\n\n", forms.trim_end())
    };
    let methods = service
        .operations
        .iter()
        .enumerate()
        .map(|(at, operation)| method(operation, prefix, named, at))
        .collect::<Vec<_>>()
        .join("\n\n");
    let head = fill(ACTOR_HEAD, prefix, named);
    format!("/// A `{named}` caller over `mcp`.\n{note}{head}{texts}{methods}\n}}")
}

fn method(operation: &OperationDef, prefix: &str, named: &str, at: usize) -> String {
    let shape = HttpShape::of(operation);
    let tool = swift_quoted(operation.mcp_tool_name());
    let call = swift_call(operation);
    let params = header_in_params(&message_swift_typename(operation), &shape);
    let payload = sent_payload(operation);
    let wrapped = wrapped_marker(at);
    let doc = format!(
        "  /// Calls `{}` over `mcp`.",
        on_one_line(operation.mcp_tool_name())
    );
    let mut statements = vec![format!(
        "{} headers: [String: {named}McpValue] = [:]",
        if shape.header_in.is_empty() {
            "let"
        } else {
            "var"
        }
    )];
    for header in &shape.header_in {
        let parameter = swift_parameter(&header.parameter);
        let name = swift_quoted(&header.name);
        if let Some(inner) = option_inner(&header.ty) {
            let sent =
                swift_sent_value(inner, "sentHeader").unwrap_or_else(|| "sentHeader".to_owned());
            statements.push(format!(
                "if let sentHeader = {parameter} {{ headers[{name}] = {named}McpValue({sent}) }}"
            ));
        } else {
            let sent = swift_sent_value(&header.ty, &parameter).unwrap_or(parameter);
            statements.push(format!("headers[{name}] = {named}McpValue({sent})"));
        }
    }
    statements.push(format!(
        "arguments = {named}McpArguments(wrapped: {wrapped}, message: {named}McpValue({payload}), headers: headers)"
    ));
    // A `catch` that nothing in its `do` can reach is a warning, so a call that writes only
    // values that cannot refuse has none.
    let can_refuse = statements
        .iter()
        .any(|statement| statement.contains("try "));
    let indent = if can_refuse { "      " } else { "    " };
    let mut lines = String::new();
    for statement in &statements {
        let _ = writeln!(lines, "{indent}{statement}");
    }
    let guarded = if can_refuse {
        format!("    do {{\n{lines}    }} catch {{\n      REFUSED\n    }}\n")
    } else {
        lines
    };
    let template = format!(
        "    let arguments: {named}McpArguments\n{guarded}    \
         let raw: Data\n    switch await exchange({tool}, arguments) {{\n    \
         case .answered(let answered): raw = answered\n    \
         case .failed(let fault): FAULTED\n    }}\n"
    );
    match &operation.outcome {
        OperationOutcome::OneWay => {
            let one_way_body = template
                .replace(
                    "REFUSED",
                    &format!("throw {named}Refusal(fault: {prefix}McpFault(.failedValidation, {tool}, \"\\(error)\"))"),
                )
                .replace("FAULTED", &format!("throw {named}Refusal(fault: fault)"));
            format!(
                "{doc}\n  public func {call}({params}) async throws {{\n{one_way_body}    \
                 switch {prefix}McpClassify({tool}, raw) {{\n    \
                 case .faulted(let fault): throw {named}Refusal(fault: fault)\n    \
                 case .declared: throw {named}Refusal(fault: {prefix}McpFault(.undeserializablePayload, {tool}, \"a one-way operation answered an error\"))\n    \
                 case .succeeded: return\n    }}\n  }}"
            )
        }
        OperationOutcome::Reply { error, success } => {
            let failure = failure_name(named, operation).unwrap_or_default();
            let reply_body = template
                .replace(
                    "REFUSED",
                    &format!("return .failure(.fault({prefix}McpFault(.failedValidation, {tool}, \"\\(error)\")))"),
                )
                .replace("FAULTED", "return .failure(.fault(fault))");
            let (success_ty, success_block) =
                success_block(operation, &shape, prefix, named, &tool, success);
            let declared_block = declared_block(operation, &shape, prefix, named, &tool, error);
            format!(
                "{doc}\n  public func {call}({params}) async -> Result<{success_ty}, {failure}> {{\n{reply_body}    \
                 switch {prefix}McpClassify({tool}, raw) {{\n    \
                 case .faulted(let fault): return .failure(.fault(fault))\n    \
                 case .declared(let carried):\n{declared_block}    \
                 case .succeeded:\n{success_block}    }}\n  }}"
            )
        }
    }
}

fn unreadable(prefix: &str, tool: &str) -> String {
    format!(
        "return .failure(.fault({prefix}McpFault(.undeserializablePayload, {tool}, \"\\(error)\")))"
    )
}

fn success_block(
    operation: &OperationDef,
    shape: &HttpShape,
    prefix: &str,
    named: &str,
    tool: &str,
    success: &Type,
) -> (String, String) {
    let fail = unreadable(prefix, tool);
    if is_content_list(operation) {
        let ty = swift_typename_of(success);
        let block = format!(
            "      do {{\n        \
             let decoded = try {prefix}McpDatedDecoder().decode({named}McpBlocks<{ty}>.self, from: raw)\n        \
             return .success(decoded.result.content)\n      \
             }} catch {{\n        {fail}\n      }}\n"
        );
        return (ty, block);
    }
    if is_content_pair(operation) {
        let ty = swift_typename_of(success);
        let block = format!(
            "      do {{\n        \
             let decoded = try {prefix}McpDatedDecoder().decode({named}McpPair.self, from: raw)\n        \
             if let structured = decoded.result.structuredContent {{\n          \
             return .success((structured.bytes, structured.contentType))\n        }}\n        \
             guard let block = decoded.result.content?.first, let data = block.data, let mimeType = block.mimeType else {{\n          \
             throw DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: \"the answer holds no image or audio block\"))\n        }}\n        \
             return .success(([UInt8](data), mimeType))\n      \
             }} catch {{\n        {fail}\n      }}\n"
        );
        return (ty, block);
    }
    if !shape.header_out.is_empty() {
        let ty = swift_typename_of(success);
        let type_name = format!("{named}Mcp{}Success", pascal(operation));
        let count = shape.header_out.len();
        let mut parts = vec!["decoded.result.structuredContent.value".to_owned()];
        parts.extend(
            (0..count).map(|index| format!("decoded.result.structuredContent.headerOut{index}")),
        );
        let block = format!(
            "      do {{\n        \
             let decoded = try {prefix}McpDatedDecoder().decode({named}McpResult<{type_name}>.self, from: raw)\n        \
             return .success(({}))\n      \
             }} catch {{\n        {fail}\n      }}\n",
            parts.join(", ")
        );
        return (ty, block);
    }
    if is_unit_type(success) {
        return ("Void".to_owned(), "      return .success(())\n".to_owned());
    }
    let ty = swift_typename_of(success);
    let block = format!(
        "      do {{\n        \
         let decoded = try {prefix}McpDatedDecoder().decode({named}McpResult<{ty}>.self, from: raw)\n        \
         return .success(decoded.result.structuredContent)\n      \
         }} catch {{\n        {fail}\n      }}\n"
    );
    (ty, block)
}

fn declared_block(
    operation: &OperationDef,
    shape: &HttpShape,
    prefix: &str,
    named: &str,
    tool: &str,
    error: &Type,
) -> String {
    let fail = unreadable(prefix, tool);
    if !shape.error_header_out.is_empty() {
        let type_name = format!("{named}Mcp{}Declared", pascal(operation));
        let count = shape.error_header_out.len();
        let mut parts = vec!["decoded.error.value".to_owned()];
        parts.extend((0..count).map(|index| format!("decoded.error.errorHeaderOut{index}")));
        return format!(
            "      do {{\n        \
             let decoded = try {prefix}McpDatedDecoder().decode({named}McpDeclared<{type_name}>.self, from: carried)\n        \
             return .failure(.declared(({})))\n      \
             }} catch {{\n        {fail}\n      }}\n",
            parts.join(", ")
        );
    }
    let ty = swift_typename_of(error);
    format!(
        "      do {{\n        \
         let decoded = try {prefix}McpDatedDecoder().decode({named}McpDeclared<{ty}>.self, from: carried)\n        \
         return .failure(.declared(decoded.error))\n      \
         }} catch {{\n        {fail}\n      }}\n"
    )
}
