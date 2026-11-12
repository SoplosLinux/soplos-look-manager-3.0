// What Plasma can wear: global themes, start-up screens, icon themes, cursor
// themes, GTK themes and wallpapers. Each one is listed from the folders where
// Plasma looks for it, and applied with the tools of Plasma itself, so the
// desktop notices at once. Nothing here needs the administrator.

use super::login::{ini_get, ini_set};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// What Plasma calls it: the name of its folder, or the path of the picture.
    pub id: String,
    pub name: String,
    pub description: String,
    /// The path of a picture of it; the window shows it through the asset protocol.
    pub preview: Option<String>,
    /// The main colors of a GTK theme, as CSS colors, for a small mock-up.
    pub swatches: Vec<String>,
    /// The colors a color scheme is drawn with, as the Colors page of System Settings draws it.
    pub scheme: Option<SchemeColors>,
    pub active: bool,
}

/// The colors of a color scheme that its thumbnail uses, as CSS colors.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemeColors {
    pub window: String,
    pub window_text: String,
    pub button: String,
    pub button_text: String,
    pub base: String,
    pub text: String,
    pub link: String,
    pub visited: String,
    pub disabled_text: String,
    pub selection: String,
    pub selection_text: String,
    pub selection_link: String,
    pub selection_visited: String,
    pub title_bar: String,
    pub title_bar_text: String,
}

impl Item {
    fn new(id: String, name: String) -> Item {
        Item { id, name, description: String::new(), preview: None, swatches: Vec::new(), scheme: None, active: false }
    }
}

pub(crate) fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

pub(crate) fn has_command(name: &str) -> bool {
    std::env::var_os("PATH").map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file())).unwrap_or(false)
}

/// Runs a command and returns what it printed; the error is what it said.
pub(crate) fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let said = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if said.is_empty() { format!("{program} failed") } else { said })
    }
}

/// A key of the settings of Plasma.
fn kread(file: &str, group: &str, key: &str) -> Option<String> {
    let tool = if has_command("kreadconfig6") { "kreadconfig6" } else { "kreadconfig5" };
    run(tool, &["--file", file, "--group", group, "--key", key]).ok().filter(|v| !v.is_empty())
}

pub(crate) fn kwrite(file: &str, group: &str, key: &str, value: &str) -> Result<(), String> {
    let tool = if has_command("kwriteconfig6") { "kwriteconfig6" } else { "kwriteconfig5" };
    run(tool, &["--file", file, "--group", group, "--key", key, value]).map(|_| ())
}

pub(crate) fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

fn folders(kind: &str) -> Vec<PathBuf> {
    let home = home();
    match kind {
        "themes" | "splash" => vec![PathBuf::from("/usr/share/plasma/look-and-feel"), home.join(".local/share/plasma/look-and-feel")],
        "icons" | "cursors" => vec![PathBuf::from("/usr/share/icons"), home.join(".local/share/icons"), home.join(".icons")],
        "gtk" => vec![PathBuf::from("/usr/share/themes"), home.join(".local/share/themes"), home.join(".themes")],
        // The folders where Plasma looks for wallpapers.
        "wallpapers" => vec![PathBuf::from("/usr/share/wallpapers"), home.join(".local/share/wallpapers")],
        "colors" => vec![PathBuf::from("/usr/share/color-schemes"), home.join(".local/share/color-schemes")],
        "plasmastyle" => vec![PathBuf::from("/usr/share/plasma/desktoptheme"), home.join(".local/share/plasma/desktoptheme")],
        "sounds" => vec![PathBuf::from("/usr/share/sounds"), home.join(".local/share/sounds")],
        "decorations" => vec![PathBuf::from("/usr/share/aurorae/themes"), home.join(".local/share/aurorae/themes")],
        // The decorations written in QML that KWin brings (Plastik) and the ones installed as packages.
        "qmldecorations" => vec![PathBuf::from("/usr/share/kwin/decorations"), home.join(".local/share/kwin/decorations")],
        _ => Vec::new(),
    }
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    found.sort();
    found
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// The real path of a file, with its links resolved. The protocol that shows the
/// files to the window does not follow the relative links of the icon themes,
/// and many icons are links.
fn real(path: &Path) -> String {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()).to_string_lossy().to_string()
}

fn file_stem(path: &Path) -> String {
    path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// The name and the description of a package of Plasma, from its metadata.
fn metadata(dir: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = fs::read_to_string(dir.join("metadata.json")) else { return (None, None) };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return (None, None) };
    let plugin = &json["KPlugin"];
    (plugin["Name"].as_str().map(str::to_string), plugin["Description"].as_str().map(str::to_string))
}

/// A value of the `[Icon Theme]` group of an `index.theme`.
fn theme_key(index: &Path, key: &str) -> Option<String> {
    ini_get(&fs::read_to_string(index).ok()?, "[Icon Theme]", key)
}

fn is_picture(path: &Path) -> bool {
    path.extension().map(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp")).unwrap_or(false)
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("zz"), 16) {
                out.push(value);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

// ── Wallpapers ───────────────────────────────────────────────────────────────

/// The picture of a wallpaper package: the biggest file of its `contents/images`.
fn wallpaper_picture(dir: &Path) -> Option<PathBuf> {
    let images = dir.join("contents/images");
    let from = if images.is_dir() { images } else { dir.to_path_buf() };
    fs::read_dir(from)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_picture(p))
        .max_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
}

/// The wallpaper of the first desktop, as Plasma keeps it.
fn current_wallpaper() -> Option<String> {
    let text = fs::read_to_string(home().join(".config/plasma-org.kde.plasma.desktop-appletsrc")).ok()?;
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line.ends_with("[Wallpaper][org.kde.image][General]");
        } else if inside {
            if let Some(value) = line.strip_prefix("Image=") {
                return Some(percent_decode(value.strip_prefix("file://").unwrap_or(value)));
            }
        }
    }
    None
}

/// Whether the wallpaper in use is this picture, or the package that holds it.
fn is_current(current: &Option<String>, id: &str) -> bool {
    match current {
        Some(c) => {
            let c = c.trim_end_matches('/');
            c == id || id.starts_with(&format!("{c}/"))
        }
        None => false,
    }
}

fn picture_item(path: &Path, name: String, current: &Option<String>) -> Item {
    let id = path.to_string_lossy().to_string();
    let mut item = Item::new(id.clone(), name);
    item.active = is_current(current, &id);
    item.preview = Some(id);
    item
}

