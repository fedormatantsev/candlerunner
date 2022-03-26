use std::any::{Any, TypeId};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;

use crate::config::{ConfigError, ConfigProvider};
use crate::resolution::ComponentResolver;

pub type ComponentFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[derive(Error, Debug)]
pub enum ComponentError {
    #[error(
        "Component `{source_component}` requires `{dependency_component}`, 
        but `{source_component}` is a dependency of `{dependency_component}`"
    )]
    DependencyCycle {
        source_component: String,
        dependency_component: String,
    },

    #[error("Failed to initialize component")]
    InitializationFailed {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Configuration error")]
    ConfigurationError {
        #[from]
        source: ConfigError,
    },
}

pub fn init_err<E: std::error::Error + Send + Sync + 'static>(err: E) -> ComponentError {
    ComponentError::InitializationFailed {
        source: Box::new(err),
    }
}

pub trait CreateComponent {
    fn create(
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<Arc<Self>, ComponentError>>;
}

pub trait DestroyComponent {
    fn destroy(&self) -> ComponentFuture<()> {
        Box::pin(std::future::ready(()))
    }
}

pub trait ComponentName {
    fn component_name() -> &'static str;
}

pub trait Component:
    CreateComponent + DestroyComponent + ComponentName + Send + Sync + 'static
{
}

pub type AnyComponent = Arc<dyn Any + Send + Sync + 'static>;
pub type ComponentDtor = Arc<dyn DestroyComponent + Send + Sync + 'static>;

#[derive(Copy, Clone)]
pub struct ComponentInfo {
    pub name: &'static str,
    pub type_id: TypeId,
}

impl PartialEq<Self> for ComponentInfo {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for ComponentInfo {}

impl ComponentInfo {
    pub fn new<C: Component>() -> Self {
        ComponentInfo {
            name: C::component_name(),
            type_id: TypeId::of::<C>(),
        }
    }
}
