use std::fmt;
use std::str::FromStr;

use mime::Mime;
use serde::de;
use serde::Deserialize;

pub fn nested_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: de::Deserializer<'de>,
    T: Deserialize<'de>,
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
