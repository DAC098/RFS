use std::iter::Iterator;

use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::{
    Limit,
    Payload,
    Pagination,
};

pub trait Pageable {
    type Id;
    type Item;

    fn set_limit(&mut self, limit: Option<Limit>);
    fn set_last_id(&mut self, id: Option<Self::Id>);
    fn get_last_id(item: &Self::Item) -> Option<Self::Id>;

    fn send(&self, client: &ApiClient) -> Result<Payload<Vec<Self::Item>>, RequestError>;
}

pub fn iterate<P, F>(client: &ApiClient, pageable: &mut P, mut cb: F) -> Result<(), RequestError>
where
    P: Pageable,
    F: FnMut(usize, P::Item) -> bool
{
    loop {
        let (pagination, payload) = pageable.send(client)?.into_tuple();

        if let Some(last_item) = payload.last() {
            let Some(last_id) = P::get_last_id(last_item) else {
                break;
            };

            pageable.set_last_id(Some(last_id));
        } else {
            break;
        }

        let len = payload.len();

        for (index, item) in payload.into_iter().enumerate() {
            if !cb(index, item) {
                return Ok(());
            }
        }

        if let Some(pagination) = pagination {
            let limit = (*pagination.limit()) as usize;

            if len != limit {
                break;
            }
        }
    }

    Ok(())
}

struct IterateData<I> {
    iter: std::vec::IntoIter<I>,
    pagination: Pagination,
    len: usize,
}

enum IterateState<I> {
    Empty,
    Done,
    Ready(IterateData<I>)
}

impl<I> IterateState<I> {
    fn request_chunk<P>(
        client: &ApiClient,
        pageable: &mut P
    ) -> (Option<Result<I, RequestError>>, IterateState<I>)
    where
        P: Pageable<Item = I>
    {
        match pageable.send(client) {
            Ok(result) => {
                let (pagination, payload) = result.into_tuple();

                let len = payload.len();

                let Some(last_item) = payload.last() else {
                    return (None, IterateState::Done);
                };

                let Some(last_id) = P::get_last_id(last_item) else {
                    return (None, IterateState::Done);
                };

                pageable.set_last_id(Some(last_id));

                let mut iter = payload.into_iter();
                let item = iter.next().unwrap();

                (Some(Ok(item)), IterateState::Ready(IterateData {
                    iter,
                    pagination: pagination.expect("no pagination data with request"),
                    len
                }))
            },
            Err(err) => (Some(Err(err)), IterateState::Done),
        }
    }

    fn next_state<P>(
        self,
        client: &ApiClient,
        pageable: &mut P
    ) -> (Option<Result<I, RequestError>>, IterateState<I>)
    where
        P: Pageable<Item = I>
    {
        match self {
            IterateState::Done => (None, IterateState::Done),
            IterateState::Empty => Self::request_chunk(client, pageable),
            IterateState::Ready(mut data) => match data.iter.next() {
                Some(value) => (Some(Ok(value)), IterateState::Ready(data)),
                None => {
                    let limit = (*data.pagination.limit()) as usize;

                    if data.len != limit {
                        (None, IterateState::Done)
                    } else {
                        Self::request_chunk(client, pageable)
                    }
                }
            }
        }
    }
}

pub struct Iterate<'a, 'b, P>
where
    P: Pageable
{
    client: &'a ApiClient,
    pageable: &'b mut P,
    state: IterateState<P::Item>,
}

impl<'a, 'b, P> Iterate<'a, 'b, P>
where
    P: Pageable
{
    pub fn new(client: &'a ApiClient, pageable: &'b mut P) -> Self {
        Iterate {
            client,
            pageable,
            state: IterateState::Empty
        }
    }
}

impl<'a, 'b, P> Iterator for Iterate<'a, 'b, P>
where
    P: Pageable
{
    type Item = Result<P::Item, RequestError>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = std::mem::replace(&mut self.state, IterateState::Done);
        let (item, state) = curr.next_state(
            self.client,
            self.pageable
        );

        self.state = state;

        item
    }
}
