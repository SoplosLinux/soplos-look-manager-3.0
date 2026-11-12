// KWin: the virtual desktops, the desktop effects, the scripts and the window switcher
// (Alt+Tab). Each one is done as the page of System Settings does it, from the source
// of KWin 6.7.4:
//
// - Virtual desktops: the interface `org.kde.KWin.VirtualDesktopManager` of KWin on
//   the session bus (`src/org.kde.KWin.VirtualDesktopManager.xml`): the properties
//   `desktops`, `rows` and `navigationWrappingAround`, and the methods `createDesktop`,
//   `setDesktopName` and `removeDesktop`.
// - Effects: `kwinrc`, group `Plugins`, key `<id>Enabled` (`kcms/common/effectsmodel.cpp`),
//   and KWin is told with `loadEffect` or `unloadEffect` of `org.kde.kwin.Effects` at `/Effects`.
// - Scripts: the same key in `kwinrc`, and then `start` of `org.kde.kwin.Scripting` at
//   `/Scripting` (`kcms/scripts/module.cpp`).
// - Window switcher: `kwinrc`, group `TabBox`, key `LayoutName`, and the signal
//   `reloadConfig` of `org.kde.KWin` at `/KWin` (`kcms/tabbox/main.cpp`).
//
// The names and descriptions of effects, scripts and layouts come from the helper, which
// lists them as the pages of System Settings do.

use super::login::ini_get;
use super::looks::{home, icon_files, kwrite, preview_helper, run, valid_id};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Stdio};

/// Runs `busctl` on the session bus.
fn busctl(args: &[&str]) -> Result<String, String> {
    let mut all = vec!["--user"];
    all.extend_from_slice(args);
    run("busctl", &all)
}

/// Tells KWin to read its settings again, as the pages of System Settings do.
pub(crate) fn reload_config() -> Result<(), String> {
    busctl(&["emit", "/KWin", "org.kde.KWin", "reloadConfig"]).map(|_| ())
}

/// The words of an answer of `busctl`: a text in quotes is one word, and `\"` and `\\`
/// inside it mean a quote and a backslash.
fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut word = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        if let Some(next) = chars.next() {
                            word.push(match next {
                                'n' => '\n',
                                't' => '\t',
                                other => other,
                            });
                        }
                    }
                    '"' => break,
                    other => word.push(other),
                }
            }
            out.push(word);
        } else {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                word.push(c);
                chars.next();
            }
            out.push(word);
        }
    }
    out
}

// ── Virtual desktops ─────────────────────────────────────────────────────────

const DESKTOPS: [&str; 3] = ["org.kde.KWin", "/VirtualDesktopManager", "org.kde.KWin.VirtualDesktopManager"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Desktop {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Desktops {
    pub desktops: Vec<Desktop>,
    pub rows: u32,
    /// Whether going past the last desktop goes to the first one.
    pub wrap: bool,
}

fn desktop_property(name: &str) -> Result<Vec<String>, String> {
    busctl(&["get-property", DESKTOPS[0], DESKTOPS[1], DESKTOPS[2], name]).map(|text| words(&text))
}

fn read_desktops() -> Result<Desktops, String> {
    // `desktops` is `a(iss)`: busctl prints the type, the number of desktops and then
    // the three fields of each one (position, id and name).
    let list = desktop_property("desktops")?;
    let count: usize = list.get(1).and_then(|n| n.parse().ok()).ok_or("unexpected answer for the desktops")?;
    let mut desktops = Vec::new();
    for i in 0..count {
        let at = 2 + i * 3;
        let (Some(id), Some(name)) = (list.get(at + 1), list.get(at + 2)) else { return Err("unexpected answer for the desktops".to_string()) };
        desktops.push(Desktop { id: id.clone(), name: name.clone() });
    }
    let rows = desktop_property("rows")?.get(1).and_then(|n| n.parse().ok()).unwrap_or(1);
    let wrap = desktop_property("navigationWrappingAround")?.get(1).map(|v| v == "true").unwrap_or(false);
    Ok(Desktops { desktops, rows, wrap })
}

/// A name that can be given to a desktop: plain text, not empty and not long.
fn desktop_name_ok(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name.chars().count() <= 60 && !name.chars().any(char::is_control)
}

/// The identifier KWin gives a desktop: letters, digits, dashes and braces.
fn desktop_id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '{' | '}'))
}

