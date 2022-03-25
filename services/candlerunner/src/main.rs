mod components;
mod generated;
mod models;

use anyhow;
use clap::Parser;
use tokio;

use component_store::ComponentStore;
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

    let component_store = ComponentStore::builder()
        .register::<components::TinkoffClient>()
        .register::<components::StrategyRegistry>()
        .build(config)
        .await?;

    Ok(())
}
