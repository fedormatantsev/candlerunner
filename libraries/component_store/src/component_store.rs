use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::component::{
    AnyComponent, Component, ComponentDtor, ComponentError, ComponentFuture, ComponentInfo,
};
use crate::config::ConfigProvider;
use crate::resolution::{ComponentDAG, ComponentResolver, ResolutionContext};

///
/// Type-errased constructor for component.
/// Note: we can't use `CreateComponent` trait with dynamic dispatch.
///
trait ComponentFactory: Send + Sync + 'static {
    fn create(
        &self,
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<(AnyComponent, ComponentDtor, ComponentInfo), ComponentError>>;

    fn component_info(&self) -> ComponentInfo;
}

struct DefaultComponentFactory<C: Component> {
    _marker: PhantomData<C>,
}

impl<C: Component> Default for DefaultComponentFactory<C> {
    fn default() -> Self {
        DefaultComponentFactory {
            _marker: PhantomData,
        }
    }
}

impl<C: Component> ComponentFactory for DefaultComponentFactory<C> {
    fn create(
        &self,
        resolver: ComponentResolver,
        config: Box<dyn ConfigProvider>,
    ) -> ComponentFuture<Result<(AnyComponent, ComponentDtor, ComponentInfo), ComponentError>> {
        let info = self.component_info();
        Box::pin(async move {
            let component = Arc::new(C::init(resolver, config).await?);
            let dtor: ComponentDtor = component.clone();
            let component: AnyComponent = component;
            Ok((component, dtor, info))
        })
    }

    fn component_info(&self) -> ComponentInfo {
        ComponentInfo::new::<C>()
    }
}

///
/// Destroys components in DAG.
///
#[derive(Default)]
struct DAGDestructor {
    destructors: Vec<(ComponentInfo, ComponentDtor)>,
}

impl DAGDestructor {
    pub fn new(mut destructors: Vec<(ComponentInfo, ComponentDtor)>, dag: ComponentDAG) -> Self {
        let depth = destructors.iter().fold(
            HashMap::<TypeId, usize>::default(),
            |mut state, (info, _)| {
                state.insert(
                    info.type_id,
                    usize::MAX - dag.get_transitive_dependencies(&info.type_id).len(),
                );

                state
            },
        );

        destructors.sort_by_cached_key(|(info, _)| {
            *depth.get(&info.type_id).cloned().get_or_insert(usize::MAX)
        });

        Self { destructors }
    }

    pub async fn destroy(self) {
        for (info, dtor) in self.destructors {
            println!("Shutting down {}", info.name);
            dtor.shutdown().await;
        }
    }
}

///
/// Holds component singletons and manages their dependencies.
///
pub struct ComponentStore {
    components: HashMap<TypeId, AnyComponent>,
    destructor: DAGDestructor,
}

#[derive(Default)]
pub struct ComponentStoreBuilder {
    known_types: HashSet<TypeId>,
    factories: Vec<Box<dyn ComponentFactory>>,
}

impl ComponentStoreBuilder {
    ///
    /// Registers a new component type in component store
    ///
    pub fn register<C: Component>(mut self) -> anyhow::Result<Self> {
        if !self.known_types.insert(TypeId::of::<C>()) {
            return Err(anyhow::anyhow!(
                "Component {} is already registered",
                C::component_name()
            ));
        }

        self.factories
            .push(Box::new(DefaultComponentFactory::<C>::default()));
        Ok(self)
    }

    ///
    /// Creates component store.
    /// Effectively invokes `Component::create()` for each registered component.
    ///
    /// May return error if any component failed to initialize.
    ///
    pub async fn build(
        self,
        config_provider: Box<dyn ConfigProvider>,
    ) -> Result<ComponentStore, ComponentError> {
        if self.factories.is_empty() {
            return Ok(ComponentStore {
                components: Default::default(),
                destructor: Default::default(),
            });
        }

        let context = Arc::new(ResolutionContext::new(self.known_types));

        let (sender, mut receiver) = mpsc::channel(self.factories.len());

        for factory in self.factories {
            let sender = sender.clone();
            let resolver = ComponentResolver::new(context.clone(), factory.component_info());

            let config = config_provider.get_subconfig(factory.component_info().name)?;

            tokio::spawn(async move {
                println!("Creating {}", factory.component_info().name);
                let res = factory.create(resolver, config).await;
                if sender.send(res).await.is_err() {
                    panic!("Unable to send component to queue")
                }
            });
        }

        drop(sender);

        let mut destructors: Vec<(ComponentInfo, ComponentDtor)> = Default::default();

        while let Some(res) = receiver.recv().await {
            let (component, dtor, component_info) = res?;
            context.add_component(&component_info, component);
            destructors.push((component_info, dtor));
        }

        let (components, dag) = context.finalize();
        let destructor = DAGDestructor::new(destructors, dag);

        Ok(ComponentStore {
            components,
            destructor,
        })
    }
}

impl ComponentStore {
    ///
    /// Creates component store builder.
    ///
    pub fn builder() -> ComponentStoreBuilder {
        ComponentStoreBuilder::default()
    }

    ///
    /// Tries to resolve component in component store.
    ///
    pub fn resolve<C: Component>(&self) -> Option<Arc<C>> {
        self.components
            .get(&TypeId::of::<C>())
            .map(|component| component.clone().downcast::<C>().unwrap())
    }

