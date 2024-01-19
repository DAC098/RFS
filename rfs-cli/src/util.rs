use std::collections::HashMap;
use std::iter::Iterator;
use std::str::FromStr;

use clap::Args;

pub fn parse_flake_id<T>(arg: &str) -> Result<T, String>
where
    T: TryFrom<i64>
{
    let Ok(int): Result<i64, _> = i64::from_str(arg) else {
        return Err("invalid i64 value".into());
    };

    let Ok(flake) = int.try_into() else {
        return Err("invalid flake id".into());
    };

    Ok(flake)
}

pub fn parse_mime(arg: &str) -> Result<mime::Mime, String> {
    match mime::Mime::from_str(arg) {
        Ok(m) => Ok(m),
        Err(_) => Err("Invalid mime format".into())
    }
}

pub type Tag = (String, Option<String>);

pub fn parse_tag(arg: &str) -> Result<Tag, String> {
    if let Some((name, value)) = arg.split_once(':') {
        if name.is_empty() {
            return Err("tag name is empty".into());
        }

        if value.is_empty() {
            Ok((name.into(), None))
        } else {
            Ok((name.into(), Some(value.into())))
        }
    } else {
        if arg.is_empty() {
            return Err("tag is empty".into());
        }

        Ok((arg.into(), None))
    }
}

#[derive(Debug, Args)]
pub struct TagArgs {
    /// overrides current tags
    #[arg(
        short,
        long,
        conflicts_with_all(["add_tag", "drop_tag"]),
        value_parser(parse_tag)
    )]
    tag: Vec<Tag>,

    /// adds to existing tags
    #[arg(
        long,
        conflicts_with("tag"),
        value_parser(parse_tag)
    )]
    add_tag: Vec<Tag>,

    /// drops an existing tag
    #[arg(
        long,
        conflicts_with("tag")
    )]
    drop_tag: Vec<String>
}

impl TagArgs {
    pub fn merge_existing(
        self,
        mut hash_map: HashMap<String, Option<String>>
    ) -> HashMap<String, Option<String>> {
        if !self.tag.is_empty() {
            HashMap::from_iter(self.tag)
        } else if !self.drop_tag.is_empty() || !self.add_tag.is_empty() {
            for tag in self.drop_tag {
                hash_map.remove(&tag);
            }

            hash_map.extend(self.add_tag);

            hash_map
        } else {
            hash_map
        }
    }
}

