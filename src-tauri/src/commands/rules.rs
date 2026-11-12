// The rules of the windows: what KWin does to a window that matches (where it opens, that
// it stays above, that it has no border...). They are the page "Window Rules" of System
// Settings (`kcms/rules` in KWin 6.7), kept in `kwinrulesrc`:
//
//   [General]
//   rules=<id>,<id>      the order in which they are tried
//   [<id>]
//   Description=..., Enabled=true
//   wmclass=..., wmclassmatch=1, wmclasscomplete=false, title=..., titlematch=2
//   <property>=<value>, <property>rule=<action>
//
// A property has an action: 1 do not affect, 2 force, 3 apply initially, 4 remember,
// 5 apply now, 6 force temporarily; the properties of "force" kind take only 1, 2 and 6
// (`rules.h`). No action (the key is missing) means the property is not used.
//
// The file is read and written here, line by line, so what the program does not know is
// kept as it was. KWin is told with `reloadConfig`, as the page of System Settings does.

use super::kwin::reload_config;
use super::looks::home;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// What a property holds, and which actions it takes.
struct Spec {
    key: &'static str,
    kind: &'static str,
    force: bool,
}

/// The properties the window offers.
const SPECS: &[Spec] = &[
    Spec { key: "position", kind: "point", force: false },
    Spec { key: "size", kind: "size", force: false },
    Spec { key: "maximizehoriz", kind: "bool", force: false },
    Spec { key: "maximizevert", kind: "bool", force: false },
    Spec { key: "minimize", kind: "bool", force: false },
    Spec { key: "fullscreen", kind: "bool", force: false },
    Spec { key: "above", kind: "bool", force: false },
    Spec { key: "below", kind: "bool", force: false },
    Spec { key: "noborder", kind: "bool", force: false },
    Spec { key: "skiptaskbar", kind: "bool", force: false },
    Spec { key: "skippager", kind: "bool", force: false },
    Spec { key: "skipswitcher", kind: "bool", force: false },
    Spec { key: "screen", kind: "screen", force: false },
    Spec { key: "opacityactive", kind: "percent", force: true },
    Spec { key: "opacityinactive", kind: "percent", force: true },
];

#[derive(Serialize, Deserialize, Clone)]
pub struct Property {
    pub key: String,
    pub value: String,
    /// 0 when the property is not used.
    pub action: u8,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// Empty for a new rule.
    pub id: String,
    pub description: String,
    pub enabled: bool,
    pub wmclass: String,
    /// 0 any, 1 exact, 2 contains, 3 regular expression.
    pub wmclassmatch: u8,
    pub wmclasscomplete: bool,
    pub title: String,
    pub titlematch: u8,
    pub properties: Vec<Property>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecInfo {
    pub key: String,
    pub kind: String,
    pub force: bool,
}

#[derive(Serialize)]
pub struct RuleBook {
    pub rules: Vec<Rule>,
    pub specs: Vec<SpecInfo>,
}

// ── The file ─────────────────────────────────────────────────────────────────

/// A settings file as groups of lines: what is not touched stays as it was.
struct Ini {
    /// The first group has no name: what comes before the first header.
    groups: Vec<(String, Vec<String>)>,
}

fn unescape(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('s') => out.push(' '),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn escape(value: &str) -> String {
    let mut out = String::new();
    let count = value.chars().count();
    for (i, c) in value.chars().enumerate() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("\\t"),
            ' ' if i == 0 || i + 1 == count => out.push_str("\\s"),
            c => out.push(c),
        }
    }
    out
}

impl Ini {
    fn parse(text: &str) -> Ini {
        let mut groups: Vec<(String, Vec<String>)> = vec![(String::new(), Vec::new())];
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                groups.push((trimmed[1..trimmed.len() - 1].to_string(), Vec::new()));
            } else {
                groups.last_mut().unwrap().1.push(line.to_string());
            }
        }
        Ini { groups }
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for (name, lines) in &self.groups {
            if !name.is_empty() {
                out.push_str(&format!("[{name}]\n"));
            }
            for line in lines {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }

    fn get(&self, group: &str, key: &str) -> Option<String> {
        let (_, lines) = self.groups.iter().find(|(name, _)| name == group)?;
        lines.iter().find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| unescape(v))
        })
    }

    fn set(&mut self, group: &str, key: &str, value: &str) {
        let line = format!("{key}={}", escape(value));
        let index = match self.groups.iter().position(|(name, _)| name == group) {
            Some(i) => i,
            None => {
                self.groups.push((group.to_string(), Vec::new()));
                self.groups.len() - 1
            }
        };
        let lines = &mut self.groups[index].1;
        match lines.iter().position(|l| l.split_once('=').is_some_and(|(k, _)| k.trim() == key)) {
            Some(i) => lines[i] = line,
            None => lines.push(line),
        }
    }

    fn unset(&mut self, group: &str, key: &str) {
        if let Some((_, lines)) = self.groups.iter_mut().find(|(name, _)| name == group) {
            lines.retain(|l| !l.split_once('=').is_some_and(|(k, _)| k.trim() == key));
        }
    }

    fn remove_group(&mut self, group: &str) {
        self.groups.retain(|(name, _)| name != group);
    }
}

