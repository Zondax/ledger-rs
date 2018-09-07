#[macro_use]
extern crate quick_error;

quick_error! {
    #[derive(Debug)]
    enum LedgerError {
        DeviceNotFound{
            description("Could not find a ledger device")
        }
        Comm(additional_description: String) {
            description("Communication Error: {}", additional_description)
        }
        APDU(additional_description: String) {
            description("APDU Error: {}", additional_description)
        }
        Unknown(additional_description: String) {
            description("Unknown Error: {}", additional_description)
        }
    }
}
