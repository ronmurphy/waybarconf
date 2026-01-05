mod config;

use libadwaita as adw;
use gtk4 as gtk;
use gtk::gdk;
use gtk::glib;
use gtk::gio;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use adw::prelude::*;
use adw::{ActionRow, Application, ApplicationWindow, HeaderBar, ViewStack, ViewSwitcher, PreferencesGroup, ToastOverlay, Toast, MessageDialog, ComboRow};
use gtk::{Box as GtkBox, ListBox, Orientation, Label, ScrolledWindow, TextView, Entry, Switch, Button, ColorButton, FileDialog, FileFilter, StringList, SearchEntry, Scale};
use crate::config::{WaybarConfig, WaybarProfile};

const DEFAULT_CONFIG_JSON: &str = r#"{
    "modules-left": ["custom/launcher", "wlr/taskbar"],
    "modules-center": ["clock"],
    "modules-right": ["cpu", "memory", "pulseaudio", "network", "tray"],
    "clock": { "format": "{:%I:%M %p}", "tooltip-format": "<big>{:%Y %B}</big>\n<tt><small>{calendar}</small></tt>" },
    "cpu": { "format": "CPU {usage}%" },
    "memory": { "format": "MEM {percentage}%" },
    "pulseaudio": { "format": "{volume}% {icon}", "format-icons": { "default": ["", "", ""] } },
    "network": { "format-wifi": "", "format-ethernet": "󰈀", "format-disconnected": "󰤮" },
    "custom/launcher": { "format": "", "on-click": "rofi -show drun" }
}"#;

const DEFAULT_STYLE_VARS: &str = r#"/* WaybarConf Style Variables */
@define-color bar_bg #1e1e2e;
@define-color bar_fg #cdd6f4;
@define-color module_bg #313244;
@define-color module_fg #cdd6f4;
@define-color hover_bg #45475a;
@define-color hover_fg #f5e0dc;
@define-color border_color #585b70;

"#;

const DEFAULT_LAYOUT_CSS: &str = r#"/* WaybarConf Layout CSS */
#clock, #cpu, #battery, #backlight, #pulseaudio, #network, #memory, #tray, #idle_inhibitor, #bluetooth, #cava, #disk, #temperature, #upower, #wireplumber, #mpris, #mpd, #backlight-slider, #pulseaudio-slider, #power-profiles-daemon, #privacy, #load, #jack, #sndio, #systemd-failed-units, #user, .user, .cpu, .load, #cffi, #gamemode, #inhibitor, #image, #keyboard-state, #workspaces, #taskbar, #wlr-taskbar, #hyprland-workspaces, #hyprland-window, #hyprland-submap, #hyprland-language, #sway-workspaces, #sway-window, #sway-mode, #sway-scratchpad, #niri-workspaces, #niri-window, #river-tags, #river-window, #river-mode, #river-layout, #dwl-tags, #dwl-window, #window, #mode, #scratchpad, #tags, #layout, #language, #submap, .custom {
    padding: 4px 8px;
    margin: 0 4px;
    background: @module_bg;
    color: @module_fg;
    border-radius: 8px;
    transition: all 0.3s ease;
}

@keyframes blink {
    to {
        background-color: rgba(255, 255, 255, 0.1);
        color: @module_fg;
    }
}

@keyframes glow_pulse {
    0% { background-color: @module_bg; }
    50% { background-color: @hover_bg; }
    100% { background-color: @module_bg; }
}

@keyframes lift {
    to { margin-top: -2px; }
}

@keyframes bounce {
    0% { margin-top: 0; }
    50% { margin-top: -5px; }
    100% { margin-top: 0; }
}

@keyframes shiver {
    0% { margin-left: 0; }
    25% { margin-left: -2px; }
    75% { margin-left: 2px; }
    100% { margin-left: 0; }
}

@keyframes rainbow {
    0% { color: #ff0000; }
    16% { color: #ff7f00; }
    33% { color: #ffff00; }
    50% { color: #00ff00; }
    66% { color: #0000ff; }
    83% { color: #4b0082; }
    100% { color: #9400d3; }
}

@keyframes spin {
    from { margin-left: 0; }
    to { margin-left: 0.1px; } 
}

@keyframes shake {
    0% { margin-left: 0; }
    10% { margin-left: -4px; }
    30% { margin-left: 4px; }
    50% { margin-left: -4px; }
    70% { margin-left: 4px; }
    90% { margin-left: -4px; }
    100% { margin-left: 0; }
}
"#;

const ICON_LIST: &[&str] = &[
    "󰣇", "󰀻", "󰀕", "󰍛", "󰘚", "", "", "", "", "", "", "", "󰋊", "󰝚", "󰂄", "", "", "󰊠", "󰀘", "󰀯",
    "", "󰠮", "󰂚", "󰵗", "󰦈", "󰃠", "󰃡", "󰖩", "󰖪", "󰤨", "󰤭", "󰥔", "󰥒", "󰥓", "󰥖", "󰥑", "󰥕", "󰥐"
];

fn main() {
    let application = Application::builder()
        .application_id("com.github.waybarconf")
        .build();

    application.connect_activate(move |app| {
        build_ui(app);
    });
    application.run();
}

struct StyleConfig {
    vars: indexmap::IndexMap<String, String>,
    path: PathBuf,
}

impl StyleConfig {
    fn from_file(path: &Path) -> Self {
        let vars = if let Ok(content) = fs::read_to_string(path) {
            parse_style_vars(&content)
        } else {
            indexmap::IndexMap::new()
        };
        Self { vars, path: path.to_path_buf() }
    }

    fn save(&self) -> std::io::Result<()> {
        self.save_to(&self.path)
    }
    
    fn save_to(&self, path: &Path) -> std::io::Result<()> {
        let mut content = String::from("/* WaybarConf Style Variables */\n\n");
        let mut metrics = Vec::new();
        for (k, v) in &self.vars {
            if v.ends_with("px") || v.parse::<f64>().is_ok() {
                metrics.push((k, v));
            } else {
                content.push_str(&format!("@define-color {} {};\n", k, v));
            }
        }
        if !metrics.is_empty() {
            content.push_str("\n* {\n");
            for (k, v) in metrics {
                content.push_str(&format!("    --{}: {};\n", k, v));
            }
            content.push_str("}\n");
        }
        fs::write(path, content)
    }
}

fn detect_wallpaper() -> Option<String> {
    if let Ok(output) = Command::new("swww").arg("query").output() {
        let s = String::from_utf8_lossy(&output.stdout);
        if let Some(pos) = s.find("image: ") {
            let path = s[pos + 7..].trim().to_string();
            if Path::new(&path).exists() { return Some(path); }
        }
    }
    if let Ok(output) = Command::new("hyprctl").args(["hyprpaper", "listactive"]).output() {
        let s = String::from_utf8_lossy(&output.stdout);
        if let Some(pos) = s.find("wallpaper: ") {
            let path = s[pos + 11..].split_whitespace().next().unwrap_or("").to_string();
            if !path.is_empty() && Path::new(&path).exists() { return Some(path); }
        }
    }
    None
}

fn get_waybar_config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(home).join(".config"));
    
    let paths = vec![
        xdg_config.join("waybar/config.jsonc"),
        xdg_config.join("waybar/config"),
    ];
    
    paths.into_iter().find(|p| p.exists())
}

fn parse_style_vars(content: &str) -> indexmap::IndexMap<String, String> {
    let mut vars = indexmap::IndexMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("@define-color") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[1].to_string();
                let value = parts[2].trim_matches(';').to_string();
                vars.insert(name, value);
            }
        } else if line.starts_with("--") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let name = parts[0].trim_matches('-').trim().to_string();
                let value = parts[1].trim_matches(';').trim().to_string();
                vars.insert(name, value);
            }
        }
    }
    vars
}

fn apply_matugen(path: &str, scheme_type: &str, style_rc: Rc<RefCell<StyleConfig>>) -> Result<(), String> {
    let output = Command::new("matugen")
        .args(["image", path, "-j", "hex", "--type", scheme_type])
        .output()
        .map_err(|e| format!("Failed to run matugen: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse matugen output: {}", e))?;

    let colors = json.get("colors").ok_or("No 'colors' in matugen output")?;

    let get_color = |role: &str| -> Option<String> {
        colors.get(role).and_then(|r| r.get("dark")).and_then(|v| v.as_str()).map(|s| s.to_string())
    };

    let mut style = style_rc.borrow_mut();
    if let Some(c) = get_color("surface") { style.vars.insert("bar_bg".into(), c); }
    if let Some(c) = get_color("on_surface") { style.vars.insert("bar_fg".into(), c); }
    if let Some(c) = get_color("secondary_container") { style.vars.insert("module_bg".into(), c); }
    if let Some(c) = get_color("on_secondary_container") { style.vars.insert("module_fg".into(), c); }
    if let Some(c) = get_color("primary") { style.vars.insert("hover_bg".into(), c); }
    if let Some(c) = get_color("on_primary") { style.vars.insert("hover_fg".into(), c); }
    if let Some(c) = get_color("outline") { style.vars.insert("border_color".into(), c); }
    
    let _ = style.save();
    Ok(())
}

fn ensure_keyframes(lines: &mut Vec<String>) {
    let css = lines.join("\n");
    if !css.contains("@keyframes rainbow") {
        lines.push("\n/* Animation Keyframes */".to_string());
        lines.push("@keyframes blink { to { background-color: rgba(255, 255, 255, 0.1); color: @module_fg; } }".to_string());
        lines.push("@keyframes glow_pulse { 0% { background-color: @module_bg; } 50% { background-color: @hover_bg; } 100% { background-color: @module_bg; } }".to_string());
        lines.push("@keyframes lift { to { margin-top: -2px; } }".to_string());
        lines.push("@keyframes bounce { 0% { margin-top: 0; } 50% { margin-top: -5px; } 100% { margin-top: 0; } }".to_string());
        lines.push("@keyframes wobble { 0% { margin-left: 0; } 25% { margin-left: -3px; } 75% { margin-left: 3px; } 100% { margin-left: 0; } }".to_string());
        lines.push("@keyframes shake { 0% { margin-left: 0; } 10% { margin-left: -4px; } 30% { margin-left: 4px; } 50% { margin-left: -4px; } 70% { margin-left: 4px; } 90% { margin-left: -4px; } 100% { margin-left: 0; } }".to_string());
        lines.push("@keyframes shiver { 0% { margin-left: 0; } 25% { margin-left: -2px; } 75% { margin-left: 2px; } 100% { margin-left: 0; } }".to_string());
        lines.push("@keyframes rainbow { 0% { color: #ff0000; } 16% { color: #ff7f00; } 33% { color: #ffff00; } 50% { color: #00ff00; } 66% { color: #0000ff; } 83% { color: #4b0082; } 100% { color: #9400d3; } }".to_string());
    }
}

