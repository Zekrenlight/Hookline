# Hookline — feature specification

This document describes intended in-game behavior only. It is not source code, and it does not name other mods’ types, files, setting keys, or screen layout numbers.

Hookline is an optional helper for fishing in Horizon (Minecraft Java, game 26.3). When it is turned on, it watches for a bite on the player’s own line, reels in, and recasts. The player can also choose what to keep or toss, deposit kept loot, eat when hungry, and rest the helper after a long lull. Everything below is the intended behavior of this implementation.

## Turning it on and talking to it

- The default toggle key is **V**. Pressing it turns Hookline on or off. A short message appears on the action bar so the player can see the change without opening a menu.
- The default panel key is **O**. It opens or closes Hookline’s own panel while the player is in a world. The panel is described later; it is a tabbed card, not a copy of another mod’s screen.
- Chat commands start with `/hookline`. Useful subcommands include toggling the helper, toggling slower “human” timing, opening the panel, showing or clearing the session ledger, and adding or removing the held item from the toss list or the catch chime list. Unknown input prints a short help blurb.
- Both keys can be rebound through Horizon’s usual mod key settings. The helper does nothing if the mod itself is disabled in Horizon.

## Detecting a bite

- Only a splash that belongs to **this player’s bobber** counts. The helper listens for the vanilla bobber-splash sound (including an entity-attached sound) and accepts it only when a hook is out and the splash is within about two blocks of that hook. A splash with no usable position, or one that is farther away, is ignored so another player’s bobber does not reel this line in.
- After a fresh cast, there is a short settle window (thirty ticks, about a second and a half) during which splashes are ignored, so a leftover sound from the previous catch is not treated as a new bite.
- While the player is eating, including the pauses before and after a meal, bites are ignored.
- If the “pace variance” option is on, the helper waits a random 5–14 ticks after a valid bite before reeling. If that option is off, it reels as soon as the bite is confirmed, still obeying the safety gaps below.
- Reeling and recasting send a use-item packet when the player is on a multiplayer server. On a connected client the helper must not call the single-player “authoritative” rod or drop APIs, which can spawn ghost items. The local (no-server) game may use the local rod API.

## Recasting and rod handling

- After a successful reel, the helper always waits before recasting. The wait is a jittered 15–24 ticks (about 0.75–1.2 seconds). If pace variance is on, an extra 3–10 ticks are added. The wait is never a single fixed number every time; it is built from two random rolls so it does not repeat identically.
- The helper never reels and recasts on the same tick.
- It never changes hotbar slot and presses use on the same tick, and never throws an item and presses use on the same tick. There must be at least two ticks between a slot change or a throw and the next use.
- It never sends a use-item packet on the same tick that the use key was released, and it leaves at least two ticks between releasing use and the next use.
- If the selected item is not a usable fishing rod, the helper looks only in the hotbar for another rod. It does not pull a rod out of the main inventory by itself. If it switches slots, it waits before casting.
- If “retire a worn rod” is on, a rod with two durability points or fewer remaining is treated as unusable. The helper switches to another healthy hotbar rod, or turns itself off if none remains, and says so on the action bar.

## Catch policy (keep or toss)

The loot list is shown in this fixed order (our order, not a vendor default):

1. Enchanted Book
2. Fishing Rod
3. Bow
4. Salmon
5. Cod
6. Tropical Fish
7. Pufferfish
8. Name Tag
9. Stick
10. Nautilus Shell
11. Leather Boots
12. Potion
13. Bone
14. Lily Pad
15. Saddle
16. Ink Sac
17. Bowl
18. String
19. Tripwire Hook
20. Rotten Flesh
21. Leather

Each of those rows has a **Keep** or **Toss** choice. Keep means every stack of that item is kept. Toss means every stack of that item is thrown — every slot, not only a newly caught one — with one exception: the fishing rod that is currently selected and in use is never thrown.

