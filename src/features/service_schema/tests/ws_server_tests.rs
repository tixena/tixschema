//! The connection-accepting `ws_rpc` server, read off the emitted text.

use super::{SINGLE_PLACEHOLDER_HTTP_SERVICE, ws_server_of};

const EXPECTED: &str = r#"/** The seam a server accepts: the client seam plus an `error` listener, which both a platform WebSocket and the `ws` package expose. */
export type ConversationClientServiceWsServerSocket = ConversationClientServiceWsSocket & {
  addEventListener(type: "error", listener: (event: { error?: unknown }) => void): void;
  removeEventListener(type: "error", listener: (event: { error?: unknown }) => void): void;
};

/** One accepted connection: the socket, a signal aborted when it closes, a way to close it, and a way to hand the same socket to another service. */
export type ConversationClientServiceWsConnection = {
  socket: ConversationClientServiceWsSocket;
  signal: AbortSignal;
  close(): void;
  /** Hands the guarded socket and this connection's abort signal to another service's attachment. The detach it returns also runs when this connection closes. Throws once the connection is closed. */
  share(attach: (socket: ConversationClientServiceWsSocket, signal: AbortSignal) => () => void): () => void;
};

export type ConversationClientServiceWsServerOptions = {
  /** Arms only after `intervalMs` with no inbound frame of any kind; closes after `timeoutMs` without a pong. `false` turns it off. */
  heartbeat?: { intervalMs: number; timeoutMs: number } | false;
  onFault: (fault: ConversationClientServiceFault, connection: ConversationClientServiceWsConnection) => void;
  /** A socket error: the connection is closed first; without this, the error is dropped after the close. */
  onSocketError?: (error: unknown, connection: ConversationClientServiceWsConnection) => void;
};

export function createConversationClientServiceWsServer<Ctx>(
  impl: ConversationClientServiceImpl<Ctx>,
  contextFor: (connection: ConversationClientServiceWsConnection) => Ctx,
  options: ConversationClientServiceWsServerOptions,
): {
  accept(socket: ConversationClientServiceWsServerSocket): ConversationClientServiceWsConnection;
  connections(): ReadonlyArray<ConversationClientServiceWsConnection>;
  closeAll(): void;
} {
  const heartbeat = options.heartbeat === undefined ? { intervalMs: 30_000, timeoutMs: 10_000 } : options.heartbeat;
  const connections = new Set<ConversationClientServiceWsConnection>();
  return {
    accept(socket) {
      const controller = new AbortController();
      let idle: ReturnType<typeof setTimeout> | undefined;
      let pongDeadline: ReturnType<typeof setTimeout> | undefined;
      const stopHeartbeat = () => {
        if (idle !== undefined) clearTimeout(idle);
        if (pongDeadline !== undefined) clearTimeout(pongDeadline);
        idle = pongDeadline = undefined;
      };
      const armIdle = () => {
        if (heartbeat === false) return;
        if (idle !== undefined) clearTimeout(idle);
        idle = setTimeout(() => {
          socket.send(JSON.stringify({ kind: "ping" }));
          pongDeadline = setTimeout(() => socket.close(), heartbeat.timeoutMs);
        }, heartbeat.intervalMs);
      };
      // The socket the attachment writes through: a write after close is dropped rather than handed to the library.
      const guarded: ConversationClientServiceWsSocket = {
        send: (text) => { if (!controller.signal.aborted) socket.send(text); },
        close: () => socket.close(),
        addEventListener: (type: "message" | "close", listener: never) => socket.addEventListener(type as "message", listener),
        removeEventListener: (type: "message" | "close", listener: never) => socket.removeEventListener(type as "message", listener),
      } as ConversationClientServiceWsSocket;
      const shared: Array<() => void> = [];
      const connection: ConversationClientServiceWsConnection = {
        socket,
        signal: controller.signal,
        close: () => socket.close(),
        share: (attach) => {
          if (controller.signal.aborted) throw new Error("the connection is closed");
          const detachShared = attach(guarded, controller.signal);
          shared.push(detachShared);
          return detachShared;
        },
      };
      const onFrame = (event: { data: unknown }) => {
        if (pongDeadline !== undefined) { clearTimeout(pongDeadline); pongDeadline = undefined; }
        armIdle();
        let frame: unknown;
        try { frame = JSON.parse(String(event.data)); } catch { return; }
        if (typeof frame === "object" && frame !== null && (frame as { kind?: unknown }).kind === "ping") {
          socket.send(JSON.stringify({ kind: "pong" }));
        }
      };
      const onError = (event: { error?: unknown }) => {
        options.onSocketError?.(event.error, connection);
        socket.close();
      };
      const onClose = () => {
        stopHeartbeat();
        socket.removeEventListener("message", onFrame);
        socket.removeEventListener("close", onClose);
        socket.removeEventListener("error", onError);
        for (const detachShared of shared.splice(0)) detachShared();
        detach();
        connections.delete(connection);
        controller.abort();
      };
      socket.addEventListener("message", onFrame);
      socket.addEventListener("close", onClose);
      socket.addEventListener("error", onError);
      const detach = attachConversationClientServiceWsDispatcher(guarded, contextFor(connection), impl, (fault) => options.onFault(fault, connection));
      connections.add(connection);
      armIdle();
      return connection;
    },
    connections: () => [...connections],
    closeAll: () => { for (const connection of connections) connection.close(); },
  };
}"#;

