use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct AccountId(pub String);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Environment {
    Sandbox,
    Production,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AccessLevel {
    Unspecified,
    FullAccess,
    ReadOnly,
    NoAccess,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub access_level: AccessLevel,
    pub environment: Environment,
}
