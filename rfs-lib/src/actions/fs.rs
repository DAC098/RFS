use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMetadata {
    pub tags: Option<HashMap<String, Option<String>>>,
    pub comment: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDir {
    pub basename: String,
    pub tags: Option<HashMap<String, Option<String>>>,
    pub comment: Option<String>,
}

impl UpdateMetadata {
    pub fn has_work(&self) -> bool {
        self.tags.is_some() ||
            self.comment.is_some()
    }
}
