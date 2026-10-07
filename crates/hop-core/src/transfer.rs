use crate::{
    crypto::{SecureReader, SecureWriter, CHUNK_SIZE},
    model::*,
    service::{filter_clipboard, Service},
};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::oneshot,
};

const CLIPBOARD_LIMIT: u64 = 8 * 1024 * 1024;
fn safe_filename(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 240
        || name == "."
        || name == ".."
        || name.ends_with('.')
        || name.ends_with(' ')
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
    {
        return false;
    }
    let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    ![
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ]
    .contains(&base.as_str())
}
struct PartialFile {
    path: PathBuf,
    file: Option<File>,
}
impl Drop for PartialFile {
    fn drop(&mut self) {
        self.file.take();
        let _ = std::fs::remove_file(&self.path);
    }
}
impl Service {
    async fn wait_cancel(&self, id: &str) {
        loop {
            if self.cancelled.lock().unwrap().contains(id) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    pub fn respond_transfer(&self, id: &str, accepted: bool) -> Result<()> {
        let tx = self
            .decisions
            .lock()
            .unwrap()
            .remove(id)
            .context("This offer has expired or was already answered")?;
        let _ = tx.send(accepted);
        Ok(())
    }
    pub fn cancel_transfer(&self, id: &str) -> Result<()> {
        anyhow::ensure!(
            self.state.lock().unwrap().transfers.contains_key(id),
            "Unknown transfer"
        );
        self.cancelled.lock().unwrap().insert(id.into());
        if let Some(tx) = self.decisions.lock().unwrap().remove(id) {
            let _ = tx.send(false);
        }
        Ok(())
    }
    fn transfer(&self, status: TransferStatus) {
        let mut s = self.state.lock().unwrap();
        if s.transfers.len() >= 100 {
            if let Some(id) = s
                .transfers
                .iter()
                .find(|(_, t)| {
                    matches!(
                        t.state.as_str(),
                        "complete" | "failed" | "cancelled" | "declined"
                    )
                })
                .map(|(id, _)| id.clone())
            {
                s.transfers.remove(&id);
            }
        }
        s.transfers.insert(status.id.clone(), status);
    }
    fn progress(&self, id: &str, bytes: u64, state: &str) {
        if let Some(t) = self.state.lock().unwrap().transfers.get_mut(id) {
            t.bytes = bytes;
            t.state = state.into();
        }
    }
    fn finish(&self, id: &str, result: &Result<()>) {
        if let Err(e) = result {
            if let Some(t) = self.state.lock().unwrap().transfers.get_mut(id) {
                t.state = if self.cancelled.lock().unwrap().contains(id) {
                    "cancelled"
                } else {
                    "failed"
                }
                .into();
                t.error = Some(e.to_string());
            }
        }
        self.cancelled.lock().unwrap().remove(id);
        self.decisions.lock().unwrap().remove(id);
    }
    fn peer_address(&self, id: &str) -> Result<String> {
        let s = self.state.lock().unwrap();
        s.peers
            .get(id)
            .filter(|p| p.trusted)
            .map(|p| p.address.clone())
            .context("Pair and connect this computer first")
    }
    pub async fn send_file(self: &Arc<Self>, peer: &str, path: PathBuf) -> Result<()> {
        let allowed = self
            .config
            .lock()
            .unwrap()
            .peers
            .get(peer)
            .is_some_and(|p| p.allow_files);
        anyhow::ensure!(allowed, "File sharing disabled for this computer");
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Filename must be valid Unicode")?
            .to_string();
        anyhow::ensure!(
            safe_filename(&name),
            "Filename contains unsupported or unsafe characters"
        );
        let mut file = File::open(&path).await?;
        let meta = file.metadata().await?;
        anyhow::ensure!(meta.is_file(), "Select a regular file; zip folders first");
        let size = meta.len();
        anyhow::ensure!(
            size <= self.config.lock().unwrap().settings.max_file_bytes,
            "File exceeds configured transfer limit"
        );
        let id = uuid::Uuid::new_v4().to_string();
        self.transfer(TransferStatus {
            id: id.clone(),
            peer_id: peer.into(),
            name: name.clone(),
            size,
            bytes: 0,
            direction: "outgoing".into(),
            state: "hashing".into(),
            error: None,
            saved_path: None,
        });
        let result:Result<()>=async{
            let mut hash=Sha256::new();let mut buffer=vec![0;CHUNK_SIZE];let mut stop=self.stop.subscribe();loop{anyhow::ensure!(!self.cancelled.lock().unwrap().contains(&id)&&!*stop.borrow(),"Transfer cancelled");let n=file.read(&mut buffer).await?;if n==0{break;}hash.update(&buffer[..n]);}
            file.rewind().await?;let hash=hex::encode(hash.finalize());
            let address=self.peer_address(peer)?;let(actual,mut reader,mut writer)=self.open(&address,"data").await?;anyhow::ensure!(actual==peer,"The address belongs to a different computer");
            writer.send(&Message::BlobStart{id:id.clone(),kind:BlobKind::File,name,size,hash}).await?;self.progress(&id,0,"awaiting_acceptance");
            let response=tokio::select!{_=stop.changed()=>anyhow::bail!("Application stopped"),_=self.wait_cancel(&id)=>anyhow::bail!("Transfer cancelled"),r=tokio::time::timeout(Duration::from_secs(65),reader.recv())=>r.context("File offer expired")??};
            anyhow::ensure!(matches!(response,Message::BlobAccept{id:ref got,accepted:true}if got==&id),"Recipient declined or file sharing is disabled");
            let mut offset=0;loop{anyhow::ensure!(!self.cancelled.lock().unwrap().contains(&id)&&!*stop.borrow(),"Transfer cancelled");let n=file.read(&mut buffer).await?;if n==0{break;}anyhow::ensure!(offset+n as u64<=size,"Source file changed during transfer");
                tokio::time::timeout(Duration::from_secs(20),writer.send(&Message::BlobChunk{id:id.clone(),offset,data:buffer[..n].to_vec()})).await.context("Transfer stalled")??;offset+=n as u64;self.progress(&id,offset,"transferring");}
            anyhow::ensure!(offset==size,"Source file changed during transfer");writer.send(&Message::BlobEnd{id:id.clone()}).await?;
            let result=tokio::time::timeout(Duration::from_secs(20),reader.recv()).await.context("No transfer confirmation")??;
            match result{Message::BlobResult{id:got,ok:true,..}if got==id=>{},Message::BlobResult{error,..}=>anyhow::bail!(error.unwrap_or_else(||"Receiver rejected transfer".into())),_=>anyhow::bail!("Invalid transfer confirmation")};self.progress(&id,size,"complete");Ok(())
        }.await;
        self.finish(&id, &result);
        result
    }
    pub(crate) async fn send_clipboard(self: &Arc<Self>, peer: &str, data: Vec<u8>) -> Result<()> {
        anyhow::ensure!(
            data.len() as u64 <= CLIPBOARD_LIMIT,
            "Clipboard exceeds 8 MiB limit"
        );
        let address = self.peer_address(peer)?;
        let (actual, mut reader, mut writer) = self.open(&address, "data").await?;
        anyhow::ensure!(actual == peer, "Peer address changed identity");
        let id = uuid::Uuid::new_v4().to_string();
        writer
            .send(&Message::BlobStart {
                id: id.clone(),
                kind: BlobKind::Clipboard,
                name: "clipboard".into(),
                size: data.len() as u64,
                hash: hex::encode(Sha256::digest(&data)),
            })
            .await?;
        let accept = tokio::time::timeout(Duration::from_secs(5), reader.recv()).await??;
        anyhow::ensure!(
            matches!(accept,Message::BlobAccept{id:ref got,accepted:true}if got==&id),
            "Clipboard disabled by peer"
        );
        let mut stop = self.stop.subscribe();
        for (offset, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
            anyhow::ensure!(!*stop.borrow(), "Application stopped");
            let message = Message::BlobChunk {
                id: id.clone(),
                offset: (offset * CHUNK_SIZE) as u64,
                data: chunk.to_vec(),
            };
            tokio::select! {_=stop.changed()=>anyhow::bail!("Application stopped"),r=tokio::time::timeout(Duration::from_secs(10),writer.send(&message))=>r??};
        }
        writer.send(&Message::BlobEnd { id: id.clone() }).await?;
        let result = tokio::time::timeout(Duration::from_secs(10), reader.recv()).await??;
        anyhow::ensure!(
            matches!(result,Message::BlobResult{id:ref got,ok:true,..}if got==&id),
            "Clipboard application failed"
        );
        Ok(())
    }
    pub(crate) async fn receive_blob(
        self: &Arc<Self>,
        peer: String,
        mut reader: SecureReader,
        mut writer: SecureWriter,
    ) -> Result<()> {
        let start = tokio::time::timeout(Duration::from_secs(10), reader.recv())
            .await
            .context("Data offer timed out")??;
        let Message::BlobStart {
            id,
            kind,
            name,
            size,
            hash,
        } = start
        else {
            anyhow::bail!("Expected data offer")
        };
        anyhow::ensure!(
            uuid::Uuid::parse_str(&id).is_ok() && hash.len() == 64 && hex::decode(&hash).is_ok(),
            "Invalid offer identity/hash"
        );
        let settings = self.config.lock().unwrap().settings.clone();
        let allowed = {
            let c = self.config.lock().unwrap();
            c.peers.get(&peer).is_some_and(|p| match kind {
                BlobKind::File => p.allow_files,
                BlobKind::Clipboard => p.allow_clipboard,
            })
        };
        let allowed = allowed
            && !settings.paused
            && match kind {
                BlobKind::File => safe_filename(&name) && size <= settings.max_file_bytes,
                BlobKind::Clipboard => {
                    size <= CLIPBOARD_LIMIT
                        && (settings.clipboard_text
                            || settings.clipboard_html
                            || settings.clipboard_images)
                }
            };
        if !allowed {
            writer
                .send(&Message::BlobAccept {
                    id,
                    accepted: false,
                })
                .await?;
            return Ok(());
        }
        let is_file = matches!(kind, BlobKind::File);
        let mut stop = self.stop.subscribe();
        if is_file {
            self.transfer(TransferStatus {
                id: id.clone(),
                peer_id: peer.clone(),
                name: name.clone(),
                size,
                bytes: 0,
                direction: "incoming".into(),
                state: "offered".into(),
                error: None,
                saved_path: None,
            });
            let accepted = if settings.auto_accept_files {
                true
            } else {
                let (tx, rx) = oneshot::channel();
                self.decisions.lock().unwrap().insert(id.clone(), tx);
                tokio::select! {_=stop.changed()=>false,r=tokio::time::timeout(Duration::from_secs(60),rx)=>matches!(r,Ok(Ok(true)))}
            };
            self.decisions.lock().unwrap().remove(&id);
            if !accepted {
                self.progress(&id, 0, "declined");
                writer
                    .send(&Message::BlobAccept {
                        id,
                        accepted: false,
                    })
                    .await?;
                return Ok(());
            }
        }
        let result:Result<()>=async {
            let mut partial=if is_file {
                tokio::fs::create_dir_all(&settings.receive_directory).await?;let dir=tokio::fs::canonicalize(&settings.receive_directory).await?;
                let path=dir.join(format!(".hop-{}.part",uuid::Uuid::new_v4()));let mut options=tokio::fs::OpenOptions::new();options.create_new(true).write(true);
                #[cfg(unix)] {options.mode(0o600);}
                let file=options.open(&path).await?;Some(PartialFile{path,file:Some(file)})
            }else{None};
            writer.send(&Message::BlobAccept{id:id.clone(),accepted:true}).await?;
            let mut memory=vec![];let mut offset=0u64;let mut digest=Sha256::new();loop {
                anyhow::ensure!(!self.cancelled.lock().unwrap().contains(&id),"Transfer cancelled");
                let still_allowed={let c=self.config.lock().unwrap();!c.settings.paused&&c.peers.get(&peer).is_some_and(|p|if is_file{p.allow_files}else{p.allow_clipboard})};anyhow::ensure!(still_allowed,"Sharing permission revoked");
                let message=tokio::select!{_=stop.changed()=>anyhow::bail!("Application stopped"),_=self.wait_cancel(&id)=>anyhow::bail!("Transfer cancelled"),r=tokio::time::timeout(Duration::from_secs(20),reader.recv())=>r.context("Transfer stalled")??};
                match message {
                    Message::BlobChunk{id:got,offset:at,data}=>{
                        anyhow::ensure!(got==id&&at==offset&&!data.is_empty()&&data.len()<=CHUNK_SIZE&&offset+data.len() as u64<=size,"Invalid chunk sequence or size");digest.update(&data);
                        if let Some(p)=&mut partial{p.file.as_mut().unwrap().write_all(&data).await?;}else{memory.extend_from_slice(&data);}
                        offset+=data.len() as u64;if is_file{self.progress(&id,offset,"transferring");}
                    },Message::BlobEnd{id:got}=>{anyhow::ensure!(got==id&&offset==size,"Incomplete transfer");break;},_=>anyhow::bail!("Invalid bulk-data message"),
                }
            }
            anyhow::ensure!(hex::encode(digest.finalize())==hash,"Integrity verification failed; file was not saved");
            if let Some(mut p)=partial {
                let file=p.file.take().unwrap();file.sync_all().await?;drop(file);let dir=p.path.parent().unwrap();let mut destination=None;
                for n in 0..10000 {let candidate=dir.join(if n==0{name.clone()}else{let path=Path::new(&name);let stem=path.file_stem().and_then(|v|v.to_str()).unwrap_or("file");let ext=path.extension().and_then(|v|v.to_str());match ext{Some(ext)=>format!("{stem} ({n}).{ext}"),None=>format!("{stem} ({n})")}});
                    match tokio::fs::hard_link(&p.path,&candidate).await{Ok(())=>{destination=Some(candidate);break;},Err(e)if e.kind()==std::io::ErrorKind::AlreadyExists=>continue,Err(e)=>return Err(e.into())}
                }
                let destination=destination.context("Too many files with the same name")?;if let Some(t)=self.state.lock().unwrap().transfers.get_mut(&id){t.saved_path=Some(destination.to_string_lossy().into());}self.progress(&id,size,"complete");
            }else {
                let parts:Vec<ClipboardPart>=serde_json::from_slice(&memory)?;anyhow::ensure!(parts.len()<=3&&parts.iter().all(|p|matches!(p.mime.as_str(),"text/plain"|"text/html"|"image/png")),"Invalid clipboard types");
                let current=self.config.lock().unwrap().settings.clone();let filtered=filter_clipboard(parts,&current);let backend=self.backend.clone();
                let applied=tokio::task::spawn_blocking(move||{backend.write_clipboard(&filtered)?;backend.read_clipboard()}).await??;
                let data=serde_json::to_vec(&filter_clipboard(applied,&current))?;self.state.lock().unwrap().clipboard_hash=Sha256::digest(&data).to_vec();
            }
            Ok(())
        }.await;
        let confirmation = Message::BlobResult {
            id: id.clone(),
            ok: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
        };
        let _ = writer.send(&confirmation).await;
        if is_file {
            self.finish(&id, &result);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_names() {
        for bad in [
            "../secret",
            "C:\\secret",
            "CON",
            "nul.txt",
            "a/b",
            "a\0b",
            "..",
            "bad.",
            "bad ",
            "a:b",
        ] {
            assert!(!safe_filename(bad), "{bad}");
        }
        for good in ["notes.md", "hello world.txt", "résumé.pdf", "项目.zip"] {
            assert!(safe_filename(good));
        }
    }
}
