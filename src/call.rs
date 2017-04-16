use std::io;

use futures::Future;

use {Service, NewService};
use stream::{StreamService, NewStreamService};

pub trait Call<Kind: 'static> {
    type Request;
    type Response;
    type Error;
    type Outcome;

    fn call(&self, req: Self::Request) -> Self::Outcome;
}

pub struct Singular;
pub struct Streaming;

impl<T: Service> Call<Singular> for T {
    type Request = <T as Service>::Request;
    type Response = <T as Service>::Response;
    type Error = <T as Service>::Error;
    type Outcome = T::Future;

    fn call(&self, req: Self::Request) -> Self::Outcome {
        Service::call(self, req)
    }
}

impl<T: StreamService> Call<Streaming> for T {
    type Request = <T as StreamService>::Request;
    type Response = <T as StreamService>::Response;
    type Error = <T as StreamService>::Error;
    type Outcome = T::Stream;

    fn call(&self, req: Self::Request) -> Self::Outcome {
        StreamService::call(self, req)
    }
}

pub trait Connect<Kind: 'static, H> {
    type Request;
    type Response;
    type Error;
    type Instance: Call<Kind, Request = Self::Request, Response = Self::Response, Error = Self::Error>;
    type Future: Future<Item = Self::Instance, Error = io::Error>;

    fn connect(&self, handle: &H) -> Self::Future;
}

impl<T: NewService<H>, H> Connect<Singular, H> for T {
    type Request = <T as NewService<H>>::Request;
    type Response = <T as NewService<H>>::Response;
    type Error = <T as NewService<H>>::Error;
    type Instance = <T as NewService<H>>::Instance;
    type Future = <T as NewService<H>>::Future;

    fn connect(&self, handle: &H) -> Self::Future {
        self.new_service(handle)
    }
}

impl<T: NewStreamService<H>, H> Connect<Streaming, H> for T {
    type Request = <T as NewStreamService<H>>::Request;
    type Response = <T as NewStreamService<H>>::Response;
    type Error = <T as NewStreamService<H>>::Error;
    type Instance = <T as NewStreamService<H>>::Instance;
    type Future = <T as NewStreamService<H>>::Future;

    fn connect(&self, handle: &H) -> Self::Future {
        self.new_service(handle)
    }
}
