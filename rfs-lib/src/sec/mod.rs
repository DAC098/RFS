pub mod chacha;

pub mod secrets {
    pub type Version = u64;
    pub type Timestamp = u64;

    pub const PASSWORDS_KEY_INFO: &[u8; 9] = b"passwords";
    pub const SESSIONS_KEY_INFO: &[u8; 8] = b"sessions";

    pub mod manager {
        use serde::{Serialize, Deserialize};

        #[derive(Debug, Serialize, Deserialize)]
        pub struct ManagerFile {
            pub count: super::Version
        }
    }
}

pub mod authz {
    pub mod permission {
        use std::error::Error;

        use serde::{Serialize, Deserialize};
        use bytes::BytesMut;
        use postgres_types::{to_sql_checked, Type, IsNull, ToSql, FromSql};

        use crate::validation::check_control_whitespace;

        pub const MAX_ROLE_CHARS: usize = 64;

        pub fn role_name_valid(given: &str) -> bool {
            !given.is_empty() && check_control_whitespace(given, Some(MAX_ROLE_CHARS))
        }

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
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

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
        pub enum Scope {
            SecSecrets,
            SecRoles,
            User,
            UserGroup,
            Fs,
            Storage
        }

        impl Scope {
            pub fn from_str(v: &str) -> Option<Self> {
                match v {
                    "SecSecrets" => Some(Scope::SecSecrets),
                    "SecRoles" => Some(Scope::SecRoles),
                    "User" => Some(Scope::User),
                    "UserGroup" => Some(Scope::UserGroup),
                    "Fs" => Some(Scope::Fs),
                    "Storage" => Some(Scope::Storage),
                    _ => None
                }
            }

            pub fn as_str(&self) -> &str {
                match self {
                    Scope::SecSecrets => "SecSecrets",
                    Scope::SecRoles => "SecRoles",
                    Scope::User => "User",
                    Scope::UserGroup => "UserGroup",
                    Scope::Fs => "Fs",
                    Scope::Storage => "Storage",
                }
            }
        }

        impl<'a> FromSql<'a> for Scope {
            fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
                let v = <&str as FromSql>::from_sql(ty, raw)?;

                Scope::from_str(v)
                    .ok_or("invalid sql value for Ability. expecting \"SecStorage\", \
                           \"SecRoles\", \
                           \"User\", \
                           \"UserGroup\", \
                           \"Fs\", \
                           \"Storage\"".into())
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
    }
}

pub mod authn {
    pub const MIN_PASSWORD_CHARS: usize = 8;
    pub const MAX_PASSWORD_CHARS: usize = 512;

    pub fn password_valid(given: &String) -> bool {
        let iter = given.chars();
        let mut char_count = 0;

        for ch in iter {
            if ch.is_control() {
                return false;
            }

            char_count += 1;

            if char_count > MAX_PASSWORD_CHARS {
                return false;
            }
        }

        if char_count < MIN_PASSWORD_CHARS {
            return false;
        }

        true
    }

    pub mod totp {
        use std::str::FromStr;

        use serde::{Serialize, Deserialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Algo {
            SHA1,
            SHA256,
            SHA512,
        }

        impl Algo {
            pub fn from_i16(v: i16) -> Option<Self> {
                match v {
                    0 => Some(Algo::SHA1),
                    1 => Some(Algo::SHA256),
                    2 => Some(Algo::SHA512),
                    _ => None
                }
            }

            pub fn as_i16(&self) -> i16 {
                match self {
                    Algo::SHA1 => 0,
                    Algo::SHA256 => 1,
                    Algo::SHA512 => 2,
                }
            }

            pub fn to_string(&self) -> String {
                match self {
                    Algo::SHA1 => String::from("SHA1"),
                    Algo::SHA256 => String::from("SHA256"),
                    Algo::SHA512 => String::from("SHA512"),
                }
            }
        }

        pub struct FromIntError;

        impl TryFrom<i16> for Algo {
            type Error = FromIntError;

            fn try_from(v: i16) -> Result<Self, Self::Error> {
                Self::from_i16(v).ok_or(FromIntError)
            }
        }

        impl From<Algo> for i16 {
            fn from(v: Algo) -> i16 {
                v.as_i16()
            }
        }

        pub struct FromStrError;

        impl FromStr for Algo {
            type Err = FromStrError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    "SHA1" => Ok(Algo::SHA1),
                    "SHA256" => Ok(Algo::SHA256),
                    "SHA512" => Ok(Algo::SHA512),
                    _ => Err(FromStrError),
                }
            }
        }

        impl TryFrom<&str> for Algo {
            type Error = FromStrError;

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::from_str(s)
            }
        }

        impl TryFrom<String> for Algo {
            type Error = FromStrError;

            fn try_from(s: String) -> Result<Self, Self::Error> {
                Self::from_str(&s)
            }
        }

        pub fn digits_valid(given: &u32) -> bool {
            *given <= 12
        }

        pub fn step_valid(given: &u64) -> bool {
            *given <= 120
        }

        pub mod recovery {
            use crate::validation::check_control_whitespace;

            pub const MAX_KEY_CHARS: usize = 64;

            pub fn key_valid(given: &String) -> bool {
                !given.is_empty() && check_control_whitespace(given, Some(MAX_KEY_CHARS))
            }

            #[cfg(test)]
            mod test {
                use super::*;

                #[test]
                fn key_validation() {
                    let valid = vec![
                        String::from("i_am_a_key"),
                        String::from("meh_for_emoji_😕"),
                    ];

                    for test in valid {
                        assert!(key_valid(&test), "valid string failed {:?}", test);
                    }

                    let invalid = vec![
                        String::new(),
                        String::from(" key \u{0000} stuff "),
                        crate::string_to_len(MAX_KEY_CHARS + 1),
                    ];

                    for test in invalid {
                        assert!(!key_valid(&test), "invalid string failed {:?}", test);
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn password_validation() {
            let valid = vec![
                String::from("-h6Ď♂♱ƲÐȷ♋🙋ȆċŶ😣ƁŨ😌☑æȘŤŎ😕♁🙍"),
                String::from("Sharper Snowboard Equinox Faucet Monoxide0"),
            ];

            for test in valid {
                assert!(password_valid(&test), "valid string failed {:?}", test);
            }

            let invalid = vec![
                String::from("   test  \u{0000} other stuff"),
                crate::string_to_len(MIN_PASSWORD_CHARS - 1),
                crate::string_to_len(MAX_PASSWORD_CHARS + 1),
            ];

            for test in invalid {
                assert!(!password_valid(&test), "invalid string failed {:?}", test);
            }
        }
    }
}
