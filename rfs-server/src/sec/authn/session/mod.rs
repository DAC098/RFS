use rfs_lib::ids;
use chrono::{DateTime, Utc};
use base64::{Engine, engine::general_purpose::URL_SAFE};
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use hmac::{Hmac, Mac, digest::CtOutput};

use crate::sec::state;
use crate::net::error::Error as NetError;
use crate::net::cookie::{SameSite, SetCookie};

pub mod token;

pub enum AuthMethod {
    None,
    Password
}

impl AuthMethod {
    fn from_i16(v: i16) -> Option<Self> {
        match v {
            0 => Some(AuthMethod::None),
            1 => Some(AuthMethod::Password),
            _ => None
        }
    }

    fn as_i16(&self) -> i16 {
        match self {
            AuthMethod::None => 0,
            AuthMethod::Password => 1,
        }
    }
}

pub enum VerifyMethod {
    None,
    Totp
}

impl VerifyMethod {
    fn from_i16(v: i16) -> Option<Self> {
        match v {
            0 => Some(VerifyMethod::None),
            1 => Some(VerifyMethod::Totp),
            _ => None
        }
    }

    fn as_i16(&self) -> i16 {
        match self {
            VerifyMethod::None => 0,
            VerifyMethod::Totp => 1,
        }
    }
}

pub enum BuilderError {
    TokenExists,
    InvalidExpires,
    UtcOverflow,

    Pg(PgError),
    Rand(rand::Error),
}

impl From<PgError> for BuilderError {
    fn from(err: PgError) -> Self {
        BuilderError::Pg(err)
    }
}

impl From<rand::Error> for BuilderError {
    fn from(err: rand::Error) -> Self {
        BuilderError::Rand(err)
    }
}

impl From<token::UniqueError> for BuilderError {
    fn from(err: token::UniqueError) -> Self {
        match err {
            token::UniqueError::Rand(err) => BuilderError::Rand(err),
            token::UniqueError::Pg(err) => BuilderError::Pg(err)
        }
    }
}

impl From<BuilderError> for NetError {
    fn from(err: BuilderError) -> NetError {
        match err {
            BuilderError::TokenExists => NetError::new()
                .source("failed to generate a unique token for the session"),
            BuilderError::InvalidExpires => NetError::new()
                .source("expires date is before the issued_on date"),
            BuilderError::UtcOverflow => NetError::new()
                .source("date time value overflowed"),
            BuilderError::Pg(err) => err.into(),
            BuilderError::Rand(err) => err.into(),
        }
    }
}

pub struct SessionBuilder {
    user_id: ids::UserId,
    token: Option<token::SessionToken>,
    issued_on: Option<DateTime<Utc>>,
    expires: Option<DateTime<Utc>>,
    authenticated: Option<bool>,
    verified: Option<bool>,
    auth_method: Option<AuthMethod>,
    verify_method: Option<VerifyMethod>
}

impl SessionBuilder {
    pub fn token(&mut self, token: token::SessionToken) -> &mut Self {
        self.token = Some(token);
        self
    }

    pub fn issued_on(&mut self, date: DateTime<Utc>) -> &mut Self {
        self.issued_on = Some(date);
        self
    }

    pub fn expires(&mut self, date: DateTime<Utc>) -> &mut Self {
        self.expires = Some(date);
        self
    }

    pub fn authenticated(&mut self, value: bool) -> &mut Self {
        self.authenticated = Some(value);
        self
    }

    pub fn verified(&mut self, value: bool) -> &mut Self {
        self.verified = Some(value);
        self
    }

    pub fn auth_method(&mut self, method: AuthMethod) -> &mut Self {
        self.auth_method = Some(method);
        self
    }

    pub fn verify_method(&mut self, method: VerifyMethod) -> &mut Self {
        self.verify_method = Some(method);
        self
    }

