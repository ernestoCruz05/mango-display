# MDisplay

A GUI monitor layout manager for [mangowm](https://mangowm.github.io/). Designed to be similar to tools like `nwg-displays` or `wdisplays`.

## Features

* **Visual Canvas**: Drag and drop your screen layouts efficiently with magnetic edge snapping.
* **Hardware Configurations**: Manipulate DPI Scaling, Refresh Rates, Resolutions, Orientation transforms, and Adaptive Sync (VRR).
* **Safe Live Previews**: Temporarily apply your changes to experiment with custom layout configurations. After every apply, a 15-second countdown lets you confirm with **Keep** — or automatically reverts to the previous configuration, so a bad mode can never lock you out.
* **Hotplug Aware**: Monitors plugged or unplugged while the app is open show up immediately on the canvas, without clobbering your unsaved edits.
* **Restore Default**: Safely revert to your base configuration. MDisplay takes a frozen snapshot of your pre-existing monitor rules the very first time it runs, allowing you to easily undo all layout changes without affecting your other `mangowm` settings.
* **Persistent Saving**: Save the finalized `monitorrule` lines directly to `~/.config/mango/monitors.conf`, automatically appended to your `config.conf`.

## Requirements

MDisplay relies on the `wlr-output-management-unstable-v1` Wayland protocol to query the currently active outputs and apply modifications. 

Before running, ensure you have:
* The `mangowm` Wayland compositor installed (or any wlroots-based compositor that supports the protocol).
* Rust toolchain (Cargo, rustc) to compile the application.

## Installation

The easiest way to install MDisplay and make it appear in application launchers (like Rofi or Wofi) is to use the provided install script. This will compile the program and set up the desktop entry automatically:

```bash
git clone https://github.com/ernestoCruz05/mdisplay.git
cd mdisplay
./install.sh
```

### Manual Installation

Alternatively, you can manually compile using Cargo and copy the `.desktop` file:

```bash
cargo install --path .
mkdir -p ~/.local/share/applications
cp mdisplay.desktop ~/.local/share/applications/
```

> [!NOTE]
> MDisplay is installed to Cargo's default binary directory. If your terminal or application launcher says "command not found", ensure `~/.cargo/bin` is in your `PATH`. You can add it by putting `export PATH="$HOME/.cargo/bin:$PATH"` in your `~/.bashrc` or `~/.zshrc`.

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

The **Save** function integrates natively with mangowm config systems. Output format generally matches:
```conf
monitorrule=name:DP-1,width:1920,height:1080,refresh:144.000000,x:0,y:0,scale:1.000000,rr:0,vrr:0
```
