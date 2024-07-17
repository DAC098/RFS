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
use crate::formatting::{self, OutputOptions, TextTable, Column, Float, PRETTY_OPTIONS};

#[derive(Debug, Args)]
pub struct GetArgs {
    /// the id of the item to retrieve
    #[arg(
        long,
        value_parser(util::parse_flake_id::<ids::FSId>)
    )]
    id: Option<ids::FSId>,

    /// will not retrieve the contents of the specified file item
    #[arg(long)]
    no_contents: bool,

    #[command(flatten)]
    output_options: OutputOptions
}

fn sort_item(a: &ItemMin, b: &ItemMin) -> bool {
    match (a, b) {
        (ItemMin::Root(a_root), ItemMin::Root(b_root)) =>
            match a_root.basename.cmp(&b_root.basename) {
                Ordering::Equal => a_root.id.id() < b_root.id.id(),
                Ordering::Less => false,
                Ordering::Greater => true,
        }
        (ItemMin::Root(_a_root), ItemMin::Directory(_b_dir)) => false,
        (ItemMin::Root(_a_root), ItemMin::File(_b_file)) => false,
        (ItemMin::Directory(a_dir), ItemMin::Directory(b_dir)) =>
            match a_dir.basename.cmp(&b_dir.basename) {
                Ordering::Equal => a_dir.id.id() < b_dir.id.id(),
                Ordering::Less => false,
                Ordering::Greater => true,
            }
        (ItemMin::Directory(_a_dir), ItemMin::Root(_b_root)) => true,
        (ItemMin::Directory(_a_dir), ItemMin::File(_b_file)) => false,
        (ItemMin::File(a_file), ItemMin::File(b_file)) =>
            match a_file.basename.cmp(&b_file.basename) {
                Ordering::Equal => a_file.id.id() < b_file.id.id(),
                Ordering::Less => false,
                Ordering::Greater => true,
            }
        (ItemMin::File(_a_file), ItemMin::Root(_b_root)) => true,
        (ItemMin::File(_a_dir), ItemMin::Directory(_b_dir)) => true,
    }
}

fn retrieve_id(client: &ApiClient, id: ids::FSId, args: GetArgs) -> error::Result {
    let mut ignore_contents = false;
    let mut stdout = std::io::stdout();

    let result = RetrieveItem::id(id.clone())
        .send(client)
        .context("failed to retrieve the fs item")?
        .context("desired fs item was not found")?
        .into_payload();

    match result {
        Item::Root(root) => {
            formatting::write_fs_root(&mut stdout, &root, &args.output_options)
                .context("failed to output to stdout")?;
        }
        Item::Directory(dir) => {
            formatting::write_fs_dir(&mut stdout, &dir, &args.output_options)
                .context("failed to output to stdout")?;
        }
        Item::File(file) => {
            ignore_contents = true;

            formatting::write_fs_file(&mut stdout, &file, &args.output_options)
                .context("failed to output to stdout")?;
        }
    }

    if !args.no_contents && !ignore_contents {
        let mut builder = RetrieveContents::id(id);
        let mut table = TextTable::with_columns([
            Column::builder("type").build(),
            Column::builder("id").float(Float::Right).build(),
            Column::builder("storage id").float(Float::Right).build(),
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
                    row.set_col(2, root.storage_id.id());
                    row.set_col(4, root.basename.clone());
                    row.set_col(5, formatting::datetime_to_string(&time, &args.output_options.ts_format));
                }
                ItemMin::Directory(dir) => {
                    let time = dir.updated.as_ref().unwrap_or(&dir.created);

                    row.set_col(0, "dir");
                    row.set_col(1, dir.id.id());
                    row.set_col(2, dir.storage_id.id());
                    row.set_col(4, dir.basename.clone());
                    row.set_col(5, formatting::datetime_to_string(&time, &args.output_options.ts_format));
                }
                ItemMin::File(file) => {
                    let time = file.updated.as_ref().unwrap_or(&file.created);

                    row.set_col(0, "file");
                    row.set_col(1, file.id.id());
                    row.set_col(2, file.storage_id.id());
                    row.set_col(3, formatting::bytes_to_unit(file.size, &args.output_options.size_format));
                    row.set_col(4, file.basename.clone());
                    row.set_col(5, formatting::datetime_to_string(&time, &args.output_options.ts_format));
                }
            }

            row.finish_sort_by(item, sort_item);
        }

        if table.is_empty() {
            println!("no contents");
        } else {
            table.write(&mut stdout, &PRETTY_OPTIONS)
                .context("failed to output results to stdout")?;
        }
    }

    Ok(())
}

fn retrieve_roots(client: &ApiClient, args: GetArgs) -> error::Result {
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
                row.set_col(1, root.basename.clone());
                row.set_col(2, formatting::datetime_to_string(&time, &args.output_options.ts_format));
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
