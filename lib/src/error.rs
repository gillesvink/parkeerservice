use std::{convert::Infallible, num::ParseIntError};

#[cfg(feature = "python-bindings")]
use pyo3::PyErr;
#[cfg(feature = "python-bindings")]
use pyo3::exceptions::PyRuntimeError;

/// Main error object for this crate, it inherits from the crates used as well externally
/// so it can be used everywhere.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}")]
    ChronoParse(#[from] chrono::ParseError),
    #[error("{0}")]
    ParseInt(#[from] ParseIntError),
    #[error("{0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("{0}")]
    Regex(#[from] regex::Error),
    #[error("Invalid index fetched")]
    InvalidIndex,
    #[error("{0}")]
    Credentials(String),
    #[error("Customer ID could not be parsed from page.")]
    CustomerIdNotAvailable,
    #[error("No permits could be found for this account.")]
    NoPermitsFound,
    #[error("Cookies could not be loaded from disk: {0}")]
    CookieLoading(String),
    #[error("Verification token could not be found on page.")]
    VerificationTokenMissing,
    #[error("Environment variable missing: {0}")]
    EnvironmentVariable(String),
    #[error("{0}")]
    Custom(String),
    #[error("{0}")]
    Infillable(#[from] Infallible),
}

#[cfg(feature = "python-bindings")]
impl std::convert::From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
