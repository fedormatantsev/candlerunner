use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct AccountId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Environment {
    Sandbox,
    Production,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AccessLevel {
    Unspecified,
    FullAccess,
    ReadOnly,
    NoAccess,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub access_level: AccessLevel,
    pub environment: Environment,
}
