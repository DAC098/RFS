use std::path::PathBuf;
use std::str::FromStr;
use std::io::ErrorKind;

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
            .arg(util::default_help_arg())
            .arg(Arg::new("id")
                .long("id")
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
            .subcommand(Command::new("dir")
                .about("creates a new directory at the given container")
                .arg(Arg::new("basename")
                    .short('n')
                    .long("basename")
                    .help("basename of the fs item.")
                    .required(true)
                )
            )
            .subcommand(Command::new("file")
                .about("creates a new file at the given container")
                .arg(util::default_help_arg())
                .arg(Arg::new("path")
                    .long("path")
                    .value_parser(value_parser!(PathBuf))
                    .required(true)
                    .help("the file path to upload")
                )
                .arg(Arg::new("basename")
                    .short('n')
                    .long("basename")
                    .help("basename of the fs item.")
                    .long_help("basename of the fs item. if uploading a file then the basename of the file will be used if one is not provided")
                )
                .arg(Arg::new("mime")
                    .long("mime")
                    .help("manually specify the mime type of the provided file")
                )
                .arg(Arg::new("fallback-mime")
                    .long("fallback-mime")
                    .help("the fallback mime if one cannot be deduced from the file extension")
                )
            )
        )
        .subcommand(Command::new("update")
            .about("updates existing fs items with new data")
            .arg(util::default_help_arg())
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
        )
}

pub fn create(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    let id = args.get_one::<i64>("id").cloned().unwrap();
    let path = format!("/fs/{}", id);

    let tags = util::tags_from_args("tag", args)?;
    let comment = args.get_one::<String>().cloned();

    match args.subcommand() {
        Some(("dir", dir_args)) => {
            let Some(basename) = dir_args.get_one::<String>("basename").cloned() else {
                return Err(error::Error::new()
                    .kind("MissingBasename")
                    .message("basename is required when creating a directory"));
            };

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

            let res = state.client.post(state.server.url.join(&path)?)
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
        Some(("file", file_args)) => {
            let mut file_path = file_args.get_one::<PathBuf>("path").cloned().unwrap();

            if !file_path.is_absolute() {
                let mut cwd = std::env::current_dir()?;
                cwd.push(&file_path);

                file_path = cwd.canonicalize()?;
            }

            let metadata = match file_path.metadata() {
                Ok(m) => m,
                Err(err) => {
                    match err.kind() {
                        ErrorKind::NotFound => {
                            return Err(error::Error::new()
                                .kind("FileNotFound")
                                .message("requested file was not found"));
                        },
                        _ => {
                            return Err(error::Error::new()
                                .kind("StdIoError")
                                .message("failed to read data about desired file")
                                .source(err));
                        }
                    }
                }
            };

            if !metadata.is_file() {
                return Err(error::Error::new()
                    .kind("NotAFile")
                    .message("requested file path is not a file"));
            }

            let basename = if let Some(given) = file_args.get_one::<String>("basename").cloned() {
                given
            } else {
                let Some(file_name) = file_path.file_name() else {
                    return Err(error::Error::new()
                        .kind("NoFileNameProvided")
                        .message("no basename was provided and the current file did not contain a file name"));
                };

                file_name.to_str()
                    .ok_or(error::Error::new()
                        .kind("InvalidUTF8Characters")
                        .message("the provided file contains invalid utf-8 characters in the name"))?
                    .to_owned()
            };

            let mime = if let Some(given) = file_args.get_one::<String>("mime").cloned() {
                mime::Mime::from_str(&given).map_err(|e| error::Error::new()
                    .kind("InvalidMime")
                    .message("the provided mime type was not valid")
                    .source(e))?
            } else {
                if let Some(ext) = file_path.extension() {
                    let ext_str = ext.to_str().ok_or(error::Error::new()
                        .kind("InvalidUTF8Characters")
                        .message("the provided file contains invalid utf-8 characters in the name"))?;

                    let guess = mime_guess::MimeGuess::from_ext(ext_str);

                    if let Some(given) = file_args.get_one::<String>("fallback-mime").cloned() {
                        let fallback = mime::Mime::from_str(&given).map_err(|e| error::Error::new()
                            .kind("InvalidMime")
                            .message("the provided fallback mime type was not valid")
                            .source(e))?;

                        guess.first().unwrap_or(fallback)
                    } else {
                        guess.first_or_octet_stream()
                    }
                } else if let Some(given) = file_args.get_one::<String>("fallback-mime").cloned() {
                    mime::Mime::from_str(&given).map_err(|e| error::Error::new()
                        .kind("InvalidMime")
                        .message("the provided fallback mime type was not valid")
                        .source(e))?
                } else {
                    mime::APPLICATION_OCTET_STREAM.clone()
                }
            };

            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(&file_path)
                .map_err(|e| error::Error::new()
                    .kind("StdIoError")
                    .message("failed to open the desired file")
                    .source(e))?;
            let url = state.server.url.join(&path)?;
            let res = state.client.put(url)
                .header("x-basename", basename)
                .header("content-type", mime.as_ref())
                .header("content-length", metadata.len())
                .body(file)
                .send()?;

            let status = res.status();

            if status != reqwest::StatusCode::OK {
                let json = res.json::<rfs_lib::json::Error>()?;

                return Err(error::Error::new()
                    .kind("FailedFileUpload")
                    .message("failed to upload the desired file the server")
                    .source(format!("{:?}", json)));
            }

            let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::fs::Item>>()?;

            let action = rfs_lib::actions::fs::UpdateMetadata {
                tags,
                comment
            };

            if action.has_work() {
                let url = state.server.url.join(&path)?;
                let res = stats.client.patch(url)
                    .json(&action)
                    .send()?;

                let status = res.status();

                if status != reqwest::StatusCode::OK {
                    let json = res.json::<rfs_lib::json::Error>()?;

                    return Err(error::Error::new()
                        .kind("FailedUpdateFs")
                        .message("updated file to server buf failed to update metadata")
                        .source(format!("{:?}", json)));
                }

                let update_result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::fs::Item>>()?;

                println!("{:?}", update_result);
            } else {
                println!("{:?}", result);
            }
        },
        _ => unreachable!()
    }

    Ok(())
}

pub fn update(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    let id = args.get_one::<i64>("id").cloned().unwrap();
    let path = format!("/fs/{}", id);

    let tags = util::tags_from_args("tag", args)?;
    let add_tags = util::tags_from_args("add-tag", args)?;
    let drop_tags = if let Some(given) = args.get_many::<String>("drop-tag") {
        given.collect()
    } else {
        Vec::new()
    };

    let mut current = {
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

    let new_comment = if let Some(comment) = args.get_one::<String>("comment").cloned() {
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

    println!("{:?}", result);

    Ok(())
}
