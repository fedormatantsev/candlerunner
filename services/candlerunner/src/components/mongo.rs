use std::collections::BTreeMap;

use bson::from_bson;
use chrono::prelude::*;
use futures::stream::StreamExt;
use futures::{TryFutureExt, TryStreamExt};
use mongodb::bson::{doc, from_document, to_document, Document};
use mongodb::options::{
    CreateCollectionOptions, TimeseriesGranularity, TimeseriesOptions, UpdateOptions,
};
use mongodb::{options::ClientOptions, Client, Database};
use serde::{de::DeserializeOwned, Serialize};

use component_store::{init_err, prelude::*};

use crate::models::instance_id::InstanceId;
use crate::models::instruments::{Figi, Instrument};
use crate::models::market_data::{Candle, CandleTimeline, DataAvailability};
use crate::models::position_manager::{
    PositionManagerExecutionState, PositionManagerInstanceDefinition,
};
use crate::models::strategy::{
    StrategyExecutionContext, StrategyExecutionState, StrategyInstanceDefinition,
};

const CANDLE_DATA_COLLECTION_NAME: &str = "candle_data";
const STRATEGY_EXECUTION_COLLECTION_NAME: &str = "strategy_execution";

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

fn get_datetime(doc: &Document, field_name: &str) -> anyhow::Result<DateTime<Utc>> {
    let ts = doc
        .get(field_name)
        .ok_or_else(|| anyhow::anyhow!("Field `{}` is missing", field_name))?;

    Ok(ts
        .as_datetime()
        .ok_or_else(|| anyhow::anyhow!("Field `{}` is not a datetime", field_name))?
        .to_chrono())
}

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

        let collections = db.list_collection_names(None).await.map_err(init_err)?;

        if !collections.contains(&CANDLE_DATA_COLLECTION_NAME.to_string()) {
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
        }

        if !collections.contains(&STRATEGY_EXECUTION_COLLECTION_NAME.to_string()) {
            db.create_collection(
                STRATEGY_EXECUTION_COLLECTION_NAME,
                CreateCollectionOptions::builder()
                    .timeseries(
                        TimeseriesOptions::builder()
                            .time_field("ts".into())
                            .meta_field(Some("strategy_id".to_string()))
                            .granularity(Some(TimeseriesGranularity::Seconds))
                            .build(),
                    )
                    .build(),
            )
            .map_err(init_err)
            .await?;
        }

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
                    UpdateOptions::builder().upsert(true).build(),
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

            collection
                .insert_one(
                    doc! {
                        "ts": ts,
                        "figi": &figi.0,
                        "candle": candle
                    },
                    None,
                )
                .await?;
        }

        Ok(())
    }

    pub async fn read_candles(
        &self,
        figi: &Figi,
        time_from: DateTime<Utc>,
        time_to: DateTime<Utc>,
    ) -> anyhow::Result<CandleTimeline> {
        let collection = self.db.collection::<Document>("candle_data");

        let raw_data: Vec<_> = collection
            .find(
                doc! {
                    "figi": &figi.0,
                    "$and": [
                        {
                            "ts": {
                                "$gt": time_from
                            },
                        },
                        {
                            "ts": {
                                "$lt": time_to
                            }
                        }
                    ]
                },
                None,
            )
            .await?
            .try_collect()
            .await?;

        let mut candles = CandleTimeline::default();

        for doc in raw_data {
            let ts = get_datetime(&doc, "ts")?;
            let candle_doc = doc
                .get("candle")
                .ok_or_else(|| anyhow::anyhow!("`candle` field is missing"))?;

            let candle = from_bson::<Candle>(candle_doc.clone())?;

            candles.insert(ts, candle);
        }

        Ok(candles)
    }

    pub async fn write_candle_data_availability(
        &self,
        figi: &Figi,
        date: Date<Utc>,
        availability: DataAvailability,
    ) -> anyhow::Result<()> {
        let collection = self.db.collection::<Document>("candle_data_availability");
        let ts = mongodb::bson::DateTime::from_chrono(date.and_hms(0, 0, 0));
        let availability = to_document(&availability)?;

        collection
            .update_one(
                doc! {"figi": &figi.0, "ts": ts},
                doc! { "$set": { "availability": availability }},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    pub async fn read_candle_data_availability(
        &self,
        figi: &Figi,
    ) -> anyhow::Result<BTreeMap<Date<Utc>, DataAvailability>> {
        let collection = self.db.collection::<Document>("candle_data_availability");
        let raw_data: Vec<_> = collection
            .find(doc! {"figi": &figi.0}, None)
            .await?
            .try_collect()
            .await?;

        let mut res: BTreeMap<Date<Utc>, DataAvailability> = Default::default();

        for doc in raw_data {
            let ts = get_datetime(&doc, "ts")?;
            let availability = doc
                .get("availability")
                .ok_or_else(|| anyhow::anyhow!("`availability` field is missing"))?;

            let availability = from_bson::<DataAvailability>(availability.clone())?;
            res.insert(ts.date(), availability);
        }

        println!(
            "Fetched {} items from `candle_data_availability` collection",
            res.len()
        );

        Ok(res)
    }

    pub async fn read_strategy_execution_contexts(
        &self,
        strategy_id: &uuid::Uuid,
        time_from: DateTime<Utc>,
        time_to: Option<DateTime<Utc>>,
    ) -> anyhow::Result<BTreeMap<DateTime<Utc>, StrategyExecutionContext>> {
        let collection = self
            .db
            .collection::<Document>(STRATEGY_EXECUTION_COLLECTION_NAME);

        let filter = match time_to {
            Some(time_to) => doc! {
                "strategy_id": strategy_id,
                "$and": [
                    {
                        "ts": {
                            "$gt": time_from
                        }
                    },
                    {
                        "ts": {
                            "$lt": time_to
                        }

                    }
                ]
            },
            None => doc! {
                "strategy_id": strategy_id,
                "ts": {
                    "$gt": time_from
                }
            },
        };

        let raw_data: Vec<_> = collection.find(filter, None).await?.try_collect().await?;
        let mut contexts: BTreeMap<DateTime<Utc>, StrategyExecutionContext> = Default::default();

        for doc in raw_data {
            let ts = get_datetime(&doc, "ts")?;
            let ctx_doc = doc
                .get("context")
                .ok_or_else(|| anyhow::anyhow!("`context` field is missing from document"))?;

            let ctx = from_bson::<StrategyExecutionContext>(ctx_doc.clone())?;

            contexts.insert(ts, ctx);
        }

        Ok(contexts)
    }

    pub async fn write_strategy_execution_contexts(
        &self,
        strategy_id: &uuid::Uuid,
        contexts: Vec<(DateTime<Utc>, StrategyExecutionContext)>,
    ) -> anyhow::Result<()> {
        let collection = self
            .db
            .collection::<Document>(STRATEGY_EXECUTION_COLLECTION_NAME);

        for (ts, ctx) in contexts {
            let doc = to_document(&ctx)?;

            collection
                .insert_one(
                    doc! {
                        "ts": ts,
                        "strategy_id": strategy_id,
                        "context": doc
                    },
                    None,
                )
                .await?;
        }

        Ok(())
    }

    pub async fn write_strategy_execution_state(
        &self,
        strategy_id: &uuid::Uuid,
        state: &StrategyExecutionState,
    ) -> anyhow::Result<()> {
        let collection = self.db.collection::<Document>("strategy_execution_state");
        let state = to_document(state)?;

        collection
            .update_one(
                doc! { "strategy_id": strategy_id },
                doc! { "$set": { "state": state } },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    pub async fn read_strategy_execution_state(
        &self,
        strategy_id: &uuid::Uuid,
    ) -> anyhow::Result<Option<StrategyExecutionState>> {
        let collection = self.db.collection::<Document>("strategy_execution_state");

        let doc = collection
            .find_one(doc! {"strategy_id": strategy_id}, None)
            .await?;

        if let Some(doc) = doc {
            let state_doc = doc
                .get("state")
                .ok_or_else(|| anyhow::anyhow!("Field `state` is missing"))?
                .clone();

            return Ok(Some(from_bson::<StrategyExecutionState>(state_doc)?));
        };

        Ok(None)
    }

    pub async fn write_position_manager_instance(
        &self,
        instance_def: &PositionManagerInstanceDefinition,
    ) -> anyhow::Result<()> {
        let collection = self.db.collection::<Document>("position_manager_instances");
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

    pub async fn read_position_manager_instances(
        &self,
    ) -> anyhow::Result<Vec<PositionManagerInstanceDefinition>> {
        return self
            .read_items::<PositionManagerInstanceDefinition>("position_manager_instances")
            .await;
    }

    pub async fn write_position_manager_execution_state(
        &self,
        position_manager_id: &uuid::Uuid,
        state: &PositionManagerExecutionState,
    ) -> anyhow::Result<()> {
        let collection = self
            .db
            .collection::<Document>("position_manager_execution_state");
        let state = to_document(state)?;

        collection
            .update_one(
                doc! { "position_manager_id": position_manager_id },
                doc! { "$set": { "state": state } },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    pub async fn read_position_manager_execution_state(
        &self,
        position_manager_id: &uuid::Uuid,
    ) -> anyhow::Result<Option<PositionManagerExecutionState>> {
        let collection = self
            .db
            .collection::<Document>("position_manager_execution_state");

        let doc = collection
            .find_one(doc! {"position_manager_id": position_manager_id}, None)
            .await?;

        if let Some(doc) = doc {
            let state_doc = doc
                .get("state")
                .ok_or_else(|| anyhow::anyhow!("Field `state` is missing"))?
                .clone();

            return Ok(Some(from_bson::<PositionManagerExecutionState>(state_doc)?));
        };

        Ok(None)
    }
}
