use std::io;
use std::marker::PhantomData;

use futures::{Future, Poll, Async};

use stream::*;


pub trait NewStreamMiddleware<S: StreamService> {
    type WrappedService: StreamService;
    type Instance: StreamMiddleware<S, WrappedService = Self::WrappedService>;
    type Future: Future<Item = Self::Instance, Error = io::Error>;

    fn new_middleware(&self) -> Self::Future;

    fn wrap<N, H>(self, new_service: N) -> NewStreamServiceWrapper<Self, N, H>
        where N: NewStreamService<H, Instance = S, Request = S::Request, Response = S::Response, Error = S::Error>,
              Self: Sized,
    {
        NewStreamServiceWrapper {
            service: new_service,
            middleware: self,
            _marker: PhantomData,
        }
    }

    fn chain<M>(self, new_middleware: M) -> NewStreamMiddlewareChain<Self, M>
        where M: NewStreamMiddleware<Self::WrappedService>,
              Self: Sized,
    {
        NewStreamMiddlewareChain {
            inner_middleware: self,
            outer_middleware: new_middleware,
        }
    }

    fn reduce<R>(self, new_reducer: R) -> NewStreamMiddlewareReduceChain<Self, R>
        where R: NewStreamReduce<Self::WrappedService>,
              Self: Sized,
    {
        NewStreamMiddlewareReduceChain {
            reducer: new_reducer,
            middleware: self,
        }
    }
}

pub struct NewStreamServiceWrapper<M: NewStreamMiddleware<S::Instance>, S: NewStreamService<H>, H> {
    service: S,
    middleware: M,
    _marker: PhantomData<H>,
}

impl<M, S, W, H> NewStreamService<H> for NewStreamServiceWrapper<M, S, H>
    where S: NewStreamService<H>,
          M: NewStreamMiddleware<S::Instance, WrappedService = W>,
          W: StreamService,
{
    type Request = W::Request;
    type Response = W::Response;
    type Error = W::Error;
    type Instance = W;
    type Future = WrappedStreamServiceFuture<S::Future, M::Future>;

    fn new_service(&self, handle: &H) -> Self::Future {
        WrappedStreamServiceFuture {
            service_future: self.service.new_service(handle),
            middleware_future: self.middleware.new_middleware(),
            service: None,
        }
    }
}

pub struct WrappedStreamServiceFuture<S: Future, M: Future> {
    service_future: S,
    middleware_future: M,
    service: Option<S::Item>
}

impl<S, M, W> Future for WrappedStreamServiceFuture<S, M>
    where S: Future<Error = io::Error>,
          S::Item: StreamService,
          M: Future<Error = io::Error>,
          M::Item: StreamMiddleware<S::Item, WrappedService = W>,
          W: StreamService,
{
    type Item = W;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, io::Error> {
        let service = if let Some(service) = self.service.take() {
            service
        } else if let Async::Ready(service) = self.service_future.poll()? {
            service
        } else { return Ok(Async::NotReady) };
        
        if let Async::Ready(middleware) = self.middleware_future.poll()? {
            Ok(Async::Ready(service.wrap(middleware)))
        } else {
            self.service = Some(service);
            Ok(Async::NotReady)
        }
    }
}

pub struct NewStreamMiddlewareChain<InnerM, OuterM> {
    inner_middleware: InnerM,
    outer_middleware: OuterM,
}

impl<S, InnerM, OuterM> NewStreamMiddleware<S> for NewStreamMiddlewareChain<InnerM, OuterM>
    where S: StreamService,
          InnerM: NewStreamMiddleware<S>,
          OuterM: NewStreamMiddleware<InnerM::WrappedService>,
{
    type Instance = StreamMiddlewareChain<InnerM::Instance, OuterM::Instance>;
    type WrappedService = OuterM::WrappedService;
    type Future = ChainedStreamMiddlewareFuture<InnerM::Future, OuterM::Future, S>;

    fn new_middleware(&self) -> Self::Future {
        ChainedStreamMiddlewareFuture {
            inner_future: self.inner_middleware.new_middleware(),
            outer_future: self.outer_middleware.new_middleware(),
            inner: None,
            _marker: PhantomData,
        }
    }
}

pub struct ChainedStreamMiddlewareFuture<InnerF: Future, OuterF: Future, S> {
    inner_future: InnerF,
    outer_future: OuterF,
    inner: Option<InnerF::Item>,
    _marker: PhantomData<S>,
}

impl<InnerF, OuterF, InnerM, OuterM, S> Future for ChainedStreamMiddlewareFuture<InnerF, OuterF, S>
    where InnerF: Future<Item = InnerM, Error = io::Error>,
          OuterF: Future<Item = OuterM, Error = io::Error>,
          InnerM: StreamMiddleware<S>,
          OuterM: StreamMiddleware<InnerM::WrappedService>,
          S: StreamService,
{
    type Item = StreamMiddlewareChain<InnerM, OuterM>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let inner = if let Some(inner) = self.inner.take() {
            inner
        } else if let Async::Ready(inner) = self.inner_future.poll()? {
            inner
        } else { return Ok(Async::NotReady) };
        
        if let Async::Ready(outer) = self.outer_future.poll()? {
            Ok(Async::Ready(inner.chain(outer)))
        } else {
            self.inner = Some(inner);
            Ok(Async::NotReady)
        }
    }
}

pub struct NewStreamMiddlewareReduceChain<M, R> {
    middleware: M,
    reducer: R,
}

impl<S, M, R> NewStreamReduce<S> for NewStreamMiddlewareReduceChain<M, R>
    where S: StreamService,
          M: NewStreamMiddleware<S>,
          R: NewStreamReduce<M::WrappedService>,
{
    type ReducedService = R::ReducedService;
    type Instance = StreamMiddlewareReduceChain<M::Instance, R::Instance>;
    type Future = ReducedStreamMiddlewareFuture<M::Future, R::Future, S>;

    fn new_reducer(&self) -> Self::Future {
        ReducedStreamMiddlewareFuture {
            middleware_future: self.middleware.new_middleware(),
            reducer_future: self.reducer.new_reducer(),
            middleware: None,
            _marker: PhantomData,
        }
    }
}

pub struct ReducedStreamMiddlewareFuture<M: Future, R: Future, S> {
    middleware_future: M,
    reducer_future: R,
    middleware: Option<M::Item>,
    _marker: PhantomData<S>,
}

impl<MFut, RFut, M, R, S> Future for ReducedStreamMiddlewareFuture<MFut, RFut, S>
    where MFut: Future<Item = M, Error = io::Error>,
          RFut: Future<Item = R, Error = io::Error>,
          M: StreamMiddleware<S>,
          R: StreamReduce<M::WrappedService>,
          S: StreamService,
{
    type Item = StreamMiddlewareReduceChain<M, R>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let middleware = if let Some(middleware) = self.middleware.take() {
            middleware
        } else if let Async::Ready(middleware) = self.middleware_future.poll()? {
            middleware
        } else { return Ok(Async::NotReady) };
        
        if let Async::Ready(reducer) = self.reducer_future.poll()? {
            Ok(Async::Ready(middleware.reduce(reducer)))
        } else {
            self.middleware = Some(middleware);
            Ok(Async::NotReady)
        }
    }
}
