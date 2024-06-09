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

impl Node {
    pub fn as_local(&self) -> Option<&NodeLocal> {
        match &self {
            Node::Local(local) => Some(&local),
        }
    }
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

// ----------------------------------------------------------------------------

use crate::net;

pub enum Pair<'a, 'b> {
    Local((&'a ConfigLocal, &'b NodeLocal))
}

#[derive(Debug, thiserror::Error)]
#[error("provided backends do not match")]
pub struct MissMatched;

impl From<MissMatched> for net::error::Error {
    fn from(err: MissMatched) -> Self {
        net::error::Error::new()
            .source(err)
    }
}

impl<'a, 'b> Pair<'a, 'b> {
    pub fn match_up(config: &'a Config, node: &'b Node) -> Result<Self, MissMatched> {
        match config {
            Config::Local(conf_local) => {
                match node {
                    Node::Local(node_local) => Ok(Pair::Local((conf_local, node_local))),
                    // _ => Err(MissMatched)
                }
            }
        }
    }
}
