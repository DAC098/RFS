use std::cmp::Ordering;

use rfs_lib::ids;
use rfs_api::fs::{Item, ItemMin};
use rfs_api::client::{ApiClient, iterate};
use rfs_api::client::fs::{
    RetrieveItem,
    RetrieveRoots,
    RetrieveContents,
};
use clap::Args;

use crate::error::{self, Context};
use crate::util;
use crate::formatting::{self, TextTable, Column, Float, PRETTY_OPTIONS};

#[derive(Debug, Args)]
pub struct GetArgs {
    /// retrieves the roots available
    #[arg(long, required_unless_present("id"))]
    roots: bool,

    /// the id of the item to retrieve
    #[arg(
        long,
        value_parser(util::parse_flake_id::<ids::FSId>),
        required_unless_present("roots")
    )]
    id: Option<ids::FSId>,

    /// will retrieve the contents of the specified file item
    #[arg(long)]
    contents: bool,

    /// specifies the format for the file size output
    #[arg(long, default_value_t)]
    size_format: formatting::BaseSize,

    /// specifies the format for the timestamp output
    #[arg(long, default_value_t)]
    ts_format: formatting::DateFormat,
}

fn sort_item(a: &ItemMin, b: &ItemMin) -> bool {
    match b {
        ItemMin::Root(root) => match a {
            ItemMin::Root(a_root) => root.id.id() < a_root.id.id(),
            ItemMin::Directory(_a_dir) => false,
            ItemMin::File(_a_file) => false,
        },
        ItemMin::Directory(dir) => match a {
            ItemMin::Root(_a_root) => true,
            ItemMin::Directory(a_dir) => match dir.basename.cmp(&a_dir.basename) {
                Ordering::Equal => dir.id.id() < a_dir.id.id(),
                Ordering::Less => true,
                Ordering::Greater => false,
            },
            ItemMin::File(a_file) => match dir.basename.cmp(&a_file.basename) {
                Ordering::Equal => dir.id.id() < a_file.id.id(),
                Ordering::Less => true,
                Ordering::Greater => false,
            },
        },
        ItemMin::File(file) => match a {
            ItemMin::Root(_a_root) => true,
            ItemMin::Directory(a_dir) => match file.basename.cmp(&a_dir.basename) {
                Ordering::Equal => file.id.id() < a_dir.id.id(),
                Ordering::Less => true,
                Ordering::Greater => false,
            },
            ItemMin::File(a_file) => match file.basename.cmp(&a_file.basename) {
                Ordering::Equal => file.id.id() < a_file.id.id(),
                Ordering::Less => true,
                Ordering::Greater => false,
            },
        },
    }
}

