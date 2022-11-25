use thiserror::Error;
#[derive(Error, Debug)]
pub enum InternalError {
    GroupChunkReadAsDataChunk,
    DataChunkReadAsGroupChunk,
    OutOfBounds,
    Unreachable,
    InvalidChunkBy,
}
impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InternalError::GroupChunkReadAsDataChunk => write!(f, "GroupChunk read as data."),
            InternalError::DataChunkReadAsGroupChunk => write!(f, "DataChunk read as group."),
            InternalError::OutOfBounds => write!(f, "Out of bounds access."),
            InternalError::Unreachable => write!(f, "Unreachable code was reached."),
            InternalError::InvalidChunkBy => write!(
                f,
                "Attempted to chunk data in vector that is not compatible to the target type."
            ),
        }
    }
}

#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("Invalid Alembic File")]
    InvalidAlembicFile,
    #[error("Unsupported Alembic File")]
    UnsupportedAlembicFile,

    #[error("Cannot parse schema on object structure")]
    IncompatibleSchema,

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

#[derive(Error, Debug)]
pub enum UserError {
    OutOfBounds,
    InvalidParameter,
}
impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::OutOfBounds => write!(f, "Out of bounds"),
            UserError::InvalidParameter => write!(f, "Invalid parameter"),
        }
    }
}

#[derive(Error, Debug)]
pub enum OgawaError {
    #[error("Internal error")]
    InternalError(#[from] InternalError),

    #[error("Parsing error")]
    ParsingError(#[from] ParsingError),

    #[error("User error")]
    UserError(#[from] UserError),

    #[error("I/O error")]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
pub type Result<V, E = OgawaError> = ::std::result::Result<V, E>;
