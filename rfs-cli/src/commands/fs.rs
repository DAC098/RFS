use std::path::PathBuf;
use std::str::FromStr;
use std::io::ErrorKind;
use std::ffi::OsStr;

use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error;
use crate::util;
use crate::state::AppState;

pub fn command() -> Command {
    Command::new("fs")
        .subcommand_required(true)
        .about("interacts with fs items on a server")
        .arg(util::default_help_arg())
        .subcommand(Command::new("create")
            .subcommand_required(true)
            .about("creates new fs items")
            .arg(Arg::new("parent")
                .long("parent")
                .value_parser(value_parser!(i64))
                .required(true)
                .help("the parent fs item to create the new fs item under")
                .long_help("the parent fs item to create the new fs item under")
            )
            .arg(Arg::new("tag")
                .short('t')
                .long("tag")
                .action(ArgAction::Append)
                .help("tags to apply to the fs item")
            )
            .arg(Arg::new("comment")
                .short('c')
                .long("comment")
                .help("comment to apply to the fs item")
            )
            .arg(util::default_help_arg())
            .subcommand(Command::new("dir")
                .about("creates a new directory at the given container")
                .arg(Arg::new("basename")
                    .short('n')
                    .long("basename")
                    .help("basename of the directory.")
                    .required(true)
                )
                .arg(util::default_help_arg())
            )
        )
        .subcommand(Command::new("update")
            .about("updates existing fs items with new data")
            .arg(Arg::new("id")
                .long("id")
                .value_parser(value_parser!(i64))
                .required(true)
                .help("the id of the fs item to update")
            )
            .arg(Arg::new("tag")
                .short('t')
                .long("tag")
                .action(ArgAction::Append)
                .help("tags to apply to the fs item. overrides currently set tags")
                .conflicts_with_all(["add-tag", "drop-tag"])
            )
            .arg(Arg::new("add-tag")
                .long("add-tag")
                .action(ArgAction::Append)
                .help("adds a tag to the given fs item")
                .conflicts_with("tag")
            )
            .arg(Arg::new("drop-tag")
                .long("drop-tag")
                .action(ArgAction::Append)
                .help("drops a tag from the given fs item")
                .conflicts_with("tag")
            )
            .arg(Arg::new("comment")
                .long("comment")
                .help("updates the comment of the given fs item")
                .conflicts_with("drop-comment")
            )
            .arg(Arg::new("drop-comment")
                .long("drop-comment")
                .action(ArgAction::SetTrue)
                .help("removes the comment of the given fs item")
                .conflicts_with("comment")
            )
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("upload")
            .about("uploads a file to the server")
            .arg(Arg::new("path")
                .long("path")
                .value_parser(value_parser!(PathBuf))
                .required(true)
                .help("the file path to upload")
            )
            .arg(util::default_help_arg())
            .subcommand(Command::new("new")
                .about("uploads a new file to the server")
                .arg(Arg::new("parent")
                    .long("parent")
                    .value_parser(value_parser!(i64))
                    .required(true)
                    .help("parent id to upload the file to")
                )
                .arg(Arg::new("basename")
                    .short('n')
                    .long("basename")
                    .help("basename of the fs item.")
                    .long_help("basename of the fs item. if one is not specified it will use the basename of the file")
                )
                .arg(Arg::new("mime")
                    .long("mime")
                    .value_parser(mime_value_parser)
                    .help("manually specify the mime type of the provided file")
                    .conflicts_with("fallback-mime")
                )
                .arg(Arg::new("fallback-mime")
                    .long("fallback-mime")
                    .value_parser(mime_value_parser)
                    .help("the fallback mime if one cannot be deduced from the file extension")
                    .conflicts_with("mime")
                )
                .arg(util::default_help_arg())
            )
            .subcommand(Command::new("existing")
                .about("uploads to an existing file on the server")
                .arg(Arg::new("id")
                    .long("id")
                    .value_parser(value_parser!(i64))
                    .required(true)
                    .help("the id of the fs item to update")
                )
                .arg(Arg::new("mime")
                    .long("mime")
                    .value_parser(mime_value_parser)
                    .help("manually specify the mime type of the provided file")
                    .conflicts_with("fallback-mime")
                )
                .arg(Arg::new("fallback-mime")
                    .long("fallback-mime")
                    .value_parser(mime_value_parser)
                    .help("the fallback mime if one cannot be deduced from the file extension")
                    .conflicts_with("mime")
                )
                .arg(util::default_help_arg())
            )
        )
}

