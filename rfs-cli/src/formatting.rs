use std::collections::{HashMap, BinaryHeap};
use std::fmt::{Write, Formatter, Display, Result as FmtResult};
use std::default::Default;

use chrono::{DateTime, Utc, Local, SecondsFormat};
use clap::ValueEnum;

#[derive(Debug)]
pub struct SizeType {
    value: u64,
    unit: &'static str,
}

pub const KIBIBYTE: u64 = 1024;
pub const MEBIBYTE: u64 = KIBIBYTE * 1024;
pub const GIBIBYTE: u64 = MEBIBYTE * 1024;
pub const TEBIBYTE: u64 = GIBIBYTE * 1024;
pub const PEBIBYTE: u64 = TEBIBYTE * 1024;
pub const EXBIBYTE: u64 = PEBIBYTE * 1024;

pub const BASE_2_SIZE: [SizeType; 6] = [
    SizeType {
        value: EXBIBYTE,
        unit: "EiB",
    },
    SizeType {
        value: PEBIBYTE,
        unit: "PiB",
    },
    SizeType {
        value: TEBIBYTE,
        unit: "TiB",
    },
    SizeType {
        value: GIBIBYTE,
        unit: "GiB",
    },
    SizeType {
        value: MEBIBYTE,
        unit: "MiB",
    },
    SizeType {
        value: KIBIBYTE,
        unit: "KiB",
    },
];

pub const KILOBYTE: u64 = 1000;
pub const MEGABYTE: u64 = KILOBYTE * 1000;
pub const GIGABYTE: u64 = MEGABYTE * 1000;
pub const TERABYTE: u64 = GIGABYTE * 1000;
pub const PETABYTE: u64 = TERABYTE * 1000;
pub const EXABYTE: u64 = TERABYTE * 1000;

pub const BASE_10_SIZE: [SizeType; 6] = [
    SizeType {
        value: EXABYTE,
        unit: "EB",
    },
    SizeType {
        value: PETABYTE,
        unit: "PB",
    },
    SizeType {
        value: TERABYTE,
        unit: "TB",
    },
    SizeType {
        value: GIGABYTE,
        unit: "GB",
    },
    SizeType {
        value: MEGABYTE,
        unit: "MB",
    },
    SizeType {
        value: KILOBYTE,
        unit: "KB",
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

pub fn bytes_to_unit(size: u64, base: &BaseSize) -> String {
    let list = match base {
        BaseSize::Base2 => BASE_2_SIZE,
        BaseSize::Base10 => BASE_10_SIZE,
        BaseSize::Raw => {
            return format!("{size}");
        }
    };

    for base in list {
        if size > base.value {
            let output_size = size / base.value;

            if output_size < 10 {
                continue;
            }

            if size % base.value == 0 {
                return format!("{output_size}{}", base.unit);
            } else {
                return format!("~{output_size}{}", base.unit);
            }
        }
    }

    format!("{size}B")
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
            write!(f, " {key}\n")?;
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

    pub fn print_header(&self) {
        let mut stdout = std::io::stdout();

        self.write_value(&self.name, &mut stdout).unwrap();
    }

    pub fn print_value<D>(&self, value: &D)
    where
        D: Display + Sized
    {
        let mut stdout = std::io::stdout();

        self.write_value(value, &mut stdout).unwrap();
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
