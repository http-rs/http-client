use std::io;
use std::pin::Pin;

use futures::future::Future;
use futures::io::AsyncRead;
use futures::task::{Context, Poll};

/// Like the `futures::io::Read` future, but returning the underlying `futures::task::Poll`.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub(crate) struct PollRead<'a, R: ?Sized> {
    reader: &'a mut R,
    buf: &'a mut [u8],
}

impl<R: ?Sized + Unpin> Unpin for PollRead<'_, R> {}

impl<'a, R: AsyncRead + ?Sized + Unpin> PollRead<'a, R> {
    pub(super) fn new(reader: &'a mut R, buf: &'a mut [u8]) -> Self {
        Self { reader, buf }
    }
}

impl<R: AsyncRead + ?Sized + Unpin> Future for PollRead<'_, R> {
    type Output = Poll<io::Result<usize>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        Poll::Ready(Pin::new(&mut this.reader).poll_read(cx, this.buf))
    }
}