#[test]
fn ts_ws_server_reproduces_the_design_s_section_3_text_byte_for_byte() {
    let written = ws_server_of(SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert_eq!(written, EXPECTED, "got: {written}");
}

#[test]
fn context_for_is_called_before_the_attachment_is_made() {
    let written = ws_server_of(SINGLE_PLACEHOLDER_HTTP_SERVICE);
    let attach_at =
        written.find("attachConversationClientServiceWsDispatcher(guarded, contextFor(connection)");
    let registered_at = written.find("connections.add(connection);");
    assert!(
        attach_at.is_some() && registered_at.is_some(),
        "the attachment call names contextFor(connection) as its context argument, and the \
         connection is registered once the attachment is in place. Got: {written}"
    );
    let attach = attach_at.unwrap();
    let registered = registered_at.unwrap();
    assert!(
        attach < registered,
        "contextFor runs synchronously inside the attachment call, so no frame can arrive on an \
         unregistered connection before a context exists for it. Got: {written}"
    );
}

#[test]
fn a_guarded_write_is_dropped_once_the_signal_is_aborted() {
    let written = ws_server_of(SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert!(
        written.contains("send: (text) => { if (!controller.signal.aborted) socket.send(text); },"),
        "the attachment writes through this socket, never the raw one, so a late reply is dropped \
         rather than handed to the library. Got: {written}"
    );
}

#[test]
fn the_heartbeat_arms_only_after_idle_and_defaults_to_thirty_and_ten_seconds() {
    let written = ws_server_of(SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert!(
        written.contains(
            "const heartbeat = options.heartbeat === undefined ? { intervalMs: 30_000, \
             timeoutMs: 10_000 } : options.heartbeat;"
        ),
        "got: {written}"
    );
    assert!(
        written.contains("if (heartbeat === false) return;"),
        "false turns the probe off. Got: {written}"
    );
    assert!(
        written.contains("if (pongDeadline !== undefined) { clearTimeout(pongDeadline); pongDeadline = undefined; }\n        armIdle();"),
        "every inbound frame cancels a pending pong deadline and re-arms the idle timer, so a busy \
         connection is never probed. Got: {written}"
    );
}

#[test]
fn every_inbound_ping_is_answered_with_a_pong() {
    let written = ws_server_of(SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert!(
        written.contains(
            "if (typeof frame === \"object\" && frame !== null && (frame as { kind?: unknown \
             }).kind === \"ping\") {\n          socket.send(JSON.stringify({ kind: \"pong\" }));"
        ),
        "got: {written}"
    );
}

#[test]
fn the_accepted_socket_requires_an_error_listener_and_the_server_attaches_one() {
    let written = ws_server_of(SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert!(
        written.contains(
            "export type ConversationClientServiceWsServerSocket = \
             ConversationClientServiceWsSocket & {\n  addEventListener(type: \"error\", \
             listener: (event: { error?: unknown }) => void): void;"
        ),
        "under `ws`, an unhandled `error` event throws out of `emit`, so the accepted socket must \
         carry the listener. Got: {written}"
    );
    assert!(
        written.contains("socket.addEventListener(\"error\", onError);")
            && written.contains(
                "const onError = (event: { error?: unknown }) => {\n        \
                 options.onSocketError?.(event.error, connection);\n        socket.close();"
            ),
        "the server attaches its own error listener, reports through onSocketError, and closes. \
         Got: {written}"
    );
}

#[test]
fn share_hands_the_guarded_socket_and_signal_and_throws_once_closed() {
    let written = ws_server_of(SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert!(
        written.contains(
            "share: (attach) => {\n          if (controller.signal.aborted) throw new \
             Error(\"the connection is closed\");\n          const detachShared = \
             attach(guarded, controller.signal);"
        ),
        "share hands the same guarded socket the owning attachment writes through, never the raw \
         one, and refuses once the connection is closed. Got: {written}"
    );
    assert!(
        written.contains("for (const detachShared of shared.splice(0)) detachShared();"),
        "a shared attachment's detach runs on close, before the owner's own detach. \
         Got: {written}"
    );
}

#[test]
fn close_all_closes_every_held_connection_through_the_seam_s_own_close() {
    let written = ws_server_of(SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert!(
        written.contains(
            "closeAll: () => { for (const connection of connections) connection.close(); },"
        ),
        "got: {written}"
    );
    assert!(
        written.contains("connections: () => [...connections],"),
        "a fresh array each call, so a caller cannot mutate the held set. Got: {written}"
    );
}
