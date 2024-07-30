use std::cmp::Ordering;
use std::collections::{HashMap, BinaryHeap};
use std::fmt::{Write, Formatter, Display, Result as FmtResult};
use std::default::Default;
use std::iter::Iterator;

use chrono::{DateTime, Utc, Local, SecondsFormat};
use clap::{Args, ValueEnum};

#[derive(Debug)]
pub struct SizeType<T = u64> {
    pub value: T,
    pub unit: &'static str,
}

pub const KIBI: u64 = 1024;
pub const MEBI: u64 = KIBI * 1024;
pub const GIBI: u64 = MEBI * 1024;
pub const TEBI: u64 = GIBI * 1024;
pub const PEBI: u64 = TEBI * 1024;
pub const EXBI: u64 = PEBI * 1024;

pub const BASE_2_SIZE: [SizeType; 6] = [
    SizeType {
        value: EXBI,
        unit: "Ei",
    },
    SizeType {
        value: PEBI,
        unit: "Pi",
    },
    SizeType {
        value: TEBI,
        unit: "Ti",
    },
    SizeType {
        value: GIBI,
        unit: "Gi",
    },
    SizeType {
        value: MEBI,
        unit: "Mi",
    },
    SizeType {
        value: KIBI,
        unit: "Ki",
    },
];

pub const KILO: u64 = 1000;
pub const MEGA: u64 = KILO * 1000;
pub const GIGA: u64 = MEGA * 1000;
pub const TERA: u64 = GIGA * 1000;
pub const PETA: u64 = TERA * 1000;
pub const EXA: u64 = TERA * 1000;

pub const BASE_10_SIZE: [SizeType; 6] = [
    SizeType {
        value: EXA,
        unit: "E",
    },
    SizeType {
        value: PETA,
        unit: "P",
    },
    SizeType {
        value: TERA,
        unit: "T",
    },
    SizeType {
        value: GIGA,
        unit: "G",
    },
    SizeType {
        value: MEGA,
        unit: "M",
    },
    SizeType {
        value: KILO,
        unit: "K",
    },
];

#[derive(Debug, Clone, ValueEnum)]
pub enum BaseSize {
    Base2,
    Base10,
    Raw,
}

impl Default for BaseSize {
    fn default() -> Self {
        BaseSize::Base2
    }
}

impl Display for BaseSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            BaseSize::Base2 => write!(f, "base2"),
            BaseSize::Base10 => write!(f, "base10"),
            BaseSize::Raw => write!(f, "raw"),
        }
    }
}

pub fn value_to_unit(size: u64, base: &BaseSize, suffix: &str) -> String {
    let list = match base {
        BaseSize::Base2 => BASE_2_SIZE,
        BaseSize::Base10 => BASE_10_SIZE,
        BaseSize::Raw => {
            return format!("{size}{suffix}");
        }
    };

    for base in list {
        if size > base.value {
            let output_size = size / base.value;

            if output_size < 10 {
                continue;
            }

            if size % base.value == 0 {
                return format!("{output_size}{}{suffix}", base.unit);
            } else {
                return format!("~{output_size}{}{suffix}", base.unit);
            }
        }
    }

    format!("{size}{suffix}")
}

pub fn bytes_to_unit(size: u64, base: &BaseSize) -> String {
    value_to_unit(size, base, "B")
}

pub struct WriteTags<'a> {
    tags: &'a HashMap<String, Option<String>>
}

impl<'a> WriteTags<'a> {
    pub fn new(tags: &'a HashMap<String, Option<String>>) -> Self {
        WriteTags { tags }
    }
}

impl<'a> Display for WriteTags<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut width = 0usize;
        let mut no_value = BinaryHeap::new();
        let mut with_value = BinaryHeap::new();

        for (key, value) in self.tags {
            if value.is_some() {
                with_value.push(key.clone());

                let chars_count = key.chars().count();

                if chars_count > width {
                    width = chars_count;
                }
            } else {
                no_value.push(key.clone());
            }
        }

        write!(f, "tags:")?;

        for key in no_value.into_sorted_vec() {
            write!(f, ", {key}")?;
        }

        write!(f, "\n")?;

        for key in with_value.into_sorted_vec() {
            let value = self.tags.get(&key)
                .unwrap()
                .as_ref()
                .unwrap();

            write!(f, "    {key:>width$}: {value}\n")?;
        }

        Ok(())
    }
}

const HEX_LIST_LOWER: [char; 16] = ['0','1','2','3','4','5','6','7','8','9','a','b','c','d','e','f'];

pub struct HexString<'a> {
    slice: &'a [u8]
}

impl<'a> HexString<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        HexString { slice }
    }
}

impl<'a> Display for HexString<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for byte in self.slice {
            let upper = (byte >> 4) as usize;
            let lower = (byte & 0xf) as usize;

            f.write_char(HEX_LIST_LOWER[upper])?;
            f.write_char(HEX_LIST_LOWER[lower])?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum DateFormat {
    Local,
    Utc,
    Unix
}