    pub async fn build(self, conn: &impl GenericClient) -> Result<Session, BuilderError> {
        let user_id = self.user_id;
        let dropped = false;
        let issued_on = self.issued_on.unwrap_or(Utc::now());
        let token = if let Some(token) = self.token {
            if !token::exists_check(conn, &token).await? {
                return Err(BuilderError::TokenExists);
            }

            token
        } else {
            token::SessionToken::unique(conn, 10).await?.unwrap()
        };
        let expires = if let Some(expires) = self.expires {
            if expires <= issued_on {
                return Err(BuilderError::InvalidExpires);
            }

            expires
        } else {
            let duration = chrono::Duration::days(7);

            let Some(expires) = issued_on.clone().checked_add_signed(duration) else {
                return Err(BuilderError::UtcOverflow);
            };

            expires
        };

        let authenticated_calc;
        let verified_calc;
        let auth_method = if let Some(method) = self.auth_method {
            match method {
                AuthMethod::None => {
                    authenticated_calc = true;
                }
                _ => {
                    authenticated_calc = false;
                }
            }

            method
        } else {
            authenticated_calc = true;
            AuthMethod::None
        };
        let verify_method = if let Some(method) = self.verify_method {
            match method {
                VerifyMethod::None => {
                    verified_calc = true;
                }
                _ => {
                    verified_calc = false;
                }
            }

            method
        } else {
            verified_calc = true;
            VerifyMethod::None
        };
        let authenticated = self.authenticated.unwrap_or(authenticated_calc);
        let verified = self.verified.unwrap_or(verified_calc);

        {
            let auth_method_int = auth_method.as_i16();
            let verify_method_int = verify_method.as_i16();

            let _ = conn.execute(
                "\
                insert into auth_session (\
                    token, \
                    user_id, \
                    dropped, \
                    issued_on, \
                    expires, \
                    authenticated, \
                    verified, \
                    auth_method, \
                    verify_method\
                ) values \
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[
                    &token.as_slice(),
                    &user_id,
                    &dropped,
                    &issued_on,
                    &expires,
                    &authenticated,
                    &verified,
                    &auth_method_int,
                    &verify_method_int,
                ]
            ).await?;
        }

        Ok(Session {
            token,
            user_id,
            dropped: false,
            issued_on,
            expires,
            authenticated,
            verified,
            auth_method,
            verify_method
        })
    }
}


pub struct Session {
    pub token: token::SessionToken,
    pub user_id: ids::UserId,
    pub dropped: bool,
    pub issued_on: chrono::DateTime<chrono::Utc>,
    pub expires: chrono::DateTime<chrono::Utc>,
    pub authenticated: bool,
    pub verified: bool,
    pub auth_method: AuthMethod,
    pub verify_method: VerifyMethod,
}

impl Session {
    pub fn builder(user_id: ids::UserId) -> SessionBuilder {
        SessionBuilder {
            user_id,
            token: None,
            issued_on: None,
            expires: None,
            authenticated: None,
            verified: None,
            auth_method: None,
            verify_method: None,
        }
    }

