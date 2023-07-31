use std::convert::From;
use std::time::SystemTime;

pub fn utc_now() -> Option<u64> {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => Some(d.as_secs()),
        Err(_) => None
    }
}

pub struct HistoryField<T> {
    original: T,
    updated: Option<T>,
}

impl<T> HistoryField<T> {
    pub fn new(original: T) -> Self {
        HistoryField {
            original,
            updated: None
        }
    }

    pub fn get(&self) -> &T {
        self.updated.as_ref().unwrap_or(&self.original)
    }

    pub fn set(&mut self, v: T) -> () {
        self.updated = Some(v);
    }

    pub fn original(&self) -> &T {
        &self.original
    }

    pub fn updated(&self) -> Option<&T> {
        self.updated.as_ref()
    }

    pub fn is_updated(&self) -> bool {
        self.updated.is_some()
    }

    #[allow(dead_code)]
    pub fn rollback(&mut self) -> bool {
        if let Some(_) = self.updated.take() {
            true
        } else {
            false
        }
    }

    pub fn commit(&mut self) -> bool {
        if let Some(v) = self.updated.take() {
            self.original = v;
            true
        } else {
            false
        }
    }

    pub fn into_inner(self) -> T {
        self.updated.unwrap_or(self.original)
    }

    #[allow(dead_code)]
    pub fn into_original(self) -> T {
        self.original
    }

    #[allow(dead_code)]
    pub fn into_updated(self) -> Option<T> {
        self.updated
    }
}

impl HistoryField<String> {
    #[allow(dead_code)]
    pub fn get_str(&self) -> &str {
        if let Some(v) = self.updated.as_ref() {
            v.as_str()
        } else {
            &self.original.as_str()
        }
    }
}

impl<T> AsRef<T> for HistoryField<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T> From<T> for HistoryField<T> {
    fn from(v: T) -> Self {
        HistoryField::new(v)
    }
}

impl<T> PartialEq<HistoryField<T>> for HistoryField<T>
where
    T: PartialEq<T>
{
    fn eq(&self, rhs: &HistoryField<T>) -> bool {
        self.get().eq(rhs.get())
    }
}

impl<T> PartialEq<T> for HistoryField<T>
where
    T: PartialEq<T>
{
    fn eq(&self, rhs: &T) -> bool {
        self.get().eq(rhs)
    }
}

pub mod sql {
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::fmt::Debug;

    use blake3::Hash;
    use serde::{Serialize, Deserialize};
    use tokio_postgres::types::{self, ToSql};

    pub type PgJson<T> = types::Json<T>;

    pub type ParamsVec<'a> = Vec<&'a (dyn ToSql + Sync)>;

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

    pub fn try_blake3_hash_from_sql(mut value: Vec<u8>) -> Option<Hash> {
        if value.len() != 32 {
            None
        } else {
            let mut index = 0;
            let mut bytes = [0u8; 32];
            let mut drain = value.drain(..);

            while let Some(byte) = drain.next() {
                bytes[index] = byte;
                index += 1;
            }

            Some(blake3::Hash::from(bytes))
        }
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
}
