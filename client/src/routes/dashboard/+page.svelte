<script lang="ts">
    import Hold from "../../js/app/stronghold.ts";
    import { Store } from '@tauri-apps/plugin-store';
    import { info, error, warn } from '@tauri-apps/plugin-log';

    const result = (async () => { 
        const secretStore = await Store.load('secrets.json', { autoSave: false });
        const password = await secretStore.get<{ value: string }>("stronghold_password");

        const store = await Store.load('store.json', { autoSave: false });
        const currentServer = await store.get<{ value: string }>("current_server");
        info(`Current server: ${currentServer?.value}`);

        if (password?.value) {
            const stronghold = await Hold.new("servers", password.value);
            const server = await stronghold.get(currentServer!.value);
            const s = JSON.stringify(server);
            info(`Server: ${s}`);
        }
    })();
</script>

This is the dashboard