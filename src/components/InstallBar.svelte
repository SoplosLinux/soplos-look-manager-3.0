<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Download, FolderOpen } from "lucide-svelte";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import Modal from "./Modal.svelte";

  /**
   * The two ways to put something new in a page: the store of KDE (the same window as
   * System Settings has) and a file. A page shows only what can be done for its kind.
   */
  export let kind: string;
  /** Called when something was installed, so the page reads its list again. */
  export let onDone: () => void | Promise<void>;

  let options = { store: false, file: false };
  let busy = false;
  let errorText = "";

  onMount(async () => {
    try {
      options = await api.invoke<{ store: boolean; file: boolean }>("install_options", { kind });
    } catch {
      // No buttons.
    }
  });

  async function store() {
    if (busy) return;
    busy = true;
    try {
      await api.invoke("get_new", { kind });
      // Whatever was installed in the window is in the list when it closes.
      await onDone();
    } catch (error) {
      errorText = `${$_("install.errors.store")}\n${error}`;
    }
    busy = false;
  }

  async function file() {
    if (busy) return;
    busy = true;
    progress.set("busy");
    try {
      if (await api.invoke<boolean>("install_file", { kind })) await onDone();
    } catch (error) {
      errorText = `${$_("install.errors.file")}\n${error}`;
    }
    progress.set(null);
    busy = false;
  }

  const button = "flex items-center gap-1.5 px-3 py-1.5 rounded border border-border bg-surface-2 text-text hover:border-primary text-[0.8rem] disabled:opacity-60";
</script>

{#if options.store || options.file}
  <div class="flex justify-end gap-2 mb-3">
    {#if options.store}
      <button class={button} disabled={busy} on:click={store}><Download size={14} />{$_("install.store")}</button>
    {/if}
    {#if options.file}
      <button class={button} disabled={busy} on:click={file}><FolderOpen size={14} />{$_("install.file")}</button>
    {/if}
  </div>
{/if}

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} onClose={() => (errorText = "")}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{/if}
