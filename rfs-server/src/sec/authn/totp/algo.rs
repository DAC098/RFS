use std::str::FromStr;

#[derive(Clone)]
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

impl From<Algo> for rust_otp::Algo {
    fn from(algo: Algo) -> rust_otp::Algo {
        match algo {
            Algo::SHA1 => rust_otp::Algo::SHA1,
            Algo::SHA256 => rust_otp::Algo::SHA256,
            Algo::SHA512 => rust_otp::Algo::SHA512,
        }
    }
}

impl From<&Algo> for rust_otp::Algo {
    fn from(algo: &Algo) -> rust_otp::Algo {
        match algo {
            Algo::SHA1 => rust_otp::Algo::SHA1,
            Algo::SHA256 => rust_otp::Algo::SHA256,
            Algo::SHA512 => rust_otp::Algo::SHA512,
        }
    }
}