/// Every wallpaper, one card for each picture: the packages of Plasma (a folder
/// with several sizes of one picture) are one, and a folder of loose pictures
/// gives one for each of them.
fn list_wallpapers() -> Vec<Item> {
    let current = current_wallpaper();
    let mut items = Vec::new();
    for base in folders("wallpapers") {
        let Ok(read) = fs::read_dir(&base) else { continue };
        let mut paths: Vec<PathBuf> = read.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if path.is_file() {
                if is_picture(&path) {
                    items.push(picture_item(&path, file_stem(&path), &current));
                }
                continue;
            }
            if !path.is_dir() {
                continue;
            }
            if path.join("metadata.json").is_file() || path.join("contents/images").is_dir() {
                let Some(picture) = wallpaper_picture(&path) else { continue };
                let (name, _) = metadata(&path);
                let screenshot = path.join("contents/screenshot.png");
                let mut item = picture_item(&picture, name.unwrap_or_else(|| file_name(&path)), &current);
                if screenshot.is_file() {
                    item.preview = Some(screenshot.to_string_lossy().to_string());
                }
                items.push(item);
            } else {
                let dir_name = file_name(&path);
                let Ok(inside) = fs::read_dir(&path) else { continue };
                let mut pictures: Vec<PathBuf> = inside.flatten().map(|e| e.path()).filter(|p| p.is_file() && is_picture(p)).collect();
                pictures.sort();
                for picture in pictures {
                    items.push(picture_item(&picture, format!("{dir_name} {}", file_stem(&picture)), &current));
                }
            }
        }
    }
    items
}

// ── GTK colors ───────────────────────────────────────────────────────────────

/// The `@define-color` lines of a stylesheet and of the ones it imports.
fn read_css(path: &Path, defs: &mut HashMap<String, String>, depth: u8) {
    let Ok(text) = fs::read_to_string(path) else { return };
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("@define-color") {
            let rest = rest.trim();
            if let Some((name, value)) = rest.split_once(char::is_whitespace) {
                defs.entry(name.to_string()).or_insert_with(|| value.trim().trim_end_matches(';').trim().to_string());
            }
        } else if line.starts_with("@import") && depth < 2 {
            // The file of `@import "x.css";` and of `@import url("x.css");`.
            let target = line
                .split(|c| c == '"' || c == '\'')
                .nth(1)
                .map(str::to_string)
                .or_else(|| line.split_once('(').and_then(|(_, rest)| rest.split_once(')')).map(|(t, _)| t.trim().to_string()));
            if let (Some(parent), Some(target)) = (path.parent(), target) {
                if !target.is_empty() && !target.starts_with("resource:") {
                    read_css(&parent.join(target), defs, depth + 1);
                }
            }
        }
    }
}

fn is_css_color(value: &str) -> bool {
    let value = value.trim();
    (value.starts_with('#') && matches!(value.len(), 4 | 7 | 9) && value[1..].chars().all(|c| c.is_ascii_hexdigit()))
        || ((value.starts_with("rgb(") || value.starts_with("rgba(")) && value.ends_with(')'))
}

fn resolve_color(defs: &HashMap<String, String>, name: &str, depth: u8) -> Option<String> {
    if depth > 5 {
        return None;
    }
    let value = defs.get(name)?;
    match value.strip_prefix('@') {
        Some(other) => resolve_color(defs, other, depth + 1),
        None if is_css_color(value) => Some(value.trim().to_string()),
        None => None,
    }
}

/// The window color, the color of the contents, the accent and the text of a GTK
/// theme, when its stylesheets say them as plain colors.
fn gtk_colors(dir: &Path) -> Vec<String> {
    let mut defs: HashMap<String, String> = HashMap::new();
    for rel in ["gtk-3.0/gtk.css", "gtk-4.0/gtk.css", "gtk-3.0/colors.css", "gtk-4.0/colors.css"] {
        read_css(&dir.join(rel), &mut defs, 0);
    }
    let pick = |names: &[&str]| names.iter().find_map(|n| resolve_color(&defs, n, 0));
    let bg = pick(&["window_bg_color", "theme_bg_color", "bg_color", "base_color"]);
    let accent = pick(&["accent_bg_color", "theme_selected_bg_color", "selected_bg_color", "accent_color"]);
    let (Some(bg), Some(accent)) = (bg, accent) else { return Vec::new() };
    let view = pick(&["view_bg_color", "theme_base_color", "base_color"]).unwrap_or_else(|| bg.clone());
    let fg = pick(&["window_fg_color", "theme_fg_color", "fg_color", "text_color"]).unwrap_or_else(|| "#888888".to_string());
    vec![bg, view, accent, fg]
}

/// The themes GTK has inside itself (Adwaita, Adwaita-dark and HighContrast)
/// leave a stylesheet that says it is no longer used, or that only imports the
/// one that is inside GTK.
fn is_builtin_stub(dir: &Path) -> bool {
    fs::read_to_string(dir.join("gtk-3.0/gtk.css")).is_ok_and(|text| text.len() < 400 && (text.contains("no longer used") || text.contains("resource://")))
}

/// The colors of those themes, as GTK 3 draws them, to the nearest the
/// stylesheets of GTK let us say: window, contents, accent and text.
fn builtin_palette(id: &str) -> Vec<String> {
    let id = id.to_lowercase();
    let colors: [&str; 4] = if id.contains("highcontrastinverse") {
        ["#000000", "#000000", "#ffffff", "#ffffff"]
    } else if id.contains("highcontrast") {
        ["#ffffff", "#ffffff", "#000000", "#000000"]
    } else if id.contains("dark") {
        ["#353535", "#2d2d2d", "#15539e", "#eeeeec"]
    } else {
        ["#f6f5f4", "#ffffff", "#3584e4", "#2e3436"]
    };
    colors.iter().map(|c| c.to_string()).collect()
}

/// Whether every theme a theme inherits from is installed; `hicolor` and
/// `default` are always there.
fn parents_installed(index: &Path) -> bool {
    let Some(inherits) = theme_key(index, "Inherits") else { return true };
    inherits
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty() && *name != "hicolor" && *name != "default")
        .all(|name| folders("cursors").iter().any(|base| base.join(name).is_dir()))
}

// ── Colors, Plasma styles, sounds, application styles and window decorations ──

