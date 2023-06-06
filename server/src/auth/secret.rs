use std::collections::HashMap;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref EMPTY_SECRET: Secret = Secret::empty();
}

#[derive(Debug)]
pub struct Secret {
    version: u32,
    bytes: Vec<u8>
}

impl Secret {
    pub fn new(version: u32, bytes: Vec<u8>) -> Secret {
        Secret {
            version,
            bytes
        }
    }

    pub fn empty() -> Secret {
        Secret {
            version: 0,
            bytes: Vec::new(),
        }
    }

    pub fn version(&self) -> &u32 {
        &self.version
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes.as_slice()
    }
}

#[derive(Debug)]
pub struct Manager {
    ordering: Vec<u32>,
    map: HashMap<u32, Secret>,
}

impl Manager {
    pub fn new() -> Self {
        let mut rtn = Manager {
            ordering: Vec::new(),
            map: HashMap::new(),
        };

        rtn.add(Secret::empty());
        rtn
    }

    pub fn add(&mut self, secret: Secret) -> bool {
        let Err(pos) = self.ordering.binary_search(secret.version()) else {
            return false;
        };

        self.ordering.insert(pos, *secret.version());
        self.map.insert(*secret.version(), secret);

        true
    }

    pub fn latest(&self) -> &Secret {
        let ver = self.ordering.last().unwrap();

        self.map.get(ver).unwrap()
    }

    pub fn get(&self, version: &u32) -> Option<&Secret> {
        self.map.get(version)
    }
}
