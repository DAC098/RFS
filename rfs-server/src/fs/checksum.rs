use std::task::{Context, Poll};
use std::pin::Pin;

use base64::{Engine, engine::general_purpose::URL_SAFE};
use futures::{Stream, TryStream};
use pin_project::pin_project;

pub enum Checksum {
    Blake3(blake3::Hash),
}

impl Checksum {
    pub fn as_string(&self) -> String {
        match self {
            Checksum::Blake3(hash) => {
                URL_SAFE.encode(hash.as_bytes())
            }
        }
    }
}


pub trait Digest {
    fn update(&mut self, bytes: &[u8]);

    fn finalize(&self) -> Checksum;
}

pub struct Blake3(blake3::Hasher);

impl Blake3 {
    pub fn new() -> Blake3 {
        Blake3(blake3::Hasher::new())
    }
}

impl Digest for Blake3 {
    fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn finalize(&self) -> Checksum {
        Checksum::Blake3(self.0.finalize())
    }
}

pub struct ChecksumBuilder {
    list: Vec<Box<dyn Digest + Send>>,
}

impl ChecksumBuilder {
    pub fn new() -> Self {
        ChecksumBuilder {
            list: Vec::new()
        }
    }

    pub fn add<D>(&mut self, digest: D) -> ()
    where
        D: Digest + Send + 'static
    {
        self.list.push(Box::new(digest));
    }

    pub fn update(&mut self, bytes: &[u8]) -> () {
        for digest in &mut self.list {
            digest.update(bytes);
        }
    }

    pub fn finalize(&self) -> Vec<Checksum> {
        let mut rtn = Vec::with_capacity(self.list.len());

        for digest in &self.list {
            rtn.push(digest.finalize());
        }

        rtn
    }

    pub fn stream<'a, I>(&'a mut self, source: I) -> ChecksumStream<'a, I> {
        ChecksumStream {
            source,
            checksums: self
        }
    }
}

#[pin_project]
pub struct ChecksumStream<'a, I>{
    #[pin]
    source: I,
    checksums: &'a mut ChecksumBuilder
}

impl<'a, I, T, E> Stream for ChecksumStream<'a, I>
where
    T: AsRef<[u8]>,
    I: Stream<Item = Result<T, E>> + Unpin,
{
    type Item = Result<T, E>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.source.poll_next(cx) {
            Poll::Ready(next) => {
                let Some(res) = next else {
                    return Poll::Ready(None);
                };

                match res {
                    Ok(bytes) => {
                        this.checksums.update(bytes.as_ref());

                        Poll::Ready(Some(Ok(bytes)))
                    },
                    Err(err) => Poll::Ready(Some(Err(err)))
                }
            },
            Poll::Pending => Poll::Pending
        }
    }
}

/*
impl<I> ChecksumBuilder<I>
where
    I: TryStream + Unpin,
    I::Ok: AsRef<[u8]>
{
    pub fn get_stream<'a>(&'a mut self) -> ChecksumStream<'a, I> {
        ChecksumStream(self)
    }
}

#[pin_project]
pub struct ChecksumStream<'a, I>(
    #[pin]
    &'a mut ChecksumBuilder<I>
);

impl<'a, I> Stream for ChecksumStream<'a, I>
where
    I: TryStream + Unpin,
    I::Ok: AsRef<[u8]>
{
    type Item = Result<I::Ok, I::Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match Pin::new(&mut this.0.source).try_poll_next(cx) {
            Poll::Ready(next) => {
                let Some(res) = next else {
                    return Poll::Ready(None);
                };

                match res {
                    Ok(bytes) => {
                        for digest in &mut this.0.list {
                            digest.update(bytes.as_ref());
                        }

                        Poll::Ready(Some(Ok(bytes)))
                    },
                    Err(err) => Poll::Ready(Some(Err(err)))
                }
            },
            Poll::Pending => Poll::Pending
        }
    }
}
*/
