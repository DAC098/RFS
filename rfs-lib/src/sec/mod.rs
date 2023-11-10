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
