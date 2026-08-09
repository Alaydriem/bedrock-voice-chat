<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import RadFrame from "../../components/shell/RadFrame.svelte";
    import Cover from "../../components/shell/Cover.svelte";
    import DashboardScreen from "../../components/dashboard/DashboardScreen.svelte";
    import StatusPanel from "../../components/dashboard/StatusPanel.svelte";
    import { SelfController } from "../../js/app/dashboard/SelfController";
    import { PlayerLevelSources } from "../../js/app/dashboard/PlayerLevelSources";

    let statusOpen = $state(false);
    let { coverOpen = false } = $props();

    const self = new SelfController(
        { get: async () => undefined } as never,
        new PlayerLevelSources(),
    );
    void invoke;
</script>

<RadFrame>
    <Cover open={coverOpen} ondismiss={() => {}}>
        {#snippet under()}
            <DashboardScreen
                servers={[
                    {
                        server: "https://a.example.com",
                        host: "a.example.com",
                        player: "Al",
                        isCurrent: true,
                    },
                ]}
                serverName="a.example.com"
                currentHost="a.example.com"
                player="Alaydriem"
                {self}
                selfState={{
                    muted: false,
                    deafened: false,
                    recording: false,
                    mode: "activated",
                    holding: false,
                    transmitting: true,
                    recordAllowed: true,
                }}
                headline="NOBODY IN EARSHOT"
                {statusOpen}
                onswitch={() => {}}
                onadd={() => {}}
                onsettings={() => {}}
                onsignout={() => {}}
                onstatus={(open) => (statusOpen = open)}
            >
                {#snippet main()}
                    <StatusPanel
                        snapshot={null}
                        health={{ connected: true, reconnecting: false }}
                        pttIdle={false}
                        visiblePlayers={0}
                        reconnecting={false}
                        onreconnect={() => {}}
                        oncopy={() => {}}
                        onreset={() => {}}
                        onclose={() => (statusOpen = false)}
                    />
                {/snippet}
            </DashboardScreen>
        {/snippet}
    </Cover>
</RadFrame>
