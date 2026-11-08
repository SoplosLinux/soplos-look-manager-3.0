// The panels of Plasma: where each one is, how tall it is, whether it floats and
// when it hides; and adding and removing panels. Plasma has no tool for it, but
// the shell runs scripts over D-Bus, which is how Plasma itself changes its
// panels.

use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

#[derive(Serialize, Deserialize, Clone)]
pub struct PanelState {
    /// The number Plasma gives the panel.
    pub id: u32,
    pub location: String,
    pub height: u32,
    pub floating: bool,
    pub hiding: String,
}

const LOCATIONS: [&str; 4] = ["top", "bottom", "left", "right"];
const HIDINGS: [&str; 4] = ["none", "autohide", "dodgewindows", "windowsgobelow"];

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

fn read_panels() -> Result<Vec<PanelState>, String> {
    let said = evaluate(
        r#"var out = []; var all = panels(); for (var i = 0; i < all.length; i++) { var p = all[i]; out.push(p.id + "|" + p.location + "|" + p.height + "|" + p.floating + "|" + p.hiding); } print(out.join("\n"));"#,
    )?;
    let mut found = Vec::new();
    for line in said.lines().filter(|l| !l.trim().is_empty()) {
        let fields: Vec<&str> = line.trim().split('|').collect();
        if fields.len() != 5 {
            return Err(format!("unexpected answer: {line}"));
        }
        found.push(PanelState {
            id: fields[0].parse().map_err(|_| format!("unexpected id: {}", fields[0]))?,
            location: fields[1].to_string(),
            height: fields[2].parse().map_err(|_| format!("unexpected height: {}", fields[2]))?,
            floating: fields[3] == "true",
            hiding: fields[4].to_string(),
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
    if !LOCATIONS.contains(&state.location.as_str()) || !HIDINGS.contains(&state.hiding.as_str()) || !(24..=200).contains(&state.height) {
        return Err("invalid panel settings".to_string());
    }
    let script = format!(
        r#"var p = panelById({}); p.location = "{}"; p.height = {}; p.floating = {}; p.hiding = "{}";"#,
        state.id, state.location, state.height, state.floating, state.hiding
    );
    evaluate(&script).map(|_| ())
}

/// A new panel on a side that has none, with the widgets every desktop needs:
/// the launcher, the tasks, the system tray and the clock. A panel with no
/// widgets would be empty and of no use.
#[tauri::command(async)]
pub fn add_panel() -> Result<Vec<PanelState>, String> {
    let used: Vec<String> = read_panels()?.into_iter().map(|p| p.location).collect();
    let side = ["bottom", "top", "left", "right"].into_iter().find(|s| !used.iter().any(|u| u == s)).unwrap_or("bottom");
    let script = format!(
        r#"var p = new Panel; p.location = "{side}"; p.height = 44; p.addWidget("org.kde.plasma.kickoff"); p.addWidget("org.kde.plasma.icontasks"); p.addWidget("org.kde.plasma.marginsseparator"); p.addWidget("org.kde.plasma.systemtray"); p.addWidget("org.kde.plasma.digitalclock"); print(p.id);"#
    );
    evaluate(&script)?;
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
