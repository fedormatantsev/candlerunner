mod component;
mod component_store;
mod config;
mod resolution;

pub use crate::component::{
    init_err, Component, ComponentError, ComponentFuture, ComponentName, CreateComponent,
    DestroyComponent,
};

pub use crate::component_store::{ComponentStore, ComponentStoreBuilder};
pub use crate::config::{ConfigError, ConfigProvider};
pub use crate::resolution::ComponentResolver;

pub mod prelude {
    pub use super::{
        Component, ComponentError, ComponentFuture, ComponentName, ComponentResolver,
        ConfigProvider, CreateComponent, DestroyComponent,
    };
}