#[tauri::command(async)]
pub fn list_desktops() -> Result<Desktops, String> {
    read_desktops()
}

/// A new desktop at the end.
#[tauri::command(async)]
pub fn add_desktop(name: String) -> Result<Desktops, String> {
    if !desktop_name_ok(&name) {
        return Err("invalid name".to_string());
    }
    let position = read_desktops()?.desktops.len().to_string();
    busctl(&["call", DESKTOPS[0], DESKTOPS[1], DESKTOPS[2], "createDesktop", "us", &position, name.trim()])?;
    read_desktops()
}

/// KWin does not let the last desktop go.
#[tauri::command(async)]
pub fn remove_desktop(id: String) -> Result<Desktops, String> {
    if !desktop_id_ok(&id) {
        return Err("invalid desktop".to_string());
    }
    if read_desktops()?.desktops.len() < 2 {
        return Err("the last desktop cannot be removed".to_string());
    }
    busctl(&["call", DESKTOPS[0], DESKTOPS[1], DESKTOPS[2], "removeDesktop", "s", &id])?;
    read_desktops()
}

#[tauri::command(async)]
pub fn rename_desktop(id: String, name: String) -> Result<Desktops, String> {
    if !desktop_id_ok(&id) || !desktop_name_ok(&name) {
        return Err("invalid desktop".to_string());
    }
    busctl(&["call", DESKTOPS[0], DESKTOPS[1], DESKTOPS[2], "setDesktopName", "ss", &id, name.trim()])?;
    read_desktops()
}

#[tauri::command(async)]
pub fn set_desktop_rows(rows: u32) -> Result<Desktops, String> {
    if !(1..=20).contains(&rows) {
        return Err("invalid number of rows".to_string());
    }
    busctl(&["set-property", DESKTOPS[0], DESKTOPS[1], DESKTOPS[2], "rows", "u", &rows.to_string()])?;
    read_desktops()
}

#[tauri::command(async)]
pub fn set_desktop_wrap(wrap: bool) -> Result<Desktops, String> {
    busctl(&["set-property", DESKTOPS[0], DESKTOPS[1], DESKTOPS[2], "navigationWrappingAround", "b", if wrap { "true" } else { "false" }])?;
    read_desktops()
}

// ── Effects, scripts and the window switcher ─────────────────────────────────

/// An effect, a script or a layout of the window switcher.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KwinItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    /// The file of its icon, when the icon theme in use has it.
    pub icon: Option<String>,
    /// Whether it is on (effects and scripts) or the one chosen (window switcher).
    pub enabled: bool,
}

fn short_lang(lang: Option<String>) -> String {
    lang.unwrap_or_default().chars().filter(|c| c.is_ascii_alphabetic()).take(2).collect::<String>().to_lowercase()
}

/// The `[Plugins]` group of `kwinrc`, as the text of the file.
fn kwinrc() -> String {
    let config = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home().join(".config"));
    fs::read_to_string(config.join("kwinrc")).unwrap_or_default()
}

