<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Plus, Pencil, Trash2, Check, X } from "lucide-svelte";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import Modal from "../components/Modal.svelte";
  import Switch from "../components/Switch.svelte";
  import NumberField from "../components/NumberField.svelte";

  interface Desktop {
    id: string;
    name: string;
  }
  interface Desktops {
    desktops: Desktop[];
    rows: number;
    wrap: boolean;
  }

  let state: Desktops = { desktops: [], rows: 1, wrap: false };
  let loaded = false;
  let busy = false;
  let errorText = "";
  // The desktop that is being renamed, and the name being typed.
  let editing: string | null = null;
  let typed = "";

  onMount(async () => {
    progress.set("busy");
    try {
      state = await api.invoke<Desktops>("list_desktops");
    } catch (error) {
      errorText = `${$_("desktops.errors.read")}\n${error}`;
    }
    progress.set(null);
    loaded = true;
  });

  // One change at a time; KWin answers with the desktops as they are now.
  async function change(command: string, args: Record<string, unknown>) {
    if (busy) return;
    busy = true;
    progress.set("busy");
    try {
      state = await api.invoke<Desktops>(command, args);
    } catch (error) {
      errorText = `${$_("desktops.errors.apply")}\n${error}`;
      try {
        state = await api.invoke<Desktops>("list_desktops");
      } catch {
        // What is shown stays as it was.
      }
    }
    progress.set(null);
    busy = false;
  }

  const add = () => change("add_desktop", { name: $_("desktops.name", { values: { n: state.desktops.length + 1 } }) });

  function startRename(desktop: Desktop) {
    editing = desktop.id;
    typed = desktop.name;
  }

  async function saveRename() {
    const id = editing;
    editing = null;
    if (id && typed.trim()) await change("rename_desktop", { id, name: typed.trim() });
  }

  $: off = !loaded || busy;
</script>

<div class="flex flex-col gap-4">
  <div class="flex items-center justify-between gap-3 flex-wrap">
    <div class="flex items-center gap-5 flex-wrap">
      <div class="flex items-center gap-3">
        <span class="text-[0.86rem] text-text-bright">{$_("desktops.rows")}</span>
        <NumberField value={state.rows} min={1} max={Math.max(1, state.desktops.length)} label={$_("desktops.rows")} disabled={off} on:change={(e) => change("set_desktop_rows", { rows: e.detail })} />
      </div>
      <div class="flex items-center gap-3">
        <span class="text-[0.86rem] text-text-bright">{$_("desktops.wrap")}</span>
        <Switch checked={state.wrap} label={$_("desktops.wrap")} disabled={off} on:change={(e) => change("set_desktop_wrap", { wrap: e.detail })} />
      </div>
    </div>
    <button
      class="flex items-center gap-1.5 px-3.5 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.84rem] font-semibold disabled:opacity-50"
      disabled={off}
      on:click={add}
    >
      <Plus size={14} />{$_("desktops.add")}
    </button>
  </div>

  <ul class="flex flex-col gap-1.5">
    {#each state.desktops as desktop, i (desktop.id)}
      <li class="flex items-center gap-3 px-3.5 py-2.5 rounded border border-border bg-surface">
        <span class="w-5 text-[0.74rem] text-text-muted text-right">{i + 1}</span>
        {#if editing === desktop.id}
          <input
            class="flex-1 min-w-0 px-2 py-1 rounded border border-primary bg-surface-2 text-[0.88rem] text-text-bright outline-none"
            bind:value={typed}
            maxlength="60"
            on:keydown={(e) => (e.key === "Enter" ? saveRename() : e.key === "Escape" ? (editing = null) : null)}
            autofocus
          />
          <button class="p-1.5 rounded text-primary hover:bg-surface-2" title={$_("common.ok")} aria-label={$_("common.ok")} on:click={saveRename}><Check size={15} /></button>
          <button class="p-1.5 rounded text-text-muted hover:bg-surface-2" title={$_("common.cancel")} aria-label={$_("common.cancel")} on:click={() => (editing = null)}><X size={15} /></button>
        {:else}
          <span class="flex-1 min-w-0 truncate text-[0.88rem] text-text-bright">{desktop.name}</span>
          <button class="p-1.5 rounded text-text-muted hover:text-text-bright hover:bg-surface-2 disabled:opacity-40" title={$_("desktops.rename")} aria-label={$_("desktops.rename")} disabled={off} on:click={() => startRename(desktop)}>
            <Pencil size={15} />
          </button>
          <button
            class="p-1.5 rounded text-text-muted hover:text-danger hover:bg-surface-2 disabled:opacity-40"
            title={$_("desktops.remove")}
            aria-label={$_("desktops.remove")}
            disabled={off || state.desktops.length < 2}
            on:click={() => change("remove_desktop", { id: desktop.id })}
          >
            <Trash2 size={15} />
          </button>
        {/if}
      </li>
    {/each}
  </ul>
</div>

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} onClose={() => (errorText = "")}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{/if}
