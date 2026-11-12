<script lang="ts">
  import { onMount } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Check, Puzzle, Trash2 } from "lucide-svelte";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import InstallBar from "./InstallBar.svelte";
  import Modal from "./Modal.svelte";
  import RemoveDialog from "./RemoveDialog.svelte";
  import Switch from "./Switch.svelte";

  /**
   * What KWin has of a kind: the desktop effects and the scripts (each one is turned
   * on or off at once) and the layouts of the window switcher (one is chosen).
   */
  export let kind: "effects" | "scripts" | "tabbox";

  interface Item {
    id: string;
    name: string;
    description: string;
    category: string;
    icon: string | null;
    enabled: boolean;
  }

  const LIST = { effects: "list_kwin_effects", scripts: "list_kwin_scripts", tabbox: "list_window_switchers" };

  let items: Item[] = [];
  let loaded = false;
  let busy = false;
  let errorText = "";
  let removable: string[] = [];
  let askRemove: Item | null = null;

  const src = (path: string) => convertFileSrc(path);

  async function load() {
    progress.set("busy");
    try {
      items = await api.invoke<Item[]>(LIST[kind], { lang: $locale });
    } catch (error) {
      items = [];
      errorText = `${$_("looks.errors.read")}\n${error}`;
    }
    try {
      removable = await api.invoke<string[]>("removable_looks", { kind, ids: items.map((i) => i.id) });
    } catch {
      removable = [];
    }
    progress.set(null);
    loaded = true;
  }

  onMount(load);

  async function run(task: () => Promise<void>) {
    if (busy) return;
    busy = true;
    progress.set("busy");
    try {
      await task();
    } catch (error) {
      errorText = `${$_("looks.errors.apply")}\n${error}`;
      // What is shown is read again, since the change may not have been made.
      try {
        items = await api.invoke<Item[]>(LIST[kind], { lang: $locale });
      } catch {
        // The list stays as it was.
      }
    }
    progress.set(null);
    busy = false;
  }

  const toggle = (item: Item, enabled: boolean) =>
    run(async () => {
      await api.invoke(kind === "effects" ? "set_kwin_effect" : "set_kwin_script", { id: item.id, enabled });
      items = items.map((i) => (i.id === item.id ? { ...i, enabled } : i));
    });

  const choose = (item: Item) =>
    run(async () => {
      await api.invoke("set_window_switcher", { id: item.id });
      items = items.map((i) => ({ ...i, enabled: i.id === item.id }));
    });
</script>

<InstallBar {kind} onDone={load} />

{#if loaded && !items.length}
  <p class="text-[0.85rem] text-text-muted">{$_("looks.none")}</p>
{/if}

<ul class="flex flex-col gap-1.5">
  {#each items as item (item.id)}
    <li class="flex items-center gap-3 px-3.5 py-2.5 rounded border {kind === 'tabbox' && item.enabled ? 'border-primary' : 'border-border'} bg-surface">
      {#if item.icon}
        <img src={src(item.icon)} alt="" class="w-6 h-6 object-contain shrink-0" />
      {:else}
        <Puzzle size={20} class="text-text-muted shrink-0" />
      {/if}
      {#if kind === "tabbox"}
        <button class="flex-1 min-w-0 text-left disabled:opacity-60" disabled={busy} on:click={() => choose(item)}>
          <span class="block text-[0.88rem] truncate {item.enabled ? 'font-semibold text-text-bright' : 'text-text'}">{item.name}</span>
          {#if item.description}<span class="block text-[0.74rem] text-text-muted truncate">{item.description}</span>{/if}
        </button>
        {#if item.enabled}<Check size={16} class="text-primary shrink-0" />{/if}
      {:else}
        <span class="flex-1 min-w-0">
          <span class="block text-[0.88rem] text-text-bright truncate">{item.name}</span>
          {#if item.description}<span class="block text-[0.74rem] text-text-muted truncate">{item.description}</span>{/if}
        </span>
        <Switch checked={item.enabled} label={item.name} disabled={busy} on:change={(e) => toggle(item, e.detail)} />
      {/if}
      {#if removable.includes(item.id)}
        <button class="shrink-0 w-8 h-8 rounded grid place-items-center border border-border bg-surface-2 text-text-bright hover:bg-danger hover:text-white hover:border-danger" title={$_("install.remove")} aria-label={$_("install.remove")} on:click={() => (askRemove = item)}>
          <Trash2 size={16} />
        </button>
      {/if}
    </li>
  {/each}
</ul>

{#if askRemove}
  <RemoveDialog {kind} item={askRemove} onDone={load} onClose={() => (askRemove = null)} />
{/if}

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} onClose={() => (errorText = "")}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{/if}
