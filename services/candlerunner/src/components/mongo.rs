use futures::stream::StreamExt;

use mongodb::bson::{doc, Document};
use mongodb::options::UpdateOptions;
use mongodb::{options::ClientOptions, Client, Database};

use component_store::{init_err, prelude::*};

use crate::models::instruments::{Figi, Instrument, Ticker};

pub struct Mongo {
    db: Database,
}

impl InitComponent for Mongo {
    fn init(
        resolver: component_store::ComponentResolver,
        config: Box<dyn component_store::ConfigProvider>,
    ) -> component_store::ComponentFuture<Result<Self, component_store::ComponentError>> {
        Box::pin(Mongo::new(resolver, config))
    }
}

impl ShutdownComponent for Mongo {}

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

    pub async fn read_instruments(&self) -> anyhow::Result<Vec<Instrument>> {
        let collection = self.db.collection::<Document>("instruments");
        let cursor = collection.find(None, None).await?;

        let res = cursor
            .fold(Vec::<Instrument>::default(), |mut state, elem| async move {
                match elem {
                    Ok(d) => {
                        let i = make_instrument(&d);
                        match i {
                            Some(i) => state.push(i),
                            None => println!("Failed to parse document: {:?}", d),
                        }
                    }
                    Err(err) => println!("Failed to get instrument: {}", err),
                }

                state
            })
            .await;

        println!("Fetched {} instruments", res.len());

        Ok(res)
    }
}

fn make_instrument(d: &Document) -> Option<Instrument> {
    let figi = d.get("figi")?.to_string();
    let ticker = d.get("ticker")?.to_string();
    let display_name = d.get("display_name")?.to_string();

    Some(Instrument {
        figi: Figi(figi),
        ticker: Ticker(ticker),
        display_name,
    })
}
