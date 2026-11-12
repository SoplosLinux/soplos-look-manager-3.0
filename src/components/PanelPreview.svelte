<script lang="ts">
  import { _ } from "svelte-i18n";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { theme } from "../lib/theme";

  /** A drawing of the screen with the panel where and as the settings say. */
  /** What is on the panel, in order: the file of the icon of each element, if it has one. */
  export let items: { icon: string | null; name: string }[] = [];
  export let location: string;
  export let height: number;
  export let floating: boolean;
  export let alignment: string;
  export let lengthMode: string;
  export let offset: number;
  export let minimumLength: number;
  export let maximumLength: number;
  export let opacity: string;
  export let disabled = false;
  /** Called when an edge of the screen is clicked. */
  export let onLocation: (place: string) => void;

  // The drawing has the proportions of the screen the window is on, which the window
  // itself knows (`window.screen`, in the same logical pixels as the panels), and
  // everything in it is as big, in proportion, as it is on the screen.
  const FLOATING_GAP = 8;
  const PLACES = ["top", "bottom", "left", "right"];

  const WIDTH = window.screen.width > 0 ? window.screen.width : 1920;
  const HEIGHT = window.screen.height > 0 ? window.screen.height : 1080;
  $: horizontal = location === "top" || location === "bottom";
  $: along = horizontal ? WIDTH : HEIGHT;
  $: across = horizontal ? HEIGHT : WIDTH;
  // How thick the panel is, and how long, as a percentage of its screen.
  $: thickness = Math.max(2, (height / across) * 100);
  $: length = lengthMode === "fill" ? 100 : lengthMode === "fit" ? 42 : Math.min(100, Math.max(8, (Math.max(minimumLength, maximumLength) / along) * 100 || 42));
  $: shift = Math.min(60, (offset / along) * 100);
  $: alpha = opacity === "opaque" ? 1 : opacity === "translucent" ? 0.55 : 0.86;
  // The space a floating panel leaves around it, as a percentage of each side.
  $: gapAlong = floating ? (FLOATING_GAP / along) * 100 : 0;
  $: gapAcross = floating ? (FLOATING_GAP / across) * 100 : 0;

  // Where the panel goes along its edge: from the start, from the end or in the
  // middle, moved by the offset.
  function span(): string {
    const [start, end, size] = horizontal ? ["left", "right", "width"] : ["top", "bottom", "height"];
    if (lengthMode === "fill") return `${start}: ${gapAlong}%; ${end}: ${gapAlong}%;`;
    if (alignment === "left") return `${start}: calc(${shift}% + ${gapAlong}%); ${size}: ${length}%;`;
    if (alignment === "right") return `${end}: calc(${shift}% + ${gapAlong}%); ${size}: ${length}%;`;
    return `${start}: calc(50% - ${length / 2}% + ${shift}%); ${size}: ${length}%;`;
  }

  // The drawing follows the light or the dark theme of the program, as the panel of a
  // desktop with a light or a dark color scheme would look.
  $: dark = $theme === "dark";
  $: wallpaper = dark
    ? "radial-gradient(120% 120% at 85% 90%, #ffb347 0%, #c96a10 22%, #3a2a1a 46%, #19191b 70%)"
    : "radial-gradient(120% 120% at 85% 90%, #ffc76b 0%, #f0a24e 24%, #f3e3cf 50%, #f6f6f8 72%)";
  $: panelColor = dark ? `rgba(58, 58, 64, ${alpha})` : `rgba(247, 247, 249, ${alpha})`;
  $: style = `${location}: ${gapAcross}%; ${horizontal ? "height" : "width"}: ${thickness}%; ${span()} background: ${panelColor}; border-radius: ${floating ? 6 : 0}px;`;
</script>

<div
  class="relative w-full rounded-lg overflow-hidden border border-border"
  style="aspect-ratio: {WIDTH} / {HEIGHT}; background: {wallpaper};"
>
  <!-- A window and two icons, so it looks like a desktop. -->
  <div class="absolute left-[22%] top-[18%] w-[52%] h-[48%] rounded-md {dark ? 'bg-white/10 border-white/10' : 'bg-white/55 border-black/10'} border"></div>
  <div class="absolute left-[5%] top-[10%] w-[6%] aspect-square rounded {dark ? 'bg-white/15' : 'bg-black/10'}"></div>
  <div class="absolute left-[5%] top-[26%] w-[6%] aspect-square rounded {dark ? 'bg-white/15' : 'bg-black/10'}"></div>

  <!-- The panel, with its elements in their order: each one is drawn with its own
       icon, as big as the panel is thick. -->
  <div class="absolute border {dark ? 'border-white/30' : 'border-black/15'} shadow-lg flex items-center justify-center gap-[2.5%] transition-all duration-200 overflow-hidden {horizontal ? 'flex-row' : 'flex-col'}" {style}>
    {#each items as item}
      {#if item.icon}
        <img src={convertFileSrc(item.icon)} alt={item.name} title={item.name} class="{horizontal ? 'h-[56%]' : 'w-[56%]'} aspect-square object-contain shrink-0" />
      {:else}
        <span title={item.name} class="{horizontal ? 'h-[52%]' : 'w-[52%]'} aspect-square rounded-sm {dark ? 'bg-white/70' : 'bg-black/45'} shrink-0"></span>
      {/if}
    {/each}
  </div>

  <!-- The edges of the screen: a click moves the panel there. -->
  {#each PLACES as place}
    <button
      class="absolute z-10 transition-colors {location === place ? '' : 'hover:bg-primary/25'}
        {place === 'top' ? 'top-0 inset-x-0 h-[22%]' : place === 'bottom' ? 'bottom-0 inset-x-0 h-[22%]' : place === 'left' ? 'left-0 inset-y-[22%] w-[14%]' : 'right-0 inset-y-[22%] w-[14%]'}"
      {disabled}
      aria-label={$_(`panel.location.${place}`)}
      title={$_(`panel.location.${place}`)}
      on:click={() => onLocation(place)}
    ></button>
  {/each}
</div>