/// The color schemes of Plasma, drawn with the colors they say: window, contents,
/// selection and text.
fn list_colors() -> Vec<Item> {
    let active = kread("kdeglobals", "General", "ColorScheme");
    let mut items: Vec<Item> = Vec::new();
    for base in folders("colors") {
        let Ok(read) = fs::read_dir(&base) else { continue };
        let mut files: Vec<PathBuf> = read.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "colors")).collect();
        files.sort();
        for file in files {
            let id = file_stem(&file);
            if items.iter().any(|i| i.id == id) {
                continue;
            }
            let text = fs::read_to_string(&file).unwrap_or_default();
            let mut item = Item::new(id.clone(), ini_get(&text, "[General]", "Name").unwrap_or_else(|| id.clone()));
            item.active = active.as_deref() == Some(id.as_str());
            item.swatches = color_swatches(&text);
            item.scheme = scheme_colors(&text);
            items.push(item);
        }
    }
    items
}

/// The colors of a scheme that the Colors page of System Settings uses for its
/// thumbnails (kcms/colors/ui/main.qml). A key that the scheme does not say takes
/// the value of Breeze, as KDE does.
fn scheme_colors(text: &str) -> Option<SchemeColors> {
    if !text.contains("[Colors:Window]") {
        return None;
    }
    let color = |group: &str, key: &str, default: &str| format!("rgb({})", ini_get(text, group, key).unwrap_or_else(|| default.to_string()).replace(' ', ""));
    Some(SchemeColors {
        window: color("[Colors:Window]", "BackgroundNormal", "239,240,241"),
        window_text: color("[Colors:Window]", "ForegroundNormal", "35,38,39"),
        button: color("[Colors:Button]", "BackgroundNormal", "239,240,241"),
        button_text: color("[Colors:Button]", "ForegroundNormal", "35,38,39"),
        base: color("[Colors:View]", "BackgroundNormal", "252,252,252"),
        text: color("[Colors:View]", "ForegroundNormal", "35,38,39"),
        link: color("[Colors:View]", "ForegroundLink", "41,128,185"),
        visited: color("[Colors:View]", "ForegroundVisited", "127,140,141"),
        disabled_text: color("[Colors:View]", "ForegroundInactive", "112,125,138"),
        selection: color("[Colors:Selection]", "BackgroundNormal", "61,174,233"),
        selection_text: color("[Colors:Selection]", "ForegroundNormal", "252,252,252"),
        selection_link: color("[Colors:Selection]", "ForegroundLink", "253,188,75"),
        selection_visited: color("[Colors:Selection]", "ForegroundVisited", "155,89,182"),
        title_bar: color("[WM]", "activeBackground", "71,80,87"),
        title_bar_text: color("[WM]", "activeForeground", "252,252,252"),
    })
}

/// Window, contents, selection and text colors of a `.colors` file; empty when
/// the file does not say all four.
fn color_swatches(text: &str) -> Vec<String> {
    // A key a scheme does not say takes the value of Breeze, as KDE does (the
    // Breeze themes of Plasma leave out the background of the selection).
    let color = |group: &str, key: &str, default: &str| format!("rgb({})", ini_get(text, group, key).unwrap_or_else(|| default.to_string()).replace(' ', ""));
    if !text.contains("[Colors:Window]") {
        return Vec::new();
    }
    vec![
        color("[Colors:Window]", "BackgroundNormal", "239,240,241"),
        color("[Colors:View]", "BackgroundNormal", "252,252,252"),
        color("[Colors:Selection]", "BackgroundNormal", "61,174,233"),
        color("[Colors:Window]", "ForegroundNormal", "35,38,39"),
    ]
}

/// The themes of the panel and the widgets of Plasma.
fn list_plasma_styles() -> Vec<Item> {
    let active = kread("plasmarc", "Theme", "name");
    let mut items: Vec<Item> = Vec::new();
    for base in folders("plasmastyle") {
        for dir in entries(&base) {
            let id = file_name(&dir);
            if items.iter().any(|i| i.id == id) {
                continue;
            }
            let (name, description) = metadata(&dir);
            let mut item = Item::new(id.clone(), name.unwrap_or_else(|| id.clone()));
            item.active = active.as_deref() == Some(id.as_str());
            item.description = description.unwrap_or_default();
            // The panel and the widgets take their colors from the `colors` file of the theme.
            if let Ok(text) = fs::read_to_string(dir.join("colors")) {
                item.swatches = color_swatches(&text);
            }
            items.push(item);
        }
    }
    items
}

/// The sound themes: the folders of `/usr/share/sounds` that say they are one.
fn list_sounds() -> Vec<Item> {
    // Plasma gives `ocean` when nothing is chosen.
    let active = kread("kdeglobals", "Sounds", "Theme").unwrap_or_else(|| "ocean".to_string());
    let mut items: Vec<Item> = Vec::new();
    for base in folders("sounds") {
        for dir in entries(&base) {
            let id = file_name(&dir);
            let Ok(text) = fs::read_to_string(dir.join("index.theme")) else { continue };
            if items.iter().any(|i| i.id == id) || ini_get(&text, "[Sound Theme]", "Hidden").as_deref() == Some("true") {
                continue;
            }
            let mut item = Item::new(id.clone(), ini_get(&text, "[Sound Theme]", "Name").unwrap_or_else(|| id.clone()));
            item.active = active == id;
            item.description = ini_get(&text, "[Sound Theme]", "Comment").unwrap_or_default();
            items.push(item);
        }
    }
    items
}

/// The folders of the plugins of Qt of this computer.
fn qt_plugin_dirs(sub: &str) -> Vec<PathBuf> {
    ["/usr/lib/x86_64-linux-gnu/qt6/plugins", "/usr/lib/aarch64-linux-gnu/qt6/plugins", "/usr/lib/qt6/plugins", "/usr/lib64/qt6/plugins"]
        .iter()
        .map(|d| Path::new(d).join(sub))
        .filter(|p| p.is_dir())
        .collect()
}

/// The names of the `.so` plugins of a folder, with no extension.
fn plugin_names(dir: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    let mut names: Vec<String> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "so"))
        .map(|p| file_stem(&p))
        .collect();
    names.sort();
    names
}

/// A name with its first letter in capitals.
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

