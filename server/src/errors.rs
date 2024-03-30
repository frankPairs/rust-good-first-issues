use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum RustGoodFirstIssuesError {
    ValidationError(String),
    ReqwestError(reqwest::Error),
    ParseUrlError(url::ParseError),
}

impl std::fmt::Display for RustGoodFirstIssuesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustGoodFirstIssuesError::ValidationError(err) => {
                write!(f, "Bad request: {}", err)
            }
            RustGoodFirstIssuesError::ReqwestError(err) => {
                write!(f, "External API request error: {}", err)
            }
            RustGoodFirstIssuesError::ParseUrlError(err) => {
                write!(f, "Parse url error: {}", err)
            }
        }
    }
}

impl IntoResponse for RustGoodFirstIssuesError {
    fn into_response(self) -> Response {
        match self {
            RustGoodFirstIssuesError::ValidationError(err) => {
                tracing::error!("Bad request:  {}", err);

                (StatusCode::BAD_REQUEST, err).into_response()
            }
            RustGoodFirstIssuesError::ReqwestError(err) => {
                tracing::error!("External API request error:  {}", err);

                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            RustGoodFirstIssuesError::ParseUrlError(err) => {
                tracing::error!("Parse url error:  {}", err);

                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
        }
    }
}
