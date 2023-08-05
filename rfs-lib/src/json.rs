use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Wrapper<T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    payload: T
}

impl<T> Wrapper<T> {
    pub fn new(payload: T) -> Self {
        Self {
            kind: None,
            message: None,
            payload
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn kind(&self) -> Option<&String> {
        self.kind.as_ref()
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn set_message<M>(&mut self, msg: M) -> ()
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
        self
    }

    pub fn set_kind<K>(&mut self, kind: K) -> ()
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
    }

    pub fn with_kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
        self
    }

    pub fn with_payload<P>(self, payload: P) -> Wrapper<P> {
        Wrapper {
            kind: self.kind,
            message: self.message,
            payload
        }
    }

    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T> std::fmt::Display for Wrapper<T>
where
    T: std::fmt::Display
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match (&self.kind, &self.message) {
            (Some(kind), Some(message)) => {
                if f.alternate() {
                    write!(f, "{}: {} -> {:#}", kind, message, self.payload)
                } else {
                    write!(f, "{}: {} -> {}", kind, message, self.payload)
                }
            },
            (Some(kind), None) => {
                if f.alternate() {
                    write!(f, "{} -> {:#}", kind, self.payload)
                } else {
                    write!(f, "{} -> {}", kind, self.payload)
                }
            },
            (None, Some(message)) => {
                if f.alternate() {
                    write!(f, "{} -> {:#}", message, self.payload)
                } else {
                    write!(f, "{} -> {}", message, self.payload)
                }
            },
            (None, None) => {
                if f.alternate() {
                    write!(f, "{:#}", self.payload)
                } else {
                    write!(f, "{}", self.payload)
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListWrapper<T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    total: usize,
    payload: T,
}

impl<T> ListWrapper<T> {
    pub fn new(payload: T) -> Self {
        Self {
            kind: None,
            message: None,
            total: 0,
            payload
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn kind(&self) -> Option<&String> {
        self.kind.as_ref()
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn set_message<M>(&mut self, msg: M) -> ()
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>,
    {
        self.message = Some(msg.into());
        self
    }

    pub fn set_total(&mut self, total: usize) -> () {
        self.total = total;
    }

    pub fn with_total(mut self, total: usize) -> Self {
        self.total = total;
        self
    }

    pub fn set_kind<K>(&mut self, kind: K) -> ()
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
    }

    pub fn with_kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
        self
    }

    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T> ListWrapper<Vec<T>> {
    pub fn with_vec(vec: Vec<T>) -> Self {
        Self {
            kind: None,
            message: None,
            total: vec.len(),
            payload: vec
        }
    }

    pub fn with_slice(slice: &[T]) -> Self
    where
        T: Clone
    {
        Self {
            kind: None,
            message: None,
            total: slice.len(),
            payload: slice.to_vec(),
        }
    }
}

impl<T> std::fmt::Display for ListWrapper<T>
where
    T: std::fmt::Display
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match (&self.kind, &self.message) {
            (Some(kind), Some(message)) => {
                if f.alternate() {
                    write!(f, "{}: {} -> ({}) {:#}", kind, message, self.total, self.payload)
                } else {
                    write!(f, "{}: {} -> ({}) {}", kind, message, self.total, self.payload)
                }
            },
            (Some(kind), None) => {
                if f.alternate() {
                    write!(f, "{} -> ({}) {:#}", kind, self.total, self.payload)
                } else {
                    write!(f, "{} -> ({}) {}", kind, self.total, self.payload)
                }
            },
            (None, Some(message)) => {
                if f.alternate() {
                    write!(f, "{} -> ({}) {:#}", message, self.total, self.payload)
                } else {
                    write!(f, "{} -> ({}) {}", message, self.total, self.payload)
                }
            },
            (None, None) => {
                if f.alternate() {
                    write!(f, "({}) {:#}", self.total, self.payload)
                } else {
                    write!(f, "({}) {}", self.total, self.payload)
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    kind: String,
    message: Option<String>
}

impl Error {
    pub fn new<K>(kind: K) -> Self
    where
        K: Into<String>
    {
        Self {
            kind: kind.into(),
            message: None
        }
    }

    pub fn kind(&self) -> &String {
        &self.kind
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn set_message<M>(&mut self, message: M)
    where
        M: Into<String>
    {
        self.message = Some(message.into());
    }

    pub fn with_message<M>(mut self, message: M) -> Self
    where
        M: Into<String>
    {
        self.message = Some(message.into());
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(msg) = &self.message {
            write!(f, "{}: {}", self.kind, msg)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

impl std::error::Error for Error {}
