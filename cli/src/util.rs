use std::collections::HashMap;
use std::iter::Iterator;

use clap::{Arg, ArgMatches};

use crate::error;

pub fn default_help_arg() -> Arg {
    use clap::ArgAction;

    Arg::new("help")
        .long("help")
        .action(ArgAction::Help)
        .help("display the current help information")
}

pub fn parse_tags_list<'a, I>(mut iter: I) -> (HashMap<String, Option<String>>, Vec<&'a str>)
where
    I: Iterator<Item = &'a str>
{
    let mut map = HashMap::new();
    let mut invalid = Vec::new();

    while let Some(parse) = iter.next() {
        if let Some((name, value)) = parse.split_once(':') {
            if name.len() == 0 {
                invalid.push(parse);
                continue;
            }

            if value.len() == 0 {
                map.insert(name.to_owned(), None);
            } else {
                map.insert(name.to_owned(), Some(value.to_owned()));
            }
        } else {
            if parse.len() == 0 {
                invalid.push(parse);
                continue;
            }

            map.insert(parse.to_owned(), None);
        }
    }

    (map, invalid)
}

pub fn tags_from_args(name: &str, args: &ArgMatches) -> error::Result<HashMap<String, Option<String>>> {
    if let Some(given) = args.get_many::<String>(name) {
        let (parsed, invalid) = parse_tags_list(given.map(|v| v.as_str()));

        if invalid.len() > 0 {
            return Err(error::Error::new()
                .kind("InvalidTags")
                .message("provided tags that are an invalid format")
                .source(format!("{:?}", invalid)));
        }

        Ok(parsed)
    } else {
        Ok(HashMap::new())
    }
}

