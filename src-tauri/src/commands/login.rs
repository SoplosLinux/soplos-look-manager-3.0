// The login screen of Plasma Login Manager: the picture behind the greeter. The greeter runs as the `plasmalogin` user before any
// session exists, so what it reads lives in /etc and /var/lib/plasmalogin and
// only the administrator can write it.
//
// The window never runs as root and never sends a script. It writes a request
// with a named operation, and pkexec starts this same program again as
// `--run-ops <file>` under the polkit action of the package. That second run
// never opens a window: it checks everything again as root, because the file
// can be written by any user, and only then does the work.

use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

const CONF: &str = "/etc/plasmalogin.conf";
const GREETER_HOME: &str = "/var/lib/plasmalogin";
const WALLPAPERS: &str = "/var/lib/plasmalogin/wallpapers";
/// The group of the greeter's wallpaper, as the module of System Settings writes it.
const GREETER_GROUP: &str = "[Greeter][Wallpaper][org.kde.image][General]";
const BIGGEST_PICTURE: u64 = 40_000_000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginState {
    /// The picture the greeter shows now, as the path of its file.
    pub image: Option<String>,
    /// Whether Plasma Login Manager is on this computer.
    pub installed: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Op {
    /// Sets the picture of the greeter.
    WriteLogin { image: Option<String> },
    /// Removes an icon or cursor theme that the system brings, from `/usr/share/icons`.
    RemoveSystemIcons { id: String },
}

#[derive(Serialize, Deserialize)]
struct Request {
    ops: Vec<Op>,
}

// ── Text of the settings files ───────────────────────────────────────────────

/// The value of a key inside a group of an ini text.
pub(crate) fn ini_get(text: &str, header: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == header;
        } else if inside {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == key {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

/// The same text with a key set inside a group; the group and the key are made
/// when they are not there, and every other line stays as it was.
pub(crate) fn ini_set(text: &str, header: &str, key: &str, value: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut inside = false;
    let mut found_group = false;
    let mut done = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            // Leaving the group without having found the key: it goes at its end.
            if inside && !done {
                out.push(format!("{key}={value}"));
                done = true;
            }
            inside = trimmed == header;
            found_group |= inside;
        } else if inside && !done {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == key {
                    out.push(format!("{key}={value}"));
                    done = true;
                    continue;
                }
            }
        }
        out.push(line.to_string());
    }
    if inside && !done {
        out.push(format!("{key}={value}"));
        done = true;
    }
    if !found_group {
        if out.last().is_some_and(|l| !l.is_empty()) {
            out.push(String::new());
        }
        out.push(header.to_string());
        out.push(format!("{key}={value}"));
    }
    let _ = done;
    let mut joined = out.join("\n");
    joined.push('\n');
    joined
}

// ── User side ────────────────────────────────────────────────────────────────

#[tauri::command(async)]
pub fn get_login() -> LoginState {
    let conf = fs::read_to_string(CONF).unwrap_or_default();
    let image = ini_get(&conf, GREETER_GROUP, "Image").map(|v| v.strip_prefix("file://").unwrap_or(&v).to_string());
    let installed = Path::new(CONF).exists() || Path::new(GREETER_HOME).is_dir() || Path::new("/usr/bin/plasmalogin").exists();
    LoginState { image, installed }
}

fn cache_dir(home: &Path) -> PathBuf {
    home.join(".cache").join("soplos-look-manager")
}

/// Asks pkexec to write the login screen as root. One call is one password.
#[tauri::command(async)]
pub fn apply_login(image: Option<String>) -> Result<(), String> {
    ask_root(vec![Op::WriteLogin { image }])
}

/// Removes an icon or cursor theme of the system, as the administrator.
pub(crate) fn remove_system_icons(id: &str) -> Result<(), String> {
    ask_root(vec![Op::RemoveSystemIcons { id: id.to_string() }])
}

fn ask_root(ops: Vec<Op>) -> Result<(), String> {
    let home = PathBuf::from(std::env::var("HOME").map_err(|e| e.to_string())?);
    let dir = cache_dir(&home);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("request-{}.json", std::process::id()));

    let request = serde_json::to_string(&Request { ops }).map_err(|e| e.to_string())?;
    {
        use std::io::Write;
        let mut out = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&file).map_err(|e| e.to_string())?;
        out.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = Command::new("pkexec").arg(exe).arg("--run-ops").arg(&file).status();
    let _ = fs::remove_file(&file);

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("The administrator step did not finish".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

// ── Root side ────────────────────────────────────────────────────────────────

// The home folder of a user, read from the password file.
fn home_of(uid: u32) -> Option<PathBuf> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let fields: Vec<&str> = line.split(':').collect();
        (fields.len() >= 6 && fields[2].parse::<u32>().ok() == Some(uid)).then(|| PathBuf::from(fields[5]))
    })
}

fn give_to_greeter(path: &Path) {
    let _ = Command::new("chown").arg("plasmalogin:plasmalogin").arg(path).status();
}

