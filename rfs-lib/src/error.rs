type BoxDynError = Box<dyn std::error::Error>;

pub struct CommonError<T = String> {
    kind: T,
    message: Option<String>,
    source: Option<BoxDynError>,
}

pub struct Result<T> = std::result::Result<T, CommonError>;

impl<T> CommonError<T> {
    pub fn new() -> Self 
    where
        T: Default
    {
        CommonError {
            kind: Default::default()
