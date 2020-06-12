use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    io::Error as IOError,
};

#[derive(Debug)]
pub enum QueryError {
    IOErr(IOError),
    UnknownHeader(u8, &'static str),
}

impl From<IOError> for QueryError {
    fn from(err: IOError) -> QueryError {
        QueryError::IOErr(err)
    }
}

impl Display for QueryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            QueryError::IOErr(ref err) => write!(f, "IO error: {}", err),
            QueryError::UnknownHeader(ref header, ref expected) => {
                write!(f, "Wrong header {}, expected {}", header, expected)
            }
        }
    }
}

impl Error for QueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            QueryError::IOErr(ref err) => Some(err),
            QueryError::UnknownHeader(_, _) => None,
        }
    }
}

pub type QueryResult<T> = Result<T, QueryError>;
