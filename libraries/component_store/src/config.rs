use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration field `{path}` is missing")]
    NotFound { path: String },

    #[error(
        "Configuration field `{path}` is of type `{actual_ty}`, but `{expected_ty}` was expected"
    )]
    TypeMismatch {
        path: String,
        expected_ty: &'static str,
        actual_ty: &'static str,
    },
}

pub trait ConfigProvider: Send + Sync + 'static {
    fn get_str(&self, name: &str) -> Result<&str, ConfigError>;
    fn get_u64(&self, name: &str) -> Result<u64, ConfigError>;
    fn get_i64(&self, name: &str) -> Result<i64, ConfigError>;
    fn get_f64(&self, name: &str) -> Result<f64, ConfigError>;
    fn get_bool(&self, name: &str) -> Result<bool, ConfigError>;
    fn get_subconfig(&self, name: &str) -> Result<Box<dyn ConfigProvider>, ConfigError>;
}
