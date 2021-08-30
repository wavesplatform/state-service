#[macro_use]
extern crate diesel;

pub mod api;
pub mod config;
pub mod data_entries;
pub mod db;
pub mod error;
pub mod schema;
pub mod text_utils;

// tracing
use opentelemetry::global;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let config = config::load()?;

    let mut tracing_enabled = false;

    if let (Some(service_name_prefix), Some(jaeger_agent_endpoint)) = (
        config.tracing.service_name_prefix,
        config.tracing.jaeger_agent_endpoint,
    ) {
        tracing_enabled = true;
        println!("tracing enabled: {}, {}", service_name_prefix, jaeger_agent_endpoint);
        global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

        let tracer = opentelemetry_jaeger::new_pipeline()
            .with_service_name(format!("{}/state-service", service_name_prefix))
            .with_agent_endpoint(jaeger_agent_endpoint)
            .install_batch(opentelemetry::runtime::Tokio)?;

        let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        let fmt_layer = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish();

        fmt_layer.with(opentelemetry).try_init()?;
    }

    let data_entries_repo = {
        let pg_pool = db::pool(&config.postgres)?;
        data_entries::Repo::new(pg_pool)
    };

    api::start(config.port, data_entries_repo).await;

    if tracing_enabled {
        global::shutdown_tracer_provider();
    }

    Ok(())
}
