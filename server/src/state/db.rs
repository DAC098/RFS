use tokio_postgres::{Config, NoTls};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};

#[derive(Debug)]
pub struct Builder {
    with_tls: bool,
    user: Option<String>,
    password: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    dbname: Option<String>,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            with_tls: false,
            user: None,
            password: None,
            host: None,
            port: None,
            dbname: None,
        }
    }

    pub fn set_user<U>(&mut self, user: U) -> &mut Self
    where
        U: Into<String>
    {
        self.user = Some(user.into());
        self
    }

    pub fn set_password<P>(&mut self, password: P) -> &mut Self
    where
        P: Into<String>
    {
        self.password = Some(password.into());
        self
    }

    pub fn set_host<H>(&mut self, host: H) -> &mut Self
    where
        H: Into<String>
    {
        self.host = Some(host.into());
        self
    }

    pub fn set_port<P>(&mut self, port: P) -> &mut Self
    where
        P: Into<u16>
    {
        self.port = Some(port.into());
        self
    }

    pub fn set_dbname<D>(&mut self, dbname: D) -> &mut Self
    where
        D: Into<String>
    {
        self.dbname = Some(dbname.into());
        self
    }

    pub fn build(self) -> Result<Pool, deadpool_postgres::BuildError> {
        let mut config = Config::new();

        if let Some(user) = self.user {
            config.user(user.as_str());
        }

        if let Some(password) = self.password {
            config.password(password.as_str());
        }

        if let Some(host) = self.host {
            config.host(host.as_str());
        }

        if let Some(port) = self.port {
            config.port(port);
        }

        if let Some(dbname) = self.dbname {
            config.dbname(dbname.as_str());
        }

        let manager_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let manager = Manager::from_config(config, NoTls, manager_config);

        Pool::builder(manager)
            .max_size(4)
            .build()
    }
}
