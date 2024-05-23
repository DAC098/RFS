use rfs_lib::context_trait;

type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    context: Option<String>,
    src: Option<BoxDynError>,
}

pub type Result<T = ()> = std::result::Result<T, Error>;

impl Error {
    pub fn new() -> Error {
        Error {
            context: None,
            src: None,
        }
    }

    pub fn context<C>(mut self, cxt: C) -> Error
    where
        C: Into<String>
    {
        self.context = Some(cxt.into());
        self
    }

    pub fn source<S>(mut self, src: S) -> Error
    where
        S: Into<BoxDynError>
    {
        self.src = Some(src.into());
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.context, &self.src) {
            (Some(cxt), Some(src)) => write!(f, "{}: {:?}", cxt, src),
            (Some(cxt), None) => write!(f, "{}", cxt),
            (None, Some(src)) => write!(f, "{:?}", src),
            (None, None) => write!(f, "UNKNOWN ERROR"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.src.as_ref().map(|v| & **v as _)
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::new().context(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::new().context(msg)
    }
}

context_trait!(Error);

impl<T, E> Context<T, E> for std::result::Result<T, E>
where
    E: Into<BoxDynError>
{
    fn context<C>(self, cxt: C) -> std::result::Result<T, Error>
    where
        C: Into<String>
    {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(Error::new()
                .context(cxt)
                .source(err))
        }
    }
}

impl<T> Context<T, ()> for std::option::Option<T> {
    fn context<C>(self, cxt: C) -> std::result::Result<T, Error>
    where
        C: Into<String>
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Error::new().context(cxt))
        }
    }
}

macro_rules! simple_catch {
    ($e:path) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new().source(err)
            }
        }
    };
}

simple_catch!(std::io::Error);
simple_catch!(url::ParseError);
simple_catch!(reqwest::Error);
simple_catch!(rfs_api::ApiError);

impl From<rfs_api::client::error::RequestError> for Error {
    fn from(err: rfs_api::client::error::RequestError) -> Self {
        match err {
            rfs_api::client::error::RequestError::Reqwest(err) => Self::from(err)
                .context("error server request"),
            rfs_api::client::error::RequestError::Api(err) => Self::from(err)
        }
    }
}
