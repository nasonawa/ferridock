
use actix_web::{http::{header::ContentType, StatusCode}, HttpResponse, ResponseError};
use derive_builder::Builder;
use oci_spec::{distribution::{ErrorCode, ErrorInfoBuilder}, image::{self}};
use thiserror::Error;

use crate::storage::error::StorageError;


pub type Result<T> = std::result::Result<T,ApiError>;

#[derive(Builder)]
#[builder(field(private))]
pub struct ApiErrorResponse {
    message: String,
    content_type: ContentType,
    code: u16
}

impl Default for ApiErrorResponse {
    fn default() -> Self {
        Self { message: Default::default(), content_type: ContentType::plaintext(), code: Default::default() }
    }
}

#[derive(Debug,Error)]
pub enum ApiError{

    Storage(StorageError),

    InvalidManifestFormat(String),

    ContentNotFound{kind: image::MediaType, mesg: String },

    RangeIsNotStatisfied,

    BlobUploadUnknown
}

impl ApiError{

    fn get_status_code(&self)  -> ApiErrorResponse {
        match self {
            ApiError::Storage(_) => {
                ApiErrorResponseBuilder::default()
                    .code(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
                    .content_type(ContentType::plaintext())
                    .message("Internal Server Error".to_string())
                    .build().unwrap()
            },
            ApiError::InvalidManifestFormat(s) =>{
                let errror_json = ErrorInfoBuilder::default()
                .code(ErrorCode::ManifestInvalid)
                .message(s).build().unwrap();

                let msg = serde_json::to_string(&errror_json).unwrap();

                ApiErrorResponseBuilder::default()
                    .code(StatusCode::NOT_FOUND.as_u16())
                    .content_type(ContentType::json())
                    .message(msg)
                    .build().unwrap()
            }
            ApiError::ContentNotFound { kind , mesg} => {
                
                let mut errror_info = ErrorInfoBuilder::default()
                .message(mesg);
                
                if kind.eq(&image::MediaType::ImageManifest) {
                    errror_info = errror_info.code(ErrorCode::ManifestUnknown);
                }else {
                   errror_info = errror_info.code(ErrorCode::BlobUnknown);
                }

                let error_json = errror_info.build().unwrap();

                let msg = serde_json::to_string(&error_json).unwrap();

                ApiErrorResponseBuilder::default()
                    .code(StatusCode::NOT_FOUND.as_u16())
                    .content_type(ContentType::json())
                    .message(msg)
                    .build().unwrap()
            },
            ApiError::RangeIsNotStatisfied => {
                let errror_json = ErrorInfoBuilder::default()
                .code(ErrorCode::BlobUploadInvalid)
                .message("Invalid blob upload").build().unwrap();

                let msg = serde_json::to_string(&errror_json).unwrap();

                ApiErrorResponseBuilder::default()
                    .code(StatusCode::RANGE_NOT_SATISFIABLE.as_u16())
                    .content_type(ContentType::json())
                    .message(msg)
                    .build().unwrap()
            },
            ApiError::BlobUploadUnknown => {
                
                let errror_json = ErrorInfoBuilder::default()
                .code(ErrorCode::BlobUploadUnknown)
                .message("Blob upload unknown").build().unwrap();

                let msg = serde_json::to_string(&errror_json).unwrap();

                ApiErrorResponseBuilder::default()
                    .code(StatusCode::NOT_FOUND.as_u16())
                    .content_type(ContentType::json())
                    .message(msg)
                    .build().unwrap()
            },
        }
    }
}

impl ResponseError for ApiError {
  
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let ar = self.get_status_code();

        HttpResponse::build(StatusCode::from_u16(ar.code).unwrap())
        .content_type(ar.content_type)
        .body(ar.message)
    }
}

impl From<StorageError> for ApiError {
    fn from(value: StorageError) -> Self {
       match value {
        StorageError::RangeIsNotStatisfied => ApiError::RangeIsNotStatisfied,
        e => ApiError::Storage(e)
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self)
    }
}