/// The styles of the controls of the applications (Qt): the ones installed as
/// plugins, and the two that Qt brings itself.
fn list_app_styles() -> Vec<Item> {
    let active = kread("kdeglobals", "KDE", "widgetStyle").unwrap_or_else(|| "Breeze".to_string());
    let mut names: Vec<String> = vec!["Fusion".to_string(), "Windows".to_string()];
    for dir in qt_plugin_dirs("styles") {
        for stem in plugin_names(&dir) {
            // `breeze6` is the style Breeze; the other names go as they are.
            let plain = stem.trim_end_matches(|c: char| c.is_ascii_digit());
            let name = if plain.eq_ignore_ascii_case("breeze") { "Breeze".to_string() } else if plain.eq_ignore_ascii_case("kvantum") { "kvantum".to_string() } else { capitalized(plain) };
            if !name.is_empty() && !names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                names.push(name);
            }
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    names
        .into_iter()
        .map(|name| {
            let mut item = Item::new(name.clone(), name.clone());
            item.active = active.eq_ignore_ascii_case(&name);
            item
        })
        .collect()
}

/// The decorations of the windows: the plugins of KWin (Breeze) and the
/// themes of Aurorae. The id is "library|theme", as KWin keeps them.
fn list_decorations() -> Vec<Item> {
    let library = kread("kwinrc", "org.kde.kdecoration2", "library").unwrap_or_else(|| "org.kde.breeze".to_string());
    let theme = kread("kwinrc", "org.kde.kdecoration2", "theme").unwrap_or_else(|| "Breeze".to_string());
    let current = format!("{library}|{theme}");
    let mut items: Vec<Item> = Vec::new();
    for sub in ["org.kde.kdecoration3", "org.kde.kdecoration2"] {
        for dir in qt_plugin_dirs(sub) {
            for stem in plugin_names(&dir) {
                // A plugin that lists themes (the engines of Aurorae, `org.kde.kwin.aurorae`
                // and `org.kde.kwin.aurorae.v2`) is not a decoration: its themes are. KWin
                // skips them for the same reason, and applying one leaves the windows
                // with no decorations.
                if fs::read(dir.join(format!("{stem}.so"))).is_ok_and(|bytes| bytes.windows(16).any(|w| w == b"themeListKeyword")) {
                    continue;
                }
                let name = capitalized(stem.rsplit('.').next().unwrap_or(&stem));
                let id = format!("{stem}|{name}");
                if !items.iter().any(|i| i.id == id) {
                    let mut item = Item::new(id.clone(), name);
                    item.active = id == current;
                    items.push(item);
                }
            }
        }
    }
    for base in folders("decorations") {
        for dir in entries(&base) {
            let name = file_name(&dir);
            // KWin needs to know the kind of theme: a QML one has `main.qml`, an SVG one `decoration.svg`.
            let kind = if dir.join("contents/ui/main.qml").is_file() || dir.join("main.qml").is_file() {
                "qml"
            } else if dir.join("decoration.svg").is_file() || dir.join("decoration.svgz").is_file() {
                "svg"
            } else {
                continue;
            };
            let id = format!("org.kde.kwin.aurorae|__aurorae__{kind}__{name}");
            if !items.iter().any(|i| i.id == id) {
                let mut item = Item::new(id.clone(), name);
                item.active = id == current;
                items.push(item);
            }
        }
    }
    // The QML decorations are packages: KWin keeps their name as the theme (its own
    // default for Aurorae is `kwin4_decoration_qml_plastik`).
    for base in folders("qmldecorations") {
        for dir in entries(&base) {
            if !dir.join("contents/ui/main.qml").is_file() {
                continue;
            }
            let package = file_name(&dir);
            let id = format!("org.kde.kwin.aurorae|{package}");
            if !items.iter().any(|i| i.id == id) {
                let (name, description) = metadata(&dir);
                let mut item = Item::new(id.clone(), name.unwrap_or_else(|| package.clone()));
                item.description = description.unwrap_or_default();
                item.active = id == current;
                items.push(item);
            }
        }
    }
    items
}

// ── The lists ────────────────────────────────────────────────────────────────

#[tauri::command(async)]
pub fn list_looks(kind: String) -> Result<Vec<Item>, String> {
    let mut items: Vec<Item> = Vec::new();
    match kind.as_str() {
        "themes" | "splash" => {
            let active = if kind == "themes" { kread("kdeglobals", "KDE", "LookAndFeelPackage") } else { kread("ksplashrc", "KSplash", "Theme") };
            for base in folders(&kind) {
                for dir in entries(&base) {
                    let id = file_name(&dir);
                    if (kind == "splash" && !dir.join("contents/splash").is_dir()) || items.iter().any(|i| i.id == id) {
                        continue;
                    }
                    let (name, description) = metadata(&dir);
                    let previews = dir.join("contents/previews");
                    let preview = [if kind == "splash" { "splash.png" } else { "preview.png" }, "preview.png", "fullscreenpreview.jpg"]
                        .iter()
                        .map(|f| previews.join(f))
                        .find(|p| p.is_file());
                    let mut item = Item::new(id.clone(), name.unwrap_or_else(|| id.clone()));
                    item.active = active.as_deref() == Some(id.as_str());
                    item.description = description.unwrap_or_default();
                    item.preview = preview.map(|p| real(&p));
                    items.push(item);
                }
            }
            // The page of System Settings has a "None" entry (the theme `None`) for no
            // splash screen at all, and puts it last. The name is translated by the window.
            if kind == "splash" {
                let mut none = Item::new("None".to_string(), "None".to_string());
                none.active = active.as_deref() == Some("None");
                items.push(none);
            }
        }
        "icons" | "cursors" => {
            let active = if kind == "icons" { kread("kdeglobals", "Icons", "Theme") } else { kread("kcminputrc", "Mouse", "cursorTheme") };
            for base in folders(&kind) {
                for dir in entries(&base) {
                    let id = file_name(&dir);
                    let index = dir.join("index.theme");
                    let belongs = if kind == "icons" {
                        id != "hicolor" && theme_key(&index, "Directories").is_some() && theme_key(&index, "Hidden").as_deref() != Some("true")
                    } else {
                        dir.join("cursors").is_dir()
                    };
                    // A cursor theme whose parent is not installed cannot be applied
                    // (Adwaita needs AdwaitaLegacy), so it is not offered.
                    if !belongs || items.iter().any(|i| i.id == id) || (kind == "cursors" && !parents_installed(&index)) {
                        continue;
                    }
                    let mut item = Item::new(id.clone(), theme_key(&index, "Name").unwrap_or_else(|| id.clone()));
                    item.active = active.as_deref() == Some(id.as_str()) || (active.is_none() && kind == "cursors" && id == "breeze_cursors");
                    item.description = theme_key(&index, "Comment").unwrap_or_default();
                    items.push(item);
                }
            }
        }
        "gtk" => {
            let active = fs::read_to_string(gtk_settings(3)).ok().and_then(|t| ini_get(&t, "[Settings]", "gtk-theme-name"));
            for base in folders(&kind) {
                for dir in entries(&base) {
                    let id = file_name(&dir);
                    // Only a theme with a stylesheet looks like anything: the ones
                    // with only key bindings (Default, Emacs) are not offered.
                    if !(dir.join("gtk-3.0/gtk.css").is_file() || dir.join("gtk-4.0/gtk.css").is_file()) || items.iter().any(|i| i.id == id) {
                        continue;
                    }
                    let name = ini_get(&fs::read_to_string(dir.join("index.theme")).unwrap_or_default(), "[Desktop Entry]", "Name").unwrap_or_else(|| id.clone());
                    let mut item = Item::new(id.clone(), name);
                    item.active = active.as_deref() == Some(id.as_str());
                    // The picture the theme brings, when it brings one.
                    item.preview = ["thumbnail.png", "gtk-3.0/thumbnail.png", "gtk-4.0/thumbnail.png", "preview.png"]
                        .iter()
                        .map(|f| dir.join(f))
                        .find(|p| p.is_file())
                        .map(|p| real(&p));
                    item.swatches = gtk_colors(&dir);
                    if item.swatches.is_empty() && is_builtin_stub(&dir) {
                        item.swatches = builtin_palette(&id);
                    }
                    items.push(item);
                }
            }
        }
        "wallpapers" => items = list_wallpapers(),
        "colors" => items = list_colors(),
        "plasmastyle" => items = list_plasma_styles(),
        "sounds" => items = list_sounds(),
        "appstyle" => items = list_app_styles(),
        "decorations" => items = list_decorations(),
        other => return Err(format!("unknown kind: {other}")),
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    // "None" of the splash screens goes last, as in System Settings.
    if kind == "splash" {
        items.sort_by_key(|i| i.id == "None");
    }
    Ok(items)
}

// ── Samples of icon and cursor themes ────────────────────────────────────────

/// The folder of a theme and the ones it inherits from.
fn chain(kind: &str, id: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut queue = vec![id.to_string()];
    let mut seen: Vec<String> = Vec::new();
    while let Some(name) = queue.pop() {
        if seen.contains(&name) || seen.len() > 6 {
            continue;
        }
        seen.push(name.clone());
        for base in folders(kind) {
            let dir = base.join(&name);
            if dir.is_dir() {
                if let Some(inherits) = theme_key(&dir.join("index.theme"), "Inherits") {
                    for parent in inherits.split(',') {
                        queue.insert(0, parent.trim().to_string());
                    }
                }
                out.push(dir);
                break;
            }
        }
    }
    out
}

/// The icons that say what an icon theme looks like: a folder, the home, a
/// terminal, the settings, a browser and a text editor.
const ICON_SLOTS: [&[&str]; 6] = [
    &["folder"],
    &["user-home", "go-home"],
    &["utilities-terminal", "terminal"],
    &["preferences-system", "systemsettings"],
    &["internet-web-browser", "web-browser", "firefox"],
    &["accessories-text-editor", "text-editor"],
];

/// The files of icons of the icon theme in use, for a list of names of icons, so
/// the window can show them (it cannot look icons up in a theme by name). The
/// names that the theme and the themes it inherits from do not have are left out.
pub(crate) fn icon_files(names: &[String]) -> HashMap<String, String> {
    let theme = kread("kdeglobals", "Icons", "Theme").unwrap_or_else(|| "breeze".to_string());
    let wanted: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut best: HashMap<String, (i32, PathBuf)> = HashMap::new();
    for dir in chain("icons", &theme) {
        collect_icons(&dir, &wanted, 0, &mut best);
    }
    best.into_iter().map(|(name, (_, path))| (name, real(&path))).collect()
}

fn collect_icons(dir: &Path, wanted: &[&str], depth: u8, best: &mut HashMap<String, (i32, PathBuf)>) {
    if depth > 5 {
        return;
    }
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_icons(&path, wanted, depth + 1, best);
            continue;
        }
        let Some(ext) = path.extension().map(|e| e.to_string_lossy().to_lowercase()) else { continue };
        if ext != "svg" && ext != "png" {
            continue;
        }
        let stem = file_stem(&path);
        if !wanted.contains(&stem.as_str()) {
            continue;
        }
        // The vector icon, or the size that looks best in a small card.
        let text = path.to_string_lossy();
        let size = if text.contains("scalable") {
            100
        } else if text.contains("/48") {
            90
        } else if text.contains("/64") {
            85
        } else if text.contains("/32") {
            80
        } else if text.contains("/128") {
            70
        } else {
            50
        };
        let score = size + if ext == "svg" { 1 } else { 0 };
        if best.get(&stem).map_or(true, |(s, _)| score > *s) {
            best.insert(stem, (score, path.clone()));
        }
    }
}

