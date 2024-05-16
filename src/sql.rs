use std::path::PathBuf;
use std::str::FromStr;
use std::fmt::Debug;

use blake3::Hash;
use serde::{Serialize, Deserialize};
use tokio_postgres::{Error as PgError};
use tokio_postgres::error::SqlState;
use tokio_postgres::types::{self, ToSql};

pub type PgJson<T> = types::Json<T>;

pub type ParamsValue<'a> = &'a (dyn ToSql + Sync);
pub type ParamsVec<'a> = Vec<&'a (dyn ToSql + Sync)>;
pub type ParamsArray<'a, const N: usize> = [&'a (dyn ToSql + Sync); N];

pub fn push_param<'a, T>(params: &mut ParamsVec<'a>, v: &'a T) -> usize
where
    T: ToSql + Sync
{
    params.push(v);
    params.len()
}

pub fn pathbuf_from_sql(value: &str) -> PathBuf {
    PathBuf::from(value)
}

pub fn mime_from_sql(type_: &str, subtype: &str) -> mime::Mime {
    let joined = format!("{}/{}", type_, subtype);

    mime::Mime::from_str(joined.as_str()).unwrap()
}

pub fn try_u64_from_sql(value: i64) -> Option<u64> {
    if value >= 0 {
        Some(value as u64)
    } else {
        None
    }
}

#[inline]
pub fn u64_from_sql(value: i64) -> u64 {
    try_u64_from_sql(value).expect("i64 is not a positive value")
}

pub fn try_blake3_hash_from_sql(value: Vec<u8>) -> Option<Hash> {
    let Ok(bytes): Result<[u8; 32], _> = value.try_into() else {
        return None;
    };

    Some(blake3::Hash::from(bytes))
}

#[inline]
pub fn blake3_hash_from_sql(value: Vec<u8>) -> Hash {
    try_blake3_hash_from_sql(value).expect("invalid byte vector length")
}

#[inline]
pub fn de_from_sql<'a, T>(value: PgJson<T>) -> T
where
    T: Deserialize<'a>
{
    value.0
}

#[inline]
pub fn ser_to_sql<'a, T>(value: &'a T) -> PgJson<&'a T>
where
    T: Serialize + Debug
{
    types::Json(value)
}

pub fn unique_constraint_error(error: &PgError) -> Option<&str> {
    let Some(db_error) = error.as_db_error() else {
        return None;
    };

    if *db_error.code() == SqlState::UNIQUE_VIOLATION {
        db_error.constraint()
    } else {
        None
    }
}
