use std::path::PathBuf;

use handlebars::Handlebars;

use crate::error;
use crate::fs;

fn get_registry_name<'a>(base: &PathBuf, path: &'a PathBuf, strip_ext: &str) -> error::Result<&'a str> {
    let stripped = path.strip_prefix(base)
        .unwrap()
        .to_str()
        .ok_or(error::Error::new()
            .kind("InvalidTemplateName")
            .message(format!("template file contains invalid UTF-8 characters. path: {}", path.display())))?;

    if let Some((name, _)) = stripped.rsplit_once(strip_ext) {
        Ok(name)
    } else {
        Ok(stripped)
    }
}

/// registers files for a handlebars registry
///
/// currently just loads everything as a template file but can be setup to
/// handle other files in the future
fn load_template_directory(registry: &mut Handlebars<'_>, directory: &PathBuf) -> error::Result<()> {
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
                    tracing::debug!("non handlebars file");
                }
            } else if entry_type.is_dir() {
                let entry_iter = read_dir(&entry_path)?;

                dir_queue.push((entry_path, entry_iter));
            } else {
                tracing::debug!(
                    path = %entry_path.display(),
                    "symlink"
                );
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct Builder {
    templates: Option<PathBuf>,
    dev_mode: bool,
}

impl Builder {
    pub fn set_templates<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>
    {
        self.templates = Some(path.into());
        self
    }

    pub fn set_dev_mode(&mut self, value: bool) -> &mut Self {
        self.dev_mode = value;
        self
    }

    pub fn build(self) -> error::Result<Templates> {
        let cwd = std::env::current_dir()?;

        let templates = fs::validate_dir(
            "templates",
            &cwd,
            self.templates.unwrap_or("templates".into())
        )?;

        let mut registry = Handlebars::new();
        registry.set_dev_mode(self.dev_mode);

        load_template_directory(&mut registry, &templates)?;

        Ok(Templates {
            registry: registry
        })
    }
}

#[derive(Debug)]
pub struct Templates {
    registry: Handlebars<'static>,
}

impl Templates {
    pub fn builder() -> Builder {
        Builder {
            templates: None,
            dev_mode: false,
        }
    }

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

    pub fn render<N, T>(&self, name: N, data: &T) -> Result<String, handlebars::RenderError>
    where
        N: AsRef<str>,
        T: serde::Serialize,
    {
        self.registry.render(name.as_ref(), data)
    }
}
