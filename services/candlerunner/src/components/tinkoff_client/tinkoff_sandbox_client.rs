use chrono::{prelude::*, Duration};
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Channel;

use crate::generated::tinkoff_invest_api;
use crate::generated::tinkoff_invest_api::instruments_service_client::InstrumentsServiceClient;
use crate::generated::tinkoff_invest_api::market_data_service_client::MarketDataServiceClient;
use crate::generated::tinkoff_invest_api::sandbox_service_client::SandboxServiceClient;
use crate::models::account::{AccessLevel, Account, AccountId, Environment};
use crate::models::instruments::{Figi, Instrument};
use crate::models::market_data::{Candle, CandleTimeline};
use crate::models::positions::{AccountPositions, Position, Currency};

use super::interceptor::AuthorizationInterceptor;
use super::tinkoff_generic_client::TinkoffGenericClient;

pub struct TinkoffSandboxClient {
    client: InterceptedService<Channel, AuthorizationInterceptor>,
}

impl TinkoffSandboxClient {
    pub fn new(
        channel: Channel,
        auth_token: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let interceptor = AuthorizationInterceptor::new(auth_token)?;
        let client = InterceptedService::new(channel, interceptor);

        Ok(TinkoffSandboxClient { client })
    }

    pub async fn open_sandbox_account(&self) -> anyhow::Result<()> {
        let mut sandbox_client = SandboxServiceClient::new(self.client.clone());

        sandbox_client
            .open_sandbox_account(tinkoff_invest_api::OpenSandboxAccountRequest {})
            .await?;

        Ok(())
    }

    pub async fn close_sandbox_account(&self, account: &Account) -> anyhow::Result<()> {
        let mut sandbox_client = SandboxServiceClient::new(self.client.clone());

        sandbox_client
            .close_sandbox_account(tinkoff_invest_api::CloseSandboxAccountRequest {
                account_id: account.id.0.clone(),
            })
            .await?;

        Ok(())
    }

    pub async fn pay_in(&self, account: &Account, value: f64) -> anyhow::Result<()> {
        let mut sandbox_client = SandboxServiceClient::new(self.client.clone());

        sandbox_client
            .sandbox_pay_in(tinkoff_invest_api::SandboxPayInRequest {
                account_id: account.id.0.clone(),
                amount: Some(tinkoff_invest_api::MoneyValue {
                    currency: "RUB".to_owned(),
                    units: value.ceil() as i64,
                    nano: (value.fract() * 1.0e9).ceil() as i32,
                }),
            })
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl TinkoffGenericClient for TinkoffSandboxClient {
    async fn get_instruments(&self) -> anyhow::Result<Vec<Instrument>> {
        let mut instruments_client = InstrumentsServiceClient::new(self.client.clone());

        let request = tinkoff_invest_api::InstrumentsRequest {
            instrument_status: tinkoff_invest_api::InstrumentStatus::Base as i32,
        };

        let shares_resp = instruments_client.shares(request).await?;
        let res: Vec<_> = shares_resp
            .into_inner()
            .instruments
            .into_iter()
            .map(From::<tinkoff_invest_api::Share>::from)
            .collect();

        Ok(res)
    }

    async fn get_candles(
        &self,
        figi: &Figi,
        mut from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<CandleTimeline> {
        let mut market_data_client = MarketDataServiceClient::new(self.client.clone());

        let time_step = Duration::days(1);

        let mut timeline = CandleTimeline::default();

        while from < to {
            let req_to = std::cmp::min(from + time_step, to);

            let request = tinkoff_invest_api::GetCandlesRequest {
                figi: figi.0.clone(),
                from: Some(::prost_types::Timestamp {
                    seconds: from.timestamp(),
                    nanos: from.nanosecond() as i32,
                }),
                to: Some(::prost_types::Timestamp {
                    seconds: req_to.timestamp(),
                    nanos: req_to.nanosecond() as i32,
                }),
                interval: tinkoff_invest_api::CandleInterval::CandleInterval1Min as i32,
            };

            let candles_resp = market_data_client.get_candles(request).await?;

            for proto_candle in candles_resp.into_inner().candles {
                let time = proto_candle
                    .time
                    .as_ref()
                    .map(|ts| Utc.timestamp(ts.seconds, ts.nanos as u32))
                    .ok_or_else(|| anyhow::anyhow!("HistoricalCandle `time` field is missing"))?;

                let candle = Candle::try_from(proto_candle)?;

                timeline.insert(time, candle);
            }

            from = req_to;
        }

        Ok(timeline)
    }

    async fn list_accounts(&self) -> anyhow::Result<Vec<Account>> {
        let mut sandbox_client = SandboxServiceClient::new(self.client.clone());

        let request = tinkoff_invest_api::GetAccountsRequest {};
        let resp = sandbox_client
            .get_sandbox_accounts(request)
            .await?
            .into_inner();

        let res: Vec<_> = resp
            .accounts
            .into_iter()
            .filter_map(|proto| {
                let access_level = AccessLevel::from(proto.access_level());

                if proto.status() != tinkoff_invest_api::AccountStatus::Open {
                    return None;
                }

                Some(Account {
                    id: AccountId(proto.id),
                    name: proto.name,
                    access_level,
                    environment: Environment::Sandbox,
                })
            })
            .collect();

        Ok(res)
    }

    async fn list_positions(&self, account: &Account) -> anyhow::Result<AccountPositions> {
        let mut sandbox_client = SandboxServiceClient::new(self.client.clone());

        let resp = sandbox_client
            .get_sandbox_positions(tinkoff_invest_api::PositionsRequest {
                account_id: account.id.0.clone(),
            })
            .await?
            .into_inner();

        let positions = resp.securities.into_iter().map(|proto| Position {
            figi: Figi(proto.figi),
            lots: proto.balance,
        });

        let currencies = resp.money.into_iter().map(|proto| Currency {
            iso_currency: proto.currency,
            amount: (proto.units as f64) + (proto.nano as f64) * 1.0e-9f64,
        });

        Ok(AccountPositions {
            currencies: currencies.collect(),
            positions: positions.collect(),
        })
    }
}
