<script lang="ts">
  import ImageCache from "../js/app/components/imageCache";
  import ImageCacheOptions from "../js/app/components/imageCacheOptions";

  export let id: string;
  export let server: string;
  
  const canvasUrl = `${server}/assets/canvas.png`;
  const avatarUrl = `${server}/assets/avatar.png`;

  const defaultTtl = 60 * 60 * 24 * 7; // 7 days
  const imageCacher = new ImageCache();
  const canvasImageCacheOptions = new ImageCacheOptions(canvasUrl, defaultTtl);
  const avatarImageCacheOptions = new ImageCacheOptions(avatarUrl, defaultTtl);

  imageCacher.getImage(canvasImageCacheOptions).then((image) => {
    document.getElementById(id)?.querySelector("#canvas-logo")?.setAttribute("src", image);
  });
  imageCacher.getImage(avatarImageCacheOptions).then((image) => {
    document.getElementById(id)?.querySelector("#avatar-logo")?.setAttribute("src", image);
  });
</script>

<div id="{id}" class="card">
  <div class="h-24 rounded-t-lg bg-primary dark:bg-accent">
    <img id="canvas-logo" class="h-full w-full rounded-t-lg object-cover object-center" src="" alt="cover">
  </div>
  <div class="px-4 py-2 sm:px-5">
    <div class="flex justify-between space-x-4">
      <div class="avatar -mt-12 size-20">
        <img id="avatar-logo" class="rounded-full border-2 border-white dark:border-navy-700" src="" alt="avatar">
      </div>
    </div>
    <h3 id="name" class="text-center pb-4 pt-2 text-lg font-medium text-slate-700 dark:text-navy-100">
      {server}
    </h3>
    <div class="flex justify-center space-x-3 py-3">
       <button class="btn h-12 bg-primary text-base font-medium text-grey hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90" disabled>
        <div class="spinner size-7 animate-spin rounded-full border-[3px] border-slate-500 border-r-transparent dark:border-navy-300 dark:border-r-transparent"></div>
        <div id="message">Checking Server</div>
      </button>
    </div>
  </div>
</div>