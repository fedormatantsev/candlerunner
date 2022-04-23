mod components;
mod generated;
mod models;
mod service;
mod strategies;

use anyhow;
use clap::Parser;
use tokio;

use component_store::{ComponentStore, ConfigProvider};
use tonic::transport::Server;
use yaml_config_provider::YamlConfigProvider;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, parse(from_os_str))]
    /// The path to the config file
    config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = Box::new(YamlConfigProvider::new(args.config)?);

    let service_config = config.get_subconfig("service")?;
    let addr = service_config.get_str("address")?.parse()?;

    let component_store = ComponentStore::builder()
        .register::<components::InstrumentCache>()?
        .register::<components::InstrumentSync>()?
        .register::<components::MarketDataSync>()?
        .register::<components::Mongo>()?
        .register::<components::StrategyCache>()?
        .register::<components::StrategyRegistry>()?
        .register::<components::TinkoffClient>()?
        .build(config)
        .await?;

    Server::builder()
        .add_service(service::Service::new(&component_store)?)
        .serve(addr)
        .await?;

    component_store.destroy().await;

    Ok(())
}
