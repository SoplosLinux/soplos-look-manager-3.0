<script lang="ts">
  import { onMount } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Ban, Check, Palette, Image, Shapes, MousePointer2, Sparkles, LogIn, Paintbrush, Pipette, Layers, AppWindow, Frame, Volume2, Trash2 } from "lucide-svelte";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import { theme } from "../lib/theme";
  import InstallBar from "./InstallBar.svelte";
  import Modal from "./Modal.svelte";
  import RemoveDialog from "./RemoveDialog.svelte";

  /**
   * What the page lists: global themes, start-up screens, icons, cursors, GTK
   * themes or wallpapers; "login" lists the wallpapers for the login screen.
   */
  export let kind: "themes" | "colors" | "appstyle" | "plasmastyle" | "decorations" | "splash" | "icons" | "cursors" | "gtk" | "sounds" | "wallpapers" | "login";

  interface LoginState {
    image: string | null;
    installed: boolean;
  }

  // The colors a color scheme is drawn with, as the Colors page of System Settings does.
  interface SchemeColors {
    window: string;
    windowText: string;
    button: string;
    buttonText: string;
    base: string;
    text: string;
    link: string;
    visited: string;
    disabledText: string;
    selection: string;
    selectionText: string;
    selectionLink: string;
    selectionVisited: string;
    titleBar: string;
    titleBarText: string;
  }

  interface Item {
    id: string;
    name: string;
    description: string;
    preview: string | null;
    swatches: string[];
    scheme: SchemeColors | null;
    active: boolean;
  }

  const FALLBACK = {
    themes: Palette,
    colors: Pipette,
    appstyle: AppWindow,
    plasmastyle: Layers,
    decorations: Frame,
    splash: Sparkles,
    icons: Shapes,
    cursors: MousePointer2,
    gtk: Paintbrush,
    sounds: Volume2,
    wallpapers: Image,
    login: LogIn,
  };

  // The login screen: what the greeter has now, and what is asked for.
  let login: LoginState = { image: null, installed: true };

  let items: Item[] = [];
  // The pictures that show what an icon theme or a cursor theme looks like.
  let samples: Record<string, string[]> = {};
  // The small copies of the wallpapers, made once and kept in the cache.
  let thumbs: Record<string, string> = {};
  let selected: string | null = null;
  let loaded = false;
  let applying = false;
  let errorText = "";
  // What the user installed, which can be removed, and the one being asked about.
  let removable: string[] = [];
  let askRemove: Item | null = null;

  // The choices of System Settings when a global theme is applied.
  let askTheme: Item | null = null;
  let withAppearance = true;
  let withLayout = false;

  const src = (path: string) => convertFileSrc(path);

  async function load() {
    progress.set("busy");
    try {
      items = await api.invoke<Item[]>("list_looks", { kind: kind === "login" ? "wallpapers" : kind });
      if (kind === "login") {
        login = await api.invoke<LoginState>("get_login");
        // The greeter has its own copy of the picture, with the same file name.
        const name = login.image?.split("/").pop();
        items = items.map((i) => ({ ...i, active: !!name && i.id.split("/").pop() === name }));
      }
    } catch (error) {
      items = [];
      errorText = `${$_("looks.errors.read")}\n${error}`;
    }
    progress.set(null);
    loaded = true;
    try {
      removable = kind === "login" ? [] : await api.invoke<string[]>("removable_looks", { kind, ids: items.map((i) => i.id) });
    } catch {
      removable = [];
    }
    selected = items.find((i) => i.active)?.id ?? null;
    if (kind === "icons" || kind === "cursors") void loadSamples();
    if (kind === "wallpapers" || kind === "login") void loadThumbs();
    if (kind === "appstyle" || kind === "plasmastyle" || kind === "decorations") void loadStylePreviews();
  }

  // The controls of an application style, the frame and clock of a Plasma theme
  // and a window decoration are drawn by a helper that uses the libraries of
  // Plasma, once each, and kept in the cache.
  // They are drawn with the light or the dark colors of the program, and again when
  // the theme of the program changes.
  async function loadStylePreviews() {
    const variant = $theme;
    const lang = $locale;
    for (const item of items) {
      try {
        const drawn = await api.invoke<string | null>("style_preview", { kind, id: item.id, variant, name: item.name, lang });
        // The theme or the language may have changed while this one was being drawn.
        if (drawn && variant === $theme && lang === $locale) thumbs = { ...thumbs, [item.id]: drawn };
      } catch {
        // The card keeps its icon.
      }
    }
  }

  let shownTheme = $theme;
  let shownLocale = $locale;
  $: if ($theme !== shownTheme || $locale !== shownLocale) {
    shownTheme = $theme;
    shownLocale = $locale;
    if (loaded && (kind === "appstyle" || kind === "plasmastyle" || kind === "decorations")) void loadStylePreviews();
  }

  // A wallpaper weighs megabytes: the card shows a small copy of it, and shows
  // it as soon as it is ready.
  async function loadThumbs() {
    for (const item of items) {
      if (!item.preview) continue;
      try {
        const copy = await api.invoke<string | null>("make_thumbnail", { path: item.preview });
        if (copy) thumbs = { ...thumbs, [item.id]: copy };
      } catch {
        // The card keeps its icon.
      }
    }
  }

  // The picture a card shows: the small copy of a wallpaper, or the picture of
  // the theme itself.
  function picture(item: Item, copies: Record<string, string>): string | null {
    if (kind === "wallpapers" || kind === "login" || kind === "appstyle" || kind === "plasmastyle" || kind === "decorations") return copies[item.id] ?? null;
    // The kinds that have no picture of their own.
    if (!(kind === "themes" || kind === "splash" || kind === "gtk")) return null;
    // A GTK theme is drawn with its colors; the picture it brings is for the ones
    // whose colors cannot be read.
    if (kind === "gtk" && item.swatches.length === 4) return null;
    return item.preview;
  }

  // One theme at a time, so the list shows at once.
  async function loadSamples() {
    for (const item of items) {
      try {
        const found = await api.invoke<string[]>("theme_samples", { kind, id: item.id });
        samples = { ...samples, [item.id]: found };
      } catch {
        // The card keeps its icon.
      }
    }
  }

  onMount(load);

  // A global theme asks first what to apply, as System Settings does.
  function start(id: string | null = selected) {
    if (!id || applying) return;
    if (kind === "themes") {
      askTheme = items.find((i) => i.id === id) ?? null;
      withAppearance = true;
      withLayout = false;
      return;
    }
    void apply(id);
  }

  async function apply(id: string | null = selected) {
    if (applying) return;
    const isLogin = kind === "login";
    if (!id) return;
    applying = true;
    progress.set("busy");
    try {
      if (isLogin) {
        const changedPicture = id && !items.find((i) => i.id === id)?.active;
        await api.invoke("apply_login", { image: changedPicture ? id : null });
      } else if (id) {
        await api.invoke("apply_look", { kind, id, appearance: kind === "themes" ? withAppearance : null, layout: kind === "themes" ? withLayout : null });
      }
      if (id && (kind !== "themes" || withAppearance)) items = items.map((i) => ({ ...i, active: i.id === id }));
    } catch (error) {
      errorText = `${$_(isLogin ? "login.errors.apply" : "looks.errors.apply")}\n${error}`;
    }
    progress.set(null);
    applying = false;
  }

  function confirmTheme() {
    const id = askTheme?.id ?? null;
    askTheme = null;
    void apply(id);
  }

  // The kinds whose card shows a picture of the thing itself.
  $: pictured = kind === "themes" || kind === "splash" || kind === "wallpapers" || kind === "login" || kind === "gtk" || kind === "appstyle" || kind === "plasmastyle" || kind === "decorations";
