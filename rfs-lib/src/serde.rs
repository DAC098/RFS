use std::fmt;
use std::str::FromStr;
use std::marker::PhantomData;
use core::convert::TryFrom;

use serde::{ser, de};

use mime::Mime;
use serde::Deserialize;

pub fn nested_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: de::Deserializer<'de>,
    T: de::Deserialize<'de>,
{
    Ok(Some(Deserialize::deserialize(deserializer)?))
}

struct MimeVisitor;

impl<'de> de::Visitor<'de> for MimeVisitor {
    type Value = Mime;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to parse into a valid Mime type")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Mime::from_str(s).map_err(|_| 
            E::invalid_value(de::Unexpected::Str(s), &self)
        )
    }
}

struct OptionMimeVisitor;

impl<'de> de::Visitor<'de> for OptionMimeVisitor {
    type Value = Option<Mime>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to parse into a valid Mime type")
    }

    fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>
    {
        d.deserialize_str(MimeVisitor).map(Some)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error
    {
        Ok(None)
    }
}

pub mod mime_str {
    use mime::Mime;
    use serde::{ser, de};

    use super::MimeVisitor;

    pub fn serialize<S>(mime: &Mime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer
    {
        serializer.serialize_str(mime.essence_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Mime, D::Error>
    where
        D: de::Deserializer<'de>
    {
        deserializer.deserialize_str(MimeVisitor)
    }
}

pub mod mime_opt_str {
    use mime::Mime;
    use serde::{ser, de};

    use super::OptionMimeVisitor;

    pub fn serialize<S>(mime: &Option<Mime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer
    {
        match mime {
            Some(ref v) => serializer.serialize_some(v.essence_str()),
            None => serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Mime>, D::Error>
    where
        D: de::Deserializer<'de>
    {
        deserializer.deserialize_option(OptionMimeVisitor)
    }
}

pub struct StringVisitor<F> {
    phantom: PhantomData<F>
}

impl<'de, F> de::Visitor<'de> for StringVisitor<F>
where
    F: FromStr
{
    type Value = F;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "non empty integer string within the valid range of the Id")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let Ok(num) = FromStr::from_str(s) else {
            return Err(E::invalid_value(de::Unexpected::Str(s), &self));
        };

        Ok(num)
    }
}

/*
pub struct OptionStringVisitor<F> {
    phantom: PhantomData<F>
}

impl<'de, F> de::Visitor<'de> for OptionStringVisitor<F>
where
    F: FromStr
{
    type Value = Option<F>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "non empty integer string with the valid range of the Id")
    }

    fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>
    {
        d.deserialize_str(StringVisitor {
            phantom: PhantomData
        }).map(Some)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error
    {
        Ok(None)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error
    {
        Ok(None)
    }
}
*/

pub mod from_to_str {
    use std::marker::PhantomData;
    use std::str::FromStr;

    use serde::{ser, de};

    use super::StringVisitor;

    /// serializes a given snowflake to a string
    pub fn serialize<F, S>(v: &F, serializer: S) -> Result<S::Ok, S::Error>
    where
        F: ToString,
        S: ser::Serializer
    {
        let v_str = v.to_string();

        serializer.serialize_str(v_str.as_str())
    }

    /// deserializes a given string to a snowflake
    pub fn deserialize<'de, F, D>(deserializer: D) -> Result<F, D::Error>
    where
        F: FromStr,
        D: de::Deserializer<'de>
    {
        deserializer.deserialize_str(StringVisitor {
            phantom: PhantomData
        })
    }
}

/*
pub mod pathbuf {
    use std::fmt;
    use std::path::PathBuf;
    use std::str::FromStr;

    use serde::{ser, de};

    pub fn serialize<S>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer
    {
        if let Some(s) = path.to_str() {
            serializer.serialize_str(s)
        } else {
            Err(ser::Error::custom("path contains invalid UTF-8 characters"))
        }
    }

    struct StringVisitor {}

    impl<'de> de::Visitor<'de> for StringVisitor {
        type Value = PathBuf;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "failed to parse string into valid PathBuf")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(PathBuf::from_str(s).unwrap())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
    where
        D: de::Deserializer<'de>
    {
        deserializer.deserialize_str(StringVisitor {})
    }
}
*/
