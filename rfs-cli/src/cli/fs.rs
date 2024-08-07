use std::path::PathBuf;
use std::ffi::OsStr;
use std::io::Seek;

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
use crate::formatting::{self, OutputOptions};
use crate::path::{normalize_from, metadata};

mod get;
mod download;
mod storage;

#[derive(Debug, Args)]
pub struct FsArgs {
    #[command(flatten)]
    get: get::GetArgs,

    #[command(subcommand)]
    command: Option<FsCmds>
}

#[derive(Debug, Subcommand)]
enum FsCmds {
    /// downloads the desired fs item
    Download(download::DownloadArgs),

    /// creates a new fs item
    Create(CreateArgs),

    /// updates existing fs items with new data
    Update(UpdateArgs),

    /// uploads a file to the server
    Upload(UploadArgs),

    /// deletes the desired fs item
    Delete(DeleteArgs),

    /// interacts with storage mediums on a server
    Storage(storage::StorageArgs),
}

pub fn handle(client: &ApiClient, args: FsArgs) -> error::Result {
    if let Some(cmd) = args.command {
        match cmd {
            FsCmds::Download(given) => download::download(client, given),
            FsCmds::Create(given) => create(client, given),
            FsCmds::Update(given) => update(client, given),
            FsCmds::Upload(given) => upload(client, given),
            FsCmds::Delete(given) => delete(client, given),
            FsCmds::Storage(given) => storage::handle(client, given),
        }
    } else {
        get::get(client, args.get)
    }
}

fn cwd() -> error::Result<PathBuf> {
    std::env::current_dir()
        .context("failed to retrieve the current working directory")
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
struct CreateArgs {
    /// the parent fs item to create the new fs item under
    parent: rfs_lib::ids::FSUid,

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

    #[command(flatten)]
    output_options: OutputOptions,

    #[command(subcommand)]
    create_type: CreateType,
}

#[derive(Debug, Subcommand)]
enum CreateType {
    /// creates a directory
    Dir {
        /// basename of the new directory
        basename: String
    }
}

fn create(client: &ApiClient, args: CreateArgs) -> error::Result {
    match args.create_type {
        CreateType::Dir { basename } => {
            let mut stdout = std::io::stdout();
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

            formatting::write_fs_item(&mut stdout, &result, &args.output_options)
                .context("failed to output to stdout")?;
        }
    }

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// the uid of the fs item to update
    uid: rfs_lib::ids::FSUid,

    #[command(flatten)]
    tags: Option<util::TagArgs>,

    /// updates the comment of the given fs item
    #[arg(long, conflicts_with("drop_comment"))]
    comment: Option<String>,

    /// removes the comment of the given fs item
    #[arg(long, conflicts_with("comment"))]
    drop_comment: bool,

    #[command(flatten)]
    output_options: OutputOptions,
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result {
    let current = {
        let result = RetrieveItem::uid(args.uid.clone())
            .send(client)
            .context("failed to retrieve fs item")?;

        let Some(payload) = result else {
            println!("fs item not found");
            return Ok(());
        };

        payload.into_payload()
    };

    let mut builder = UpdateMetadata::uid(args.uid);

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

    let mut stdout = std::io::stdout();

    formatting::write_fs_item(&mut stdout, &result, &args.output_options)
        .context("failed to output to stdout")?;

    Ok(())
}

#[derive(Debug, Args)]
struct UploadArgs {
    /// path of the file to upload
    path: PathBuf,

    /// upload a hash of the file to the server to validate against
    #[arg(long)]
    hash: bool,

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

    #[command(flatten)]
    output_options: OutputOptions,

    #[command(subcommand)]
    upload_type: UploadType
}

#[derive(Debug, Subcommand)]
enum UploadType {
    /// sends a new file to the server
    New {
        /// parent id to upload the fiel to
        parent: rfs_lib::ids::FSUid,

        /// basename of the fs item
        #[arg(short = 'n', long)]
        basename: Option<String>,
    },
    /// updates an existing file on the server
    Existing {
        /// id of fs item to update
        uid: rfs_lib::ids::FSUid,
    }
}

fn get_hash(file: &std::fs::File) -> error::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();

    hasher.update_reader(file)
        .context("error when creating hash of file")?;

    Ok(hasher.finalize())
}

fn upload(client: &ApiClient, args: UploadArgs) -> error::Result {
    let cwd = cwd()?;
    let file_path = normalize_from(&cwd, args.path);
    let metadata = metadata(&file_path)
        .context("failed to retrieve metadata for file")?
        .context("file not found")?;

    if !metadata.is_file() {
        return Err(error::Error::new()
            .context("requested file path is not a file"));
    }

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .open(&file_path)
        .context("failed to open file")?;

    let mut builder = match args.upload_type {
        UploadType::New { parent, basename } => {
            let basename = basename.unwrap_or(path_basename(&file_path)?
                .context("no basename was provided and the current file did not contain a file name")?);

            SendReadable::create(parent, basename)
        }
        UploadType::Existing { uid } => {
            SendReadable::update(uid)
        }
    };

    builder.content_length(metadata.len());

    if let Some(given) = args.mime {
        builder.content_type(given);
    } else {
        if let Some(ext) = file_path.extension() {
            builder.content_type(ext_mime(ext, args.fallback)?);
        } else if let Some(given) = args.fallback {
            builder.content_type(given);
        } else {
            builder.content_type(mime::APPLICATION_OCTET_STREAM);
        }
    }

    if args.hash {
        builder.hash("blake3", get_hash(&file)?.to_string());

        file.rewind()
            .context("failed to reset file cursor after hashing")?;
    }

    let result = builder.send(client, file)
        .context("failed to upload file")?
        .into_payload();

    let mut stdout = std::io::stdout();

    formatting::write_fs_item(&mut stdout, &result, &args.output_options)
        .context("failed to output to stdout")?;

    Ok(())
}

#[derive(Debug, Args)]
struct DeleteArgs {
    /// uid of the fs item to delete
    uid: rfs_lib::ids::FSUid,
}

fn delete(client: &ApiClient, args: DeleteArgs) -> error::Result {
    DeleteItem::uid(args.uid)
        .send(client)
        .context("failed to delete fs item")?;

    Ok(())
}
