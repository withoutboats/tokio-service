mod middleware;
mod new_stream_service;
mod new_reduce;
mod new_middleware;
mod reduce;

pub use self::middleware::*;
pub use self::new_middleware::*;
pub use self::new_stream_service::*;
pub use self::new_reduce::*;
pub use self::reduce::*;

use std::rc::Rc;
use std::sync::Arc;

use futures::Stream;

pub trait StreamService {
    type Request;
    type Response;
    type Error;
    type Stream: Stream<Item = Self::Response, Error = Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Stream;

    fn wrap<M>(self, middleware: M) -> M::WrappedService
        where M: StreamMiddleware<Self>,
              Self: Sized,
    {
        middleware.wrap(self)
    }

    fn reduce<R>(self, reducer: R) -> R::ReducedService
        where R: StreamReduce<Self>,
              Self: Sized,
    {
        reducer.reduce(self)
    }
}

impl<S: StreamService + ?Sized> StreamService for Box<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Stream = S::Stream;

    fn call(&self, request: S::Request) -> S::Stream {
        (**self).call(request)
    }
}

impl<S: StreamService + ?Sized> StreamService for Rc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Stream = S::Stream;

    fn call(&self, request: S::Request) -> S::Stream {
        (**self).call(request)
    }
}

impl<S: StreamService + ?Sized> StreamService for Arc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Stream = S::Stream;

    fn call(&self, request: S::Request) -> S::Stream {
        (**self).call(request)
    }
}
