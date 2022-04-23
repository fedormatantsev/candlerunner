use chrono::{prelude::*, Duration};
use tonic::service::interceptor::InterceptedService;
use tonic::transport::{Channel, Endpoint};

use component_store::{init_err, prelude::*};

use crate::generated::tinkoff_invest_api::market_data_service_client::MarketDataServiceClient;
use crate::generated::tinkoff_invest_api::{
    self, instruments_service_client::InstrumentsServiceClient,
};
use crate::models::instruments::{Figi, Instrument};
use crate::models::market_data::{Candle, CandleTimeline};

use super::interceptor::AuthorizationInterceptor;

pub struct TinkoffClient {
    client: InterceptedService<Channel, AuthorizationInterceptor>,
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
        let auth_token = config.get_str("auth_token")?;

        let channel = Endpoint::new(url.to_string())
            .map_err(init_err)?
            .connect()
            .await
            .map_err(init_err)?;

        let interceptor = AuthorizationInterceptor::new(auth_token)?;
        let client = InterceptedService::new(channel, interceptor);

        return Ok(Self { client });
    }

    pub async fn get_instruments(&self) -> anyhow::Result<Vec<Instrument>> {
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

    pub async fn get_candles(
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
}
