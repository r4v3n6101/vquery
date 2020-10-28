use thiserror::Error;

type NomError<'a> = nom::Err<(&'a [u8], nom::error::ErrorKind)>;
type NomErrorOwned = nom::Err<(Vec<u8>, nom::error::ErrorKind)>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(NomErrorOwned),
}

impl From<NomError<'_>> for Error {
    fn from(error: NomError<'_>) -> Self {
        Error::Parse(error.to_owned())
    }
}

pub type QueryResult<T> = Result<T, Error>;
