use rfs_lib::ids;
use rfs_api::fs::{Item, ItemMin};
use rfs_api::client::ApiClient;
use rfs_api::client::fs::{
    RetrieveItem,
    RetrieveRoots,
    RetrieveContents,
};
use clap::Args;

use crate::error::{self, Context};
use crate::util;
use crate::formatting::{self, Column, Float};

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

fn insert_item_min<T>(list: &mut Vec<(ItemMin, T)>, original: ItemMin, output: T) {
    let pos = list.partition_point(|(v, _)| {
        match v {
            ItemMin::Root(root) => {
                match &original {
                    ItemMin::Root(orig_root) => {
                        root.id.id() < orig_root.id.id()
                    },
                    ItemMin::Directory(_orig_dir) => {
                        false
                    },
                    ItemMin::File(_orig_file) => {
                        false
                    },
                }
            },
            ItemMin::Directory(dir) => {
                match &original {
                    ItemMin::Root(_orig_root) => {
                        true
                    },
                    ItemMin::Directory(orig_dir) => {
                        if dir.basename == orig_dir.basename {
                            dir.id.id() < orig_dir.id.id()
                        } else {
                            dir.basename < orig_dir.basename
                        }
                    },
                    ItemMin::File(orig_file) => {
                        if dir.basename == orig_file.basename {
                            dir.id.id() < orig_file.id.id()
                        } else {
                            dir.basename < orig_file.basename
                        }
                    },
                }
            },
            ItemMin::File(file) => {
                match &original {
                    ItemMin::Root(_orig_root) => {
                        true
                    },
                    ItemMin::Directory(orig_dir) => {
                        if file.basename == orig_dir.basename {
                            file.id.id() < orig_dir.id.id()
                        } else {
                            file.basename < orig_dir.basename
                        }
                    },
                    ItemMin::File(orig_file) => {
                        if file.basename == orig_file.basename {
                            file.id.id() < orig_file.id.id()
                        } else {
                            file.basename < orig_file.basename
                        }
                    },
                }
            },
        }
    });

    list.insert(pos, (original, output));
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
        let mut output_list: Vec<(ItemMin, [Option<String>; 5])> = Vec::new();
        let mut columns = [
            Column::builder("type").build(),
            Column::builder("id").float(Float::Right).build(),
            Column::builder("size").float(Float::Right).build(),
            Column::builder("name").build(),
            Column::builder("mod").float(Float::Right).build(),
        ];

        loop {
            let (_pagination, payload) = builder.send(client)
                .context("failed to retrieve fs item contents")?
                .into_tuple();

            let Some(last) = payload.last() else {
                break;
            };

            let last_id = match last {
                ItemMin::Root(root) => root.id.clone(),
                ItemMin::Directory(dir) => dir.id.clone(),
                ItemMin::File(file) => file.id.clone(),
            };

            builder.last_id(last_id);

            for item in payload {
                let mut output_item = std::array::from_fn(|_| None);

                match &item {
                    ItemMin::Root(root) => {
                        let time = root.updated.as_ref().unwrap_or(&root.created);

                        output_item[0] = Some("root".into());
                        output_item[1] = Some(root.id.id().to_string());
                        output_item[4] = Some(formatting::datetime_to_string(&time, &ts_format));
                    }
                    ItemMin::Directory(dir) => {
                        let time = dir.updated.as_ref().unwrap_or(&dir.created);

                        output_item[0] = Some("dir".into());
                        output_item[1] = Some(dir.id.id().to_string());
                        output_item[3] = Some(dir.basename.clone());
                        output_item[4] = Some(formatting::datetime_to_string(&time, &ts_format));
                    }
                    ItemMin::File(file) => {
                        let time = file.updated.as_ref().unwrap_or(&file.created);

                        output_item[0] = Some("file".into());
                        output_item[1] = Some(file.id.id().to_string());
                        output_item[2] = Some(formatting::bytes_to_unit(file.size, &size_format));
                        output_item[3] = Some(file.basename.clone());
                        output_item[4] = Some(formatting::datetime_to_string(&time, &ts_format));
                    }
                }

                for index in 0..columns.len() {
                    if let Some(st) = &output_item[index] {
                        let chars_count = st.chars().count();

                        columns[index].update_width(chars_count);
                    }
                }

                insert_item_min(&mut output_list, item, output_item);
            }
        }

        let empty = "";

        if output_list.is_empty() {
            println!("no contents");
        } else {
            let total = output_list.len();
            let index_width = (total.ilog10() + 1) as usize;

            println!("contents: {total}");

            print!("{:index_width$}", "");

            for col in &columns {
                print!(" ");

                col.print_header();
            }

            println!("");

            for (index, (_, item)) in output_list.iter().enumerate() {
                print!("{:>index_width$}", index + 1);

                for (value, col) in item.iter().zip(&columns) {
                    print!(" ");

                    if let Some(st) = value {
                        col.print_value(st);
                    } else {
                        col.print_value(&empty);
                    }
                }

                println!("");
            }
        }
    }

    Ok(())
}

fn retrieve_roots(client: &ApiClient, args: GetArgs) -> error::Result {
    let ts_format = args.ts_format;
    let mut builder = RetrieveRoots::new();
    let mut output_list: Vec<(ItemMin, [Option<String>; 4])> = Vec::new();
    let mut columns = [
        Column::builder("id").float(Float::Right).build(),
        Column::builder("name").build(),
        Column::builder("mod").float(Float::Right).build(),
    ];

    loop {
        let (_pagination, payload) = builder.send(client)
            .context("failed to retrieve fs roots")?
            .into_tuple();

        let Some(last) = payload.last() else {
            break;
        };

        let last_id = match last {
            ItemMin::Root(root) => root.id.clone(),
            ItemMin::Directory(dir) => dir.id.clone(),
            ItemMin::File(file) => file.id.clone(),
        };

        builder.last_id(last_id);

        for item in payload {
            let mut output_item = std::array::from_fn(|_| None);

            match &item {
                ItemMin::Root(root) => {
                    let time = root.updated.as_ref().unwrap_or(&root.created);

                    output_item[0] = Some(root.id.id().to_string());
                    output_item[2] = Some(formatting::datetime_to_string(&time, &ts_format));
                }
                ItemMin::Directory(_dir) => {
                    println!("unexpected fs item in result");
                }
                ItemMin::File(_file) => {
                    println!("unexpected fs item in result");
                }
            }

            for index in 0..columns.len() {
                if let Some(st) = &output_item[index] {
                    let chars_count = st.chars().count();

                    columns[index].update_width(chars_count);
                }
            }

            insert_item_min(&mut output_list, item, output_item);
        }
    }

    let empty = "";

    if output_list.is_empty() {
        println!("no roots");
    } else {
        let total = output_list.len();
        let index_width = (total.ilog10() + 1) as usize;

        println!("contents: {total}");

        print!("{:index_width$}", "");

        for col in &columns {
            print!(" ");

            col.print_header();
        }

        println!("");

        for (index, (_, item)) in output_list.iter().enumerate() {
            print!("{:>index_width$}", index + 1);

            for (value, col) in item.iter().zip(&columns) {
                print!(" ");

                if let Some(st) = value {
                    col.print_value(st);
                } else {
                    col.print_value(&empty);
                }
            }

            println!("");
        }
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