Default Toss items (the usual “junk” set) are: lily pad, bowl, leather, leather boots, rotten flesh, stick, string, bone, ink sac, tripwire hook, and potion. Everything else in the list defaults to Keep.

Item identifiers are normalized (optional `minecraft:` prefix, lowercase) so equivalent names match. Unlisted items are kept unless the player later adds them. Optional catch packs that use the `gofish:` prefix are treated as fish, except names that look like crates or rods, which count as treasure, so they can be deposited or tallied correctly.

## Enchant rules and voting

Enchanted books, fishing rods, and bows have an extra layer. The **item row is the master switch**:

- If the row is Toss, every instance is thrown (except the in-use rod). Enchant votes are not consulted.
- If the row is Keep, each known enchant on the stack casts a vote.

Each known enchant has a player setting:

- **Toss** — this enchant always votes to throw the stack.
- **Any** — any positive level votes to keep.
- **A minimum level** — a level at or above the minimum votes to keep; a lower level votes to toss. The comparison is “greater or equal,” not “exactly this level.”

Unlisted enchants (ones the panel does not name) do not vote, and do not by themselves cause a throw. A book that “comes with something extra” is still fine if the known enchants say to keep it.

When votes disagree:

1. If Mending is present on the stack, **Mending’s vote wins**, no matter what the other enchants say.
2. Otherwise, the strongest vote wins, where “stronger” means closer to that enchant’s vanilla maximum (a maxed enchant outranks a mid-level one; closeness is the level divided by the max).
3. If the strongest keep vote and the strongest toss vote are equally close to their maxima, the stack is **kept**.

Default settings: Mending is “keep at level 1” (its only level). Unbreaking is “keep at level 3” (so Unbreaking I or II votes toss). Every other listed enchant defaults to Any. Vanilla maxima follow current Minecraft 26.3, including trident, crossbow, and mace enchants (Density, Breach, Wind Burst, and so on). Protection, Fire Protection, Blast Protection, and Projectile Protection are distinct names and must not be confused with each other.

Tossing happens by a server inventory-click “throw” packet (window 0, the player inventory, even if no screen is open) so items in the main inventory are not left behind. In a purely local game with no menu open, a hotbar throw may briefly select the slot, throw, and select the rod again; a main-inventory throw is best-effort.

## Depositing kept loot

When the inventory has one free slot or none, the helper pauses fishing, reels if the hook is out (honoring the use-item gaps), and looks for a nearby chest, trapped chest, or barrel in a small area around the player (a few blocks, a bit wider than it is tall).

- It only deposits into a container **it opened itself**. If the player already has a container or inventory screen open, it will not dump items into it.
- It only moves **kept fishing loot** — items on the catch list (or the optional catch-pack fish/treasure above) that the keep/toss and enchant rules would keep. It never deposits any fishing rod, and never deposits unrelated inventory.
- After one pass over the thirty-six player slots (quick-moving each kept stack, with a tiny pause between moves), it closes the container, looks back where the player was facing, and waits a cooldown of two hundred ticks (ten seconds) before trying again, even if the bag is still tight.
- If no container is in range, or the open attempt fails, it says so in chat and gives up that pass.

## Meals (auto-eat)

When the meal option is on and Hookline is on:

