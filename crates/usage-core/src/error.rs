use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// No saved sign-in, or the proxy rejected the token (expired or revoked).
    SignedOut,
    /// No proxy URL configured yet.
    NotConfigured,
    /// The OS secure storage is unavailable or refused access.
    Keyring(String),
    /// macOS needs the user to allow Keychain access, and this process may not ask.
    NeedsApproval,
    /// The proxy answered with an error status.
    Http { status: u16, message: String },
    /// The proxy could not be reached.
    Network(String),
    /// The proxy answered with a body we could not read.
    Parse(String),
    Io(String),
    Invalid(String),
}

impl Error {
    /// Stable short tag used in the cache file and logs.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::SignedOut => "signed_out",
            Error::NotConfigured => "not_configured",
            Error::Keyring(_) => "keyring",
            Error::NeedsApproval => "needs_approval",
            Error::Http { .. } => "http",
            Error::Network(_) => "network",
            Error::Parse(_) => "parse",
            Error::Io(_) => "io",
            Error::Invalid(_) => "invalid",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SignedOut => write!(f, "Signed out of LiteLLM. Run `ccline login` to sign in again."),
            Error::NotConfigured => write!(f, "No LiteLLM proxy configured. Run `ccline login --url <proxy url>`."),
            Error::Keyring(m) => write!(f, "Secure storage unavailable: {m}"),
            Error::NeedsApproval => write!(
                f,
                "Keychain access needs your approval. Run `ccline status` in a terminal and choose Always Allow."
            ),
            Error::Http { status, message } => write!(f, "LiteLLM returned {status}: {message}"),
            Error::Network(m) => write!(f, "Can't reach LiteLLM: {m}"),
            Error::Parse(m) => write!(f, "Unexpected response from LiteLLM: {m}"),
            Error::Io(m) => write!(f, "File error: {m}"),
            Error::Invalid(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(e.to_string())
    }
}