    ///
    /// Stops execution of held components.
    ///
    pub async fn destroy(self) {
        self.destructor.destroy().await;
    }
}

#[cfg(test)]
mod tests {
    use crate::{ComponentName, InitComponent, ShutdownComponent};

    use super::*;

    struct TestComponentA {}

    struct TestComponentB {
        _a: Arc<TestComponentA>,
    }

    struct TestComponentC {
        _a: Arc<TestComponentA>,
        _b: Arc<TestComponentB>,
    }

    struct TestComponentD {
        _a: Arc<TestComponentA>,
        _b: Arc<TestComponentB>,
        _c: Arc<TestComponentC>,
    }

    impl InitComponent for TestComponentA {
        fn init(
            _: ComponentResolver,
            _: Box<dyn ConfigProvider>,
        ) -> ComponentFuture<Result<Self, ComponentError>> {
            Box::pin(std::future::ready(Ok(Self {})))
        }
    }

    impl ShutdownComponent for TestComponentA {}

    impl ComponentName for TestComponentA {
        fn component_name() -> &'static str {
            "test-component-a"
        }
    }

    impl Component for TestComponentA {}

    impl InitComponent for TestComponentB {
        fn init(
            resolver: ComponentResolver,
            _: Box<dyn ConfigProvider>,
        ) -> ComponentFuture<Result<Self, ComponentError>> {
            Box::pin(async move {
                let a = resolver.resolve::<TestComponentA>().await?;
                Ok(Self { _a: a })
            })
        }
    }

    impl ShutdownComponent for TestComponentB {}

    impl ComponentName for TestComponentB {
        fn component_name() -> &'static str {
            "test-component-b"
        }
    }

    impl Component for TestComponentB {}

    impl InitComponent for TestComponentC {
        fn init(
            resolver: ComponentResolver,
            _: Box<dyn ConfigProvider>,
        ) -> ComponentFuture<Result<Self, ComponentError>> {
            Box::pin(async move {
                let a = resolver.resolve::<TestComponentA>().await?;
                let b = resolver.resolve::<TestComponentB>().await?;
                Ok(Self { _a: a, _b: b })
            })
        }
    }

    impl ShutdownComponent for TestComponentC {}

    impl ComponentName for TestComponentC {
        fn component_name() -> &'static str {
            "test-component-c"
        }
    }

    impl Component for TestComponentC {}

    impl InitComponent for TestComponentD {
        fn init(
            resolver: ComponentResolver,
            _: Box<dyn ConfigProvider>,
        ) -> ComponentFuture<Result<Self, ComponentError>> {
            Box::pin(async move {
                let a = resolver.resolve::<TestComponentA>().await?;
                let b = resolver.resolve::<TestComponentB>().await?;
                let c = resolver.resolve::<TestComponentC>().await?;
                Ok(Self {
                    _a: a,
                    _b: b,
                    _c: c,
                })
            })
        }
    }

    impl ShutdownComponent for TestComponentD {}

    impl ComponentName for TestComponentD {
        fn component_name() -> &'static str {
            "test-component-d"
        }
    }

    impl Component for TestComponentD {}

    #[derive(Default)]
    struct TestConfigProvider {}

    impl ConfigProvider for TestConfigProvider {
        fn get_str(&self, _: &str) -> Result<&str, crate::config::ConfigError> {
            unreachable!()
        }

        fn get_u64(&self, _: &str) -> Result<u64, crate::config::ConfigError> {
            unreachable!()
        }

        fn get_i64(&self, _: &str) -> Result<i64, crate::config::ConfigError> {
            unreachable!()
        }

        fn get_f64(&self, _: &str) -> Result<f64, crate::config::ConfigError> {
            unreachable!()
        }

        fn get_bool(&self, _: &str) -> Result<bool, crate::config::ConfigError> {
            unreachable!()
        }

        fn get_subconfig(
            &self,
            _: &str,
        ) -> Result<Box<dyn ConfigProvider>, crate::config::ConfigError> {
            Ok(Box::new(TestConfigProvider {}))
        }
    }

    #[tokio::test]
    async fn test_basic() -> Result<(), anyhow::Error> {
        let config = Box::new(TestConfigProvider::default());

        let component_store = ComponentStore::builder()
            .register::<TestComponentA>()
            .register::<TestComponentB>()
            .register::<TestComponentC>()
            .register::<TestComponentD>()
            .build(config)
            .await?;

        assert!(component_store.resolve::<TestComponentA>().is_some());
        assert!(component_store.resolve::<TestComponentB>().is_some());
        assert!(component_store.resolve::<TestComponentC>().is_some());
        assert!(component_store.resolve::<TestComponentD>().is_some());

        let dtor_order: Vec<_> = component_store
            .destructor
            .destructors
            .iter()
            .map(|(info, _)| info.type_id)
            .collect();

        assert_eq!(
            vec![
                TypeId::of::<TestComponentD>(),
                TypeId::of::<TestComponentC>(),
                TypeId::of::<TestComponentB>(),
                TypeId::of::<TestComponentA>()
            ],
            dtor_order
        );

        component_store.destroy().await;

        Ok(())
    }
}
