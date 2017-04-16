use std::io;
use std::marker::PhantomData;

use futures::{Future, Poll, Async};

use service::*;
use stream::*;

pub trait NewStreamReduce<S: StreamService> {
    type ReducedService: Service;
    type Instance: StreamReduce<S, ReducedService = Self::ReducedService>;
    type Future: Future<Item = Self::Instance, Error = io::Error>;

    fn new_reducer(&self) -> Self::Future;

    fn reduce<N, H>(self, new_service: N) -> NewStreamServiceReducer<Self, N, H>
        where N: NewStreamService<H, Instance = S, Request = S::Request, Response = S::Response, Error = S::Error>,
              Self: Sized,
    {
        NewStreamServiceReducer {
            service: new_service,
            reducer: self,
            _marker: PhantomData,
        }
    }

    fn chain<M>(self, new_middleware: M) -> NewStreamReduceMiddlewareChain<Self, M>
        where M: NewMiddleware<Self::ReducedService>,
              Self: Sized,
    {
        NewStreamReduceMiddlewareChain {
            reducer: self,
            middleware: new_middleware,
        }
    }
}

pub struct NewStreamServiceReducer<R: NewStreamReduce<S::Instance>, S: NewStreamService<H>, H> {
    service: S,
    reducer: R,
    _marker: PhantomData<H>,
}

impl<R, S, W, H> NewService<H> for NewStreamServiceReducer<R, S, H>
    where S: NewStreamService<H>,
          R: NewStreamReduce<S::Instance, ReducedService = W>,
          W: Service,
{
    type Request = W::Request;
    type Response = W::Response;
    type Error = W::Error;
    type Instance = W;
    type Future = ReducedStreamServiceFuture<S::Future, R::Future>;

    fn new_service(&self, handle: &H) -> Self::Future {
        ReducedStreamServiceFuture {
            service_future: self.service.new_service(handle),
            reducer_future: self.reducer.new_reducer(),
            service: None,
        }
    }
}

pub struct ReducedStreamServiceFuture<S: Future, R: Future> {
    service_future: S,
    reducer_future: R,
    service: Option<S::Item>
}

impl<S, R> Future for ReducedStreamServiceFuture<S, R>
    where S: Future<Error = io::Error>,
          S::Item: StreamService,
          R: Future<Error = io::Error>,
          R::Item: StreamReduce<S::Item>,
{
    type Item = <R::Item as StreamReduce<S::Item>>::ReducedService;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, io::Error> {
        let service = if let Some(service) = self.service.take() {
            service
        } else if let Async::Ready(service) = self.service_future.poll()? {
            service
        } else { return Ok(Async::NotReady) };
        
        if let Async::Ready(reducer) = self.reducer_future.poll()? {
            Ok(Async::Ready(service.reduce(reducer)))
        } else {
            self.service = Some(service);
            Ok(Async::NotReady)
        }
    }
}

pub struct NewStreamReduceMiddlewareChain<R, M> {
    reducer: R,
    middleware: M,
}

impl<S, R, M> NewStreamReduce<S> for NewStreamReduceMiddlewareChain<R, M>
where
    S: StreamService,
    R: NewStreamReduce<S>,
    M: NewMiddleware<R::ReducedService>,
{
    type ReducedService = M::WrappedService;
    type Instance = StreamReduceMiddlewareChain<R::Instance, M::Instance>;
    type Future = ChainedStreamReduceFuture<R::Future, M::Future, S>;

    fn new_reducer(&self) -> Self::Future {
        ChainedStreamReduceFuture {
            reducer_future: self.reducer.new_reducer(),
            middleware_future: self.middleware.new_middleware(),
            reducer: None,
            _marker: PhantomData,
        }
    }
}

pub struct ChainedStreamReduceFuture<R: Future, M: Future, S> {
    reducer_future: R,
    middleware_future: M,
    reducer: Option<R::Item>,
    _marker: PhantomData<S>,
}

impl<RFut, MFut, R, M, S> Future for ChainedStreamReduceFuture<RFut, MFut, S>
    where RFut: Future<Item = R, Error = io::Error>,
          MFut: Future<Item = M, Error = io::Error>,
          R: StreamReduce<S>,
          M: Middleware<R::ReducedService>,
          S: StreamService,
{
    type Item = StreamReduceMiddlewareChain<R, M>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let reducer = if let Some(reducer) = self.reducer.take() {
            reducer
        } else if let Async::Ready(reducer) = self.reducer_future.poll()? {
            reducer
        } else { return Ok(Async::NotReady) };
        
        if let Async::Ready(middleware) = self.middleware_future.poll()? {
            Ok(Async::Ready(reducer.chain(middleware)))
        } else {
            self.reducer = Some(reducer);
            Ok(Async::NotReady)
        }
    }
}
