`bvc-play-root-2026.pem` is the production root (subject `Bedrock Voice Chat Play Root CA 2026`, SHA-256 `55:E2:BF:B3:47:17:5F:48:32:87:5A:67:4D:1F:57:25:BB:AD:44:95:0E:C9:7F:03:40:46:11:53:05:7E:79:60`). Its key is offline. `PlayRoots::builtin` compiles it into every build, and a test pins its exact bytes.

`bvc-play-root-dev.pem` is a DEVELOPMENT root from `mise run play-ca-init` (subject `BVC Play Root CA 2026 (DEVELOPMENT)`). It is added in debug builds only, next to the production root. To use a new local CA, replace this file with that CA's `root_ca.crt`.
