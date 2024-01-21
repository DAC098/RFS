use std::error::Error;
use std::cmp::PartialEq;
use std::default::Default;

use bytes::BytesMut;
use postgres_types::{to_sql_checked, Type, IsNull, ToSql};
use serde_repr::{Serialize_repr, Deserialize_repr};

pub type Offset = u8;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq,
    Serialize_repr, Deserialize_repr
)]
#[repr(u8)]
pub enum Limit {
    Small = 25,
    Medium = 50,
    Large = 100
}

impl Limit {
    pub fn sql_offset(&self, offset: Offset) -> i64 {
        (*self as i64) * (offset as i64)
    }
}

impl Default for Limit {
    fn default() -> Limit {
        Limit::Small
    }
}

impl ToSql for Limit {
    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let v = *self as i64;

        v.to_sql(ty, w)
    }

    fn accepts(ty: &Type) -> bool {
        <i64 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}
