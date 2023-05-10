pub mod db {
    use clap::{Arg, ArgAction, value_parser};

    pub fn connect() -> Arg {
        Arg::new("connect")
            .short('c')
            .long("connect")
            .action(ArgAction::Set)
            .help("connection string for postgres")
            .conflicts_with_all([
                "user",
                "password",
                "req_password",
                "host",
                "port",
                "dbname",
            ])
    }

    pub fn user() -> Arg {
        Arg::new("user")
            .short('u')
            .long("user")
            .action(ArgAction::Set)
            .default_value("postgres")
            .help("user for postgres connection.")
            .conflicts_with("connect")
    }

    pub fn password() -> Arg {
        Arg::new("password")
            .short('P')
            .long("password")
            .action(ArgAction::Set)
            .help("password for postgres connection.")
            .conflicts_with_all([
                "connect",
                "req_password",
            ])
    }

    pub fn req_password() -> Arg {
        Arg::new("req_password")
            .long("req-password")
            .action(ArgAction::SetTrue)
            .help("requests password via input before connection")
            .conflicts_with_all([
                "connect",
                "password",
            ])
    }

    pub fn host() -> Arg {
        Arg::new("host")
            .long("host")
            .action(ArgAction::Set)
            .default_value("localhost")
            .help("host for postgres connection.")
            .conflicts_with("connect")
    }

    pub fn port() -> Arg {
        Arg::new("port")
            .short('p')
            .long("port")
            .action(ArgAction::Set)
            .default_value("5432")
            .value_parser(value_parser!(u16))
            .help("port for postgres connection.")
            .conflicts_with("connect")
    }

    pub fn dbname() -> Arg {
        Arg::new("dbname")
            .long("dbname")
            .action(ArgAction::Set)
            .default_value("fs_server")
            .help("dbname for postgres connection.")
            .conflicts_with("connect")
    }

}
