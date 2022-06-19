use warp::hyper::StatusCode;

use crate::models::strategy::InstantiateStrategyError;

pub enum ServiceError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl From<ServiceError> for warp::reply::WithStatus<warp::reply::Json> {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::NotFound(msg) => {
                let payload = serde_json::json!({ "message": msg });

                warp::reply::with_status(warp::reply::json(&payload), StatusCode::NOT_FOUND)
            }
            ServiceError::BadRequest(msg) => {
                let payload = serde_json::json!({ "message": msg });

                warp::reply::with_status(warp::reply::json(&payload), StatusCode::BAD_REQUEST)
            }
            ServiceError::InternalError(msg) => {
                let payload = serde_json::json!({ "message": msg });

                warp::reply::with_status(
                    warp::reply::json(&payload),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        }
    }
}

impl From<InstantiateStrategyError> for ServiceError {
    fn from(err: InstantiateStrategyError) -> Self {
        match err {
            InstantiateStrategyError::NotFound(_) => ServiceError::NotFound(err.to_string()),
            InstantiateStrategyError::FailedToInstantiate(_) => {
                ServiceError::InternalError(err.to_string())
            }
            InstantiateStrategyError::ParamValidationFailed { source: _ } => {
                ServiceError::BadRequest(err.to_string())
            }
        }
    }
}

impl From<anyhow::Error> for ServiceError {
    fn from(err: anyhow::Error) -> Self {
        ServiceError::InternalError(err.to_string())
    }
}
