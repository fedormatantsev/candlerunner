use chrono::prelude::*;
use futures::stream::StreamExt;
use futures::TryFutureExt;
use mongodb::bson::{doc, from_document, to_document, Document};
use mongodb::options::{
    CreateCollectionOptions, TimeseriesGranularity, TimeseriesOptions, UpdateOptions,
};
use mongodb::{options::ClientOptions, Client, Database};
use serde::{de::DeserializeOwned, Serialize};

use component_store::{init_err, prelude::*};

use crate::models::instruments::{Figi, Instrument};
use crate::models::market_data::CandleTimeline;
use crate::models::strategy::StrategyInstanceDefinition;

const CANDLE_DATA_COLLECTION_NAME: &str = "candle_data";

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

        db.create_collection(
            CANDLE_DATA_COLLECTION_NAME,
            CreateCollectionOptions::builder()
                .timeseries(
                    TimeseriesOptions::builder()
                        .time_field("ts".into())
                        .meta_field(Some("figi".to_string()))
                        .granularity(Some(TimeseriesGranularity::Seconds))
                        .build(),
                )
                .build(),
        )
        .map_err(init_err)
        .await?;

        Ok(Self { db })
    }

    pub async fn write_instruments(&self, instruments: Vec<Instrument>) -> anyhow::Result<()> {
        println!("Updating {} instruments", instruments.len());

        let collection = self.db.collection::<Document>("instruments");

        let mut modified = 0;
        let mut errors = 0;

        for instrument in instruments {
            let doc = match to_document(&instrument) {
                Ok(doc) => doc,
                Err(err) => {
                    println!("Failed to serialize instrument: {}", err);
                    continue;
                }
            };

            match collection
                .update_one(
                    doc! { "figi": &instrument.figi.0 },
                    doc! {
                        "$set": doc
                    },
                    Some(UpdateOptions::builder().upsert(true).build()),
                )
                .await
            {
                Ok(_) => modified += 1,
                Err(err) => {
                    println!("Failed to update instrument {:?}: {}", instrument, err);
                    errors += 1;
                }
            }
        }

        println!("{} modified, {} errors", modified, errors);

        Ok(())
    }

    async fn read_items<T: Serialize + DeserializeOwned>(
        &self,
        collection_name: &'static str,
    ) -> anyhow::Result<Vec<T>> {
        let collection = self.db.collection::<Document>(collection_name);
        let cursor = collection.find(None, None).await?;

        let res = cursor
            .fold(Vec::<T>::default(), |mut state, item| async move {
                match item {
                    Ok(doc) => {
                        let res = from_document::<T>(doc).map(|deserialized| {
                            state.push(deserialized);
                            ()
                        });

                        if let Err(err) = res {
                            println!(
                                "Failed to parse item from `{}` collection: {}",
                                collection_name, err
                            );
                        }
                    }
                    Err(err) => println!(
                        "Failed to get item from `{}` collection: {}",
                        collection_name, err
                    ),
                }

                state
            })
            .await;

        println!(
            "Fetched {} items from `{}` collection",
            res.len(),
            collection_name
        );

        Ok(res)
    }

    pub async fn read_instruments(&self) -> anyhow::Result<Vec<Instrument>> {
        return self.read_items::<Instrument>("instruments").await;
    }

    pub async fn write_strategy_instance(
        &self,
        instance_def: &StrategyInstanceDefinition,
    ) -> anyhow::Result<()> {
        let collection = self.db.collection::<Document>("strategy_instances");
        let doc = to_document(instance_def)?;

        collection
            .update_one(
                doc! {"_id": instance_def.id()},
                doc! {"$set": doc},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    pub async fn read_strategy_instances(&self) -> anyhow::Result<Vec<StrategyInstanceDefinition>> {
        return self
            .read_items::<StrategyInstanceDefinition>("strategy_instances")
            .await;
    }

    pub async fn write_candles(&self, figi: &Figi, candles: CandleTimeline) -> anyhow::Result<()> {
        let collection = self.db.collection::<Document>("candle_data");

        for (ts, candle) in candles {
            let candle = to_document(&candle)?;
            let ts = mongodb::bson::DateTime::from_chrono(ts);

            collection
                .update_one(
                    doc! {},
                    doc! {
                        "$set": {
                            "figi": &figi.0,
                            "ts": ts,
                            "candle": candle
                        }
                    },
                    UpdateOptions::builder().upsert(true).build(),
                )
                .await?;
        }

        Ok(())
    }

    pub async fn set_candle_data_availability(
        &self,
        figi: &Figi,
        date: Date<Utc>,
    ) -> anyhow::Result<()> {
        let collection = self.db.collection::<Document>("candle_data_availability");
        let ts = mongodb::bson::DateTime::from_chrono(date.and_hms(0, 0, 0));

        collection
            .update_one(
                doc! {"figi": &figi.0, "ts": ts},
                doc! { "$set": { "available": true }},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    pub async fn get_candle_data_availability(
        &self,
        figi: &Figi,
        date: Date<Utc>,
    ) -> anyhow::Result<bool> {
        let collection = self.db.collection::<Document>("candle_data_availability");
        let ts = mongodb::bson::DateTime::from_chrono(date.and_hms(0, 0, 0));

        let res = collection
            .find_one(doc! {"figi": &figi.0, "ts": ts}, None)
            .await?;

        let available = res
            .and_then(|elem| elem.get("available").cloned())
            .map(|available| available.as_bool().unwrap_or(false));

        Ok(available.unwrap_or(false))
    }
}
