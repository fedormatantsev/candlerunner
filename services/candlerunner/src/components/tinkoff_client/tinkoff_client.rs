use chrono::prelude::*;
use tonic::transport::Endpoint;

use component_store::{init_err, prelude::*};

use crate::models::account::{Account, Environment};
use crate::models::instruments::{Figi, Instrument};
use crate::models::market_data::CandleTimeline;
use crate::models::positions::AccountPositions;

use super::tinkoff_generic_client::TinkoffGenericClient;
use super::tinkoff_production_client::TinkoffProductionClient;
use super::tinkoff_sandbox_client::TinkoffSandboxClient;

pub struct TinkoffClient {
    sandbox_client: TinkoffSandboxClient,
    production_client: TinkoffProductionClient,
}

impl InitComponent for TinkoffClient {
    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<Self, ComponentError>> {
        Box::pin(TinkoffClient::new(resolver, config))
    }
}

impl ShutdownComponent for TinkoffClient {}

impl ComponentName for TinkoffClient {
    fn component_name() -> &'static str {
        "tinkoff-client"
    }
}

impl Component for TinkoffClient {}

impl TinkoffClient {
    async fn new(
        _: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> Result<Self, ComponentError> {
        let url = config.get_str("url")?;
        let sandbox_auth_token = config.get_str("sandbox_auth_token")?.to_owned();
        let production_auth_token = config.get_str("production_auth_token")?.to_owned();

        let channel = Endpoint::new(url.to_string())
            .map_err(init_err)?
            .connect()
            .await
            .map_err(init_err)?;

        let sandbox_client = TinkoffSandboxClient::new(channel.clone(), sandbox_auth_token)?;
        let production_client = TinkoffProductionClient::new(channel, production_auth_token)?;

        return Ok(Self {
            sandbox_client,
            production_client,
        });
    }

    pub async fn get_instruments(&self) -> anyhow::Result<Vec<Instrument>> {
        self.production_client.get_instruments().await
    }

    pub async fn get_candles(
        &self,
        figi: &Figi,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<CandleTimeline> {
        self.production_client.get_candles(figi, from, to).await
    }

    pub async fn list_accounts(&self) -> anyhow::Result<Vec<Account>> {
        let sandbox_accounts = self.sandbox_client.list_accounts().await;
        let sandbox_accounts = match sandbox_accounts {
            Ok(accts) => accts,
            Err(err) => {
                println!("Failed to fetch sandbox accounts: {}", err.to_string());
                vec![]
            }
        };

        let production_accounts = self.production_client.list_accounts().await;
        let production_accounts = match production_accounts {
            Ok(accts) => accts,
            Err(err) => {
                println!("Failed to fetch production accounts: {}", err.to_string());
                vec![]
            }
        };

        return Ok(sandbox_accounts
            .into_iter()
            .chain(production_accounts.into_iter())
            .collect());
    }

    pub async fn open_sandbox_account(&self) -> anyhow::Result<()> {
        self.sandbox_client.open_sandbox_account().await
    }

    pub async fn close_sandbox_account(&self, account: &Account) -> anyhow::Result<()> {
        self.sandbox_client.close_sandbox_account(account).await
    }

    fn get_client(&self, account: &Account) -> &dyn TinkoffGenericClient {
        match account.environment {
            Environment::Sandbox => &self.sandbox_client,
            Environment::Production => &self.production_client,
        }
    }

    pub async fn list_positions(&self, account: &Account) -> anyhow::Result<AccountPositions> {
        let client = self.get_client(account);
        client.list_positions(account).await
    }
}
