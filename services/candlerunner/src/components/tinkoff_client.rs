use tonic::metadata::{AsciiMetadataKey, AsciiMetadataValue};
use tonic::service::interceptor::{InterceptedService, Interceptor};
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};

use component_store::{init_err, prelude::*};

use crate::generated::tinkoff_invest_api::{
    self, instruments_service_client::InstrumentsServiceClient,
};
use crate::models::instruments::{Figi, Instrument, Ticker};

#[derive(Clone)]
struct AuthorizationInterceptor {
    auth_key: AsciiMetadataKey,
    auth_value: AsciiMetadataValue,
}

impl Interceptor for AuthorizationInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let meta = request.metadata_mut();
        meta.insert(self.auth_key.clone(), self.auth_value.clone());

        Ok(request)
    }
}

impl AuthorizationInterceptor {
    pub fn new(
        auth_token: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let key = AsciiMetadataKey::from_static("authorization");
        let value = AsciiMetadataValue::from_str(format!("Bearer {}", auth_token).as_str())?;

        Ok(Self {
            auth_key: key,
            auth_value: value,
        })
    }
}

fn from_share(proto: tinkoff_invest_api::Share) -> Instrument {
    Instrument {
        figi: Figi(proto.figi),
        ticker: Ticker(proto.ticker),
        display_name: proto.name,
    }
}

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

    pub async fn get_instruments(&self) -> Result<Vec<Instrument>, anyhow::Error> {
        let mut instruments_client = InstrumentsServiceClient::new(self.client.clone());

        let request = tinkoff_invest_api::InstrumentsRequest {
            instrument_status: tinkoff_invest_api::InstrumentStatus::Base as i32,
        };

        let shares_resp = instruments_client.shares(request).await?;
        let res: Vec<_> = shares_resp
            .into_inner()
            .instruments
            .into_iter()
            .map(from_share)
            .collect();

        Ok(res)
    }
}
