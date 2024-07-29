use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::Metadata;
use std::io::ErrorKind;
use std::path::{PathBuf, Path, Component};

use mime::Mime;
use lazy_static::lazy_static;

lazy_static! {
    static ref EXT_MIME_MAP: HashMap<&'static OsStr, &'static str> = {
        let mut m = HashMap::new();
        // image mime types
        m.insert(OsStr::new("jpg"), "image/jpeg");
        m.insert(OsStr::new("jpeg"), "image/jpeg");
        m.insert(OsStr::new("png"), "image/png");
        m.insert(OsStr::new("gif"), "image/gif");
        m.insert(OsStr::new("svg"), "image/svg+xml");
        m.insert(OsStr::new("webp"),"image/webp");
        m.insert(OsStr::new("ico"), "image/x-icon");

        // text mime types
        m.insert(OsStr::new("css"), "text/css");
        m.insert(OsStr::new("html"), "text/html");
        m.insert(OsStr::new("txt"), "text/plain");

        // application mime types
        m.insert(OsStr::new("js"), "application/javascript");
        m.insert(OsStr::new("json"), "application/json");
        m
    };
}

pub fn metadata<P>(path: P) -> Result<Option<Metadata>, std::io::Error>
where
    P: AsRef<Path>
{
    match path.as_ref().metadata() {
        Ok(m) => Ok(Some(m)),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Ok(None),
            _ => Err(err)
        }
    }
}

pub fn normalize<P>(path: P) -> PathBuf
where
    P: AsRef<Path>
{
    let components = path.as_ref().components();
    let mut rtn = PathBuf::new();

    for comp in components {
        match comp {
            Component::Prefix(prefix) => {
                rtn.push(prefix.as_os_str());
            }
            Component::ParentDir => {
                rtn.pop();
            }
            Component::Normal(c) => {
                rtn.push(c);
            }
            Component::RootDir => {
                rtn.push(comp.as_os_str());
            }
            Component::CurDir => {}
        }
    }

    rtn
}

pub fn mime_from_ext(ext: Option<&OsStr>) -> Mime {
    if let Some(ext) = ext {
        if let Some(mime_str) = EXT_MIME_MAP.get(ext) {
            (*mime_str).parse().unwrap()
        } else {
            mime::APPLICATION_OCTET_STREAM
        }
    } else {
        mime::APPLICATION_OCTET_STREAM
    }
}
