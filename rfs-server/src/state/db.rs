use tokio_postgres::{Config, NoTls};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};

use crate::config;

pub fn from_config(config: &config::Config) -> Result<Pool, deadpool_postgres::BuildError> {
    let mut pg_config = Config::new();

    pg_config.user(config.settings.db.user.as_str());

    if let Some(password) = &config.settings.db.password {
        pg_config.password(password.as_str());
    }

    pg_config.host(config.settings.db.host.as_str());
    pg_config.port(config.settings.db.port);
    pg_config.dbname(config.settings.db.dbname.as_str());

    let manager_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };

    let manager = Manager::from_config(pg_config, NoTls, manager_config);

    Pool::builder(manager)
        .max_size(4)
        .build()
}