    pub async fn retrieve_token(
        conn: &impl GenericClient, 
        token: &token::SessionToken
    ) -> Result<Option<Session>, PgError> {
        if let Some(row) = conn.query_opt(
            "\
            select auth_session.token, \
                   auth_session.user_id, \
                   auth_session.dropped, \
                   auth_session.issued_on, \
                   auth_session.expires, \
                   auth_session.authenticated, \
                   auth_session.verified, \
                   auth_session.auth_method, \
                   auth_session.verify_method \
            from auth_session \
            where auth_session.token = $1",
            &[&token.as_slice()]
        ).await? {
            Ok(Some(Session {
                token: token::SessionToken::from_vec(row.get(0)),
                user_id: row.get(1),
                dropped: row.get(2),
                issued_on: row.get(3),
                expires: row.get(4),
                authenticated: row.get(5),
                verified: row.get(6),
                auth_method: AuthMethod::from_i16(row.get(7))
                    .expect("invalid auth method returned from database for session"),
                verify_method: VerifyMethod::from_i16(row.get(8))
                    .expect("invalid verify method returned from database for session"),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update(&self, conn: &impl GenericClient) -> Result<(), PgError> {
        let auth_method = self.auth_method.as_i16();
        let verify_method = self.verify_method.as_i16();

        let _ = conn.execute(
            "\
            update auth_session
            set user_id = $2, \
                dropped = $3, \
                issued_on = $4, \
                expires = $5, \
                authenticated = $6, \
                verified = $7, \
                auth_method = $8, \
                verify_method = $9 \
            where token = $1",
            &[
                &self.token.as_slice(),
                &self.user_id,
                &self.dropped,
                &self.issued_on,
                &self.expires,
                &self.authenticated,
                &self.verified,
                &auth_method,
                &verify_method,
            ]
        ).await?;

        Ok(())
    }

    pub async fn delete(&self, conn: &impl GenericClient) -> Result<(), PgError> {
        let _ = conn.execute(
            "delete from auth_session where token = $1",
            &[&self.token.as_slice()]
        ).await?;

        Ok(())
    }
}

pub type Hash = blake3::Hash;

/*
type HS256 = Hmac<sha3::Sha3_256>;
type HS384 = Hmac<sha3::Sha3_384>;
type HS512 = Hmac<sha3::Sha3_512>;

#[derive(Clone)]
pub enum Hash {
    Blake3(blake3::Hash),
    HS256(CtOutput<HS256>),
    HS384(CtOutput<HS384>),
    HS512(CtOutput<HS512>),
}
*/

pub fn create_hash<T>(auth: &state::Sec, token: T) -> Option<Hash>
where
    T: AsRef<[u8]>
{
    let Some(reader) = auth.session_info().keys().inner().read().ok() else {
        return None;
    };

    if let Some(latest) = reader.newest() {
        Some(blake3::keyed_hash(latest.data(), token.as_ref()))
    } else {
        Some(blake3::hash(token.as_ref()))
    }
}

/*
pub fn create_hash<T>(auth: &state::Sec, token: T) -> Hash
where
    T: AsRef<[u8]>
{
    match auth.session_info().key() {
        state::SessionKey::Blake3(key) => {
            Hash::Blake3(blake3::keyed_hash(key, token.as_ref()))
        },
        state::SessionKey::HS256(key) => {
            let mut mac = HS256::new_from_slice(key).unwrap();
            mac.update(token.as_ref());

            Hash::HS256(mac.finalize())
        },
        state::SessionKey::HS384(key) => {
            let mut mac = HS384::new_from_slice(key).unwrap();
            mac.update(token.as_ref());

            Hash::HS384(mac.finalize())
        },
        state::SessionKey::HS512(key) => {
            let mut mac = HS512::new_from_slice(key).unwrap();
            mac.update(token.as_ref());

            Hash::HS512(mac.finalize())
        }
    }
}
*/

pub fn encode_base64<T>(token: T, hash: Hash) -> String
where
    T: AsRef<[u8]>
{
    let token_ref = token.as_ref();

    let slice = hash.as_bytes();

    let mut joined = Vec::with_capacity(token_ref.len() + slice.len());
    joined.extend_from_slice(token_ref);
    joined.extend_from_slice(slice);

    URL_SAFE.encode(joined)
}

/*
pub fn encode_base64<T>(token: T, hash: Hash) -> String 
where
    T: AsRef<[u8]>
{
    let token_ref = token.as_ref();

    let bytes = match hash {
        Hash::Blake3(hash) => {
            let slice = hash.as_bytes();

            let mut joined = Vec::with_capacity(token_ref.len() + slice.len());
            joined.extend_from_slice(token_ref);
            joined.extend_from_slice(slice);
            joined
        },
        Hash::HS256(hash) => {
            let bytes = hash.into_bytes();
            let slice = bytes.as_slice();

            let mut joined = Vec::with_capacity(token_ref.len() + slice.len());
            joined.extend_from_slice(token_ref);
            joined.extend_from_slice(slice);
            joined
        },
        Hash::HS384(hash) => {
            let bytes = hash.into_bytes();
            let slice = bytes.as_slice();

            let mut joined = Vec::with_capacity(token_ref.len() + slice.len());
            joined.extend_from_slice(token_ref);
            joined.extend_from_slice(slice);
            joined
        },
        Hash::HS512(hash) => {
            let bytes = hash.into_bytes();
            let slice = bytes.as_slice();

            let mut joined = Vec::with_capacity(token_ref.len() + slice.len());
            joined.extend_from_slice(token_ref);
            joined.extend_from_slice(slice);
            joined
        }
    };

    URL_SAFE.encode(bytes)
}
*/

#[derive(Debug)]
pub enum DecodeError {
    InvalidString,
    InvalidLength,
    InvalidHash,
    KeysPoisoned,
}

pub fn decode_base64<S>(
    auth: &state::Sec,
    session_id: S
) -> Result<(token::SessionToken, Hash), DecodeError>
where
    S: AsRef<[u8]>
{
    let Ok(mut bytes) = URL_SAFE.decode(session_id) else {
        return Err(DecodeError::InvalidString);
    };

    if bytes.len() != token::SESSION_ID_BYTES + blake3::OUT_LEN {
        return Err(DecodeError::InvalidLength);
    };

    let token = token::SessionToken::drain_vec(&mut bytes);
    let hash: [u8; blake3::OUT_LEN] = bytes.try_into()
        .expect("remaing bytes does not match expected length");
    let given = blake3::Hash::from(hash);

    {
        let keys = auth.session_info().keys().inner();
        let reader = keys.read()
            .map_err(|_| DecodeError::KeysPoisoned)?;

        for key in reader.iter() {
            let expected = blake3::keyed_hash(key.data(), token.as_slice());

            if given == expected {
                return Ok((token, given));
            }
        }
    }

    let empty_key = [0; 32];
    let expected = blake3::keyed_hash(&empty_key, token.as_slice());

    if given != expected {
        Err(DecodeError::InvalidHash)
    } else {
        Ok((token, given))
    }
}

/*
pub fn decode_base64<S>(auth: &state::Sec, session_id: S) -> Result<(token::SessionToken, Hash), DecodeError>
where
    S: AsRef<[u8]>
{
    let mut bytes = match URL_SAFE.decode(session_id) {
        Ok(b) => b,
        Err(_) => {
            return Err(DecodeError::InvalidString);
        }
    };

    match auth.session_info().key() {
        state::SessionKey::Blake3(key) => {
            if bytes.len() != token::SESSION_ID_BYTES + blake3::OUT_LEN {
                return Err(DecodeError::InvalidLength);
            }

            let token = token::SessionToken::drain_vec(&mut bytes);
            let mut hash = [0; blake3::OUT_LEN];
            let mut index = 0;

            for b in bytes.drain(0..blake3::OUT_LEN) {
                hash[index] = b;
                index += 1;
            }

            let given = blake3::Hash::from(hash);
            let expected = blake3::keyed_hash(key, &token.as_slice());

            if given != expected {
                Err(DecodeError::InvalidHash)
            } else {
                Ok((token, Hash::Blake3(given)))
            }
        },
        state::SessionKey::HS256(key) => {
            if bytes.len() != token::SESSION_ID_BYTES + 32 {
                return Err(DecodeError::InvalidLength);
            }

            let token = token::SessionToken::drain_vec(&mut bytes);

            let mut mac = HS256::new_from_slice(key).unwrap();
            mac.update(&token.as_slice());

            let verify = mac.clone();

            if verify.verify_slice(&bytes).is_err() {
                Err(DecodeError::InvalidHash)
            } else {
                Ok((token, Hash::HS256(mac.finalize())))
            }
        },
        state::SessionKey::HS384(key) => {
            if bytes.len() != token::SESSION_ID_BYTES + 48 {
                return Err(DecodeError::InvalidLength);
            }

            let token = token::SessionToken::drain_vec(&mut bytes);
            let mut mac = HS384::new_from_slice(key).unwrap();
            mac.update(&token.as_slice());

            let verify = mac.clone();

            if verify.verify_slice(&bytes).is_err() {
                Err(DecodeError::InvalidHash)
            } else {
                Ok((token, Hash::HS384(mac.finalize())))
            }
        },
        state::SessionKey::HS512(key) => {
            if bytes.len() != token::SESSION_ID_BYTES + 64 {
                return Err(DecodeError::InvalidLength);
            }

            let token = token::SessionToken::drain_vec(&mut bytes);
            let mut mac = HS512::new_from_slice(key).unwrap();
            mac.update(&token.as_slice());

            let verify = mac.clone();

            if verify.verify_slice(&bytes).is_err() {
                Err(DecodeError::InvalidHash)
            } else {
                Ok((token, Hash::HS512(mac.finalize())))
            }
        }
    }
}
*/

pub fn create_session_cookie(auth: &state::Sec, session: &Session) -> Option<SetCookie> {
    let Some(hash) = create_hash(auth, &session.token) else {
        return None;
    };
    let encoded_token = encode_base64(&session.token, hash);

    let mut cookie = SetCookie::new("session_id", encoded_token)
        .with_expires(session.expires.clone())
        .with_path("/")
        .with_http_only(true)
        .with_secure(*auth.session_info().secure())
        .with_same_site(SameSite::Strict);

    if let Some(domain) = auth.session_info().domain() {
        cookie.set_domain(domain);
    }

    Some(cookie)
}

pub fn expire_session_cookie(auth: &state::Sec) -> SetCookie {
    let mut cookie = SetCookie::new("session_id", "")
        .with_max_age(std::time::Duration::new(0, 0))
        .with_path("/")
        .with_http_only(true)
        .with_secure(*auth.session_info().secure())
        .with_same_site(SameSite::Strict);

    if let Some(domain) = auth.session_info().domain() {
        cookie.set_domain(domain);
    }

    cookie
}

#[cfg(ignore)]
mod test {
    use super::*;
    use crate::sec::state;

    fn check_encode_decode(auth: state::Sec) {
        let bytes = [0; token::SESSION_ID_BYTES].to_owned();
        let token = token::SessionToken::from(bytes);
        let hash = create_hash(&auth, &token);

        let encode_string = encode_base64(&token, hash.clone());

        let (decode_token, decode_hash) = match decode_base64(&auth, &encode_string) {
            Ok(res) => res,
            Err(err) => {
                let bytes = URL_SAFE.decode(encode_string.as_bytes())
                    .expect("failed to decode original base64 encoded string");

                panic!("failed to decode token. len: {} bytes {:#?}", bytes.len(), bytes);
            }
        };

        assert_eq!(token, decode_token, "tokens do not match");

        match decode_hash {
            Hash::Blake3(d) => match hash {
                Hash::Blake3(g) => assert_eq!(d, g, "hashes do not match"),
                _ => panic!("hash mismatch")
            },
            Hash::HS256(d) => match hash {
                Hash::HS256(g) => {
                    if d != g {
                        panic!("hashes do not match");
                    }
                },
                _ => panic!("hash mismatch")
            },
            Hash::HS384(d) => match hash {
                Hash::HS384(g) => {
                    if d != g {
                        panic!("hashes do not match");
                    }
                },
                _ => panic!("hash mismatch")
            },
            Hash::HS512(d) => match hash {
                Hash::HS512(g) => {
                    if d != g {
                        panic!("hashes do not match");
                    }
                },
                _ => panic!("hash mismatch")
            }
        }
    }

    #[test]
    fn encode_decode_blake3() {
        let auth = state::Sec::builder()
            .with_session_hash(state::SessionHash::Blake3)
            .build()
            .expect("failed to create auth state");

        check_encode_decode(auth);
    }

    #[test]
    fn encode_decode_hs256() {
        let auth = state::Sec::builder()
            .with_session_hash(state::SessionHash::HS256)
            .build()
            .expect("failed to create auth state");

        check_encode_decode(auth);
    }

    #[test]
    fn encode_decode_hs384() {
        let auth = state::Sec::builder()
            .with_session_hash(state::SessionHash::HS384)
            .build()
            .expect("failed to create auth state");

        check_encode_decode(auth);
    }

    #[test]
    fn encode_decode_hs512() {
        let auth = state::Sec::builder()
            .with_session_hash(state::SessionHash::HS512)
            .build()
            .expect("failed to create auth state");

        check_encode_decode(auth);
    }
}
