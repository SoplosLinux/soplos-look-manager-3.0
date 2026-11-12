// The panels of Plasma: where each one is, how tall and how long, how it is
// aligned, whether it floats, how opaque it is and when it hides; and adding
// and removing panels. Plasma has no tool for it, but the shell runs scripts
// over D-Bus, which is how Plasma itself changes its panels.
//
// The properties and their values are the ones of the scripting API of Plasma
// 6.7 (`shell/scripting/panel.cpp`).

use super::looks::icon_files;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PanelState {
    /// The number Plasma gives the panel.
    pub id: u32,
    pub location: String,
    pub height: u32,
    pub floating: bool,
    pub hiding: String,
    pub alignment: String,
    /// "fill", "fit" or "custom".
    pub length_mode: String,
    pub offset: u32,
    pub minimum_length: u32,
    pub maximum_length: u32,
    /// "adaptive", "opaque" or "translucent".
    pub opacity: String,
}

const LOCATIONS: [&str; 4] = ["top", "bottom", "left", "right"];
const HIDINGS: [&str; 4] = ["none", "autohide", "dodgewindows", "windowsgobelow"];
const ALIGNMENTS: [&str; 3] = ["left", "center", "right"];
const LENGTH_MODES: [&str; 3] = ["fill", "fit", "custom"];
const OPACITIES: [&str; 3] = ["adaptive", "opaque", "translucent"];
/// The templates of Plasma for a new panel: the default one, an empty one and
/// the bar of the menu of the application.
const TEMPLATES: [&str; 3] = [
    "org.kde.plasma.desktop.defaultPanel",
    "org.kde.plasma.desktop.emptyPanel",
    "org.kde.plasma.desktop.appmenubar",
];

fn has_command(name: &str) -> bool {
    std::env::var_os("PATH").map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file())).unwrap_or(false)
}

/// Runs a script in the shell of Plasma and returns what it printed.
fn evaluate(script: &str) -> Result<String, String> {
    let tool = ["qdbus6", "qdbus-qt6", "qdbus"].into_iter().find(|t| has_command(t)).ok_or_else(|| "qdbus was not found".to_string())?;
    let output = Command::new(tool)
        .args(["org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.evaluateScript", script])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// A number that came out of the script; what is not a number is zero.
fn number(text: &str) -> u32 {
    text.trim().parse::<f64>().map(|v| v.max(0.0) as u32).unwrap_or(0)
}

/// A word that came out of the script, or the one given when the panel did not
/// say it (a property that this version of Plasma does not have is `undefined`).
fn word(text: &str, allowed: &[&str], otherwise: &str) -> String {
    let text = text.trim();
    if allowed.contains(&text) { text.to_string() } else { otherwise.to_string() }
}

fn read_panels() -> Result<Vec<PanelState>, String> {
    // Each property is asked on its own, so one that a version of Plasma does
    // not have leaves the others as they are.
    let said = evaluate(
        r#"var names = ["location", "height", "floating", "hiding", "alignment", "lengthMode", "offset", "minimumLength", "maximumLength", "opacity"];
var out = [];
var all = panels();
for (var i = 0; i < all.length; i++) {
    var p = all[i];
    var row = [p.id];
    for (var j = 0; j < names.length; j++) {
        var v = p[names[j]];
        row.push(typeof v === "undefined" ? "" : v);
    }
    out.push(row.join("|"));
}
print(out.join("\n"));"#,
    )?;
    let mut found = Vec::new();
    for line in said.lines().filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = line.trim().split('|').collect();
        if f.len() != 11 {
            return Err(format!("unexpected answer: {line}"));
        }
        found.push(PanelState {
            id: f[0].trim().parse().map_err(|_| format!("unexpected id: {}", f[0]))?,
            location: word(f[1], &LOCATIONS, "bottom"),
            height: number(f[2]),
            floating: f[3].trim() == "true",
            hiding: word(f[4], &HIDINGS, "none"),
            alignment: word(f[5], &ALIGNMENTS, "center"),
            length_mode: word(f[6], &LENGTH_MODES, "fill"),
            offset: number(f[7]),
            minimum_length: number(f[8]),
            maximum_length: number(f[9]),
            opacity: word(f[10], &OPACITIES, "adaptive"),
        });
    }
    Ok(found)
}

