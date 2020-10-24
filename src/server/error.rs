use super::packet::error::Error as PacketError;
use thiserror::Error;

type NomError<'a> = nom::Err<(&'a [u8], nom::error::ErrorKind)>;
type NomErrorOwned = nom::Err<(Vec<u8>, nom::error::ErrorKind)>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Packet(#[from] PacketError),
    #[error(transparent)]
    A2SParse(NomErrorOwned),
}

impl From<NomError<'_>> for Error {
    fn from(error: NomError<'_>) -> Self {
        Error::A2SParse(error.to_owned())
    }
}

pub type QueryResult<T> = Result<T, Error>;
