use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use byteorder::{ByteOrder, NetworkEndian};
use ledger_transport::{Exchange, ApduCmd, ApduBase};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

mod error;
pub use error::Error;

/// Ledger TCP (speculos) ADPU transport
pub struct TransportTcp {
    s: Mutex<TcpStream>,
    timeout: Duration,
}

/// Configuration options for [`TransportTcp`]
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct TcpOptions {
    /// TCP address for ADPU connection
    #[cfg_attr(feature = "clap", clap(long, default_value_t = TcpOptions::default().addr, env = "TCP_ADDR"))]
    pub addr: IpAddr,

    /// TCP port for ADPU connection
    #[cfg_attr(feature = "clap", clap(long, default_value_t = TcpOptions::default().port, env = "TCP_PORT"))]
    pub port: u16,

    /// ADPU timeout in milliseconds
    #[cfg_attr(feature = "clap", clap(default_value_t = TcpOptions::default().timeout_ms, env = "TCP_TIMEOUT_MS"))]
    pub timeout_ms: u64,
}

/// Default configuration for [`TransportTcp`]
impl Default for TcpOptions {
    fn default() -> Self {
        Self {
            addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 1237,
            timeout_ms: 3000,
        }
    }
}

impl TransportTcp {
    /// Create a new speculos (TCP) transport
    pub async fn new(o: TcpOptions) -> Result<Self, Error> {
        let addr = SocketAddr::new(o.addr, o.port);

        log::debug!("Using socket: {}", addr);

        // Bind TCP connection
        let s = match TcpStream::connect(addr).await {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Failed to connect to TCP socket: {}", addr);
                return Err(Error::Io(e));
            }
        };

        log::debug!("Socket bound ({:?})", s.local_addr());

        // Build object
        Ok(Self {
            s: Mutex::new(s),
            timeout: Duration::from_millis(o.timeout_ms),
        })
    }
}

#[async_trait::async_trait]
impl Exchange for TransportTcp {
    type Error = Error;

    /// Exchange an ADPU with via the TCP transport
    async fn exchange<'a, CMD: ApduCmd<'a>, ANS: ApduBase<'a>>(
        &self,
        command: CMD,
        buff: &'a mut [u8],
    ) -> Result<ANS, Self::Error> {
        let mut s = self.s.lock().await;

        // Encode command object
        let tx_len = command.encode(&mut buff[4..]);
        NetworkEndian::write_u32(&mut buff[..4], tx_len as u32);

        log::debug!("Sending command: {:02x?} ({})", &buff[..tx_len + 4], tx_len);


        // Send command
        s.write(&buff[..tx_len + 4]).await?;


        // Await response
        log::debug!("Awaiting response...");

        // Read length header
        let rx_len = match tokio::time::timeout(self.timeout, s.read(&mut buff[..4])).await?? {
            4 => NetworkEndian::read_u32(&buff[..4]) as usize,
            _ => return Err(Error::InvalidLength),
        };

        log::debug!("Expected {} byte response", rx_len);


        // Read response body
        tokio::time::timeout(self.timeout, s.read_exact(&mut buff[..rx_len])).await??;

        log::debug!("Received answer: {:02x?} ({})", buff, rx_len);

        // Decode answer ADPU
        let answer = ANS::decode(&buff[..rx_len])
            .map_err(|_| Error::InvalidAnswer)?;

        log::debug!("Decoded APDU: {:02x?}", answer);

        // Return ADPU
        Ok(answer) 
    }
}
