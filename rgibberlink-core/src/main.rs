#[cfg(feature = "python")]
use clap::{Parser, Subcommand};
#[cfg(feature = "python")]
use crate::crypto::CryptoEngine;
#[cfg(feature = "python")]
use crate::visual::{VisualEngine, VisualPayload};
#[cfg(feature = "python")]
use std::fs;

#[cfg(feature = "python")]
/// RealGibber - Secure directional communication protocol CLI
#[derive(Parser)]
#[command(name = "rgibberlink")]
#[command(about = "CLI tool for RealGibber secure communication protocol")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "python")]
#[derive(Subcommand)]
enum Commands {
    /// Generate QR code for handshake initiation
    Handshake {
        /// Payload data to encode in the QR code
        #[arg(short, long)]
        payload: String,

        /// Output file path (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Output format (svg or png, defaults to svg)
        #[arg(short, long, default_value = "svg")]
        format: String,
    },
    /// Generate cryptographic keys
    Keygen {
        /// Output file for private key
        #[arg(short, long)]
        private_key: Option<String>,

        /// Output file for public key
        #[arg(short, long)]
        public_key: Option<String>,
    },
    /// Encrypt data
    Encrypt {
        /// Data to encrypt
        #[arg(short, long)]
        data: String,

        /// Key file path
        #[arg(short, long)]
        key_file: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Decrypt data
    Decrypt {
        /// Encrypted data file
        #[arg(short, long)]
        input: String,

        /// Key file path
        #[arg(short, long)]
        key_file: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[cfg(all(feature = "async", feature = "python"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Handshake { payload, output, format } => {
            handle_handshake(payload, output, format).await?;
        }
        Commands::Keygen { private_key, public_key } => {
            handle_keygen(private_key, public_key).await?;
        }
        Commands::Encrypt { data, key_file, output } => {
            handle_encrypt(data, key_file, output).await?;
        }
        Commands::Decrypt { input, key_file, output } => {
            handle_decrypt(input, key_file, output).await?;
        }
    }

    Ok(())
}

#[cfg(not(all(feature = "async", feature = "python")))]
fn main() {
    println!("CLI not available in this build configuration");
}

#[cfg(all(feature = "async", feature = "python"))]
async fn handle_handshake(payload: String, output: Option<String>, format: String) -> Result<(), Box<dyn std::error::Error>> {
    // Create crypto engine for key generation
    let crypto = CryptoEngine::new();
    let session_id = CryptoEngine::generate_nonce();
    let nonce = CryptoEngine::generate_nonce();

    // Create a dummy signature for demo purposes
    let dummy_signature = vec![0u8; 64];

    // Create visual payload
    let visual_payload = VisualPayload {
        session_id,
        public_key: crypto.public_key().to_vec(),
        nonce,
        signature: dummy_signature,
    };

    // Create visual engine and encode
    let visual_engine = VisualEngine::new();
    let qr_svg = visual_engine.encode_payload(&visual_payload)?;

    // Handle output
    match output {
        Some(path) => {
            if format == "svg" {
                fs::write(&path, qr_svg)?;
                println!("QR code saved to {}", path);
            } else {
                return Err("Only SVG format is currently supported".into());
            }
        }
        None => {
            println!("{}", qr_svg);
        }
    }

    Ok(())
}

#[cfg(all(feature = "async", feature = "python"))]
async fn handle_keygen(private_key_path: Option<String>, public_key_path: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let crypto = CryptoEngine::new();

    if let Some(path) = public_key_path {
        fs::write(path, crypto.public_key())?;
        println!("Public key saved");
    } else {
        println!("Public key: {}", hex::encode(crypto.public_key()));
    }

    // Note: Private key handling would need secure key storage implementation
    if let Some(path) = private_key_path {
        println!("Warning: Private key storage not implemented yet. Key not saved to {}", path);
    }

    Ok(())
}

#[cfg(all(feature = "async", feature = "python"))]
async fn handle_encrypt(data: String, key_file: String, output: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let key_data = fs::read(&key_file)?;
    if key_data.len() != 32 {
        return Err("Key file must contain exactly 32 bytes".into());
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_data);

    let encrypted = CryptoEngine::encrypt_data(&key, data.as_bytes())?;

    match output {
        Some(path) => {
            fs::write(path, &encrypted)?;
            println!("Encrypted data saved");
        }
        None => {
            println!("{}", hex::encode(&encrypted));
        }
    }

    Ok(())
}

#[cfg(all(feature = "async", feature = "python"))]
async fn handle_decrypt(input: String, key_file: String, output: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let key_data = fs::read(&key_file)?;
    if key_data.len() != 32 {
        return Err("Key file must contain exactly 32 bytes".into());
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_data);

    let encrypted_data = if std::path::Path::new(&input).exists() {
        fs::read(&input)?
    } else {
        hex::decode(&input)?
    };

    let decrypted = CryptoEngine::decrypt_data(&key, &encrypted_data)?;

    match output {
        Some(path) => {
            fs::write(path, &decrypted)?;
            println!("Decrypted data saved");
        }
        None => {
            println!("{}", String::from_utf8(decrypted)?);
        }
    }

    Ok(())
}