</script>

<InstallBar {kind} onDone={load} />

{#if loaded && !items.length}
  <p class="mb-4 text-[0.85rem] text-text-muted">{$_("looks.none")}</p>
{/if}

{#if kind === "login"}
  {#if !login.installed}
    <p class="mb-4 text-[0.85rem] text-text-muted">{$_("login.notInstalled")}</p>
  {/if}
{/if}

<div class="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-3">
  {#each items as item (item.id)}
    <div class="relative group">
    <button
      class="w-full text-left rounded border bg-surface overflow-hidden transition-colors
        {selected === item.id ? 'border-primary' : 'border-border hover:border-primary/60'}"
      aria-pressed={selected === item.id}
      disabled={kind === "login" && !login.installed}
      on:click={() => {
        // A click on a card applies it: there is no Apply button. A global theme asks
        // first what to apply, and the login screen asks for the password.
        selected = item.id;
        if (kind === "login") void apply(item.id);
        else start(item.id);
      }}
    >
      <!-- The pictures of a theme are of the whole screen (16:9), so its box has the
           same shape and nothing of the panels is cut off. -->
      <div class="relative overflow-hidden {kind === 'themes' || kind === 'splash' || kind === 'colors' || kind === 'appstyle' || kind === 'plasmastyle' || kind === 'decorations' ? 'aspect-video' : 'h-[110px]'} bg-surface-2">
        {#if pictured && picture(item, thumbs)}
          <img src={src(picture(item, thumbs) ?? "")} alt="" loading="lazy" decoding="async" class="absolute inset-0 w-full h-full {kind === 'gtk' ? 'object-contain p-2' : 'object-cover'}" />
        {:else if kind === "icons" && samples[item.id]?.length}
          <div class="absolute inset-0 grid grid-cols-3 gap-1.5 p-2.5 place-items-center">
            {#each samples[item.id] as sample}
              <img src={src(sample)} alt="" loading="lazy" decoding="async" class="w-9 h-9 object-contain" />
            {/each}
          </div>
        {:else if kind === "cursors" && samples[item.id]?.length}
          <div class="absolute inset-0 flex items-center justify-center gap-4">
            {#each samples[item.id] as sample}
              <img src={src(sample)} alt="" class="h-9 w-auto object-contain" />
            {/each}
          </div>
        {:else if kind === "colors" && item.scheme}
          <!-- The thumbnail of the Colors page of System Settings: a window with a title
               bar, a label and a button, and a frame with normal, highlighted and
               disabled text, each with a link and a visited link. -->
          {@const s = item.scheme}
          <div class="absolute inset-2 flex flex-col text-[10px] leading-tight rounded-[3px] overflow-hidden shadow-[0_1px_5px_rgba(0,0,0,0.45)]" style="background: {s.window}; color: {s.windowText}; border: 1px solid {s.windowText}33">
            <div class="h-4 shrink-0 grid place-items-center" style="background: {s.titleBar}; color: {s.titleBarText}">{$_("looks.scheme.windowTitle")}</div>
            <div class="flex-1 flex flex-col gap-1 p-1 min-h-0">
              <div class="flex items-center justify-between gap-1">
                <span class="truncate">{$_("looks.scheme.windowText")}</span>
                <span class="px-2 py-[2px] rounded-sm shrink-0" style="background: {s.button}; color: {s.buttonText}; border: 1px solid {s.windowText}33">{$_("looks.scheme.button")}</span>
              </div>
              <div class="flex-1 flex flex-col min-h-0 rounded-sm overflow-hidden" style="background: {s.base}; color: {s.text}; border: 1px solid {s.text}4d">
                <div class="px-1.5 py-[3px] truncate">{$_("looks.scheme.normal")} <span style="color: {s.link}; text-decoration: underline">{$_("looks.scheme.link")}</span> <span style="color: {s.visited}; text-decoration: underline">{$_("looks.scheme.visited")}</span></div>
                <div class="px-1.5 py-[3px] truncate" style="background: {s.selection}; color: {s.selectionText}">{$_("looks.scheme.highlighted")} <span style="color: {s.selectionLink}; text-decoration: underline">{$_("looks.scheme.link")}</span> <span style="color: {s.selectionVisited}; text-decoration: underline">{$_("looks.scheme.visited")}</span></div>
                <div class="px-1.5 py-[3px] truncate" style="color: {s.disabledText}">{$_("looks.scheme.disabled")} <span style="text-decoration: underline">{$_("looks.scheme.link")}</span> <span style="text-decoration: underline">{$_("looks.scheme.visited")}</span></div>
              </div>
            </div>
          </div>
        {:else if (kind === "gtk" || kind === "colors" || kind === "plasmastyle") && item.swatches.length === 4}
          <!-- A small window painted with the colors of the theme. -->
          <div class="absolute inset-0 grid place-items-center p-3" style="background: {item.swatches[0]}">
            <div class="w-full h-full rounded-sm overflow-hidden" style="background: {item.swatches[1]}; border: 1px solid {item.swatches[3]}44">
              <div class="h-3.5" style="background: {item.swatches[0]}; border-bottom: 1px solid {item.swatches[3]}33"></div>
              <div class="p-2 flex flex-col gap-1.5">
                <div class="h-1.5 w-3/4 rounded-full" style="background: {item.swatches[3]}; opacity: 0.7"></div>
                <div class="h-1.5 w-1/2 rounded-full" style="background: {item.swatches[3]}; opacity: 0.4"></div>
                <div class="h-4 w-14 rounded-sm mt-1" style="background: {item.swatches[2]}"></div>
              </div>
            </div>
          </div>
        {:else}
          <div class="absolute inset-0 grid place-items-center text-text-muted">
            <svelte:component this={kind === "splash" && item.id === "None" ? Ban : FALLBACK[kind]} size={34} />
          </div>
        {/if}
        {#if item.active}
          <span class="absolute top-1.5 right-1.5 w-6 h-6 rounded-full bg-primary text-text-bright grid place-items-center" title={$_("looks.active")}>
            <Check size={14} />
          </span>
        {/if}
      </div>
      <div class="px-3 py-2">
        <div class="text-[0.86rem] truncate {item.active ? 'font-semibold text-text-bright' : 'text-text'}">{kind === "splash" && item.id === "None" ? $_("looks.splashNone") : item.name}</div>
      </div>
    </button>
    {#if removable.includes(item.id) && !item.active}
      <button
        class="absolute top-2 left-2 w-8 h-8 rounded-full bg-surface text-text-bright border border-border shadow-[0_1px_4px_rgba(0,0,0,0.5)] grid place-items-center hover:bg-danger hover:text-white hover:border-danger"
        title={$_("install.remove")}
        aria-label={$_("install.remove")}
        on:click={() => (askRemove = item)}
      >
        <Trash2 size={16} />
      </button>
    {/if}
    </div>
  {/each}
</div>

{#if askRemove}
  <RemoveDialog {kind} item={askRemove} onDone={load} onClose={() => (askRemove = null)} />
{/if}

{#if askTheme}
  <Modal
    variant="confirm"
    title={$_("looks.theme.title", { values: { name: askTheme.name } })}
    confirmLabel={$_("looks.apply")}
    confirmDisabled={!withAppearance && !withLayout}
    onConfirm={confirmTheme}
    onClose={() => (askTheme = null)}
  >
    <p class="mb-3">{$_("looks.theme.intro")}</p>
    <label class="flex items-center gap-2.5 mb-2 cursor-pointer">
      <input type="checkbox" class="accent-[#ff7d2e]" bind:checked={withAppearance} />
      <span>{$_("looks.theme.appearance")}</span>
    </label>
    <label class="flex items-center gap-2.5 cursor-pointer">
      <input type="checkbox" class="accent-[#ff7d2e]" bind:checked={withLayout} />
      <span>{$_("looks.theme.layout")}</span>
    </label>
    {#if withLayout}
      <p class="mt-3 text-[0.78rem] text-text-muted">{$_("looks.theme.layoutHint")}</p>
    {/if}
  </Modal>
{/if}

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} onClose={() => (errorText = "")}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{/if}
