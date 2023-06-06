type BoxDynError = Box<dyn std::error::Error>;

#[derive(Debug)]
pub struct Error {
    kind: String,
    msg: Option<String>,
    src: Option<BoxDynError>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new() -> Error {
        Error {
            kind: String::from("Error"),
            msg: None,
            src: None,
        }
    }

    pub fn kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.kind = kind.into();
        self
    }

    pub fn message<M>(mut self, msg: M) -> Error
    where
        M: Into<String>
    {
        self.msg = Some(msg.into());
        self
    }

    pub fn source<S>(mut self, src: S) -> Error
    where
        S: Into<BoxDynError>
    {
        self.src = Some(src.into());
        self
    }

    pub fn into_parts(self) -> (String, Option<String>, Option<BoxDynError>) {
        (self.kind, self.msg, self.src)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = self.msg.as_ref() {
            write!(f, "{}", msg)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.src.as_ref().map(|v| & **v as _)
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::new()
            .message(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::new()
            .message(msg)
    }
}

macro_rules! generic_catch {
    ($k:expr, $e:path) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .kind($k)
                    .source(err)
            }
        }
    };
    ($k:expr, $e:path, $m:expr) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .kind($k)
                    .message($m)
                    .source(err)
            }
        }
    }
}

generic_catch!("std::io::Error", std::io::Error);
generic_catch!("tokio_postgres::Error", tokio_postgres::Error);
