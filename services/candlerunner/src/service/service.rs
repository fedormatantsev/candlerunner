use std::{collections::HashMap, convert::Infallible, net::SocketAddr, sync::Arc};

use serde::{Deserialize, Serialize};
use std::time::Duration;
use warp::{
    hyper::{Method, StatusCode},
    Filter,
};

use component_store::ComponentStore;

use crate::{
    components,
    models::{
        account::{AccountId, Environment},
        strategy::{CreateStrategyError, StrategyInstanceDefinition},
    },
};

#[derive(Clone)]
struct ListInstruments {
    instrument_cache: Arc<components::InstrumentCache>,
}

impl ListInstruments {
    pub fn new(component_store: &ComponentStore) -> anyhow::Result<Self> {
        let instrument_cache = component_store
            .resolve::<components::InstrumentCache>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `InstrumentsCache`"))?;

        Ok(Self { instrument_cache })
    }

    pub fn view(&self) -> warp::reply::Json {
        let instruments_cache = self.instrument_cache.state();
        let instruments: Vec<_> = instruments_cache.values().collect();

        warp::reply::json(&instruments)
    }
}

#[derive(Clone)]
struct ListStrategies {
    strategy_registry: Arc<components::StrategyRegistry>,
}

impl ListStrategies {
    pub fn new(component_store: &ComponentStore) -> anyhow::Result<Self> {
        let strategy_registry = component_store
            .resolve::<components::StrategyRegistry>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `StrategyRegistry`"))?;

        Ok(Self { strategy_registry })
    }

