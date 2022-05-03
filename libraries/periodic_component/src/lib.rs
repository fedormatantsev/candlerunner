use std::cell::UnsafeCell;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::{future::Future, marker::PhantomData};

use tokio::{sync::mpsc, sync::Notify, task::JoinHandle, time};

use component_store::prelude::*;

pub type PeriodicFuture<'periodic, S> =
    Pin<Box<dyn Future<Output = anyhow::Result<Arc<S>>> + Send + 'periodic>>;
pub type PeriodicCreateFuture<P> = Pin<Box<dyn Future<Output = Result<P, ComponentError>> + Send>>;
pub trait Periodic: ComponentName + Send + Sync + Sized + 'static {
    type State: Send + Sync + 'static;

    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> PeriodicCreateFuture<(Self, Self::State)>;

    fn step(
        &mut self,
        state: Arc<Self::State>,
    ) -> PeriodicFuture<'_, Self::State>;
}

struct StateHolder<S> {
    state: RwLock<UnsafeCell<Arc<S>>>,
}

impl<S> StateHolder<S> {
    pub fn new(state: Arc<S>) -> Self {
        Self {
            state: RwLock::new(UnsafeCell::new(state)),
        }
    }

    pub fn set(&self, state: Arc<S>) {
        match self.state.write() {
            Ok(cell) => unsafe { *cell.get() = state },
            Err(_) => panic!("StateHolder was poisoned"),
        }
    }

    pub fn get(&self) -> Arc<S> {
        match self.state.read() {
            Ok(cell) => unsafe { (*cell.get()).clone() },
            Err(_) => panic!("StateHolder was poisoned"),
        }
    }
}

unsafe impl<S> Sync for StateHolder<S> {}
unsafe impl<S> Send for StateHolder<S> {}

enum Control {
    // Stop periodic updates.
    Stop,

    // Perform usual periodic update.
    Update,

    // Perform forced update.
    ForceUpdate(Arc<Notify>),
}

pub struct PeriodicComponent<P: Periodic> {
    state: Arc<StateHolder<P::State>>,
    inner: Mutex<Option<JoinHandle<()>>>,
    control: mpsc::Sender<Control>,
    _marker: PhantomData<P>,
}

impl<P: Periodic> PeriodicComponent<P> {
    pub fn state(&self) -> Arc<P::State> {
        self.state.get()
    }

    pub async fn force_update(&self, timeout: Option<time::Duration>) {
        let notify = Arc::new(Notify::default());

        if let Err(err) = self
            .control
            .send(Control::ForceUpdate(notify.clone()))
            .await
        {
            print!(
                "Failed to initiate forced update for periodic {}: {}",
                P::component_name(),
                err
            );

            return;
        }

        let sleep = match timeout {
            Some(duration) => time::sleep(duration),
            None => { return; },
        };

        tokio::select! {
            _ = sleep => { println!("Forced update timed out for periodic {}", P::component_name()); },
            _ = notify.notified() => ()
        };
    }
}

impl<P: Periodic> InitComponent for PeriodicComponent<P> {
    fn init(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<Self, ComponentError>> {
        println!("init {}", P::component_name());
        Box::pin(async move {
            let period = time::Duration::from_secs(config.get_u64("update_period")?);

            let (mut periodic, init_state) = P::init(resolver, config).await?;
            let init_state = Arc::new(init_state);

            let state = match periodic.step(init_state.clone()).await {
                Ok(new_state) => new_state,
                Err(err) => {
                    println!("{} failed to init state: {}", P::component_name(), err);
                    init_state
                }
            };

            let state_holder = Arc::new(StateHolder::new(state));

            let (control, mut control_receiver) = mpsc::channel(32);

            let inner_state = state_holder.clone();
            let inner = tokio::spawn(async move {
                let mut interval = time::interval(period);
                interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

                // First tick fires instantly. We have already called step() on initialization.
                interval.tick().await;

                loop {
                    let first_ctrl = tokio::select! {
                        _ = interval.tick() => Some(Control::Update),
                        ctrl = control_receiver.recv() => ctrl
                    };

                    let first_ctrl = match first_ctrl {
                        Some(ctrl) => ctrl,
                        None => {
                            println!(
                                "Control channel was closed for periodic {}",
                                P::component_name()
                            );
                            break;
                        }
                    };

                    let mut will_stop = false;
                    let mut will_force_update = false;
                    let mut notifies: Vec<Arc<Notify>> = Default::default();

                    let mut visit = |ctrl: Control| match ctrl {
                        Control::Stop => {
                            will_stop = true;
                        }
                        Control::Update => (),
                        Control::ForceUpdate(notify) => {
                            notifies.push(notify);
                            will_force_update = true;
                        }
                    };

                    visit(first_ctrl);

                    while let Ok(next_ctrl) = control_receiver.try_recv() {
                        visit(next_ctrl);
                    }

                    if will_stop {
                        break;
                    }
                    if will_force_update {
                        println!(
                            "Performing forced update for periodic {}",
                            P::component_name()
                        );
                    }

                    match periodic.step(inner_state.get()).await {
                        Ok(state) => inner_state.set(state),
                        Err(err) => {
                            println!("Periodic {} update failed: {}", P::component_name(), err)
                        }
                    }

                    for notify in notifies {
                        notify.notify_one();
                    }
                }

                println!("Stopping periodic: {}", P::component_name());
            });

            println!("Created periodic {}", P::component_name());
            Ok(Self {
                state: state_holder,
                inner: Mutex::new(Some(inner)),
                control,
                _marker: PhantomData,
            })
        })
    }
}

impl<P: Periodic> ShutdownComponent for PeriodicComponent<P> {
    fn shutdown(&self) -> ComponentFuture<()> {
        fn ready() -> ComponentFuture<()> {
            Box::pin(std::future::ready(()))
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

        let control = self.control.clone();
        Box::pin(async move {
            if control.send(Control::Stop).await.is_err() {
                println!(
                    "Failed to gracefully shutdown periodic {}",
                    P::component_name()
                );

                return;
            }

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