impl Default for DateFormat {
    fn default() -> Self {
        DateFormat::Local
    }
}

impl Display for DateFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            DateFormat::Local => write!(f, "local"),
            DateFormat::Utc => write!(f, "utc"),
            DateFormat::Unix => write!(f, "unix"),
        }
    }
}

pub fn datetime_to_string(dt: &DateTime<Utc>, format: &DateFormat) -> String {
    match format {
        DateFormat::Local => {
            DateTime::<Local>::from(dt.clone()).to_rfc3339_opts(SecondsFormat::Secs, false)
        },
        DateFormat::Utc => {
            dt.to_rfc3339_opts(SecondsFormat::Secs, false)
        },
        DateFormat::Unix => {
            dt.timestamp().to_string()
        }
    }
}

pub enum Float {
    Left,
    //Center,
    Right,
}

pub struct ColumnBuilder {
    name: String,
    width: Option<usize>,
    float: Option<Float>,
}

impl ColumnBuilder {
    /*
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }
    */

    pub fn float(mut self, float: Float) -> Self {
        self.float = Some(float);
        self
    }

    pub fn build(self) -> Column {
        let width = if let Some(w) = self.width {
            w
        } else {
            self.name.len()
        };

        Column {
            name: self.name,
            width,
            float: self.float.unwrap_or(Float::Left),
        }
    }
}

pub struct Column {
    name: String,
    width: usize,
    float: Float,
}

impl Column {
    pub fn builder<N>(name: N) -> ColumnBuilder
    where
        N: Into<String>,
    {
        ColumnBuilder {
            name: name.into(),
            width: None,
            float: None,
        }
    }

    pub fn write_value<D, O>(&self, value: &D, output: &mut O) -> std::io::Result<()>
    where
        D: Display + Sized,
        O: std::io::Write,
    {
        match self.float {
            Float::Left => {
                write!(output, "{value:<width$}", width=self.width)
            }
            /*
            Float::Center => {
                write!(output, "{value:^width$}", width=self.width)
            }
            */
            Float::Right => {
                write!(output, "{value:>width$}", width=self.width)
            }
        }
    }

    pub fn write_header<O>(&self, output: &mut O) -> std::io::Result<()>
    where
        O: std::io::Write
    {
        self.write_value(&self.name, output)
    }

    pub fn update_width(&mut self, width: usize) -> bool {
        if width > self.width {
            self.width = width;
            true
        } else {
            false
        }
    }
}

pub struct HeaderSep {
    pub ch: char,
    pub col_sep: &'static str,
}

pub struct TableOptions {
    pub col_sep: &'static str,
    pub header_sep: Option<HeaderSep>,
}

/*
pub const SIMPLE_OPTIONS: TableOptions = TableOptions {
    col_sep: " ",
    header_sep: None,
};
*/

pub const PRETTY_OPTIONS: TableOptions = TableOptions {
    col_sep: " | ",
    header_sep: Some(HeaderSep {
        ch: '-',
        col_sep: "-+-",
    })
};

pub fn write_table<O, T, U, const N: usize>(
    output: &mut O,
    rows: &[(T, [Option<U>; N])],
    columns: &[Column; N],
    options: &TableOptions,
) -> std::io::Result<()>
where
    O: std::io::Write,
    U: Display + Sized,
{
    let empty = "";
    let total = rows.len();
    let index_width = (total.ilog10() + 2) as usize;

    write!(output, "{empty:index_width$}")?;

    for col in columns {
        write!(output, "{}", options.col_sep)?;

        col.write_header(output)?;
    }

    if let Some(header_sep) = &options.header_sep {
        write!(output, "\n")?;

        for _ in 0..index_width {
            write!(output, "{}", header_sep.ch)?;
        }

        for col in columns {
            write!(output, "{}", header_sep.col_sep)?;

            for _ in 0..col.width {
                write!(output, "{}", header_sep.ch)?;
            }
        }
    }

    write!(output, "\n")?;

    for (index, (_, row)) in rows.iter().enumerate() {
        write!(output, "{:>index_width$}", index + 1)?;

        for (col, value) in columns.iter().zip(row) {
            write!(output, "{}", options.col_sep)?;

            if let Some(st) = value {
                col.write_value(st, output)?;
            } else {
                col.write_value(&empty, output)?;
            }
        }

        write!(output, "\n")?;
    }

    Ok(())
}

pub struct TextTable<T, const N: usize> {
    columns: [Column; N],
    rows: Vec<(T, [Option<String>; N])>
}

pub struct TextRow<'a, T, const N: usize> {
    table: &'a mut TextTable<T, N>,
    row: [Option<String>; N],
}

impl<'a, T, const N: usize> TextRow<'a, T, N> {
    pub fn set_col<V>(&mut self, index: usize, value: V) -> bool
    where
        V: Display
    {
        if index >= N {
            false
        } else {
            self.row[index] = Some(value.to_string());
            true
        }
    }

    fn update_cols(&mut self) {
        for (value, col) in self.row.iter().zip(&mut self.table.columns) {
            if let Some(st) = &value {
                let chars_count = st.chars().count();

                col.update_width(chars_count);
            }
        }
    }

