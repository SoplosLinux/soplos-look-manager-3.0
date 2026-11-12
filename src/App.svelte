<script lang="ts">
  import { setContext, onMount, type ComponentType } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getVersion } from "@tauri-apps/api/app";
  import { _ } from "svelte-i18n";
  import { api } from "./lib/api";
  import { progress } from "./lib/progress";
  import Sidebar from "./components/Sidebar.svelte";
  import WindowControls from "./components/WindowControls.svelte";
  import AboutDialog from "./components/AboutDialog.svelte";
  import PageHero from "./components/PageHero.svelte";
  import Login from "./views/Login.svelte";
  import Themes from "./views/Themes.svelte";
  import Wallpapers from "./views/Wallpapers.svelte";
  import Icons from "./views/Icons.svelte";
  import Cursors from "./views/Cursors.svelte";
  import Gtk from "./views/Gtk.svelte";
  import Panel from "./views/Panel.svelte";
  import Splash from "./views/Splash.svelte";
  import Colors from "./views/Colors.svelte";
  import AppStyle from "./views/AppStyle.svelte";
  import PlasmaStyle from "./views/PlasmaStyle.svelte";
  import Decorations from "./views/Decorations.svelte";
  import Sounds from "./views/Sounds.svelte";
  import Desktops from "./views/Desktops.svelte";
  import Behavior from "./views/Behavior.svelte";
  import Rules from "./views/Rules.svelte";
  import Effects from "./views/Effects.svelte";
  import Scripts from "./views/Scripts.svelte";
  import Tabbox from "./views/Tabbox.svelte";
  import { Palette, Image, Shapes, MousePointer2, Paintbrush, PanelTop, Sparkles, LogIn, Pipette, Layers, AppWindow, Frame, Volume2, LayoutGrid, WandSparkles, FileCode2, Repeat, MousePointerClick, ListChecks } from "lucide-svelte";

  // The menu is in four groups, as the pages of Plasma are.
  $: tabs = [
    { id: "themes", group: "look", icon: Palette, label: $_("app.tabs.themes.label"), title: $_("app.tabs.themes.title") },
    { id: "colors", group: "look", icon: Pipette, label: $_("app.tabs.colors.label"), title: $_("app.tabs.colors.title") },
    { id: "appstyle", group: "look", icon: AppWindow, label: $_("app.tabs.appstyle.label"), title: $_("app.tabs.appstyle.title") },
    { id: "plasmastyle", group: "look", icon: Layers, label: $_("app.tabs.plasmastyle.label"), title: $_("app.tabs.plasmastyle.title") },
    { id: "decorations", group: "look", icon: Frame, label: $_("app.tabs.decorations.label"), title: $_("app.tabs.decorations.title") },
    { id: "icons", group: "look", icon: Shapes, label: $_("app.tabs.icons.label"), title: $_("app.tabs.icons.title") },
    { id: "cursors", group: "look", icon: MousePointer2, label: $_("app.tabs.cursors.label"), title: $_("app.tabs.cursors.title") },
    { id: "gtk", group: "look", icon: Paintbrush, label: $_("app.tabs.gtk.label"), title: $_("app.tabs.gtk.title") },
    { id: "sounds", group: "look", icon: Volume2, label: $_("app.tabs.sounds.label"), title: $_("app.tabs.sounds.title") },
    { id: "wallpapers", group: "desktop", icon: Image, label: $_("app.tabs.wallpapers.label"), title: $_("app.tabs.wallpapers.title") },
    { id: "panel", group: "desktop", icon: PanelTop, label: $_("app.tabs.panel.label"), title: $_("app.tabs.panel.title") },
    { id: "desktops", group: "windows", icon: LayoutGrid, label: $_("app.tabs.desktops.label"), title: $_("app.tabs.desktops.title") },
    { id: "behavior", group: "windows", icon: MousePointerClick, label: $_("app.tabs.behavior.label"), title: $_("app.tabs.behavior.title") },
    { id: "rules", group: "windows", icon: ListChecks, label: $_("app.tabs.rules.label"), title: $_("app.tabs.rules.title") },
    { id: "effects", group: "windows", icon: WandSparkles, label: $_("app.tabs.effects.label"), title: $_("app.tabs.effects.title") },
    { id: "scripts", group: "windows", icon: FileCode2, label: $_("app.tabs.scripts.label"), title: $_("app.tabs.scripts.title") },
    { id: "tabbox", group: "windows", icon: Repeat, label: $_("app.tabs.tabbox.label"), title: $_("app.tabs.tabbox.title") },
    { id: "splash", group: "startup", icon: Sparkles, label: $_("app.tabs.splash.label"), title: $_("app.tabs.splash.title") },
    { id: "login", group: "startup", icon: LogIn, label: $_("app.tabs.login.label"), title: $_("app.tabs.login.title") },
  ];

  const views: Record<string, ComponentType> = {
    themes: Themes,
    colors: Colors,
    appstyle: AppStyle,
    plasmastyle: PlasmaStyle,
    decorations: Decorations,
    wallpapers: Wallpapers,
    icons: Icons,
    cursors: Cursors,
    gtk: Gtk,
    sounds: Sounds,
    panel: Panel,
    desktops: Desktops,
    behavior: Behavior,
    rules: Rules,
    effects: Effects,
    scripts: Scripts,
    tabbox: Tabbox,
    splash: Splash,
    login: Login,
  };

  let active = "themes";
  let appVersion = "";
  let envInfo: { desktopEnvironment: string; displayProtocol: string; edition: string } | null = null;

  // With `decorations` off in tauri.conf.json (and `transparent` on) the system
  // draws no frame: the window is one card with rounded corners and the buttons
  // of the window float at its top right. With `decorations` on, nothing of
  // this shows, so that setting is the only switch between the two.
  let frameless = false;
  let maximized = false;

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        appVersion = await getVersion();
      } catch {
        appVersion = "";
      }
      try {
        envInfo = await api.invoke("get_environment_info");
      } catch {
        envInfo = null;
      }
      try {
        const win = getCurrentWindow();
        frameless = !(await win.isDecorated());
        if (frameless) {
          maximized = await win.isMaximized();
          unlisten = await win.onResized(async () => {
            maximized = await win.isMaximized();
          });
        }
      } catch {
        frameless = false;
      }
    })();
    return () => unlisten?.();
  });

  function select(id: string) {
    active = id;
    document.getElementById("main-scroll")?.scrollTo({ top: 0 });
  }

  setContext("navigate", select);
  setContext("openAbout", () => (aboutOpen = true));
  setContext("environment", () => envInfo);

  $: ActiveView = views[active];
  $: activeTab = tabs.find((t) => t.id === active)!;

  // F1 -> About, Ctrl+1..7 -> pages, Ctrl+Tab -> next page, Ctrl+Q -> quit.
  let aboutOpen = false;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "F1") {
      e.preventDefault();
      aboutOpen = true;
      return;
    }
    if (e.ctrlKey && (e.key === "q" || e.key === "Q")) {
      e.preventDefault();
      getCurrentWindow().close();
      return;
    }
    // Ctrl+Tab and Ctrl+Shift+Tab: the next and the previous page.
    if (e.ctrlKey && !e.altKey && e.key === "Tab") {
      e.preventDefault();
      const i = tabs.findIndex((t) => t.id === active);
      const next = (i + (e.shiftKey ? -1 : 1) + tabs.length) % tabs.length;
      select(tabs[next].id);
      return;
    }
    // Ctrl+1 to Ctrl+7: the pages, in the order of the sidebar.
    if (e.ctrlKey && !e.shiftKey && !e.altKey && /^[1-9]$/.test(e.key)) {
      const tab = tabs[Number(e.key) - 1];
      if (tab) {
        e.preventDefault();
        select(tab.id);
      }
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="relative flex h-screen overflow-hidden bg-dark {frameless && !maximized ? 'window-card' : ''}">
  <Sidebar {tabs} {active} onSelect={select} {appVersion} {envInfo} />

  {#if frameless}
    <div class="absolute right-2 top-[5px] z-40"><WindowControls {maximized} /></div>
  {/if}

  <div class="flex-1 flex flex-col overflow-hidden min-w-0">
    <!-- No page has a top bar: the strip over the content holds the window
         buttons and moves the window. -->
    {#if frameless}
      <div class="h-7 shrink-0" data-tauri-drag-region></div>
    {/if}
    <!-- The banner of the page stays where it is while the page scrolls under it. -->
    <div class="shrink-0 px-7 {frameless ? '' : 'pt-7'}">
      <PageHero title={activeTab.title} icon={activeTab.icon} />
    </div>
    <div id="main-scroll" class="content-scroll flex-1 overflow-y-auto px-7 pb-7">
      <svelte:component this={ActiveView} />
    </div>

    <div class="h-1.5 bg-surface-2 overflow-hidden shrink-0" class:invisible={$progress === null}>
      {#if typeof $progress === "number"}
        <div class="h-full bg-primary transition-[width]" style="width: {$progress * 100}%"></div>
      {:else if $progress === "busy"}
        <div class="h-full w-1/3 bg-primary progress-fill"></div>
      {/if}
    </div>
  </div>

  {#if aboutOpen}
    <AboutDialog onClose={() => (aboutOpen = false)} />
  {/if}
</div>
