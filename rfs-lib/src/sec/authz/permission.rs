use std::error::Error;

use serde::{Serialize, Deserialize};
use bytes::BytesMut;
use postgres_types::{to_sql_checked, Type, IsNull, ToSql, FromSql};

use crate::validation::check_control_whitespace;

pub const MAX_ROLE_CHARS: usize = 64;

pub fn role_name_valid(given: &str) -> bool {
    !given.is_empty() && check_control_whitespace(given, Some(MAX_ROLE_CHARS))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
pub enum Ability {
    Read,
    Write,
}

impl Ability {
    pub fn from_str(v: &str) -> Option<Self> {
        match v {
            "Read" => Some(Ability::Read),
            "Write" => Some(Ability::Write),
            _ => None
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Ability::Read => "Read",
            Ability::Write => "Write",
        }
    }
}

impl<'a> FromSql<'a> for Ability {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Ability, Box<dyn Error + Sync + Send>> {
        let v = <&str as FromSql>::from_sql(ty, raw)?;

        Ability::from_str(v)
            .ok_or("invalid sql value for Ability. expecting \"Read\" or \"Write\"".into())
    }

    fn accepts(ty: &Type) -> bool {
        <&str as FromSql>::accepts(ty)
    }
}

impl ToSql for Ability {
    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let v = self.as_str();

        v.to_sql(ty, w)
    }

    fn accepts(ty: &Type) -> bool {
        <&str as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
pub enum Scope {
    Fs,
    SecRoles,
    SecSecrets,
    Storage,
    User,
    UserGroup,
}

impl Scope {
    pub fn from_str(v: &str) -> Option<Self> {
        match v {
            "Fs" => Some(Scope::Fs),
            "SecRoles" => Some(Scope::SecRoles),
            "SecSecrets" => Some(Scope::SecSecrets),
            "Storage" => Some(Scope::Storage),
            "User" => Some(Scope::User),
            "UserGroup" => Some(Scope::UserGroup),
            _ => None
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Scope::Fs => "Fs",
            Scope::SecRoles => "SecRoles",
            Scope::SecSecrets => "SecSecrets",
            Scope::Storage => "Storage",
            Scope::User => "User",
            Scope::UserGroup => "UserGroup",
        }
    }
}

impl<'a> FromSql<'a> for Scope {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let v = <&str as FromSql>::from_sql(ty, raw)?;

        Scope::from_str(v)
            .ok_or("invalid sql value for Ability. expecting \
                \"Fs\", \
                \"SecRoles\", \
                \"SecStorage\", \
                \"Storage\", \
                \"User\", \
                \"UserGroup\"".into())
    }

    fn accepts(ty: &Type) -> bool {
        <&str as FromSql>::accepts(ty)
    }
}

impl ToSql for Scope {
    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let v = self.as_str();

        v.to_sql(ty, w)
    }

    fn accepts(ty: &Type) -> bool {
        <&str as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}
