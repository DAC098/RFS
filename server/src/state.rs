use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fmt;

use handlebars::{Handlebars, TemplateError};

use crate::error;

#[derive(Debug)]
pub enum Error {
    InvalidTemplateName(PathBuf),
    Io(std::io::Error),
    Handlebars(TemplateError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidTemplateName(path) => write!(f, "cannot create template name from path: {}", path.display()),
            Error::Io(err) => write!(f, "std::io::Error {:#?}", err),
            Error::Handlebars(err) => write!(f, "handlebars::TemplateError {}", err)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Handlebars(err) => Some(err),
            _ => None
        }
    }
}

impl From<TemplateError> for Error {
    fn from(err: TemplateError) -> Self {
        Error::Handlebars(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<Error> for error::Error {
    fn from(err: Error) -> error::Error {
        error::Error::new()
            .source(err)
    }
}

fn validate_dir<P>(name: &str, cwd: &PathBuf, path: P) -> std::io::Result<PathBuf>
where
    P: AsRef<Path>
{
    use std::io::{Error as IoError, ErrorKind};

    let path_ref = path.as_ref();

    let rtn = if !path_ref.is_absolute() {
        match std::fs::canonicalize(cwd.join(path_ref)) {
            Ok(p) => p,
            Err(err) => {
                return Err(match err.kind() {
                    ErrorKind::NotFound => {
                        let mut msg = String::new();
                        msg.push_str("given ");
                        msg.push_str(name);
                        msg.push_str(" does not exist");

                        IoError::new(ErrorKind::NotFound, msg)
                    },
                    _ => err
                });
            }
        }
    } else {
        path_ref.to_path_buf()
    };

    if !rtn.try_exists()? {
        let mut msg = String::new();
        msg.push_str("given ");
        msg.push_str(name);
        msg.push_str(" does not exist");

        return Err(IoError::new(ErrorKind::NotFound, msg));
    } else if !rtn.is_dir() {
        let mut msg = String::new();
        msg.push_str("given ");
        msg.push_str(name);
        msg.push_str(" is not a directory");

        return Err(IoError::new(ErrorKind::NotFound, msg));
    }

    Ok(rtn)
}

fn get_registry_name<'a>(base: &PathBuf, path: &'a PathBuf, strip_ext: &str) -> Result<&'a str, Error> {
    let stripped = path.strip_prefix(base)
        .unwrap()
        .to_str()
        .ok_or(Error::InvalidTemplateName(path.clone()))?;

    if let Some((name, _)) = stripped.rsplit_once(strip_ext) {
        Ok(name)
    } else {
        Ok(stripped)
    }
}

fn load_template_directory(registry: &mut Handlebars<'_>, directory: &PathBuf) -> Result<(), Error> {
    use std::fs::read_dir;

    let mut dir_queue = Vec::with_capacity(1);
    dir_queue.push((directory.clone(), read_dir(&directory)?));

    // breath first directory loading
    while let Some((path, iter)) = dir_queue.pop() {
        tracing::debug!(
            path = %path.display(),
            "loading directory",
        );

        for item in iter {
            let entry = item?;
            let entry_path = entry.path();
            let entry_type = entry.file_type()?;

            if entry_type.is_file() {
                let file_name = {
                    let Some(file_name) = entry_path.file_name() else {
                        continue;
                    };

                    file_name.to_string_lossy()
                };

                let mut name_parts = file_name.rsplit(".");

                let ext = name_parts.next().unwrap();

                if ext == "hbs" {
                    let name = get_registry_name(&directory, &entry_path, ".hbs")?;

                    /*
                    if let Some(specific) = name_parts.next() {
                        if specific == "partial" {
                            tracing::debug!(
                                name = name,
                                path = %entry_path.display(),
                                "handlebars partial template"
                            );
                            let contents = std::fs::read_to_string(&entry_path)?;
                            registry.register_partial(name, contents.as_str())?;
                            continue;
                        }
                    }
                    */

                    tracing::debug!(
                        name = name,
                        path = %entry_path.display(),
                        "handlebars template",
                    );

                    registry.register_template_file(name, &entry_path)?;
                } else {
                    tracing::debug!("non a handlebars file");
                }
            } else if entry_type.is_dir() {
                let entry_iter = read_dir(&entry_path)?;

                dir_queue.push((entry_path, entry_iter));
            } else {
                println!("{} symlink", entry_path.display());
            }
        }
    }

    Ok(())
}

pub struct SharedBuilder {
    assets: Option<PathBuf>,
    pages: Option<PathBuf>,
    templates: Option<PathBuf>,
    files_directory: Option<PathBuf>,
    hbs_dev_mode: bool,
}

impl SharedBuilder {
    pub fn with_assets<P>(&mut self, path: P) -> &mut Self 
    where
        P: Into<PathBuf>
    {
        self.assets = Some(path.into());
        self
    }

    pub fn with_pages<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>
    {
        self.pages = Some(path.into());
        self
    }

    pub fn with_templates<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>
    {
        self.templates = Some(path.into());
        self
    }

    pub fn with_files_directory<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>
    {
        self.files_directory = Some(path.into());
        self
    }

    pub fn set_hbs_dev_mode(&mut self, flag: bool) -> &mut Self {
        self.hbs_dev_mode = flag;
        self
    }

    pub fn build(self) -> Result<Shared, Error> {
        let cwd = std::env::current_dir()?;

        let assets = validate_dir(
            "assets",
            &cwd,
            self.assets.unwrap_or("assets".into())
        )?;

        let pages = validate_dir(
            "pages",
            &cwd,
            self.pages.unwrap_or("pages".into())
        )?;

        let templates = validate_dir(
            "templates",
            &cwd,
            self.templates.unwrap_or("templates".into())
        )?;

        let files_directory = validate_dir(
            "files_directory",
            &cwd,
            self.files_directory.unwrap_or("files_directory".into())
        )?;

        let mut registry = Handlebars::new();
        registry.set_dev_mode(self.hbs_dev_mode);

        load_template_directory(&mut registry, &templates)?;

        Ok(Shared {
            assets,
            pages,
            files_directory,
            templates: Templates {
                registry: Arc::new(registry)
            }
        })
    }
}

#[derive(Clone)]
pub struct Templates {
    registry: Arc<Handlebars<'static>>
}

impl Templates {
    #[allow(dead_code)]
    pub fn registry(&self) -> &Handlebars<'_> {
        &self.registry
    }

    pub fn has_template<N>(&self, name: N) -> bool
    where
        N: AsRef<str>
    {
        self.registry.has_template(name.as_ref())
    }

    pub fn render<N,T>(&self, name: N, data: &T) -> Result<String, handlebars::RenderError>
    where
        N: AsRef<str>,
        T: serde::Serialize,
    {
        self.registry.render(name.as_ref(), data)
    }
}


#[derive(Clone)]
pub struct Shared {
    assets: PathBuf,
    pages: PathBuf,
    files_directory: PathBuf,
    templates: Templates,
}

impl Shared {
    pub fn builder() -> SharedBuilder {
        SharedBuilder {
            assets: None,
            pages: None,
            templates: None,
            files_directory: None,
            hbs_dev_mode: false,
        }
    }

    pub fn assets(&self) -> &PathBuf {
        &self.assets
    }

    pub fn pages(&self) -> &PathBuf {
        &self.pages
    }

    pub fn templates(&self) -> &Templates {
        &self.templates
    }
}
