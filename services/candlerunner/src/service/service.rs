use std::{collections::HashMap, net::SocketAddr};

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
        position_manager::PositionManagerInstanceDefinition,
        strategy::StrategyInstanceDefinition,
    },
};

use super::error::ServiceError;

fn list_instruments_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(warp::reply::Json,)>> {
    let instrument_cache = component_store
        .resolve::<components::InstrumentCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `InstrumentsCache`"))?;

    let list_instruments = warp::get()
        .and(warp::path!("list-instruments"))
        .map(move || {
            let instrument_cache = instrument_cache.state();
            let instruments: Vec<_> = instrument_cache.values().collect();
            warp::reply::json(&instruments)
        })
        .boxed();

    Ok(list_instruments)
}

fn list_strategies_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let strategy_registry = component_store
        .resolve::<components::StrategyRegistry>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `StrategyRegistry`"))?;

    let list_strategies = warp::get()
        .and(warp::path!("list-strategies"))
        .map(move || {
            let definitions: Vec<_> = strategy_registry.definitions().collect();
            warp::reply::json(&definitions)
        })
        .boxed();

    Ok(list_strategies)
}

fn list_position_managers_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let position_manager_registry = component_store
        .resolve::<components::PositionManagerRegistry>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `PositionManagerRegistry`"))?;

    let list_position_managers = warp::get()
        .and(warp::path!("list-position-managers"))
        .map(move || {
            let definitions: Vec<_> = position_manager_registry.definitions().collect();
            warp::reply::json(&definitions)
        })
        .boxed();

    Ok(list_position_managers)
}

fn instantiate_strategy_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let strategy_registry = component_store
        .resolve::<components::StrategyRegistry>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `StrategyRegistry`"))?;

    let strategy_cache = component_store
        .resolve::<components::StrategyCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `StrategyCache`"))?;

    let mongo = component_store
        .resolve::<components::Mongo>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `Mongo`"))?;

    let instantiate_strategy = warp::post()
        .and(warp::path!("instantiate-strategy"))
        .and(warp::body::json())
        .then(move |def: StrategyInstanceDefinition| {
            let strategy_registry = strategy_registry.clone();
            let mongo = mongo.clone();
            let strategy_cache = strategy_cache.clone();

            let view = async move {
                strategy_registry
                    .validate_instance_definition(&def)
                    .map_err(ServiceError::from)?;

                if let Err(err) = mongo.write_strategy_instance(&def).await {
                    println!(
                        "Failed to write strategy instance to mongo: {}",
                        err.to_string()
                    );
                    return Err(ServiceError::InternalError(err.to_string()));
                }

                strategy_cache
                    .force_update(Some(Duration::from_millis(500)))
                    .await;

                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({})),
                    StatusCode::OK,
                ))
            };

            async move {
                match view.await {
                    Ok(reply) => reply,
                    Err(err) => warp::reply::WithStatus::<warp::reply::Json>::from(err),
                }
            }
        })
        .boxed();

    Ok(instantiate_strategy)
}

fn instantiate_position_manager_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let position_manager_registry = component_store
        .resolve::<components::PositionManagerRegistry>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `PositionManagerRegistry`"))?;

    let position_manager_cache = component_store
        .resolve::<components::PositionManagerCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `PositionManagerCache`"))?;

    let mongo = component_store
        .resolve::<components::Mongo>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `Mongo`"))?;

    let instantiate_position_manager = warp::post()
        .and(warp::path!("instantiate-position-manager"))
        .and(warp::body::json())
        .then(move |def: PositionManagerInstanceDefinition| {
            let position_manager_registry = position_manager_registry.clone();
            let position_manager_cache = position_manager_cache.clone();
            let mongo = mongo.clone();

            let view = async move {
                position_manager_registry
                    .validate_instance_definition(&def)
                    .map_err(ServiceError::from)?;

                if let Err(err) = mongo.write_position_manager_instance(&def).await {
                    println!(
                        "Failed to write position manager instance to mongo: {}",
                        err.to_string()
                    );
                    return Err(ServiceError::InternalError(err.to_string()));
                }

                position_manager_cache
                    .force_update(Some(Duration::from_millis(500)))
                    .await;

                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({})),
                    StatusCode::OK,
                ))
            };

            async move {
                match view.await {
                    Ok(reply) => reply,
                    Err(err) => warp::reply::WithStatus::<warp::reply::Json>::from(err),
                }
            }
        })
        .boxed();

    Ok(instantiate_position_manager)
}

fn list_strategy_instances_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let strategy_cache = component_store
        .resolve::<components::StrategyCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `StrategyCache`"))?;

    let list_strategy_instances = warp::get()
        .and(warp::path!("list-strategy-instances"))
        .map(move || {
            let strategy_cache = strategy_cache.state();

            let payload: HashMap<_, _> = strategy_cache
                .iter()
                .map(|(instance_id, (def, _))| (instance_id.clone(), def.clone()))
                .collect();

            warp::reply::json(&payload)
        })
        .boxed();

    Ok(list_strategy_instances)
}

fn list_position_manager_instances_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let position_manager_cache = component_store
        .resolve::<components::PositionManagerCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `PositionManagerCache`"))?;

    let list_position_manager_instances = warp::get()
        .and(warp::path!("list-position-manager-instances"))
        .map(move || {
            let position_manager_cache = position_manager_cache.state();

            let payload: HashMap<_, _> = position_manager_cache
                .iter()
                .map(|(instance_id, (def, _))| (instance_id.clone(), def.clone()))
                .collect();

            warp::reply::json(&payload)
        })
        .boxed();

    Ok(list_position_manager_instances)
}

