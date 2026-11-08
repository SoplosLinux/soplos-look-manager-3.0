<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Plus, Trash2 } from "lucide-svelte";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import Modal from "../components/Modal.svelte";
  import Switch from "../components/Switch.svelte";
  import Select from "../components/Select.svelte";

  interface PanelState {
    id: number;
    location: string;
    height: number;
    floating: boolean;
    hiding: string;
  }

  const LOCATIONS = ["top", "bottom", "left", "right"];

  let panels: PanelState[] = [];
  // The panel the controls change: a copy, so that nothing is touched until Apply.
  let state: PanelState | null = null;
  let loaded = false;
  let busy = false;
  let errorText = "";
  let askRemove = false;

  function choose(id: number | null) {
    const found = panels.find((p) => p.id === id) ?? panels[0] ?? null;
    state = found ? { ...found } : null;
  }

  function take(list: PanelState[], keep: number | null = state?.id ?? null) {
    panels = list;
    choose(keep);
  }

  onMount(async () => {
    try {
      take(await api.invoke<PanelState[]>("list_panels"));
    } catch (error) {
      errorText = `${$_("panel.errors.read")}\n${error}`;
    }
    loaded = true;
  });

  async function work(task: () => Promise<void>, failure: string) {
    busy = true;
    progress.set("busy");
    try {
      await task();
    } catch (error) {
      errorText = `${$_(failure)}\n${error}`;
    }
    progress.set(null);
    busy = false;
  }

  const apply = () =>
    work(async () => {
      if (!state) return;
      await api.invoke("set_panel", { state });
      take(await api.invoke<PanelState[]>("list_panels"), state.id);
    }, "panel.errors.apply");

  const add = () =>
    work(async () => {
      const before = new Set(panels.map((p) => p.id));
      const list = await api.invoke<PanelState[]>("add_panel");
      take(list, list.find((p) => !before.has(p.id))?.id ?? null);
    }, "panel.errors.add");

  const remove = () => {
    askRemove = false;
    const id = state?.id;
    if (id === undefined) return;
    return work(async () => {
      take(await api.invoke<PanelState[]>("remove_panel", { id }), null);
    }, "panel.errors.remove");
  };

  $: hidings = ["none", "autohide", "dodgewindows", "windowsgobelow"].map((value) => ({ value, label: $_(`panel.hiding.${value}`) }));
  $: off = !loaded || busy || !state;
  const row = "flex items-center justify-between gap-4 px-4 py-3 rounded border border-border bg-surface";
</script>

<div class="flex flex-col gap-2.5">
  <div class="flex items-center gap-2 flex-wrap">
    {#each panels as p, i (p.id)}
      <button
        class="px-4 py-1.5 rounded border text-[0.84rem] transition-colors disabled:opacity-60
          {state?.id === p.id ? 'border-primary bg-primary/10 text-primary font-semibold' : 'border-border bg-surface text-text hover:border-primary'}"
        aria-pressed={state?.id === p.id}
        disabled={busy}
        on:click={() => choose(p.id)}
      >
        {$_("panel.name", { values: { n: i + 1 } })}
      </button>
    {/each}
    <button
      class="flex items-center gap-1.5 px-3.5 py-1.5 rounded border border-border bg-surface-2 text-text hover:border-primary text-[0.84rem] disabled:opacity-60"
      disabled={!loaded || busy}
      on:click={add}
    >
      <Plus size={14} />{$_("panel.add")}
    </button>
    <button
      class="flex items-center gap-1.5 px-3.5 py-1.5 rounded border border-border bg-surface-2 text-text hover:border-danger hover:text-danger text-[0.84rem] disabled:opacity-60 disabled:hover:border-border disabled:hover:text-text"
      disabled={off || panels.length < 2}
      title={panels.length < 2 ? $_("panel.lastOne") : ""}
      on:click={() => (askRemove = true)}
    >
      <Trash2 size={14} />{$_("panel.remove")}
    </button>
  </div>

  {#if state}
    <div class={row}>
      <span class="text-[0.9rem] text-text-bright">{$_("panel.location.title")}</span>
      <div class="flex gap-1.5">
        {#each LOCATIONS as place}
          <button
            class="px-3.5 py-1.5 rounded border text-[0.82rem] transition-colors disabled:opacity-60
              {state.location === place ? 'border-primary bg-primary/10 text-primary font-semibold' : 'border-border bg-surface-2 text-text hover:border-primary'}"
            aria-pressed={state.location === place}
            disabled={off}
            on:click={() => state && (state.location = place)}
          >
            {$_(`panel.location.${place}`)}
          </button>
        {/each}
      </div>
    </div>

    <div class={row}>
      <label class="text-[0.9rem] text-text-bright" for="panel-height">{$_("panel.height")}</label>
      <div class="flex items-center gap-3">
        <input id="panel-height" type="range" min="24" max="120" step="2" bind:value={state.height} disabled={off} class="w-[200px] accent-[#ff7d2e]" />
        <span class="w-10 text-right text-[0.85rem] text-text-muted">{state.height}</span>
      </div>
    </div>

    <div class={row}>
      <span class="text-[0.9rem] text-text-bright">{$_("panel.floating")}</span>
      <Switch bind:checked={state.floating} label={$_("panel.floating")} disabled={off} />
    </div>

    <div class={row}>
      <span class="text-[0.9rem] text-text-bright">{$_("panel.hiding.title")}</span>
      <div class="w-[240px]"><Select bind:value={state.hiding} options={hidings} label={$_("panel.hiding.title")} /></div>
    </div>

    <div>
      <button
        class="mt-2 px-6 py-2.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.88rem] font-semibold disabled:opacity-50 disabled:hover:bg-primary"
        disabled={off}
        on:click={apply}
      >
        {$_("panel.apply")}
      </button>
      <p class="mt-2 text-[0.76rem] text-text-muted">{$_("panel.note")}</p>
    </div>
  {/if}
</div>

{#if askRemove}
  <Modal
    variant="danger"
    title={$_("panel.removeTitle")}
    confirmLabel={$_("panel.remove")}
    onConfirm={remove}
    onClose={() => (askRemove = false)}
  >
    <p>{$_("panel.removeQuestion")}</p>
  </Modal>
{/if}

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} onClose={() => (errorText = "")}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{/if}
