use std::{collections::HashMap, sync::Arc};

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent};

use crate::{
    components,
    models::{account::AccountId, positions::AccountPositions},
};

pub struct PositionsCachePeriodic {
    tinkoff_client: Arc<components::TinkoffClient>,
    accounts_cache: Arc<components::AccountsCache>,
}

impl ComponentName for PositionsCachePeriodic {
    fn component_name() -> &'static str {
        "positions-cache"
    }
}

impl Periodic for PositionsCachePeriodic {
    type State = HashMap<AccountId, AccountPositions>;

    fn init(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> periodic_component::PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(async move {
            let tinkoff_client = resolver.resolve::<components::TinkoffClient>().await?;
            let accounts_cache = resolver.resolve::<components::AccountsCache>().await?;

            let periodic = Self {
                tinkoff_client,
                accounts_cache,
            };

            Ok((periodic, Self::State::default()))
        })
    }

    fn step(&mut self, state: Arc<Self::State>) -> periodic_component::PeriodicFuture<Self::State> {
        Box::pin(async move {
            let accounts_cache = self.accounts_cache.state();

            let mut next_state = Self::State::default();

            for (_, account) in accounts_cache.iter() {
                let positions = match self.tinkoff_client.list_positions(account).await {
                    Ok(p) => p,
                    Err(err) => {
                        println!(
                            "Failed to retrieve positions for account {}: {}",
                            &account.id.0, err
                        );
                        state
                            .get(&account.id)
                            .cloned()
                            .unwrap_or_else(|| AccountPositions {
                                currencies: Default::default(),
                                positions: Default::default(),
                            })
                    }
                };

                next_state.insert(account.id.clone(), positions);
            }

            Ok(Arc::new(next_state))
        })
    }
}

pub type PositionsCache = PeriodicComponent<PositionsCachePeriodic>;