const CURSOR_SLOTS: [&[&str]; 4] = [&["left_ptr", "default", "arrow"], &["pointer", "hand2", "hand1"], &["text", "xterm", "ibeam"], &["wait", "watch", "progress"]];

fn read_u32(data: &[u8], offset: usize) -> Result<u32, String> {
    data.get(offset..offset + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| "the cursor file is cut short".to_string())
}

/// Writes the picture of a cursor file of X (the Xcursor format) as a PNG: the
/// size nearest to 48 pixels, with its colors taken out of their alpha.
fn xcursor_to_png(source: &Path, dest: &Path) -> Result<(), String> {
    let data = fs::read(source).map_err(|e| e.to_string())?;
    if data.len() < 16 || &data[0..4] != b"Xcur" {
        return Err("not a cursor file".to_string());
    }
    let count = (read_u32(&data, 12)? as usize).min(1024);
    let mut best: Option<(u32, usize)> = None;
    for i in 0..count {
        let entry = 16 + i * 12;
        if read_u32(&data, entry)? != 0xfffd_0002 {
            continue;
        }
        let size = read_u32(&data, entry + 4)?;
        let position = read_u32(&data, entry + 8)? as usize;
        let distance = (size as i64 - 48).unsigned_abs() as u32;
        if best.map_or(true, |(d, _)| distance < d) {
            best = Some((distance, position));
        }
    }
    let position = best.ok_or_else(|| "no picture in the cursor".to_string())?.1;
    let width = read_u32(&data, position + 16)?;
    let height = read_u32(&data, position + 20)?;
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return Err("the cursor has a strange size".to_string());
    }
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for i in 0..(width * height) as usize {
        let pixel = read_u32(&data, position + 36 + i * 4)?;
        let (a, r, g, b) = ((pixel >> 24) & 255, (pixel >> 16) & 255, (pixel >> 8) & 255, pixel & 255);
        let straight = |c: u32| if a == 0 { 0 } else { (c * 255 / a).min(255) as u8 };
        rgba.extend_from_slice(&[straight(r), straight(g), straight(b), a as u8]);
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&rgba).map_err(|e| e.to_string())
}

