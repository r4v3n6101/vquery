use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    io::Error as IOError,
};

type NomError<'a> = nom::Err<(&'a [u8], nom::error::ErrorKind)>;
type NomErrorOwned = nom::Err<(Vec<u8>, nom::error::ErrorKind)>;

#[derive(Debug)]
pub enum QueryError {
    IOErr(IOError),
    NomErr(NomErrorOwned),
}

impl From<IOError> for QueryError {
    fn from(err: IOError) -> Self {
        QueryError::IOErr(err)
    }
}

impl From<NomError<'_>> for QueryError {
    fn from(err: NomError<'_>) -> Self {
        QueryError::NomErr(err.to_owned())
    }
}

impl Display for QueryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            QueryError::IOErr(ref err) => write!(f, "IO error: {}", err),
            QueryError::NomErr(ref err) => write!(f, "Nom error: {}", err),
        }
    }
}

impl Error for QueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            QueryError::IOErr(ref err) => Some(err),
            QueryError::NomErr(ref err) => Some(err),
        }
    }
}

pub type QueryResult<T> = Result<T, QueryError>;