fn kwin_items(kind: &str, lang: &str) -> Result<Vec<KwinItem>, String> {
    let helper = preview_helper().ok_or("the helper was not found")?;
    let mut command = Command::new(helper);
    command.arg("kwin").arg(kind).stdin(Stdio::null()).stderr(Stdio::null());
    if !lang.is_empty() {
        command.env("SOPLOS_PREVIEW_LANG", lang);
    }
    let output = command.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("the helper could not list them".to_string());
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let text = kwinrc();
    let layout = ini_get(&text, "[TabBox]", "LayoutName");

    let mut items = Vec::new();
    let mut icon_names: Vec<String> = Vec::new();
    for entry in json.as_array().ok_or("unexpected list")? {
        let id = entry["id"].as_str().unwrap_or_default().to_string();
        if !valid_id(&id) {
            continue;
        }
        let enabled = if kind == "tabbox" {
            layout.as_deref() == Some(id.as_str())
        } else {
            match ini_get(&text, "[Plugins]", &format!("{id}Enabled")).as_deref() {
                Some("true") => true,
                Some("false") => false,
                _ => entry["enabledByDefault"].as_bool().unwrap_or(false),
            }
        };
        let icon = entry["icon"].as_str().unwrap_or_default().to_string();
        if !icon.is_empty() {
            icon_names.push(icon.clone());
        }
        let name = entry["name"].as_str().unwrap_or_default().to_string();
        items.push((
            KwinItem {
                id: id.clone(),
                name: if name.is_empty() { id } else { name },
                description: entry["description"].as_str().unwrap_or_default().to_string(),
                category: entry["category"].as_str().unwrap_or_default().to_string(),
                icon: None,
                enabled,
            },
            icon,
        ));
    }
    icon_names.sort();
    icon_names.dedup();
    let icons: HashMap<String, String> = icon_files(&icon_names);
    let mut items: Vec<KwinItem> = items
        .into_iter()
        .map(|(mut item, icon)| {
            item.icon = icons.get(&icon).cloned();
            item
        })
        .collect();
    items.sort_by(|a, b| a.category.to_lowercase().cmp(&b.category.to_lowercase()).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(items)
}

#[tauri::command(async)]
pub fn list_kwin_effects(lang: Option<String>) -> Result<Vec<KwinItem>, String> {
    kwin_items("effects", &short_lang(lang))
}

#[tauri::command(async)]
pub fn list_kwin_scripts(lang: Option<String>) -> Result<Vec<KwinItem>, String> {
    kwin_items("scripts", &short_lang(lang))
}

#[tauri::command(async)]
pub fn list_window_switchers(lang: Option<String>) -> Result<Vec<KwinItem>, String> {
    kwin_items("tabbox", &short_lang(lang))
}

/// Turns an effect on or off, and tells KWin so it takes effect now.
#[tauri::command(async)]
pub fn set_kwin_effect(id: String, enabled: bool) -> Result<(), String> {
    if !valid_id(&id) {
        return Err("invalid effect".to_string());
    }
    kwrite("kwinrc", "Plugins", &format!("{id}Enabled"), if enabled { "true" } else { "false" })?;
    busctl(&["call", "org.kde.KWin", "/Effects", "org.kde.kwin.Effects", if enabled { "loadEffect" } else { "unloadEffect" }, "s", &id]).map(|_| ())
}

/// Turns a script on or off; KWin is asked to start the ones that are on, as System
/// Settings does.
#[tauri::command(async)]
pub fn set_kwin_script(id: String, enabled: bool) -> Result<(), String> {
    if !valid_id(&id) {
        return Err("invalid script".to_string());
    }
    kwrite("kwinrc", "Plugins", &format!("{id}Enabled"), if enabled { "true" } else { "false" })?;
    busctl(&["call", "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting", "start"]).map(|_| ())
}

/// Chooses the layout of the window switcher, and asks KWin to read its settings again.
#[tauri::command(async)]
pub fn set_window_switcher(id: String) -> Result<(), String> {
    if !valid_id(&id) {
        return Err("invalid layout".to_string());
    }
    kwrite("kwinrc", "TabBox", "LayoutName", &id)?;
    busctl(&["emit", "/KWin", "org.kde.KWin", "reloadConfig"]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_answer_of_busctl() {
        let list = words(r#"a(iss) 2 0 "{a-1}" "Desktop 1" 1 "{b-2}" "My \"desk\"""#);
        assert_eq!(list[0], "a(iss)");
        assert_eq!(list[1], "2");
        assert_eq!(list[3], "{a-1}");
        assert_eq!(list[4], "Desktop 1");
        assert_eq!(list[7], "My \"desk\"");
        assert_eq!(words("u 4"), vec!["u", "4"]);
        assert_eq!(words("b true"), vec!["b", "true"]);
    }
}