    pub fn finish(mut self, data: T) {
        self.update_cols();

        self.table.rows.push((data, self.row));
    }

    pub fn finish_sort_by<F>(mut self, data: T, cb: F)
    where
        F: Fn(&T, &T) -> bool
    {
        self.update_cols();

        let index = self.table.rows.partition_point(|(v, _)| cb(&data, v));

        self.table.rows.insert(index, (data, self.row));
    }
}

impl<'a, T, const N: usize> TextRow<'a, T, N>
where
    T: Ord
{
    pub fn finish_sort(mut self, data: T) {
        self.update_cols();

        let index = self.table.rows.partition_point(|(v, _)| {
            match v.cmp(&data) {
                Ordering::Equal => true,
                Ordering::Less => true,
                Ordering::Greater=> false,
            }
        });

        self.table.rows.insert(index, (data, self.row));
    }
}

impl<T, const N: usize> TextTable<T, N> {
    pub fn with_columns(columns: [Column; N]) -> Self {
        TextTable {
            columns,
            rows: Vec::new()
        }
    }

    pub fn add_row(&mut self) -> TextRow<'_, T, N> {
        TextRow {
            table: self,
            row: std::array::from_fn(|_| None),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn write<O>(&self, output: &mut O, options: &TableOptions) -> std::io::Result<()>
    where
        O: std::io::Write
    {
        write_table(output, &self.rows, &self.columns, options)
    }

    #[inline]
    pub fn print(&self, options: &TableOptions) -> std::io::Result<()> {
        let mut stdout = std::io::stdout();

        self.write(&mut stdout, options)
    }
}

#[derive(Debug, Args)]
pub struct OutputOptions {
    /// specifies the format for the file size output
    #[arg(long, default_value_t)]
    pub size_format: BaseSize,

    /// specifies the format for the timestamp output
    #[arg(long, default_value_t)]
    pub ts_format: DateFormat,
}

pub fn write_fs_file<O>(output: &mut O, file: &rfs_api::fs::File, options: &OutputOptions) -> std::io::Result<()>
where
    O: std::io::Write
{
    write!(
        output,
        "file {} {}/{} {}\n",
        file.id.id(),
        file.path,
        file.basename,
        bytes_to_unit(file.size, &options.size_format)
    )?;
    write!(output, "parent: {}\n", file.parent.id())?;
    write!(output, "created: {}\n", datetime_to_string(&file.created, &options.ts_format))?;

    if let Some(updated) = file.updated {
        write!(output, "updated: {}\n", datetime_to_string(&updated, &options.ts_format))?;
    }

    write!(output, "mime: {}\n", file.mime)?;
    write!(output, "hash: {}\n", HexString::new(&file.hash))?;

    if !file.tags.is_empty() {
        write!(output, "{}", WriteTags::new(&file.tags))?;
    }

    if let Some(comment) = &file.comment {
        write!(output, "comment: {comment}\n")?;
    }

    Ok(())
}

pub fn write_fs_root<O>(output: &mut O, root: &rfs_api::fs::Root, options: &OutputOptions) -> std::io::Result<()>
where
    O: std::io::Write
{
    write!(output, "root {} {}\n", root.id.id(), root.basename)?;
    write!(output, "created: {}\n", datetime_to_string(&root.created, &options.ts_format))?;

    if let Some(updated) = root.updated {
        write!(output, "updated: {}\n", datetime_to_string(&updated, &options.ts_format))?;
    }

    if !root.tags.is_empty() {
        write!(output, "{}", WriteTags::new(&root.tags))?;
    }

    if let Some(comment) = &root.comment {
        write!(output, "comment: {comment}\n")?;
    }

    Ok(())
}

pub fn write_fs_dir<O>(output: &mut O, dir: &rfs_api::fs::Directory, options: &OutputOptions) -> std::io::Result<()>
where
    O: std::io::Write
{
    write!(output, "directory {} {}/{}\n", dir.id.id(), dir.path, dir.basename)?;
    write!(output, "parent: {}\n", dir.parent.id())?;
    write!(output, "created: {}\n", datetime_to_string(&dir.created, &options.ts_format))?;

    if let Some(updated) = dir.updated {
        write!(output, "updated: {}\n", datetime_to_string(&updated, &options.ts_format))?;
    }

    if !dir.tags.is_empty() {
        write!(output, "{}", WriteTags::new(&dir.tags))?;
    }

    if let Some(comment) = &dir.comment {
        write!(output, "comment: {comment}\n")?;
    }

    Ok(())
}

pub fn write_fs_item<O>(output: &mut O, item: &rfs_api::fs::Item, options: &OutputOptions) -> std::io::Result<()>
where
    O: std::io::Write
{
    match item {
        rfs_api::fs::Item::Root(root) => write_fs_root(output, &root, options),
        rfs_api::fs::Item::Directory(dir) => write_fs_dir(output, &dir, options),
        rfs_api::fs::Item::File(file) => write_fs_file(output, &file, options),
    }
}
