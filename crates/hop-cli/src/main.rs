use anyhow::Result;
use clap::{Parser, Subcommand};
use hop_core::{config, platform, Service};
use std::path::PathBuf;
#[derive(Parser)]
#[command(version, about = "The Hopping Cursor: encrypted local input sharing")]
struct Args {
    #[arg(long)]
    config_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Start the native engine; Ctrl+C releases input and exits.
    Daemon {
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        connect: Option<String>,
    },
    /// Print the local public identity (never the private key).
    Identity,
    /// Trust a public key after comparing it directly on the other computer.
    Trust {
        #[arg(long)]
        public_key: String,
        #[arg(long)]
        address: String,
        #[arg(long, default_value = "Paired computer")]
        name: String,
    },
    /// Send a file to an already trusted peer; recipient must accept it.
    Send {
        #[arg(long)]
        peer: String,
        path: PathBuf,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args = Args::parse();
    let dir = args
        .config_dir
        .map(Ok)
        .unwrap_or_else(config::default_config_dir)?;
    match args.command {
        Command::Identity => {
            let c = config::load(&dir)?;
            let id = config::Identity::from_config(&c)?;
            println!(
                "Computer ID: {}\nPublic key: {}\nFingerprint: {}",
                id.id,
                hex_key(&id.public),
                config::fingerprint(&id.public)
            );
        }
        Command::Trust {
            public_key,
            address,
            name,
        } => {
            let key = decode_key(&public_key)?;
            anyhow::ensure!(key.len() == 32, "Public key must be 32 bytes");
            let mut c = config::load(&dir)?;
            c.peers.insert(
                config::device_id(&key),
                hop_core::TrustedPeer {
                    public_key,
                    name,
                    address: Some(address),
                    allow_input: true,
                    allow_clipboard: true,
                    allow_files: true,
                },
            );
            config::save(&dir, &c)?;
            println!("Trusted public key. Both computers must approve each other.");
        }
        Command::Daemon { port, connect } => {
            let service = Service::start(dir, port, platform::native()).await?;
            println!(
                "Engine listening on TCP {}. Ctrl+C to quit.",
                service.listening_port
            );
            if let Some(a) = connect {
                if let Err(e) = service.connect(&a).await {
                    eprintln!("{e}");
                }
            }
            tokio::signal::ctrl_c().await?;
            service.shutdown();
        }
        Command::Send { peer, path } => {
            let service = Service::start(dir, Some(0), platform::native()).await?;
            let result = service.send_file(&peer, path).await;
            service.shutdown();
            result?;
            println!("File delivered and verified.");
        }
    }
    Ok(())
}
fn hex_key(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}
fn decode_key(s: &str) -> Result<Vec<u8>> {
    anyhow::ensure!(s.len() == 64 && s.is_ascii(), "Invalid public key");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(Into::into))
        .collect()
}