#[tauri::command(async)]
pub fn list_panels() -> Result<Vec<PanelState>, String> {
    read_panels()
}

#[tauri::command(async)]
pub fn set_panel(state: PanelState) -> Result<(), String> {
    let valid = LOCATIONS.contains(&state.location.as_str())
        && HIDINGS.contains(&state.hiding.as_str())
        && ALIGNMENTS.contains(&state.alignment.as_str())
        && LENGTH_MODES.contains(&state.length_mode.as_str())
        && OPACITIES.contains(&state.opacity.as_str())
        && (24..=200).contains(&state.height)
        && state.offset <= 20_000
        && state.minimum_length <= 20_000
        && state.maximum_length <= 20_000;
    if !valid {
        return Err("invalid panel settings".to_string());
    }
    // The length mode goes before the lengths, and the lengths only count in
    // the custom mode. Each change is made on its own, so one that this version
    // of Plasma does not have does not stop the others.
    let lengths = if state.length_mode == "custom" {
        format!(r#"set("minimumLength", {}); set("maximumLength", {});"#, state.minimum_length, state.maximum_length)
    } else {
        String::new()
    };
    let script = format!(
        r#"var p = panelById({}); function set(name, value) {{ try {{ p[name] = value; }} catch (e) {{}} }}
set("location", "{}"); set("height", {}); set("floating", {}); set("hiding", "{}"); set("opacity", "{}"); set("alignment", "{}"); set("lengthMode", "{}"); {} set("offset", {});"#,
        state.id, state.location, state.height, state.floating, state.hiding, state.opacity, state.alignment, state.length_mode, lengths, state.offset
    );
    evaluate(&script).map(|_| ())
}

/// A new panel from one of the templates of Plasma.
#[tauri::command(async)]
pub fn add_panel(template: String) -> Result<Vec<PanelState>, String> {
    if !TEMPLATES.contains(&template.as_str()) {
        return Err("unknown template".to_string());
    }
    evaluate(&format!(r#"loadTemplate("{template}");"#))?;
    read_panels()
}

/// Removes a panel, never the last one: the desktop would be left with none.
#[tauri::command(async)]
pub fn remove_panel(id: u32) -> Result<Vec<PanelState>, String> {
    if read_panels()?.len() < 2 {
        return Err("the last panel cannot be removed".to_string());
    }
    evaluate(&format!(r#"var p = panelById({id}); if (p) {{ p.remove(); }}"#))?;
    read_panels()
}

// ── The elements of a panel ──────────────────────────────────────────────────
//
// What goes on a panel are the widgets of Plasma (the application launcher, the
// tasks, the system tray, the clock...). The scripting API of Plasma can list the
// widgets of a panel, add one by the name of its plugin and remove one. It cannot
// move them: `index` does nothing in `shell/scripting/widget.cpp`, so a new widget
// goes at the end of the panel.

/// A widget that can be put on a panel, from the metadata of its package.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetType {
    pub plugin: String,
    pub name: String,
    pub description: String,
    pub category: String,
    /// The file of its icon, when the icon theme in use has it.
    pub icon: Option<String>,
}

/// A widget that is on a panel.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelWidget {
    pub id: u32,
    pub plugin: String,
    pub name: String,
    pub icon: Option<String>,
}

fn plugin_name_ok(plugin: &str) -> bool {
    !plugin.is_empty() && plugin.len() <= 128 && plugin.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The text of a key in the language of the program: `Name[es]`, or `Name`.
fn localized(plugin: &serde_json::Value, key: &str, lang: &str) -> String {
    let variants: Vec<String> = match lang {
        "" => Vec::new(),
        "pt" => vec!["pt".to_string(), "pt_BR".to_string()],
        other => vec![other.to_string()],
    };
    variants
        .iter()
        .find_map(|v| plugin[format!("{key}[{v}]")].as_str())
        .or_else(|| plugin[key].as_str())
        .unwrap_or_default()
        .to_string()
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

/// Every widget that Plasma has, as Plasma lists them: the helper asks libplasma for
/// them, which finds the packages of QML and also the plugins that are compiled (the
/// application launcher, the pager, the system tray, the clock...), which have no
/// `metadata.json` to read. If the helper is not there, the packages are read.
fn read_widget_types(lang: &str) -> Vec<WidgetType> {
    let from_helper = super::looks::preview_helper().and_then(|helper| {
        let mut command = Command::new(helper);
        command.arg("widgets").stdin(Stdio::null()).stderr(Stdio::null());
        if !lang.is_empty() {
            command.env("SOPLOS_PREVIEW_LANG", lang);
        }
        let output = command.output().ok().filter(|o| o.status.success())?;
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        let mut found: Vec<WidgetType> = Vec::new();
        let mut icon_names: Vec<String> = Vec::new();
        for widget in json.as_array()? {
            let id = widget["id"].as_str().unwrap_or_default().to_string();
            // A containment or a widget of the system tray is not put on a panel, and a
            // widget that says its form factors must say the desktop.
            let wrong_form = widget["formFactors"].as_array().is_some_and(|forms| !forms.is_empty() && !forms.iter().any(|f| f.as_str() == Some("desktop")));
            if !plugin_name_ok(&id) || widget["containment"].as_bool() == Some(true) || widget["tray"].as_bool() == Some(true) || wrong_form || found.iter().any(|w| w.plugin == id) {
                continue;
            }
            let icon = widget["icon"].as_str().unwrap_or_default().to_string();
            if !icon.is_empty() {
                icon_names.push(icon.clone());
            }
            let name = widget["name"].as_str().unwrap_or_default().to_string();
            found.push(WidgetType {
                plugin: id.clone(),
                name: if name.is_empty() { id } else { name },
                description: widget["description"].as_str().unwrap_or_default().to_string(),
                category: widget["category"].as_str().unwrap_or_default().to_string(),
                icon: if icon.is_empty() { None } else { Some(icon) },
            });
        }
        icon_names.sort();
        icon_names.dedup();
        let icons = icon_files(&icon_names);
        for widget in &mut found {
            widget.icon = widget.icon.as_ref().and_then(|name| icons.get(name).cloned());
        }
        found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Some(found)
    });
    from_helper.filter(|found| !found.is_empty()).unwrap_or_else(|| read_widget_types_from_files(lang))
}

/// The same, from the `metadata.json` of the packages in `plasma/plasmoids`.
fn read_widget_types_from_files(lang: &str) -> Vec<WidgetType> {
    let folders = [PathBuf::from("/usr/share/plasma/plasmoids"), home().join(".local/share/plasma/plasmoids")];
    let mut found: Vec<WidgetType> = Vec::new();
    let mut icon_names: Vec<String> = Vec::new();
    let mut icons: HashMap<String, String> = HashMap::new();
    for base in folders {
        let Ok(read) = fs::read_dir(&base) else { continue };
        for entry in read.flatten() {
            let dir = entry.path();
            let Ok(text) = fs::read_to_string(dir.join("metadata.json")) else { continue };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
            // Only the widgets of the desktop shell that Plasma lists to be added: not a
            // containment (the desktop, the folder view).
            if json["KPackageStructure"].as_str().is_some_and(|s| s != "Plasma/Applet") || !json["X-Plasma-ContainmentType"].is_null() {
                continue;
            }
            let plugin = &json["KPlugin"];
            if let Some(forms) = plugin["FormFactors"].as_array() {
                if !forms.iter().any(|f| f.as_str() == Some("desktop")) {
                    continue;
                }
            }
            // The ones of the system tray are put in the tray, not on the panel.
            if matches!(json["X-Plasma-NotificationArea"].as_str(), Some("true")) || json["X-Plasma-NotificationArea"].as_bool() == Some(true) {
                continue;
            }
            let id = plugin["Id"].as_str().map(str::to_string).or_else(|| dir.file_name().map(|n| n.to_string_lossy().to_string())).unwrap_or_default();
            if !plugin_name_ok(&id) || found.iter().any(|w| w.plugin == id) {
                continue;
            }
            let icon = plugin["Icon"].as_str().unwrap_or_default().to_string();
            if !icon.is_empty() {
                icon_names.push(icon.clone());
            }
            let name = localized(plugin, "Name", lang);
            found.push(WidgetType {
                plugin: id.clone(),
                name: if name.is_empty() { id } else { name },
                description: localized(plugin, "Description", lang),
                category: localized(plugin, "Category", lang),
                icon: if icon.is_empty() { None } else { Some(icon) },
            });
        }
    }
    icon_names.sort();
    icon_names.dedup();
    icons.extend(icon_files(&icon_names));
    for widget in &mut found {
        widget.icon = widget.icon.as_ref().and_then(|name| icons.get(name).cloned());
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
}

fn short_lang(lang: Option<String>) -> String {
    lang.unwrap_or_default().chars().filter(|c| c.is_ascii_alphabetic()).take(2).collect::<String>().to_lowercase()
}

/// The widgets that can be added to a panel, with the names in the language given.
#[tauri::command(async)]
pub fn list_widget_types(lang: Option<String>) -> Vec<WidgetType> {
    read_widget_types(&short_lang(lang))
}

fn read_panel_widgets(panel: u32, lang: &str) -> Result<Vec<PanelWidget>, String> {
    let said = evaluate(&format!(
        r#"var out = []; var all = panels();
for (var i = 0; i < all.length; i++) {{
    var p = all[i];
    if (p.id == {panel}) {{
        var ids = p.widgetIds;
        for (var j = 0; j < ids.length; j++) {{
            var w = p.widgetById(ids[j]);
            out.push(ids[j] + "|" + (w ? w.type : ""));
        }}
    }}
}}
print(out.join("\n"));"#
    ))?;
    let types = read_widget_types(lang);
    let mut widgets = Vec::new();
    for line in said.lines().filter(|l| !l.trim().is_empty()) {
        let Some((id, plugin)) = line.trim().split_once('|') else { continue };
        let Ok(id) = id.trim().parse::<u32>() else { continue };
        let plugin = plugin.trim().to_string();
        let known = types.iter().find(|t| t.plugin == plugin);
        widgets.push(PanelWidget {
            id,
            name: known.map(|t| t.name.clone()).unwrap_or_else(|| plugin.clone()),
            icon: known.and_then(|t| t.icon.clone()),
            plugin,
        });
    }
    Ok(widgets)
}

/// The widgets on a panel, in the order they are on it.
#[tauri::command(async)]
pub fn list_panel_widgets(panel: u32, lang: Option<String>) -> Result<Vec<PanelWidget>, String> {
    read_panel_widgets(panel, &short_lang(lang))
}

/// Puts a widget at the end of a panel.
#[tauri::command(async)]
pub fn add_panel_widget(panel: u32, plugin: String, lang: Option<String>) -> Result<Vec<PanelWidget>, String> {
    if !plugin_name_ok(&plugin) {
        return Err("invalid widget".to_string());
    }
    evaluate(&format!(r#"var p = panelById({panel}); if (p) {{ p.addWidget("{plugin}"); }}"#))?;
    read_panel_widgets(panel, &short_lang(lang))
}

/// Takes a widget off a panel.
#[tauri::command(async)]
pub fn remove_panel_widget(panel: u32, widget: u32, lang: Option<String>) -> Result<Vec<PanelWidget>, String> {
    evaluate(&format!(r#"var p = panelById({panel}); if (p) {{ var w = p.widgetById({widget}); if (w) {{ w.remove(); }} }}"#))?;
    read_panel_widgets(panel, &short_lang(lang))
}
