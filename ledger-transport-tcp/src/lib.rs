mod errors;

use ledger_apdu::{APDUAnswer, APDUCommand};

use std::net::{TcpStream};
use std::io::{Read, Write};


pub use errors::LedgerTCPError;

pub struct TransportTCP {
    url: String,
}


impl TransportTCP {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            url: format!("{}:{}", host, port),
        }
    } 

    pub async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, LedgerTCPError> {
        let raw_command = command.serialize();

        match TcpStream::connect(&self.url) {
            Ok(mut stream) => {
                log::debug!("successfully connected to server {}", &self.url);

                let mut send_length_bytes = [0u8;4];
                send_length_bytes[0] = ((raw_command.len() & 0xff000000) >> 24) as u8;
                send_length_bytes[1] = ((raw_command.len() & 0x00ff0000) >> 16) as u8;
                send_length_bytes[2] = ((raw_command.len() & 0x0000ff00) >> 8) as u8;
                send_length_bytes[3] = (raw_command.len() & 0x000000ff) as u8;
                
                // first send number of bytes
                let _ = stream.write(&send_length_bytes[..]);
                
                // then send bytes
                let _ = stream.write(&raw_command[..]);
    
                let mut rcv_length_bytes = [0u8;4];
                
                // first read number of bytes 
                let _ = stream.read_exact(&mut rcv_length_bytes);
            
                // it's in big endian
                let mut rcv_length : u32 = 
                    (rcv_length_bytes[0] as u32) << 24 | 
                    (rcv_length_bytes[1] as u32) << 16 | 
                    (rcv_length_bytes[2] as u32) << 8 | 
                    rcv_length_bytes[3] as u32;

                rcv_length += 2;    // return code

                let mut buf = vec![0u8; rcv_length as usize];

                let _ = stream.read_exact(&mut buf);
                return Ok(APDUAnswer::from_answer(buf))
            },
            Err(e) => {
                log::error!("Failed to connect: {}", e);
                return Err(LedgerTCPError::ConnectError)
            }
        }        
    }
}
