use std::path::PathBuf;
use std::str::FromStr;
use std::net::{SocketAddr, IpAddr};

use clap::Parser;

use crate::error;
use crate::sec;
use crate::state;

mod file;

#[derive(Debug)]
pub struct Config {
    pub socket: SocketAddr,
    pub state: state::Shared,
}

#[derive(Parser, Debug)]
#[command(author, version, version, about, long_about = None)]
pub struct CliArgs {
    /// the config file to load
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// id of the server
    #[arg(long)]
    pub id: Option<i64>,

    /// the directory that will store data for the server
    #[arg(long)]
    pub data: Option<PathBuf>,

    /// ip address to bind the server to
    #[arg(short, long)]
    pub ip: Option<String>,

    /// port for the server to listen on
    #[arg(short, long)]
    pub port: Option<u16>,

    /// specifies a file that the server will provide, publicly available
    #[arg(long)]
    pub assets_file: Vec<String>,

    /// specifies a directory that the server will provide, publicly available
    #[arg(long)]
    pub assets_directory: Vec<String>,

    /// specified the directory to load handlebars templates from
    #[arg(long)]
    pub templates_directory: Option<PathBuf>,

    /// enabled dev mode for templates
    #[arg(long)]
    pub templates_dev_mode: bool,

    /// postgres username for connecting to database
    #[arg(long)]
    pub db_user: Option<String>,

    /// postgres user password for connecting to database
    #[arg(long)]
    pub db_password: Option<String>,

    /// postgres host address for database
    #[arg(long)]
    pub db_host: Option<String>,

    /// postgres port for connecting to host
    #[arg(long)]
    pub db_port: Option<u16>,

    /// postgres database name
    #[arg(long)]
    pub db_dbname: Option<String>,

    /// hashing algorithm to use for session key
    #[arg(long)]
    pub sec_session_hash: Option<sec::state::SessionHash>,

    /// inidicates that the session cookie should only be available in a secure context
    #[arg(long)]
    pub sec_session_secure: bool,
}

pub fn get_config(arg: CliArgs) -> error::Result<Config> {
    let mut state_builder = state::Shared::builder();
    let config = if let Some(file_path) = arg.config {
        file::load(file_path.clone())?
    } else {
        file::Root::default()
    };

    if let Some(id) = config.id {
        state_builder.set_primary_id(id);
    }

    if let Some(id) = arg.id {
        state_builder.set_primary_id(id);
    }

    {
        let builder = state_builder.templates();

        if let Some(config_templates) = config.templates {
            if let Some(path) = config_templates.directory {
                builder.set_templates(path);
            }

            if let Some(flag) = config_templates.dev_mode {
                builder.set_dev_mode(flag);
            }
        }

        if let Some(path) = arg.templates_directory {
            builder.set_templates(path);
        }

        if arg.templates_dev_mode {
            builder.set_dev_mode(true);
        }
    }

    {
        let pg_options = state_builder.pg_options();

        if let Some(db) = config.db {
            if let Some(user) = db.user {
                pg_options.set_user(user);
            }

            if let Some(password) = db.password {
                pg_options.set_password(password);
            }

            if let Some(host) = db.host {
                pg_options.set_host(host);
            }

            if let Some(port) = db.port {
                pg_options.set_port(port);
            }

            if let Some(dbname) = db.dbname {
                pg_options.set_dbname(dbname);
            }
        }

        if let Some(user) = arg.db_user {
            pg_options.set_user(user);
        }

        if let Some(password) = arg.db_password {
            pg_options.set_password(password);
        }

        if let Some(host) = arg.db_host {
            pg_options.set_host(host);
        }

        if let Some(port) = arg.db_port {
            pg_options.set_port(port);
        }

        if let Some(dbname) = arg.db_dbname {
            pg_options.set_dbname(dbname);
        }
    }

    {
        let sec_builder = state_builder.sec();
        let sec = config.sec;
        let session = sec.map(|v| v.session).flatten();

        {
            let session_builder = sec_builder.session_info();

            if let Some(session) = session {
                if let Some(session_hash) = session.hash {
                    session_builder.set_hash(session_hash);
                }

                if let Some(secure) = session.secure {
                    session_builder.set_secure(secure);
                }
            }

            if let Some(session_hash) = arg.sec_session_hash {
                session_builder.set_hash(session_hash);
            }

            if arg.sec_session_secure {
                session_builder.set_secure(true);
            }
        }
    }

    let port = arg.port.unwrap_or(config.port.unwrap_or(0));
    let ip = if let Some(ip) = arg.ip {
        IpAddr::from_str(&ip)
            .map_err(|_| error::Error::new()
                .kind("InvalidIp")
                .message("invalid ip address provided"))?
    } else if let Some(ip) = config.ip {
        IpAddr::from_str(&ip)
            .map_err(|_| error::Error::new()
                .kind("InvalidIp")
                .message("invalid ip address provided from config"))?
    } else {
        IpAddr::from([0,0,0,0])
    };

    tracing::debug!("shared state builder {:#?}", state_builder);

    let rtn = Config {
        state: state_builder.build()?,
        socket: SocketAddr::new(ip, port)
    };

    tracing::debug!("{:#?}", rtn);

    Ok(rtn)
}