fn cwd() -> error::Result<PathBuf> {
    std::env::current_dir().map_err(|e| error::Error::new()
        .kind("StdIoError")
        .message("failed to retrieve the current working directory")
        .source(e)
    )
}

fn canonicalize_file_path(file_path: PathBuf, cwd: &PathBuf) -> error::Result<PathBuf> {
    let rtn = if !file_path.is_absolute() {
        cwd.join(&file_path)
            .canonicalize()
            .map_err(|e| error::Error::new()
                .kind("StdIoError")
                .message("failed to canonicalize file path")
                .source(e))?
    } else {
        file_path
    };

    Ok(rtn)
}

fn path_metadata(file_path: &PathBuf) -> error::Result<std::fs::Metadata> {
    match file_path.metadata() {
        Ok(m) => Ok(m),
        Err(err) => {
            match err.kind() {
                ErrorKind::NotFound => {
                    Err(error::Error::new()
                        .kind("PathNotFound")
                        .message("requested file system item was not found"))
                },
                _ => {
                    Err(error::Error::new()
                        .kind("StdIoError")
                        .message("failed to read data about desired file system item")
                        .source(err))
                }
            }
        }
    }
}

fn mime_value_parser(value: &str) -> Result<mime::Mime, String> {
    match mime::Mime::from_str(value) {
        Ok(mime) => Ok(mime),
        Err(_err) => Err(format!("provided string is not a valid mime"))
    }
}

fn path_basename(fs_path: &PathBuf) -> error::Result<Option<String>> {
    let Some(file_name) = fs_path.file_name() else {
        return Ok(None)
    };

    let rtn = file_name.to_str()
        .ok_or(error::Error::new()
            .kind("InvalidUTF8Characters")
            .message("the provided file contains invalid utf-8 characters in the name"))?
        .to_owned();

    Ok(Some(rtn))
}

fn ext_mime(ext: &OsStr, fallback: Option<mime::Mime>) -> error::Result<mime::Mime> {
    let ext_str = ext.to_str().ok_or(error::Error::new()
        .kind("InvalidUTF8Characters")
        .message("the provided file extension contains invalid UTF-8 characters in the name"))?;

    let guess = mime_guess::MimeGuess::from_ext(ext_str);

    if let Some(fb) = fallback {
        Ok(guess.first().unwrap_or(fb))
    } else {
        Ok(guess.first_or_octet_stream())
    }
}

pub fn create(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    let parent = args.get_one::<i64>("parent").cloned().unwrap();
    let path = format!("/fs/{}", parent);

    let tags = util::tags_from_args("tag", args)?;
    let _comment = args.get_one::<String>("comment").cloned();

    match args.subcommand() {
        Some(("dir", dir_args)) => {
            let basename = dir_args.get_one::<String>("basename")
                .cloned()
                .unwrap();

            let tags = if tags.len() > 0 {
                Some(tags)
            } else {
                None
            };

            let action = rfs_lib::actions::fs::CreateDir {
                basename,
                tags,
                comment: args.get_one::<String>("comment").cloned()
            };

            let url = state.server.url.join(&path)?;
            let res = state.client.post(url)
                .json(&action)
                .send()?;

            let status = res.status();

            if status != reqwest::StatusCode::OK {
                let json = res.json::<rfs_lib::json::Error>()?;

                return Err(error::Error::new()
                    .kind("FailedCreatingDirectory")
                    .message("failed to create the new directory")
                    .source(format!("{:?}", json)));
            }

            let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::fs::Item>>()?;

            println!("{:?}", result.into_payload());
        },
        _ => unreachable!()
    }

    Ok(())
}

