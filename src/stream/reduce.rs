use Service;
use service::Middleware;
use stream::StreamService;

pub trait StreamReduce<S: StreamService> {
    type ReducedService: Service;

    fn reduce(self, service: S) -> Self::ReducedService;

    fn chain<M>(self, middleware: M) -> StreamReduceMiddlewareChain<Self, M>
        where M: Middleware<Self::ReducedService>,
              Self: Sized,
    {
        StreamReduceMiddlewareChain {
            reducer: self,
            middleware: middleware,
        }
    }
}

pub struct StreamReduceMiddlewareChain<R, M> {
    reducer: R,
    middleware: M,
}

impl<S, R, M> StreamReduce<S> for StreamReduceMiddlewareChain<R, M>
    where S: StreamService,
          R: StreamReduce<S>,
          M: Middleware<R::ReducedService>,
{
    type ReducedService = M::WrappedService;

    fn reduce(self, service: S) -> Self::ReducedService {
        service.reduce(self.reducer).wrap(self.middleware)
    }
}
