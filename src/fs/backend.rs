use std::path::PathBuf;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeLocal {
    pub path: PathBuf,
}

impl From<NodeLocal> for rfs_api::fs::backend::NodeLocal {
    fn from(local: NodeLocal) -> Self {
        rfs_api::fs::backend::NodeLocal {
            path: local.path
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
    Local(NodeLocal)
}

impl From<Node> for rfs_api::fs::backend::Node {
    fn from(node: Node) -> Self {
        match node {
            Node::Local(local) => rfs_api::fs::backend::Node::Local(local.into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigLocal {
    pub path: PathBuf,
}

impl From<ConfigLocal> for rfs_api::fs::backend::ConfigLocal {
    fn from(local: ConfigLocal) -> Self {
        rfs_api::fs::backend::ConfigLocal {
            path: local.path
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Config {
    Local(ConfigLocal)
}

impl From<Config> for rfs_api::fs::backend::Config {
    fn from(config: Config) -> Self {
        match config {
            Config::Local(local) => rfs_api::fs::backend::Config::Local(local.into()),
        }
    }
}