fn update_module_css(path: &Path, mod_name: &str, suffix: &str, prop: &str, value: &str) {
    let id = format!("#{}{}", mod_name.replace("/", "-"), suffix);
    let full_css = fs::read_to_string(path).unwrap_or_else(|_| DEFAULT_LAYOUT_CSS.to_string());
    let mut lines: Vec<String> = full_css.lines().map(|s| s.to_string()).collect();
    
    ensure_keyframes(&mut lines);
    
    let mut block_start = None;
    let mut block_end = None;
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with(&id) && (line.contains('{') || i + 1 < lines.len() && lines[i+1].contains('{')) {
            block_start = Some(i);
        }
        if block_start.is_some() && line.contains('}') {
            block_end = Some(i);
            break;
        }
    }

    if let (Some(start), Some(end)) = (block_start, block_end) {
        let mut prop_idx = None;
        for i in start + 1..end {
            if lines[i].trim().starts_with(prop) {
                prop_idx = Some(i);
                break;
            }
        }

        if value.is_empty() {
             if let Some(idx) = prop_idx { lines.remove(idx); }
        } else {
            let new_line = format!("    {}: {};", prop, value);
            if let Some(idx) = prop_idx {
                lines[idx] = new_line;
            } else {
                lines.insert(end, new_line);
            }
        }
    } else if !value.is_empty() {
        lines.push(format!("\n{} {{\n    {}: {};\n}}", id, prop, value));
    }

    let _ = fs::write(path, lines.join("\n"));
}

fn get_module_css_prop(path: &Path, mod_name: &str, suffix: &str, prop: &str) -> Option<String> {
    let id = format!("#{}{}", mod_name.replace("/", "-"), suffix);
    let full_css = fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = full_css.lines().collect();
    
    let mut block_start = None;
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with(&id) && (line.contains('{') || i + 1 < lines.len() && lines[i+1].contains('{')) {
            block_start = Some(i);
            break;
        }
    }

    if let Some(start) = block_start {
        for line in &lines[start + 1..] {
            if line.contains('}') { break; }
            if line.trim().starts_with(prop) {
                let val = line.split(':').collect::<Vec<&str>>()[1].trim().trim_matches(';').to_string();
                return Some(val);
            }
        }
    }
    None
}