pub fn update(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    let id: i64 = args.get_one("id").cloned().unwrap();
    let path = format!("/fs/{}", id);

    let tags = util::tags_from_args("tag", args)?;
    let add_tags = util::tags_from_args("add-tag", args)?;
    let drop_tags = if let Some(given) = args.get_many::<String>("drop-tag") {
        given.collect()
    } else {
        Vec::new()
    };

    let current = {
        let url = state.server.url.join(&path)?;
        let res = state.client.get(url).send()?;

        let status = res.status();

        if status != reqwest::StatusCode::OK {
            let json = res.json::<rfs_lib::json::Error>()?;

            return Err(error::Error::new()
                .kind("FailedFsLookup")
                .message("failed to get the desired fs item")
                .source(format!("{:?}", json)));
        }

        let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::fs::Item>>()?;

        tracing::debug!(
            "currnet fs item: {:?}",
            result
        );

        result.into_payload()
    };

    let new_tags = {
        if tags.len() > 0 {
            Some(tags)
        } else if drop_tags.len() > 0 || add_tags.len() > 0 {
            let mut current_tags = match current {
                rfs_lib::schema::fs::Item::Root(root) => root.tags,
                rfs_lib::schema::fs::Item::Directory(dir) => dir.tags,
                rfs_lib::schema::fs::Item::File(file) => file.tags,
            };

            for tag in drop_tags {
                current_tags.remove(tag);
            }

            for (tag, value) in add_tags {
                current_tags.insert(tag, value);
            }

            Some(current_tags)
        } else {
            None
        }
    };

    let new_comment = if let Some(comment) = args.get_one("comment").cloned() {
        Some(comment)
    } else if args.get_flag("drop-comment") {
        Some(String::new())
    } else {
        None
    };

    let action = rfs_lib::actions::fs::UpdateMetadata {
        tags: new_tags,
        comment: new_comment
    };

    if !action.has_work() {
        println!("no changes have been specified");
        return Ok(());
    }

    tracing::debug!("sending patch request to server: {:?}", action);

    let url = state.server.url.join(&path)?;
    let res = state.client.patch(url)
        .json(&action)
        .send()?;

    tracing::debug!("response: {:#?}", res);

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedUpdateFs")
            .message("failed to update the fs item")
            .source(format!("{:?}", json)));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::fs::Item>>()?;

    println!("{:#?}", result);

    Ok(())
}

pub fn upload(state: &mut AppState, upload_args: &ArgMatches) -> error::Result<()> {
    let arg_path = upload_args.get_one("path")
        .cloned()
        .unwrap();
    let cwd = cwd()?;
    let file_path = canonicalize_file_path(arg_path, &cwd)?;
    let metadata = path_metadata(&file_path)?;

    if !metadata.is_file() {
        return Err(error::Error::new()
            .kind("NotAFile")
            .message("requested file path is not a file"));
    }

    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(&file_path)
        .map_err(|e| error::Error::new()
            .kind("StdIoError")
            .message("failed to open the desired file")
            .source(e))?;

    let res = match upload_args.subcommand() {
        Some(("new", new_args)) => {
            let parent: i64 = new_args.get_one("parent")
                .cloned()
                .unwrap();
            let url_path = format!("/fs/{}", parent);

            let basename = if let Some(given) = new_args.get_one("basename").cloned() {
                given
            } else {
                let Some(found) = path_basename(&file_path)? else {
                    return Err(error::Error::new()
                        .kind("NoBasenameProvided")
                        .message("no basename was provided and the current file did not contain a file name"));
                };

                found
            };

            let mime: mime::Mime = if let Some(given) = new_args.get_one("mime").cloned() {
                given
            } else {
                if let Some(ext) = file_path.extension() {
                    let fallback = new_args.get_one("fallback-mime").cloned();

                    ext_mime(ext, fallback)?
                } else if let Some(given) = new_args.get_one("fallback-mime").cloned() {
                    given
                } else {
                    mime::APPLICATION_OCTET_STREAM.clone()
                }
            };

            let url = state.server.url.join(&url_path)?;

            state.client.put(url)
                .header("x-basename", basename)
                .header("content-type", mime.as_ref())
                .header("content-length", metadata.len())
                .body(file)
                .send()?
        },
        Some(("existing", existing_args)) => {
            let id: i64 = existing_args.get_one("id")
                .cloned()
                .unwrap();
            let url_path = format!("/fs/{}", id);

            let mime: mime::Mime = if let Some(given) = existing_args.get_one("mime").cloned() {
                given
            } else {
                if let Some(ext) = file_path.extension() {
                    let fallback = existing_args.get_one("fallback-mime").cloned();

                    ext_mime(ext, fallback)?
                } else if let Some(given) = existing_args.get_one("fallback-mime").cloned() {
                    given
                } else {
                    mime::APPLICATION_OCTET_STREAM.clone()
                }
            };

            let url = state.server.url.join(&url_path)?;

            state.client.put(url)
                .header("content-type", mime.as_ref())
                .header("content-length", metadata.len())
                .body(file)
                .send()?
        },
        _ => unreachable!()
    };

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedFileUpload")
            .message("failed to upload the desired file the server")
            .source(format!("{:?}", json)));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::fs::Item>>()?;

    println!("{:#?}", result.into_payload());

    Ok(())
}
