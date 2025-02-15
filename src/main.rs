use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all paired Bluetooth devices
    List,
    /// Connect to a Bluetooth device by name
    Connect {
        /// Name of the device to connect to
        name: String,
    },
}

#[derive(Debug)]
struct BluetoothDevice {
    address: String,
    name: String,
    connected: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => list_devices()?,
        Commands::Connect { name } => connect_to_device(&name)?,
    }

    Ok(())
}

fn get_devices() -> Result<Vec<BluetoothDevice>> {
    let output = Command::new("blueutil")
        .arg("--paired")
        .output()
        .context("Failed to execute blueutil command")?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in output_str.lines() {
        if line.contains("address:") {
            let address = line
                .split(',')
                .next()
                .and_then(|s| s.strip_prefix("address: "))
                .unwrap_or("")
                .to_string();

            let name = line
                .split("name: \"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .unwrap_or("")
                .to_string();

            let connected = line.contains("connected");

            devices.push(BluetoothDevice {
                address,
                name,
                connected,
            });
        }
    }

    Ok(devices)
}

fn list_devices() -> Result<()> {
    let devices = get_devices()?;
    println!("Paired devices:");
    
    for device in devices {
        let status = if device.connected {
            "Connected".green()
        } else {
            "Disconnected".red()
        };
        println!("  {} {} \"{}\"", device.address, status, device.name);
    }

    Ok(())
}

fn connect_to_device(search_name: &str) -> Result<()> {
    let devices = get_devices()?;
    let matcher = SkimMatcherV2::default();
    
    let mut matches: Vec<_> = devices
        .iter()
        .filter_map(|device| {
            matcher
                .fuzzy_match(&device.name.to_lowercase(), &search_name.to_lowercase())
                .map(|score| (device, score))
        })
        .collect();

    matches.sort_by_key(|(_, score)| -score);

    match matches.len() {
        0 => println!("No devices found matching '{}'", search_name),
        1 => {
            let device = matches[0].0;
            println!("Connecting to {}...", device.name);
            Command::new("blueutil")
                .args(["--connect", &device.address])
                .output()
                .context("Failed to connect to device")?;
            println!("Connected successfully!");
        }
        _ => {
            println!("Multiple devices found. Please choose one:");
            for (i, (device, _)) in matches.iter().enumerate() {
                println!("{}. {}", i + 1, device.name);
            }
            // In a real implementation, you would handle user input here
            // For now, we'll just connect to the best match
            let device = matches[0].0;
            println!("Connecting to best match: {}...", device.name);
            Command::new("blueutil")
                .args(["--connect", &device.address])
                .output()
                .context("Failed to connect to device")?;
            println!("Connected successfully!");
        }
    }

    Ok(())
} 