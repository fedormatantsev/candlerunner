mod components;
mod generated;
mod models;
mod service;
mod strategies;
mod utils;

use std::net::SocketAddr;

use clap::Parser;

use component_store::{ComponentStore, ConfigProvider};
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
    let addr: SocketAddr = service_config.get_str("address")?.parse()?;

    let component_store = ComponentStore::builder()
        .register::<components::AccountsCache>()?
        .register::<components::InstrumentCache>()?
        .register::<components::InstrumentSync>()?
        .register::<components::MarketDataSync>()?
        .register::<components::Mongo>()?
        .register::<components::ParamValidator>()?
        .register::<components::PositionsCache>()?
        .register::<components::StrategyCache>()?
        .register::<components::StrategyRegistry>()?
        .register::<components::StrategyRunner>()?
        .register::<components::TinkoffClient>()?
        .build(config)
        .await?;

    service::serve(addr, &component_store).await?;

    component_store.destroy().await;

    Ok(())
}
