# Changelog

## 1.0.0 — 2026-11-12

First public Hookline release for Horizon 26.3 (Minecraft: Java Edition).

- Reels your own bobber and recasts, with optional slower “pace variance” timing
- Keep/Toss catch list in a curated order (book, rod, bow, then the four fish, then the rest)
- Enchant voting for books, rods, and bows (Mending first, then closest to max, ties keep)
- Main-inventory tosses over ordinary client packets
- Deposits only kept catch into a chest or barrel Hookline opened itself
- Optional meals: off-hand then hotbar only, eat until hunger is full, one item at a time, with human-like pauses between bites and before swapping back to the rod and recasting; no same-tick swap-and-use
- Idle rest after five quiet minutes, look-around, worn-rod handling, leave if badly hurt
- Tabbed panel: Loot, Enchants, Behavior, Stats, with a search field, scrollbars on Loot and Enchants, and every list row reachable by scrolling
- Fresh config file `hookline.cfg` (format 1)
