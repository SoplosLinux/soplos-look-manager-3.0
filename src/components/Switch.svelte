<script lang="ts">
  import { createEventDispatcher } from "svelte";

  /** An on and off switch; `change` says the new value when it is flipped by the user. */
  export let checked = false;
  export let label = "";
  export let disabled = false;

  const dispatch = createEventDispatcher<{ change: boolean }>();
  function flip() {
    checked = !checked;
    dispatch("change", checked);
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  class="relative w-[34px] h-[19px] rounded-full transition-colors shrink-0 disabled:opacity-50 {checked ? 'bg-primary' : 'bg-surface-2 border border-border'}"
  on:click={flip}
>
  <span class="absolute top-[2px] left-[2px] w-[15px] h-[15px] rounded-full bg-white transition-transform {checked ? 'translate-x-[15px]' : ''}"></span>
</button>
