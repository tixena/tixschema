//! The transports a service can ask for, and the one place a name is bound to one.

mod amqp_rpc;
mod http_rest;
mod ws_rpc;

use super::parse::ServiceDef;
use crate::rename_rule::RenameRule;
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::spanned::Spanned as _;
use syn::{Expr, ExprLit, Ident, Lit, meta::parser, parse::Parser as _};

const TRANSPORTS_ARGUMENT: &str = "transports";
const NON_EXHAUSTIVE_ARGUMENT: &str = "non_exhaustive";

const UNKNOWN_ARGUMENT_MESSAGE: &str = concat!(
    "service_schema: unknown `service_schema` argument\n",
    "       the arguments are `transports`, written `transports = [\"amqp_rpc\"]`,\n",
    "       and `non_exhaustive`, written bare"
);

const WRITTEN_SHAPE_MESSAGE: &str = concat!(
    "service_schema: `transports` takes a bracketed list of transport names\n",
    "       write `transports = [\"amqp_rpc\"]`, or `transports = []` for none"
);

const NON_EXHAUSTIVE_SHAPE_MESSAGE: &str = concat!(
    "service_schema: `non_exhaustive` is a bare flag and takes no value\n",
    "       write `non_exhaustive`, and leave it out for the exhaustive generated types"
);

/// Everything `#[service_schema(...)]`'s own arguments say, read once by [`parse_arguments`].
#[derive(Debug, Default, Eq, PartialEq)]
pub struct ServiceArguments {
    /// Whether the generated types carry `#[non_exhaustive]`.
    pub non_exhaustive: bool,
    /// The transports asked for, in the order written. Nothing sorts or dedupes it.
    pub transports: Vec<Transport>,
}

/// One transport a service asks for by name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transport {
    AmqpRpc,
    HttpRest,
    WsRpc,
}

impl Transport {
    /// Every transport this version knows, in the order a refusal lists them.
    pub const KNOWN: &'static [Self] = &[Self::AmqpRpc, Self::HttpRest, Self::WsRpc];

    fn from_name(written: &str) -> Option<Self> {
        Self::KNOWN
            .iter()
            .copied()
            .find(|known| known.name() == written)
    }

    /// The name a service writes for it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::AmqpRpc => "amqp_rpc",
            Self::HttpRest => "http_rest",
            Self::WsRpc => "ws_rpc",
        }
    }
}

/// The name a transport's client macro publishes under: `{service}_{transport}_client`.
pub fn client_macro_ident(service: &ServiceDef, transport: Transport) -> Ident {
    macro_ident(service, transport, "client")
}

/// The name a transport's dispatcher macro publishes under: `{service}_{transport}_dispatcher`.
pub fn dispatcher_macro_ident(service: &ServiceDef, transport: Transport) -> Ident {
    macro_ident(service, transport, "dispatcher")
}

/// The name a transport's server macro publishes under: `{service}_{transport}_server`.
pub fn server_macro_ident(service: &ServiceDef, transport: Transport) -> Ident {
    macro_ident(service, transport, "server")
}

/// Either half's macro name, spelled in one place so the two cannot drift apart.
///
/// `#[macro_export]` places each at the declaring crate's root whatever module it was written in,
/// so the service name is what keeps two services in one crate from claiming one name.
fn macro_ident(service: &ServiceDef, transport: Transport, half: &str) -> Ident {
    format_ident!(
        "{}_{}_{}",
        RenameRule::SnakeCase.apply_to_variant(&service.ident.to_string()),
        transport.name(),
        half,
        span = service.ident.span()
    )
}

/// What every transport the service asked for contributes, in the registry's own order.
///
/// Each contributes at most once however many times it was named: what it publishes is
/// `#[macro_export]`ed, and one name at a crate root can be defined once.
pub fn emit(service: &ServiceDef, asked: &[Transport]) -> TokenStream {
    Transport::KNOWN
        .iter()
        .filter(|known| asked.contains(known))
        .map(|known| match *known {
            Transport::AmqpRpc => amqp_rpc::emit(service, *known),
            Transport::HttpRest => http_rest::emit(service, *known),
            Transport::WsRpc => ws_rpc::emit(service, *known),
        })
        .collect()
}

