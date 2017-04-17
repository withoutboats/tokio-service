//! Definition of the core `Service` trait to Tokio
//!
//! More information can be found on [the trait] itself and online at
//! [https://tokio.rs](https://tokio.rs)
//!
//! [the trait]: trait.Service.html

//#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/tokio-service/0.1")]

extern crate futures;

pub mod middleware;
pub mod streaming;

mod service;

pub use service::*;