fn list_accounts_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let accounts_cache = component_store
        .resolve::<components::AccountsCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `AccountsCache`"))?;

    let list_accounts = warp::get()
        .and(warp::path!("list-accounts"))
        .map(move || {
            let accounts_cache = accounts_cache.state();
            let payload: Vec<_> = accounts_cache.iter().map(|(_, acc)| acc.clone()).collect();

            warp::reply::json(&payload)
        })
        .boxed();

    Ok(list_accounts)
}

fn open_sandbox_account_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let accounts_cache = component_store
        .resolve::<components::AccountsCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `AccountsCache`"))?;

    let positions_cache = component_store
        .resolve::<components::PositionsCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `PositionsCache`"))?;

    let tinkoff_client = component_store
        .resolve::<components::TinkoffClient>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `TinkoffClient`"))?;

    let open_sandbox_account = warp::post()
        .and(warp::path!("open-sandbox-account"))
        .then(move || {
            let accounts_cache = accounts_cache.clone();
            let positions_cache = positions_cache.clone();
            let tinkoff_client = tinkoff_client.clone();

            let view = async move {
                tinkoff_client
                    .open_sandbox_account()
                    .await
                    .map_err(ServiceError::from)?;

                accounts_cache
                    .force_update(Some(Duration::from_millis(500)))
                    .await;

                positions_cache
                    .force_update(Some(Duration::from_millis(500)))
                    .await;

                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({})),
                    StatusCode::OK,
                ))
            };

            async {
                let reply: Result<warp::reply::WithStatus<warp::reply::Json>, ServiceError> =
                    view.await;

                match reply {
                    Ok(reply) => reply,
                    Err(err) => warp::reply::WithStatus::<warp::reply::Json>::from(err),
                }
            }
        })
        .boxed();

    Ok(open_sandbox_account)
}

#[derive(Serialize, Deserialize)]
struct CloseAccountRequest {
    account_id: AccountId,
}

fn close_sandbox_account_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let accounts_cache = component_store
        .resolve::<components::AccountsCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `AccountsCache`"))?;

    let tinkoff_client = component_store
        .resolve::<components::TinkoffClient>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `TinkoffClient`"))?;

    let close_sandbox_account = warp::post()
        .and(warp::path!("close-sandbox-account"))
        .and(warp::body::json())
        .then(move |request: ListPositionsRequest| {
            let accounts_cache = accounts_cache.clone();
            let tinkoff_client = tinkoff_client.clone();

            let view = async move {
                let accounts_cache_state = accounts_cache.state();
                let account = accounts_cache_state
                    .get(&request.account_id)
                    .ok_or_else(|| ServiceError::NotFound("Account not found".to_owned()))?;

                if account.environment != Environment::Sandbox {
                    return Err(ServiceError::BadRequest(
                        "Unable to close production account".to_owned(),
                    ));
                }

                tinkoff_client
                    .close_sandbox_account(account)
                    .await
                    .map_err(ServiceError::from)?;

                accounts_cache
                    .force_update(Some(Duration::from_millis(500)))
                    .await;

                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({})),
                    StatusCode::OK,
                ))
            };

            async {
                let reply: Result<warp::reply::WithStatus<warp::reply::Json>, ServiceError> =
                    view.await;

                match reply {
                    Ok(reply) => reply,
                    Err(err) => warp::reply::WithStatus::<warp::reply::Json>::from(err),
                }
            }
        })
        .boxed();

    Ok(close_sandbox_account)
}

#[derive(Serialize, Deserialize, Clone)]
struct ListPositionsRequest {
    account_id: AccountId,
}

fn list_positions_view(
    component_store: &ComponentStore,
) -> anyhow::Result<warp::filters::BoxedFilter<(impl warp::Reply,)>> {
    let positions_cache = component_store
        .resolve::<components::PositionsCache>()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve `PositionsCache`"))?;

    let list_positions = warp::post()
        .and(warp::path!("list-positions"))
        .and(warp::body::json())
        .map(move |request: ListPositionsRequest| {
            let positions_cache = positions_cache.state();

            let payload = match positions_cache.get(&request.account_id) {
                Some(p) => p,
                None => {
                    return ServiceError::NotFound("Account not found".to_owned()).into();
                }
            };

            warp::reply::with_status(warp::reply::json(&payload), StatusCode::OK)
        })
        .boxed();

    Ok(list_positions)
}

pub async fn serve(addr: SocketAddr, component_store: &ComponentStore) -> anyhow::Result<()> {
    let cors = warp::cors()
        .allow_methods(&[Method::GET, Method::POST, Method::OPTIONS])
        .allow_any_origin()
        .allow_headers(["access-control-allow-origin", "content-type"]);

    let routes = warp::any()
        .and(
            list_instruments_view(component_store)?
                .or(list_strategies_view(component_store)?)
                .or(list_position_managers_view(component_store)?)
                .or(list_strategy_instances_view(component_store)?)
                .or(instantiate_strategy_view(component_store)?)
                .or(list_accounts_view(component_store)?)
                .or(open_sandbox_account_view(component_store)?)
                .or(close_sandbox_account_view(component_store)?)
                .or(list_positions_view(component_store)?)
                .or(instantiate_position_manager_view(component_store)?)
                .or(list_position_manager_instances_view(component_store)?),
        )
        .with(cors);

    println!("Listening on {}", addr);
    warp::serve(routes).run(addr).await;

    Ok(())
}
