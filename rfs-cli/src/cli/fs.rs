use std::path::PathBuf;
use std::io::ErrorKind;
use std::ffi::OsStr;

use rfs_api::client::ApiClient;
use rfs_api::client::fs::{
    CreateDir,
    RetrieveItem,
    SendReadable,
    UpdateMetadata,
    DeleteItem,
};
use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::util;

#[derive(Debug, Args)]
pub struct FsArgs {
    #[command(subcommand)]
    command: FsCmds
}

#[derive(Debug, Subcommand)]
enum FsCmds {
    /// retrieves the desired fs item
    Get(GetArgs),

    /// creates a new fs item
    Create(CreateArgs),

    /// updates existing fs items with new data
    Update(UpdateArgs),

    /// uploads a file to the server
    Upload(UploadArgs),

    /// deletes the desired fs item
    Delete(DeleteArgs),
}

pub fn handle(client: &ApiClient, args: FsArgs) -> error::Result {
    match args.command {
        FsCmds::Get(given) => get(client, given),
        FsCmds::Create(given) => create(client, given),
        FsCmds::Update(given) => update(client, given),
        FsCmds::Upload(given) => upload(client, given),
        FsCmds::Delete(given) => delete(client, given),
    }
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

#[derive(Debug, Args)]
struct GetArgs {
    /// the id of the item to retrieve
    #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::FSId>))]
    id: rfs_lib::ids::FSId,
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    let result = RetrieveItem::id(args.id)
        .send(client)
        .context("failed to retrieve the fs item")?
        .context("desired fs item was not found")?
        .into_payload();

    println!("{result:#?}");

    Ok(())
}

#[derive(Debug, Args)]
struct CreateArgs {
    /// the parent fs item to create the new fs item under
    #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::FSId>))]
    parent: rfs_lib::ids::FSId,

    /// tags to apply to the fs item
    #[arg(
        short = 't',
        long = "tag",
        value_parser(util::parse_tag)
    )]
    tags: Vec<util::Tag>,

    /// comment to apply to the fs item
    #[arg(short, long)]
    comment: Option<String>,

    #[command(subcommand)]
    create_type: CreateType,
}

#[derive(Debug, Subcommand)]
enum CreateType {
    /// creates a directory
    Dir {
        /// basename of the new directory
        #[arg(short = 'n', long)]
        basename: String
    }
}

fn create(client: &ApiClient, args: CreateArgs) -> error::Result {
    match args.create_type {
        CreateType::Dir { basename } => {
            let mut builder = CreateDir::basename(args.parent, basename);

            if !args.tags.is_empty() {
                builder.add_iter_tags(args.tags);
            }

            if let Some(comment) = args.comment {
                builder.comment(comment);
            }

            let result = builder.send(client)
                .context("failed to create directory")?
                .into_payload();

            println!("{:?}", result);
        }
    }

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// the id of the fs item to update
    #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::FSId>))]
    id: rfs_lib::ids::FSId,

    #[command(flatten)]
    tags: Option<util::TagArgs>,

    /// updates the comment of the given fs item
    #[arg(long, conflicts_with("drop_comment"))]
    comment: Option<String>,

    /// removes the comment of the given fs item
    #[arg(long, conflicts_with("comment"))]
    drop_comment: bool
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result {
    let current = {
        let result = RetrieveItem::id(args.id.clone())
            .send(client)
            .context("failed to retrieve fs item")?;

        let Some(payload) = result else {
            println!("fs item not found");
            return Ok(());
        };

        payload.into_payload()
    };

    let mut builder = UpdateMetadata::id(args.id);

    if let Some(tags) = args.tags {
        let current_tags = match current {
            rfs_api::fs::Item::Root(root) => root.tags,
            rfs_api::fs::Item::Directory(dir) => dir.tags,
            rfs_api::fs::Item::File(file) => file.tags,
        };

        builder.add_iter_tags(tags.merge_existing(current_tags));
    }

    if let Some(comment) = args.comment {
        builder.comment(comment);
    } else if args.drop_comment {
        builder.comment(String::new());
    }

    let result = builder.send(client)
        .context("failed to update fs item")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct UploadArgs {
    /// path of the file to upload
    #[arg(long)]
    path: PathBuf,

    #[command(subcommand)]
    upload_type: UploadType
}

#[derive(Debug, Subcommand)]
enum UploadType {
    /// sends a new file to the server
    New {
        /// parent id to upload the fiel to
        #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::FSId>))]
        parent: rfs_lib::ids::FSId,

        /// basename of the fs item
        #[arg(short = 'n', long)]
        basename: Option<String>,

        /// manually specify the mime type
        #[arg(
            long,
            conflicts_with("fallback"),
            value_parser(util::parse_mime)
        )]
        mime: Option<mime::Mime>,

        /// fallback mime if one cannot be deduced
        #[arg(
            long,
            conflicts_with("mime"),
            value_parser(util::parse_mime)
        )]
        fallback: Option<mime::Mime>,
    },
    /// updates an existing file on the server
    Existing {
        /// id of fs item to update
        #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::FSId>))]
        id: rfs_lib::ids::FSId,

        /// manually specify the mime type
        #[arg(
            long,
            conflicts_with("fallback"),
            value_parser(util::parse_mime)
        )]
        mime: Option<mime::Mime>,

        /// fallback mime if one cannot be deduced
        #[arg(
            long,
            conflicts_with("mime"),
            value_parser(util::parse_mime)
        )]
        fallback: Option<mime::Mime>,
    }
}

fn upload(client: &ApiClient, args: UploadArgs) -> error::Result {
    let cwd = cwd()?;
    let file_path = canonicalize_file_path(args.path, &cwd)?;
    let metadata = path_metadata(&file_path)?;

    if !metadata.is_file() {
        return Err(error::Error::new()
            .context("requested file path is not a file"));
    }

    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(&file_path)
        .context("failed to open file")?;

    match args.upload_type {
        UploadType::New { parent, basename, mime, fallback } => {
            let basename = basename.unwrap_or(path_basename(&file_path)?
                .context("no basename was provided and the current file dod not contain a file name")?);

            let mut builder = SendReadable::create(parent, basename, file);
            builder.content_length(metadata.len());

            if let Some(given) = mime {
                builder.content_type(given);
            } else {
                if let Some(ext) = file_path.extension() {
                    builder.content_type(ext_mime(ext, fallback)?);
                } else if let Some(given) = fallback {
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
        UploadType::Existing { id, mime, fallback } => {
            let mut builder = SendReadable::update(id, file);
            builder.content_length(metadata.len());

            if let Some(given) = mime {
                builder.content_type(given);
            } else {
                if let Some(ext) = file_path.extension() {
                    builder.content_type(ext_mime(ext, fallback)?);
                } else if let Some(given) = fallback {
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
    }

    Ok(())
}

#[derive(Debug, Args)]
struct DeleteArgs {
    /// id of the fs item to delete
    #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::FSId>))]
    id: rfs_lib::ids::FSId,
}

fn delete(client: &ApiClient, args: DeleteArgs) -> error::Result {
    DeleteItem::id(args.id)
        .send(client)
        .context("failed to delete fs item")?;

    Ok(())
}
