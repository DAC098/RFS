use crate::{ApiError, ApiErrorKind};

pub trait Validator {
    fn validate(&self) -> Result<(), ApiError> {
        Ok(())
    }

    fn has_work(&self) -> bool {
        true
    }

    fn assert_ok(&self) -> Result<(), ApiError> {
        self.validate()?;

        if !self.has_work() {
            Err(ApiError::from(ApiErrorKind::NoWork))
        } else {
            Ok(())
        }
    }
}
