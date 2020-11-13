use bzip2::Error as Bz2Error;
use thiserror::Error;

type NomError<'a> = nom::Err<nom::error::Error<&'a [u8]>>;
type NomErrorOwned = nom::Err<nom::error::Error<Vec<u8>>>;

#[derive(Debug)]
pub struct MultiHeader {
    pub uid: u32,
    pub total: usize,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Nom(NomErrorOwned),
    #[error("Expected -2, but found {0}")]
    WrongHeader(i32),
    #[error("Mismatched packet headers: expected {base:?}, found {wrong:?}")]
    Interrupted {
        base: MultiHeader,
        wrong: MultiHeader,
    },
    #[error(transparent)]
    Decompress(#[from] Bz2Error),
    #[error("Wrong crc32 of decompressed data: expected {0}, found {1}")]
    Crc32(u32, u32),
}

impl From<NomError<'_>> for Error {
    fn from(error: NomError<'_>) -> Self {
        Error::Nom(error.map(|e| nom::error::make_error(e.input.to_vec(), e.code)))
    }
}

pub type PacketResult<T> = Result<T, Error>;
