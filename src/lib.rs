#[cfg(test)]
mod hidapi_checks {
    #[test]
    fn retrieve_devices() {
        extern crate hidapi;

        let api = hidapi::HidApi::new().unwrap();
        for device in &api.devices() {
            println!("{:#?}", device);
        }
    }
}
