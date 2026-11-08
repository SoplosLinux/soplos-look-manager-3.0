// What Plasma can wear: global themes, start-up screens, icon themes, cursor
// themes and wallpapers. Each one is listed from the folders where Plasma looks
// for it, and applied with the tools of Plasma itself, so the desktop notices at
// once. Nothing here needs the administrator.

use super::login::{ini_get, ini_set};
use serde::Serialize;
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
    /// The path of a picture of it, to be read with `read_preview`.
    pub preview: Option<String>,
    pub active: bool,
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

fn has_command(name: &str) -> bool {
    std::env::var_os("PATH").map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file())).unwrap_or(false)
}

/// Runs a command and returns what it printed; the error is what it said.
fn run(program: &str, args: &[&str]) -> Result<String, String> {
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

fn kwrite(file: &str, group: &str, key: &str, value: &str) -> Result<(), String> {
    let tool = if has_command("kwriteconfig6") { "kwriteconfig6" } else { "kwriteconfig5" };
    run(tool, &["--file", file, "--group", group, "--key", key, value]).map(|_| ())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

fn folders(kind: &str) -> Vec<PathBuf> {
    let home = home();
    match kind {
        "themes" | "splash" => vec![PathBuf::from("/usr/share/plasma/look-and-feel"), home.join(".local/share/plasma/look-and-feel")],
        "icons" | "cursors" => vec![PathBuf::from("/usr/share/icons"), home.join(".local/share/icons"), home.join(".icons")],
        "gtk" => vec![PathBuf::from("/usr/share/themes"), home.join(".local/share/themes"), home.join(".themes")],
        "wallpapers" => vec![PathBuf::from("/usr/share/wallpapers"), home.join(".local/share/wallpapers")],
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

/// The name and the description of a package of Plasma, from its metadata.
fn metadata(dir: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = fs::read_to_string(dir.join("metadata.json")) else { return (None, None) };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return (None, None) };
    let plugin = &json["KPlugin"];
    (plugin["Name"].as_str().map(str::to_string), plugin["Description"].as_str().map(str::to_string))
}

/// A value of the `[Icon Theme]` group of an `index.theme`.
fn theme_key(index: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(index).ok()?;
    let mut in_group = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_group = line == "[Icon Theme]";
        } else if in_group {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == key {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

fn is_picture(path: &Path) -> bool {
    path.extension().map(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp")).unwrap_or(false)
}

/// The picture of a wallpaper: the biggest file of its `contents/images`, or
/// the picture that lies alone in its folder.
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

#[tauri::command(async)]
pub fn list_looks(kind: String) -> Result<Vec<Item>, String> {
    let mut items: Vec<Item> = Vec::new();
    match kind.as_str() {
        "themes" | "splash" => {
            let active = if kind == "themes" { kread("kdeglobals", "KDE", "LookAndFeelPackage") } else { kread("ksplashrc", "KSplash", "Theme") };
            for base in folders(&kind) {
                for dir in entries(&base) {
                    let id = file_name(&dir);
                    if kind == "splash" && !dir.join("contents/splash").is_dir() {
                        continue;
                    }
                    let (name, description) = metadata(&dir);
                    let previews = dir.join("contents/previews");
                    let preview = [if kind == "splash" { "splash.png" } else { "preview.png" }, "preview.png", "fullscreenpreview.jpg"]
                        .iter()
                        .map(|f| previews.join(f))
                        .find(|p| p.is_file());
                    items.push(Item {
                        active: active.as_deref() == Some(id.as_str()),
                        name: name.unwrap_or_else(|| id.clone()),
                        description: description.unwrap_or_default(),
                        preview: preview.map(|p| p.to_string_lossy().to_string()),
                        id,
                    });
                }
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
                    if !belongs || items.iter().any(|i| i.id == id) {
                        continue;
                    }
                    items.push(Item {
                        active: active.as_deref() == Some(id.as_str()) || (active.is_none() && kind == "cursors" && id == "breeze_cursors"),
                        name: theme_key(&index, "Name").unwrap_or_else(|| id.clone()),
                        description: theme_key(&index, "Comment").unwrap_or_default(),
                        preview: None,
                        id,
                    });
                }
            }
        }
        "gtk" => {
            let active = fs::read_to_string(gtk_settings(3)).ok().and_then(|t| ini_get(&t, "[Settings]", "gtk-theme-name"));
            for base in folders(&kind) {
                for dir in entries(&base) {
                    let id = file_name(&dir);
                    if !(dir.join("gtk-3.0").is_dir() || dir.join("gtk-4.0").is_dir()) || items.iter().any(|i| i.id == id) {
                        continue;
                    }
                    items.push(Item {
                        active: active.as_deref() == Some(id.as_str()),
                        name: theme_key_in(&dir.join("index.theme"), "[Desktop Entry]", "Name").unwrap_or_else(|| id.clone()),
                        description: String::new(),
                        preview: None,
                        id,
                    });
                }
            }
        }
        "wallpapers" => {
            for base in folders(&kind) {
                for dir in entries(&base) {
                    let Some(picture) = wallpaper_picture(&dir) else { continue };
                    let (name, _) = metadata(&dir);
                    let screenshot = dir.join("contents/screenshot.png");
                    let preview = if screenshot.is_file() { screenshot } else { picture.clone() };
                    items.push(Item {
                        id: picture.to_string_lossy().to_string(),
                        name: name.unwrap_or_else(|| file_name(&dir)),
                        description: String::new(),
                        preview: Some(preview.to_string_lossy().to_string()),
                        active: false,
                    });
                }
            }
        }
        other => return Err(format!("unknown kind: {other}")),
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(items)
}

fn base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() / 3 * 4 + 4);
    for chunk in data.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
    }
    out
}

/// A picture of a theme as a data URL, so the window can show it. Only the
/// pictures of the folders of Plasma and of the home folder are read, and not
/// the big ones.
#[tauri::command(async)]
pub fn read_preview(path: String) -> Option<String> {
    let path = fs::canonicalize(&path).ok()?;
    let allowed = path.starts_with("/usr/share") || path.starts_with(home());
    if !allowed || !is_picture(&path) {
        return None;
    }
    let size = fs::metadata(&path).ok()?.len();
    if size > 2_000_000 {
        return None;
    }
    let mime = match path.extension()?.to_string_lossy().to_lowercase().as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    let data = fs::read(&path).ok()?;
    Some(format!("data:{mime};base64,{}", base64(&data)))
}

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

/// A value of a group of an `index.theme`, for the groups that are not
/// `[Icon Theme]`.
fn theme_key_in(index: &Path, header: &str, key: &str) -> Option<String> {
    ini_get(&fs::read_to_string(index).ok()?, header, key)
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

#[tauri::command(async)]
pub fn apply_look(kind: String, id: String) -> Result<(), String> {
    match kind.as_str() {
        "themes" if valid_id(&id) => run("plasma-apply-lookandfeel", &["-a", &id]).map(|_| ()),
        "cursors" if valid_id(&id) => run("plasma-apply-cursortheme", &[&id]).map(|_| ()),
        "gtk" if valid_id(&id) => apply_gtk(&id),
        "splash" if valid_id(&id) => {
            kwrite("ksplashrc", "KSplash", "Theme", &id)?;
            kwrite("ksplashrc", "KSplash", "Engine", "KSplashQML")
        }
        "icons" if valid_id(&id) => match libexec("plasma-changeicons") {
            Some(tool) => run(&tool.to_string_lossy(), &[&id]).map(|_| ()),
            None => {
                kwrite("kdeglobals", "Icons", "Theme", &id)?;
                // Tells the programs that are open that the icons changed.
                let _ = run("dbus-send", &["--session", "--type=signal", "/KIconLoader", "org.kde.KIconLoader.iconChanged", "int32:0"]);
                Ok(())
            }
        },
        "wallpapers" => {
            let path = fs::canonicalize(&id).map_err(|e| e.to_string())?;
            if !(path.starts_with("/usr/share/wallpapers") || path.starts_with(home())) || !is_picture(&path) {
                return Err("not a wallpaper".to_string());
            }
            run("plasma-apply-wallpaperimage", &[&path.to_string_lossy()]).map(|_| ())
        }
        _ => Err(format!("cannot apply {kind} {id}")),
    }
}
