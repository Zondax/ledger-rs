use thiserror::Error;

#[derive(Error, Debug)]
pub enum LedgerZemuError {
    /// Device not found error
    #[error("Ledger connect error")]
    ConnectError,
    /// zemu reponse error
    #[error("Zemu response error")]
    ResponseError,
    /// Inner error
    #[error("Ledger inner error")]
    InnerError,
}
