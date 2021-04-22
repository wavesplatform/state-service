use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    LoadConfigFailed(envy::Error),
    InvalidMessage(String),
    DbError(diesel::result::Error),
    ConnectionPoolError(r2d2::Error),
    OpenTelemetryTraceError(opentelemetry::trace::TraceError),
    TracingSubscriberTryInitError(tracing_subscriber::util::TryInitError),
    TracingSubscriberFilterParseError(tracing_subscriber::filter::ParseError),
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

impl From<opentelemetry::trace::TraceError> for Error {
    fn from(err: opentelemetry::trace::TraceError) -> Self {
        OpenTelemetryTraceError(err)
    }
}

impl From<tracing_subscriber::util::TryInitError> for Error {
    fn from(err: tracing_subscriber::util::TryInitError) -> Self {
        TracingSubscriberTryInitError(err)
    }
}

impl From<tracing_subscriber::filter::ParseError> for Error {
    fn from(err: tracing_subscriber::filter::ParseError) -> Self {
        TracingSubscriberFilterParseError(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadConfigFailed(err) => write!(f, "LoadConfigFailed: {}", err),
            InvalidMessage(message) => write!(f, "InvalidMessage: {}", message),
            DbError(err) => write!(f, "DbError: {}", err),
            ConnectionPoolError(err) => write!(f, "ConnectionPoolError: {}", err),
            OpenTelemetryTraceError(err) => write!(f, "OpenTelemetryTraceError: {}", err),
            TracingSubscriberTryInitError(err) => {
                write!(f, "TracingSubscriberTryInitError: {}", err)
            }
            TracingSubscriberFilterParseError(err) => {
                write!(f, "TracingSubscriberFilterParseError: {}", err)
            }
        }
    }
}

impl Into<String> for Error {
    fn into(self) -> String {
        self.to_string()
    }
}
