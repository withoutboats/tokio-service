use std::io;
use std::rc::Rc;
use std::sync::Arc;

use futures::{Future, IntoFuture};

use stream::*;

pub trait NewStreamService<H> {
    type Request;
    type Response;
    type Error;
    type Instance: StreamService<Request = Self::Request, Response = Self::Response, Error = Self::Error>;
    type Future: Future<Item = Self::Instance, Error = io::Error>;

    fn new_service(&self, handle: &H) -> Self::Future;

    fn wrap<M>(self, new_middleware: M) -> NewStreamServiceWrapper<M, Self, H>
        where M: NewStreamMiddleware<Self::Instance>,
              Self: Sized,
    {
        new_middleware.wrap(self)
    }

    fn reduce<R>(self, new_reducer: R) -> NewStreamServiceReducer<R, Self, H>
        where R: NewStreamReduce<Self::Instance>,
              Self: Sized,
    {
        new_reducer.reduce(self)
    }
}

impl<F, R, H, I> NewStreamService<H> for F
    where F: Fn(&H) -> I,
          R: StreamService,
          I: IntoFuture<Item = R, Error = io::Error>,
{
    type Request = R::Request;
    type Response = R::Response;
    type Error = R::Error;
    type Instance = R;
    type Future = I::Future;

    fn new_service(&self, handle: &H) -> Self::Future {
        (*self)(handle).into_future()
    }
}

impl<S: NewStreamService<H> + ?Sized, H> NewStreamService<H> for Arc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Instance = S::Instance;
    type Future = S::Future;

    fn new_service(&self, handle: &H) -> Self::Future {
        (**self).new_service(handle)
    }
}

impl<S: NewStreamService<H> + ?Sized, H> NewStreamService<H> for Rc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Instance = S::Instance;
    type Future = S::Future;

    fn new_service(&self, handle: &H) -> Self::Future {
        (**self).new_service(handle)
    }
}
