<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Plus, Pencil, Trash2 } from "lucide-svelte";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import Modal from "../components/Modal.svelte";
  import Select from "../components/Select.svelte";
  import Switch from "../components/Switch.svelte";
  import NumberField from "../components/NumberField.svelte";

  /**
   * The window rules of KWin (`kwinrulesrc`): what is done to the windows that match a rule.
   * A rule says which windows (the class of the application and the title), and a list of
   * properties, each with what to do: force it, apply it when the window opens...
   */
  interface Property {
    key: string;
    value: string;
    action: number;
  }
  interface Rule {
    id: string;
    description: string;
    enabled: boolean;
    wmclass: string;
    wmclassmatch: number;
    wmclasscomplete: boolean;
    title: string;
    titlematch: number;
    properties: Property[];
  }
  interface Spec {
    key: string;
    kind: "bool" | "point" | "size" | "screen" | "percent";
    force: boolean;
  }
  interface Book {
    rules: Rule[];
    specs: Spec[];
  }

  // What a property starts with when it is turned on.
  const START: Record<Spec["kind"], string> = { bool: "true", point: "0,0", size: "800,600", screen: "0", percent: "100" };
  // The names of the actions, in the order of KWin; a "force" property takes only some.
  const ACTIONS = [1, 2, 3, 4, 5, 6];
  const NAME = { 1: "dontAffect", 2: "force", 3: "applyInitially", 4: "remember", 5: "applyNow", 6: "forceTemporarily" } as Record<number, string>;

  let book: Book = { rules: [], specs: [] };
  let loaded = false;
  let busy = false;
  let errorText = "";
  let editing: Rule | null = null;
  let used: Record<string, Property> = {};
  let removing: Rule | null = null;

  onMount(async () => {
    progress.set("busy");
    try {
      book = await api.invoke<Book>("list_rules");
    } catch (error) {
      errorText = `${$_("rules.errors.read")}\n${error}`;
    }
    progress.set(null);
    loaded = true;
  });

  async function run(command: string, args: Record<string, unknown>) {
    if (busy) return;
    busy = true;
    progress.set("busy");
    try {
      book = await api.invoke<Book>(command, args);
    } catch (error) {
      errorText = `${$_("rules.errors.apply")}\n${error}`;
      try {
        book = await api.invoke<Book>("list_rules");
      } catch {
        // What is shown stays as it was.
      }
    }
    progress.set(null);
    busy = false;
  }

  function open(rule: Rule | null) {
    editing = rule
      ? structuredClone(rule)
      : { id: "", description: $_("rules.newName"), enabled: true, wmclass: "", wmclassmatch: 1, wmclasscomplete: false, title: "", titlematch: 0, properties: [] };
    used = Object.fromEntries(editing.properties.map((p) => [p.key, { ...p }]));
  }

  async function save() {
    if (!editing) return;
    const rule = { ...editing, properties: book.specs.filter((s) => used[s.key]).map((s) => used[s.key]) };
    editing = null;
    await run("save_rule", { rule });
  }

  function toggle(spec: Spec, on: boolean) {
    if (on) used = { ...used, [spec.key]: { key: spec.key, value: START[spec.kind], action: spec.force ? 2 : 3 } };
    else {
      const { [spec.key]: _gone, ...rest } = used;
      used = rest;
    }
  }

  const set = (key: string, patch: Partial<Property>) => (used = { ...used, [key]: { ...used[key], ...patch } });

  // The two numbers of a point or a size, stored as "x,y".
  const part = (value: string, i: number) => Number(value.split(",")[i] ?? 0);
  const join = (value: string, i: number, n: number) => {
    const p = value.split(",");
    p[i] = String(n);
    return `${p[0] ?? 0},${p[1] ?? 0}`;
  };

  const matches = [0, 1, 2, 3].map((n) => ["any", "exact", "contains", "regex"][n]);
  const summary = (rule: Rule) => [rule.wmclassmatch ? rule.wmclass : "", rule.titlematch ? rule.title : ""].filter(Boolean).join("  ·  ") || $_("rules.anyWindow");

  $: off = !loaded || busy;
</script>

