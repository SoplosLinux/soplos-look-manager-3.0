<script lang="ts">
  import { _ } from "svelte-i18n";
  import { api } from "../lib/api";
  import { progress } from "../lib/progress";
  import Modal from "./Modal.svelte";

  /** Asks before removing something the user installed, and removes it. */
  export let kind: string;
  export let item: { id: string; name: string };
  /** Called after it was removed, so the page reads its list again. */
  export let onDone: () => void | Promise<void>;
  export let onClose: () => void;

  let errorText = "";

  async function remove() {
    progress.set("busy");
    try {
      await api.invoke("remove_look", { kind, id: item.id });
      progress.set(null);
      onClose();
      await onDone();
    } catch (error) {
      progress.set(null);
      errorText = `${$_("install.removeError")}\n${error}`;
    }
  }
</script>

{#if errorText}
  <Modal variant="warning" title={$_("common.error")} {onClose}>
    <p class="whitespace-pre-line">{errorText}</p>
  </Modal>
{:else}
  <Modal variant="confirm" title={$_("install.removeTitle", { values: { name: item.name } })} confirmLabel={$_("install.remove")} onConfirm={remove} {onClose}>
    <p>{$_("install.removeQuestion")}</p>
  </Modal>
{/if}
