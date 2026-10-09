use crate::{
    config::{Identity, NOISE},
    model::Message,
};
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};
use zeroize::Zeroizing;

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
    ciphertext: Vec<u8>,
    plain: Zeroizing<Vec<u8>>,
}
pub struct SecureWriter {
    io: OwnedWriteHalf,
    noise: Arc<Mutex<snow::TransportState>>,
    plain: Zeroizing<Vec<u8>>,
    ciphertext: Vec<u8>,
}
async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    read_frame_into(r, &mut b).await?;
    Ok(b)
}
async fn read_frame_into<R: AsyncRead + Unpin>(r: &mut R, b: &mut Vec<u8>) -> Result<()> {
    let n = r.read_u16().await? as usize;
    anyhow::ensure!(n > 0, "Empty frame");
    b.resize(n, 0);
    r.read_exact(b).await?;
    Ok(())
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
                ciphertext: Vec::with_capacity(MAX_FRAME),
                plain: Zeroizing::new(vec![0; MAX_FRAME]),
            },
            SecureWriter {
                io: w,
                noise,
                plain: Zeroizing::new(vec![0; MAX_FRAME - 16]),
                ciphertext: vec![0; MAX_FRAME],
            },
        )
    }
}
impl SecureReader {
    pub async fn recv(&mut self) -> Result<Message> {
        read_frame_into(&mut self.io, &mut self.ciphertext).await?;
        let n = self
            .noise
            .lock()
            .map_err(|_| anyhow::anyhow!("Cipher lock poisoned"))?
            .read_message(&self.ciphertext, &mut self.plain)?;
        decode_plain(&self.plain[..n])
    }
}
impl SecureWriter {
    pub async fn send(&mut self, message: &Message) -> Result<()> {
        let plain = postcard::to_slice(message, &mut self.plain)?;
        let n = self
            .noise
            .lock()
            .map_err(|_| anyhow::anyhow!("Cipher lock poisoned"))?
            .write_message(plain, &mut self.ciphertext)?;
        write_frame(&mut self.io, &self.ciphertext[..n]).await
    }
}

fn decode_plain(plain: &[u8]) -> Result<Message> {
    let (message, remaining) = postcard::take_from_bytes(plain)?;
    anyhow::ensure!(remaining.is_empty(), "Trailing data in control frame");
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use tokio::net::TcpListener;
    #[test]
    fn rejects_malformed_and_trailing_plaintext() {
        assert!(decode_plain(&[]).is_err());
        assert!(decode_plain(&[255; 20]).is_err());
        // Synthetic CI fuzz input: a truncated display list must fail cleanly.
        assert!(decode_plain(include_bytes!(
            "../tests/fixtures/malformed-display-count.bin"
        ))
        .is_err());
        let mut bytes = postcard::to_allocvec(&Message::Leave).unwrap();
        assert!(matches!(decode_plain(&bytes).unwrap(), Message::Leave));
        bytes.push(0);
        assert!(decode_plain(&bytes).is_err());
    }
    async fn malicious_proxy(replay: bool) {
        let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr().unwrap();
        let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy.local_addr().unwrap();
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let initiator = Identity::from_config(&config::load(a.path()).unwrap()).unwrap();
        let responder = Identity::from_config(&config::load(b.path()).unwrap()).unwrap();
        let receiver = tokio::spawn(async move {
            let (stream, _) = server.accept().await.unwrap();
            let connection = SecureConnection::handshake(stream, &responder, false)
                .await
                .unwrap();
            let (mut reader, _) = connection.split();
            if replay {
                assert!(reader.recv().await.is_ok());
            }
            assert!(
                reader.recv().await.is_err(),
                "Tampered/replayed ciphertext must fail authentication"
            );
        });
        let intermediary = tokio::spawn(async move {
            let (mut client, _) = proxy.accept().await.unwrap();
            let mut target = TcpStream::connect(server_addr).await.unwrap();
            let one = read_frame(&mut client).await.unwrap();
            write_frame(&mut target, &one).await.unwrap();
            let two = read_frame(&mut target).await.unwrap();
            write_frame(&mut client, &two).await.unwrap();
            let three = read_frame(&mut client).await.unwrap();
            write_frame(&mut target, &three).await.unwrap();
            let mut encrypted = read_frame(&mut client).await.unwrap();
            assert!(!encrypted
                .windows(b"secret-file-name.txt".len())
                .any(|w| w == b"secret-file-name.txt"));
            if replay {
                write_frame(&mut target, &encrypted).await.unwrap();
            } else {
                let n = encrypted.len() - 1;
                encrypted[n] ^= 0x80;
            }
            write_frame(&mut target, &encrypted).await.unwrap();
        });
        let conn = SecureConnection::handshake(
            TcpStream::connect(proxy_addr).await.unwrap(),
            &initiator,
            true,
        )
        .await
        .unwrap();
        let (_, mut writer) = conn.split();
        writer
            .send(&Message::BlobStart {
                id: "test".into(),
                kind: crate::model::BlobKind::File,
                name: "secret-file-name.txt".into(),
                size: 0,
                hash: "test".into(),
            })
            .await
            .unwrap();
        intermediary.await.unwrap();
        receiver.await.unwrap();
    }
    #[tokio::test]
    async fn rejects_tampered_ciphertext() {
        malicious_proxy(false).await;
    }
    #[tokio::test]
    async fn rejects_replayed_ciphertext() {
        malicious_proxy(true).await;
    }
    #[tokio::test]
    async fn rejects_empty_handshake_frames() {
        let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let id = Identity::from_config(&config::load(dir.path()).unwrap()).unwrap();
        let task = tokio::spawn(async move {
            let (stream, _) = server.accept().await.unwrap();
            assert!(SecureConnection::handshake(stream, &id, false)
                .await
                .is_err());
        });
        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_u16(0).await.unwrap();
        task.await.unwrap();
    }
}

/// Exercise only bounded decrypted input; this feature is absent from release builds.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub fn fuzz_plaintext(bytes: &[u8]) {
    if bytes.len() > MAX_FRAME - 16 {
        return;
    }
    if let Ok(message) = decode_plain(bytes) {
        if let Message::Input(event) = &message {
            let _ = event.validate();
        }
        if let Message::Hello { device, .. } = &message {
            for display in &device.displays {
                let _ = display.bounds.valid();
            }
        }
        let encoded = postcard::to_allocvec(&message).expect("Decoded message must serialize");
        assert!(encoded.len() <= bytes.len());
        assert!(decode_plain(&encoded).is_ok());
    }
}
