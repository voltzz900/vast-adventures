# Apotheosis for Horizon

A native mod and a content mod for [Horizon](https://github.com/horizon-rust/horizon): a port of
[Apotheosis](https://github.com/Shadows-of-Fire/Apotheosis)' adventure module, one slice at a time.

## 0.1: rarities and affixes

Loot gets rarities and affixes the way Apotheosis rolls them, from Apotheosis' own data:

- **Chests** sometimes hold an extra affix item (35% of the game's chest tables, 30% of other mods'): a weapon,
  tool, bow or armour piece drawn from Apotheosis' loot list.
- **Gear any loot table drops** (chests, mobs, fishing; not blocks) turns into affix loot 35% of the time.
- Only a player's loot rolls: the one who opened the chest or made the kill.
- Each item gets a **rarity** (Common, Uncommon, Rare, Epic, Mythic), weighted by the world tier and the
  player's luck. Its **affixes** come from the rarity's rules, and its **name** is built from them in the
  rarity's colour, for example *Fierce Diamond Sword of Fury*. The rarity shows below the name.
- **Affixes** are attribute bonuses the game applies itself, in the item's slot, alongside the item's own
  bonuses. Examples: attack damage, attack speed, reach, armour, toughness, health, speed, knockback
  resistance and mining speed. Their size depends on the rarity and on a roll.
- **World tier**: `/apotheosis tier` shows it, and `/apotheosis tier <haven|frontier|ascent|summit|pinnacle>`
  sets it. Higher tiers roll rarer items. It is one tier for the whole world, saved with it. Anyone may set it
  until Apotheosis' tier unlocks are ported.

## 0.2: sockets and gems

- **Sockets**: affix items get sockets from their rarity's rules (none to four; mythic items usually one to three).
  The tooltip lists them under the rarity: an empty one, or the gem in it.
- **Gems**: Apotheosis' 21 gems are items of their own, each with a sprite drawn for this port, at one of six
  purities (Cracked to Perfect, Perfect ones glint) rolled by the world tier and luck. They drop from monsters a
  player kills (4.5%) and turn up in chests (25% of the game's, 20% of other mods'); the nether, overworld and
  end gems only in their dimension and higher tiers. A gem's tooltip lists what it gives in each kind of item.
- **Socketing**: put an item with a free socket on the left of an anvil and a gem on the right; the result, for a
  level, has the gem in its first free socket and the gem's bonus for that kind of item. A unique gem goes once
  per item.
- **Bonuses that work**: an affix's or gem's attribute bonuses now really apply to the player and show in the
  tooltip. Horizon applies only an item kind's own modifiers, so this mod adds a stack's from its data (0.1's
  affixes were stored but had no effect).

## 0.3: bosses and elites

- **Bosses** (Apotheosis' invaders): a monster the world spawns is now and then a boss instead, 1.8% to 3.5% of
  the time by dimension and world tier (none in Haven), at most one every three minutes. A boss is one of the
  dimension's 20 (bosses tied to a biome wait for a later version), at a rarity its stats list, with:
  - more health, speed, damage and knockback, rolled from Apotheosis' stats for that rarity;
  - fire resistance, and a glow for three minutes;
  - its tier's gear set (iron to netherite), one piece of it made affix loot at the boss's rarity, which
    always drops;
  - a name in the rarity's colour, such as *Koreth the Cruel* (this port's own words);
  - 30% of the time a gem in its off hand, which also always drops.
  The chat says where it appeared. `/apotheosis boss` (operators) summons one at the player.
- **Elites**: some skeletons (the Undead Knight and the Honeyed Archer) and wither skeletons (the Withering
  Archer) spawn stronger, named, in their tier's gear, sometimes with one affix item that always drops.

### Not yet

- Affixes and gem bonuses on attributes Horizon doesn't have: crit chance, life steal, fire and cold damage,
  armour pierce/shred, draw speed and the other Apothic Attributes ones. Gems whose bonuses are all of these
  (Earth, Ender Surge, Inferno, Mageslayer) don't drop yet; a gem goes only into an item it has a working bonus
  for.
- The effect and ability affixes and gem bonuses (bleeding, executing, enchantments, durability...).
- Removing gems and adding sockets (the sigils), gem cutting, the tier screen and its unlocks, and the
  reforging, salvaging and augmenting tables.
- For bosses and elites: mounts and riders, scale, armour and the Apothic attributes in their stats, the
  enchanting of their gear, the beam over a new boss, boss dungeons and rogue spawners, and the tier augments
  that toughen every monster.

## Install

- **The native mod:** `apotheosis_native-<version>.hnmod`. Install it, then build natives in the launcher.
- **The content mod:** `apotheosis-<version>.hmod`, with the gem items and the names in English and Brazilian
  Portuguese. Install both.

## Building

- `tools/gen_data.py <Apotheosis jar>` regenerates `mixins/apotheosis_native/data.rs` from Apotheosis' data files.
- `tools/build_content.py` builds `content/apotheosis` (gem items, sprites, lang).
- `cargo test` runs the tests: `src/lib.rs` builds the injected modules as an ordinary crate.
- `tools/package.py` packs both files into `dist/`.

It is pinned to Horizon `fdc87ae` (Nightly 2026-11-08): besides the native hook points it hooks the game's own
`apply_enchantment_attributes` (for the stacks' modifiers), the anvil's `native_anvil_update` and
`native_mobs_joined` (for bosses and elites).

## License

MIT, see [LICENSE](LICENSE).

Apotheosis is by Shadows_of_Fire (Stormraven Studios, LLC). Its code and data are MIT
([NOTICE-apotheosis.txt](NOTICE-apotheosis.txt)), and this port carries its data files. Its assets
(textures, models, sounds and lang) are all rights reserved and none are used. The names and words in this
port are its own. Not affiliated with or endorsed by Apotheosis' authors.
