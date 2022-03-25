use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::{future::Future, marker::PhantomData};

use tokio::{sync::watch, task::JoinHandle, time};

use component_store::prelude::*;

pub type PeriodicFuture = Pin<Box<dyn Future<Output = ()>>>;
pub type PeriodicCreateFuture<P> =
    Pin<Box<dyn Future<Output = Result<P, ComponentError>> + Send + Sync>>;

pub trait Periodic: ComponentName + Send + Sync + Sized + 'static {
    fn create(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> PeriodicCreateFuture<Self>;
    fn step(&mut self) -> PeriodicFuture;
}

pub struct PeriodicComponent<P: Periodic> {
    inner: Mutex<Option<JoinHandle<()>>>,
    stop: watch::Sender<bool>,
    _marker: PhantomData<P>,
}

impl<P: Periodic> CreateComponent for PeriodicComponent<P> {
    fn create(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<Arc<Self>, ComponentError>> {
        Box::pin(async move {
            let period = time::Duration::from_secs(config.get_u64("period")?);
            let mut periodic = P::create(resolver, config).await?;
            let (stop, mut will_stop) = watch::channel(false);

            let inner = tokio::spawn(async move {
                let mut interval = time::interval(period);

                loop {
                    tokio::select! {
                        _ = interval.tick() => (),
                        _ = will_stop.changed() => ()
                    };

                    if *will_stop.borrow() {
                        break;
                    }

                    periodic.step();
                }

                println!("Stopping periodic: {}", P::component_name());
            });

            Ok(Arc::new(Self {
                inner: Mutex::new(Some(inner)),
                stop,
                _marker: PhantomData,
            }))
        })
    }
}

impl<P: Periodic> DestroyComponent for PeriodicComponent<P> {
    fn destroy(&self) -> ComponentFuture<()> {
        fn ready() -> ComponentFuture<()> {
            Box::pin(std::future::ready(()))
        }

        if self.stop.send(true).is_err() {
            println!("Failed to gracefully stop {}", P::component_name());
            return ready();
        }

        enum AcquisitionError {
            PeriodicPanicked,
            PeriodicIsNone,
        }

        let inner = self
            .inner
            .lock()
            .map_err(|_| AcquisitionError::PeriodicPanicked)
            .map(|mut guard| guard.take())
            .and_then(|i| i.ok_or(AcquisitionError::PeriodicIsNone));

        let inner = match inner {
            Ok(inner) => inner,
            Err(err) => {
                match err {
                    AcquisitionError::PeriodicPanicked => {
                        println!("{} periodic panicked", P::component_name())
                    }
                    AcquisitionError::PeriodicIsNone => println!(
                        "{} periodic task is None. Was it running?",
                        P::component_name()
                    ),
                }
                return ready();
            }
        };

        Box::pin(async move {
            if inner.await.is_err() {
                println!("Failed to join periodic task of {}", P::component_name())
            }
        })
    }
}

impl<P: Periodic> ComponentName for PeriodicComponent<P> {
    fn component_name() -> &'static str {
        P::component_name()
    }
}

impl<P: Periodic> Component for PeriodicComponent<P> {}
