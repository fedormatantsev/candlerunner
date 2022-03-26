use std::sync::{Arc, Mutex};
use tokio::{sync::watch, task::JoinHandle, time};

use component_store::{
    Component, ComponentError, ComponentFuture, ComponentName, ComponentResolver, ConfigProvider,
    CreateComponent, DestroyComponent,
};

use crate::components;

pub struct InstrumentSync {
    stop: watch::Sender<bool>,
    periodic: Mutex<Option<JoinHandle<()>>>,
}

impl CreateComponent for InstrumentSync {
    fn create(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<std::sync::Arc<Self>, ComponentError>> {
        Box::pin(async { Ok(Arc::new(Self::new(resolver, config).await?)) })
    }
}

impl DestroyComponent for InstrumentSync {
    fn destroy(&self) -> ComponentFuture<()> {
        let periodic = self.periodic.lock().ok();

        let periodic = match periodic {
            Some(mut p) => p.take(),
            None => {
                println!("Failed to acquire periodic handle.");
                return Box::pin(std::future::ready(()));
            }
        };

        let periodic = match periodic {
            Some(p) => p,
            None => {
                println!("Periodic handle is None. Was it running?");
                return Box::pin(std::future::ready(()));
            }
        };

        if self.stop.send(true).is_err() {
            println!("Failed to gracefully stop periodic");
            return Box::pin(std::future::ready(()));
        }

        Box::pin(async move {
            if let Err(err) = periodic.await {
                println!("Failed to gracefully stop periodic: {}", err);
            }
        })
    }
}

impl ComponentName for InstrumentSync {
    fn component_name() -> &'static str {
        "instrument-sync"
    }
}

impl Component for InstrumentSync {}

impl InstrumentSync {
    async fn update(
        tinkoff_client: &Arc<components::TinkoffClient>,
        mongo: &Arc<components::Mongo>,
    ) -> anyhow::Result<()> {
        let instruments = tinkoff_client.get_instruments().await?;
        mongo.write_instruments(instruments).await?;

        Ok(())
    }

    async fn new(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> Result<Self, ComponentError> {
        let update_period = config.get_u64("update_period")?;
        let tinkoff_client = resolver.resolve::<components::TinkoffClient>().await?;
        let mongo = resolver.resolve::<components::Mongo>().await?;

        let (sender, mut receiver) = watch::channel(false);

        let periodic = tokio::spawn(async move {
            let mut period = time::interval(time::Duration::from_secs(update_period));

            loop {
                tokio::select! {
                    _ = period.tick() => (),
                    _ = receiver.changed() => ()
                };

                if *receiver.borrow() == true {
                    break;
                }

                if let Err(err) = Self::update(&tinkoff_client, &mongo).await {
                    println!("Failed to get instruments: {}", err);
                }
            }

            println!("Exiting periodic")
        });

        Ok(Self {
            stop: sender,
            periodic: Mutex::new(Some(periodic)),
        })
    }
}
