use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    LoadConfigFailed(envy::Error),
    InvalidMessage(String),
    DbError(diesel::result::Error),
    ConnectionPoolError(r2d2::Error),
}

use Error::*;

impl From<r2d2::Error> for Error {
    fn from(v: r2d2::Error) -> Self {
        ConnectionPoolError(v)
    }
}

impl From<diesel::result::Error> for Error {
    fn from(v: diesel::result::Error) -> Self {
        DbError(v)
    }
}

impl From<envy::Error> for Error {
    fn from(err: envy::Error) -> Self {
        LoadConfigFailed(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadConfigFailed(err) => write!(f, "LoadConfigFailed: {}", err),
            InvalidMessage(message) => write!(f, "InvalidMessage: {}", message),
            DbError(err) => write!(f, "DbError: {}", err),
            ConnectionPoolError(err) => write!(f, "ConnectionPoolError: {}", err),
        }
    }
}

impl Into<String> for Error {
    fn into(self) -> String {
        self.to_string()
    }
}
