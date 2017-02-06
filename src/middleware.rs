use std::marker::PhantomData;
use std::sync::Arc;

use futures::Future;

use Service;

pub trait Middleware<S: Service> {
    type WrappedService: Service;
    fn wrap(self, service: S) -> Self::WrappedService;

    fn chain<M>(self, middleware: M) -> MiddlewareChain<S, Self, M> where
        M: Middleware<Self::WrappedService>,
        Self: Sized,
    {
        MiddlewareChain {
            inner_middleware: self,
            outer_middleware: middleware,
            _marker: PhantomData,
        }
    }
}

pub struct MiddlewareChain<S, M1, M2> where
    S: Service,
    M1: Middleware<S>,
    M2: Middleware<M1::WrappedService>,
{
    inner_middleware: M1,
    outer_middleware: M2,
    _marker: PhantomData<S>,
}

pub trait Around<S: Service> {
    type Request;
    type Response;
    type Error;
    type Future: Future<Item = Self::Response, Error = Self::Error>;
    
    fn around(&self, service: &S, req: Self::Request) -> Self::Future;
}

pub struct AroundService<A, S> where
    A: Around<S>,
    S: Service,
{
    middleware: A,
    service: S,
}

impl<A, S> Service for AroundService<A, S> where
    A: Around<S>,
    S: Service,
{
    type Request = A::Request;
    type Response = A::Response;
    type Error = A::Error;
    type Future = A::Future;

    fn call(&self, req: Self::Request) -> Self::Future {
        self.middleware.around(&self.service, req)
    }
}

pub trait Before<S: Service> {
    type Request;
    type Error: Into<S::Error>;
    type Future: Future<Item = S::Request, Error = Self::Error>;

    fn before(&self, request: Self::Request) -> Self::Future;
}

pub struct BeforeService<B, S> where
    B: Before<S>,
    S: Service,
{
    middleware: B,
    service: Arc<S>,
}

impl<B, S> Service for BeforeService<B, S> where
    B: Before<S> + 'static,
    S: Service + 'static,
{
    type Request = B::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        let service = self.service.clone();
        Box::new(self.middleware.before(request).map_err(Into::into)
                     .and_then(move |req| service.call(req))) as Box<_>
    }
}

pub trait After<S: Service> {
    type Response;
    type Error: Into<S::Error>;
    type Future: Future<Item = Self::Response, Error = S::Error>;

    fn after(&self, response: S::Response) -> Self::Future;
}

pub struct AfterService<A, S> where
    A: After<S>,
    S: Service,
{
    middleware: Arc<A>,
    service: S,
}

impl<A, S> Service for AfterService<A, S> where
    A: After<S> + 'static,
    S: Service + 'static,
{
    type Request = S::Request;
    type Response = A::Response;
    type Error = S::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        let middleware = self.middleware.clone();
        Box::new(self.service.call(request)
                             .and_then(move |resp| middleware.after(resp).map_err(Into::into)))
    }
}

impl<S, M1, M2> Middleware<S> for MiddlewareChain<S, M1, M2> where
    S: Service,
    M1: Middleware<S>,
    M2: Middleware<M1::WrappedService>,
{
    type WrappedService = M2::WrappedService;

    fn wrap(self, service: S) -> Self::WrappedService {
        self.outer_middleware.wrap(self.inner_middleware.wrap(service))
    }
}

impl<B, S> Middleware<S> for B where
    B: Before<S> + 'static,
    S: Service + 'static,
{
    type WrappedService = BeforeService<B, S>;

    fn wrap(self, service: S) -> Self::WrappedService {
        BeforeService {
            middleware: self,
            service: Arc::new(service),
        }
    }
}

impl<A, S> Middleware<S> for A where
    A: Around<S>,
    S: Service,
{
    type WrappedService = AroundService<A, S>;

    fn wrap(self, service: S) -> Self::WrappedService {
        AroundService {
           middleware: self,
           service: service,
        }
    }
}

impl<A, S> Middleware<S> for A where
    A: After<S> + 'static,
    S: Service + 'static,
{
    type WrappedService = AfterService<A, S>;

    fn wrap(self, service: S) -> Self::WrappedService {
        AfterService {
            middleware: Arc::new(self),
            service: service,
        }
    }
}
