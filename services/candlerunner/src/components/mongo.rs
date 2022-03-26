use std::sync::Arc;

use mongodb::bson::{doc, Document};
use mongodb::options::UpdateOptions;
use mongodb::{options::ClientOptions, Client, Database};

use component_store::{
    init_err, Component, ComponentError, ComponentName, CreateComponent, DestroyComponent,
};

use crate::models::instruments::Instrument;

pub struct Mongo {
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

        Ok(Self { db })
    }

    pub async fn write_instruments(&self, instruments: Vec<Instrument>) -> anyhow::Result<()> {
        println!("Updating {} instruments", instruments.len());

        let collection = self.db.collection::<Document>("instruments");

        let mut modified = 0;
        let mut errors = 0;

        for instrument in instruments {
            match collection
                .update_one(
                    doc! { "figi": &instrument.figi.0 },
                    doc! {
                        "$set": {
                            "figi": &instrument.figi.0,
                            "ticker": &instrument.ticker.0,
                            "display_name": instrument.display_name
                        }
                    },
                    Some(UpdateOptions::builder().upsert(true).build()),
                )
                .await
            {
                Ok(_) => modified += 1,
                Err(err) => {
                    println!(
                        "Failed to update instrument (ticker: {}, figi: {}): {}",
                        instrument.ticker.0, instrument.figi.0, err
                    );
                    errors += 1;
                }
            }
        }

        println!("{} modified, {} errors", modified, errors);

        Ok(())
    }
}
