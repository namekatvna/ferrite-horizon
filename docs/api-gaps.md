# Horizon mod API gaps

Things Ferrite needs that `horizon-mod-sdk` does not provide yet. Each entry: what, why, workaround, upstream status.

| Gap | Needed for | Workaround | Upstream |
|---|---|---|---|
| Inventory read/write | machines, storage mods | none | planned (API 0.4) |
| Entity read/spawn | mobs, combat | none | planned |
| Registries with code (blocks, items) | content with behaviour | JSON content mods (API 0.2), no logic | planned |
| Client-server `net` channel | anything synced | none | planned |
| Code patching (Mixin-style) | deep changes | a Horizon fork | not planned |
| Block events from explosions, pistons, other mods | protection, logging | none | not listed |
