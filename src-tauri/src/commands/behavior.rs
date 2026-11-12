// The behavior of the windows: focus, placement, snapping and what the mouse does on the
// title bar and on the windows. They are the options of the page "Window Behavior" of
// System Settings (`kcms/options` in KWin 6.7), kept in the groups `Windows` and
// `MouseBindings` of `kwinrc`, with the same keys and the same stored values.
//
// The table below is the list of what the page offers: the window shows the rows as
// they come, and a change is accepted only for a row of the table with one of its values.
// KWin is told with `reloadConfig`, as the page of System Settings does.

use super::kwin::reload_config;
use super::looks::{home, kwrite};
use serde::Serialize;
use std::fs;

/// What a row holds.
enum Kind {
    /// One of these: the stored text, and the name the window translates it with.
    Choice(&'static [(&'static str, &'static str)]),
    Bool,
    /// A whole number from 0 to this.
    Number(i32),
}

struct Row {
    /// The key of `kwinrc`, also the name of the label.
    key: &'static str,
    group: &'static str,
    section: &'static str,
    kind: Kind,
    default: &'static str,
}

const FOCUS: &[(&str, &str)] = &[("ClickToFocus", "click"), ("FocusFollowsMouse", "follows"), ("FocusUnderMouse", "under"), ("FocusStrictlyUnderMouse", "strictly")];
const STEAL: &[(&str, &str)] = &[("0", "stealNone"), ("1", "low"), ("2", "medium"), ("3", "high"), ("4", "extreme")];
const PLACEMENT: &[(&str, &str)] = &[("Smart", "smart"), ("Maximizing", "maximizing"), ("Random", "random"), ("Centered", "centered"), ("ZeroCornered", "zeroCorner"), ("UnderMouse", "underMouse")];
const ACTIVATION: &[(&str, &str)] = &[("SwitchToOtherDesktop", "switchDesktop"), ("BringToCurrentDesktop", "bringHere")];
const DOUBLE: &[(&str, &str)] = &[
    ("Maximize", "maximize"),
    ("Maximize (vertical only)", "maximizeV"),
    ("Maximize (horizontal only)", "maximizeH"),
    ("Minimize", "minimize"),
    ("Lower", "lower"),
    ("Close", "close"),
    ("OnAllDesktops", "onAllDesktops"),
    ("Nothing", "nothing"),
];
const MAXIMIZE: &[(&str, &str)] = &[("Maximize", "maximize"), ("Maximize (vertical only)", "maximizeV"), ("Maximize (horizontal only)", "maximizeH")];
const ACTIVE: &[(&str, &str)] = &[
    ("Raise", "raise"),
    ("Lower", "lower"),
    ("Toggle raise and lower", "toggle"),
    ("Minimize", "minimize"),
    ("Close", "close"),
    ("Operations menu", "menu"),
    ("Nothing", "nothing"),
];
const INACTIVE: &[(&str, &str)] = &[
    ("Activate and raise", "activateRaise"),
    ("Activate and lower", "activateLower"),
    ("Activate", "activate"),
    ("Raise", "raise"),
    ("Lower", "lower"),
    ("Toggle raise and lower", "toggle"),
    ("Minimize", "minimize"),
    ("Close", "close"),
    ("Operations menu", "menu"),
    ("Nothing", "nothing"),
];
const WHEEL: &[(&str, &str)] = &[
    ("Raise/Lower", "wheelRaiseLower"),
    ("Maximize/Restore", "wheelMaximize"),
    ("Above/Below", "wheelAbove"),
    ("Previous/Next Desktop", "wheelDesktop"),
    ("Change Opacity", "wheelOpacity"),
    ("Nothing", "nothing"),
];
const WINDOW_FIRST: &[(&str, &str)] = &[
    ("Activate, pass click and raise on release", "passRaiseRelease"),
    ("Activate, raise and pass click", "activateRaisePass"),
    ("Activate and pass click", "activatePass"),
    ("Activate", "activate"),
    ("Activate and raise", "activateRaise"),
];
const WINDOW_OTHER: &[(&str, &str)] = &[("Activate, raise and pass click", "activateRaisePass"), ("Activate and pass click", "activatePass"), ("Activate", "activate"), ("Activate and raise", "activateRaise")];
const WINDOW_WHEEL: &[(&str, &str)] = &[("Scroll", "scroll"), ("Activate and scroll", "activateScroll"), ("Activate, raise and scroll", "activateRaiseScroll")];
const ALL_KEY: &[(&str, &str)] = &[("Meta", "meta"), ("Alt", "alt")];
const ALL: &[(&str, &str)] = &[
    ("Move", "move"),
    ("Activate, raise and move", "activateRaiseMove"),
    ("Toggle raise and lower", "toggle"),
    ("Resize", "resize"),
    ("Raise", "raise"),
    ("Lower", "lower"),
    ("Minimize", "minimize"),
    ("Decrease Opacity", "decOpacity"),
    ("Increase Opacity", "incOpacity"),
    ("Operations menu", "menu"),
    ("Nothing", "nothing"),
];

fn rows() -> Vec<Row> {
    use Kind::*;
    let w = "Windows";
    let m = "MouseBindings";
    let r = |key, group, section, kind, default| Row { key, group, section, kind, default };
    vec![
        r("FocusPolicy", w, "focus", Choice(FOCUS), "ClickToFocus"),
        r("FocusStealingPreventionLevel", w, "focus", Choice(STEAL), "1"),
        r("ClickRaise", w, "focus", Bool, "true"),
        r("AutoRaise", w, "focus", Bool, "false"),
        r("AutoRaiseInterval", w, "focus", Number(10000), "750"),
        r("DelayFocusInterval", w, "focus", Number(10000), "300"),
        r("NextFocusPrefersMouse", w, "focus", Bool, "false"),
        r("SeparateScreenFocus", w, "focus", Bool, "true"),
        r("Placement", w, "placement", Choice(PLACEMENT), "Centered"),
        r("ActivationDesktopPolicy", w, "placement", Choice(ACTIVATION), "SwitchToOtherDesktop"),
        r("BorderSnapZone", w, "snap", Number(100), "10"),
        r("WindowSnapZone", w, "snap", Number(100), "10"),
        r("CenterSnapZone", w, "snap", Number(100), "0"),
        r("SnapOnlyWhenOverlapping", w, "snap", Bool, "false"),
        r("TitlebarDoubleClickCommand", w, "titlebar", Choice(DOUBLE), "Maximize"),
        r("DoubleClickBorderToMaximize", m, "titlebar", Bool, "true"),
        r("MaximizeButtonLeftClickCommand", w, "titlebar", Choice(MAXIMIZE), "Maximize"),
        r("MaximizeButtonMiddleClickCommand", w, "titlebar", Choice(MAXIMIZE), "Maximize (vertical only)"),
        r("MaximizeButtonRightClickCommand", w, "titlebar", Choice(MAXIMIZE), "Maximize (horizontal only)"),
        r("CommandActiveTitlebar1", m, "activeTitlebar", Choice(ACTIVE), "Raise"),
        r("CommandActiveTitlebar2", m, "activeTitlebar", Choice(ACTIVE), "Nothing"),
        r("CommandActiveTitlebar3", m, "activeTitlebar", Choice(ACTIVE), "Operations menu"),
        r("CommandTitlebarWheel", m, "activeTitlebar", Choice(WHEEL), "Nothing"),
        r("CommandInactiveTitlebar1", m, "inactiveTitlebar", Choice(INACTIVE), "Activate and raise"),
        r("CommandInactiveTitlebar2", m, "inactiveTitlebar", Choice(INACTIVE), "Nothing"),
        r("CommandInactiveTitlebar3", m, "inactiveTitlebar", Choice(INACTIVE), "Operations menu"),
        r("CommandWindow1", m, "windows", Choice(WINDOW_FIRST), "Activate, pass click and raise on release"),
        r("CommandWindow2", m, "windows", Choice(WINDOW_OTHER), "Activate and pass click"),
        r("CommandWindow3", m, "windows", Choice(WINDOW_OTHER), "Activate and pass click"),
        r("CommandWindowWheel", m, "windows", Choice(WINDOW_WHEEL), "Scroll"),
        r("CommandAllKey", m, "modifier", Choice(ALL_KEY), "Meta"),
        r("CommandAll1", m, "modifier", Choice(ALL), "Move"),
        r("CommandAll2", m, "modifier", Choice(ALL), "Toggle raise and lower"),
        r("CommandAll3", m, "modifier", Choice(ALL), "Resize"),
        r("CommandAllWheel", m, "modifier", Choice(WHEEL), "Nothing"),
    ]
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorRow {
    pub key: String,
    pub section: String,
    /// `choice`, `bool` or `number`.
    pub kind: String,
    /// For a choice: the stored text and the name it is translated with.
    pub choices: Vec<(String, String)>,
    pub max: i32,
    pub value: String,
}

fn kwinrc() -> String {
    let config = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home().join(".config"));
    fs::read_to_string(config.join("kwinrc")).unwrap_or_default()
}

/// What the page offers, with what each option holds now (its default when `kwinrc` says nothing).
#[tauri::command(async)]
pub fn list_behavior() -> Vec<BehaviorRow> {
    let text = kwinrc();
    rows()
        .into_iter()
        .map(|row| {
            let value = super::login::ini_get(&text, &format!("[{}]", row.group), row.key).unwrap_or_else(|| row.default.to_string());
            let (kind, choices, max) = match &row.kind {
                Kind::Choice(list) => ("choice", list.iter().map(|(v, n)| (v.to_string(), n.to_string())).collect(), 0),
                Kind::Bool => ("bool", Vec::new(), 0),
                Kind::Number(max) => ("number", Vec::new(), *max),
            };
            BehaviorRow { key: row.key.to_string(), section: row.section.to_string(), kind: kind.to_string(), choices, max, value }
        })
        .collect()
}

/// Whether the value is one that the row accepts.
fn accepts(kind: &Kind, value: &str) -> bool {
    match kind {
        Kind::Choice(list) => list.iter().any(|(stored, _)| *stored == value),
        Kind::Bool => value == "true" || value == "false",
        Kind::Number(max) => value.parse::<i32>().is_ok_and(|n| (0..=*max).contains(&n)),
    }
}

#[tauri::command(async)]
pub fn set_behavior(key: String, value: String) -> Result<(), String> {
    let row = rows().into_iter().find(|r| r.key == key).ok_or("unknown option")?;
    if !accepts(&row.kind, &value) {
        return Err("that value is not accepted".to_string());
    }
    kwrite("kwinrc", row.group, row.key, &value)?;
    reload_config()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_is_accepted_by_its_row() {
        for row in rows() {
            assert!(accepts(&row.kind, row.default), "{} does not accept its default {}", row.key, row.default);
        }
    }

    #[test]
    fn only_listed_values_are_accepted() {
        assert!(accepts(&Kind::Choice(FOCUS), "FocusUnderMouse") && !accepts(&Kind::Choice(FOCUS), "x"));
        assert!(accepts(&Kind::Number(100), "100") && !accepts(&Kind::Number(100), "101") && !accepts(&Kind::Number(100), "-1"));
        assert!(accepts(&Kind::Bool, "false") && !accepts(&Kind::Bool, "1"));
    }

    #[test]
    fn keys_are_not_repeated() {
        let list = rows();
        for (i, a) in list.iter().enumerate() {
            assert!(!list[i + 1..].iter().any(|b| b.key == a.key), "{} is repeated", a.key);
        }
    }
}