<div class="flex flex-col gap-3">
  <div class="flex justify-end">
    <button class="flex items-center gap-1.5 px-3.5 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.84rem] font-semibold disabled:opacity-50" disabled={off} on:click={() => open(null)}>
      <Plus size={14} />{$_("rules.add")}
    </button>
  </div>

  {#if loaded && !book.rules.length}
    <p class="text-[0.85rem] text-text-muted">{$_("rules.none")}</p>
  {/if}

  <ul class="flex flex-col gap-1.5">
    {#each book.rules as rule (rule.id)}
      <li class="flex items-center gap-3 px-3.5 py-2.5 rounded border border-border bg-surface">
        <Switch checked={rule.enabled} label={rule.description} disabled={off} on:change={(e) => run("set_rule_enabled", { id: rule.id, enabled: e.detail })} />
        <span class="flex-1 min-w-0">
          <span class="block text-[0.88rem] text-text-bright truncate">{rule.description}</span>
          <span class="block text-[0.74rem] text-text-muted truncate">{summary(rule)}</span>
        </span>
        <button class="p-1.5 rounded text-text-muted hover:text-text-bright hover:bg-surface-2 disabled:opacity-40" title={$_("rules.edit")} aria-label={$_("rules.edit")} disabled={off} on:click={() => open(rule)}><Pencil size={15} /></button>
        <button class="p-1.5 rounded text-text-muted hover:text-danger hover:bg-surface-2 disabled:opacity-40" title={$_("rules.remove")} aria-label={$_("rules.remove")} disabled={off} on:click={() => (removing = rule)}><Trash2 size={15} /></button>
      </li>
    {/each}
  </ul>
</div>

{#if editing}
  <Modal variant="confirm" title={editing.id ? $_("rules.edit") : $_("rules.add")} showIcon={false} maxWidth={660} confirmLabel={$_("rules.save")} confirmDisabled={!editing.description.trim()} onConfirm={save} onClose={() => (editing = null)}>
    <div class="flex flex-col gap-3 max-h-[58vh] overflow-y-auto content-scroll pr-2">
      <label class="flex flex-col gap-1">
        <span class="text-[0.78rem] text-text-muted">{$_("rules.description")}</span>
        <input class="px-2.5 py-1.5 rounded border border-border bg-surface-2 text-[0.86rem] text-text-bright outline-none focus:border-primary" bind:value={editing.description} maxlength="80" />
      </label>

      <div class="grid grid-cols-[1fr_11rem] gap-2 items-end">
        <label class="flex flex-col gap-1">
          <span class="text-[0.78rem] text-text-muted">{$_("rules.windowClass")}</span>
          <input class="px-2.5 py-1.5 rounded border border-border bg-surface-2 text-[0.86rem] text-text-bright outline-none focus:border-primary" bind:value={editing.wmclass} maxlength="120" />
        </label>
        <Select value={String(editing.wmclassmatch)} label={$_("rules.windowClass")} options={matches.map((m, n) => ({ value: String(n), label: $_(`rules.match.${m}`) }))} on:change={(e) => editing && (editing.wmclassmatch = Number(e.detail))} />
      </div>
      <label class="flex items-center gap-2.5 cursor-pointer">
        <input type="checkbox" class="accent-[#ff7d2e]" bind:checked={editing.wmclasscomplete} />
        <span class="text-[0.84rem]">{$_("rules.wholeClass")}</span>
      </label>

      <div class="grid grid-cols-[1fr_11rem] gap-2 items-end">
        <label class="flex flex-col gap-1">
          <span class="text-[0.78rem] text-text-muted">{$_("rules.windowTitle")}</span>
          <input class="px-2.5 py-1.5 rounded border border-border bg-surface-2 text-[0.86rem] text-text-bright outline-none focus:border-primary" bind:value={editing.title} maxlength="200" />
        </label>
        <Select value={String(editing.titlematch)} label={$_("rules.windowTitle")} options={matches.map((m, n) => ({ value: String(n), label: $_(`rules.match.${m}`) }))} on:change={(e) => editing && (editing.titlematch = Number(e.detail))} />
      </div>

      <h4 class="mt-1 text-[0.7rem] font-semibold tracking-[0.08em] uppercase text-text-muted">{$_("rules.properties")}</h4>
      {#each book.specs as spec (spec.key)}
        {@const p = used[spec.key]}
        <div class="flex items-center gap-3 flex-wrap">
          <Switch checked={!!p} label={$_(`rules.props.${spec.key}`)} on:change={(e) => toggle(spec, e.detail)} />
          <span class="w-44 text-[0.84rem] {p ? 'text-text-bright' : 'text-text-muted'}">{$_(`rules.props.${spec.key}`)}</span>
          {#if p}
            <div class="w-44">
              <Select value={String(p.action)} label={$_("rules.action")} options={ACTIONS.filter((a) => !spec.force || a === 1 || a === 2 || a === 6).map((a) => ({ value: String(a), label: $_(`rules.actions.${NAME[a]}`) }))} on:change={(e) => set(spec.key, { action: Number(e.detail) })} />
            </div>
            {#if spec.kind === "bool"}
              <Switch checked={p.value === "true"} label={$_(`rules.props.${spec.key}`)} on:change={(e) => set(spec.key, { value: e.detail ? "true" : "false" })} />
            {:else if spec.kind === "point" || spec.kind === "size"}
              <NumberField value={part(p.value, 0)} min={spec.kind === "size" ? 1 : -10000} max={10000} label={spec.kind === "size" ? $_("rules.width") : "X"} on:change={(e) => set(spec.key, { value: join(p.value, 0, e.detail) })} />
              <NumberField value={part(p.value, 1)} min={spec.kind === "size" ? 1 : -10000} max={10000} label={spec.kind === "size" ? $_("rules.height") : "Y"} on:change={(e) => set(spec.key, { value: join(p.value, 1, e.detail) })} />
            {:else}
              <NumberField value={Number(p.value)} min={0} max={spec.kind === "percent" ? 100 : 64} label={$_(`rules.props.${spec.key}`)} on:change={(e) => set(spec.key, { value: String(e.detail) })} />
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  </Modal>
{/if}

{#if removing}
  <Modal variant="danger" title={$_("rules.removeTitle", { values: { name: removing.description } })} confirmLabel={$_("rules.remove")} onConfirm={() => { const id = removing?.id ?? ""; removing = null; void run("remove_rule", { id }); }} onClose={() => (removing = null)}>
    <p>{$_("rules.removeQuestion")}</p>
  </Modal>
{/if}

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} onClose={() => (errorText = "")}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{/if}
