mod response;

pub use self::response::*;

use futures::Future;

use service::Service;

pub trait StreamingService {
    type Request;
    type Header;
    type Member;
    type Error;
    type Response: StreamingResponse<Self::Header, Self::Member, Self::Error>;
    type Future: Future<Item = Self::Response, Error = Self::Error>;

    /// Process the request and return the response asynchronously.
    fn call(&self, req: Self::Request) -> Self::Future;
}

impl<S: StreamingService> Service for S {
    type Request = <S as StreamingService>::Request;
    type Response = <S as StreamingService>::Response;
    type Error = <S as StreamingService>::Error;
    type Future = <S as StreamingService>::Future;

    fn call(&self, req: Self::Request) -> Self::Future {
        <S as StreamingService>::call(self, req)
    }
}
