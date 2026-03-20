use snafu::Snafu;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("{}", source))]
    DbBuilder {
        source: turso::Error,
    },

    #[snafu(display("{}", source))]
    DbConnect {
        source: turso::Error,
    },

    #[snafu(display("{}", source))]
    DbExecute {
        source: turso::Error,
    },

    #[snafu(display("{}", source))]
    DbPrepare {
        source: turso::Error,
    },

    #[snafu(display("{}", source))]
    DbStatement {
        source: turso::Error,
    },

    #[snafu(display("{}", source))]
    DbRow {
        source: turso::Error,
    },

    #[snafu(display("{}", source))]
    DbValue {
        source: turso::Error,
    },

    #[snafu(display("{}", source))]
    DbTransaction {
        source: turso::Error,
    },

    ParseDate {
        source: chrono::ParseError,
    },

    #[snafu(display("{}", msg))]
    Validation {
        msg: String,
    },

    #[snafu(display("{}", msg))]
    Whatever {
        msg: String,
    },
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
