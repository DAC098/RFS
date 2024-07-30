use std::path::PathBuf;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigLocal {
    pub path: PathBuf
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Config {
    Local(ConfigLocal)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeLocal {
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
    Local(NodeLocal)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateConfig {
    Local {
        path: PathBuf
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UpdateConfig {
    Local {}
}
