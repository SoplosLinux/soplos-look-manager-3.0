// Installing what the pages list: from the store of KDE, and from a file.
//
// The store window is the one of KDE itself: `knewstuff-dialog6 <file>.knsrc` (package
// knewstuff-dialog6) opens the "Get new" window of KNewStuff for the configuration file
// that each kind of thing has in `/usr/share/knsrcfiles`, and installs where Plasma does.
//
// From a file, the program puts the file where Plasma looks for it:
// - the packages of Plasma and KWin (global themes, start-up screens, Plasma styles,
//   effects, scripts, window switchers) are installed with `kpackagetool6`, as System
//   Settings does;
// - the themes of icons, cursors, sounds, GTK and window decorations are archives that
//   are unpacked in the folder of the user that holds them;
// - a color scheme (`.colors`) and a wallpaper (a picture) are copied.
//
// Nothing runs through a shell: every command has its arguments apart.

use super::looks::{has_command, home, run};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// What is done with the file that was chosen.
enum Plan {
    /// A package of Plasma or KWin of this type, with `kpackagetool6`.
    Package(&'static str),
    /// An archive, unpacked in this folder of the user (relative to the home).
    Unpack(&'static str),
    /// A color scheme, copied to the folder of the color schemes.
    Colors,
    /// A picture, copied to a folder of its own in the folder of the wallpapers.
    Picture,
}

fn plan(kind: &str) -> Option<Plan> {
    Some(match kind {
        "themes" | "splash" => Plan::Package("Plasma/LookAndFeel"),
        "plasmastyle" => Plan::Package("Plasma/Theme"),
        "effects" => Plan::Package("KWin/Effect"),
        "scripts" => Plan::Package("KWin/Script"),
        "tabbox" => Plan::Package("KWin/WindowSwitcher"),
        "icons" | "cursors" => Plan::Unpack(".local/share/icons"),
        "sounds" => Plan::Unpack(".local/share/sounds"),
        "gtk" => Plan::Unpack(".local/share/themes"),
        "decorations" => Plan::Unpack(".local/share/aurorae/themes"),
        "colors" => Plan::Colors,
        "wallpapers" => Plan::Picture,
        _ => return None,
    })
}

/// The file of the store that each kind has (`/usr/share/knsrcfiles`), the same ones
/// that the pages of System Settings use.
fn store_file(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "themes" => "lookandfeel.knsrc",
        "splash" => "ksplash.knsrc",
        "plasmastyle" => "plasma-themes.knsrc",
        "colors" => "colorschemes.knsrc",
        "decorations" => "window-decorations.knsrc",
        "icons" => "icons.knsrc",
        "cursors" => "xcursor.knsrc",
        "wallpapers" => "wallpaper.knsrc",
        "gtk" => "gtk_themes.knsrc",
        "effects" => "kwineffect.knsrc",
        "scripts" => "kwinscripts.knsrc",
        "tabbox" => "kwinswitcher.knsrc",
        _ => return None,
    })
}

/// What can be installed here: whether there is a store for the kind and whether a
/// file can be installed, so the window shows only the buttons that do something.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOptions {
    pub store: bool,
    pub file: bool,
}

#[tauri::command(async)]
pub fn install_options(kind: String) -> InstallOptions {
    InstallOptions { store: store_file(&kind).is_some(), file: plan(&kind).is_some() }
}

/// Opens the store window of KDE for a kind and waits until it is closed.
#[tauri::command(async)]
pub fn get_new(kind: String) -> Result<(), String> {
    let file = store_file(&kind).ok_or("there is no store for this")?;
    let tool = if has_command("knewstuff-dialog6") { "knewstuff-dialog6" } else { return Err("knewstuff-dialog6 is not installed".to_string()) };
    Command::new(tool)
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())
        .map(|_| ())
}

// ── From a file ──────────────────────────────────────────────────────────────

/// The filter of the file chooser for each kind.
fn filter(plan: &Plan) -> &'static str {
    match plan {
        Plan::Colors => "Color schemes (*.colors)",
        Plan::Picture => "Pictures (*.png *.jpg *.jpeg *.webp)",
        _ => "Packages and archives (*.tar.gz *.tgz *.tar.xz *.txz *.tar.bz2 *.tbz2 *.tar.zst *.tar *.zip *.plasmoid *.kwinscript *.kwinoutline)",
    }
}