fn build_ui(app: &Application) {
    let waybar_config: WaybarConfig = serde_json::from_str(DEFAULT_CONFIG_JSON).unwrap();
    let config_rc = Rc::new(RefCell::new(waybar_config));
    
    let style_vars = parse_style_vars(DEFAULT_STYLE_VARS);
    let default_style_path = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/waybar/colors/wallpaper.css");
    let style_rc = Rc::new(RefCell::new(StyleConfig { vars: style_vars, path: default_style_path }));
    
    // Use a temporary path for session CSS
    let layout_css_path = std::env::temp_dir().join("waybarconf_style.css");
    let _ = fs::write(&layout_css_path, DEFAULT_LAYOUT_CSS);
    
    let selected_module_state = Rc::new(RefCell::new(None::<(String, String)>));

    let main_box = GtkBox::new(Orientation::Vertical, 0);
    let toast_overlay = ToastOverlay::new();
    
    let header = HeaderBar::new();
    let save_profile_btn = Button::with_label("Save Profile");
    save_profile_btn.add_css_class("suggested-action");
    header.pack_start(&save_profile_btn);
    
    let load_profile_btn = Button::with_label("Load Profile");
    header.pack_start(&load_profile_btn);


    
    let apply_btn = Button::with_label("Apply");
    apply_btn.add_css_class("accent");
    header.pack_start(&apply_btn);

    main_box.append(&header);
    
    let paned = gtk::Paned::new(Orientation::Horizontal);
    paned.set_wide_handle(true);

    let columns_box = GtkBox::new(Orientation::Horizontal, 12);
    columns_box.set_margin_top(12);
    columns_box.set_margin_bottom(12);
    columns_box.set_margin_start(12);
    columns_box.set_margin_end(12);
    columns_box.set_homogeneous(true);

    let settings_panel = GtkBox::new(Orientation::Vertical, 0);
    settings_panel.set_width_request(450);
    
    let view_stack = ViewStack::new();
    let view_switcher = ViewSwitcher::new();
    view_switcher.set_stack(Some(&view_stack));
    settings_panel.append(&view_switcher);
    
    let properties_page = GtkBox::new(Orientation::Vertical, 12);
    properties_page.set_margin_top(12);
    properties_page.set_margin_bottom(12);
    properties_page.set_margin_start(12);
    properties_page.set_margin_end(12);
    let props_scroll = ScrolledWindow::builder().child(&properties_page).vexpand(true).build();
    
    let styles_page = GtkBox::new(Orientation::Vertical, 12);
    styles_page.set_margin_top(12);
    styles_page.set_margin_bottom(12);
    styles_page.set_margin_start(12);
    styles_page.set_margin_end(12);
    let styles_scroll = ScrolledWindow::builder().child(&styles_page).vexpand(true).build();

    let code_page = GtkBox::new(Orientation::Vertical, 12);
    code_page.set_margin_top(12);
    code_page.set_margin_bottom(12);
    code_page.set_margin_start(12);
    code_page.set_margin_end(12);
    
    view_stack.add_titled(&props_scroll, Some("properties"), "Properties");
    view_stack.add_titled(&styles_scroll, Some("styles"), "Styles");
    view_stack.add_titled(&code_page, Some("code"), "Code");
    settings_panel.append(&view_stack);

    let left_list = ListBox::new();
    left_list.add_css_class("boxed-list");
    let center_list = ListBox::new();
    center_list.add_css_class("boxed-list");
    let right_list = ListBox::new();
    right_list.add_css_class("boxed-list");

    let module_options = vec![
        "clock", "battery", "cpu", "memory", "network", "pulseaudio", "backlight", "tray",
        "keyboard-state", "wlr/taskbar", "idle_inhibitor", "bluetooth", "cava", "disk", "mpd", "mpris",
        "hyprland/workspaces", "hyprland/window", "hyprland/submap", "hyprland/language",
        "sway/workspaces", "sway/window", "sway/mode", "sway/scratchpad",
        "temperature", "upower", "wireplumber", "image", "gamemode", "inhibitor",
        "backlight/slider", "pulseaudio/slider", "power-profiles-daemon", "niri/workspaces",
        "niri/window", "privacy", "load", "river/tags", "river/window", "river/mode",
        "river/layout", "dwl/tags", "dwl/window", "jack", "sndio", "systemd-failed-units",
        "user", "cffi", "custom/new-module"
    ];

    let update_properties_fn = Rc::new(RefCell::new(None::<Box<dyn Fn(String)>>));
    let refresh_ui_fn: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let refresh_ui = {
        let config_rc = Rc::clone(&config_rc);
        let left_list = left_list.clone();
        let center_list = center_list.clone();
        let right_list = right_list.clone();
        let update_props_ref = Rc::clone(&update_properties_fn);
        let sel_state = Rc::clone(&selected_module_state);
        let refresh_ui_fn_c = Rc::clone(&refresh_ui_fn);
        
        move || {
            let config = config_rc.borrow();
            
            fn populate_recursive(list: &ListBox, modules: &[String], col_id: &str, depth: u32, cfg: &WaybarConfig, 
                                 update_cb: &Rc<RefCell<Option<Box<dyn Fn(String)>>>>, 
                                 sel_s: &Rc<RefCell<Option<(String, String)>>>,
                                 config_rc: &Rc<RefCell<WaybarConfig>>,
                                 refresh_ui_fn: &Rc<RefCell<Option<Box<dyn Fn()>>>>) {
                for m in modules {
                    let row = create_module_row(m, depth);
                    let name = m.clone();
                    let cid = col_id.to_string();
                    let update_cb_c = Rc::clone(update_cb);
                    let sel_s_c = Rc::clone(sel_s);
                    
                    row.connect_activated(move |_| {
                        *sel_s_c.borrow_mut() = Some((cid.clone(), name.clone()));
                        if let Some(f) = &*update_cb_c.borrow() { f(name.clone()); }
                    });
                    
                    if depth > 0 {
                        let ungroup_btn = Button::builder().icon_name("edit-undo-symbolic").has_frame(false).tooltip_text("Ungroup").build();
                        let cfg_u = Rc::clone(config_rc); let ref_u = Rc::clone(refresh_ui_fn); 
                        let name_u = m.clone(); let cid_u = col_id.to_string();
                        ungroup_btn.connect_clicked(move |_| {
                            let mut cfg = cfg_u.borrow_mut();
                            if let Some(it) = remove_module_anywhere(&mut cfg, &name_u) {
                                match cid_u.as_str() { 
                                    "left" => cfg.modules_left.push(it),
                                    "center" => cfg.modules_center.push(it),
                                    "right" => cfg.modules_right.push(it),
                                    _ => {}
                                }
                            }
                            drop(cfg); 
                            if let Some(f) = &*ref_u.borrow() { f(); }
                        });
                        row.add_suffix(&ungroup_btn);
                    }
                    
                    let ds = gtk::DragSource::new();
                    ds.set_actions(gdk::DragAction::MOVE);
                    let full_id = format!("{}:{}", col_id, m);
                    ds.connect_prepare(move |_, _, _| Some(gdk::ContentProvider::for_value(&full_id.to_value())));
                    row.add_controller(ds);
                    list.append(&row);
                    
                    if m.starts_with("group/") {
                        if let Some(def) = cfg.module_definitions.get(m) {
                            if let Some(children) = def.get("modules").and_then(|v| v.as_array()) {
                                let child_names: Vec<String> = children.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                                populate_recursive(list, &child_names, col_id, depth + 1, cfg, update_cb, sel_s, config_rc, refresh_ui_fn);
                            }
                        }
                    }
                }
            }

            while let Some(child) = left_list.first_child() { left_list.remove(&child); }
            while let Some(child) = center_list.first_child() { center_list.remove(&child); }
            while let Some(child) = right_list.first_child() { right_list.remove(&child); }

            let r_fn = Rc::clone(&refresh_ui_fn_c);
            populate_recursive(&left_list, &config.modules_left, "left", 0, &config, &update_props_ref, &sel_state, &config_rc, &r_fn);
            populate_recursive(&center_list, &config.modules_center, "center", 0, &config, &update_props_ref, &sel_state, &config_rc, &r_fn);
            populate_recursive(&right_list, &config.modules_right, "right", 0, &config, &update_props_ref, &sel_state, &config_rc, &r_fn);
        }
    };

    *refresh_ui_fn.borrow_mut() = Some(Box::new(refresh_ui.clone()));
    
    let refresh_call = {
        let refresh_ui_fn = Rc::clone(&refresh_ui_fn);
        move || { if let Some(f) = &*refresh_ui_fn.borrow() { f(); } }
    };
    let refresh_rc: Rc<dyn Fn()> = Rc::new(refresh_call);

    // --- Tab Update Logic ---
    let toast_ref = toast_overlay.clone();
    *update_properties_fn.borrow_mut() = Some(Box::new({
        let config_rc = Rc::clone(&config_rc);
        let style_rc = Rc::clone(&style_rc);
        let props_page = properties_page.clone();
        let code_page = code_page.clone();
        let refresh_rc = Rc::clone(&refresh_rc);
        let update_props_self = Rc::clone(&update_properties_fn);
        let layout_css_path = layout_css_path.clone();
        let sel_state_props = Rc::clone(&selected_module_state);
        let toast_p = toast_ref.clone();
        
        move |mod_name| {
            let config_borrow_orig = config_rc.borrow();
            
            while let Some(child) = props_page.first_child() { props_page.remove(&child); }
            let header = GtkBox::new(Orientation::Horizontal, 12);
            let title = Label::new(Some(&format!("Settings: {}", mod_name)));
            title.add_css_class("title-3");
            header.append(&title);

            if mod_name.starts_with("custom/") {
                let rename_btn = Button::builder().icon_name("edit-symbolic").has_frame(false).valign(gtk::Align::Center).build();
                let mod_rename_orig = mod_name.clone();
                let config_rename = Rc::clone(&config_rc);
                let refresh_rename = Rc::clone(&refresh_rc);
                let update_rename = Rc::clone(&update_props_self);
                let sel_rename = Rc::clone(&sel_state_props);
                let toast_rename = toast_p.clone();
                
                rename_btn.connect_clicked(move |_| {
                    let dialog = MessageDialog::builder().heading("Rename Module").body("Enter the new name for this module (must start with 'custom/')").build();
                    let entry = Entry::builder().text(&mod_rename_orig).margin_top(12).build();
                    dialog.set_extra_child(Some(&entry));
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("rename", "Rename");
                    dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
                    let cfg = Rc::clone(&config_rename);
                    let ref_r = Rc::clone(&refresh_rename);
                    let upd_r = Rc::clone(&update_rename);
                    let old_name = mod_rename_orig.clone();
                    let sel_r = Rc::clone(&sel_rename);
                    let toast_r = toast_rename.clone();
                    dialog.connect_response(None, move |d, response| {
                        if response == "rename" {
                            let new_name = entry.text().to_string();
                            if !new_name.is_empty() && new_name.starts_with("custom/") {
                                let mut c = cfg.borrow_mut();
                                if let Some(def) = c.module_definitions.shift_remove(&old_name) { c.module_definitions.insert(new_name.clone(), def); }
                                let replace = |list: &mut Vec<String>| { for m in list.iter_mut() { if m == &old_name { *m = new_name.clone(); } } };
                                replace(&mut c.modules_left); replace(&mut c.modules_center); replace(&mut c.modules_right);
                                if let Some(ref mut s) = *sel_r.borrow_mut() { if s.1 == old_name { s.1 = new_name.clone(); } }
                                drop(c); ref_r();
                                if let Some(f) = &*upd_r.borrow() { f(new_name.clone()); }
                                toast_r.add_toast(Toast::new(&format!("Renamed to {}", new_name)));
                            }
                        }
                        d.close();
                    });
                    dialog.present();
                });
                header.append(&rename_btn);
            }
            props_page.append(&header);
            
            let group = PreferencesGroup::new();
            if let Some(def) = config_borrow_orig.module_definitions.get(&mod_name) {
                if let Some(obj) = def.as_object() {
                    for (k, v) in obj {
                        let row = ActionRow::new();
                        row.set_title(k);
                        let del_btn = Button::builder().icon_name("user-trash-symbolic").has_frame(false).build();
                        let k_del = k.clone();
                        let mod_del = mod_name.clone();
                        let config_del = Rc::clone(&config_rc);
                        let refresh_del = Rc::clone(&refresh_rc);
                        let update_del = Rc::clone(&update_props_self);
                        del_btn.connect_clicked(move |_| {
                            if let Some(o) = config_del.borrow_mut().module_definitions.get_mut(&mod_del).and_then(|d| d.as_object_mut()) { o.remove(&k_del); }
                            refresh_del();
                            if let Some(f) = &*update_del.borrow() { f(mod_del.clone()); }
                        });
                        row.add_suffix(&del_btn);
                        let k_inner = k.clone();
                        let mod_inner = mod_name.clone();
                        let config_inner = Rc::clone(&config_rc);
                        match v {
                            serde_json::Value::Bool(b) => {
                                let sw = Switch::builder().active(*b).valign(gtk::Align::Center).build();
                                sw.connect_state_set(move |_, state| {
                                    if let Some(o) = config_inner.borrow_mut().module_definitions.get_mut(&mod_inner).and_then(|d| d.as_object_mut()) { o.insert(k_inner.clone(), serde_json::Value::Bool(state)); }
                                    glib::Propagation::Proceed
                                });
                                row.add_suffix(&sw);
                            }
                            _ => {
                                let en = Entry::builder().text(v.to_string().replace("\"", "")).valign(gtk::Align::Center).build();
                                en.connect_changed(move |e| {
                                    let text = e.text().to_string();
                                    if let Some(o) = config_inner.borrow_mut().module_definitions.get_mut(&mod_inner).and_then(|d| d.as_object_mut()) {
                                        let val = if let Ok(n) = text.parse::<i64>() { serde_json::Value::Number(n.into()) } else { serde_json::Value::String(text) };
                                        o.insert(k_inner.clone(), val);
                                    }
                                });
                                
                                let icon_btn = Button::builder().icon_name("face-smile-symbolic").has_frame(false).build();
                                let pop = gtk::Popover::new();
                                let grid = gtk::FlowBox::builder().max_children_per_line(8).min_children_per_line(8).selection_mode(gtk::SelectionMode::None).build();
                                grid.set_margin_top(8); grid.set_margin_bottom(8); grid.set_margin_start(8); grid.set_margin_end(8);
                                
                                for icon in ICON_LIST {
                                    let btn = Button::with_label(icon);
                                    btn.add_css_class("flat");
                                    let e_c = en.clone(); let i_c = icon.to_string();
                                    let p_weak = pop.downgrade();
                                    btn.connect_clicked(move |_| {
                                        let text = e_c.text().to_string();
                                        e_c.set_text(&(text + &i_c));
                                        if let Some(p) = p_weak.upgrade() { p.popdown(); }
                                    });
                                    grid.insert(&btn, -1);
                                }
                                pop.set_child(Some(&grid));
                                pop.set_parent(&icon_btn);
                                icon_btn.connect_clicked(move |_| pop.popup());
                                
                                row.add_suffix(&en);
                                row.add_suffix(&icon_btn);
                            }
                        }
                        group.add(&row);
                    }
                }
            }
            props_page.append(&group);
            
            let add_btn = Button::with_label("Add Property");
            add_btn.add_css_class("pill"); add_btn.add_css_class("suggested-action");
            let mod_add = mod_name.clone();
            let config_add = Rc::clone(&config_rc);
            let refresh_add = Rc::clone(&refresh_rc);
            let update_add = Rc::clone(&update_props_self);
            add_btn.connect_clicked(move |_| {
                if let Some(o) = config_add.borrow_mut().module_definitions.get_mut(&mod_add).and_then(|d| d.as_object_mut()) {
                    for d in vec!["format", "tooltip", "on-click", "interval", "exec"] { if !o.contains_key(d) { o.insert(d.to_string(), serde_json::Value::String("".to_string())); break; } }
                }
                refresh_add();
                if let Some(f) = &*update_add.borrow() { f(mod_add.clone()); }
            });
            props_page.append(&add_btn);
            
            // --- Visual Overrides Section ---
            let vis_group = PreferencesGroup::new();
            vis_group.set_title("Visual Overrides");
            vis_group.set_description(Some("These settings override default styles and are saved to your layout CSS."));
            
            let create_css_row = |title: &str, prop: &str, min: f64, max: f64, layout_path: PathBuf, mod_name: String, update_fn: Rc<RefCell<Option<Box<dyn Fn(String)>>>>| {
                let row = ActionRow::new();
                row.set_title(title);
                let current = get_module_css_prop(&layout_path, &mod_name, "", prop)
                    .and_then(|v| v.chars().filter(|c| c.is_digit(10)).collect::<String>().parse::<f64>().ok())
                    .unwrap_or(if prop == "font-size" { 14.0 } else { 0.0 });
                
                let scale = gtk::Scale::with_range(Orientation::Horizontal, min, max, 1.0);
                scale.set_value(current);
                scale.set_width_request(150);
                scale.set_valign(gtk::Align::Center);
                
                let lp = layout_path.clone(); let mn = mod_name.clone(); let pr = prop.to_string();
                let upd = Rc::clone(&update_fn);
                scale.connect_value_changed(move |s| {
                    let val = format!("{}px", s.value() as i32);
                    update_module_css(&lp, &mn, "", &pr, &val);
                    if let Some(f) = &*upd.borrow() { f(mn.clone()); }
                });
                row.add_suffix(&scale);
                row
            };
            
            vis_group.add(&create_css_row("Font Size", "font-size", 6.0, 48.0, layout_css_path.clone(), mod_name.clone(), Rc::clone(&update_props_self)));
            vis_group.add(&create_css_row("Margin", "margin", 0.0, 50.0, layout_css_path.clone(), mod_name.clone(), Rc::clone(&update_props_self)));
            vis_group.add(&create_css_row("Padding", "padding", 0.0, 50.0, layout_css_path.clone(), mod_name.clone(), Rc::clone(&update_props_self)));
            vis_group.add(&create_css_row("Border Radius", "border-radius", 0.0, 50.0, layout_css_path.clone(), mod_name.clone(), Rc::clone(&update_props_self)));
            
            props_page.append(&vis_group);
            
            // --- Color Overrides (FG/BG) ---
            let color_group = PreferencesGroup::new();
            color_group.set_title("Color Overrides");
            
            // Helper to resolve color
            let get_current_color = |prop: &str, style_rc: &Rc<RefCell<StyleConfig>>| -> Option<gdk::RGBA> {
                if let Some(val) = get_module_css_prop(&layout_css_path, &mod_name, "", prop) {
                    if let Ok(c) = gdk::RGBA::parse(&val) { return Some(c); }
                    if val.starts_with('@') {
                        let var_name = val.trim_start_matches('@');
                        if let Some(resolved) = style_rc.borrow().vars.get(var_name) {
                             if let Ok(c) = gdk::RGBA::parse(resolved) { return Some(c); }
                        }
                    }
                }
                None
            };
            
            // Text Color
            let fg_row = ActionRow::new();
            fg_row.set_title("Text Color");
            let fg_btn = ColorButton::new();
            if let Some(c) = get_current_color("color", &style_rc) { fg_btn.set_rgba(&c); }
            let lp_fg = layout_css_path.clone(); let mn_fg = mod_name.clone(); let upd_fg = Rc::clone(&update_props_self);
            fg_btn.connect_color_set(move |btn| {
                let rgba = btn.rgba();
                let hex = format!("#{:02x}{:02x}{:02x}", (rgba.red() * 255.0) as u8, (rgba.green() * 255.0) as u8, (rgba.blue() * 255.0) as u8);
                update_module_css(&lp_fg, &mn_fg, "", "color", &hex);
                if let Some(f) = &*upd_fg.borrow() { f(mn_fg.clone()); }
            });
            fg_row.add_suffix(&fg_btn);
            color_group.add(&fg_row);
            
            // Background Color
            let bg_row = ActionRow::new();
            bg_row.set_title("Background Color");
            let bg_btn = ColorButton::new();
            if let Some(c) = get_current_color("background-color", &style_rc).or_else(|| get_current_color("background", &style_rc)) { bg_btn.set_rgba(&c); }
            let lp_bg = layout_css_path.clone(); let mn_bg = mod_name.clone(); let upd_bg = Rc::clone(&update_props_self);
            bg_btn.connect_color_set(move |btn| {
                 let rgba = btn.rgba();
                 let hex = format!("#{:02x}{:02x}{:02x}", (rgba.red() * 255.0) as u8, (rgba.green() * 255.0) as u8, (rgba.blue() * 255.0) as u8);
                 update_module_css(&lp_bg, &mn_bg, "", "background-color", &hex);
                 update_module_css(&lp_bg, &mn_bg, "", "background", "");
                 if let Some(f) = &*upd_bg.borrow() { f(mn_bg.clone()); }
            });
            bg_row.add_suffix(&bg_btn);
            color_group.add(&bg_row);
            
            props_page.append(&color_group);

            // --- Group Configuration (Drawer/Orientation) ---
            if mod_name.starts_with("group/") {
                let group_cfg = PreferencesGroup::new();
                group_cfg.set_title("Group Configuration");
                
                // Drawer Switch
                let drawer_row = ActionRow::new();
                drawer_row.set_title("Drawer Mode (Slide-out)");
                let drawer_sw = Switch::builder().valign(gtk::Align::Center).build();
                if let Some(def) = config_borrow_orig.module_definitions.get(&mod_name) {
                    if def.get("drawer").is_some() { drawer_sw.set_active(true); }
                }

                let cfg_d = Rc::clone(&config_rc); let mn_d = mod_name.clone(); let ref_d = Rc::clone(&refresh_rc);
                drawer_sw.connect_state_set(move |_, state| {
                    let mut c = cfg_d.borrow_mut();
                    if let Some(def) = c.module_definitions.get_mut(&mn_d) {
                        if state {
                            if def.get("drawer").is_none() {
                                def.as_object_mut().unwrap().insert("drawer".to_string(), serde_json::json!({ "transition-duration": 500 }));
                            }
                        } else {
                            def.as_object_mut().unwrap().remove("drawer");
                        }
                    }
                    drop(c); ref_d();
                    glib::Propagation::Proceed
                });
                drawer_row.add_suffix(&drawer_sw);
                group_cfg.add(&drawer_row);

                // Drawer Duration
                let dur_row = ActionRow::new();
                dur_row.set_title("Drawer Duration (ms)");
                let dur_adj = gtk::Adjustment::new(500.0, 0.0, 2000.0, 50.0, 100.0, 0.0);
                let dur_scale = Scale::new(gtk::Orientation::Horizontal, Some(&dur_adj));
                dur_scale.set_width_request(150);
                dur_scale.set_draw_value(true);
                
                if let Some(def) = config_borrow_orig.module_definitions.get(&mod_name) {
                    if let Some(d) = def.get("drawer").and_then(|v| v.get("transition-duration")) {
                        if let Some(f) = d.as_f64() { dur_scale.set_value(f); }
                    }
                }

                let cfg_dur = Rc::clone(&config_rc); let mn_dur = mod_name.clone(); let ref_dur = Rc::clone(&refresh_rc);
                dur_scale.connect_value_changed(move |s| {
                    let mut c = cfg_dur.borrow_mut();
                    if let Some(def) = c.module_definitions.get_mut(&mn_dur).and_then(|d| d.as_object_mut()) {
                        if let Some(drawer) = def.get_mut("drawer").and_then(|d| d.as_object_mut()) {
                            drawer.insert("transition-duration".to_string(), serde_json::json!(s.value() as i64));
                        }
                    }
                    drop(c); ref_dur();
                });
                dur_row.add_suffix(&dur_scale);
                group_cfg.add(&dur_row);

                // Click to Reveal
                let click_row = ActionRow::new();
                click_row.set_title("Click to reveal");
                let click_sw = Switch::builder().valign(gtk::Align::Center).build();
                if let Some(def) = config_borrow_orig.module_definitions.get(&mod_name) {
                    if let Some(d) = def.get("drawer") {
                        if d.get("click-to-reveal").and_then(|v| v.as_bool()).unwrap_or(false) { click_sw.set_active(true); }
                    }
                }
                let cfg_c = Rc::clone(&config_rc); let mn_c = mod_name.clone(); let ref_c = Rc::clone(&refresh_rc);
                click_sw.connect_state_set(move |_, state| {
                    let mut c = cfg_c.borrow_mut();
                    if let Some(def) = c.module_definitions.get_mut(&mn_c).and_then(|d| d.as_object_mut()) {
                        if let Some(drawer) = def.get_mut("drawer").and_then(|d| d.as_object_mut()) {
                            drawer.insert("click-to-reveal".to_string(), serde_json::json!(state));
                        }
                    }
                    drop(c); ref_c();
                    glib::Propagation::Proceed
                });
                click_row.add_suffix(&click_sw);
                group_cfg.add(&click_row);

                // Left to Right
                let ltr_row = ActionRow::new();
                ltr_row.set_title("Left to right transition");
                let ltr_sw = Switch::builder().valign(gtk::Align::Center).build();
                if let Some(def) = config_borrow_orig.module_definitions.get(&mod_name) {
                    if let Some(d) = def.get("drawer") {
                        if d.get("transition-left-to-right").and_then(|v| v.as_bool()).unwrap_or(false) { ltr_sw.set_active(true); }
                    }
                }
                let cfg_l = Rc::clone(&config_rc); let mn_l = mod_name.clone(); let ref_l = Rc::clone(&refresh_rc);
                ltr_sw.connect_state_set(move |_, state| {
                    let mut c = cfg_l.borrow_mut();
                    if let Some(def) = c.module_definitions.get_mut(&mn_l).and_then(|d| d.as_object_mut()) {
                        if let Some(drawer) = def.get_mut("drawer").and_then(|d| d.as_object_mut()) {
                            drawer.insert("transition-left-to-right".to_string(), serde_json::json!(state));
                        }
                    }
                    drop(c); ref_l();
                    glib::Propagation::Proceed
                });
                ltr_row.add_suffix(&ltr_sw);
                group_cfg.add(&ltr_row);

                // Orientation
                let orient_row = ComboRow::new();
                orient_row.set_title("Orientation");
                let orients = vec!["inherit", "horizontal", "vertical"];
                let model = StringList::new(&orients);
                orient_row.set_model(Some(&model));
                
                if let Some(def) = config_borrow_orig.module_definitions.get(&mod_name) {
                    if let Some(v) = def.get("orientation").and_then(|v| v.as_str()) {
                        let idx = match v { "horizontal" => 1, "vertical" => 2, _ => 0 };
                        orient_row.set_selected(idx);
                    }
                }

                let cfg_o = Rc::clone(&config_rc); let mn_o = mod_name.clone(); let ref_o = Rc::clone(&refresh_rc);
                orient_row.connect_selected_notify(move |row| {
                    let mut c = cfg_o.borrow_mut();
                    if let Some(def) = c.module_definitions.get_mut(&mn_o) {
                        let val = match row.selected() {
                            1 => Some("horizontal"),
                            2 => Some("vertical"),
                            _ => None,
                        };
                        if let Some(v) = val { def.as_object_mut().unwrap().insert("orientation".to_string(), serde_json::json!(v)); }
                        else { def.as_object_mut().unwrap().remove("orientation"); }
                    }
                    drop(c); ref_o();
                });
                group_cfg.add(&orient_row);
                props_page.append(&group_cfg);
            }

            let anim_group = PreferencesGroup::new();
            anim_group.set_title("Animations and Effects");
            
            // --- Smooth Transitions ---
            let transition_row = ActionRow::new();
            transition_row.set_title("Smooth Transitions");
            let trans_active = get_module_css_prop(&layout_css_path, &mod_name, "", "transition").is_some();
            let trans_sw = Switch::builder().active(trans_active).valign(gtk::Align::Center).build();
            let lp_t = layout_css_path.clone(); let mn_t = mod_name.clone(); let upd_t = Rc::clone(&update_props_self);
            trans_sw.connect_state_set(move |_, state| {
                let val = if state { "all 0.3s ease" } else { "" };
                update_module_css(&lp_t, &mn_t, "", "transition", val);
                if let Some(f) = &*upd_t.borrow() { f(mn_t.clone()); }
                glib::Propagation::Proceed
            });
            transition_row.add_suffix(&trans_sw);
            anim_group.add(&transition_row);

            // --- Hover Effects ---
            let hover_row = ComboRow::new();
            hover_row.set_title("Hover Effect");
            let hover_effects = vec!["none", "glow", "lift", "bounce", "wobble", "shake", "blink"];
            let hover_model = StringList::new(&hover_effects);
            hover_row.set_model(Some(&hover_model));
            
            let current_hover = if get_module_css_prop(&layout_css_path, &mod_name, ":hover", "background-color").is_some() { 1 }
                               else if get_module_css_prop(&layout_css_path, &mod_name, ":hover", "margin-top").is_some() && get_module_css_prop(&layout_css_path, &mod_name, ":hover", "animation").is_none() { 2 }
                               else if let Some(a) = get_module_css_prop(&layout_css_path, &mod_name, ":hover", "animation") {
                                   if a.contains("bounce") { 3 } else if a.contains("wobble") { 4 } else if a.contains("shake") { 5 } else if a.contains("blink") { 6 } else { 0 }
                               }
                               else { 0 };
            hover_row.set_selected(current_hover);
            
            let lp_h = layout_css_path.clone(); let mn_h = mod_name.clone(); let upd_h = Rc::clone(&update_props_self);
            hover_row.connect_selected_notify(move |row| {
                let sel = row.selected();
                // Clear old hover effects
                update_module_css(&lp_h, &mn_h, ":hover", "background-color", "");
                update_module_css(&lp_h, &mn_h, ":hover", "margin-top", "");
                update_module_css(&lp_h, &mn_h, ":hover", "color", "");
                update_module_css(&lp_h, &mn_h, ":hover", "animation", "");
                
                match sel {
                    1 => { // Glow
                        update_module_css(&lp_h, &mn_h, ":hover", "background-color", "@hover_bg");
                        update_module_css(&lp_h, &mn_h, ":hover", "color", "@hover_fg");
                    }
                    2 => { // Lift (static)
                        update_module_css(&lp_h, &mn_h, ":hover", "margin-top", "-2px");
                    }
                    3 => { // Bounce
                        update_module_css(&lp_h, &mn_h, ":hover", "animation", "bounce 0.5s infinite");
                    }
                    4 => { // Wobble
                        update_module_css(&lp_h, &mn_h, ":hover", "animation", "wobble 0.4s infinite");
                    }
                    5 => { // Shake
                        update_module_css(&lp_h, &mn_h, ":hover", "animation", "shake 0.3s infinite");
                    }
                    6 => { // Blink
                        update_module_css(&lp_h, &mn_h, ":hover", "animation", "blink 1s infinite alternate");
                    }
                    _ => {}
                }
                if let Some(f) = &*upd_h.borrow() { f(mn_h.clone()); }
            });
            anim_group.add(&hover_row);

            // --- Constant Animation ---
            let const_row = ComboRow::new();
            const_row.set_title("Constant Animation");
            let const_effects = vec!["none", "blink", "pulse", "rainbow", "shiver"];
            let const_model = StringList::new(&const_effects);
            const_row.set_model(Some(&const_model));

            let current_const = if let Some(a) = get_module_css_prop(&layout_css_path, &mod_name, "", "animation") {
                if a.contains("pulse") { 2 } else if a.contains("rainbow") { 3 } else if a.contains("shiver") { 4 } else if a.contains("blink") { 1 } else { 0 }
            } else { 0 };
            const_row.set_selected(current_const);

            let lp_c = layout_css_path.clone(); let mn_c = mod_name.clone(); let upd_c = Rc::clone(&update_props_self);
            const_row.connect_selected_notify(move |row| {
                let sel = row.selected();
                update_module_css(&lp_c, &mn_c, "", "animation", "");
                match sel {
                    1 => { 
                        update_module_css(&lp_c, &mn_c, "", "animation", "blink 1s infinite alternate");
                        update_module_css(&lp_c, &mn_c, "", "transition", "none");
                    }
                    2 => { 
                        update_module_css(&lp_c, &mn_c, "", "animation", "glow_pulse 2s infinite");
                        update_module_css(&lp_c, &mn_c, "", "transition", "none");
                    }
                    3 => {
                        update_module_css(&lp_c, &mn_c, "", "animation", "rainbow 4s infinite linear");
                        update_module_css(&lp_c, &mn_c, "", "transition", "none");
                    }
                    4 => {
                        update_module_css(&lp_c, &mn_c, "", "animation", "shiver 0.2s infinite");
                        update_module_css(&lp_c, &mn_c, "", "transition", "none");
                    }
                    _ => {
                        update_module_css(&lp_c, &mn_c, "", "transition", "all 0.3s ease");
                    }
                }
                if let Some(f) = &*upd_c.borrow() { f(mn_c.clone()); }
            });
            anim_group.add(&const_row);

            props_page.append(&anim_group);

            // --- Module States & Thresholds (Battery, CPU, Memory, Temperature, Disk, Load) ---
            if ["battery", "cpu", "memory", "temperature", "disk", "load"].contains(&mod_name.as_str()) {
                let state_group = PreferencesGroup::new();
                state_group.set_title("Module States and Thresholds");
                
                let states_to_add = match mod_name.as_str() {
                    "battery" => vec![("Warning", "warning", 30.0), ("Critical", "critical", 15.0), ("Full", "full", 100.0)],
                    "temperature" => vec![("Critial", "critical", 80.0)],
                    "disk" => vec![("Warning", "warning", 80.0), ("Critical", "critical", 90.0)],
                    _ => vec![("Warning", "warning", 70.0), ("Critical", "critical", 90.0)],
                };

                for (label, key, default_val) in states_to_add {
                    let row = ActionRow::new();
                    row.set_title(label);
                    
                    // JSON threshold setup
                    let adj = gtk::Adjustment::new(default_val, 0.0, 100.0, 1.0, 10.0, 0.0);
                    let scale = Scale::new(gtk::Orientation::Horizontal, Some(&adj));
                    scale.set_width_request(150);
                    scale.set_draw_value(true);
                    
                    // Pre-load current JSON value
                    if let Some(def) = config_borrow_orig.module_definitions.get(&mod_name) {
                        if let Some(states) = def.get("states") {
                            if let Some(v) = states.get(key) {
                                if let Some(f) = v.as_f64() { scale.set_value(f); }
                            }
                        }
                    }

                    let config_s = Rc::clone(&config_rc); let mod_s = mod_name.clone(); let key_s = key.to_string();
                    let refresh_s = Rc::clone(&refresh_rc);
                    scale.connect_value_changed(move |s: &Scale| {
                        let val = s.value();
                        {
                            let mut cfg = config_s.borrow_mut();
                            if let Some(def) = cfg.module_definitions.get_mut(&mod_s) {
                                if def.get("states").is_none() {
                                    def.as_object_mut().unwrap().insert("states".to_string(), serde_json::json!({}));
                                }
                                def.get_mut("states").unwrap().as_object_mut().unwrap().insert(key_s.clone(), serde_json::json!(val));
                            }
                        }
                        refresh_s();
                    });
                    row.add_suffix(&scale);

                    // Animation picker for this state
                    let anim_combo = ComboRow::new();
                    let sub = format!("Triggered at {}%", label);
                    anim_combo.set_subtitle(&sub);
                    let effects = vec!["none", "blink", "pulse", "rainbow", "shiver", "shake"];
                    let model = StringList::new(&effects);
                    anim_combo.set_model(Some(&model));
                    
                    let suffix = format!(".{}", key);
                    let current = if let Some(a) = get_module_css_prop(&layout_css_path, &mod_name, &suffix, "animation") {
                        if a.contains("pulse") { 2 } else if a.contains("rainbow") { 3 } else if a.contains("shiver") { 4 } else if a.contains("shake") { 5 } else if a.contains("blink") { 1 } else { 0 }
                    } else { 0 };
                    anim_combo.set_selected(current);

                    let lp_s = layout_css_path.clone(); let mn_s = mod_name.clone(); let suf_s = suffix.clone();
                    let upd_s = Rc::clone(&update_props_self);
                    anim_combo.connect_selected_notify(move |row| {
                        let sel = row.selected();
                        update_module_css(&lp_s, &mn_s, &suf_s, "animation", "");
                        match sel {
                            1 => { update_module_css(&lp_s, &mn_s, &suf_s, "animation", "blink 1s infinite alternate"); }
                            2 => { update_module_css(&lp_s, &mn_s, &suf_s, "animation", "glow_pulse 2s infinite"); }
                            3 => { update_module_css(&lp_s, &mn_s, &suf_s, "animation", "rainbow 4s infinite linear"); }
                            4 => { update_module_css(&lp_s, &mn_s, &suf_s, "animation", "shiver 0.2s infinite"); }
                            5 => { update_module_css(&lp_s, &mn_s, &suf_s, "animation", "shake 0.3s infinite"); }
                            _ => {}
                        }
                        if let Some(f) = &*upd_s.borrow() { f(mn_s.clone()); }
                    });
                    
                    state_group.add(&row);
                    state_group.add(&anim_combo);
                }
                props_page.append(&state_group);
            }

            while let Some(child) = code_page.first_child() { code_page.remove(&child); }
            let json_label = Label::new(Some("JSON Module Definition"));
            json_label.set_halign(gtk::Align::Start); json_label.add_css_class("caption");
            code_page.append(&json_label);
            let json_view = TextView::builder().margin_top(8).margin_bottom(8).margin_start(8).margin_end(8).build();
            if let Some(def) = config_borrow_orig.module_definitions.get(&mod_name) { json_view.buffer().set_text(&serde_json::to_string_pretty(def).unwrap_or_default()); }
            code_page.append(&ScrolledWindow::builder().child(&json_view).height_request(200).build());
            let json_apply = Button::with_label("Apply JSON Changes");
            json_apply.add_css_class("pill");
            let mod_json = mod_name.clone(); let config_json = Rc::clone(&config_rc); let refresh_json = Rc::clone(&refresh_rc); let update_json = Rc::clone(&update_props_self); let toast_json = toast_p.clone();
            json_apply.connect_clicked(move |_| {
                let text = json_view.buffer().text(&json_view.buffer().start_iter(), &json_view.buffer().end_iter(), false).to_string();
                if let Ok(new_def) = serde_json::from_str::<serde_json::Value>(&text) {
                    config_json.borrow_mut().module_definitions.insert(mod_json.clone(), new_def);
                    refresh_json();
                    if let Some(f) = &*update_json.borrow() { f(mod_json.clone()); }
                    toast_json.add_toast(Toast::new("JSON Applied"));
                }
            });
            code_page.append(&json_apply);

            let css_label = Label::new(Some("Relevant CSS"));
            css_label.set_halign(gtk::Align::Start); css_label.add_css_class("caption");
            code_page.append(&css_label);
            let css_view = TextView::builder().margin_top(8).margin_bottom(8).margin_start(8).margin_end(8).build();
            let mut css_content = String::new();
            let full_css = fs::read_to_string(&layout_css_path).unwrap_or_else(|_| DEFAULT_LAYOUT_CSS.to_string());
            let id = format!("#{}", mod_name.replace("/", "-"));
            if let Some(start) = full_css.find(&id) {
                if let Some(end) = full_css[start..].find('}') { css_content = full_css[start..start + end + 1].to_string(); }
            }
            if css_content.is_empty() { css_content = format!("{} {{\n    background: @module_bg;\n}}", id); }
            
            css_view.buffer().set_text(&css_content);
            code_page.append(&ScrolledWindow::builder().child(&css_view).height_request(200).build());
            let css_apply = Button::with_label("Apply CSS Changes");
            css_apply.add_css_class("pill");
            let mod_css = mod_name.clone(); let layout_css_path_inner = layout_css_path.clone(); let toast_css = toast_p.clone();
            css_apply.connect_clicked(move |_| {
                let text = css_view.buffer().text(&css_view.buffer().start_iter(), &css_view.buffer().end_iter(), false).to_string();
                let full_css = fs::read_to_string(&layout_css_path_inner).unwrap_or_else(|_| DEFAULT_LAYOUT_CSS.to_string());
                let id = format!("#{}", mod_css.replace("/", "-"));
                let mut new_full = full_css.clone();
                if let Some(start) = full_css.find(&id) {
                    if let Some(end_rel) = full_css[start..].find('}') { new_full.replace_range(start..start + end_rel + 1, &text); }
                } else { new_full.push_str("\n\n"); new_full.push_str(&text); }
                let _ = fs::write(&layout_css_path_inner, new_full);
                toast_css.add_toast(Toast::new("CSS Applied to session"));
            });
            code_page.append(&css_apply);
            drop(config_borrow_orig);
        }
    }));

    // --- Style Editor ---
    let refresh_styles_fn = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
    {
        let style_rc = Rc::clone(&style_rc);
        let styles_page = styles_page.clone();
        let toast_styles = toast_overlay.clone();
        
        let refresh_styles = {
            let style_rc = Rc::clone(&style_rc);
            let styles_page = styles_page.clone();
            let toast_styles = toast_styles.clone();
            let style_rc = Rc::clone(&style_rc);
            let styles_page = styles_page.clone();
            let toast_styles = toast_styles.clone();
            let refresh_self = Rc::clone(&refresh_styles_fn);
            let layout_css_path = layout_css_path.clone();
            
            move || {
                while let Some(child) = styles_page.first_child() { styles_page.remove(&child); }
                let title = Label::new(Some("Visual Style Editor"));
                title.add_css_class("title-3");
                styles_page.append(&title);

                // --- Base Layout Selector ---
                let layout_group = PreferencesGroup::new();
                layout_group.set_title("Base Layout");
                
                // Find presets/layouts directory
                let exe_path = std::env::current_exe().unwrap_or_default();
                let exe_dir = exe_path.parent().unwrap_or(Path::new("."));
                let mut layouts_path = PathBuf::from("presets/layouts");
                if !layouts_path.exists() {
                    layouts_path = exe_dir.join("presets/layouts");
                    if !layouts_path.exists() {
                        layouts_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("presets/layouts");
                    }
                    if !layouts_path.exists() {
                        let home = std::env::var("HOME").unwrap_or_default();
                        layouts_path = PathBuf::from(home).join(".local/share/waybarconf/presets/layouts");
                    }
                }

                if layouts_path.exists() {
                    let mut layouts = Vec::new();
                    if let Ok(entries) = fs::read_dir(&layouts_path) {
                        for entry in entries.flatten() {
                            if let Ok(ft) = entry.file_type() {
                                if ft.is_file() {
                                    if let Some(name) = entry.file_name().to_str() {
                                        if name.ends_with(".css") {
                                            layouts.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    layouts.sort();

                    if !layouts.is_empty() {
                        let layout_row = ComboRow::new();
                        layout_row.set_title("Layout Template");
                        let model = StringList::new(layouts.iter().map(|s| s.as_str()).collect::<Vec<&str>>().as_slice());
                        layout_row.set_model(Some(&model));

                        // Determine current selection
                        let current_css = fs::read_to_string(&layout_css_path).unwrap_or_default();
                        let mut current_idx = 0;
                        for (i, name) in layouts.iter().enumerate() {
                            if current_css.contains(&format!("layouts/{}", name)) {
                                current_idx = i as u32;
                                break;
                            }
                        }
                        layout_row.set_selected(current_idx);

                        let lp = layout_css_path.clone();
                        let toast_l = toast_styles.clone();
                        let layouts_c = layouts.clone();
                        layout_row.connect_selected_notify(move |row| {
                            let idx = row.selected() as usize;
                            if idx < layouts_c.len() {
                                let new_layout = &layouts_c[idx];
                                let mut css = fs::read_to_string(&lp).unwrap_or_default();
                                
                                // Replace existing import or add new one
                                let import_str = format!("@import \"layouts/{}\";", new_layout);
                                let re = regex::Regex::new(r#"@import\s+"layouts/[^"]+";"#).unwrap();
                                
                                if re.is_match(&css) {
                                    css = re.replace(&css, import_str.as_str()).to_string();
                                } else {
                                    // Insert at a reasonable place (after color import or at top)
                                    if let Some(pos) = css.find("@import \"colors/wallpaper.css\";") {
                                        css.insert_str(pos + 31, &format!("\n\n/** Imports a layout (style) for bar **/\n{}\n", import_str));
                                    } else {
                                        css.insert_str(0, &format!("{}\n", import_str));
                                    }
                                }
                                
                                if let Ok(_) = fs::write(&lp, css) {
                                    toast_l.add_toast(Toast::new(&format!("Switched to {}", new_layout)));
                                }
                            }
                        });
                        layout_group.add(&layout_row);
                    }
                }
                styles_page.append(&layout_group);

                // --- Color Preset Selector ---
                let color_preset_group = PreferencesGroup::new();
                color_preset_group.set_title("Color Presets");
                
                // Find presets/colors directory
                let mut colors_path = PathBuf::from("presets/colors");
                if !colors_path.exists() {
                    colors_path = exe_dir.join("presets/colors");
                    if !colors_path.exists() {
                        colors_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("presets/colors");
                    }
                    if !colors_path.exists() {
                        let home = std::env::var("HOME").unwrap_or_default();
                        colors_path = PathBuf::from(home).join(".local/share/waybarconf/presets/colors");
                    }
                }

                if colors_path.exists() {
                    let mut preset_colors = Vec::new();
                    if let Ok(entries) = fs::read_dir(&colors_path) {
                        for entry in entries.flatten() {
                            if let Ok(ft) = entry.file_type() {
                                if ft.is_file() {
                                    if let Some(name) = entry.file_name().to_str() {
                                        if name.ends_with(".css") {
                                            preset_colors.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    preset_colors.sort();

                    if !preset_colors.is_empty() {
                        let cf_row = ComboRow::new();
                        cf_row.set_title("Color Theme");
                        let model = StringList::new(preset_colors.iter().map(|s| s.as_str()).collect::<Vec<&str>>().as_slice());
                        cf_row.set_model(Some(&model));
                        
                        // We don't verify current selection against file content because vars are parsed values. 
                        // Just set to -1 or 0. Since it's a dropdown, 0 is fine, or we could add a "Custom" entry.
                        // For simplicity, we just won't try to sync the dropdown to current state if it doesn't match a filename.

                        let cp_p = colors_path.clone();
                        let style_cp = Rc::clone(&style_rc);
                        let refresh_cp = Rc::clone(&refresh_self);
                        let toast_cp = toast_styles.clone();
                        let presets_c = preset_colors.clone();
                        
                        cf_row.connect_selected_notify(move |row| {
                            let idx = row.selected() as usize;
                            if idx < presets_c.len() {
                                let filename = &presets_c[idx];
                                let target_file = cp_p.join(filename);
                                if let Ok(content) = fs::read_to_string(&target_file) {
                                    let new_vars = parse_style_vars(&content);
                                    if !new_vars.is_empty() {
                                        {
                                            let mut s = style_cp.borrow_mut();
                                            // Merge or Replace? Replaced as per "Load Preset" logic usually.
                                            // The user said "use EITHER mutagen OR one of the css files", implying full replacement of the scheme.
                                            // But let's keep unknown vars? No, themes usually define the whole palette.
                                            // However, `parse_style_vars` only returns what it finds.
                                            // Let's iterate and update.
                                            for (k, v) in new_vars {
                                                s.vars.insert(k, v);
                                            }
                                        }
                                        let _ = style_cp.borrow().save();
                                        if let Some(f) = &*refresh_cp.borrow() { f(); }
                                        toast_cp.add_toast(Toast::new(&format!("Applied {} Theme", filename)));
                                    }
                                }
                            }
                        });
                        color_preset_group.add(&cf_row);
                    }
                }
                styles_page.append(&color_preset_group);

                let auto_group = PreferencesGroup::new();
                auto_group.set_title("Auto-Color from Wallpaper");
                let scheme_row = ComboRow::new();
                scheme_row.set_title("Scheme Type");
                let schemes = vec!["scheme-vibrant", "scheme-expressive", "scheme-fruit-salad", "scheme-rainbow", "scheme-monochrome", "scheme-neutral", "scheme-tonal-spot", "scheme-content", "scheme-fidelity"];
                let model = StringList::new(&schemes);
                scheme_row.set_model(Some(&model));
                auto_group.add(&scheme_row);

                let extract_btn = Button::with_label("Extract Colors");
                extract_btn.add_css_class("suggested-action");
                extract_btn.set_margin_top(6);
                let style_extract = Rc::clone(&style_rc);
                let toast_extract = toast_styles.clone();
                let refresh_extract = Rc::clone(&refresh_self);
                extract_btn.connect_clicked(move |_| {
                    if let Some(wp) = detect_wallpaper() {
                        let stype = schemes[scheme_row.selected() as usize];
                        match apply_matugen(&wp, stype, Rc::clone(&style_extract)) {
                            Ok(_) => {
                                toast_extract.add_toast(Toast::new("Colors Applied from Wallpaper"));
                                if let Some(f) = &*refresh_extract.borrow() { f(); }
                            }
                            Err(e) => {
                                let escaped = glib::markup_escape_text(&e);
                                toast_extract.add_toast(Toast::new(&format!("Error: {}", escaped)));
                            }
                        }
                    } else {
                        toast_extract.add_toast(Toast::new("Could not target wallpaper"));
                    }
                });
                auto_group.add(&extract_btn);
                styles_page.append(&auto_group);
                
                let color_group = PreferencesGroup::new();
                color_group.set_title("Color Scheme");
                let vars: Vec<(String, String)> = style_rc.borrow().vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                for (name, value) in vars {
                    if value.starts_with("#") {
                        let row = ActionRow::new();
                        row.set_title(&name.replace("_", " ").to_uppercase());
                        let color_btn = ColorButton::new();
                        if let Ok(rgba) = gdk::RGBA::parse(&value) { color_btn.set_rgba(&rgba); }
                        let style_inner = Rc::clone(&style_rc);
                        let name_inner = name.clone();
                        color_btn.connect_color_set(move |btn| {
                            let rgba = btn.rgba();
                            let hex = format!("#{:02x}{:02x}{:02x}", (rgba.red() * 255.0) as u8, (rgba.green() * 255.0) as u8, (rgba.blue() * 255.0) as u8);
                            style_inner.borrow_mut().vars.insert(name_inner.clone(), hex);
                            let _ = style_inner.borrow().save();
                        });
                        row.add_suffix(&color_btn);
                        color_group.add(&row);
                    }
                }
                styles_page.append(&color_group);
                
                let metrics_group = PreferencesGroup::new();
                metrics_group.set_title("Layout Metrics");
                let m_vars: Vec<(String, String)> = style_rc.borrow().vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                for (name, value) in m_vars {
                    if name.contains("padding") || name.contains("margin") || name.contains("spacing") {
                        let row = ActionRow::new();
                        row.set_title(&name.replace("_", " ").to_uppercase());
                        let initial_val = value.chars().filter(|c| c.is_digit(10)).collect::<String>().parse::<f64>().unwrap_or(0.0);
                        let scale = gtk::Scale::with_range(Orientation::Horizontal, 0.0, 50.0, 1.0);
                        scale.set_value(initial_val);
                        scale.set_width_request(150);
                        let name_inner = name.clone();
                        let style_inner = Rc::clone(&style_rc);
                        scale.connect_value_changed(move |s| {
                            let val = s.value() as i32;
                            style_inner.borrow_mut().vars.insert(name_inner.clone(), format!("{}px", val));
                            let _ = style_inner.borrow().save();
                        });
                        row.add_suffix(&scale);
                        metrics_group.add(&row);
                    }
                }
                styles_page.append(&metrics_group);
            }
        };
        *refresh_styles_fn.borrow_mut() = Some(Box::new(refresh_styles.clone()));
        refresh_styles();
    }

    // --- Header Actions ---
    let win_rc: Rc<RefCell<Option<ApplicationWindow>>> = Rc::new(RefCell::new(None));
    let t_overlay = toast_overlay.clone();

    save_profile_btn.connect_clicked({
        let config_rc = Rc::clone(&config_rc);
        let style_rc = Rc::clone(&style_rc);
        let layout_css_path = layout_css_path.clone();
        let win_rc = Rc::clone(&win_rc);
        let t_save = t_overlay.clone();
        move |_| {
            let filter = FileFilter::new();
            filter.add_pattern("*.wc"); filter.set_name(Some("Waybar Config Profile (*.wc)"));
            let dialog = FileDialog::builder().title("Save Profile").default_filter(&filter).build();
            
            let config_rc = Rc::clone(&config_rc);
            let style_rc = Rc::clone(&style_rc);
            let layout_css_path = layout_css_path.clone();
            let t_s = t_save.clone();
            
            if let Some(win) = win_rc.borrow().as_ref() {
                dialog.save(Some(win), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            let mut path = path.to_path_buf();
                            if path.extension().and_then(|s| s.to_str()) != Some("wc") { path.set_extension("wc"); }
                            
                            let profile = WaybarProfile {
                                config: config_rc.borrow().clone(),
                                style_vars: style_rc.borrow().vars.clone(),
                                layout_css: fs::read_to_string(&layout_css_path).unwrap_or_else(|_| DEFAULT_LAYOUT_CSS.to_string()),
                            };
                            
                            if let Ok(_) = profile.save_to_file(path.to_str().unwrap()) {
                                t_s.add_toast(Toast::new("Profile Saved"));
                            }
                        }
                    }
                });
            }
        }
    });

    load_profile_btn.connect_clicked({
        let config_rc = Rc::clone(&config_rc);
        let style_rc = Rc::clone(&style_rc);
        let layout_css_path = layout_css_path.clone();
        let win_rc = Rc::clone(&win_rc);
        let refresh_rc = Rc::clone(&refresh_rc);
        let refresh_styles_fn = Rc::clone(&refresh_styles_fn);
        let t_load = t_overlay.clone();
        
        move |_| {
            let filter = FileFilter::new(); filter.add_pattern("*.wc");
            let dialog = FileDialog::builder().title("Load Profile").default_filter(&filter).build();
            
            let config_rc = Rc::clone(&config_rc);
            let style_rc = Rc::clone(&style_rc);
            let layout_css_path = layout_css_path.clone();
            let refresh_rc = Rc::clone(&refresh_rc);
            let refresh_styles_fn = Rc::clone(&refresh_styles_fn);
            let t_l = t_load.clone();
            
            if let Some(win) = win_rc.borrow().as_ref() {
                dialog.open(Some(win), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            if let Ok(profile) = WaybarProfile::from_file(path.to_str().unwrap()) {
                                *config_rc.borrow_mut() = profile.config;
                                style_rc.borrow_mut().vars = profile.style_vars;
                                let _ = fs::write(&layout_css_path, profile.layout_css);
                                
                                refresh_rc();
                                if let Some(f) = &*refresh_styles_fn.borrow() { f(); }
                                t_l.add_toast(Toast::new("Profile Loaded"));
                            }
                        }
                    }
                });
            }
        }
    });



    apply_btn.connect_clicked({
        let config_rc = Rc::clone(&config_rc);
        let style_rc = Rc::clone(&style_rc);
        let layout_css_path_apply = layout_css_path.clone();
        let t_apply = t_overlay.clone();
        move |_| {
            let home = std::env::var("HOME").unwrap_or_default();
            let waybar_cfg_dir = PathBuf::from(home).join(".config/waybar");
            let _ = fs::create_dir_all(&waybar_cfg_dir);
            let target_cfg = waybar_cfg_dir.join("config.jsonc");
            let target_style = waybar_cfg_dir.join("colors/wallpaper.css");
            let target_layout = waybar_cfg_dir.join("style.css");
            let _ = fs::create_dir_all(waybar_cfg_dir.join("colors"));
            
            // Sync layout templates to ~/.config/waybar/layouts
            let layouts_src = PathBuf::from("presets/layouts");
            let layouts_dst = waybar_cfg_dir.join("layouts");
            let _ = fs::create_dir_all(&layouts_dst);
            if let Ok(entries) = fs::read_dir(layouts_src) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(name) = path.file_name() {
                            let _ = fs::copy(&path, layouts_dst.join(name));
                        }
                    }
                }
            }

            let _ = config_rc.borrow().save_to_file(target_cfg.to_str().unwrap());
            let _ = style_rc.borrow().save_to(&target_style);
            
            // Persist session CSS to the real Waybar path
            if let Ok(css) = fs::read_to_string(&layout_css_path_apply) {
                let _ = fs::write(&target_layout, css);
            } else {
                let _ = fs::write(&target_layout, DEFAULT_LAYOUT_CSS);
            }

            let _ = Command::new("pkill").args(["-x", "waybar"]).status();
            let _ = Command::new("waybar").spawn();
            let escaped = glib::markup_escape_text("Applied & Restored Styles");
            t_apply.add_toast(Toast::new(&escaped));
        }
    });

    // --- Drag & Drop ---
    let setup_drop = |list: &ListBox, col_id: &str, config_rc: Rc<RefCell<WaybarConfig>>, refresh: Rc<dyn Fn()>| {
        let dt = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
        let cid = col_id.to_string(); let list_c = list.clone();
        dt.connect_drop(move |_, val, _, y| {
            let data = val.get::<String>().unwrap_or_default();
            let mut parts = data.splitn(2, ':');
            let (_scol, mname) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            if mname.is_empty() { return false; }
            let mut cfg = config_rc.borrow_mut();
            let item = if let Some(it) = remove_module_anywhere(&mut cfg, mname) { it } else { return false };
            
            let mapping = get_flat_mapping(&cfg, &cid);
            let mut target_idx = mapping.len();
            if let Some(r) = list_c.row_at_y(y as i32) { target_idx = r.index() as usize; }

            if target_idx < mapping.len() {
                let (parent, relative_to) = mapping[target_idx].clone();
                if relative_to.starts_with("group/") {
                    // Drop ON group: nest at start
                    let def = cfg.module_definitions.get_mut(&relative_to).unwrap();
                    if def.get("modules").is_none() { def.as_object_mut().unwrap().insert("modules".to_string(), serde_json::json!([])); }
                    def.get_mut("modules").unwrap().as_array_mut().unwrap().insert(0, serde_json::json!(item));
                } else if let Some(p) = parent {
                    // Drop BEFORE sibling in group
                    let def = cfg.module_definitions.get_mut(&p).unwrap();
                    let mods = def.get_mut("modules").unwrap().as_array_mut().unwrap();
                    let p_idx = mods.iter().position(|v| v.as_str() == Some(&relative_to)).unwrap();
                    mods.insert(p_idx, serde_json::json!(item));
                } else {
                    // Drop BEFORE sibling at top level
                    let slist = match cid.as_str() { "left" => &mut cfg.modules_left, "center" => &mut cfg.modules_center, "right" => &mut cfg.modules_right, _ => return false };
                    let p_idx = slist.iter().position(|m| m == &relative_to).unwrap();
                    slist.insert(p_idx, item);
                }
            } else {
                // Append to top level
                let slist = match cid.as_str() { "left" => &mut cfg.modules_left, "center" => &mut cfg.modules_center, "right" => &mut cfg.modules_right, _ => return false };
                slist.push(item);
            }
            drop(cfg); refresh(); true
        });
        list.add_controller(dt);
    };
    setup_drop(&left_list, "left", Rc::clone(&config_rc), Rc::clone(&refresh_rc));
    setup_drop(&center_list, "center", Rc::clone(&config_rc), Rc::clone(&refresh_rc));
    setup_drop(&right_list, "right", Rc::clone(&config_rc), Rc::clone(&refresh_rc));

    refresh_ui();

    let build_col = |name: &str, list: &ListBox, col_id: &str, config_rc: Rc<RefCell<WaybarConfig>>, refresh_rc: Rc<dyn Fn()>, options: &[&str], sel_state: Rc<RefCell<Option<(String, String)>>>| {
        let b = GtkBox::new(Orientation::Vertical, 6);
        let h = GtkBox::new(Orientation::Horizontal, 6);
        let l = Label::new(Some(name)); l.add_css_class("title-4"); h.append(&l);
        
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        h.append(&spacer);

        let add_btn = Button::builder().icon_name("list-add-symbolic").has_frame(false).tooltip_text("Add Module").build();
        let group_btn = Button::builder().icon_name("folder-new-symbolic").has_frame(false).tooltip_text("Create Group").build();
        h.append(&group_btn);
        h.append(&add_btn);

        let cfg_g = Rc::clone(&config_rc); let ref_g = Rc::clone(&refresh_rc); let cid_g = col_id.to_string();
        group_btn.connect_clicked(move |_| {
            let dialog = MessageDialog::builder().heading("Create Group").body("Enter a name for the new group").build();
            let entry = Entry::builder().placeholder_text("hardware, stats, etc.").build();
            dialog.set_extra_child(Some(&entry));
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("create", "Create");
            dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
            let cfg = Rc::clone(&cfg_g); let ref_r = Rc::clone(&ref_g); let cid = cid_g.clone();
            dialog.connect_response(None, move |d, response| {
                if response == "create" {
                    let mut name = entry.text().to_string();
                    if !name.is_empty() {
                        if !name.starts_with("group/") { name = format!("group/{}", name); }
                        let mut c = cfg.borrow_mut();
                        match cid.as_str() { "left" => c.modules_left.push(name.clone()), "center" => c.modules_center.push(name.clone()), "right" => c.modules_right.push(name.clone()), _ => {} }
                        c.module_definitions.insert(name.clone(), serde_json::json!({ "modules": [] }));
                        drop(c); ref_r();
                    }
                }
                d.close();
            });
            dialog.present();
        });
        let popover = gtk::Popover::new(); popover.set_width_request(250);
        let pop_list = ListBox::new();
        let search = SearchEntry::builder().margin_top(6).margin_bottom(6).margin_start(6).margin_end(6).build();
        let pop_content = GtkBox::new(Orientation::Vertical, 0);
        pop_content.append(&search);
        
        for opt in options {
            let r = ActionRow::builder().title(*opt).activatable(true).build();
            let opt_s = opt.to_string(); let cfg_pop = Rc::clone(&config_rc); let ref_pop = Rc::clone(&refresh_rc); let cid_pop = col_id.to_string(); let p_close = popover.clone();
            r.connect_activated(move |_| {
                let mut cfg = cfg_pop.borrow_mut();
                match cid_pop.as_str() { "left" => cfg.modules_left.push(opt_s.clone()), "center" => cfg.modules_center.push(opt_s.clone()), "right" => cfg.modules_right.push(opt_s.clone()), _ => {} }
                if !cfg.module_definitions.contains_key(&opt_s) {
                    let default_props = match opt_s.as_str() {
                        "temperature" => serde_json::json!({ "format": "{temperatureC}°C {icon}", "format-icons": ["", "", "", "", ""] }),
                        "disk" => serde_json::json!({ "format": "{percentage_used}% 󰋊", "path": "/" }),
                        "bluetooth" => serde_json::json!({ "format": " {status}", "format-connected": " {device_alias}", "format-connected-battery": " {device_alias} {device_battery_percentage}%" }),
                        "network" => serde_json::json!({ "format-wifi": " {essid} ({signalStrength}%)", "format-ethernet": "󰈀 {ifname}", "format-disconnected": "󰤮 Disconnected" }),
                        "wireplumber" => serde_json::json!({ "format": "{volume}% {icon}", "format-muted": "󰝟", "format-icons": ["", "", ""] }),
                        "pulseaudio" => serde_json::json!({ "format": "{volume}% {icon}", "format-muted": "󰝟", "format-icons": { "default": ["", "", ""] } }),
                        "battery" => serde_json::json!({ "format": "{capacity}% {icon}", "format-icons": ["", "", "", "", ""] }),
                        "hyprland/workspaces" | "sway/workspaces" => serde_json::json!({ "format": "{name}" }),
                        "mpris" => serde_json::json!({ "format": "{player_icon} {title}", "player-icons": { "default": "" } }),
                        "clock" => serde_json::json!({ "format": "{:%I:%M %p}", "tooltip-format": "<big>{:%Y %B}</big>\n<tt><small>{calendar}</small></tt>" }),
                        "cpu" => serde_json::json!({ "format": "CPU {usage}%" }),
                        "memory" => serde_json::json!({ "format": "MEM {percentage}%" }),
                        "backlight/slider" => serde_json::json!({ "min": 0, "max": 100, "orientation": "horizontal" }),
                        "pulseaudio/slider" => serde_json::json!({ "min": 0, "max": 140, "orientation": "horizontal" }),
                        "power-profiles-daemon" => serde_json::json!({ "format": "{icon}", "format-icons": {"default": "", "performance": "", "balanced": "", "power-saver": ""} }),
                        "privacy" => serde_json::json!({ "icon-spacing": 4, "icon-size": 18, "transition-duration": 250, "modules": [{"type": "screenshare"}, {"type": "audio-out"}, {"type": "audio-in"}] }),
                        "systemd-failed-units" => serde_json::json!({ "hide-on-ok": true, "format": "✗ {nr_failed}", "format-ok": "✓" }),
                        "jack" => serde_json::json!({ "format": "DSP {}%", "format-xrun": "{xruns} xruns", "interval": 5 }),
                        "load" => serde_json::json!({ "interval": 10, "format": "Load {load1}" }),
                        "user" => serde_json::json!({ "format": "{user}", "interval": 60 }),
                        "dwl/tags" | "river/tags" => serde_json::json!({ "num-tags": 9 }),
                        "niri/workspaces" => serde_json::json!({ "format": "{icon}" }),
                        "cffi" => serde_json::json!({ "module_path": "/path/to/lib.so" }),
                        "sndio" => serde_json::json!({ "format": "󰓃 {volume}%" }),
                        _ => serde_json::json!({}),
                    };
                    cfg.module_definitions.insert(opt_s.clone(), default_props);
                }
                drop(cfg); ref_pop(); p_close.popdown();
            });
            pop_list.append(&r);
        }

        let search_filter = search.clone();
        pop_list.set_filter_func(move |row| {
            let row = row.downcast_ref::<ActionRow>().unwrap();
            let text = search_filter.text().to_string().to_lowercase();
            if text.is_empty() { return true; }
            row.title().to_string().to_lowercase().contains(&text)
        });

        let pop_list_invalidate = pop_list.clone();
        search.connect_search_changed(move |_| { pop_list_invalidate.invalidate_filter(); });

        pop_content.append(&ScrolledWindow::builder().child(&pop_list).max_content_height(300).propagate_natural_height(true).build());
        popover.set_child(Some(&pop_content));
        let p_popup = popover.clone(); add_btn.connect_clicked(move |_| p_popup.popup()); popover.set_parent(&add_btn);
        let del_btn = Button::builder().icon_name("user-trash-symbolic").has_frame(false).build();
        let cfg_del = Rc::clone(&config_rc); let ref_del = Rc::clone(&refresh_rc); let cid_del = col_id.to_string(); let sel_del = Rc::clone(&sel_state);
        del_btn.connect_clicked(move |_| {
            let selection = sel_del.borrow().clone();
            if let Some((col, mod_name)) = selection {
                if col == cid_del {
                    let mut cfg = cfg_del.borrow_mut();
                    let list = match col.as_str() { "left" => &mut cfg.modules_left, "center" => &mut cfg.modules_center, "right" => &mut cfg.modules_right, _ => return };
                    if let Some(pos) = list.iter().position(|m| m == &mod_name) { list.remove(pos); }
                    drop(cfg);
                    ref_del();
                    *sel_del.borrow_mut() = None;
                }
            }
        });
        h.append(&del_btn); b.append(&h); b.append(&ScrolledWindow::builder().child(list).vexpand(true).build());
        b
    };

    let col_opts: Vec<&str> = module_options.iter().map(|s| *s).collect();
    columns_box.append(&build_col("Left", &left_list, "left", Rc::clone(&config_rc), Rc::clone(&refresh_rc), &col_opts, Rc::clone(&selected_module_state)));
    columns_box.append(&build_col("Center", &center_list, "center", Rc::clone(&config_rc), Rc::clone(&refresh_rc), &col_opts, Rc::clone(&selected_module_state)));
    columns_box.append(&build_col("Right", &right_list, "right", Rc::clone(&config_rc), Rc::clone(&refresh_rc), &col_opts, Rc::clone(&selected_module_state)));

    paned.set_start_child(Some(&columns_box));
    paned.set_end_child(Some(&settings_panel));
    toast_overlay.set_child(Some(&paned));
    main_box.append(&toast_overlay);

    let win = ApplicationWindow::builder().application(app).title("WaybarConf").default_width(1200).default_height(800).content(&main_box).build();
    *win_rc.borrow_mut() = Some(win.clone());

    // --- Startup Check ---
    if let Some(local_path) = get_waybar_config_path() {
        let dialog = MessageDialog::builder()
            .transient_for(&win)
            .heading("Welcome to WaybarConf")
            .body("We found an existing Waybar configuration on your system. Would you like to load it, or start with a template?")
            .build();
        
        dialog.add_response("load", "Load Local Config");
        dialog.add_response("template", "Use Template");
        dialog.add_response("blank", "Start Blank");
        
        dialog.set_response_appearance("load", adw::ResponseAppearance::Suggested);
        
        let config_rc_startup = Rc::clone(&config_rc);
        let style_rc_startup = Rc::clone(&style_rc);
        let layout_css_path_startup = layout_css_path.clone();
        let refresh_ui_startup = Rc::clone(&refresh_rc);
        let refresh_styles_startup = Rc::clone(&refresh_styles_fn);
        
        dialog.connect_response(None, move |d, response| {
            match response {
                "load" => {
                    if let Ok(new_cfg) = WaybarConfig::from_file(&local_path) {
                        *config_rc_startup.borrow_mut() = new_cfg;
                        
                        let home = std::env::var("HOME").unwrap_or_default();
                        let local_style_path = PathBuf::from(home.clone()).join(".config/waybar/colors/wallpaper.css");
                        if local_style_path.exists() {
                            *style_rc_startup.borrow_mut() = StyleConfig::from_file(&local_style_path);
                        }
                        
                        let local_layout_path = PathBuf::from(home).join(".config/waybar/style.css");
                        if local_layout_path.exists() {
                            if let Ok(css) = fs::read_to_string(&local_layout_path) {
                                let _ = fs::write(&layout_css_path_startup, css);
                            }
                        }
                        
                        refresh_ui_startup();
                        if let Some(f) = &*refresh_styles_startup.borrow() { f(); }
                    }
                }
                "template" => {
                    *config_rc_startup.borrow_mut() = serde_json::from_str(DEFAULT_CONFIG_JSON).unwrap();
                    style_rc_startup.borrow_mut().vars = parse_style_vars(DEFAULT_STYLE_VARS);
                    let _ = fs::write(&layout_css_path_startup, DEFAULT_LAYOUT_CSS);
                    refresh_ui_startup();
                    if let Some(f) = &*refresh_styles_startup.borrow() { f(); }
                }
                "blank" => {
                    *config_rc_startup.borrow_mut() = WaybarConfig {
                        modules_left: vec![],
                        modules_center: vec![],
                        modules_right: vec![],
                        module_definitions: indexmap::IndexMap::new(),
                    };
                    style_rc_startup.borrow_mut().vars = indexmap::IndexMap::new();
                    let _ = fs::write(&layout_css_path_startup, "");
                    refresh_ui_startup();
                    if let Some(f) = &*refresh_styles_startup.borrow() { f(); }
                }
                _ => {}
            }
            d.close();
        });
        dialog.present();
    }

    win.present();
}

fn create_module_row(n: &str, depth: u32) -> ActionRow {
    let is_group = n.starts_with("group/");
    let icon = if is_group { "folder-symbolic" } 
               else if n.starts_with("custom/") { "applications-system-symbolic" } 
               else { match n { 
                   "clock" => "x-office-calendar-symbolic", 
                   "battery" => "battery-full-symbolic", 
                   "cpu" => "chip-symbolic", 
                   "memory" => "ram-symbolic", 
                   "network" => "network-wireless-symbolic", 
                   "pulseaudio" => "audio-volume-high-symbolic", 
                   "backlight" => "display-brightness-symbolic", 
                   "tray" => "panel-show-symbolic",
                   "bluetooth" => "bluetooth-active-symbolic",
                   "disk" => "drive-harddisk-symbolic",
                   "mpd" => "audio-x-generic-symbolic",
                   "mpris" => "media-playback-start-symbolic",
                   "hyprland/workspaces" | "sway/workspaces" | "wlr/workspaces" | "niri/workspaces" | "river/tags" | "dwl/tags" => "view-grid-symbolic",
                   "hyprland/window" | "sway/window" | "wlr/window" | "niri/window" | "river/window" | "dwl/window" => "window-new-symbolic",
                   "temperature" => "sensors-temperature-symbolic",
                   "upower" => "battery-full-symbolic",
                   "wireplumber" => "audio-card-symbolic",
                   "image" => "image-x-generic-symbolic",
                   "gamemode" => "input-gaming-symbolic",
                   "keyboard-state" => "input-keyboard-symbolic",
                   "idle_inhibitor" => "eye-not-looking-symbolic",
                   "backlight/slider" => "display-brightness-symbolic",
                   "pulseaudio/slider" | "sndio" => "audio-volume-high-symbolic",
                   "power-profiles-daemon" => "power-profile-balanced-symbolic",
                   "privacy" => "security-high-symbolic",
                   "load" => "chip-symbolic",
                   "river/layout" => "view-list-symbolic",
                   "systemd-failed-units" => "software-update-available-symbolic",
                   "user" => "avatar-default-symbolic",
                   _ => "extension-symbolic" 
               } };
    let image = gtk::Image::from_icon_name(icon);
    let row = ActionRow::builder().title(n).activatable(true).build();
    row.add_prefix(&image);
    if depth > 0 {
        row.set_margin_start((depth * 20) as i32);
        row.add_css_class("nested-row");
    }
    if is_group {
        row.add_css_class("group-row");
    }
    row
}

fn remove_module_anywhere(cfg: &mut WaybarConfig, name: &str) -> Option<String> {
    let mut found = None;
    if let Some(p) = cfg.modules_left.iter().position(|m| m == name) { found = Some(cfg.modules_left.remove(p)); }
    else if let Some(p) = cfg.modules_center.iter().position(|m| m == name) { found = Some(cfg.modules_center.remove(p)); }
    else if let Some(p) = cfg.modules_right.iter().position(|m| m == name) { found = Some(cfg.modules_right.remove(p)); }
    else {
        for def in cfg.module_definitions.values_mut() {
            if let Some(mods) = def.get_mut("modules").and_then(|m| m.as_array_mut()) {
                if let Some(p) = mods.iter().position(|v| v.as_str() == Some(name)) {
                    let removed = mods.remove(p);
                    found = Some(removed.as_str().unwrap().to_string());
                    break;
                }
            }
        }
    }
    found
}

fn get_flat_mapping(cfg: &WaybarConfig, col: &str) -> Vec<(Option<String>, String)> {
    let mut mapping = Vec::new();
    let root = match col { "left" => &cfg.modules_left, "center" => &cfg.modules_center, "right" => &cfg.modules_right, _ => return mapping };
    
    fn walk(cfg: &WaybarConfig, modules: &[String], parent: Option<String>, mapping: &mut Vec<(Option<String>, String)>) {
        for m in modules {
            mapping.push((parent.clone(), m.clone()));
            if m.starts_with("group/") {
                if let Some(def) = cfg.module_definitions.get(m) {
                    if let Some(children) = def.get("modules").and_then(|v| v.as_array()) {
                        let child_names: Vec<String> = children.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                        walk(cfg, &child_names, Some(m.clone()), mapping);
                    }
                }
            }
        }
    }
    walk(cfg, root, None, &mut mapping);
    mapping
}
