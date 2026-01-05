# WaybarConf

A powerful, native GTK4/Libadwaita configuration editor for **Waybar**.

WaybarConf provides a "no-code" experience for ricing your Waybar. It allows you to visually manage modules, edit properties, swap layout templates, and create professional color schemes using presets or Material Design 3 extraction.


<img width="1599" height="899" alt="2025-12-31_17-58" src="https://github.com/user-attachments/assets/23e28323-cf8c-4baf-a85f-e95854ca31b6" />
<img width="1599" height="899" alt="2025-12-31_17-58_1" src="https://github.com/user-attachments/assets/486d81e7-921f-4d89-b10a-494e038c4078" />
<img width="1599" height="899" alt="2025-12-31_17-59" src="https://github.com/user-attachments/assets/62871678-25be-4a7d-8249-1f66465c7fde" />


https://github.com/user-attachments/assets/fb5b069f-e2ba-494d-be4d-8e48e70b3383


## Features

- **Three-Column Layout**: Mirrors Waybar's `Left`, `Center`, and `Right` module structure.
- **Group Manager 📁**: 
    - Create and nest modules within hierarchical `group/` types.
    - **Advanced Group Settings**: Enable **Drawer Mode** (slide-out on hover/click), adjust slide duration, and toggle orientation.
- **Drag & Drop**: Easily reorder and relocate modules across columns and into groups.
- **Visual Style Editor 🎨**:
    - **Base Layout Selector**: Switch between fundamental styles (Outline, Pill, Square, Standard) instantly without losing your config.
    - **Bar Position**: Toggle the bar position between **Top** and **Bottom** securely from the UI.
    - **Color Presets**: One-click apply popular themes (Catppuccin, Dracula, Nord, etc.).
    - **Material Extraction**: Automatically generate palettes from your current wallpaper using `matugen`.
    - **Visual Overrides**:
        - Fine-tune **Border Radius**, Margin, Padding, and Font Size per module.
        - **Color Overrides**: Picker for **Text Color** and **Background Color** for specific modules.
- **Animation Engine ⚡**:
    - **Hover Effects**: Glow, Lift, Bounce, Wobble, Shake, and Blink presets.
    - **Constant Animations**: Vibrant ROYGBIV Rainbow, Shiver, and Pulse effects.
    - **Conditional States**: Set percentage thresholds for Battery, CPU, and Memory to trigger animations automatically.
- **Integrated Icon Picker 💠**: Specialized icon grid for easy property customization.
- **Integrated Code Tab**:
    - **JSON Editor**: Direct access to raw module configurations.
    - **CSS Overrides**: Persistent manual CSS patching for specific modules.
- **Profile System**: Save and load your designs as `.wc` profiles.
- **Live Apply**: Push changes to `~/.config/waybar/` and restart Waybar instantly with one click.

## Dependencies

- **Rust** (and Cargo)
- **GTK 4** & **Libadwaita**
- **Matugen** (optional, for wallpaper color extraction)
- **swww** or **hyprpaper** (optional, for auto-wallpaper detection)
- **modern-labwc** (optional, provides the base configuration, fonts, and assets)
- **Script Dependencies** (for specific bricks):
    - `python3`: For weather modules.
    - `pacman-contrib`: For `checkupdates` (Arch Updates brick).
    - `yay` or `paru`: For AUR checks.

## Preset Bricks Library 🧱

WaybarConf comes with a suite of "Bricks"—pre-configured modules you can drag and drop.

### Windows 11 Style
- **Windows Start**: Start button invoking Rofi.
- **Windows Taskbar**: Icon-only task list.
- **Windows Clock**: 12-hour clock with date tooltip.
- **Windows Audio/Network/Power**: Styled system controls.

### Advanced Modules
- **Advanced Weather**: Real-time weather with icons and tooltips (via `wttr.in`).
- **Arch Updates**: Counter for system and AUR updates.
- **Media Player**: MPRIS control for Spotify/Music.
- **Hardware Monitor**: Grouped CPU, RAM, and Temp graph.

## Installation

### Using the Install Script

```bash
git clone https://github.com/ronmurphy/waybarconf
cd waybarconf
chmod +x dependencies.sh install.sh
./dependencies.sh  # Installs system deps and clones modern-labwc
./install.sh       # Builds and installs WaybarConf
```

The script will:
1. Build the binary using Cargo.
2. Install the binary to `~/.local/bin/`.
3. Set up a desktop entry in `~/.local/share/applications/`.
4. Install the application icon.

## Usage

Simply run `waybarconf` from your application launcher or terminal.

1. **Base Style**: Go to the **Styles** tab and pick a "Base Layout" (e.g., Pill or Outline).
2. **Colors**: Pick a "Color Theme" preset or use "Extract Colors" to match your wallpaper.
3. **Add Modules**: Use the search bar in the `+` popover to find Waybar modules.
4. **Customize**: Click a module to edit properties. Use "Visual Overrides" to change specific colors or metrics.
5. **Apply**: Click the **Apply** button to push your new design live.

## Development

To run from source:
```bash
cargo run
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

This project targeting **Waybar** on **Arch Linux** owes significant credit to:

-   **[modern-labwc](https://github.com/Harsh-bin/modern-labwc)**: For providing the exceptional base configurations, style assets, and scripts that power the default experience. `dependencies.sh` automatically integrates these resources to ensure a rich out-of-the-box experience.
-   **Antigravity (Google Deepmind)**: For developing the installation scripts (`install.sh`, `dependencies.sh`), implementing the Windows-style & Advanced Preset Bricks, and architecting the robust dependency integration system.

## License

MIT
