pub struct HistoryField<T> {
    original: T,
    updated: Option<T>,
}

impl<T> HistoryField<T> {
    pub fn new(original: T) -> Self {
        HistoryField {
            original,
            updated: None
        }
    }

    pub fn with_updated(original: T, updated: T) -> Self {
        HistoryField {
            original,
            updated: Some(updated)
        }
    }

    pub fn from_tuple(tuple: (T, Option<T>)) -> Self {
        HistoryField {
            original: tuple.0,
            updated: tuple.1
        }
    }

    pub fn get(&self) -> &T {
        self.updated.as_ref().unwrap_or(&self.original)
    }

    pub fn set(&mut self, v: T) -> Option<T> {
        self.updated.replace(v)
    }

    pub fn original(&self) -> &T {
        &self.original
    }

    pub fn updated(&self) -> Option<&T> {
        self.updated.as_ref()
    }

    pub fn is_updated(&self) -> bool {
        self.updated.is_some()
    }

    pub fn rollback(&mut self) -> Option<T> {
        self.updated.take()
    }

    pub fn commit(&mut self) -> Option<T> {
        if let Some(v) = self.updated.take() {
            Some(std::mem::replace(&mut self.original, v))
        } else {
            None
        }
    }

    pub fn into_inner(self) -> T {
        self.updated.unwrap_or(self.original)
    }

    pub fn into_original(self) -> T {
        self.original
    }

    pub fn into_updated(self) -> Option<T> {
        self.updated
    }

    pub fn into_tuple(self) -> (T, Option<T>) {
        (self.original, self.updated)
    }
}

impl<T> std::default::Default for HistoryField<T>
where
    T: std::default::Default
{
    fn default() -> Self {
        HistoryField::new(T::default())
    }
}

impl HistoryField<String> {
    pub fn get_str(&self) -> &str {
        if let Some(v) = self.updated.as_ref() {
            v.as_str()
        } else {
            &self.original.as_str()
        }
    }
}

impl<T> Clone for HistoryField<T>
where
    T: Clone
{
    fn clone(&self) -> Self {
        HistoryField {
            original: self.original.clone(),
            updated: Option::clone(&self.updated)
        }
    }
}

impl<T> std::ops::Deref for HistoryField<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> AsRef<T> for HistoryField<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T> PartialEq<HistoryField<T>> for HistoryField<T>
where
    T: PartialEq<T>
{
    fn eq(&self, rhs: &HistoryField<T>) -> bool {
        self.get().eq(rhs.get())
    }
}

impl<T> PartialEq<T> for HistoryField<T>
where
    T: PartialEq<T>
{
    fn eq(&self, rhs: &T) -> bool {
        self.get().eq(rhs)
    }
}

impl<T> From<T> for HistoryField<T> {
    fn from(v: T) -> Self {
        HistoryField {
            original: v,
            updated: None
        }
    }
}

impl<T> From<(T, T)> for HistoryField<T> {
    fn from(tuple: (T, T)) -> Self {
        HistoryField {
            original: tuple.0,
            updated: Some(tuple.1)
        }
    }
}

impl<T> From<(T, Option<T>)> for HistoryField<T> {
    fn from(tuple: (T, Option<T>)) -> Self {
        HistoryField {
            original: tuple.0,
            updated: tuple.1,
        }
    }
}

// ----------------------------------------------------------------------------
// std From impl's
// ----------------------------------------------------------------------------

macro_rules! std_from {
    ($($e:path)*) => ($(
        impl From<HistoryField<$e>> for $e {
            fn from(v: HistoryField<$e>) -> $e {
                v.into_inner()
            }
        }
    )*);
}

std_from! {
    bool
    u8 u16 u32 u64 u128
    i8 i16 i32 i64 i128
    String char
    std::path::PathBuf
}

impl<'a> From<HistoryField<&'a str>> for &'a str {
    fn from(v: HistoryField<&'a str>) -> &'a str {
        v.into_inner()
    }
}

impl<T> From<HistoryField<Vec<T>>> for Vec<T> {
    fn from(v: HistoryField<Vec<T>>) -> Vec<T> {
        v.into_inner()
    }
}

