#![cfg_attr(
    not(feature = "unstable-server-transport"),
    doc = " The `unstable-server-transport` feature is necessary to enable this module."
)]
// #![warn(missing_docs)]
// #![warn(clippy::missing_docs_in_private_items)]
//! Receiving requests and sending responses.
//!
//! This module provides skeleton asynchronous server implementations based on
//! the [Tokio](https://tokio.rs/) async runtime. In combination with an
//! appropriate network source, optional middleware services and your own
//! [`Service`] implementation, they can be used to run a standards compliant
//! DNS server that answers requests based on the application logic you
//! specify.
//!
//! In addtion, this module provides a less complex service interface called
//! [SingleService][`single_service::SingleService`]. This interface supports
//! only a single response per request.
//! In other words, it does not support the AXFR and IXFR requests.
//! Adaptors are available to connect SingleServer to [`Service`] and to the
//! [Client][`crate::net::client`] transports. See the
//! Section [Single Service][crate::net::server#single-service] for
//! more details.
//!
//! # Architecture
//!
//! A layered stack of components is responsible for handling incoming
//! requests and outgoing responses:
//!
//! ```text
//! --> network source                         - reads bytes from the client
//!     --> server                             - deserializes requests
//!         --> (optional) middleware services - pre-processes requests
//!             --> service                    - processes requests &
//!             <--                              generates responses
//!         <-- (optional) middleware services - post-processes responses
//!     <-- server                             - serializes responses
//! <-- network source                         - writes bytes to the client
//! ```
//!
//! # Getting started
//!
//! Servers are implemented by combining a server transport (see [dgram] and
//! [stream]), [`BufSource`] and [`Service`] together. Middleware [`Service`]
//! impls take an upstream [`Service`] instance as input during construction
//! allowing them to be layered on top of one another, with your own
//! application [`Service`] impl at the peak.
//!
//! Whether using [`DgramServer`] or [`StreamServer`] the required steps are
//! the same:
//!
//!   1. Create an appropriate network source (more on this below).
//!   2. Construct a server transport with `new()` passing in the network
//!      source and service instance as arguments.
//!      - (optional) Tune the server behaviour via builder functions such as
//!        `with_config()`.
//!   3. `run()` the server.
//!   4. `shutdown()` the server, explicitly or on [`drop`].
//!
//! See [`DgramServer`] and [`StreamServer`] for example code to help you get
//! started.
//!
//! # Core concepts
//!
//! ## Network transports
//!
//! Historically DNS servers communicated primarily via datagram based
//! connection-less network transport protocols, and used stream based
//! connection-oriented network transport protocols only for zone transfers.
//!
//! Modern DNS servers increasingly need to support stream based
//! connection-oriented network transport protocols for additional response
//! capacity and connection security. This module provides support for both
//! via the [`DgramServer`] and [`StreamServer`] types respectively.
//!
//! ## Datagram (e.g. UDP) servers
//!
//! [`DgramServer`] can communicate via any "network source" type that
//! implements the [`AsyncDgramSock`] trait, with an implementation provided
//! for [`tokio::net::UdpSocket`].
//!
//! The type alias [`UdpServer`] is provided for convenience for
//! implementations based on [`tokio::net::UdpSocket`].
//!
//! ## Stream (e.g. TCP) servers
//!
//! [`StreamServer`] can communicate via any "network source" type that
//! implements the [`AsyncAccept`] trait, and whose associated stream type
//! implements the [`tokio::io::AsyncRead`] and [`tokio::io::AsyncWrite`]
//! traits, with an implementation provided for [`tokio::net::TcpListener`]
//! and associated stream type [`tokio::net::TcpStream`].
//!
//! The type alias [`TcpServer`] is provided for convenience for
//! implementations based on [`tokio::net::TcpListener`].
//!
//! ## Middleware
//!
//! [Middleware] provides a means to add logic for request pre-processing and
//! response post-processing which doesn't belong in the outermost transport
//! specific layer of a server nor does it constitute part of the core
//! application logic.
//!
//! Mandatory functionality and logic required by all standards compliant DNS
//! servers can be incorporated into your server by layering your service on
//! top of [`MandatoryMiddlewareSvc`]. Additional layers of behaviour can be
//! optionally added from a selection of [pre-supplied middleware] or
//! middleware that you create yourself.
//!
//! ## Application logic
//!
//! With the basic work of handling DNS requests and responses taken care of,
//! the actual application logic that differentiates your DNS server from
//! other DNS servers is left for you to define by implementing the
//! [`Service`] trait yourself and passing an instance of that service to the
//! server or middleware service as input.
//!
//! ## Zone maintenance and zone transfers
//!
//! This crate provides everything you need to do zone maintenance, i.e.
//! serving entire zones to clients and keeping your own zones synchronized
//! with those of a primary server.
//!
//! If acting as a primary nameserver:
//! - Use [`XfrMiddlewareSvc`] to respond to AXFR and IXFR requests from
//!   secondary nameservers.
//! - Implement [`XfrDataProvider`] to define your XFR access policy.
//! - Create [`ZoneDiff`]s when making changes to [`Zone`] content and make
//!   those diffs available via your [`XfrDataProvider`] implementation.
//! - Use [`TsigMiddlewareSvc`] to authenticate transfer requests from
//!   secondary nameservers.
//! - Use the UDP client support in `net::client` to send out NOTIFY messages
//!   on zone change.
//!
//! If acting as a secondary nameserver:
//! - Use [`NotifyMiddlewareSvc`] to detect changes at the primary to zones
//!   that you are mirroring.
//! - Use the TCP client support in [`net::client`] to make outbound XFR
//!   requests on SOA timer expiration or NOTIFY to fetch changes to zone
//!   content.
//! - Use [`net::client::tsig`] to authenticate your transfer requests to
//!   primary nameservers.
//! - Use [`XfrResponseInterpreter`] and [`ZoneUpdater`] to parse transfer
//!   responses and apply the changes to your zones.
//!
//! Additionally you may wish to use [`ZoneTree`] to simplify serving multiple
//! zones.
//!
//! [`net::client`]: crate::net::client
//! [`net::client::tsig`]: crate::net::client::tsig
//! [`NotifyMiddlewareSvc`]: middleware::notify::NotifyMiddlewareSvc
//! [`TsigMiddlewareSvc`]: middleware::tsig::TsigMiddlewareSvc
//! [`XfrMiddlewareSvc`]: middleware::xfr::XfrMiddlewareSvc
//! [`XfrDataProvider`]: middleware::xfr::XfrDataProvider
//! [`XfrResponseInterpreter`]:
//!     crate::net::xfr::protocol::XfrResponseInterpreter
//! [`Zone`]: crate::zonetree::Zone
//! [`ZoneDiff`]: crate::zonetree::ZoneDiff
//! [`ZoneTree`]: crate::zonetree::ZoneTree
//! [`ZoneUpdater`]: crate::zonetree::update::ZoneUpdater
//!
//! # Advanced
//!
//! ## Memory allocation
//!
//! The allocation of buffers, e.g. for receiving DNS messages, is delegated
//! to an implementation of the [`BufSource`] trait, giving you some control
//! over the memory allocation strategy in use.
//!
//! ## Dynamic reconfiguration
//!
//! Servers in principle support the ability to dynamically reconfigure
//! themselves in response to [`ServerCommand::Reconfigure`] while running,
//! though the actual degree of support for this is server implementation
//! dependent.
//!
//! ## Performance
//!
//! Calls into the service layer from the servers are asynchronous and thus
//! managed by the Tokio async runtime. As with any Tokio application, long
//! running tasks should be spawned onto a separate threadpool, e.g. via
//! [`tokio::task::spawn_blocking()`] to avoid blocking the Tokio async
//! runtime.
//!
//! ## Clone, Arc, and shared state
//!
//! Both [`DgramServer`] and [`StreamServer`] take ownership of the
//! [`Service`] impl passed to them.
//!
//! For each request received a new Tokio task is spawned to parse the request
//! bytes, pass it to the first service and process the response(s).
//! [`Service`] impls are therefore required to implement the [`Clone`] trait,
//! either directly or indirectly by for example wrapping the service instance
//! in an [`Arc`], so that [`Service::call`] can be invoked inside the task
//! handling the request.
//!
//! There are various approaches you can take to manage the sharing of state
//! between server instances and processing tasks, for example:
//!
//! | # | Difficulty | Summary | Description |
//! |---|------------|---------|-------------|
//! | 1 | Easy | `#[derive(Clone)]` | Add `#[derive(Clone)]` to your [`Service`] impl. If your [`Service`] impl has no state that needs to be shared amongst instances of itself then this may be good enough for you. |
//! | 2 | Medium | [`Arc`] wrapper | Wrap your [`Service`] impl instance inside an [`Arc`] via [`Arc::new`]. This crate implements the [`Service`] trait for `Arc<Service>` so you can pass an `Arc<Service>` to both [`DgramServer`] and [`StreamServer`] and they will [`Clone`] the [`Arc`] rather than the [`Service`] instance itself. |
//! | 3 | Hard | Do it yourself | Manually implement [`Clone`] and/or your own locking and interior mutability strategy for your [`Service`] impl, giving you complete control over how state is shared by your server instances. |
//!
//! [`Arc`]: std::sync::Arc
//! [`Arc::new`]: std::sync::Arc::new()
//! [`AsyncAccept`]: sock::AsyncAccept
//! [`AsyncDgramSock`]: sock::AsyncDgramSock
//! [`BufSource`]: buf::BufSource
//! [`DgramServer`]: dgram::DgramServer
//! [Middleware]: middleware
//! [`MandatoryMiddlewareSvc`]: middleware::mandatory::MandatoryMiddlewareSvc
//! [pre-supplied middleware]: middleware
//! [`Service`]: service::Service
//! [`Service::call`]: service::Service::call()
//! [`StreamServer`]: stream::StreamServer
//! [`TcpServer`]: stream::TcpServer
//! [`UdpServer`]: dgram::UdpServer
//! [`tokio::io::AsyncRead`]:
//!     https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html
//! [`tokio::io::AsyncWrite`]:
//!     https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html
//! [`tokio::net::TcpListener`]:
//!     https://docs.rs/tokio/latest/tokio/net/struct.TcpListener.html
//! [`tokio::net::TcpStream`]:
//!     https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html
//! [`tokio::net::UdpSocket`]:
//!     https://docs.rs/tokio/latest/tokio/net/struct.UdpSocket.html
//!
//! # Single Service
//!
//! The [SingleService][single_service::SingleService] trait has a single
//! method [call()][single_service::SingleService::call()] that takes a
//! [Request][message::Request] and returns a Future that results in
//! either an error or a reply.
//! To assist building reply messages there is the trait
//! [ComposeReply][single_service::ComposeReply].
//! The [ComposeReply][single_service::ComposeReply] trait is implemented by
//! [ReplyMessage][single_service::ReplyMessage]
//!
//! To assist in connecting [SingleService][single_service::SingleService]
//! to the rest of the ecosystem, there are three adapters:
//! 1) The first adapter,
//!    [SingleServiceToService][adapter::SingleServiceToService] implements
//!    [Service][service::Service] for
//!    [SingleService][single_service::SingleService]. This allows any
//!    object that implements [SingleService][single_service::SingleService]
//!    to connect to a place where [Service][service::Service] is required.
//! 2) The second adapter,
//!    [ClientTransportToSingleService][adapter::ClientTransportToSingleService]
//!    implements  [SingleService][single_service::SingleService] for an
//!    object that implements
//!    [SendRequest][crate::net::client::request::SendRequest]. This
//!    allows any [Client][crate::net::client] transport connection to be
//!    used as a [SingleService][single_service::SingleService].
//! 3) The third adapter,
//!    [BoxClientTransportToSingleService][adapter::BoxClientTransportToSingleService]
//!    is similar to the second one, except that it implements
//!    [SingleService][single_service::SingleService] for a boxed
//!    [SendRequest][crate::net::client::request::SendRequest] trait object.
//!
//! This module provides a simple query router called
//! [QnameRouter][qname_router::QnameRouter]. This router uses the query
//! name to decide with upstream [SingleService][single_service::SingleService]
//! has to handle the request.
//! This router is deliberately kept very simple. It is assumed that
//! applications that need more complex routers implement them themselves
//! in the application.

#![cfg(feature = "unstable-server-transport")]
#![cfg_attr(docsrs, doc(cfg(feature = "unstable-server-transport")))]

mod connection;
pub use connection::Config as ConnectionConfig;

pub mod adapter;
pub mod batcher;
pub mod buf;
pub mod dgram;
pub mod error;
pub mod message;
pub mod metrics;
pub mod middleware;
pub mod qname_router;
pub mod service;
pub mod single_service;
pub mod sock;
pub mod stream;
pub mod util;

#[cfg(test)]
pub mod tests;

//------------ ServerCommand ------------------------------------------------

/// Command a server to do something.
#[derive(Copy, Clone, Debug)]
pub enum ServerCommand<T: Sized> {
    #[doc(hidden)]
    /// This command is for internal use only.
    Init,

    /// Command the server to alter its configuration.
    Reconfigure(T),

    /// Command the connection handler to terminate.
    ///
    /// This command is only for connection handlers for connection-oriented
    /// transport protocols, it should be ignored by servers.
    CloseConnection,

    /// Command the server to terminate.
    Shutdown,
}
