use uuid::Uuid;

pub trait InstanceId {
    fn id(&self) -> Uuid;
}
