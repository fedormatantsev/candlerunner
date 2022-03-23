use std::path::Path;
use thiserror::Error;
use yaml_rust::{
    yaml::{Hash, Yaml},
    ScanError, YamlLoader,
};

use component_store::{ConfigError, ConfigProvider};

#[derive(Error, Debug)]
pub enum YamlConfigProviderError {
    #[error("Failed to read configuration file")]
    ReadFailed {
        #[from]
        source: std::io::Error,
    },

    #[error("Failed to parse configuration file")]
    ParseFailed {
        #[from]
        source: ScanError,
    },

    #[error("Invalid configuration format ({reason})")]
    InvalidFormat { reason: String },
}

const DOC_TYPE_REAL: &str = "real";
const DOC_TYPE_INTEGER: &str = "integer";
const DOC_TYPE_STRING: &str = "string";
const DOC_TYPE_BOOLEAN: &str = "boolean";
const DOC_TYPE_ARRAY: &str = "array";
const DOC_TYPE_DICT: &str = "dict";
const DOC_TYPE_ALIAS: &str = "alias";
const DOC_TYPE_NULL: &str = "null";
const DOC_TYPE_BAD_VALUE: &str = "bad value";

fn get_doc_type(doc: &Yaml) -> &'static str {
    match doc {
        Yaml::Real(_) => DOC_TYPE_REAL,
        Yaml::Integer(_) => DOC_TYPE_INTEGER,
        Yaml::String(_) => DOC_TYPE_STRING,
        Yaml::Boolean(_) => DOC_TYPE_BOOLEAN,
        Yaml::Array(_) => DOC_TYPE_ARRAY,
        Yaml::Hash(_) => DOC_TYPE_DICT,
        Yaml::Alias(_) => DOC_TYPE_ALIAS,
        Yaml::Null => DOC_TYPE_NULL,
        Yaml::BadValue => DOC_TYPE_BAD_VALUE,
    }
}

///
/// Provides configuration from yaml file.
///
pub struct YamlConfigProvider {
    inner: Hash,
    path: Vec<String>,
}

impl YamlConfigProvider {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, YamlConfigProviderError> {
        let buf = std::fs::read_to_string(file_path.as_ref())?;
        let mut yaml = YamlLoader::load_from_str(&buf)?;

        let doc = yaml
            .drain(..)
            .next()
            .ok_or_else(|| YamlConfigProviderError::InvalidFormat {
                reason: "no root object".into(),
            })?;

        let inner =
            doc.as_hash()
                .cloned()
                .ok_or_else(|| YamlConfigProviderError::InvalidFormat {
                    reason: "root is not a dict".into(),
                })?;

        Ok(Self {
            inner,
            path: vec!["#".to_string()],
        })
    }

    fn get_doc(&self, name: &str) -> Result<&Yaml, ConfigError> {
        self.inner
            .get(&Yaml::from_str(name))
            .ok_or_else(|| ConfigError::NotFound {
                path: self.get_path(name).join("/"),
            })
    }

    fn get_path(&self, name: &str) -> Vec<String> {
        let mut path = self.path.clone();
        path.push(name.to_string());

        path
    }
}

impl ConfigProvider for YamlConfigProvider {
    fn get_str(&self, name: &str) -> Result<&str, ConfigError> {
        let doc = self.get_doc(name)?;

        doc.as_str().ok_or_else(|| ConfigError::TypeMismatch {
            path: self.get_path(name).join("/"),
            expected_ty: DOC_TYPE_STRING,
            actual_ty: get_doc_type(doc),
        })
    }

    fn get_u64(&self, name: &str) -> Result<u64, ConfigError> {
        self.get_i64(name).map(|i| i as u64)
    }

    fn get_i64(&self, name: &str) -> Result<i64, ConfigError> {
        let doc = self.get_doc(name)?;

        doc.as_i64().ok_or_else(|| ConfigError::TypeMismatch {
            path: self.get_path(name).join("/"),
            expected_ty: DOC_TYPE_INTEGER,
            actual_ty: get_doc_type(doc),
        })
    }

    fn get_f64(&self, name: &str) -> Result<f64, ConfigError> {
        let doc = self.get_doc(name)?;

        doc.as_f64()
            .or_else(|| doc.as_i64().map(|i| i as f64))
            .ok_or_else(|| ConfigError::TypeMismatch {
                path: self.get_path(name).join("/"),
                expected_ty: DOC_TYPE_REAL,
                actual_ty: get_doc_type(doc),
            })
    }

    fn get_bool(&self, name: &str) -> Result<bool, ConfigError> {
        let doc = self.get_doc(name)?;

        doc.as_bool().ok_or_else(|| ConfigError::TypeMismatch {
            path: self.get_path(name).join("/"),
            expected_ty: DOC_TYPE_BOOLEAN,
            actual_ty: get_doc_type(doc),
        })
    }

    fn get_subconfig(&self, name: &str) -> Result<Box<dyn ConfigProvider>, ConfigError> {
        let doc = self.get_doc(name)?;
        let inner = doc.as_hash().ok_or_else(|| ConfigError::TypeMismatch {
            path: self.get_path(name).join("/"),
            expected_ty: DOC_TYPE_DICT,
            actual_ty: get_doc_type(doc),
        })?;

        Ok(Box::new(YamlConfigProvider {
            inner: inner.clone(),
            path: self.get_path(name),
        }))
    }
}
