use axum::{
    http::StatusCode,
    response::IntoResponse,
    response::Response,
};
use crate::error::{ChainError, TransactionError};

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    InternalError(String),
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::InternalError(e.to_string())
    }
}

impl From<TransactionError> for ApiError {
    fn from(e: TransactionError) -> Self {
        ApiError::BadRequest(e.to_string())
    }
}

impl From<ChainError> for ApiError {
    fn from(e: ChainError) -> Self {
        match e {
            ChainError::InsufficientBalance { .. } | ChainError::Transaction(_) => {
                ApiError::BadRequest(e.to_string())
            }
            ChainError::Io(_) | ChainError::Serialization(_) => {
                ApiError::InternalError(e.to_string())
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg)    => (StatusCode::BAD_REQUEST, msg),
            ApiError::NotFound(msg)      => (StatusCode::NOT_FOUND, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, message).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_request_returns_400() {
        let response = ApiError::BadRequest("oops".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn not_found_returns_404() {
        let response = ApiError::NotFound("gone".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn internal_error_returns_500() {
        let response = ApiError::InternalError("boom".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn from_transaction_error_is_bad_request() {
        let err: ApiError = TransactionError::InvalidSignature.into();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn from_transaction_error_message_preserved() {
        let err: ApiError = TransactionError::InvalidSignatureLength.into();
        let ApiError::BadRequest(msg) = err else { panic!("wrong variant") };
        assert_eq!(msg, "invalid signature length (expected 64 bytes)");
    }

    #[test]
    fn from_chain_error_insufficient_balance_is_bad_request() {
        let err: ApiError = ChainError::InsufficientBalance { available: 10, required: 100 }.into();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn from_chain_error_transaction_is_bad_request() {
        let err: ApiError = ChainError::Transaction(TransactionError::InvalidPublicKey).into();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn from_chain_error_io_is_internal() {
        use std::io;
        let err: ApiError = ChainError::Io(io::Error::new(io::ErrorKind::Other, "disk")).into();
        assert!(matches!(err, ApiError::InternalError(_)));
    }

    #[test]
    fn from_chain_error_serialization_is_internal() {
        let json_err: serde_json::Error = serde_json::from_str::<i32>("bad").unwrap_err();
        let err: ApiError = ChainError::Serialization(json_err).into();
        assert!(matches!(err, ApiError::InternalError(_)));
    }
}