- A meal starts when food is below 20 (not full) and continues, one item at a time, until hunger is full (20).
- Food is anything with the game’s food component. Potions are not food. Rotten flesh, spider eye, pufferfish, poisonous potato, and suspicious stew are skipped unless they are the only food available.
- Search order: off-hand first, then the hotbar only. Food already held in the main hand counts as hotbar food. The helper does not pull food out of the main inventory for a meal.
- Holding food is allowed; the helper will eat it. A **manual** right-click already in progress is not interrupted. The helper’s own held use key does not cancel the meal. If the player switches away from the food mid-meal, the meal is cancelled.
- If the hook is out, the helper reels first, then waits a random **15–30 ticks** before swapping to food or starting to eat. That reel is not counted as fishing activity for the idle timer.
- A slot swap and a use-key press never share a tick. After swapping to food there is a further **2–5 tick** pause before use begins.
- The use key is held for forty ticks (about two seconds) for each item. On a server this is a use-item packet with the correct hand, and the client use-key flag is held every tick so the game does not immediately release the eat. Off-hand meals must not start a main-hand use on the rod.
- If hunger is still not full and more food is in the off-hand or hotbar, the helper waits a random **15–35 ticks** (not a chain of instant bites) before starting the next item. The same swap/use gaps apply if it has to change hotbar slots.
- After the last bite, use is released on its own tick. Then a random **30–70 tick** pause (always; never skipped) runs before swapping back to the rod. Then a random **40–100 tick** pause (always) runs before recasting. These pauses use the same two-roll jitter as above. If pace variance is on, each pause is stretched by an extra 3–10 ticks.
- If no food is found in the off-hand or hotbar, a short action-bar notice appears, at most once every ten seconds.

## Idle auto-off

When this option is on (it defaults on), Hookline turns itself off after **six thousand ticks** (five minutes) with no bite, reel, or recast, and says so on the action bar. Eating — including the waits before and after a meal — does not reset that timer and does not count as idle time either, so a long meal cannot trip the timeout and is not treated as a fresh cast.

## Other behavior flags

- **Pace variance** (default on): slower, jittered reel-after-bite timing, and slightly longer meal and recast pauses, as described above. Meal pauses after eating still run even when this is off.
- **Look-around** (default off): about once a minute, nudge the view a few degrees and jump if the player is on the ground, so a server is less likely to kick an idle fisher.
- **Retire a worn rod** (default off): described under recasting.
- **Leave if badly hurt** (default on): if health falls below three hearts (six health points), Hookline stops and leaves the server, with a clear message.
- **Corner overlay** (default off): when on, a small heads-up line can show the current status (on/off, a brief catch or toss note).

All of these, plus keep/toss, chimes, and enchant minima, are saved to a Hookline config file (`hookline.cfg`) under the mod’s data directory. The file is the source of truth; a leftover Horizon native default must not overwrite a value the player already saved. There is no automatic import of any other mod’s config. Format version starts at 1.

## Catch chimes and the session ledger

- Each catch-list item can have a chime. When a stack of a chimed item increases, the helper plays a short local pickup sound and flashes a brief “caught” note.
- The session ledger tracks fish, treasure, junk, successful hooks, casts, an estimated experience total, and time spent running. Hook rate is successful hooks over casts. Treasure rate is treasure over casts. The player can reset the ledger.

## The panel (appearance)

The panel is a **tabbed card** opened with O:

- Tabs along the top: **Loot**, **Enchants**, **Behavior**, **Stats**.
- **Loot**: a scrollable list of the twenty-one catch items (in the order above), each row showing the item, a Keep/Toss chip, and a chime chip. A search field filters the list. Every row is reachable by scrolling, including the last (Leather). A track-and-thumb scrollbar on the right shows how far the list runs.
- **Enchants**: a scrollable list of known enchants, each with chips for Toss, Any, or a minimum level. This applies when a book, rod, or bow row is set to Keep. The same scrollbar and reachability rules apply.
- **Behavior**: one labeled row per option, each with an on/off chip (pace variance, look-around, meals, worn rod, leave if hurt, idle auto-off, overlay).
- **Stats**: the session ledger and a clear-ledger control.

There are no separate settings or stats buttons parked in a corner, and row labels are written as chips (Keep / Toss, On / Off), not a copied “Item: STATUS” sentence. Colors are Hookline’s own (deep teal panel, coral toss chips, sand-colored text, amber accents).

## Safety and limits

- Hookline only runs on the client window when the mod is enabled and a player exists.
- Multiplayer actions use ordinary client packets (use item, change held slot, inventory click, use item on a block). The helper must not drive the single-player authoritative use/drop/hand APIs while connected to a server.
- This specification is for a from-scratch implementation intended to be published by its author. It describes behavior the author already ships, including catch-list order and extras they added themselves.
