use std::path::PathBuf;

use axum::extract::rejection::JsonRejection;
use axum::response::IntoResponse;
use axum::{body::Body, http::StatusCode, response::Response};
use snafu::Snafu;
use yaas::role::{InvalidPermissionsError, InvalidRolesError, InvalidScopesError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Error reading config file: {}", source))]
    ConfigFile { source: std::io::Error },

    #[snafu(display("Error parsing config file: {}", source))]
    ConfigParse { source: toml::de::Error },

    #[snafu(display("Unable to create upload dir: {}", source))]
    UploadDir { source: std::io::Error },

    #[snafu(display("Config error: {}", msg))]
    Config { msg: String },

    #[snafu(display("{}", source))]
    Db { source: db::Error },

    #[snafu(display("{} - {}", msg, source))]
    PasswordPrompt { msg: String, source: std::io::Error },

    #[snafu(display("{}", msg))]
    Validation { msg: String },

    #[snafu(display("Maximum number of clients reached: 10"))]
    MaxClientsReached,

    #[snafu(display("Maximum number of users reached: 100"))]
    MaxUsersReached,

    #[snafu(display("Maximum number of buckets reached: 50"))]
    MaxBucketsReached,

    #[snafu(display("Maximum number of directories reached: 1000"))]
    MaxDirsReached,

    #[snafu(display("Maximum number of files reached: 1000"))]
    MaxFilesReached,

    #[snafu(display("Google Cloud error: {}", msg))]
    Google { msg: String },

    #[snafu(display("{}", msg))]
    BadRequest { msg: String },

    #[snafu(display("Invalid protobuf message"))]
    BadProtobuf,

    #[snafu(display("{}", msg))]
    Forbidden { msg: String },

    #[snafu(display("{}", msg))]
    JsonRejection { msg: String, source: JsonRejection },

    #[snafu(display("{}", msg))]
    MissingUploadFile { msg: String },

    #[snafu(display("Unable to create file: {:?}", path))]
    CreateFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("File type not allowed"))]
    FileTypeNotAllowed,

    #[snafu(display("{}", msg))]
    NotFound { msg: String },

    #[snafu(display("{}", source))]
    Password { source: password::Error },

    #[snafu(display("Invalid auth token"))]
    InvalidAuthToken,

    #[snafu(display("Insufficient auth scope"))]
    InsufficientAuthScope,

    #[snafu(display("No auth token"))]
    NoAuthToken,

    #[snafu(display("Invalid client"))]
    InvalidClient,

    #[snafu(display("Requires authentication"))]
    RequiresAuth,

    #[snafu(display("{}", msg))]
    HashPassword { msg: String },

    #[snafu(display("{}", msg))]
    VerifyPasswordHash { msg: String },

    #[snafu(display("Invalid username or password"))]
    InvalidPassword,

    #[snafu(display("Inactive user"))]
    InactiveUser,

    #[snafu(display("User not found"))]
    UserNotFound,

    #[snafu(display("User has no organization"))]
    UserNoOrg,

    #[snafu(display("{}", source))]
    InvalidRoles { source: InvalidRolesError },

    #[snafu(display("{}", source))]
    InvalidPermissions { source: InvalidPermissionsError },

    #[snafu(display("{}", source))]
    InvalidScopes { source: InvalidScopesError },

    #[snafu(display("Failed to parse JWT claims: {}", source))]
    JwtClaimsParse { source: serde_json::Error },

    #[snafu(display("Failed to serialize JSON: {}", source))]
    JsonSerialize { source: serde_json::Error },

    #[snafu(display("OAuth redirect_uri mismatch"))]
    RedirectUriMistmatch,

    #[snafu(display("OAuth app not registered in the org"))]
    AppNotRegistered,

    #[snafu(display("OAuth state mismatch"))]
    OauthStateMismatch,

    #[snafu(display("OAuth code invalid"))]
    OauthCodeInvalid,

    #[snafu(display("OAuth scopes invalid"))]
    OauthInvalidScopes,

    #[snafu(display("{}", msg))]
    Whatever { msg: String },
}

// Allow string slices to be converted to Error
impl From<&str> for Error {
    fn from(val: &str) -> Self {
        Self::Whatever {
            msg: val.to_string(),
        }
    }
}

impl From<String> for Error {
    fn from(val: String) -> Self {
        Self::Whatever { msg: val }
    }
}

/// Allow Error to be converted to StatusCode
impl From<&Error> for StatusCode {
    fn from(err: &Error) -> Self {
        match err {
            Error::Validation { .. } => StatusCode::BAD_REQUEST,
            Error::MaxClientsReached => StatusCode::BAD_REQUEST,
            Error::MaxUsersReached => StatusCode::BAD_REQUEST,
            Error::MaxBucketsReached => StatusCode::BAD_REQUEST,
            Error::MaxDirsReached => StatusCode::BAD_REQUEST,
            Error::MaxFilesReached => StatusCode::BAD_REQUEST,
            Error::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Error::Forbidden { .. } => StatusCode::FORBIDDEN,
            Error::JsonRejection { .. } => StatusCode::BAD_REQUEST,
            Error::MissingUploadFile { .. } => StatusCode::BAD_REQUEST,
            Error::CreateFile { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::FileTypeNotAllowed => StatusCode::BAD_REQUEST,
            Error::NotFound { .. } => StatusCode::NOT_FOUND,
            Error::InvalidAuthToken => StatusCode::UNAUTHORIZED,
            Error::InsufficientAuthScope => StatusCode::UNAUTHORIZED,
            Error::NoAuthToken => StatusCode::UNAUTHORIZED,
            Error::InvalidClient => StatusCode::UNAUTHORIZED,
            Error::RequiresAuth => StatusCode::UNAUTHORIZED,
            Error::InvalidPassword => StatusCode::UNAUTHORIZED,
            Error::InactiveUser => StatusCode::UNAUTHORIZED,
            Error::UserNotFound => StatusCode::UNAUTHORIZED,
            Error::InvalidRoles { .. } => StatusCode::BAD_REQUEST,
            Error::InvalidPermissions { .. } => StatusCode::BAD_REQUEST,
            Error::InvalidScopes { .. } => StatusCode::BAD_REQUEST,
            Error::RedirectUriMistmatch => StatusCode::UNAUTHORIZED,
            Error::AppNotRegistered => StatusCode::UNAUTHORIZED,
            Error::OauthStateMismatch => StatusCode::UNAUTHORIZED,
            Error::OauthCodeInvalid => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

fn get_error_code(error: Error) -> Option<String> {
    // Only specific errors get an error code
    match error {
        Error::NoAuthToken => Some("NoAuthToken".to_string()),
        Error::InvalidPassword => Some("InvalidPassword".to_string()),
        Error::InactiveUser => Some("InactiveUser".to_string()),
        Error::UserNotFound => Some("UserNotFound".to_string()),
        _ => None,
    }
}

// Allow errors to be rendered as response
impl IntoResponse for Error {
    fn into_response(self) -> Response<Body> {
        let status_code = StatusCode::from(&self);
        let message = format!("{}", self);

        // Build a dummy response
        let mut res = Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .unwrap();

        res.extensions_mut().insert(ErrorInfo {
            status_code,
            message,
            error_code: get_error_code(self),
        });

        res
    }
}

#[derive(Debug, Clone)]
pub struct ErrorInfo {
    pub status_code: StatusCode,
    pub message: String,
    pub error_code: Option<String>,
}
