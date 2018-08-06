extern crate hidapi;

use std::ffi::CString;
use hidapi::HidDevice;

const LEDGER_VID: u16 = 0x2c97;
const LEDGER_USAGE_PAGE: u16 = 0x0032;
const LEDGER_CHANNEL: u16 = 0x0101;
const LEDGER_PACKET_SIZE: u8 = 64;

// TODO: Create a struct for a ledger device so it can be opened/closed, etc.
// link exchange to the device

fn find_ledger_device_path() -> Result<CString, &'static str>
{
    extern crate hidapi;

    let api = hidapi::HidApi::new().expect("Could not open HID API");
    for device in api.devices() {
        if device.vendor_id == LEDGER_VID && device.usage_page == LEDGER_USAGE_PAGE
            {
                return Ok(device.path.clone());
            }
    }
    Err("Could not find Ledger Nano S device")
}

fn write_packet(channel: u16, command: &[u8], packet_size: u8, sequence_idx: u16) -> Result<i32, &'static str>
{
    // TODO: Move packet size to the device and set as an attribute

    if packet_size < 3 {
        return Err("Packet size must be at least 3");
    }

    let mut buffer = vec![0u8; packet_size as usize];

    buffer[0] = ((channel >> 8) & 0xFF) as u8;         // channel big endian
    buffer[1] = ((channel >> 0) & 0xFF) as u8;         // channel big endian
    buffer[2] = 0x05u8;

    let command_length = command.len() as u16;

    // TODO: First packet should include the command length, other packets just raw command chunks
    unimplemented!()
}

fn write_apdu(device: &HidDevice, channel: u16, apdu_command: &[u8], packet_size: u8) -> Result<i32, &'static str>
{
    // TODO: Prepare command
    // TODO: call write_packet until all packets have been sent

    device.write(apdu_command);
    println!("{:?}", apdu_command);

    Ok(0)
}

fn read_packet()
{
    // TODO: read from device
    // TODO: extract the packet
    unimplemented!()
}

fn read_apdu()
{
    // TODO: Receive all packets
    // TODO: Extract APDU and return
    unimplemented!()
}

fn exchange(cla: u8, ins: u8, p1: u8, p2: u8) -> Result<i32, &'static str>
{
    extern crate hidapi;
    use std::borrow::Borrow;

    // Get device path
    let device_path = find_ledger_device_path().expect("Could not find a ledger device");
    println!("Device Path :\t{:?}", device_path);

    // Open device
    let api = hidapi::HidApi::new().expect("Could not open HID API");
    let device = api.open_path(&device_path).expect("Failed to open device");

    println!("Manufacturer:\t{:?}", device.get_manufacturer_string().expect("Failed to read serial number"));
    println!("Product     :\t{:?}", device.get_product_string().expect("Failed to read serial number"));
    println!("Serial      :\t{:?}", device.get_serial_number_string().expect("Failed to read serial number"));

    // TODO: append data, adjust length byte, etc.
    let command = [cla, ins, p1, p2, 0x00u8];

    write_apdu(&device,
               LEDGER_CHANNEL,
               &command,
               LEDGER_PACKET_SIZE).expect("Failed to write APDU");

    let mut answer = [0u8; 256];
    let res = device.read(&mut answer[..]).unwrap();

    let mut data_string = String::new();
    for u in &answer[..res] {
        data_string.push_str(&(u.to_string() + "\t"));
    }

    Ok(1)
}

#[cfg(test)]
mod hidapi_checks {
    #[test]
    fn test_ledger_device_path() {
        use find_ledger_device_path;

        let result = find_ledger_device_path();

        match result
            {
                Ok(x) => println!("{:?}", x),
                Err(x) => {
                    println!("{:?}", x);
                    assert!(false);
                }
            }
    }

    #[test]
    fn test_exchange() {
        use exchange;

        let result = exchange(0x55, 0x00, 0x00, 0x00);

        match result
            {
                Ok(x) => println!("{:?}", x),
                Err(x) => println!("{:?}", x)
            }
    }

    #[test]
    fn list_all_devices() {
        extern crate hidapi;

        let api = hidapi::HidApi::new().expect("Could not open HID API");
        for device in api.devices() {
            println!("{:#?}", device);
        }
    }
}
