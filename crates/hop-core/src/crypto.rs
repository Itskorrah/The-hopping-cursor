use crate::{
    config::{Identity, NOISE},
    model::Message,
};
use anyhow::{Context, Result};
use bincode::Options;
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

pub const MAX_FRAME: usize = 65535;
pub const CHUNK_SIZE: usize = 32 * 1024;
const PROLOGUE: &[u8] = b"TheHoppingCursor/protocol/v1";
pub struct SecureConnection {
    stream: TcpStream,
    noise: snow::TransportState,
    pubkey: Vec<u8>,
}
pub struct SecureReader {
    io: OwnedReadHalf,
    noise: Arc<Mutex<snow::TransportState>>,
}
pub struct SecureWriter {
    io: OwnedWriteHalf,
    noise: Arc<Mutex<snow::TransportState>>,
}
async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> Result<Vec<u8>> {
    let n = r.read_u16().await? as usize;
    anyhow::ensure!(n > 0, "Empty frame");
    let mut b = vec![0; n];
    r.read_exact(&mut b).await?;
    Ok(b)
}
async fn write_frame<W: AsyncWrite + Unpin>(w: &mut W, b: &[u8]) -> Result<()> {
    anyhow::ensure!(!b.is_empty() && b.len() <= MAX_FRAME, "Frame too large");
    w.write_u16(b.len() as u16).await?;
    w.write_all(b).await?;
    Ok(())
}
impl SecureConnection {
    pub async fn handshake(
        mut stream: TcpStream,
        identity: &Identity,
        initiator: bool,
    ) -> Result<Self> {
        stream.set_nodelay(true)?;
        let builder = snow::Builder::new(NOISE.parse()?)
            .local_private_key(&identity.private)
            .prologue(PROLOGUE);
        let mut hs = if initiator {
            builder.build_initiator()?
        } else {
            builder.build_responder()?
        };
        let mut out = vec![0; MAX_FRAME];
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            if initiator {
                let n = hs.write_message(&[], &mut out)?;
                write_frame(&mut stream, &out[..n]).await?;
                hs.read_message(&read_frame(&mut stream).await?, &mut out)?;
                let n = hs.write_message(&[], &mut out)?;
                write_frame(&mut stream, &out[..n]).await?;
            } else {
                hs.read_message(&read_frame(&mut stream).await?, &mut out)?;
                let n = hs.write_message(&[], &mut out)?;
                write_frame(&mut stream, &out[..n]).await?;
                hs.read_message(&read_frame(&mut stream).await?, &mut out)?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await
        .context("Encrypted handshake timed out")??;
        let pubkey = hs
            .get_remote_static()
            .context("Missing peer identity")?
            .to_vec();
        Ok(Self {
            stream,
            noise: hs.into_transport_mode()?,
            pubkey,
        })
    }
    pub fn remote_key(&self) -> &[u8] {
        &self.pubkey
    }
    pub fn split(self) -> (SecureReader, SecureWriter) {
        let (r, w) = self.stream.into_split();
        let noise = Arc::new(Mutex::new(self.noise));
        (
            SecureReader {
                io: r,
                noise: noise.clone(),
            },
            SecureWriter { io: w, noise },
        )
    }
}
impl SecureReader {
    pub async fn recv(&mut self) -> Result<Message> {
        let ciphertext = read_frame(&mut self.io).await?;
        let mut plain = vec![0; MAX_FRAME];
        let n = self
            .noise
            .lock()
            .map_err(|_| anyhow::anyhow!("Cipher lock poisoned"))?
            .read_message(&ciphertext, &mut plain)?;
        Ok(bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .with_limit(MAX_FRAME as u64)
            .reject_trailing_bytes()
            .deserialize(&plain[..n])?)
    }
}
impl SecureWriter {
    pub async fn send(&mut self, message: &Message) -> Result<()> {
        let plain = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .with_limit((MAX_FRAME - 16) as u64)
            .serialize(message)?;
        let mut ciphertext = vec![0; MAX_FRAME];
        let n = self
            .noise
            .lock()
            .map_err(|_| anyhow::anyhow!("Cipher lock poisoned"))?
            .write_message(&plain, &mut ciphertext)?;
        write_frame(&mut self.io, &ciphertext[..n]).await
    }
}
