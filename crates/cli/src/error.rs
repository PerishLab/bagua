#[derive(Debug)]
pub(crate) struct Error {
    code: String,
    message: String,
    request: Option<u64>,
}

impl Error {
    pub fn endpoint(message: impl Into<String>) -> Self {
        Self {
            code: "endpoint".into(),
            message: message.into(),
            request: None,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self {
            code: "configuration".into(),
            message: message.into(),
            request: None,
        }
    }

    pub fn at(mut self, request: u64) -> Self {
        self.request = Some(request);
        self
    }
    pub fn correlation(&self, fallback: u64) -> u64 {
        self.request.unwrap_or(fallback)
    }
    pub fn code(&self) -> &str {
        &self.code
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<protocol::Failure> for Error {
    fn from(error: protocol::Failure) -> Self {
        Self {
            request: None,
            code: error.code().into(),
            message: error.message().into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self {
            request: None,
            code: "io".into(),
            message: error.to_string(),
        }
    }
}

impl From<protocol::wire::Fault> for Error {
    fn from(error: protocol::wire::Fault) -> Self {
        Self {
            request: None,
            code: error.code,
            message: error.message,
        }
    }
}
