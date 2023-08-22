use axum::http::StatusCode;

#[derive(Debug)]
pub enum ErrorKind {
    FileExists,
    FileNotFound,
    DirNotFound,
    VersionExists,

    NotAFile,
    NotADirectory,
    NotManagerFile,
    NotKeyFile,

    InvalidFile,
    InvalidEncoding,
    UnknownFile,

    EncryptFailed,
    DecryptFailed,

    SerializeFailed,
    DeserializeFailed,

    Io,
    Rand,

    Poisoned,
    Timestamp,
}

impl ErrorKind {
    pub fn to_str(&self) -> &str {
        match self {
            ErrorKind::FileExists => "FileExits",
            ErrorKind::FileNotFound => "FileNotFound",
            ErrorKind::DirNotFound => "DirNotFound",

            ErrorKind::VersionExists => "VersionExits",

            ErrorKind::NotAFile => "NotAFile",
            ErrorKind::NotADirectory => "NotADirectory",
            ErrorKind::NotManagerFile => "NotManagerFile",
            ErrorKind::NotKeyFile => "NotKeyFile",

            ErrorKind::InvalidFile => "InvalidFile",
            ErrorKind::InvalidEncoding => "invalidEncoding",
            ErrorKind::UnknownFile => "UnknownFile",

            ErrorKind::EncryptFailed => "EncryptFailed",
            ErrorKind::DecryptFailed => "DecryptFailed",

            ErrorKind::SerializeFailed => "SerializeFailed",
            ErrorKind::DeserializeFailed => "DeserializeFailed",

            ErrorKind::Io => "Io",
            ErrorKind::Rand => "Rand",

            ErrorKind::Poisoned => "Poisoned",
            ErrorKind::Timestamp => "Timestamp",
        }

    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

pub type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: Option<String>,
    source: Option<BoxDynError>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Error {
            kind,
            message: None,
            source: None
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn with_kind(mut self, kind: ErrorKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
        self
    }

    pub fn with_source<S>(mut self, src: S) -> Self
    where
        S: Into<BoxDynError>
    {
        self.source = Some(src.into());
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.message, &self.source) {
            (Some(msg), Some(src)) => write!(f, "{}: {} -> {:?}", self.kind, msg, src),
            (Some(msg), None) => write!(f, "{}: {}", self.kind, msg),
            (None, Some(src)) => write!(f, "{}: {:?}", self.kind, src),
            (None, None) => write!(f, "{}", self.kind),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| &**e as _)
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(e: std::sync::PoisonError<T>) -> Self {
        Error::new(ErrorKind::Poisoned)
    }
}

impl From<Error> for crate::net::error::Error {
    fn from(v: Error) -> Self {
        crate::net::error::Error::new().source(v)
    }
}
