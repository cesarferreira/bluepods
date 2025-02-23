use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::process::Command;
use serde_json::Value;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show Bluetooth system status and devices
    Status,
    /// List all paired Bluetooth devices
    List,
    /// Connect to a Bluetooth device by name
    Connect {
        /// Name of the device to connect to
        name: String,
    },
    /// Disconnect a Bluetooth device by name
    Disconnect {
        /// Name of the device to disconnect from
        name: String,
    },
}

#[derive(Debug)]
struct BatteryInfo {
    left: Option<i32>,
    right: Option<i32>,
    single: Option<i32>,
}

#[derive(Debug)]
struct BluetoothDevice {
    address: String,
    name: String,
    connected: bool,
    battery: Option<BatteryInfo>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => show_status()?,
        Commands::List => list_devices()?,
        Commands::Connect { name } => connect_to_device(&name)?,
        Commands::Disconnect { name } => disconnect_device(&name)?,
    }

    Ok(())
}

fn get_device_battery(name: &str) -> Option<i32> {
    // Try to get battery information using ioreg
    let output = Command::new("ioreg")
        .args(["-r", "-k", "BatteryPercent", "-c", "AppleDeviceModel"])
        .output()
        .ok()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Find the device section
    for section in output_str.split("+-o") {
        if section.contains(name) {
            // Try to find battery percentage
            if let Some(battery_line) = section.lines().find(|line| line.contains("\"BatteryPercent\" = ")) {
                if let Some(percent_str) = battery_line.split('=').nth(1) {
                    if let Ok(percent) = percent_str.trim().parse::<i32>() {
                        return Some(percent);
                    }
                }
            }
        }
    }
    None
}

fn get_devices_with_battery() -> Result<Vec<BluetoothDevice>> {
    let output = Command::new("system_profiler")
        .args(["-json", "SPBluetoothDataType"])
        .output()
        .context("Failed to execute system_profiler command")?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&output_str)
        .context("Failed to parse JSON output")?;

    let mut devices = Vec::new();

    // Helper function to process device entries
    fn process_device_entry(entry: &Value, connected: bool) -> Option<BluetoothDevice> {
        let (name, details) = entry.as_object()?.iter().next()?;
        
        let address = details.get("device_address")?.as_str()?.to_string();
        
        // Get battery information
        let battery = Some(BatteryInfo {
            left: details.get("device_batteryLevelLeft")
                .and_then(|v| v.as_str())
                .and_then(|s| s.trim_end_matches('%').parse().ok()),
            right: details.get("device_batteryLevelRight")
                .and_then(|v| v.as_str())
                .and_then(|s| s.trim_end_matches('%').parse().ok()),
            single: details.get("device_batteryLevel")
                .and_then(|v| v.as_str())
                .and_then(|s| s.trim_end_matches('%').parse().ok()),
        });

        Some(BluetoothDevice {
            name: name.to_string(),
            address,
            connected,
            battery,
        })
    }

    // Process connected devices
    if let Some(bluetooth_data) = json["SPBluetoothDataType"].get(0) {
        if let Some(connected_devices) = bluetooth_data["device_connected"].as_array() {
            for device in connected_devices {
                if let Some(device_info) = process_device_entry(device, true) {
                    devices.push(device_info);
                }
            }
        }

        // Process disconnected devices
        if let Some(disconnected_devices) = bluetooth_data["device_not_connected"].as_array() {
            for device in disconnected_devices {
                if let Some(device_info) = process_device_entry(device, false) {
                    devices.push(device_info);
                }
            }
        }
    }

    Ok(devices)
}

fn get_bluetooth_power() -> Result<bool> {
    let output = Command::new("blueutil")
        .arg("--power")
        .output()
        .context("Failed to get Bluetooth power state")?;
    
    let power = String::from_utf8_lossy(&output.stdout).trim() == "1";
    Ok(power)
}

fn get_discoverable() -> Result<bool> {
    let output = Command::new("blueutil")
        .arg("--discoverable")
        .output()
        .context("Failed to get discoverable state")?;
    
    let discoverable = String::from_utf8_lossy(&output.stdout).trim() == "1";
    Ok(discoverable)
}

fn get_default_output_device() -> Result<Option<String>> {
    let output = Command::new("system_profiler")
        .args(["SPAudioDataType", "-json"])
        .output()
        .context("Failed to get audio information")?;

    // Note: This is a simplified version. In a full implementation,
    // you would want to properly parse the JSON and handle all cases
    let output_str = String::from_utf8_lossy(&output.stdout);
    if output_str.contains("\"_name\" : \"") {
        if let Some(name) = output_str
            .split("\"_name\" : \"")
            .nth(1)
            .and_then(|s| s.split('"').next())
        {
            return Ok(Some(name.to_string()));
        }
    }
    Ok(None)
}

fn get_battery_color(percentage: i32) -> colored::Color {
    match percentage {
        51..=100 => colored::Color::Green,
        // 26..=50 => colored::Color::Yellow,
        20..=50 => colored::Color::TrueColor {
            r: 255,
            g: 165,
            b: 0,
        }, // Orange
        _ => colored::Color::Red,
    }
}

fn format_battery_percentage(percentage: i32) -> ColoredString {
    format!("{}%", percentage).color(get_battery_color(percentage))
}

fn show_status() -> Result<()> {
    // Get Bluetooth power state
    let power = get_bluetooth_power()?;
    let power_status = if power { "On".green() } else { "Off".red() };
    println!("Bluetooth:        {}", power_status);

    // Get default audio output
    if let Ok(Some(output_device)) = get_default_output_device() {
        println!("Default Output:   {}", output_device);
    }

    // Get paired devices with battery info
    let devices = get_devices_with_battery()?;
    println!("\nPaired Devices:");
    for device in devices {
        let status = if device.connected {
            "connected".green()
        } else {
            "not connected".red()
        };
        
        let battery_info = match device.battery {
            Some(battery) => {
                if let (Some(left), Some(right)) = (battery.left, battery.right) {
                    format!(", battery: L:{} R:{}", 
                        format_battery_percentage(left),
                        format_battery_percentage(right))
                } else if let Some(single) = battery.single {
                    format!(", battery: {}", format_battery_percentage(single))
                } else {
                    String::new()
                }
            }
            None => String::new(),
        };

        println!("  - {:<25} ({}{})", 
            device.name,
            status,
            battery_info
        );
    }

    // Get discoverable state
    let discoverable = get_discoverable()?;
    println!("\nSystem Discoverable: {}", if discoverable { "Yes".green() } else { "No".red() });

    Ok(())
}

fn list_devices() -> Result<()> {
    let devices = get_devices_with_battery()?;
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
    let devices = get_devices_with_battery()?;
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

fn disconnect_device(search_name: &str) -> Result<()> {
    let devices = get_devices_with_battery()?;
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
            println!("Disconnecting from {}...", device.name);
            Command::new("blueutil")
                .args(["--disconnect", &device.address])
                .output()
                .context("Failed to disconnect device")?;
            println!("Disconnected successfully!");
        }
        _ => {
            println!("Multiple devices found. Please choose one:");
            for (i, (device, _)) in matches.iter().enumerate() {
                println!("{}. {}", i + 1, device.name);
            }
            // In a real implementation, you would handle user input here
            // For now, we'll just disconnect the best match
            let device = matches[0].0;
            println!("Disconnecting from best match: {}...", device.name);
            Command::new("blueutil")
                .args(["--disconnect", &device.address])
                .output()
                .context("Failed to disconnect device")?;
            println!("Disconnected successfully!");
        }
    }

    Ok(())
} 