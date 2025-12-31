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
use gtk::{Box as GtkBox, ListBox, Orientation, Label, ScrolledWindow, TextView, Entry, Switch, Button, ColorButton, FileDialog, FileFilter, StringList, SearchEntry};
use crate::config::WaybarConfig;

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

* {
    --padding_top: 4px;
    --padding_bottom: 4px;
    --margin_left: 4px;
    --margin_right: 4px;
    --spacing: 8px;
}
"#;

const DEFAULT_LAYOUT_CSS: &str = r#"/* WaybarConf Layout CSS */
#clock, #cpu, #memory, #pulseaudio, #network, #tray, #custom-launcher {
    padding: var(--padding_top) var(--margin_right) var(--padding_bottom) var(--margin_left);
    margin: 0 var(--spacing);
    background: @module_bg;
    color: @module_fg;
    border-radius: 8px;
}
"#;

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

fn build_ui(app: &Application) {
    let waybar_config: WaybarConfig = serde_json::from_str(DEFAULT_CONFIG_JSON).unwrap();
    let config_rc = Rc::new(RefCell::new(waybar_config));
    
    let style_vars = parse_style_vars(DEFAULT_STYLE_VARS);
    let default_style_path = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/waybar/colors/wallpaper.css");
    let style_rc = Rc::new(RefCell::new(StyleConfig { vars: style_vars, path: default_style_path }));
    
    // In-memory paths for session (real paths only used on Apply/Save)
    let layout_css_path = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/waybar/style.css");
    
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
        "keyboard-state", "wlr/taskbar", "idle_inhibitor", "custom/new-module"
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
        
        move || {
            let config = config_rc.borrow();
            let populate = |list: &ListBox, modules: &[String], col_id: &str| {
                while let Some(child) = list.first_child() { list.remove(&child); }
                for m in modules {
                    let row = create_module_row(m);
                    let name = m.clone();
                    let cid = col_id.to_string();
                    let update_cb = Rc::clone(&update_props_ref);
                    let sel_s = Rc::clone(&sel_state);
                    row.connect_activated(move |_| {
                        *sel_s.borrow_mut() = Some((cid.clone(), name.clone()));
                        if let Some(f) = &*update_cb.borrow() { f(name.clone()); }
                    });
                    
                    let ds = gtk::DragSource::new();
                    ds.set_actions(gdk::DragAction::MOVE);
                    let full_id = format!("{}:{}", col_id, m);
                    ds.connect_prepare(move |_, _, _| Some(gdk::ContentProvider::for_value(&full_id.to_value())));
                    row.add_controller(ds);
                    list.append(&row);
                }
            };
            populate(&left_list, &config.modules_left, "left");
            populate(&center_list, &config.modules_center, "center");
            populate(&right_list, &config.modules_right, "right");
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
                                row.add_suffix(&en);
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
            let refresh_self = Rc::clone(&refresh_styles_fn);
            
            move || {
                while let Some(child) = styles_page.first_child() { styles_page.remove(&child); }
                let title = Label::new(Some("Visual Style Editor"));
                title.add_css_class("title-3");
                styles_page.append(&title);

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
        let win_rc = Rc::clone(&win_rc);
        let t_save = t_overlay.clone();
        move |_| {
            let filter = FileFilter::new();
            filter.add_pattern("*.wc"); filter.set_name(Some("Waybar Config Profile (*.wc)"));
            let dialog = FileDialog::builder().title("Save Profile").default_filter(&filter).build();
            let config_rc = Rc::clone(&config_rc);
            let t_s = t_save.clone();
            if let Some(win) = win_rc.borrow().as_ref() {
                dialog.save(Some(win), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            let mut path = path.to_path_buf();
                            if path.extension().and_then(|s| s.to_str()) != Some("wc") { path.set_extension("wc"); }
                            let _ = config_rc.borrow().save_to_file(path.to_str().unwrap());
                            t_s.add_toast(Toast::new("Profile Saved"));
                        }
                    }
                });
            }
        }
    });

    load_profile_btn.connect_clicked({
        let config_rc = Rc::clone(&config_rc);
        let win_rc = Rc::clone(&win_rc);
        let refresh_rc = Rc::clone(&refresh_rc);
        let t_load = t_overlay.clone();
        move |_| {
            let filter = FileFilter::new(); filter.add_pattern("*.wc");
            let dialog = FileDialog::builder().title("Load Profile").default_filter(&filter).build();
            let config_rc = Rc::clone(&config_rc); let refresh_rc = Rc::clone(&refresh_rc); let t_l = t_load.clone();
            if let Some(win) = win_rc.borrow().as_ref() {
                dialog.open(Some(win), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            if let Ok(new_cfg) = WaybarConfig::from_file(path.to_str().unwrap()) {
                                *config_rc.borrow_mut() = new_cfg;
                                refresh_rc();
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
        let t_apply = t_overlay.clone();
        move |_| {
            let home = std::env::var("HOME").unwrap_or_default();
            let waybar_cfg_dir = PathBuf::from(home).join(".config/waybar");
            let _ = fs::create_dir_all(&waybar_cfg_dir);
            let target_cfg = waybar_cfg_dir.join("config.jsonc");
            let target_style = waybar_cfg_dir.join("colors/wallpaper.css");
            let target_layout = waybar_cfg_dir.join("style.css");
            let _ = fs::create_dir_all(waybar_cfg_dir.join("colors"));
            
            let _ = config_rc.borrow().save_to_file(target_cfg.to_str().unwrap());
            let _ = style_rc.borrow().save_to(&target_style);
            
            // Save layout CSS as well
            if !target_layout.exists() {
                let _ = fs::write(&target_layout, DEFAULT_LAYOUT_CSS);
            }

            let _ = Command::new("pkill").args(["-x", "waybar"]).status();
            let _ = Command::new("waybar").spawn();
            let escaped = glib::markup_escape_text("Applied & Restarted Waybar");
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
            let (scol, mname) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            if mname.is_empty() { return false; }
            let mut cfg = config_rc.borrow_mut();
            let slist = match scol { "left" => &mut cfg.modules_left, "center" => &mut cfg.modules_center, "right" => &mut cfg.modules_right, _ => return false };
            let item = if let Some(p) = slist.iter().position(|m| m == mname) { slist.remove(p) } else { return false };
            let tlist = match cid.as_str() { "left" => &mut cfg.modules_left, "center" => &mut cfg.modules_center, "right" => &mut cfg.modules_right, _ => return false };
            let mut idx = tlist.len();
            if let Some(r) = list_c.row_at_y(y as i32) { idx = r.index() as usize; }
            if idx > tlist.len() { tlist.push(item); } else { tlist.insert(idx, item); }
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
        let add_btn = Button::builder().icon_name("list-add-symbolic").has_frame(false).build();
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
                if !cfg.module_definitions.contains_key(&opt_s) { cfg.module_definitions.insert(opt_s.clone(), serde_json::json!({})); }
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
        let p_popup = popover.clone(); add_btn.connect_clicked(move |_| p_popup.popup()); popover.set_parent(&add_btn); h.append(&add_btn);
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
        let refresh_ui_startup = Rc::clone(&refresh_rc);
        let refresh_styles_startup = Rc::clone(&refresh_styles_fn);
        
        dialog.connect_response(None, move |d, response| {
            match response {
                "load" => {
                    if let Ok(new_cfg) = WaybarConfig::from_file(&local_path) {
                        *config_rc_startup.borrow_mut() = new_cfg;
                        
                        let home = std::env::var("HOME").unwrap_or_default();
                        let local_style_path = PathBuf::from(home).join(".config/waybar/colors/wallpaper.css");
                        if local_style_path.exists() {
                            *style_rc_startup.borrow_mut() = StyleConfig::from_file(&local_style_path);
                        }
                        
                        refresh_ui_startup();
                        if let Some(f) = &*refresh_styles_startup.borrow() { f(); }
                    }
                }
                "template" => {
                    *config_rc_startup.borrow_mut() = serde_json::from_str(DEFAULT_CONFIG_JSON).unwrap();
                    style_rc_startup.borrow_mut().vars = parse_style_vars(DEFAULT_STYLE_VARS);
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

fn create_module_row(n: &str) -> ActionRow {
    let icon = if n.starts_with("custom/") { "applications-system-symbolic" } else { match n { "clock" => "x-office-calendar-symbolic", "battery" => "battery-full-symbolic", "cpu" => "chip-symbolic", "memory" => "ram-symbolic", "network" => "network-wireless-symbolic", "pulseaudio" => "audio-volume-high-symbolic", "backlight" => "display-brightness-symbolic", "tray" => "panel-show-symbolic", _ => "extension-symbolic" } };
    ActionRow::builder().title(n).icon_name(icon).activatable(true).build()
}