/// Reads `#[service_schema(...)]`'s own arguments into a [`ServiceArguments`], in the order they
/// were written. Argument order is free — `non_exhaustive, transports = [...]` and the reverse
/// read the same.
///
/// A bare `#[service_schema]` and an empty `#[service_schema()]` ask for nothing, and so does
/// `transports = []` — the same list, said out loud. Anything else the attribute carries is
/// refused rather than dropped, so a service cannot ask for a transport this version does not have
/// and be handed silence.
///
/// # An unknown name is refused, under the name itself
///
/// The service below asks for a transport that does not exist:
///
/// ```rust,compile_fail
/// use tixschema::service_schema;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub struct BalanceResponse;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub enum BalanceError {
///     DbError,
/// }
///
/// #[service_schema(transports = ["grpc"])]
/// pub trait UsageService<Ctx> {
///     async fn sweep(&self, ctx: &Ctx) -> Result<BalanceResponse, BalanceError>;
/// }
///
/// fn main() {}
/// ```
///
/// A `compile_fail` doctest asserts only that *something* was refused, so the file above was
/// compiled standalone and the diagnostic read off that run, verbatim. It was the only error the
/// file earned, and the caret sits under the name rather than under the attribute:
///
/// ```text
/// error: service_schema: `grpc` is not a transport this version knows
///               known transports: `amqp_rpc`, `http_rest`, `ws_rpc`
///   --> tests/zz_probe.rs:11:32
///    |
/// 11 | #[service_schema(transports = ["grpc"])]
///    |                                ^^^^^^
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// # A list written without brackets is refused, and so is an argument that is not `transports`
///
/// The same service with the brackets left off earns a sentence naming the shape that was
/// expected:
///
/// ```rust,compile_fail
/// use tixschema::service_schema;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub struct BalanceResponse;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub enum BalanceError {
///     DbError,
/// }
///
/// #[service_schema(transports = "amqp_rpc")]
/// pub trait UsageService<Ctx> {
///     async fn sweep(&self, ctx: &Ctx) -> Result<BalanceResponse, BalanceError>;
/// }
///
/// fn main() {}
/// ```
///
/// ```text
/// error: service_schema: `transports` takes a bracketed list of transport names
///               write `transports = ["amqp_rpc"]`, or `transports = []` for none
///   --> tests/zz_probe.rs:11:31
///    |
/// 11 | #[service_schema(transports = "amqp_rpc")]
///    |                               ^^^^^^^^^^
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// And the singular spelling is a name the attribute does not take, rather than a second way of
/// saying the same thing:
///
/// ```rust,compile_fail
/// use tixschema::service_schema;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub struct BalanceResponse;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub enum BalanceError {
///     DbError,
/// }
///
/// #[service_schema(transport = ["amqp_rpc"])]
/// pub trait UsageService<Ctx> {
///     async fn sweep(&self, ctx: &Ctx) -> Result<BalanceResponse, BalanceError>;
/// }
///
/// fn main() {}
/// ```
///
/// ```text
/// error: service_schema: unknown `service_schema` argument
///               the arguments are `transports`, written `transports = ["amqp_rpc"]`,
///               and `non_exhaustive`, written bare
///   --> tests/zz_probe.rs:11:18
///    |
/// 11 | #[service_schema(transport = ["amqp_rpc"])]
///    |                  ^^^^^^^^^
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// Each of the three is the service below with exactly one thing changed — the name, the
/// brackets, the argument — so the refusal can only be what was changed. This one asks for the
/// transport this version does know, and compiles:
///
/// ```rust
/// use tixschema::service_schema;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub struct BalanceResponse;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub enum BalanceError {
///     DbError,
/// }
///
/// #[service_schema(transports = ["amqp_rpc"])]
/// pub trait UsageService<Ctx> {
///     async fn sweep(&self, ctx: &Ctx) -> Result<BalanceResponse, BalanceError>;
/// }
///
/// fn main() {}
/// ```
///
/// # `non_exhaustive` is a bare flag, and takes no value
///
/// It reads as the attribute it produces, so `= true` is refused rather than read as a second way
/// of asking for the default:
///
/// ```rust,compile_fail
/// use tixschema::service_schema;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub struct BalanceResponse;
///
/// #[derive(serde::Deserialize, serde::Serialize)]
/// pub enum BalanceError {
///     DbError,
/// }
///
/// #[service_schema(non_exhaustive = true)]
/// pub trait UsageService<Ctx> {
///     async fn sweep(&self, ctx: &Ctx) -> Result<BalanceResponse, BalanceError>;
/// }
///
/// fn main() {}
/// ```
///
/// ```text
/// error: service_schema: `non_exhaustive` is a bare flag and takes no value
///               write `non_exhaustive`, and leave it out for the exhaustive generated types
///   --> tests/zz_probe.rs:11:18
///    |
/// 11 | #[service_schema(non_exhaustive = true)]
///    |                  ^^^^^^^^^^^^^^
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
pub fn parse_arguments(args: TokenStream) -> Result<ServiceArguments, syn::Error> {
    let mut read = ServiceArguments::default();
    let reader = parser(|meta| {
        if meta.path.is_ident(NON_EXHAUSTIVE_ARGUMENT) {
            if meta.input.peek(syn::Token![=]) {
                return Err(meta.error(NON_EXHAUSTIVE_SHAPE_MESSAGE));
            }
            read.non_exhaustive = true;
            return Ok(());
        }
        if !meta.path.is_ident(TRANSPORTS_ARGUMENT) {
            return Err(meta.error(UNKNOWN_ARGUMENT_MESSAGE));
        }
        let written = meta.value()?.parse::<Expr>()?;
        let Expr::Array(listed) = written else {
            return Err(syn::Error::new(written.span(), WRITTEN_SHAPE_MESSAGE));
        };
        for element in &listed.elems {
            read.transports.push(transport_written(element)?);
        }
        Ok(())
    });
    reader.parse2(args)?;
    Ok(read)
}

/// One element of the written list, which is a string naming a transport this version has.
fn transport_written(element: &Expr) -> Result<Transport, syn::Error> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(named),
        attrs: _attrs,
    }) = element
    else {
        return Err(syn::Error::new(element.span(), WRITTEN_SHAPE_MESSAGE));
    };
    let written = named.value();
    Transport::from_name(&written)
        .ok_or_else(|| syn::Error::new(named.span(), unknown_transport_message(&written)))
}

fn unknown_transport_message(written: &str) -> String {
    let known = Transport::KNOWN
        .iter()
        .map(|transport| format!("`{}`", transport.name()))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "service_schema: `{written}` is not a transport this version knows\n       \
         known transports: {known}"
    )
}

#[cfg(test)]
mod tests;
