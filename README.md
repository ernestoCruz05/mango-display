# MDisplay

A GUI monitor layout manager for [mangowc](https://mangowc.vercel.app/). Designed to be similar to tools like `nwg-displays` or `wdisplays`.

## Features

* **Visual Canvas**: Drag and drop your screen layouts efficiently with magnetic edge snapping.
* **Hardware Configurations**: Manipulate DPI Scaling, Refresh Rates, Resolutions, and Orientation transforms.
* **Live Previews**: Temporarily apply your changes to experiment with `wlr-randr` configurations.
* **Restore Default**: Safely revert to your base configuration. MDisplay takes a frozen snapshot of your pre-existing monitor rules the very first time it runs, allowing you to easily undo all layout changes without affecting your other `mangowc` settings.
* **Persistent Saving**: Save the finalized `monitorrule` lines directly to `~/.config/mango/monitors.conf`, automatically appended to your `config.conf`.

## Requirements

MDisplay relies on `wlr-randr` to query the currently active outputs and temporarily apply modifications. 

Before running, ensure you have:
* The `mangowc` Wayland compositor installed.
* `wlr-randr` installed on your system.
* Rust toolchain (Cargo, rustc) to compile the application.

## Installation

Clone the repository and compile using Cargo:

```bash
git clone https://github.com/ernestoCruz05/mdisplay.git
cd mdisplay
cargo build --release
```

The compiled binary will be located in `target/release/mdisplay`.

If you install it via `cargo install --path .`, the executable is placed in your system's cargo bin path (typically `~/.cargo/bin`).

### Application Launcher (Rofi / Wofi / Application Menu)

To make MDisplay appear in your application launcher (like Rofi), copy the provided `.desktop` file to your local applications folder:

```bash
mkdir -p ~/.local/share/applications
cp mdisplay.desktop ~/.local/share/applications/
```

## Usage

You can customize where `mdisplay` saves your hardware configurations, and whether it automatically links them, by passing arguments before launching the GUI. These preferences are permanently saved to `~/.config/mdisplay/settings.json`.

```bash
# Check current build version
mdisplay --version

# Set a custom save path for the monitors config
mdisplay --set-monitors-path ~/.config/some_folder/my_custom_monitors.conf

# Set a custom target for the main Wayland config appending
mdisplay --set-config-path ~/.config/some_other_folder/config.conf

# Disable auto-appending the source include line completely (you will need to manually add it, if you want it for some reason)
mdisplay --auto-append-source false
```

## Configuration Output Files

The **Save** function integrates natively with mangowc config systems. Output format generally matches:
```conf
monitorrule=name:DP-1,width:1920,height:1080,refresh:144.000000,x:0,y:0,scale:1.000000,rr:0
```
