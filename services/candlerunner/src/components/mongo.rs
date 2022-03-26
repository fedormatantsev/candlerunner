use std::sync::Arc;

use mongodb::{bson::Document, options::ClientOptions, Client, Database};

use component_store::{
    init_err, Component, ComponentError, ComponentName, CreateComponent, DestroyComponent,
};

use crate::models::instruments::Instrument;

pub struct Mongo {
    client: Client,
    db: Database,
}

impl CreateComponent for Mongo {
    fn create(
        resolver: component_store::ComponentResolver,
        config: Box<dyn component_store::ConfigProvider>,
    ) -> component_store::ComponentFuture<
        Result<std::sync::Arc<Self>, component_store::ComponentError>,
    > {
        Box::pin(async move { Ok(Arc::new(Mongo::new(resolver, config).await?)) })
    }
}

impl DestroyComponent for Mongo {}

impl ComponentName for Mongo {
    fn component_name() -> &'static str {
        "mongo"
    }
}

impl Component for Mongo {}

impl Mongo {
    async fn new(
        _: component_store::ComponentResolver,
        config: Box<dyn component_store::ConfigProvider>,
    ) -> Result<Self, ComponentError> {
        let url = config.get_str("url")?;
        let mut client_options = ClientOptions::parse(url).await.map_err(init_err)?;
        client_options.app_name = Some("candlerunner".to_string());

        let client = Client::with_options(client_options).map_err(init_err)?;
        let db = client.database("candlerunner");

        Ok(Self { client, db })
    }

    pub async fn write_instruments(&self, instruments: Vec<Instrument>) -> anyhow::Result<()> {
        println!("writing {} instruments to mongo", instruments.len());

        let collection = self.db.collection::<Document>("instruments");

        Ok(())
    }
}
