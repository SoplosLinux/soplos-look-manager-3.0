<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import Modal from "../components/Modal.svelte";
  import Select from "../components/Select.svelte";
  import Switch from "../components/Switch.svelte";
  import NumberField from "../components/NumberField.svelte";

  /**
   * The options of the page "Window Behavior" of System Settings: what the program offers
   * (rows, their kind and the values a choice has) and what each holds now come from the
   * backend, so the window never holds a list of its own that could drift from KWin.
   */
  interface Row {
    key: string;
    section: string;
    kind: "choice" | "bool" | "number";
    choices: [string, string][];
    max: number;
    value: string;
  }

  const SECTIONS = ["focus", "placement", "snap", "titlebar", "activeTitlebar", "inactiveTitlebar", "windows", "modifier"];

  let rows: Row[] = [];
  let loaded = false;
  let busy = false;
  let errorText = "";

  onMount(async () => {
    progress.set("busy");
    try {
      rows = await api.invoke<Row[]>("list_behavior");
    } catch (error) {
      errorText = `${$_("behavior.errors.read")}\n${error}`;
    }
    progress.set(null);
    loaded = true;
  });

  const value = (key: string) => rows.find((r) => r.key === key)?.value;

  async function change(row: Row, next: string) {
    if (busy) return;
    busy = true;
    const before = row.value;
    rows = rows.map((r) => (r.key === row.key ? { ...r, value: next } : r));
    progress.set("busy");
    try {
      await api.invoke("set_behavior", { key: row.key, value: next });
    } catch (error) {
      rows = rows.map((r) => (r.key === row.key ? { ...r, value: before } : r));
      errorText = `${$_("behavior.errors.apply")}\n${error}`;
    }
    progress.set(null);
    busy = false;
  }

  // The delays and the raising only mean something with a focus that follows the mouse,
  // and clicking to raise does not apply when the window rises by itself.
  function shown(row: Row): boolean {
    const clicking = value("FocusPolicy") === "ClickToFocus";
    switch (row.key) {
      case "AutoRaise":
      case "DelayFocusInterval":
      case "NextFocusPrefersMouse":
        return !clicking;
      case "AutoRaiseInterval":
        return !clicking && value("AutoRaise") === "true";
      case "ClickRaise":
        return clicking || value("AutoRaise") !== "true";
      default:
        return true;
    }
  }

  $: off = !loaded || busy;
</script>

<div class="flex flex-col gap-4">
  {#each SECTIONS as section}
    {@const list = rows.filter((r) => r.section === section && shown(r))}
    {#if list.length}
      <section class="rounded border border-border bg-surface px-4 py-3">
        <h3 class="text-[0.7rem] font-semibold tracking-[0.08em] uppercase text-text-muted mb-2">{$_(`behavior.sections.${section}`)}</h3>
        <div class="flex flex-col">
          {#each list as row (row.key)}
            <div class="flex items-center justify-between gap-4 py-2 border-t border-border first:border-t-0">
              <span class="text-[0.86rem] text-text-bright">{$_(`behavior.rows.${row.key}`)}</span>
              <div class="shrink-0 {row.kind === 'choice' ? 'w-[min(20rem,55%)]' : ''}">
                {#if row.kind === "choice"}
                  <Select
                    value={row.value}
                    label={$_(`behavior.rows.${row.key}`)}
                    options={row.choices.map(([stored, name]) => ({ value: stored, label: $_(`behavior.choices.${name}`) }))}
                    on:change={(e) => change(row, e.detail)}
                  />
                {:else if row.kind === "bool"}
                  <Switch checked={row.value === "true"} label={$_(`behavior.rows.${row.key}`)} disabled={off} on:change={(e) => change(row, e.detail ? "true" : "false")} />
                {:else}
                  <NumberField value={Number(row.value)} min={0} max={row.max} label={$_(`behavior.rows.${row.key}`)} disabled={off} on:change={(e) => change(row, String(e.detail))} />
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/each}
</div>

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} onClose={() => (errorText = "")}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{/if}
