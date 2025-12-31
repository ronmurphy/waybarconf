# WaybarConf

A powerful, native GTK4/Libadwaita configuration editor for **Waybar**.

WaybarConf provides a "no-code" experience for ricing your Waybar. It allows you to visually manage modules, edit properties, and create professional color schemes using Material Design 3 extraction from your wallpaper.

<img width="1599" height="899" alt="2025-12-31_17-58" src="https://github.com/user-attachments/assets/307c5829-7c99-4cb0-a753-1b8a1619b70b" />
<img width="1599" height="899" alt="2025-12-31_17-59" src="https://github.com/user-attachments/assets/3d67f358-fe0f-4b42-b461-a0379b49845a" />
<img width="1599" height="899" alt="2025-12-31_17-58_1" src="https://github.com/user-attachments/assets/c584a559-b17c-46be-8f40-5584a5655bcb" />

## Features

- **Three-Column Layout**: Mirrors Waybar's `Left`, `Center`, and `Right` module structure.
- **Group Manager 📁**: 
    - Create and nest modules within hierarchical `group/` types.
    - **Advanced Group Settings**: Enable **Drawer Mode** (slide-out on hover/click), adjust slide duration, and toggle orientation.
- **Drag & Drop**: Easily reorder and relocate modules across columns and into groups.
- **Visual Style Editor 🎨**:
    - **Visual Overrides**: Fine-tune **Border Radius (Pill mode)**, Margin, Padding, and Font Size per module.
    - **Color Pickers**: Live-edit your Waybar's color scheme.
    - **Material Extraction**: Automatically generate palettes from your current wallpaper using `matugen`.
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

## Installation

### Using the Install Script

```bash
git clone https://github.com/username/waybarconf.git
cd waybarconf
chmod +x install.sh
./install.sh
```

The script will:
1. Build the binary using Cargo.
2. Install the binary to `~/.local/bin/`.
3. Set up a desktop entry in `~/.local/share/applications/`.
4. Install the application icon.

## Usage

Simply run `waybarconf` from your application launcher or terminal.

1. **Load/Save Profiles**: Manage different specialized layouts.
2. **Add Modules**: Use the search bar in the `+` popover to find Waybar modules.
3. **Customize**: Adjust colors and layout metrics in the Styles tab.
4. **Apply**: Click the **Apply** button to push your new design live.

## Development

To run from source:
```bash
cargo run
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT
