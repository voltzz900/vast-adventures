# Registry page: Vast Adventures

- **File:** `vastadventures-0.10.0.hmod` (release v0.10.0), kind: native mod
- **Icon:** `icons/vastadventures.png`
- **Name:** Vast Adventures
- **Summary:** Loot with rarities and affixes, gems and sockets, bosses, elites, rogue spawners and boss dungeons.
- **Categories:** adventure, equipment, mobs
- **Tags:** loot, affixes, gems, bosses, rpg, dungeons, translation
- **License:** MIT
- **Source:** https://github.com/voltzz900/vast-adventures
- **Authors:** voltzz900

## Description

A native mod for an adventure layer over the game, built from the MIT code and data of [Apotheosis](https://github.com/Shadows-of-Fire/Apotheosis)' adventure module (by Shadows_of_Fire), with names, words and art of its own, on Horizon's mod API. One package: the code, the gems, materials, seals, names in English and Brazilian Portuguese, and the world generation.

- **Affix loot**: chests, mob drops and fishing give weapons, tools and armour with a rarity (Common to Mythic) and affixes: attribute bonuses, status effects on hit or when hurt, damage reduction, enchantment bonuses, executing, thunderstruck, cleaving, radial mining, telepathic and festive drops, and combat stats (crit chance and damage, life steal, fire and cold damage, armour pierce and shred, dodge, healing received, experience gained, draw speed...). The tooltip shows them.
- **Gems and sockets**: 21 gems at six purities, socketed at the anvil.
- **Salvaging, reforging and gem cutting** at the grindstone and anvil, and the Seals of Sockets, Extraction and Erasure.
- **Bosses and elites**: now and then a monster is a boss with better stats, gear and a gem, or an elite; some bosses only in their biomes. `/vastadventures boss` summons one.
- **World tier** (`/vastadventures tier`: Calm, Wilds, Rising, Peril, Apex): one per world; higher tiers roll rarer loot and give luck.
- **Rogue spawners and boss dungeons** in new chunks (from the content mod); a boss dungeon's spawner becomes a boss when a player comes near.

**What it changes in the game**: loot tables (adds affix loot and gems), the player's attributes from their gear, melee and projectile damage, healing and block experience for the player, natural monster spawns (bosses and elites), the anvil and grindstone results, and block drops for radial and telepathic tools. It adds the `/vastadventures` command. Its combat stats are attributes of its own the game applies and shows like vanilla's. It works through the mod API (hooks, Parts and attributes; no mixins) and reads and writes only the world's `world_tier.txt` in its mod data folder; no network, no other files.

0.10.0 replaces 0.9.0's two files (`vastadventures_native` .hnmod and the `vastadventures` content mod); gear, gems and sockets made with 0.9 keep working, and a world's tier goes back to Calm until set again. Pinned to Horizon 702ab1e. Not yet: the augmenting and reforging tables, the tier screen, and the effects Horizon has no equivalent for (listed in the README). Not affiliated with or endorsed by Apotheosis' authors; none of its assets or lang are used.
