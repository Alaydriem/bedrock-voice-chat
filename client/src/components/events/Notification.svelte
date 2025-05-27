<script lang="ts">
    export let title: string;
    export let body: string;
    export let level: string;
</script>
<template id="notification-template">
    <div
      class="flex max-w-xs overflow-hidden rounded-lg bg-navy-600 font-normal"
    >
      <div class="p-2">
        <div class="flex items-center justify-between space-x-2">
          <h5 id="notification-title"class="font-medium tracking-wide text-navy-100 line-clamp-1 lg:text-base" >{title}</h5>
          <button
            data-notification-remove
            class="btn size-7 rounded-full p-0 text-white hover:bg-white/20 focus:bg-white/20 active:bg-white/25"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              class="size-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M6 18L18 6M6 6l12 12"
              ></path>
            </svg>
          </button>
        </div>
        <p class="text-navy-100" id="notification-body">{body}</p>
      </div>
    </div>
  </template>