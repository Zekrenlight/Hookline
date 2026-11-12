# Hookline

Version 1.0.0.

Hookline is written entirely in Rust for Horizon, a Rust client for Minecraft: Java Edition 26.3. No Java or Fabric required.
It reels when **your** bobber splashes, recasts, and gives you a keep/toss list, enchant rules, optional meals, and a deposit pass into a chest you let it open.

Use only on servers where AFK fishing/automation is allowed.

This is an independent implementation of autofishing.

## Features

- Toggle with **V**; open the panel with **O** (rebindable). The toggle shows a short action-bar message.
- Bite detection uses your own bobber splash, with a short settle window after a cast.
- Pace variance (on by default) waits a little before reeling and stretches recast and meal pauses with non-uniform jitter.
- Catch list order: Enchanted Book, Fishing Rod, Bow, Salmon, Cod, Tropical Fish, Pufferfish, then the rest of the usual catch. Each row is Keep or Toss. The in-use rod is never thrown.
- Books, rods, and bows also use per-enchant rules. Mending wins if it is present; otherwise the closest-to-max vote wins; a tie keeps the item. Unlisted enchants do not vote. The item row is the master switch.
- Tosses go through inventory-click packets, including items in the main inventory.
- When the bag is almost full, Hookline can deposit **kept catch only** into a chest or barrel it opened, then cool down.
- Optional meals: looks in the off-hand, then the hotbar only, and eats one item at a time until hunger is full, with a pause between bites. After the last bite it waits before grabbing the rod and again before recasting. Swap and use never share a tick.
- Optional idle rest after five minutes with no bite, reel, or recast. Meals do not reset or count as idle time.
- Optional look-around, switch or stop on a worn rod, and leave the server below three hearts.
- Session ledger and per-item catch chimes.

## Screenshots

<img width="1381" height="959" alt="image" src="https://github.com/user-attachments/assets/2f0dd17d-aae5-48d3-9899-e9a07fe06620" />
<img width="1381" height="959" alt="image" src="https://github.com/user-attachments/assets/0bd11d12-4ce6-44bc-bcf7-b7e7f2268d8e" />
<img width="1381" height="959" alt="image" src="https://github.com/user-attachments/assets/aa82d264-6917-449a-b1fc-1bd6ea71f41b" />
<img width="1381" height="959" alt="image" src="https://github.com/user-attachments/assets/88e8c7d7-e675-436d-9970-95a92ae4a859" />

## Install

Drop `hookline.hnmod` into your Horizon launcher's native / `.hnmod` mods folder, then start Horizon 26.3. Enable Hookline in the mods list if it is not on already. In a world, press **V** to start fishing and **O** to open the panel.

No special "trust native" flag is required for ordinary use; the file is source the launcher compiles.

## Keybinds

| Action | Default |
| --- | --- |
| Turn Hookline on or off | V |
| Open or close the panel | O |

Chat commands use the prefix `/hookline`: `toggle`, `pace`, `panel`, `ledger`, `clear-ledger`, `toss`, `chime`, and `help`.

Settings are stored in `hookline.cfg` under the mod data directory (format version 1). There is no automatic import of another mod's config.

## Menu

The panel is a centered tabbed card:

- **Loot** — the twenty-one catch items, each with Keep/Toss and Chime/Quiet chips, a search field, and a scrollbar
- **Enchants** — per-enchant Toss / Any / minimum-level chips, with a scrollbar
- **Behavior** — on/off options (pace variance, look-around, meals, worn rod, leave if hurt, idle auto-off, overlay)
- **Stats** — the session ledger and a clear-ledger control

Colors are Hookline's own (deep teal, sand text, coral toss chips, amber accents).

## Compatibility

- Minecraft Java **26.3**
- Horizon revision `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`
- Mixins revision `5a64af938ec86b00d53d16a408aabc0038fb3e57`

## Build and verify

From this repository, with Horizon sources and `horizon-mixins` available:

```bash
# Rules tests (no game runtime required)
cargo test --offline

# Apply mixins (example paths — adjust to your tree)
horizon-mixins apply \
  --workspace /path/to/horizon-src \
  --mods ./mixins \
  --out /path/to/horizon-staged-hookline

# Typecheck the staged client
cd /path/to/horizon-staged-hookline
cargo check --locked -p horizon-client
```

Pack a distributable (a ZIP of `horizon-native.json` plus every file under `mixins/hookline/`):

```python
import zipfile
from pathlib import Path

src = Path(".")
files = [src / "horizon-native.json"] + sorted((src / "mixins/hookline").glob("*"))
with zipfile.ZipFile("hookline.hnmod", "w", zipfile.ZIP_DEFLATED) as z:
    for p in files:
        if p.is_file():
            z.write(p, p.as_posix())
print("wrote hookline.hnmod")
```

## License

Copyright (c) 2026 Zekrenlight. All rights reserved.

Personal use of official Hookline releases is permitted. Copying, redistribution, modification, or derivative works of the code, assets, or GUI design requires prior written permission. To request permission, open an issue or contact Zekrenlight on GitHub.

Ideas and inspiration are welcome; the code and GUI as they stand may not be reused. See [LICENSE](LICENSE) for the full notice.
