use std::path::{PathBuf, Path, Component};
use std::fs::Metadata;
use std::io::ErrorKind;

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
