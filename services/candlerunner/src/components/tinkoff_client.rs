use std::sync::Arc;

use component_store::{
    init_err, Component, ComponentError, ComponentFuture, ComponentName, ComponentResolver,
    ConfigProvider, CreateComponent, DestroyComponent,
};
use tonic::transport::{Channel, Endpoint};

use crate::generated::tinkoff_invest_api;

pub struct TinkoffClient {
    instruments_client:
        tinkoff_invest_api::instruments_service_client::InstrumentsServiceClient<Channel>,
}

impl CreateComponent for TinkoffClient {
    fn create(
        component_resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<Arc<Self>, ComponentError>> {
        Box::pin(async move {
            Ok(Arc::new(
                TinkoffClient::new(component_resolver, config).await?,
            ))
        })
    }
}

impl DestroyComponent for TinkoffClient {}

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
        println!("url: {}, auth_token: {}", url, auth_token);

        let channel = Endpoint::new(url.to_string())
            .map_err(init_err)?
            .connect()
            .await
            .map_err(init_err)?;

        let instruments_client =
            tinkoff_invest_api::instruments_service_client::InstrumentsServiceClient::new(
                channel.clone(),
            );

        return Ok(Self { instruments_client });
    }
}
