use std::path::PathBuf;
use std::str::FromStr;
use std::io::ErrorKind;
use std::ffi::OsStr;

use rfs_api::client::ApiClient;
use rfs_api::client::fs::{
    CreateDir,
    RetrieveItem,
    SendReadable,
    UpdateMetadata,
};
use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error::{self, Context};
use crate::util;

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
    std::env::current_dir()
        .context("failed to retrieve the current working directory")
}

fn canonicalize_file_path(file_path: PathBuf, cwd: &PathBuf) -> error::Result<PathBuf> {
    let rtn = if !file_path.is_absolute() {
        cwd.join(&file_path)
            .canonicalize()
            .context("failed to canonicalize file path")?
    } else {
        file_path
    };

    Ok(rtn)
}

fn path_metadata(file_path: &PathBuf) -> error::Result<std::fs::Metadata> {
    match file_path.metadata() {
        Ok(m) => Ok(m),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                Err(error::Error::new()
                    .context("requested file system item was not found"))
            },
            _ => {
                Err(error::Error::new()
                    .context("failed to read data about desired file system item")
                    .source(err))
            }
        }
    }
}

fn mime_value_parser(value: &str) -> error::Result<mime::Mime> {
    mime::Mime::from_str(value)
        .context("provided string is not a valid mime")
}

fn path_basename(fs_path: &PathBuf) -> error::Result<Option<String>> {
    let Some(file_name) = fs_path.file_name() else {
        return Ok(None)
    };

    let rtn = file_name.to_str()
        .context("the provided file contains invalid utf-8 characters in the name")?
        .to_owned();

    Ok(Some(rtn))
}

fn ext_mime(ext: &OsStr, fallback: Option<mime::Mime>) -> error::Result<mime::Mime> {
    let ext_str = ext.to_str()
        .context("the provided file extension contains invalid utf-8 characters in the name")?;

    let guess = mime_guess::MimeGuess::from_ext(ext_str);

    if let Some(fb) = fallback {
        Ok(guess.first().unwrap_or(fb))
    } else {
        Ok(guess.first_or_octet_stream())
    }
}

pub fn create(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let parent = args.get_one::<i64>("parent")
        .unwrap()
        .try_into()
        .context("invalid fs id format")?;

    let tags = util::tags_from_args("tag", args)?;

    match args.subcommand() {
        Some(("dir", dir_args)) => {
            let basename = dir_args.get_one::<String>("basename")
                .unwrap();

            let mut builder = CreateDir::basename(parent, basename);

            if !tags.is_empty() {
                builder.add_iter_tags(tags);
            }

            if let Some(comment) = args.get_one::<String>("comment") {
                builder.comment(comment);
            }

            let result = builder.send(client)
                .context("failed to create directory")?
                .into_payload();

            println!("{:?}", result);
        },
        _ => unreachable!()
    }

    Ok(())
}

pub fn update(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id: rfs_lib::ids::FSId = args.get_one::<i64>("id")
        .unwrap()
        .try_into()
        .context("invalid fs id format")?;
    let tags = util::tags_from_args("tag", args)?;
    let add_tags = util::tags_from_args("add-tag", args)?;
    let drop_tags = if let Some(given) = args.get_many::<String>("drop-tag") {
        given.collect()
    } else {
        Vec::new()
    };

    let current = {
        let result = RetrieveItem::id(id.clone())
            .send(client)
            .context("failed to retrieve fs item")?;

        let Some(payload) = result else {
            println!("fs item not found");
            return Ok(());
        };

        payload.into_payload()
    };

    let mut builder = UpdateMetadata::id(id);

    if !tags.is_empty() {
        builder.add_iter_tags(tags);
    } else if drop_tags.len() > 0 || add_tags.len() > 0 {
        let mut current_tags = match current {
            rfs_api::fs::Item::Root(root) => root.tags,
            rfs_api::fs::Item::Directory(dir) => dir.tags,
            rfs_api::fs::Item::File(file) => file.tags,
        };

        for tag in drop_tags {
            current_tags.remove(tag);
        }

        for (tag, value) in add_tags {
            current_tags.insert(tag, value);
        }

        builder.add_iter_tags(current_tags);
    }

    if let Some(comment) = args.get_one::<String>("comment") {
        builder.comment(comment);
    } else if args.get_flag("drop-comment") {
        builder.comment(String::new());
    }

    let result = builder.send(client)
        .context("failed to update fs item")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

pub fn upload(client: &ApiClient, upload_args: &ArgMatches) -> error::Result {
    let arg_path = upload_args.get_one("path")
        .cloned()
        .unwrap();
    let cwd = cwd()?;
    let file_path = canonicalize_file_path(arg_path, &cwd)?;
    let metadata = path_metadata(&file_path)?;

    if !metadata.is_file() {
        return Err(error::Error::new()
            .context("requested file path is not a file"));
    }

    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(&file_path)
        .context("failed to open file")?;

    match upload_args.subcommand() {
        Some(("new", new_args)) => {
            let parent = new_args.get_one::<i64>("parent")
                .unwrap()
                .try_into()
                .context("invalid fs id format")?;

            let basename = if let Some(given) = new_args.get_one("basename").cloned() {
                given
            } else {
                path_basename(&file_path)?
                    .context("no basename was provided and the current file did not contain a file name")?
            };

            let mut builder = SendReadable::create(parent, basename, file);
            builder.content_length(metadata.len());

            if let Some(given) = new_args.get_one("mime").cloned() {
                builder.content_type(given);
            } else {
                if let Some(ext) = file_path.extension() {
                    let fallback = new_args.get_one("fallback-mime").cloned();

                    builder.content_type(ext_mime(ext, fallback)?);
                } else if let Some(given) = new_args.get_one("fallback-mime").cloned() {
                    builder.content_type(given);
                } else {
                    builder.content_type(mime::APPLICATION_OCTET_STREAM);
                }
            }

            let result = builder.send(client)
                .context("failed to upload file")?
                .into_payload();

            println!("{:#?}", result);
        },
        Some(("existing", existing_args)) => {
            let id = existing_args.get_one::<i64>("id")
                .unwrap()
                .try_into()
                .context("invalid fs id format")?;

            let mut builder = SendReadable::update(id, file);
            builder.content_length(metadata.len());

            if let Some(given) = existing_args.get_one("mime").cloned() {
                builder.content_type(given);
            } else {
                if let Some(ext) = file_path.extension() {
                    let fallback = existing_args.get_one("fallback-mime").cloned();

                    builder.content_type(ext_mime(ext, fallback)?);
                } else if let Some(given) = existing_args.get_one("fallback-mime").cloned() {
                    builder.content_type(given);
                } else {
                    builder.content_type(mime::APPLICATION_OCTET_STREAM);
                }
            }

            let result = builder.send(client)
                .context("failed to upload file")?
                .into_payload();

            println!("{:#?}", result);
        },
        _ => unreachable!()
    }

    Ok(())
}
