<script lang="ts">  
  import ImageCache from "../../js/app/components/imageCache";
  import ImageCacheOptions from "../../js/app/components/imageCacheOptions";

  export let id: string;
  export let server: string;
  export let active: boolean;

  const avatarUrl = `${server}/assets/avatar.png`;

  const defaultTtl = 60 * 60 * 24 * 7; // 7 days
  const imageCacher = new ImageCache();
  const avatarImageCacheOptions = new ImageCacheOptions(avatarUrl, defaultTtl);

  imageCacher.getImage(avatarImageCacheOptions).then((image) => {
    document.getElementById(id)?.querySelector("#avatar-logo")?.setAttribute("src", image);
  });
</script>
{#if active}
    <a
    id={id}
    href="#"
    aria-label="{server}"
    data-tooltip="Dashboard {server}"
    data-placement="right"
    class="tooltip-main-sidebar flex size-11 items-center justify-center rounded-lg bg-primary/10 text-primary outline-hidden transition-colors duration-200 hover:bg-primary/20 focus:bg-primary/20 active:bg-primary/25 dark:bg-navy-600 dark:text-accent-light dark:hover:bg-navy-450 dark:focus:bg-navy-450 dark:active:bg-navy-450/90"
    >
        <img id="avatar-logo" class="rounded-full border-2 border-white dark:border-navy-700" src="" alt="avatar">
    </a>
{:else}
    <a
    id={id}
    href="/dashboard?server={server}"
    aria-label="{server}"
    data-tooltip="Dashboard {server}"
    data-placement="right"
    class="tooltip-main-sidebar flex size-11 items-center justify-center rounded-lg outline-hidden transition-colors duration-200 hover:bg-primary/20 focus:bg-primary/20 active:bg-primary/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
    >
        <img id="avatar-logo" class="rounded-full border-2 border-white dark:border-navy-700" src="" alt="avatar">
    </a>
{/if}        