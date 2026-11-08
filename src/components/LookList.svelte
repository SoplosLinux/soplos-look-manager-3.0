<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Check, Palette, Image, Shapes, MousePointer2, Sparkles, LogIn, Paintbrush } from "lucide-svelte";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import Modal from "./Modal.svelte";
  import Switch from "./Switch.svelte";

  /**
   * What the page lists: global themes, start-up screens, icons, cursors or
   * wallpapers; "login" lists the wallpapers for the login screen.
   */
  export let kind: "themes" | "splash" | "icons" | "cursors" | "gtk" | "wallpapers" | "login";

  interface LoginState {
    image: string | null;
    numlock: boolean;
    installed: boolean;
  }

  interface Item {
    id: string;
    name: string;
    description: string;
    preview: string | null;
    active: boolean;
  }

  const FALLBACK = { themes: Palette, splash: Sparkles, icons: Shapes, cursors: MousePointer2, gtk: Paintbrush, wallpapers: Image, login: LogIn };

  // The login screen: what the greeter has now, and what is asked for.
  let login: LoginState = { image: null, numlock: false, installed: true };
  let numlock = false;

  let items: Item[] = [];
  let previews: Record<string, string> = {};
  let selected: string | null = null;
  let loaded = false;
  let applying = false;
  let errorText = "";

  async function load() {
    progress.set("busy");
    try {
      items = await api.invoke<Item[]>("list_looks", { kind: kind === "login" ? "wallpapers" : kind });
      if (kind === "login") {
        login = await api.invoke<LoginState>("get_login");
        numlock = login.numlock;
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
    selected = items.find((i) => i.active)?.id ?? null;
    void loadPreviews();
  }

  // The pictures come one at a time, so the list shows at once.
  async function loadPreviews() {
    for (const item of items) {
      if (!item.preview) continue;
      try {
        const url = await api.invoke<string | null>("read_preview", { path: item.preview });
        if (url) previews = { ...previews, [item.id]: url };
      } catch {
        // The card keeps its icon.
      }
    }
  }

  onMount(load);

  async function apply(id: string | null = selected) {
    if (applying) return;
    const isLogin = kind === "login";
    // The login screen can change only the NumLock key, with no picture chosen.
    if (!id && !(isLogin && numlock !== login.numlock)) return;
    applying = true;
    progress.set("busy");
    try {
      if (isLogin) {
        const changedPicture = id && !items.find((i) => i.id === id)?.active;
        await api.invoke("apply_login", { image: changedPicture ? id : null, numlock });
        login = { ...login, numlock };
      } else if (id) {
        await api.invoke("apply_look", { kind, id });
      }
      if (id) items = items.map((i) => ({ ...i, active: i.id === id }));
    } catch (error) {
      errorText = `${$_(isLogin ? "login.errors.apply" : "looks.errors.apply")}\n${error}`;
    }
    progress.set(null);
    applying = false;
  }

  $: current = items.find((i) => i.active);
  $: canApply = (!!selected || (kind === "login" && numlock !== login.numlock)) && !applying && login.installed;
</script>

<div class="flex items-center justify-between gap-3 mb-4">
  <span class="text-[0.8rem] text-text-muted">
    {#if loaded}
      {items.length ? $_("looks.status", { values: { n: items.length } }) : $_("looks.none")}
      {#if current && kind !== "wallpapers"} · {$_("looks.inUse", { values: { name: current.name } })}{/if}
    {/if}
  </span>
  <button
    class="px-5 py-2 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.85rem] font-semibold disabled:opacity-50 disabled:hover:bg-primary"
    disabled={!canApply}
    on:click={() => apply()}
  >
    {$_("looks.apply")}
  </button>
</div>

{#if kind === "login"}
  {#if !login.installed}
    <p class="mb-4 text-[0.85rem] text-text-muted">{$_("login.notInstalled")}</p>
  {/if}
  <div class="flex items-center justify-between gap-4 px-4 py-3 mb-4 rounded border border-border bg-surface">
    <span class="text-[0.9rem] text-text-bright">{$_("login.numlock")}</span>
    <Switch bind:checked={numlock} label={$_("login.numlock")} disabled={!login.installed || applying} />
  </div>
  <p class="mb-3 text-[0.76rem] text-text-muted">{$_("login.note")}</p>
{/if}

<div class="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-3">
  {#each items as item (item.id)}
    <button
      class="text-left rounded border bg-surface overflow-hidden transition-colors
        {selected === item.id ? 'border-primary' : 'border-border hover:border-primary/60'}"
      aria-pressed={selected === item.id}
      title={item.description}
      on:click={() => (selected = item.id)}
      on:dblclick={() => apply(item.id)}
    >
      <div class="relative h-[110px] bg-surface-2 grid place-items-center text-text-muted">
        {#if previews[item.id]}
          <img src={previews[item.id]} alt="" class="w-full h-full object-cover" />
        {:else}
          <svelte:component this={FALLBACK[kind]} size={34} />
        {/if}
        {#if item.active}
          <span class="absolute top-1.5 right-1.5 w-6 h-6 rounded-full bg-primary text-text-bright grid place-items-center" title={$_("looks.active")}>
            <Check size={14} />
          </span>
        {/if}
      </div>
      <div class="px-3 py-2">
        <div class="text-[0.86rem] truncate {item.active ? 'font-semibold text-text-bright' : 'text-text'}">{item.name}</div>
      </div>
    </button>
  {/each}
</div>

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} onClose={() => (errorText = "")}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{/if}
