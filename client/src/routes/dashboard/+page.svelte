<!--
  The dashboard itself lives in `+layout.svelte`.

  Settings is presented over it rather than instead of it, and a layout is the only
  thing SvelteKit keeps mounted across a child navigation. Holding the dashboard here
  would tear down its link, roster and position feed every time somebody opened a
  settings pane, and rebuild all of it on the way back.
-->
