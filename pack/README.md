# Installation

1. Ensure that your BDS world is configured to allow for beta APIs

2. On Bedrock Dedicated Server, copy the `pack/pack/bp` directory, or extract the provided download to `behavior_packs/bedrock-voice-chat-0.0.1`.

3. Update your `config/default/variables.json` with the following. Be sure to replace the token and server with your values:

    ```
    {
        "bvc_access_token": "<YOUR_TOKEN_DEFINED_FROM_HCL_HERE>",
        "bvc_server": "https://YOUR_BVC_SERVER_FQDN"
    }
    ```

4. Update your `config/default/permissions.json` to have the following:

    ```
    {
        "allowed_modules": [
            "@minecraft/server-gametest",
            "@minecraft/server",
            "@minecraft/server-ui",
            "@minecraft/server-admin",
            "@minecraft/server-editor",
            "@minecraft/server-net"
        ]
    }
    ```

5. Start BDS Server