    pub fn view(&self) -> warp::reply::Json {
        let definitions: Vec<_> = self.strategy_registry.definitions().collect();

        warp::reply::json(&definitions)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BadRequest {
    code: String,
    message: String,
}

impl warp::reject::Reject for BadRequest {}

impl From<CreateStrategyError> for BadRequest {
    fn from(err: CreateStrategyError) -> Self {
        match err {
            CreateStrategyError::StrategyNotFound(msg) => BadRequest {
                code: "STRATEGY_NOT_FOUND".to_owned(),
                message: msg,
            },
            CreateStrategyError::ParamMissing(msg) => BadRequest {
                code: "PARAM_MISSING".to_owned(),
                message: msg,
            },
            CreateStrategyError::InvalidParam(msg) => BadRequest {
                code: "INVALID_PARAM".to_owned(),
                message: msg,
            },
            CreateStrategyError::ParamTypeMismatch(msg) => BadRequest {
                code: "PARAM_TYPE_MISMATCH".to_owned(),
                message: msg,
            },
            CreateStrategyError::FailedToInstantiateStrategy(msg) => BadRequest {
                code: "FAILED_TO_INSTANTIATE".to_owned(),
                message: msg,
            },
        }
    }
}

#[derive(Debug)]
struct InternalError {
    msg: Option<String>,
}
impl warp::reject::Reject for InternalError {}

#[derive(Clone)]
struct InstantiateStrategy {
    strategy_registry: Arc<components::StrategyRegistry>,
    strategy_cache: Arc<components::StrategyCache>,
    mongo: Arc<components::Mongo>,
}

impl InstantiateStrategy {
    pub fn new(component_store: &ComponentStore) -> anyhow::Result<Self> {
        let strategy_registry = component_store
            .resolve::<components::StrategyRegistry>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `StrategyRegistry`"))?;

        let strategy_cache = component_store
            .resolve::<components::StrategyCache>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `StrategyCache`"))?;

        let mongo = component_store
            .resolve::<components::Mongo>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `Mongo`"))?;

        Ok(Self {
            strategy_registry,
            strategy_cache,
            mongo,
        })
    }

    pub async fn view(
        self,
        definition: StrategyInstanceDefinition,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        self.strategy_registry
            .validate_instance_definition(&definition)
            .map_err(|err| warp::reject::custom(BadRequest::from(err)))?;

        if let Err(err) = self.mongo.write_strategy_instance(&definition).await {
            println!(
                "Failed to write strategy instance to mongo: {}",
                err.to_string()
            );
            return Err(warp::reject::custom(InternalError {
                msg: Some(err.to_string()),
            }));
        }

        self.strategy_cache
            .force_update(Some(Duration::from_millis(500)))
            .await;

        Ok(warp::reply::reply())
    }
}

#[derive(Clone)]
struct ListStrategyInstances {
    strategy_cache: Arc<components::StrategyCache>,
}

impl ListStrategyInstances {
    pub fn new(component_store: &ComponentStore) -> anyhow::Result<Self> {
        let strategy_cache = component_store
            .resolve::<components::StrategyCache>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `StrategyCache`"))?;

        Ok(Self { strategy_cache })
    }

    pub fn view(&self) -> warp::reply::Json {
        let strategy_cache = self.strategy_cache.state();

        let payload: HashMap<_, _> = strategy_cache
            .iter()
            .map(|(instance_id, (def, _))| (instance_id.clone(), def.clone()))
            .collect();

        warp::reply::json(&payload)
    }
}

#[derive(Clone)]
struct ListAccounts {
    accounts_cache: Arc<components::AccountsCache>,
}

impl ListAccounts {
    pub fn new(component_store: &ComponentStore) -> anyhow::Result<Self> {
        let accounts_cache = component_store
            .resolve::<components::AccountsCache>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `AccountsCache`"))?;

        Ok(Self { accounts_cache })
    }

    pub fn view(&self) -> impl warp::Reply {
        let accounts_cache = self.accounts_cache.state();

        let payload: Vec<_> = accounts_cache.iter().map(|(_, acc)| acc.clone()).collect();

        warp::reply::json(&payload)
    }
}

#[derive(Clone)]
struct OpenSandboxAccount {
    accounts_cache: Arc<components::AccountsCache>,
    tinkoff_client: Arc<components::TinkoffClient>,
}

impl OpenSandboxAccount {
    pub fn new(component_store: &ComponentStore) -> anyhow::Result<Self> {
        let accounts_cache = component_store
            .resolve::<components::AccountsCache>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `AccountsCache`"))?;

        let tinkoff_client = component_store
            .resolve::<components::TinkoffClient>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `TinkoffClient`"))?;

        Ok(Self {
            accounts_cache,
            tinkoff_client,
        })
    }

    pub async fn view(self) -> Result<impl warp::Reply, warp::Rejection> {
        self.tinkoff_client
            .open_sandbox_account()
            .await
            .map_err(|err| {
                warp::reject::custom(InternalError {
                    msg: Some(err.to_string()),
                })
            })?;

        self.accounts_cache
            .force_update(Some(Duration::from_millis(500)))
            .await;

        Ok(warp::reply())
    }
}

#[derive(Clone)]
struct CloseSandboxAccount {
    accounts_cache: Arc<components::AccountsCache>,
    tinkoff_client: Arc<components::TinkoffClient>,
}

#[derive(Serialize, Deserialize)]
struct CloseAccountRequest {
    account_id: AccountId,
}

impl CloseSandboxAccount {
    pub fn new(component_store: &ComponentStore) -> anyhow::Result<Self> {
        let accounts_cache = component_store
            .resolve::<components::AccountsCache>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `AccountsCache`"))?;

        let tinkoff_client = component_store
            .resolve::<components::TinkoffClient>()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve `TinkoffClient`"))?;

        Ok(Self {
            accounts_cache,
            tinkoff_client,
        })
    }

    pub async fn view(self, account_id: AccountId) -> Result<impl warp::Reply, warp::Rejection> {
        let accounts_cache = self.accounts_cache.state();
        let account = accounts_cache
            .get(&account_id)
            .ok_or_else(|| warp::reject::not_found())?;

        if account.environment != Environment::Sandbox {
            return Err(warp::reject::custom(BadRequest {
                code: "NOT_SANDBOX_ACCOUNT".to_owned(),
                message: "Unable to close production account".to_owned(),
            }));
        }

        self.tinkoff_client
            .close_sandbox_account(account)
            .await
            .map_err(|err| {
                warp::reject::custom(InternalError {
                    msg: Some(err.to_string()),
                })
            })?;

        self.accounts_cache
            .force_update(Some(Duration::from_millis(500)))
            .await;

        Ok(warp::reply())
    }
}

pub async fn handle_rejection(
    err: warp::Rejection,
) -> std::result::Result<impl warp::Reply, Infallible> {
    let code;
    let json;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        json = warp::reply::json(&"Not Found".to_owned());
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        json = warp::reply::json(&e.to_string());
    } else if let Some(e) = err.find::<BadRequest>() {
        code = StatusCode::BAD_REQUEST;
        json = warp::reply::json(e);
    } else if let Some(e) = err.find::<InternalError>() {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        json = warp::reply::json(&format!(
            "Internal Server Error: {}",
            e.msg.as_ref().unwrap_or(&"reason unspecified".to_owned())
        ));
    } else if let Some(e) = err.find::<warp::reject::MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        json = warp::reply::json(&e.to_string());
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        json = warp::reply::json(&"Internal Server Error".to_owned());
    }

