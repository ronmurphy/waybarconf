# WaybarConf

A powerful, native GTK4/Libadwaita configuration editor for **Waybar**.

WaybarConf provides a "no-code" experience for ricing your Waybar. It allows you to visually manage modules, edit properties, swap layout templates, and create professional color schemes using presets or Material Design 3 extraction.

![WaybarConf Screenshot](https://raw.githubusercontent.com/username/waybarconf/main/screenshot.png) <!-- Note: Replace with actual screenshot link when available -->

## Features

- **Three-Column Layout**: Mirrors Waybar's `Left`, `Center`, and `Right` module structure.
- **Group Manager 📁**: 
    - Create and nest modules within hierarchical `group/` types.
    - **Advanced Group Settings**: Enable **Drawer Mode** (slide-out on hover/click), adjust slide duration, and toggle orientation.
- **Drag & Drop**: Easily reorder and relocate modules across columns and into groups.
- **Visual Style Editor 🎨**:
    - **Base Layout Selector**: Switch between fundamental styles (Outline, Pill, Square, Standard) instantly without losing your config.
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

## License

MIT