/// A key that changes when a theme, or one that it inherits from, is installed
/// again or updated: the folders of the chain, with the time of the folder and of
/// its `index.theme`.
fn theme_stamp(kind: &str, id: &str) -> String {
    use std::time::UNIX_EPOCH;
    let seconds = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    chain(kind, id).iter().map(|dir| format!("{}:{}:{}", dir.display(), seconds(dir), seconds(&dir.join("index.theme")))).collect::<Vec<_>>().join("|")
}

/// The pictures that show what an icon theme or a cursor theme looks like, as
/// paths of files the window can show.
#[tauri::command(async)]
pub fn theme_samples(kind: String, id: String) -> Vec<String> {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    match kind.as_str() {
        "icons" => {
            // Looking for the six icons walks the folders of the theme, which is slow:
            // the list is kept in the cache, and read again only when the theme (or one
            // it inherits from) changes.
            let cache_file = home().join(".cache/soplos-look-manager/icon-samples").join(format!("{safe}.json"));
            let stamp = theme_stamp("icons", &id);
            if let Some(saved) = fs::read_to_string(&cache_file).ok().and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok()) {
                if saved["stamp"].as_str() == Some(stamp.as_str()) {
                    let paths: Vec<String> = saved["paths"].as_array().map(|list| list.iter().filter_map(|p| p.as_str().map(str::to_string)).collect()).unwrap_or_default();
                    if !paths.is_empty() && paths.iter().all(|p| Path::new(p).is_file()) {
                        return paths;
                    }
                }
            }

            let wanted: Vec<&str> = ICON_SLOTS.iter().flat_map(|slot| slot.iter().copied()).collect();
            let mut best: HashMap<String, (i32, PathBuf)> = HashMap::new();
            for dir in chain("icons", &id) {
                collect_icons(&dir, &wanted, 0, &mut best);
                if ICON_SLOTS.iter().all(|slot| slot.iter().any(|n| best.contains_key(*n))) {
                    break;
                }
            }
            let paths: Vec<String> = ICON_SLOTS
                .iter()
                .filter_map(|slot| slot.iter().find_map(|n| best.get(*n)).map(|(_, p)| real(p)))
                .collect();
            if let Some(parent) = cache_file.parent() {
                let _ = fs::create_dir_all(parent);
                let _ = fs::write(&cache_file, serde_json::json!({ "stamp": stamp, "paths": paths }).to_string());
            }
            paths
        }
        "cursors" => {
            let cache = home().join(".cache/soplos-look-manager/cursors").join(safe);
            // The pictures are made again when the theme changes.
            let stamp = theme_stamp("cursors", &id);
            let stamp_file = cache.join(".stamp");
            if fs::read_to_string(&stamp_file).ok().as_deref() != Some(stamp.as_str()) {
                let _ = fs::remove_dir_all(&cache);
                let _ = fs::create_dir_all(&cache);
                let _ = fs::write(&stamp_file, &stamp);
            }
            let dirs = chain("cursors", &id);
            let mut out = Vec::new();
            for slot in CURSOR_SLOTS {
                let dest = cache.join(format!("{}.png", slot[0]));
                if !dest.is_file() {
                    let source = dirs.iter().flat_map(|dir| slot.iter().map(move |name| dir.join("cursors").join(name))).find(|p| p.exists());
                    match source {
                        Some(source) if xcursor_to_png(&source, &dest).is_ok() => {}
                        _ => continue,
                    }
                }
                out.push(dest.to_string_lossy().to_string());
            }
            out
        }
        _ => Vec::new(),
    }
}

// ── Thumbnails of the wallpapers ─────────────────────────────────────────────

/// A small copy of a wallpaper, made once and kept in the cache, so the window
/// does not have to decode a picture of several megabytes for every card each
/// time it is shown. Returns the path of the copy.
#[tauri::command(async)]
pub fn make_thumbnail(path: String) -> Option<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let source = fs::canonicalize(&path).ok()?;
    if !(source.starts_with("/usr/share") || source.starts_with(home())) || !is_picture(&source) {
        return None;
    }
    let meta = fs::metadata(&source).ok()?;
    // The name changes when the picture does.
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    meta.len().hash(&mut hasher);
    meta.modified().ok().hash(&mut hasher);
    let dest = home().join(".cache/soplos-look-manager/thumbs").join(format!("{:016x}.jpg", hasher.finish()));
    if dest.is_file() {
        return Some(dest.to_string_lossy().to_string());
    }
    fs::create_dir_all(dest.parent()?).ok()?;
    let picture = gdk_pixbuf::Pixbuf::from_file_at_scale(&source, 400, -1, true).ok()?;
    picture.savev(&dest, "jpeg", &[("quality", "85")]).ok()?;
    Some(dest.to_string_lossy().to_string())
}

// ── Pictures drawn with the libraries of Plasma ──────────────────────────────

/// The helper that draws what the pages of System Settings draw with code of
/// their own (Plasma does not keep those pictures as files): the controls of an
/// application style, the frame and clock of a Plasma theme and a window
/// decoration, with the same libraries as those pages.
pub(crate) fn preview_helper() -> Option<PathBuf> {
    let name = "soplos-look-manager-style-preview";
    let beside = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join(name)));
    [Some(PathBuf::from("/usr/lib/soplos-look-manager").join(name)), beside].into_iter().flatten().find(|p| p.is_file())
}

