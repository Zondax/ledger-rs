use thiserror::Error;

#[derive(Error, Debug)]
pub enum LedgerTCPError {
    /// Device not found error
    #[error("Ledger connect error")]
    ConnectError,
    /// zemu reponse error
    #[error("TCP response error")]
    ResponseError,
    /// Inner error
    #[error("Ledger inner error")]
    InnerError,
}
