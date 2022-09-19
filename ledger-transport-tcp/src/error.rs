/// Speculos (TCP) transport errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error {:?}", 0)]
    Io(std::io::Error),

    #[error("Command timeout")]
    Timeout,

    #[error("Invalid response length")]
    InvalidLength,

    #[error("Invalid answer APDU")]
    InvalidAnswer,

    #[error("APDU Error")]
    ApduError,
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Error::Timeout
    }
}

impl From<ledger_apdu::ApduError> for Error {
    fn from(_: ledger_apdu::ApduError) -> Self {
        Self::ApduError
    }
}