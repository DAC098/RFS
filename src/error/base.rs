use std::default::Default;
use std::fmt::{Formatter, Display, Debug, Result as FmtResult};

pub type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct Er<I> {
    pub(super) inner: I,
    pub(super) cxt: Option<String>,
    pub(super) src: Option<BoxDynError>,
}

impl<I> Er<I> {
    pub fn context<C>(mut self, context: C) -> Self
    where
        C: Into<String>
    {
        self.cxt = Some(context.into());
        self
    }

    pub fn source<S>(mut self, source: S) -> Self
    where
        S: Into<BoxDynError>
    {
        self.src = Some(source.into());
        self
    }
}

impl<I> Default for Er<I>
where
    I: Default
{
    fn default() -> Self {
        Er {
            inner: Default::default(),
            cxt: None,
            src: None,
        }
    }
}

impl<I> Display for Er<I>
where
    I: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match (&self.inner, &self.cxt) {
            (inner, Some(cxt)) => write!(f, "{}\ncxt: {}", inner, cxt),
            (inner, None) => Display::fmt(inner, f)
        }
    }
}

impl<I> std::error::Error for Er<I>
where
    I: Debug + Display
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.src.as_ref().map(|v| & **v as _)
    }
}
