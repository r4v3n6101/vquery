use super::packet::error::Error as PacketError;
use thiserror::Error;

type NomError<'a> = nom::Err<nom::error::Error<&'a [u8]>>;
type NomErrorOwned = nom::Err<nom::error::Error<Vec<u8>>>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Packet(#[from] PacketError),
    #[error(transparent)]
    A2SParse(NomErrorOwned),
}

impl From<NomError<'_>> for Error {
    fn from(error: NomError<'_>) -> Self {
        Error::A2SParse(error.map(|e| nom::error::make_error(e.input.to_vec(), e.code)))
    }
}

pub type QueryResult<T> = Result<T, Error>;
