# Soplos Look Manager 3.0

The theme and layout manager of Soplos Linux Tyson (Plasma). A new program built on Tauri 2, Svelte 4 and Tailwind 3, in the style of the other Soplos 3.0 programs.

> `3.0.0-alpha`: written for Plasma 6 but not yet run on a Plasma desktop. Report anything that fails.

## Pages

| Page | What it does |
|------|--------------|
| Themes | Lists the global themes of Plasma, with a preview, and applies one |
| Wallpapers | Lists the wallpapers of the system and of the user, and applies one |
| Icons | Icon themes |
| Cursors | Cursor themes |
| GTK | GTK themes, for GTK 3, GTK 4 and libadwaita |
| Panel | Position, height, floating and hiding of each panel; adds and removes panels |
| Start-up | Start-up screens |
| Login | Picture and NumLock of the login screen of Plasma Login Manager (asks for the administrator password) |

Shortcuts: F1 About, Ctrl+1 to Ctrl+8 the pages, Ctrl+Tab the next page, Ctrl+Q quit.

## Development

```bash
npm install
npx tauri icon ../soplos-artwork/soplos-look-manager/soplos-look-manager.png
npm run tauri dev
```

The development server uses port 1433.

## License

GPL-3.0-or-later. Copyright © 2026 Sergi Perich.
