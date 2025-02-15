# BluePods

> A user-friendly CLI tool to manage Bluetooth devices on macOS.

![BluePods](media/ss2.png)


## Prerequisites

- macOS
- [blueutil](https://github.com/toy/blueutil) installed (`brew install blueutil`)
- Rust and Cargo installed

## Installation

1. Clone this repository
2. Build and install the binary:
```bash
cargo install --path .
```

This will install the `bluepods` binary in your system, making it available globally.

## Usage

### List all paired devices
```bash
bluepods list
```

This will show all paired Bluetooth devices with their connection status.

### Connect to a device
```bash
bluepods connect "AirPods Pro"
```

You can use partial names, and the search is case-insensitive:
```bash
bluepods connect airpods
```

If multiple devices match your search, you'll be shown a list to choose from.

## Features

- 🔍 Fuzzy search for device names
- 📱 Easy connection to devices by name
- 🎨 Colored output for better visibility
- ✨ Case-insensitive matching 