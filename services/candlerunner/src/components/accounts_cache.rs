use std::{collections::HashMap, sync::Arc};

use component_store::prelude::*;
use periodic_component::{Periodic, PeriodicComponent};

use crate::{
    components,
    models::account::{Account, AccountId},
};

pub struct AccountsCachePeriodic {
    tinkoff_client: Arc<components::TinkoffClient>,
}

impl ComponentName for AccountsCachePeriodic {
    fn component_name() -> &'static str {
        "accounts-cache"
    }
}

impl Periodic for AccountsCachePeriodic {
    type State = HashMap<AccountId, Account>;

    fn init(
        resolver: ComponentResolver,
        _: Box<dyn ConfigProvider>,
    ) -> periodic_component::PeriodicCreateFuture<(Self, Self::State)> {
        Box::pin(async move {
            let tinkoff_client = resolver.resolve::<components::TinkoffClient>().await?;

            let periodic = Self { tinkoff_client };

            Ok((periodic, Self::State::default()))
        })
    }

    fn step(&mut self, state: Arc<Self::State>) -> periodic_component::PeriodicFuture<Self::State> {
        Box::pin(async move {
            match self.tinkoff_client.list_accounts().await {
                Ok(instruments) => {
                    let res: HashMap<_, _> = instruments
                        .into_iter()
                        .map(|acc| (acc.id.clone(), acc))
                        .collect();

                    return Ok(Arc::new(res));
                }
                Err(err) => {
                    println!("Failed to update `accounts-cache`: {}", err);
                }
            };

            Ok(state)
        })
    }
}

pub type AccountsCache = PeriodicComponent<AccountsCachePeriodic>;
