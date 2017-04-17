use futures::Stream;

pub trait StreamingResponse<H, M, E> {
    type Body: Stream<Item = M, Error = E>;

    fn new(header: H, body: Option<Self::Body>) -> Self;
    fn parts(self) -> (H, Option<Self::Body>);
    fn parts_ref(&self) -> (&H, Option<&Self::Body>);
    fn parts_mut(&self) -> (&mut H, Option<&mut Self::Body>);
}
