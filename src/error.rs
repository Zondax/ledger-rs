use failure::{Compat, Error};

#[derive(Debug, Fail)]
pub enum LedgerError {
    #[fail(display = "ledger error: {}", message)]
    HidApiError { message: String },
}
