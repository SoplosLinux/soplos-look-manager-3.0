<script lang="ts">
  import { onMount } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Plus, Trash2, ChevronDown, PanelBottom, PanelLeft, LayoutDashboard, Puzzle, X, Search } from "lucide-svelte";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import Modal from "../components/Modal.svelte";
  import Switch from "../components/Switch.svelte";
  import Select from "../components/Select.svelte";
  import NumberField from "../components/NumberField.svelte";
  import PanelPreview from "../components/PanelPreview.svelte";

  interface PanelState {
    id: number;
    location: string;
    height: number;
    floating: boolean;
    hiding: string;
    alignment: string;
    lengthMode: string;
    offset: number;
    minimumLength: number;
    maximumLength: number;
    opacity: string;
  }

  // A widget that can be put on a panel, and one that is on it.
  interface WidgetType {
    plugin: string;
    name: string;
    description: string;
    category: string;
    icon: string | null;
  }
  interface PanelWidget {
    id: number;
    plugin: string;
    name: string;
    icon: string | null;
  }

  const LOCATIONS = ["top", "bottom", "left", "right"];
  const ALIGNMENTS = ["left", "center", "right"];
  const OPACITIES = ["adaptive", "opaque", "translucent"];
  // The templates Plasma offers for a new panel.
  const TEMPLATES = [
    { id: "org.kde.plasma.desktop.defaultPanel", key: "default" },
    { id: "org.kde.plasma.desktop.emptyPanel", key: "empty" },
    { id: "org.kde.plasma.desktop.appmenubar", key: "appmenu" },
  ];
  // The styles that set several things at once; nothing is applied until Apply.
  const STYLES = [
    { key: "classic", icon: PanelBottom, set: { location: "bottom", alignment: "center", lengthMode: "fill", floating: false, offset: 0 } },
    { key: "dock", icon: LayoutDashboard, set: { location: "bottom", alignment: "center", lengthMode: "fit", floating: true, offset: 0 } },
    { key: "sidebar", icon: PanelLeft, set: { location: "left", alignment: "center", lengthMode: "fill", floating: false, offset: 0 } },
  ];

  let panels: PanelState[] = [];
  // The panel the controls change: a copy, so that nothing is touched until Apply.
  let state: PanelState | null = null;
  let loaded = false;
  let busy = false;
  let errorText = "";
  let askRemove = false;
  let addMenu = false;

  // The elements of the panel in view, and the ones that can be added.
  let widgets: PanelWidget[] = [];
  let widgetTypes: WidgetType[] = [];
  let pickWidget = false;
  let search = "";

  const src = (path: string) => convertFileSrc(path);

  async function loadWidgets() {
    widgets = [];
    if (!state) return;
    try {
      widgets = await api.invoke<PanelWidget[]>("list_panel_widgets", { panel: state.id, lang: $locale });
    } catch (error) {
      errorText = `${$_("panel.errors.widgets")}\n${error}`;
    }
  }

  function choose(id: number | null) {
    const found = panels.find((p) => p.id === id) ?? panels[0] ?? null;
    const changed = found?.id !== state?.id;
    state = found ? { ...found } : null;
    if (changed) void loadWidgets();
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

  const add = (template: string) => {
    addMenu = false;
    return work(async () => {
      const before = new Set(panels.map((p) => p.id));
      const list = await api.invoke<PanelState[]>("add_panel", { template });
      take(list, list.find((p) => !before.has(p.id))?.id ?? null);
      await loadWidgets();
    }, "panel.errors.add");
  };

  const remove = () => {
    askRemove = false;
    const id = state?.id;
    if (id === undefined) return;
    return work(async () => {
      take(await api.invoke<PanelState[]>("remove_panel", { id }), null);
      await loadWidgets();
    }, "panel.errors.remove");
  };

  async function openPicker() {
    pickWidget = true;
    search = "";
    if (widgetTypes.length === 0) {
      try {
        widgetTypes = await api.invoke<WidgetType[]>("list_widget_types", { lang: $locale });
      } catch (error) {
        pickWidget = false;
        errorText = `${$_("panel.errors.widgets")}\n${error}`;
      }
    }
  }

  // Elements are put on the panel at once, with no need to press Apply.
  function addWidget(plugin: string) {
    pickWidget = false;
    const panel = state?.id;
    if (panel === undefined) return;
    return work(async () => {
      widgets = await api.invoke<PanelWidget[]>("add_panel_widget", { panel, plugin, lang: $locale });
    }, "panel.errors.addWidget");
  }

  function removeWidget(widget: number) {
    const panel = state?.id;
    if (panel === undefined) return;
    return work(async () => {
      widgets = await api.invoke<PanelWidget[]>("remove_panel_widget", { panel, widget, lang: $locale });
    }, "panel.errors.removeWidget");
  }

  function setLocation(place: string) {
    if (state) state = { ...state, location: place };
  }

  function useStyle(change: Partial<PanelState>) {
    if (state) state = { ...state, ...change };
  }

  $: hidings = ["none", "autohide", "dodgewindows", "windowsgobelow"].map((value) => ({ value, label: $_(`panel.hiding.${value}`) }));
  $: lengthModes = ["fill", "fit", "custom"].map((value) => ({ value, label: $_(`panel.width.${value}`) }));
  $: off = !loaded || busy || !state;
  $: index = state ? panels.findIndex((p) => p.id === state?.id) : -1;
  $: needle = search.trim().toLowerCase();
  $: shown = widgetTypes.filter((w) => !needle || `${w.name} ${w.description} ${w.category}`.toLowerCase().includes(needle));
  const card = "rounded-lg border border-border bg-surface p-4 flex flex-col gap-3.5";
  const heading = "text-[0.66rem] font-semibold tracking-[0.07em] uppercase text-text-muted";
  const line = "flex items-center justify-between gap-4";
  const caption = "text-[0.86rem] text-text-bright";
  const segment = "px-2 py-2 rounded border text-[0.8rem] text-center truncate transition-colors disabled:opacity-60";
  const on = "border-primary bg-primary/10 text-primary font-semibold";
  const idle = "border-border bg-surface-2 text-text hover:border-primary";
  const badge = "w-5 h-5 shrink-0 rounded-full bg-primary/15 text-primary text-[0.68rem] font-bold grid place-items-center";
</script>

<!-- The page fills the window: the row of the panels and Apply is outside of what scrolls,
     so it stays where it is and Apply is always at hand. -->
<div class="h-full flex flex-col gap-3">
  <div class="shrink-0 flex items-center gap-2 flex-wrap">
    {#each panels as p, i (p.id)}
      <button
        class="px-4 py-1.5 rounded border text-[0.84rem] transition-colors disabled:opacity-60 {state?.id === p.id ? on : 'border-border bg-surface text-text hover:border-primary'}"
        aria-pressed={state?.id === p.id}
        disabled={busy}
        on:click={() => choose(p.id)}
      >
        {$_("panel.name", { values: { n: i + 1 } })}
      </button>
    {/each}

    <div class="relative">
      <button
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded border border-border bg-surface-2 text-text hover:border-primary text-[0.84rem] disabled:opacity-60"
        disabled={!loaded || busy}
        aria-expanded={addMenu}
        on:click={() => (addMenu = !addMenu)}
      >
        <Plus size={14} />{$_("panel.add")}<ChevronDown size={13} />
      </button>
      {#if addMenu}
        <!-- A click anywhere else closes the menu. -->
        <button class="fixed inset-0 z-30 cursor-default" aria-label={$_("common.close")} on:click={() => (addMenu = false)}></button>
        <ul class="absolute left-0 top-full mt-1 z-40 min-w-[230px] rounded border border-border bg-surface shadow-xl py-1">
          {#each TEMPLATES as template}
            <li>
              <button class="w-full text-left px-3.5 py-2 text-[0.84rem] text-text hover:bg-surface-2" on:click={() => add(template.id)}>
                {$_(`panel.template.${template.key}`)}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <button
      class="ml-auto px-5 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.85rem] font-semibold disabled:opacity-50 disabled:hover:bg-primary"
      disabled={off}
      title={$_("panel.note")}
      on:click={apply}
    >
      {$_("panel.apply")}
    </button>
  </div>

  {#if state}
   <div class="flex-1 min-h-0 overflow-y-auto content-scroll pr-2 flex flex-col gap-4">
    <!-- The drawing of the panel on its own, as wide as the page and with the
         proportions of the screen. -->
    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between">
        <span class={heading}>{$_("panel.name", { values: { n: index + 1 } })}</span>
        <button
          class="flex items-center gap-1.5 px-2 py-1 rounded text-[0.74rem] text-text-muted hover:text-danger hover:bg-surface-2 disabled:opacity-40 disabled:hover:text-text-muted disabled:hover:bg-transparent"
          disabled={off || panels.length < 2}
          title={panels.length < 2 ? $_("panel.lastOne") : $_("panel.remove")}
          on:click={() => (askRemove = true)}
        >
          <Trash2 size={14} />{$_("panel.remove")}
        </button>
      </div>
      <!-- What the drawing is, and that its edges can be clicked, is in its tooltip. -->
      <div title={$_("panel.previewHint")}>
      <PanelPreview
        items={widgets.map((w) => ({ icon: w.icon, name: w.name }))}
        location={state.location}
        height={state.height}
        floating={state.floating}
        alignment={state.alignment}
        lengthMode={state.lengthMode}
        offset={state.offset}
        minimumLength={state.minimumLength}
        maximumLength={state.maximumLength}
        opacity={state.opacity}
        disabled={off}
        onLocation={setLocation}
      />
      </div>
    </div>

    <!-- Under it, the cards in two columns, two rows. -->
    <div class="grid gap-4 items-stretch min-[960px]:grid-cols-2">
      <!-- 1. What goes on the panel: without elements a panel is an empty bar. -->
      <div class={card}>
        <span class="flex items-center gap-2 {heading}"><span class={badge}>1</span>{$_("panel.group.elements")}</span>
        <div class="flex-1 flex flex-col gap-3.5 min-h-0">
          {#if widgets.length === 0}
            <p class="text-[0.82rem] text-text-muted">{$_("panel.elements.empty")}</p>
          {:else}
            <ul class="flex flex-col gap-1.5 max-h-[300px] overflow-y-auto content-scroll pr-1">
              {#each widgets as widget, i (widget.id)}
                <li class="flex items-center gap-3 px-3 py-2 rounded border border-border bg-surface-2">
                  <span class="w-5 text-[0.72rem] text-text-muted text-right">{i + 1}</span>
                  {#if widget.icon}
                    <img src={src(widget.icon)} alt="" class="w-5 h-5 object-contain shrink-0" />
                  {:else}
                    <Puzzle size={18} class="text-text-muted shrink-0" />
                  {/if}
                  <span class="flex-1 min-w-0 truncate text-[0.84rem] text-text-bright">{widget.name}</span>
                  <button
                    class="p-1 rounded text-text-muted hover:text-danger hover:bg-surface disabled:opacity-40"
                    title={$_("panel.elements.remove")}
                    aria-label={$_("panel.elements.remove")}
                    disabled={off}
                    on:click={() => removeWidget(widget.id)}
                  >
                    <X size={15} />
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
          <button
            class="flex items-center justify-center gap-1.5 px-3 py-2 rounded border border-dashed border-primary/60 text-primary hover:bg-primary/10 text-[0.82rem] font-semibold disabled:opacity-50 whitespace-nowrap"
            disabled={off}
            title={$_("panel.elements.hint")}
            on:click={openPicker}
          >
            <Plus size={14} />{$_("panel.elements.add")}
          </button>
        </div>
      </div>

      <!-- 2. Where the panel is and how thick, and the styles that set several things at once. -->
      <div class={card}>
        <span class="flex items-center gap-2 {heading}"><span class={badge}>2</span>{$_("panel.group.size")}</span>
        <div class="flex flex-col gap-2">
          <span class={caption}>{$_("panel.styles.title")}</span>
          <div class="grid grid-cols-3 gap-1.5">
            {#each STYLES as style}
              <button
                class="flex flex-col items-center gap-1 px-2 py-2.5 rounded border border-border bg-surface-2 text-text hover:border-primary transition-colors disabled:opacity-60"
                disabled={off}
                on:click={() => useStyle(style.set)}
              >
                <svelte:component this={style.icon} size={18} class="text-primary" />
                <span class="text-[0.76rem] text-center">{$_(`panel.styles.${style.key}`)}</span>
              </button>
            {/each}
          </div>
        </div>
        <div class="flex flex-col gap-2">
          <span class={caption}>{$_("panel.location.title")}</span>
          <div class="grid grid-cols-2 gap-1.5">
            {#each LOCATIONS as place}
              <button class="{segment} {state.location === place ? on : idle}" aria-pressed={state.location === place} disabled={off} on:click={() => setLocation(place)}>
                {$_(`panel.location.${place}`)}
              </button>
            {/each}
          </div>
        </div>
        <div class={line}>
          <label class={caption} for="panel-height">{$_("panel.height")}</label>
          <div class="flex items-center gap-3">
            <input id="panel-height" type="range" min="24" max="120" step="2" bind:value={state.height} disabled={off} class="w-[130px] accent-[#ff7d2e]" />
            <span class="w-12 text-right text-[0.82rem] text-text-muted">{state.height} px</span>
          </div>
        </div>
        <div class={line}>
          <span class={caption}>{$_("panel.floating")}</span>
          <Switch bind:checked={state.floating} label={$_("panel.floating")} disabled={off} />
        </div>
      </div>

      <!-- 3. How long the panel is and where along the edge. -->
      <div class={card}>
        <span class="flex items-center gap-2 {heading}"><span class={badge}>3</span>{$_("panel.group.length")}</span>
        <div class="flex flex-col gap-2">
          <span class={caption}>{$_("panel.alignment.title")}</span>
          <div class="grid grid-cols-3 gap-1.5">
            {#each ALIGNMENTS as side}
              <button class="{segment} {state.alignment === side ? on : idle}" aria-pressed={state.alignment === side} disabled={off} on:click={() => state && (state.alignment = side)}>
                {$_(`panel.alignment.${side}`)}
              </button>
            {/each}
          </div>
        </div>
        <div class={line}>
          <span class={caption}>{$_("panel.width.title")}</span>
          <div class="w-[190px]"><Select bind:value={state.lengthMode} options={lengthModes} label={$_("panel.width.title")} /></div>
        </div>
        {#if state.lengthMode === "custom"}
          <div class={line}>
            <span class={caption}>{$_("panel.minimumLength")}</span>
            <NumberField bind:value={state.minimumLength} min={0} max={20000} step={10} label={$_("panel.minimumLength")} disabled={off} />
          </div>
          <div class={line}>
            <span class={caption}>{$_("panel.maximumLength")}</span>
            <NumberField bind:value={state.maximumLength} min={0} max={20000} step={10} label={$_("panel.maximumLength")} disabled={off} />
          </div>
        {/if}
        <div class={line}>
          <span class={caption}>{$_("panel.offset")}</span>
          <NumberField bind:value={state.offset} min={0} max={20000} step={10} label={$_("panel.offset")} disabled={off} />
        </div>
      </div>

      <!-- 4. How it looks and when it hides. -->
      <div class={card}>
        <span class="flex items-center gap-2 {heading}"><span class={badge}>4</span>{$_("panel.group.more")}</span>
        <div class="flex flex-col gap-2">
          <span class={caption}>{$_("panel.opacity.title")}</span>
          <div class="grid grid-cols-3 gap-1.5">
            {#each OPACITIES as mode}
              <button class="{segment} {state.opacity === mode ? on : idle}" aria-pressed={state.opacity === mode} disabled={off} on:click={() => state && (state.opacity = mode)}>
                {$_(`panel.opacity.${mode}`)}
              </button>
            {/each}
          </div>
        </div>
        <div class="flex flex-col gap-2">
          <span class={caption}>{$_("panel.hiding.title")}</span>
          <Select bind:value={state.hiding} options={hidings} label={$_("panel.hiding.title")} />
        </div>
      </div>
    </div>
   </div>
  {/if}
</div>

{#if pickWidget}
  <Modal variant="info" showIcon={false} title={$_("panel.elements.addTitle")} maxWidth={560} showFooter={false} onClose={() => (pickWidget = false)}>
    <div class="flex flex-col gap-3">
      <div class="flex items-center gap-2 px-3 py-2 rounded border border-border bg-surface-2">
        <Search size={15} class="text-text-muted shrink-0" />
        <input
          class="flex-1 min-w-0 bg-transparent outline-none text-[0.85rem] text-text-bright placeholder:text-text-muted"
          placeholder={$_("panel.elements.search")}
          bind:value={search}
        />
      </div>
      <ul class="flex flex-col gap-1 max-h-[340px] overflow-y-auto content-scroll pr-2">
        {#each shown as widget (widget.plugin)}
          <li>
            <button class="w-full flex items-center gap-3 px-3 py-2 rounded text-left hover:bg-surface-2" on:click={() => addWidget(widget.plugin)}>
              {#if widget.icon}
                <img src={src(widget.icon)} alt="" class="w-7 h-7 object-contain shrink-0" />
              {:else}
                <Puzzle size={24} class="text-text-muted shrink-0" />
              {/if}
              <span class="flex-1 min-w-0">
                <span class="block text-[0.86rem] text-text-bright truncate">{widget.name}</span>
                <span class="block text-[0.72rem] text-text-muted truncate">{widget.description || widget.category}</span>
              </span>
            </button>
          </li>
        {:else}
          <li class="px-3 py-4 text-[0.82rem] text-text-muted">{$_("panel.elements.none")}</li>
        {/each}
      </ul>
      <div class="flex justify-end">
        <button class="px-4 py-1.5 rounded border border-border bg-surface-2 text-text hover:border-primary text-[0.82rem]" on:click={() => (pickWidget = false)}>
          {$_("common.cancel")}
        </button>
      </div>
    </div>
  </Modal>
{/if}

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
