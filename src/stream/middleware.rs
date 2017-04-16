use stream::*;

pub trait StreamMiddleware<S: StreamService> {
    type WrappedService: StreamService;

    fn wrap(self, service: S) -> Self::WrappedService;

    fn chain<M>(self, middleware: M) -> StreamMiddlewareChain<Self, M>
        where M: StreamMiddleware<Self::WrappedService>,
              Self: Sized,
    {
        StreamMiddlewareChain {
            inner_middleware: self,
            outer_middleware: middleware,
        }
    }

    fn reduce<R>(self, reducer: R) -> StreamMiddlewareReduceChain<Self, R>
        where R: StreamReduce<Self::WrappedService>,
              Self: Sized,
    {
        StreamMiddlewareReduceChain {
            middleware: self,
            reducer: reducer,
        }
    }
}

pub struct StreamMiddlewareChain<InnerM, OuterM> {
    inner_middleware: InnerM,
    outer_middleware: OuterM,
}

impl<S, InnerM, OuterM> StreamMiddleware<S> for StreamMiddlewareChain<InnerM, OuterM>
    where S: StreamService,
          InnerM: StreamMiddleware<S>,
          OuterM: StreamMiddleware<InnerM::WrappedService>,
{
    type WrappedService = OuterM::WrappedService;

    fn wrap(self, service: S) -> Self::WrappedService {
        service.wrap(self.inner_middleware).wrap(self.outer_middleware)
    }
}

pub struct StreamMiddlewareReduceChain<M, R> {
    middleware: M,
    reducer: R,
}

impl<S, M, R> StreamReduce<S> for StreamMiddlewareReduceChain<M, R>
    where S: StreamService,
          M: StreamMiddleware<S>,
          R: StreamReduce<M::WrappedService>,
{
    type ReducedService = R::ReducedService;

    fn reduce(self, service: S) -> Self::ReducedService {
        service.wrap(self.middleware).reduce(self.reducer)
    }
}
