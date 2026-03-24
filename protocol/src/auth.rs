use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMsg {
    Register { username: String, password: String },
    Login { username: String, password: String },
    Logout,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMsg {
    RegisterOk { account_id: String },
    LoginOk { session_token: String },
    LogoutOk,
    Pong,
    Error { code: ErrorCode, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorCode {
    InvalidCredentials,
    UsernameTaken,
    SessionExpired,
    InternalError,
}