fn is_picture(path: &Path) -> bool {
    path.extension().map(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp")).unwrap_or(false)
}

fn set_picture(image: &str, uid: u32) -> Result<(), String> {
    let source = Path::new(image).canonicalize().map_err(|e| e.to_string())?;
    let home = home_of(uid).ok_or("Unknown user")?;
    // Only a picture of the wallpapers of the system or of the user who asked.
    let allowed = source.starts_with("/usr/share/wallpapers") || source.starts_with("/usr/share/backgrounds") || source.starts_with(&home);
    let meta = fs::metadata(&source).map_err(|e| e.to_string())?;
    if !allowed || !meta.is_file() || !is_picture(&source) || meta.len() > BIGGEST_PICTURE {
        return Err("It is not a picture that can be used".to_string());
    }
    // A plain name, so that nothing can walk out of the folder of the greeter.
    let stem: String = source.file_stem().unwrap_or_default().to_string_lossy().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let ext = source.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    let dest = Path::new(WALLPAPERS).join(format!("{stem}.{ext}"));

    fs::create_dir_all(WALLPAPERS).map_err(|e| e.to_string())?;
    give_to_greeter(Path::new(WALLPAPERS));
    fs::copy(&source, &dest).map_err(|e| e.to_string())?;
    fs::set_permissions(&dest, fs::Permissions::from_mode(0o644)).map_err(|e| e.to_string())?;
    give_to_greeter(&dest);

    let conf = fs::read_to_string(CONF).unwrap_or_default();
    fs::write(CONF, ini_set(&conf, GREETER_GROUP, "Image", &format!("file://{}", dest.display()))).map_err(|e| e.to_string())
}

/// Removes `/usr/share/icons/<id>`: a plain name, never the fallback theme, and a link
/// is removed without touching what it points to.
fn remove_icons(id: &str) -> Result<(), String> {
    if id.is_empty() || id == "." || id == ".." || id == "hicolor" || id.contains('/') || id.contains('\0') {
        return Err("It is not a theme that can be removed".to_string());
    }
    let path = Path::new("/usr/share/icons").join(id);
    let meta = path.symlink_metadata().map_err(|e| e.to_string())?;
    if meta.is_dir() { fs::remove_dir_all(&path) } else { fs::remove_file(&path) }.map_err(|e| e.to_string())
}

fn run_op(op: &Op, uid: u32) -> Result<(), String> {
    match op {
        Op::RemoveSystemIcons { id } => remove_icons(id),
        Op::WriteLogin { image } => {
            if let Some(image) = image {
                set_picture(image, uid)?;
            }
            Ok(())
        }
    }
}

/// Called from main() before Tauri starts when pkexec starts the program again.
/// Returns the exit code.
pub fn run_ops_file(path: &str) -> i32 {
    let uid = match std::env::var("PKEXEC_UID").ok().and_then(|v| v.parse::<u32>().ok()) {
        Some(uid) => uid,
        None => {
            eprintln!("This mode is only for pkexec");
            return 2;
        }
    };
    let result = (|| -> Result<(), String> {
        let cache = home_of(uid).map(|h| cache_dir(&h)).ok_or("Unknown user")?.canonicalize().map_err(|e| e.to_string())?;
        let file = Path::new(path).canonicalize().map_err(|e| e.to_string())?;
        let meta = fs::metadata(&file).map_err(|e| e.to_string())?;
        // The request has to be a file of the user who asked, in their cache.
        if !file.starts_with(&cache) || !meta.is_file() || meta.uid() != uid {
            return Err("The request is not valid".to_string());
        }
        let request: Request = serde_json::from_str(&fs::read_to_string(&file).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        for op in &request.ops {
            run_op(op, uid)?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_set_inside_its_group() {
        let text = "[A]\nx=1\n[Greeter][Wallpaper][org.kde.image][General]\nImage=file:///old.jpg\nOther=2\n[B]\ny=3\n";
        let out = ini_set(text, GREETER_GROUP, "Image", "file:///new.jpg");
        assert_eq!(ini_get(&out, GREETER_GROUP, "Image").as_deref(), Some("file:///new.jpg"));
        assert_eq!(ini_get(&out, GREETER_GROUP, "Other").as_deref(), Some("2"));
        assert_eq!(ini_get(&out, "[A]", "x").as_deref(), Some("1"));
        assert_eq!(ini_get(&out, "[B]", "y").as_deref(), Some("3"));
    }

    #[test]
    fn the_group_and_the_key_are_made_when_missing() {
        let out = ini_set("", "[Keyboard]", "NumLock", "0");
        assert_eq!(out, "[Keyboard]\nNumLock=0\n");
        let out = ini_set("[Keyboard]\nRepeat=1\n", "[Keyboard]", "NumLock", "1");
        assert_eq!(ini_get(&out, "[Keyboard]", "NumLock").as_deref(), Some("1"));
        assert_eq!(ini_get(&out, "[Keyboard]", "Repeat").as_deref(), Some("1"));
    }
}