/// Asks which file, with the chooser of KDE (`kdialog`), or `zenity`. Nothing is
/// chosen when the window is closed.
fn choose_file(filter: &str) -> Result<Option<PathBuf>, String> {
    let start = home().to_string_lossy().to_string();
    let output = if has_command("kdialog") {
        Command::new("kdialog").args(["--getopenfilename", &start, filter]).stdin(Stdio::null()).output()
    } else if has_command("zenity") {
        Command::new("zenity").args(["--file-selection", "--filename", &format!("{start}/")]).stdin(Stdio::null()).output()
    } else {
        return Err("neither kdialog nor zenity is installed".to_string());
    }
    .map_err(|e| e.to_string())?;
    // A closed window is a code that is not zero and says nothing.
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || text.is_empty() {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(text)))
}

fn lower_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Unpacks an archive in a folder, by its extension.
fn unpack(file: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let name = lower_name(file);
    let (file, dest) = (file.to_string_lossy().to_string(), dest.to_string_lossy().to_string());
    if name.ends_with(".zip") {
        run("unzip", &["-q", "-o", &file, "-d", &dest]).map(|_| ())
    } else if [".tar", ".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2", ".tbz2", ".tar.zst"].iter().any(|e| name.ends_with(e)) {
        // `tar` does not follow names that leave the folder.
        run("tar", &["-xf", &file, "-C", &dest]).map(|_| ())
    } else {
        Err("this kind of file cannot be unpacked".to_string())
    }
}

/// Installs a file for a kind. Returns whether something was installed: it is not when
/// the chooser is closed with no file.
#[tauri::command(async)]
pub fn install_file(kind: String) -> Result<bool, String> {
    let plan = plan(&kind).ok_or("nothing can be installed here")?;
    let Some(file) = choose_file(filter(&plan))? else { return Ok(false) };
    if !file.is_file() {
        return Err("it is not a file".to_string());
    }
    let name = lower_name(&file);
    match plan {
        Plan::Package(kind) => {
            // `--upgrade` installs the package, or replaces it if it is already there.
            run("kpackagetool6", &["-t", kind, "-u", &file.to_string_lossy()])?;
        }
        Plan::Unpack(folder) => unpack(&file, &home().join(folder))?,
        Plan::Colors => {
            if !name.ends_with(".colors") {
                return Err("it is not a color scheme".to_string());
            }
            let dest = home().join(".local/share/color-schemes");
            fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            fs::copy(&file, dest.join(file.file_name().ok_or("no name")?)).map_err(|e| e.to_string())?;
        }
        Plan::Picture => {
            if ![".png", ".jpg", ".jpeg", ".webp"].iter().any(|e| name.ends_with(e)) {
                return Err("it is not a picture".to_string());
            }
            let stem: String = file.file_stem().unwrap_or_default().to_string_lossy().chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') { c } else { '_' }).collect();
            let dest = home().join(".local/share/wallpapers").join(stem.trim());
            fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            fs::copy(&file, dest.join(file.file_name().ok_or("no name")?)).map_err(|e| e.to_string())?;
        }
    }
    Ok(true)
}

// ── Removing ─────────────────────────────────────────────────────────────────

/// A name that is one step of a path: no separators, no `..`.
fn plain(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('/') && !name.contains('\0')
}

/// What removing a thing of a kind deletes. Only what is in the folders of the user
/// can be removed: what the system brings belongs to its packages.
fn user_path(kind: &str, id: &str) -> Option<PathBuf> {
    let home = home();
    let share = home.join(".local/share");
    let path = match kind {
        "themes" | "splash" if plain(id) => share.join("plasma/look-and-feel").join(id),
        "plasmastyle" if plain(id) => share.join("plasma/desktoptheme").join(id),
        "icons" | "cursors" if plain(id) => {
            let own = share.join("icons").join(id);
            if own.symlink_metadata().is_ok() { own } else { home.join(".icons").join(id) }
        }
        "gtk" if plain(id) => {
            let own = share.join("themes").join(id);
            if own.symlink_metadata().is_ok() { own } else { home.join(".themes").join(id) }
        }
        "sounds" if plain(id) => share.join("sounds").join(id),
        "colors" if plain(id) => share.join("color-schemes").join(format!("{id}.colors")),
        "effects" if plain(id) => share.join("kwin/effects").join(id),
        "scripts" if plain(id) => share.join("kwin/scripts").join(id),
        "tabbox" if plain(id) => share.join("kwin/tabbox").join(id),
        "decorations" => {
            // "library|theme": an Aurorae theme is a folder, a QML decoration a package.
            let (library, theme) = id.split_once('|')?;
            if library != "org.kde.kwin.aurorae" {
                return None;
            }
            match theme.strip_prefix("__aurorae__") {
                Some(rest) => {
                    let name = rest.split_once("__")?.1;
                    if !plain(name) {
                        return None;
                    }
                    share.join("aurorae/themes").join(name)
                }
                None if plain(theme) => share.join("kwin/decorations").join(theme),
                None => return None,
            }
        }
        "wallpapers" => {
            // The id is the path of the picture. What goes is the package that holds it,
            // or just the picture when it is loose.
            let base = fs::canonicalize(share.join("wallpapers")).ok()?;
            let picture = fs::canonicalize(id).ok()?;
            let inside = picture.strip_prefix(&base).ok()?;
            let first = inside.components().next()?;
            let top = base.join(first);
            if top == picture || top.join("metadata.json").is_file() || top.join("contents/images").is_dir() {
                top
            } else {
                picture
            }
        }
        _ => return None,
    };
    path.symlink_metadata().ok()?;
    Some(path)
}

