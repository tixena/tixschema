//! The second service's dispatcher, in a module of its own: the reply handle travels with the
//! dispatcher, so serving both services means placing both.

billing_service_amqp_rpc_dispatcher!();
