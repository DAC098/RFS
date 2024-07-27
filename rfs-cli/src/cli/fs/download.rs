use std::path::PathBuf;

#[cfg(target_family = "unix")]
use std::os::unix::fs::FileTypeExt;

use rfs_lib::ids;
use rfs_api::client::ApiClient;
use rfs_api::client::fs::DownloadItem;
use clap::Args;
use reqwest::header::HeaderMap;

use crate::error::{self, Context};
use crate::util;
use crate::formatting::{
    self,
    OutputOptions,
    BaseSize,
};
use crate::path::{metadata, normalize_from};

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// the id of the item to retrieve
    #[arg(value_parser(util::parse_flake_id::<ids::FSId>))]
    id: ids::FSId,

    /// the output path for the file
    #[arg(short, long)]
    output: Option<PathBuf>,

    #[command(flatten)]
    format_options: OutputOptions
}

struct Pipe<'a, W1, W2> {
    writer: &'a mut W1,
    next: &'a mut W2,
}

impl<'a, W1, W2> Pipe<'a, W1, W2> {
    fn new(writer: &'a mut W1, next: &'a mut W2) -> Self {
        Pipe { writer, next }
    }
}

impl<'a, W1, W2> std::io::Write for Pipe<'a, W1, W2>
where
    W1: std::io::Write,
    W2: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)?;
        self.next.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()?;
        self.next.flush()?;

        Ok(())
    }
}

fn get_filename(headers: &HeaderMap) -> error::Result<String> {
    let Some(header_value) = headers.get("content-disposition") else {
        return Err("missing content-disposition".into());
    };

    let content_disposition = header_value.to_str()
        .context("content-disposition header contains invalid characters")?;

    let Some((context, attribute)) = content_disposition.split_once("; ") else {
        return Err("invalid content-disposition format".into());
    };

    if context != "attachment" {
        return Err("content-disposition context is not \"attachment\"".into());
    }

    let Some((attr, quoted)) = attribute.split_once("=") else {
        return Err("content-disposition attribute is invalid".into());
    };

    if attr != "filename" {
        return Err("content-disposition attribute is not \"filename\"".into());
    }

    let Some(prefix) = quoted.strip_prefix("\"") else {
        return Err("invalid format for filename attribute".into());
    };

    let Some(suffix) = prefix.strip_suffix("\"") else {
        return Err("invalid format for filename attribute".into());
    };

    Ok(suffix.to_owned())
}

fn get_checksum(headers: &HeaderMap) -> error::Result<blake3::Hash> {
    let Some(header_value) = headers.get("x-checksum") else {
        return Err("missing x-checksum header".into());
    };

    let x_checksum = header_value.to_str()
        .context("x-checksum header contains invalid characters")?;

    let Some((algo, hex)) = x_checksum.split_once(":") else {
        return Err("invalid x-checksum header format".into());
    };

    match algo {
        "blake3" => blake3::Hash::from_hex(hex)
            .context("failed to parse blake3 checksum"),
        _ => {
            return Err("unknown checksum algo from server".into());
        }
    }
}

fn resolve_file_path(given: Option<PathBuf>, filename: &str) -> error::Result<PathBuf> {
    let curr_dir = std::env::current_dir()
        .context("failed to retrieve current working directory")?;

    if let Some(given) = given {
        let mut resolved = normalize_from(&curr_dir, given);

        if let Some(metadata) = metadata(&resolved)
            .context("failed to resolve the output path")?
        {
            let file_type = metadata.file_type();

            if file_type.is_dir() {
                resolved.push(filename);

                Ok(resolved)
            } else if file_type.is_file() {
                Ok(resolved)
            } else {
                if cfg!(target_family = "unix") {
                    if file_type.is_fifo() || file_type.is_char_device() {
                        Ok(resolved)
                    } else {
                        Err("output path is not a file or directory".into())
                    }
                } else {
                    Err("output path is not a file or directory".into())
                }
            }
        } else {
            let parent = resolved.parent()
                .context("output path does not exist")?;

            let metadata = metadata(&parent)
                .context("failed to resolve output_path")?
                .context("output path does not exist")?;

            if !metadata.is_dir() {
                return Err("output path is not a directory".into());
            }

            Ok(resolved)
        }
    } else {
        Ok(curr_dir.join(filename))
    }
}

pub fn download(client: &ApiClient, mut args: DownloadArgs) -> error::Result {
    let mut response = DownloadItem::id(args.id.clone())
        .send(client)
        .context("failed download file")?;

    let headers = response.headers();
    let filename = get_filename(headers)?;
    let checksum = get_checksum(headers)?;

    let output_path = resolve_file_path(args.output.take(), &filename)?;

    let mut hasher = blake3::Hasher::new();
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&output_path)
        .context("failed to open output file")?;

    let mut pipe = Pipe::new(&mut hasher, &mut output);

    let start = std::time::Instant::now();

    let bytes_read = response.copy_to(&mut pipe)
        .context("error when reading response")? as u64;

    let duration = start.elapsed();

    {
        let hash = hasher.finalize();

        if checksum != hash {
            println!("WARNING: computed hash does not equal given checksum\nchecksum: {checksum}\n    hash: {hash}");
        }
    }

    let bits_read = (bytes_read * 8) as u128;

    let millis = duration.as_millis();

    let bits_per_sec = (bits_read / millis) * 1000;

    println!(
        "{} {duration:#?} {}",
        formatting::bytes_to_unit(bytes_read, &args.format_options.size_format),
        formatting::value_to_unit(bits_per_sec as u64, &BaseSize::Base10, "b/s"),
    );

    Ok(())
}
