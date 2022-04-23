use tonic::metadata::{AsciiMetadataKey, AsciiMetadataValue};
use tonic::service::Interceptor;
use tonic::{Request, Status};

#[derive(Clone)]
pub struct AuthorizationInterceptor {
    auth_key: AsciiMetadataKey,
    auth_value: AsciiMetadataValue,
}

impl Interceptor for AuthorizationInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let meta = request.metadata_mut();
        meta.insert(self.auth_key.clone(), self.auth_value.clone());

        Ok(request)
    }
}

impl AuthorizationInterceptor {
    pub fn new<S: AsRef<str>>(
        auth_token: S,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let key = AsciiMetadataKey::from_static("authorization");
        let value =
            AsciiMetadataValue::from_str(format!("Bearer {}", auth_token.as_ref()).as_str())?;

        Ok(Self {
            auth_key: key,
            auth_value: value,
        })
    }
}
