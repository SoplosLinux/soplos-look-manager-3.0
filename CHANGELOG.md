# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [3.0.0-alpha] - 2026-11-08

First version, for testing on Soplos Linux Tyson (Plasma). It has not yet been run on a Plasma desktop: report anything that fails.

### Added
- Soplos Look Manager 3.0 on Tauri 2, Svelte 4 and Tailwind 3: the theme and layout manager of Soplos Linux Tyson. It is a new program, with no 2.x version to port.
- Frameless card window with the sidebar, light and dark theme that follows the desktop, and the interface in 8 languages (en, es, de, fr, it, pt, ro, ru). Every page has a banner with the image of the program.
- Keyboard shortcuts: F1 for About, Ctrl+1 to Ctrl+8 for the pages, Ctrl+Tab and Ctrl+Shift+Tab for the next and the previous page, and Ctrl+Q to quit. The right button of the mouse is off in the installed program.
- Themes: lists the global themes of Plasma, with a preview, and applies one with `plasma-apply-lookandfeel`.
- Wallpapers: lists the wallpapers of the system and of the user, with a preview, and applies one with `plasma-apply-wallpaperimage`.
- Icons and Cursors: list the installed themes and apply them (`plasma-changeicons`, `plasma-apply-cursortheme`).
- GTK: lists the GTK themes, and sets the one chosen for GTK 3, GTK 4 and the settings of the desktop, linking the files of the theme into the folder of GTK 4 for libadwaita. A file of the user is never overwritten.
- Panel: lists the panels of Plasma and changes the one chosen (position, height, floating and when it hides). A panel can be added, with the launcher, the tasks, the system tray and the clock, and removed, never the last one.
- Start-up: lists the start-up screens and sets the one chosen.
- Login: sets the picture of the login screen of Plasma Login Manager, from the wallpapers installed, and whether NumLock is on at login. It asks for the administrator password through polkit (`org.soplos.lookmanager.run-privileged`); the program checks everything again as root, and only a picture of the wallpapers of the system or of the user who asked can be used.
- The package: launcher in 8 languages (only shown in Plasma), AppStream metadata, copyright file and the polkit action.