/// A picture of an application style (`appstyle`), of a Plasma theme
/// (`plasmastyle`) or of a window decoration (`decorations`), drawn once by the
/// helper and kept in the cache. Returns the path of the picture, or nothing
/// when the helper is not there or fails, and the card keeps its icon.
#[tauri::command(async)]
pub fn style_preview(kind: String, id: String, variant: Option<String>, name: Option<String>, lang: Option<String>) -> Option<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    // A decoration is "plugin|theme"; the other kinds are one name.
    let decoration = if kind == "decorations" { id.split_once('|') } else { None };
    let id_ok = match decoration {
        Some((plugin, theme)) => valid_id(plugin) && valid_id(theme),
        None => kind != "decorations" && valid_id(&id),
    };
    if !id_ok {
        return None;
    }
    let helper = preview_helper()?;
    let modified = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok().unwrap_or(SystemTime::UNIX_EPOCH);

    // What the picture is made from, to draw it again when any of it changes. The
    // color scheme in use is part of all of them: they are drawn with its colors.
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    id.hash(&mut hasher);
    modified(&helper).hash(&mut hasher);
    modified(&home().join(".config/kdeglobals")).hash(&mut hasher);

    // The pictures follow the light or the dark theme of the program: they are
    // drawn with the Breeze Light or the Breeze Dark color scheme. With no variant,
    // or when the scheme is not installed, the color scheme in use is the one.
    let scheme = variant.as_deref().and_then(|v| match v {
        "light" => Some("BreezeLight.colors"),
        "dark" => Some("BreezeDark.colors"),
        _ => None,
    }).and_then(|file| folders("colors").into_iter().map(|base| base.join(file)).find(|p| p.is_file()));
    scheme.hash(&mut hasher);
    if let Some(path) = &scheme {
        modified(path).hash(&mut hasher);
    }

    // The texts of the controls are in the language of the program (two letters).
    let lang: String = lang.unwrap_or_default().chars().filter(|c| c.is_ascii_alphabetic()).take(2).collect();
    lang.hash(&mut hasher);

    let args: Vec<String> = match kind.as_str() {
        "appstyle" => {
            for dir in qt_plugin_dirs("styles") {
                modified(&dir).hash(&mut hasher);
            }
            vec!["style".to_string(), id.clone()]
        }
        "plasmastyle" => {
            let dir = folders("plasmastyle").into_iter().map(|base| base.join(&id)).find(|d| d.is_dir())?;
            modified(&dir).hash(&mut hasher);
            vec!["plasma".to_string(), id.clone()]
        }
        "decorations" => {
            let (plugin, theme) = decoration?;
            for sub in ["org.kde.kdecoration3", "org.kde.kdecoration2"] {
                for dir in qt_plugin_dirs(sub) {
                    modified(&dir.join(format!("{plugin}.so"))).hash(&mut hasher);
                }
            }
            for base in folders("decorations").into_iter().chain(folders("qmldecorations")) {
                modified(&base).hash(&mut hasher);
            }
            // The windows of the picture carry the name that the list shows.
            let caption: String = name.unwrap_or_default().chars().take(40).collect();
            caption.hash(&mut hasher);
            vec!["decoration".to_string(), plugin.to_string(), theme.to_string(), caption]
        }
        _ => return None,
    };

    let dest = home().join(".cache/soplos-look-manager/thumbs").join(format!("style-{:016x}.png", hasher.finish()));
    if dest.is_file() {
        return Some(dest.to_string_lossy().to_string());
    }
    fs::create_dir_all(dest.parent()?).ok()?;
    let mut command = Command::new(helper);
    command.args(&args).arg(&dest).stderr(Stdio::null());
    if let Some(path) = &scheme {
        command.env("SOPLOS_PREVIEW_SCHEME", path);
    }
    if !lang.is_empty() {
        command.env("SOPLOS_PREVIEW_LANG", &lang);
    }
    command.status().ok().filter(|status| status.success())?;
    dest.is_file().then(|| dest.to_string_lossy().to_string())
}

// ── Applying ─────────────────────────────────────────────────────────────────

/// A tool of Plasma that lives in one of the folders of libexec.
fn libexec(name: &str) -> Option<PathBuf> {
    ["/usr/libexec", "/usr/lib/libexec", "/usr/lib/x86_64-linux-gnu/libexec", "/usr/lib/aarch64-linux-gnu/libexec"]
        .iter()
        .map(|d| Path::new(d).join(name))
        .find(|p| p.is_file())
}

/// The settings file of GTK 3 or GTK 4 of the user.
fn gtk_settings(version: u8) -> PathBuf {
    home().join(format!(".config/gtk-{version}.0/settings.ini"))
}

/// The folder of a GTK theme, the user's own first.
fn gtk_theme_dir(id: &str) -> Option<PathBuf> {
    [home().join(".themes"), home().join(".local/share/themes"), PathBuf::from("/usr/share/themes")]
        .into_iter()
        .map(|base| base.join(id))
        .find(|dir| dir.join("gtk-3.0").is_dir() || dir.join("gtk-4.0").is_dir())
}

/// Sets the theme of GTK 3, GTK 4 and the settings of the desktop. libadwaita
/// (the GTK 4 of GNOME) ignores the name of the theme, so the files of the theme
/// are linked into the folder of GTK 4, as GNOME Tweaks and Plasma do.
fn apply_gtk(id: &str) -> Result<(), String> {
    let dir = gtk_theme_dir(id).ok_or_else(|| format!("{id} is not a GTK theme"))?;
    for version in [3u8, 4] {
        let file = gtk_settings(version);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = fs::read_to_string(&file).unwrap_or_default();
        fs::write(&file, ini_set(&text, "[Settings]", "gtk-theme-name", id)).map_err(|e| e.to_string())?;
    }

    // The links of GTK 4: only the ones this program made before are replaced;
    // a file of the user is never overwritten.
    let target = home().join(".config/gtk-4.0");
    let source = dir.join("gtk-4.0");
    for name in ["gtk.css", "gtk-dark.css", "assets"] {
        let link = target.join(name);
        if fs::symlink_metadata(&link).is_ok_and(|m| m.file_type().is_symlink()) {
            let _ = fs::remove_file(&link);
        }
        let from = source.join(name);
        if from.exists() && fs::symlink_metadata(&link).is_err() {
            std::os::unix::fs::symlink(&from, &link).map_err(|e| e.to_string())?;
        }
    }

    // The programs that read the settings of the desktop instead of the files.
    let _ = run("gsettings", &["set", "org.gnome.desktop.interface", "gtk-theme", id]);
    Ok(())
}

