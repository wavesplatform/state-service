use crate::error::Error;
use serde::Deserialize;

fn default_port() -> u16 {
    8080
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_pgport() -> u16 {
    5432
}

fn default_pgpoolsize() -> u8 {
    4
}

#[derive(Deserialize, Debug, Clone)]
struct ConfigFlat {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_metrics_port")]
    metrics_port: u16,
}

#[derive(Deserialize, Debug, Clone)]
struct PostgresConfigFlat {
    pub pghost: String,
    #[serde(default = "default_pgport")]
    pub pgport: u16,
    pub pgdatabase: String,
    pub pguser: String,
    pub pgpassword: String,
    #[serde(default = "default_pgpoolsize")]
    pub pgpoolsize: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TracingConfig {
    pub service_name_prefix: Option<String>,
    pub jaeger_agent_endpoint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub metrics_port: u16,
    pub postgres: PostgresConfig,
    pub tracing: TracingConfig,
}

#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    pub pool_size: u8,
}

impl From<PostgresConfigFlat> for PostgresConfig {
    fn from(pgcf: PostgresConfigFlat) -> Self {
        Self {
            host: pgcf.pghost,
            port: pgcf.pgport,
            database: pgcf.pgdatabase,
            user: pgcf.pguser,
            password: pgcf.pgpassword,
            pool_size: pgcf.pgpoolsize,
        }
    }
}

pub fn load() -> Result<Config, Error> {
    let config_flat = envy::from_env::<ConfigFlat>()?;
    Ok(Config {
        port: config_flat.port,
        metrics_port: config_flat.metrics_port,
        postgres: envy::from_env::<PostgresConfigFlat>()?.into(),
        tracing: envy::prefixed("TRACING__").from_env::<TracingConfig>()?,
    })
}