fn retrieve_id(client: &ApiClient, id: ids::FSId, args: GetArgs) -> error::Result {
    let mut ignore_contents = false;
    let size_format = args.size_format;
    let ts_format = args.ts_format;

    let result = RetrieveItem::id(id.clone())
        .send(client)
        .context("failed to retrieve the fs item")?
        .context("desired fs item was not found")?
        .into_payload();

    match result {
        Item::Root(root) => {
            println!("root {}", root.id.id());
            println!("created: {}", formatting::datetime_to_string(&root.created, &ts_format));

            if let Some(updated) = root.updated {
                println!("updated: {}", formatting::datetime_to_string(&updated, &ts_format));
            }

            if !root.tags.is_empty() {
                print!("{}", formatting::WriteTags::new(&root.tags));
            }

            if let Some(comment) = root.comment {
                println!("comment: {comment}");
            }
        }
        Item::Directory(dir) => {
            println!("directory {} {}/{}", dir.id.id(), dir.path.display(), dir.basename);
            println!("parent: {}", dir.parent.id());
            println!("created: {}", formatting::datetime_to_string(&dir.created, &ts_format));

            if let Some(updated) = dir.updated {
                println!("updated: {}", formatting::datetime_to_string(&updated, &ts_format));
            }

            if !dir.tags.is_empty() {
                print!("{}", formatting::WriteTags::new(&dir.tags));
            }

            if let Some(comment) = dir.comment {
                println!("comment: {comment}");
            }
        }
        Item::File(file) => {
            ignore_contents = true;

            println!(
                "file {} {}/{} {}",
                file.id.id(),
                file.path.display(),
                file.basename,
                formatting::bytes_to_unit(file.size, &size_format)
            );
            println!("parent: {}", file.parent.id());
            println!("created: {}", formatting::datetime_to_string(&file.created, &ts_format));

            if let Some(updated) = file.updated {
                println!("updated: {}", formatting::datetime_to_string(&updated, &ts_format));
            }

            println!("mime: {}", file.mime);
            println!("hash: {}", formatting::HexString::new(&file.hash));

            if !file.tags.is_empty() {
                print!("{}", formatting::WriteTags::new(&file.tags));
            }

            if let Some(comment) = file.comment {
                println!("comment: {comment}");
            }
        }
    }

    if args.contents && !ignore_contents {
        let mut builder = RetrieveContents::id(id);
        let mut table = TextTable::with_columns([
            Column::builder("type").build(),
            Column::builder("id").float(Float::Right).build(),
            Column::builder("size").float(Float::Right).build(),
            Column::builder("name").build(),
            Column::builder("mod").float(Float::Right).build(),
        ]);

        for result in iterate::Iterate::new(client, &mut builder) {
            let item = result.context("failed to retrieve fs item contents")?;
            let mut row = table.add_row();

            match &item {
                ItemMin::Root(root) => {
                    let time = root.updated.as_ref().unwrap_or(&root.created);

                    row.set_col(0, "root");
                    row.set_col(1, root.id.id());
                    row.set_col(4, formatting::datetime_to_string(&time, &ts_format));
                }
                ItemMin::Directory(dir) => {
                    let time = dir.updated.as_ref().unwrap_or(&dir.created);

                    row.set_col(0, "dir");
                    row.set_col(1, dir.id.id());
                    row.set_col(3, dir.basename.clone());
                    row.set_col(4, formatting::datetime_to_string(&time, &ts_format));
                }
                ItemMin::File(file) => {
                    let time = file.updated.as_ref().unwrap_or(&file.created);

                    row.set_col(0, "file");
                    row.set_col(1, file.id.id());
                    row.set_col(2, formatting::bytes_to_unit(file.size, &size_format));
                    row.set_col(3, file.basename.clone());
                    row.set_col(4, formatting::datetime_to_string(&time, &ts_format));
                }
            }

            row.finish_sort_by(item, sort_item);
        }

        if table.is_empty() {
            println!("no contents");
        } else {
            table.print(&PRETTY_OPTIONS)
                .context("failed to output results to stdout")?;
        }
    }

    Ok(())
}

fn retrieve_roots(client: &ApiClient, args: GetArgs) -> error::Result {
    let ts_format = args.ts_format;
    let mut builder = RetrieveRoots::new();
    let mut table = TextTable::with_columns([
        Column::builder("id").float(Float::Right).build(),
        Column::builder("name").build(),
        Column::builder("mod").float(Float::Right).build(),
    ]);

    for result in iterate::Iterate::new(client, &mut builder) {
        let item = result.context("failed to retrieve fs roots")?;
        let mut row = table.add_row();

        match &item {
            ItemMin::Root(root) => {
                let time = root.updated.as_ref().unwrap_or(&root.created);

                row.set_col(0, root.id.id());
                row.set_col(2, formatting::datetime_to_string(&time, &ts_format));
            }
            ItemMin::Directory(_dir) => {
                println!("unexpected fs item in result");
            }
            ItemMin::File(_file) => {
                println!("unexpected fs item in result");
            }
        }

        row.finish_sort_by(item, sort_item);
    }

    if table.is_empty() {
        println!("no roots");
    } else {
        table.print(&PRETTY_OPTIONS)
            .context("failed to output results to stdout")?;
    }

    Ok(())
}

pub fn get(client: &ApiClient, mut args: GetArgs) -> error::Result {
    if let Some(id) = args.id.take() {
        retrieve_id(client, id, args)
    } else {
        retrieve_roots(client, args)
    }
}