/// The theme that the system brings, for the kinds that live in `/usr/share/icons`.
fn system_path(kind: &str, id: &str) -> Option<PathBuf> {
    if !matches!(kind, "icons" | "cursors") || !plain(id) || id == "hicolor" {
        return None;
    }
    let path = Path::new("/usr/share/icons").join(id);
    path.symlink_metadata().ok()?;
    Some(path)
}

/// The things of a kind that can be removed: what the user installed, and the icon and
/// cursor themes of the system (which ask for the administrator password).
#[tauri::command(async)]
pub fn removable_looks(kind: String, ids: Vec<String>) -> Vec<String> {
    ids.into_iter().filter(|id| user_path(&kind, id).is_some() || system_path(&kind, id).is_some()).collect()
}

#[tauri::command(async)]
pub fn remove_look(kind: String, id: String) -> Result<(), String> {
    let Some(path) = user_path(&kind, &id) else {
        // Not in the folders of the user: only an icon or cursor theme of the system.
        system_path(&kind, &id).ok_or("it cannot be removed")?;
        return super::login::remove_system_icons(&id);
    };
    let meta = path.symlink_metadata().map_err(|e| e.to_string())?;
    // A link is removed, never what it points to.
    if meta.is_dir() { fs::remove_dir_all(&path) } else { fs::remove_file(&path) }.map_err(|e| e.to_string())?;
    // A folder of loose pictures that was left empty goes too.
    if kind == "wallpapers" {
        if let Some(parent) = path.parent() {
            if parent != home().join(".local/share/wallpapers") && fs::read_dir(parent).is_ok_and(|mut d| d.next().is_none()) {
                let _ = fs::remove_dir(parent);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("soplos-lm-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn every_kind_with_a_plan_or_a_store_is_known() {
        assert!(plan("icons").is_some() && plan("effects").is_some() && plan("appstyle").is_none());
        assert_eq!(store_file("themes"), Some("lookandfeel.knsrc"));
        assert!(store_file("appstyle").is_none() && store_file("login").is_none());
    }

    #[test]
    fn unpacks_a_tar_and_a_zip() {
        let work = temp("unpack");
        let theme = work.join("src/MyIcons");
        fs::create_dir_all(&theme).unwrap();
        fs::write(theme.join("index.theme"), "[Icon Theme]\nName=My\n").unwrap();

        let tar = work.join("my.tar.gz");
        run("tar", &["-czf", &tar.to_string_lossy(), "-C", &work.join("src").to_string_lossy(), "MyIcons"]).unwrap();
        let out = work.join("out-tar");
        unpack(&tar, &out).unwrap();
        assert!(out.join("MyIcons/index.theme").is_file());

        if has_command("zip") {
            let zip = work.join("my.zip");
            run("sh", &["-c", &format!("cd '{}' && zip -qr '{}' MyIcons", work.join("src").display(), zip.display())]).unwrap();
            let out = work.join("out-zip");
            unpack(&zip, &out).unwrap();
            assert!(out.join("MyIcons/index.theme").is_file());
        }
        assert!(user_path("icons", "../x").is_none() && user_path("themes", "a/b").is_none() && user_path("decorations", "org.kde.breeze|Breeze").is_none());
        assert!(unpack(&work.join("notes.txt"), &work.join("out-txt")).is_err());
        let _ = fs::remove_dir_all(&work);
    }
}
