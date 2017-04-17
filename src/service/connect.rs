use std::io;
use std::rc::Rc;
use std::sync::Arc;

use futures::{Future, IntoFuture};

use Service;
use middleware::*;

/// Creates new `Service` values.
pub trait Connect<H> {
    /// Requests handled by the service
    type Request;

    /// Responses given by the service
    type Response;

    /// Errors produced by the service
    type Error;

    /// The `Service` value created by this factory
    type Instance: Service<Request = Self::Request, Response = Self::Response, Error = Self::Error>;

    type Future: Future<Item = Self::Instance, Error = io::Error>;

    /// Create and return a new service value.
    fn connect(&self, handle: &H) -> Self::Future;

    fn wrap<M>(self, new_middleware: M) -> ConnectWrapper<M, Self, H>
        where M: ConnectMiddleware<Self::Instance, H>,
              Self: Sized,
    {
        new_middleware.wrap(self)
    }
}

impl<F, R, H, I> Connect<H> for F
    where F: Fn(&H) -> I,
          I: IntoFuture<Item = R, Error = io::Error>,
          R: Service,
{
    type Request = R::Request;
    type Response = R::Response;
    type Error = R::Error;
    type Instance = R;
    type Future = I::Future;

    fn connect(&self, handle: &H) -> I::Future {
        (*self)(handle).into_future()
    }
}

impl<S: Connect<H> + ?Sized, H> Connect<H> for Arc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Instance = S::Instance;
    type Future = S::Future;

    fn connect(&self, handle: &H) -> Self::Future {
        (**self).connect(handle)
    }
}

impl<S: Connect<H> + ?Sized, H> Connect<H> for Rc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Instance = S::Instance;
    type Future = S::Future;

    fn connect(&self, handle: &H) -> Self::Future {
        (**self).connect(handle)
    }
}
