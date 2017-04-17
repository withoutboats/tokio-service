use std::io;
use std::marker::PhantomData;

use futures::{Future, Poll, Async};

use service::{Service, NewService};
use middleware::*;

pub trait NewMiddleware<S: Service, H> {
    type WrappedService: Service;
    type Instance: Middleware<S, WrappedService = Self::WrappedService>;
    type Future: Future<Item = Self::Instance, Error = io::Error>;

    fn new_middleware(&self, handle: &H) -> Self::Future;

    fn wrap<N>(self, new_service: N) -> NewServiceWrapper<Self, N, H>
        where N: NewService<H, Instance = S, Request = S::Request, Response = S::Response, Error = S::Error>,
              Self: Sized,
    {
        NewServiceWrapper {
            service: new_service,
            middleware: self,
            _marker: PhantomData,
        }
    }

    fn chain<M>(self, new_middleware: M) -> NewMiddlewareChain<Self, M>
        where M: NewMiddleware<Self::WrappedService, H>,
              Self: Sized,
    {
        NewMiddlewareChain {
            inner_middleware: self,
            outer_middleware: new_middleware,
        }
    }
}

pub struct NewServiceWrapper<M: NewMiddleware<S::Instance, H>, S: NewService<H>, H> {
    service: S,
    middleware: M,
    _marker: PhantomData<H>,
}

impl<M, S, W, H> NewService<H> for NewServiceWrapper<M, S, H>
    where S: NewService<H>,
          M: NewMiddleware<S::Instance, H, WrappedService = W>,
          W: Service,
{
    type Request = W::Request;
    type Response = W::Response;
    type Error = W::Error;
    type Instance = W;
    type Future = WrappedServiceFuture<S::Future, M::Future>;

    fn new_service(&self, handle: &H) -> Self::Future {
        WrappedServiceFuture {
            service_future: self.service.new_service(handle),
            middleware_future: self.middleware.new_middleware(handle),
            service: None,
        }
    }
}

pub struct WrappedServiceFuture<S: Future, M: Future> {
    service_future: S,
    middleware_future: M,
    service: Option<S::Item>
}

impl<S, M> Future for WrappedServiceFuture<S, M>
    where S: Future<Error = io::Error>,
          S::Item: Service,
          M: Future<Error = io::Error>,
          M::Item: Middleware<S::Item>,
{
    type Item = <M::Item as Middleware<S::Item>>::WrappedService;
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

pub struct NewMiddlewareChain<InnerM, OuterM> {
    inner_middleware: InnerM,
    outer_middleware: OuterM,
}

impl<S, InnerM, OuterM, H> NewMiddleware<S, H> for NewMiddlewareChain<InnerM, OuterM>
    where S: Service,
          InnerM: NewMiddleware<S, H>,
          OuterM: NewMiddleware<InnerM::WrappedService, H>,
{
    type Instance = MiddlewareChain<InnerM::Instance, OuterM::Instance>;
    type WrappedService = OuterM::WrappedService;
    type Future = ChainedMiddlewareFuture<InnerM::Future, OuterM::Future, S>;

    fn new_middleware(&self, handle: &H) -> Self::Future {
        ChainedMiddlewareFuture {
            inner_future: self.inner_middleware.new_middleware(handle),
            outer_future: self.outer_middleware.new_middleware(handle),
            inner: None,
            _marker: PhantomData,
        }
    }
}

pub struct ChainedMiddlewareFuture<InnerF: Future, OuterF: Future, S> {
    inner_future: InnerF,
    outer_future: OuterF,
    inner: Option<InnerF::Item>,
    _marker: PhantomData<S>,
}

impl<InnerF, OuterF, InnerM, OuterM, S> Future for ChainedMiddlewareFuture<InnerF, OuterF, S>
    where InnerF: Future<Item = InnerM, Error = io::Error>,
          OuterF: Future<Item = OuterM, Error = io::Error>,
          InnerM: Middleware<S>,
          OuterM: Middleware<InnerM::WrappedService>,
          S: Service,
{
    type Item = MiddlewareChain<InnerM, OuterM>;
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
