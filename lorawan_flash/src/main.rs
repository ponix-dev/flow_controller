use anyhow::{bail, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use clap::{Parser, Subcommand};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

/// Custom GATT service UUID (must match firmware)
const PROVISION_SERVICE_UUID: Uuid = Uuid::from_u128(0x12345678_1234_5678_1234_56789abcdef0);
/// LoRaWAN Keys characteristic UUID (must match firmware)
const LORAWAN_KEYS_CHAR_UUID: Uuid = Uuid::from_u128(0x12345678_1234_5678_1234_56789abcdef2);

#[derive(Parser)]
#[command(name = "lorawan_flash")]
#[command(about = "BLE central CLI for provisioning LoRaWAN keys (RAK4631)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for nearby BLE devices
    Scan {
        /// Scan duration in seconds
        #[arg(short, long, default_value_t = 5)]
        duration: u64,
    },
    /// Provision LoRaWAN keys via BLE (DevEUI + AppEUI + AppKey)
    Provision {
        /// Device EUI (16 hex chars, e.g. "024B00D87ED5B370")
        #[arg(long, env = "DEVEUI")]
        deveui: String,
        /// Application EUI (16 hex chars)
        #[arg(long, env = "APPEUI")]
        appeui: String,
        /// Application Key (32 hex chars)
        #[arg(long, env = "APPKEY")]
        appkey: String,
        /// BLE device name to connect to (or set END_DEVICE env var)
        #[arg(short, long, env = "END_DEVICE")]
        name: String,
    },
}

async fn get_adapter() -> Result<Adapter> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    adapters
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No BLE adapters found"))
}

async fn find_device(adapter: &Adapter, name: &str) -> Result<Peripheral> {
    println!("Scanning for '{}'...", name);
    adapter.start_scan(ScanFilter::default()).await?;
    time::sleep(Duration::from_secs(5)).await;
    adapter.stop_scan().await?;

    let peripherals = adapter.peripherals().await?;
    for p in &peripherals {
        if let Some(props) = p.properties().await? {
            if props.local_name.as_deref() == Some(name) {
                return Ok(p.clone());
            }
        }
    }
    bail!("Device '{}' not found. Is it advertising?", name)
}

async fn scan(duration: u64) -> Result<()> {
    let adapter = get_adapter().await?;
    println!("Scanning for {} seconds...", duration);
    adapter.start_scan(ScanFilter::default()).await?;
    time::sleep(Duration::from_secs(duration)).await;
    adapter.stop_scan().await?;

    let peripherals = adapter.peripherals().await?;
    if peripherals.is_empty() {
        println!("No devices found.");
        return Ok(());
    }

    for p in &peripherals {
        let props = p.properties().await?;
        let name = props
            .and_then(|p| p.local_name)
            .unwrap_or_else(|| "Unknown".into());
        println!("  {} ({})", name, p.id());
    }
    println!("Found {} device(s)", peripherals.len());
    Ok(())
}

async fn connect_and_discover(name: &str) -> Result<Peripheral> {
    let adapter = get_adapter().await?;
    let device = find_device(&adapter, name).await?;

    println!("Connecting to '{}'...", name);
    device.connect().await?;
    println!("Connected.");

    device.discover_services().await?;

    let has_service = device
        .services()
        .iter()
        .any(|s| s.uuid == PROVISION_SERVICE_UUID);
    if !has_service {
        device.disconnect().await?;
        bail!(
            "Device '{}' does not expose the provisioning service ({})",
            name,
            PROVISION_SERVICE_UUID
        );
    }

    Ok(device)
}

async fn write_lorawan_keys(
    device: &Peripheral,
    deveui: &str,
    appeui: &str,
    appkey: &str,
) -> Result<()> {
    let deveui_bytes = hex::decode(deveui)?;
    let appeui_bytes = hex::decode(appeui)?;
    let appkey_bytes = hex::decode(appkey)?;

    if deveui_bytes.len() != 8 {
        bail!(
            "DevEUI must be 8 bytes (16 hex chars), got {}",
            deveui_bytes.len()
        );
    }
    if appeui_bytes.len() != 8 {
        bail!(
            "AppEUI must be 8 bytes (16 hex chars), got {}",
            appeui_bytes.len()
        );
    }
    if appkey_bytes.len() != 16 {
        bail!(
            "AppKey must be 16 bytes (32 hex chars), got {}",
            appkey_bytes.len()
        );
    }

    let mut payload = [0u8; 32];
    payload[0..8].copy_from_slice(&deveui_bytes);
    payload[8..16].copy_from_slice(&appeui_bytes);
    payload[16..32].copy_from_slice(&appkey_bytes);

    let chars = device.characteristics();
    let keys_char = chars
        .iter()
        .find(|c| c.uuid == LORAWAN_KEYS_CHAR_UUID)
        .ok_or_else(|| anyhow::anyhow!("LoRaWAN Keys characteristic not found"))?;

    device
        .write(keys_char, &payload, WriteType::WithoutResponse)
        .await?;
    println!("LoRaWAN keys provisioned successfully");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan { duration } => scan(duration).await,
        Commands::Provision {
            deveui,
            appeui,
            appkey,
            name,
        } => {
            let device = connect_and_discover(&name).await?;
            write_lorawan_keys(&device, &deveui, &appeui, &appkey).await?;
            device.disconnect().await?;
            println!("Disconnected.");
            Ok(())
        }
    }
}
