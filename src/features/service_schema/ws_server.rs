//! The TypeScript `ws_rpc` server for a process that accepts connections: a set of open
//! connections, a context per connection, an idle-armed heartbeat, a cancellation signal, and a
//! hook that hands one socket to a second service. Wraps the single-socket attachment rather than
//! re-emitting its frame rules.

use crate::service_schema::parse::ServiceDef;

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    vec![
        server_socket_type(&named),
        connection_type(&named),
        server_options_type(&named),
        create_server_fn(&named),
    ]
}

/// The seam `accept` takes: the client seam plus the `error` listener neither a platform
/// `WebSocket` nor the client-shaped seam declares, and Node's `EventEmitter` rule makes mandatory
/// on a server (an unhandled `error` throws out of `emit`).
fn server_socket_type(named: &str) -> String {
    format!(
        "/** The seam a server accepts: the client seam plus an `error` listener, which both a \
         platform WebSocket and the `ws` package expose. */\n\
         export type {named}WsServerSocket = {named}WsSocket & {{\n  \
         addEventListener(type: \"error\", listener: (event: {{ error?: unknown }}) => void): \
         void;\n  \
         removeEventListener(type: \"error\", listener: (event: {{ error?: unknown }}) => void): \
         void;\n\
         }};"
    )
}

/// One accepted connection: the socket, the signal `accept` aborts on close, a way to close it,
/// and the hook a second service shares it through.
fn connection_type(named: &str) -> String {
    format!(
        "/** One accepted connection: the socket, a signal aborted when it closes, a way to close \
         it, and a way to hand the same socket to another service. */\n\
         export type {named}WsConnection = {{\n  \
         socket: {named}WsSocket;\n  \
         signal: AbortSignal;\n  \
         close(): void;\n  \
         /** Hands the guarded socket and this connection's abort signal to another service's \
         attachment. The detach it returns also runs when this connection closes. Throws once the \
         connection is closed. */\n  \
         share(attach: (socket: {named}WsSocket, signal: AbortSignal) => () => void): () => \
         void;\n\
         }};"
    )
}

/// `heartbeat` defaults to the client's own 30 s / 10 s pair and `false` turns it off;
/// `onFault` is required, exactly as the single-socket attachment requires it; `onSocketError`
/// is the one hook a socket error reaches, once the connection carrying it is already closed.
fn server_options_type(named: &str) -> String {
    format!(
        "export type {named}WsServerOptions = {{\n  \
         /** Arms only after `intervalMs` with no inbound frame of any kind; closes after \
         `timeoutMs` without a pong. `false` turns it off. */\n  \
         heartbeat?: {{ intervalMs: number; timeoutMs: number }} | false;\n  \
         onFault: (fault: {named}Fault, connection: {named}WsConnection) => void;\n  \
         /** A socket error: the connection is closed first; without this, the error is dropped \
         after the close. */\n  \
         onSocketError?: (error: unknown, connection: {named}WsConnection) => void;\n\
         }};"
    )
}

fn create_server_fn(named: &str) -> String {
    format!(
        "{header}{accept}{returned}}}",
        header = server_header_stmt(named),
        accept = accept_fn_stmt(named),
        returned = returned_server_stmt(),
    )
}

/// The factory's signature, its return type, and the heartbeat default and connection set every
/// accepted socket shares.
fn server_header_stmt(named: &str) -> String {
    format!(
        "export function create{named}WsServer<Ctx>(\n  \
         impl: {named}Impl<Ctx>,\n  \
         contextFor: (connection: {named}WsConnection) => Ctx,\n  \
         options: {named}WsServerOptions,\n\
         ): {{\n  \
         accept(socket: {named}WsServerSocket): {named}WsConnection;\n  \
         connections(): ReadonlyArray<{named}WsConnection>;\n  \
         closeAll(): void;\n\
         }} {{\n  \
         const heartbeat = options.heartbeat === undefined ? {{ intervalMs: 30_000, timeoutMs: \
         10_000 }} : options.heartbeat;\n  \
         const connections = new Set<{named}WsConnection>();\n  \
         return {{\n"
    )
}

fn accept_fn_stmt(named: &str) -> String {
    format!(
        "{opening}{heartbeat}{guarded}{connection}{on_frame}{on_error}{on_close}{wiring}",
        opening = accept_opening_stmt(),
        heartbeat = accept_heartbeat_stmt(),
        guarded = guarded_socket_stmt(named),
        connection = connection_stmt(named),
        on_frame = on_frame_stmt(),
        on_error = on_error_stmt(),
        on_close = on_close_stmt(),
        wiring = wiring_and_detach_stmt(named),
    )
}

fn accept_opening_stmt() -> String {
    "    accept(socket) {\n      \
     const controller = new AbortController();\n      \
     let idle: ReturnType<typeof setTimeout> | undefined;\n      \
     let pongDeadline: ReturnType<typeof setTimeout> | undefined;\n"
        .to_owned()
}