/// Puts a picture as the wallpaper of every desktop. It is what the layout
/// scripts of the themes of Soplos do: the picture goes in `Image` and in
/// `PreviewImage`, and the config is loaded again, because without that Plasma
/// keeps the new value and goes on showing the old picture (which is what
/// `plasma-apply-wallpaperimage` leaves).
fn set_wallpaper(path: &str) -> Result<(), String> {
    // The path goes inside a script, between quotes.
    if path.chars().any(|c| matches!(c, '"' | '\'' | '\\' | '\n' | '\r')) {
        return Err("the name of the picture has characters that cannot be used".to_string());
    }
    let Some(tool) = qdbus() else {
        return run("plasma-apply-wallpaperimage", &[path]).map(|_| ());
    };
    let script = format!(
        r#"var d = desktops(); for (var i = 0; i < d.length; i++) {{ d[i].wallpaperPlugin = "org.kde.image"; d[i].currentConfigGroup = ["Wallpaper", "org.kde.image", "General"]; d[i].writeConfig("Image", "file://{path}"); d[i].writeConfig("PreviewImage", "file://{path}"); d[i].reloadConfig(); }}"#
    );
    run(tool, &["org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.evaluateScript", &script]).map(|_| ())
}

/// The tool of Plasma that talks to D-Bus.
fn qdbus() -> Option<&'static str> {
    ["qdbus6", "qdbus-qt6", "qdbus"].into_iter().find(|t| has_command(t))
}

/// The GTK theme, set as Plasma sets it: through the service `org.kde.GtkConfig`
/// of kde-gtk-config, which keeps the files of GTK 3 and GTK 4 itself (the
/// interface is in `kcms/style/org.kde.GtkConfig.xml` of plasma-workspace).
fn apply_gtk_through_plasma(id: &str) -> Result<(), String> {
    let tool = qdbus().ok_or_else(|| "qdbus was not found".to_string())?;
    run(tool, &["org.kde.GtkConfig", "/GtkConfig", "org.kde.GtkConfig.setGtkTheme", id]).map(|_| ())
}

/// The decoration of the windows: the library and the theme KWin keeps, and a
/// reconfigure so that the windows that are open take it.
fn apply_decoration(id: &str) -> Result<(), String> {
    let (library, theme) = id.split_once('|').ok_or_else(|| "not a decoration".to_string())?;
    let plain = |s: &str| !s.is_empty() && s.len() <= 160 && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ' | '+'));
    if !plain(library) || !plain(theme) {
        return Err("not a decoration".to_string());
    }
    kwrite("kwinrc", "org.kde.kdecoration2", "library", library)?;
    kwrite("kwinrc", "org.kde.kdecoration2", "theme", theme)?;
    if let Some(tool) = qdbus() {
        let _ = run(tool, &["org.kde.KWin", "/KWin", "org.kde.KWin.reconfigure"]);
    }
    Ok(())
}

/// Puts the panels and the desktop of a global theme, with no other change.
fn load_layout(id: &str) -> Result<(), String> {
    let tool = ["qdbus6", "qdbus-qt6", "qdbus"].into_iter().find(|t| has_command(t)).ok_or_else(|| "qdbus was not found".to_string())?;
    run(tool, &["org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.loadLookAndFeelDefaultLayout", id]).map(|_| ())
}

/// Applies something. A global theme can be applied in two parts, as System
/// Settings does: its appearance, and the layout of the desktop and the panels.
#[tauri::command(async)]
pub fn apply_look(kind: String, id: String, appearance: Option<bool>, layout: Option<bool>) -> Result<(), String> {
    match kind.as_str() {
        "themes" if valid_id(&id) => match (appearance.unwrap_or(true), layout.unwrap_or(false)) {
            (true, false) => run("plasma-apply-lookandfeel", &["-a", &id]).map(|_| ()),
            (true, true) => run("plasma-apply-lookandfeel", &["-a", &id, "--resetLayout"]).map(|_| ()),
            (false, true) => load_layout(&id),
            (false, false) => Err("nothing to apply".to_string()),
        },
        "cursors" if valid_id(&id) => run("plasma-apply-cursortheme", &[&id]).map(|_| ()),
        "colors" if valid_id(&id) => run("plasma-apply-colorscheme", &[&id]).map(|_| ()),
        "plasmastyle" if valid_id(&id) => run("plasma-apply-desktoptheme", &[&id]).map(|_| ()),
        "sounds" if valid_id(&id) => kwrite("kdeglobals", "Sounds", "Theme", &id),
        "appstyle" if valid_id(&id) => {
            kwrite("kdeglobals", "KDE", "widgetStyle", &id)?;
            // Tells the programs that are open that the style changed (StyleChanged = 2).
            let _ = run("dbus-send", &["--session", "--type=signal", "/KGlobalSettings", "org.kde.KGlobalSettings.notifyChange", "int32:2", "int32:0"]);
            Ok(())
        }
        "decorations" => apply_decoration(&id),
        "gtk" if valid_id(&id) => apply_gtk_through_plasma(&id).or_else(|_| apply_gtk(&id)),
        "splash" if valid_id(&id) => {
            kwrite("ksplashrc", "KSplash", "Theme", &id)?;
            // As the page of System Settings saves it: the theme `None` is the engine
            // `none`, no splash screen; any other is shown by `KSplashQML`.
            kwrite("ksplashrc", "KSplash", "Engine", if id == "None" { "none" } else { "KSplashQML" })
        }
        "icons" if valid_id(&id) => {
            match libexec("plasma-changeicons") {
                Some(tool) => run(&tool.to_string_lossy(), &[&id]).map(|_| ())?,
                None => kwrite("kdeglobals", "Icons", "Theme", &id)?,
            }
            // What the Icons page of System Settings does when the theme changes: the
            // cache of icons is thrown away and every group of icons is announced as
            // changed, which makes the programs that are open read the theme again.
            // Announcing only one group left the others with the old icons.
            let _ = fs::remove_file(home().join(".cache/icon-cache.kcache"));
            for group in 0..6 {
                let _ = run("dbus-send", &["--session", "--type=signal", "/KIconLoader", "org.kde.KIconLoader.iconChanged", &format!("int32:{group}")]);
            }
            Ok(())
        }
        "wallpapers" => {
            let path = fs::canonicalize(&id).map_err(|e| e.to_string())?;
            if !(path.starts_with("/usr/share/wallpapers") || path.starts_with(home())) || !is_picture(&path) {
                return Err("not a wallpaper".to_string());
            }
            set_wallpaper(&path.to_string_lossy())
        }
        _ => Err(format!("cannot apply {kind} {id}")),
    }
}
