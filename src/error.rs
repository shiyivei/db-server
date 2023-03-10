//! 错误类型
//!

// 使用thiserror定义自己的错误类型,用Error宏,新的Error是枚举，包含了所有可能的错误
// 注意用法
// 使用实现
use crate::Value;
use sled;
use std::io::Error as IoError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KvError {
    // 使用字段属性定义错误内容
    #[error("Not found for table: {0},key: {1}")]
    NotFound(String, String),
    #[error("Cannot parse command: `{0}`")]
    InvalidCommand(String),
    #[error("Cannot convert value {:0} to {1}")]
    ConvertError(Value, &'static str),
    #[error("Cannot process command {0} with table: {1}, key: {2}. Error: {}")]
    StorageError(&'static str, String, String, String),

    #[error("I/O error")]
    IoError(#[from] std::io::Error),

    //使用第三发库的具体Error类型
    #[error("Failed to encode protobuf message")]
    EncodeError(#[from] prost::EncodeError),
    #[error("Failed to decode protobuf message")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Invalid command error")]
    FmtError(#[from] std::fmt::Error),

    #[error("frame error")]
    FrameError,
    #[error("Failed to access sled db")]
    SledError(#[from] sled::Error),

    // #[error("I/O error")]
    // IoError(#[from] std::io::Error),
    #[error("certificate parse error server: {0}, cert: {1}")]
    CertificateParseError(&'static str, &'static str),

    #[error("TLS error")]
    TlsError(#[from] tokio_rustls::rustls::TLSError),

    // #[error("Yamux Connection error")]
    // YamuxConnectionError(#[from] yamux::ConnectionError),
    #[error("Parse config error")]
    ConfigError(#[from] toml::de::Error),
}

// impl From<FmtError> for KvError {
//     fn from(value: FmtError) -> Self {
//         KvError::InvalidCommand("Invalid Command".to_string())
//     }
// }

// impl From<IoError> for KvError {
//     fn from(value: IoError) -> Self {
//         KvError::InvalidCommand("Invalid Command".to_string())
//     }
// }

// impl From<sled::Error> for KvError {
//     fn from(value: sled::Error) -> Self {
//         match value {
//             sled::Error::CollectionNotFound(_) => KvError::NotFound(
//                 "can not find table".to_string(),
//                 "can not find key".to_string(),
//             ),
//             sled::Error::Unsupported(_) => KvError::InvalidCommand("Invalid Command".to_string()),
//             _ => KvError::Internal("Internal error".to_string()),
//         }
//     }
// }
