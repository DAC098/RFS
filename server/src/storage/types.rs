use std::path::{PathBuf, Path};

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Local {
    pub path: PathBuf,
}
