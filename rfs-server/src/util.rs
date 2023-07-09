use std::marker::Sync;
use std::convert::{From, Into};
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

    pub fn rollback(&mut self) -> bool {
        if let Some(v) = self.updated.take() {
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

    pub fn into_original(self) -> T {
        self.original
    }

    pub fn into_updated(self) -> Option<T> {
        self.updated
    }
}

impl HistoryField<String> {
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

    use serde::Deserialize;
    use tokio_postgres::types::{ToSql, Json as PgJson};

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

    pub fn u64_from_sql(value: i64) -> u64 {
        value as u64
    }

    pub fn de_from_sql<'a, T>(value: PgJson<T>) -> T
    where
        T: Deserialize<'a>
    {
        value.0
    }
}