/// The idle timer this one connection arms itself: `stopHeartbeat` clears both timers,
/// `armIdle` (re)arms the idle timer unless `heartbeat` is `false`, and its own firing is what
/// sends the probe and starts the deadline that closes the socket.
fn accept_heartbeat_stmt() -> String {
    "      const stopHeartbeat = () => {\n        \
     if (idle !== undefined) clearTimeout(idle);\n        \
     if (pongDeadline !== undefined) clearTimeout(pongDeadline);\n        \
     idle = pongDeadline = undefined;\n      \
     };\n      \
     const armIdle = () => {\n        \
     if (heartbeat === false) return;\n        \
     if (idle !== undefined) clearTimeout(idle);\n        \
     idle = setTimeout(() => {\n          \
     socket.send(JSON.stringify({ kind: \"ping\" }));\n          \
     pongDeadline = setTimeout(() => socket.close(), heartbeat.timeoutMs);\n        \
     }, heartbeat.intervalMs);\n      \
     };\n"
        .to_owned()
}

fn guarded_socket_stmt(named: &str) -> String {
    format!(
        "      // The socket the attachment writes through: a write after close is dropped rather \
         than handed to the library.\n      \
         const guarded: {named}WsSocket = {{\n        \
         send: (text) => {{ if (!controller.signal.aborted) socket.send(text); }},\n        \
         close: () => socket.close(),\n        \
         addEventListener: (type: \"message\" | \"close\", listener: never) => \
         socket.addEventListener(type as \"message\", listener),\n        \
         removeEventListener: (type: \"message\" | \"close\", listener: never) => \
         socket.removeEventListener(type as \"message\", listener),\n      \
         }} as {named}WsSocket;\n"
    )
}

/// The connection `accept` returns: the raw socket, the signal `onClose` aborts, `close`, and
/// `share` — which refuses once the signal is aborted and otherwise hands the guarded socket and
/// signal to another attachment, keeping its detach for `onClose` to run.
fn connection_stmt(named: &str) -> String {
    format!(
        "      const shared: Array<() => void> = [];\n      \
         const connection: {named}WsConnection = {{\n        \
         socket,\n        \
         signal: controller.signal,\n        \
         close: () => socket.close(),\n        \
         share: (attach) => {{\n          \
         if (controller.signal.aborted) throw new Error(\"the connection is closed\");\n          \
         const detachShared = attach(guarded, controller.signal);\n          \
         shared.push(detachShared);\n          \
         return detachShared;\n        \
         }},\n      \
         }};\n"
    )
}

/// The server's own `\"message\"` listener: it never reads a call frame itself — the attachment
/// does that — but every inbound frame re-arms the idle timer, and a `ping` draws the `pong` the
/// attachment does not answer.
fn on_frame_stmt() -> String {
    "      const onFrame = (event: { data: unknown }) => {\n        \
     if (pongDeadline !== undefined) { clearTimeout(pongDeadline); pongDeadline = undefined; }\n        \
     armIdle();\n        \
     let frame: unknown;\n        \
     try { frame = JSON.parse(String(event.data)); } catch { return; }\n        \
     if (typeof frame === \"object\" && frame !== null && (frame as { kind?: unknown }).kind === \
     \"ping\") {\n          \
     socket.send(JSON.stringify({ kind: \"pong\" }));\n        \
     }\n      \
     };\n"
        .to_owned()
}

fn on_error_stmt() -> String {
    "      const onError = (event: { error?: unknown }) => {\n        \
     options.onSocketError?.(event.error, connection);\n        \
     socket.close();\n      \
     };\n"
        .to_owned()
}

/// Tears everything about this one connection down: the timers, the server's own three listeners,
/// every attachment `share` handed the socket to, the wrapped attachment itself, the held
/// connection, and the signal every guarded write and every `share` call reads.
fn on_close_stmt() -> String {
    "      const onClose = () => {\n        \
     stopHeartbeat();\n        \
     socket.removeEventListener(\"message\", onFrame);\n        \
     socket.removeEventListener(\"close\", onClose);\n        \
     socket.removeEventListener(\"error\", onError);\n        \
     for (const detachShared of shared.splice(0)) detachShared();\n        \
     detach();\n        \
     connections.delete(connection);\n        \
     controller.abort();\n      \
     };\n"
        .to_owned()
}

/// Starts the three listeners, builds the context synchronously and attaches the existing
/// single-socket dispatcher through the guarded socket, registers and arms the connection, and
/// returns it.
fn wiring_and_detach_stmt(named: &str) -> String {
    format!(
        "      socket.addEventListener(\"message\", onFrame);\n      \
         socket.addEventListener(\"close\", onClose);\n      \
         socket.addEventListener(\"error\", onError);\n      \
         const detach = attach{named}WsDispatcher(guarded, contextFor(connection), impl, (fault) \
         => options.onFault(fault, connection));\n      \
         connections.add(connection);\n      \
         armIdle();\n      \
         return connection;\n    \
         }},\n"
    )
}

/// `connections()` answers a fresh array so a caller cannot mutate the held set; `closeAll()`
/// closes every held connection through the seam's own `close()`.
fn returned_server_stmt() -> String {
    "    connections: () => [...connections],\n    \
     closeAll: () => { for (const connection of connections) connection.close(); },\n  \
     };\n"
        .to_owned()
}
