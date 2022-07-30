use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    ops::Deref,
    time::Duration,
};

use byteorder::{ByteOrder, NetworkEndian};
use ledger_transport::{APDUAnswer, APDUCommand, Exchange};

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
    type AnswerType = Vec<u8>;

    /// Exchange an ADPU with via the TCP transport
    async fn exchange<I>(
        &self,
        command: &APDUCommand<I>,
    ) -> Result<APDUAnswer<Self::AnswerType>, Self::Error>
    where
        I: Deref<Target = [u8]> + Send + Sync,
    {
        let mut s = self.s.lock().await;

        // Encode command object
        let out = command.serialize();

        let mut buff = vec![0u8; out.len() + 4];
        NetworkEndian::write_u32(&mut buff, out.len() as u32);
        buff[4..].copy_from_slice(&out);

        log::debug!("Sending command: {:02x?} ({})", out, out.len());

        // Send command
        s.write(&buff).await?;

        // Await response
        let mut buff = [0u8; 4];

        log::debug!("Awaiting response...");

        // Read length header
        let len = match tokio::time::timeout(self.timeout, s.read(&mut buff)).await?? {
            4 => NetworkEndian::read_u32(&buff[..4]),
            _ => return Err(Error::InvalidLength),
        };

        log::debug!("Expected {} byte response", len);

        // Read response body
        let mut buff = vec![0u8; len as usize + 2];
        tokio::time::timeout(self.timeout, s.read_exact(&mut buff)).await??;

        log::debug!("Received answer: {:02x?} ({})", buff, len);

        // Decode answer ADPU
        let answer = APDUAnswer::from_answer(buff).map_err(|_| Error::InvalidAnswer)?;

        log::debug!("Decoded APDU: {:02x?}", answer);

        // Return ADPU
        Ok(answer)
    }
}
