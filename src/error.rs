use std::fmt;

use thiserror::Error;

/// Represents errors that can occur in this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// Returned when an invalid URL is used.
    #[error("failed to parse URL: {0}")]
    ParseUrl(url::ParseError),

    /// Returned when sending an HTTP request fails.
    #[error("failed to send HTTP request: {0}")]
    SendRequest(ReqwestDesensitizedError),

    /// Returned when Telegram Bot API returns an error.
    #[error("Telegram API error: {0:?}")]
    TelegramApi(Option<String>),
}

/// Represents the result type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ReqwestDesensitizedError(reqwest::Error);

impl From<reqwest::Error> for ReqwestDesensitizedError {
    fn from(value: reqwest::Error) -> Self {
        Self(value.without_url())
    }
}

impl fmt::Display for ReqwestDesensitizedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