fn path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config")).join("kwinrulesrc")
}

fn load() -> Ini {
    Ini::parse(&fs::read_to_string(path()).unwrap_or_default())
}

fn store(ini: &Ini) -> Result<(), String> {
    let file = path();
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    // Written beside it and moved over, so a failure never leaves half a file.
    let temp = file.with_extension("tmp");
    fs::write(&temp, ini.render()).map_err(|e| e.to_string())?;
    fs::rename(&temp, &file).map_err(|e| e.to_string())
}

fn ids(ini: &Ini) -> Vec<String> {
    let list = ini.get("General", "rules").unwrap_or_default();
    list.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

fn set_ids(ini: &mut Ini, list: &[String]) {
    ini.set("General", "rules", &list.join(","));
    ini.set("General", "count", &list.len().to_string());
}

/// A group name is what KWin makes (a UUID) or a number of the old files.
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn new_id() -> Result<String, String> {
    let id = fs::read_to_string("/proc/sys/kernel/random/uuid").map_err(|e| e.to_string())?;
    Ok(id.trim().to_string())
}

fn number(ini: &Ini, group: &str, key: &str) -> u8 {
    ini.get(group, key).and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

fn read_rule(ini: &Ini, id: &str) -> Rule {
    let properties = SPECS
        .iter()
        .filter_map(|spec| {
            let action = number(ini, id, &format!("{}rule", spec.key));
            (action != 0).then(|| Property { key: spec.key.to_string(), value: ini.get(id, spec.key).unwrap_or_default(), action })
        })
        .collect();
    Rule {
        id: id.to_string(),
        // The old files call it `description`.
        description: ini.get(id, "Description").or_else(|| ini.get(id, "description")).unwrap_or_default(),
        enabled: ini.get(id, "Enabled").map(|v| v != "false").unwrap_or(true),
        wmclass: ini.get(id, "wmclass").unwrap_or_default(),
        wmclassmatch: number(ini, id, "wmclassmatch"),
        wmclasscomplete: ini.get(id, "wmclasscomplete").is_some_and(|v| v == "true"),
        title: ini.get(id, "title").unwrap_or_default(),
        titlematch: number(ini, id, "titlematch"),
        properties,
    }
}

fn book() -> RuleBook {
    let ini = load();
    RuleBook {
        rules: ids(&ini).iter().map(|id| read_rule(&ini, id)).collect(),
        specs: SPECS.iter().map(|s| SpecInfo { key: s.key.to_string(), kind: s.kind.to_string(), force: s.force }).collect(),
    }
}

// ── Checking what comes from the window ──────────────────────────────────────

fn pair(value: &str, low: i32, high: i32) -> bool {
    let parts: Vec<&str> = value.split(',').collect();
    parts.len() == 2 && parts.iter().all(|p| p.trim().parse::<i32>().is_ok_and(|n| (low..=high).contains(&n)))
}

fn value_fits(kind: &str, value: &str) -> bool {
    match kind {
        "bool" => value == "true" || value == "false",
        "point" => pair(value, -100_000, 100_000),
        "size" => pair(value, 1, 100_000),
        "screen" => value.parse::<i32>().is_ok_and(|n| (0..=64).contains(&n)),
        "percent" => value.parse::<i32>().is_ok_and(|n| (0..=100).contains(&n)),
        _ => false,
    }
}

fn check(rule: &Rule) -> Result<(), String> {
    if rule.wmclassmatch > 3 || rule.titlematch > 3 {
        return Err("the kind of match is not valid".to_string());
    }
    for property in &rule.properties {
        let spec = SPECS.iter().find(|s| s.key == property.key).ok_or("unknown property")?;
        let actions: &[u8] = if spec.force { &[1, 2, 6] } else { &[1, 2, 3, 4, 5, 6] };
        if !actions.contains(&property.action) || !value_fits(spec.kind, &property.value) {
            return Err(format!("the value of {} is not valid", property.key));
        }
    }
    Ok(())
}

// ── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command(async)]
pub fn list_rules() -> RuleBook {
    book()
}

/// Saves a rule (a new one when it has no id) and tells KWin.
#[tauri::command(async)]
pub fn save_rule(rule: Rule) -> Result<RuleBook, String> {
    check(&rule)?;
    let mut ini = load();
    let mut list = ids(&ini);
    let id = if rule.id.is_empty() {
        let id = new_id()?;
        list.push(id.clone());
        id
    } else if valid_id(&rule.id) && list.contains(&rule.id) {
        rule.id.clone()
    } else {
        return Err("that rule does not exist".to_string());
    };

    ini.set(&id, "Description", &rule.description);
    ini.unset(&id, "description");
    ini.set(&id, "Enabled", if rule.enabled { "true" } else { "false" });
    ini.set(&id, "wmclass", &rule.wmclass);
    ini.set(&id, "wmclassmatch", &rule.wmclassmatch.to_string());
    ini.set(&id, "wmclasscomplete", if rule.wmclasscomplete { "true" } else { "false" });
    ini.set(&id, "title", &rule.title);
    ini.set(&id, "titlematch", &rule.titlematch.to_string());
    for spec in SPECS {
        match rule.properties.iter().find(|p| p.key == spec.key) {
            Some(p) => {
                ini.set(&id, spec.key, &p.value);
                ini.set(&id, &format!("{}rule", spec.key), &p.action.to_string());
            }
            None => {
                ini.unset(&id, spec.key);
                ini.unset(&id, &format!("{}rule", spec.key));
            }
        }
    }
    set_ids(&mut ini, &list);
    store(&ini)?;
    reload_config()?;
    Ok(book())
}

#[tauri::command(async)]
pub fn remove_rule(id: String) -> Result<RuleBook, String> {
    let mut ini = load();
    let mut list = ids(&ini);
    if !valid_id(&id) || !list.contains(&id) {
        return Err("that rule does not exist".to_string());
    }
    list.retain(|i| *i != id);
    ini.remove_group(&id);
    set_ids(&mut ini, &list);
    store(&ini)?;
    reload_config()?;
    Ok(book())
}

#[tauri::command(async)]
pub fn set_rule_enabled(id: String, enabled: bool) -> Result<RuleBook, String> {
    let mut ini = load();
    if !valid_id(&id) || !ids(&ini).contains(&id) {
        return Err("that rule does not exist".to_string());
    }
    ini.set(&id, "Enabled", if enabled { "true" } else { "false" });
    store(&ini)?;
    reload_config()?;
    Ok(book())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> Rule {
        Rule {
            id: String::new(),
            description: "Keep Kate above".to_string(),
            enabled: true,
            wmclass: "kate".to_string(),
            wmclassmatch: 1,
            wmclasscomplete: false,
            title: String::new(),
            titlematch: 0,
            properties: vec![Property { key: "above".into(), value: "true".into(), action: 2 }, Property { key: "opacityactive".into(), value: "80".into(), action: 2 }],
        }
    }

    #[test]
    fn a_rule_is_written_and_read_back() {
        let mut ini = Ini::parse("[General]\ncount=0\nrules=\n\n[Other]\nkeep=me\n");
        ini.set("abc-1", "Description", "Keep Kate above");
        ini.set("abc-1", "above", "true");
        ini.set("abc-1", "aboverule", "2");
        set_ids(&mut ini, &["abc-1".to_string()]);
        let text = ini.render();
        assert!(text.contains("[Other]\nkeep=me"));
        let read = Ini::parse(&text);
        assert_eq!(ids(&read), vec!["abc-1".to_string()]);
        let r = read_rule(&read, "abc-1");
        assert_eq!(r.description, "Keep Kate above");
        assert_eq!(r.properties.len(), 1);
        assert_eq!((r.properties[0].key.as_str(), r.properties[0].action), ("above", 2));
    }

    #[test]
    fn a_group_is_removed_with_what_it_holds() {
        let mut ini = Ini::parse("[a]\nx=1\n[b]\ny=2\n");
        ini.remove_group("a");
        assert_eq!(ini.get("a", "x"), None);
        assert_eq!(ini.get("b", "y").as_deref(), Some("2"));
    }

    #[test]
    fn text_with_special_characters_survives() {
        let mut ini = Ini::parse("");
        ini.set("g", "t", " a\\b\nc ");
        assert_eq!(Ini::parse(&ini.render()).get("g", "t").as_deref(), Some(" a\\b\nc "));
    }

    #[test]
    fn values_and_actions_are_checked() {
        assert!(check(&rule()).is_ok());
        let mut bad = rule();
        bad.properties[1].action = 3; // opacity is a force kind: no "apply initially"
        assert!(check(&bad).is_err());
        let mut bad = rule();
        bad.properties[0].value = "yes".into();
        assert!(check(&bad).is_err());
        let mut bad = rule();
        bad.properties.push(Property { key: "evil".into(), value: "1".into(), action: 2 });
        assert!(check(&bad).is_err());
        assert!(valid_id("0f8c-AA") && !valid_id("../x") && !valid_id(""));
        assert!(value_fits("point", "10,-20") && !value_fits("size", "0,10"));
    }
}