    Ok(warp::reply::with_status(json, code))
}

pub async fn serve(addr: SocketAddr, component_store: &ComponentStore) -> anyhow::Result<()> {
    let cors = warp::cors()
        .allow_methods(&[Method::GET, Method::POST, Method::OPTIONS])
        .allow_any_origin()
        .allow_headers(["access-control-allow-origin", "content-type"]);

    let hello_world = warp::path!().map(|| "Hello, World at root!");

    let list_instruments = ListInstruments::new(component_store)?;
    let list_instruments_view =
        warp::get().and(warp::path!("list-instruments").map(move || Ok(list_instruments.view())));

    let list_strategies = ListStrategies::new(component_store)?;
    let list_strategies_view =
        warp::get().and(warp::path!("list-strategies").map(move || Ok(list_strategies.view())));

    let instantiate_strategy = InstantiateStrategy::new(component_store)?;
    let instantiate_strategy_view = warp::post().and(
        warp::path!("instantiate-strategy")
            .and(warp::body::json())
            .and_then(move |def: StrategyInstanceDefinition| {
                instantiate_strategy.clone().view(def)
            }),
    );

    let list_strategy_instances = ListStrategyInstances::new(component_store)?;
    let list_strategy_instances_view = warp::get().and(
        warp::path!("list-strategy-instances").map(move || Ok(list_strategy_instances.view())),
    );

    let list_accounts = ListAccounts::new(component_store)?;
    let list_accounts_view =
        warp::get().and(warp::path!("list-accounts").map(move || list_accounts.view()));

    let open_sandbox_account = OpenSandboxAccount::new(component_store)?;
    let open_sandbox_account_view = warp::post()
        .and(warp::path!("open-sandbox-account"))
        .and_then(move || open_sandbox_account.clone().view());

    let close_sandbox_account = CloseSandboxAccount::new(component_store)?;
    let close_sandbox_account_view = warp::post()
        .and(warp::path!("close-sandbox-account"))
        .and(warp::body::json())
        .and_then(move |request: CloseAccountRequest| {
            close_sandbox_account.clone().view(request.account_id)
        });

    let routes = warp::any()
        .and(
            hello_world
                .or(list_instruments_view)
                .or(list_strategies_view)
                .or(list_strategy_instances_view)
                .or(instantiate_strategy_view)
                .or(list_accounts_view)
                .or(open_sandbox_account_view)
                .or(close_sandbox_account_view),
        )
        .recover(handle_rejection)
        .with(cors);

    println!("Listening on {}", addr);
    warp::serve(routes).run(addr).await;

    Ok(())
}
