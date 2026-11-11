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

### Not yet

- Affixes and gem bonuses on attributes Horizon doesn't have: crit chance, life steal, fire and cold damage,
  armour pierce/shred, draw speed and the other Apothic Attributes ones. Gems whose bonuses are all of these
  (Earth, Ender Surge, Inferno, Mageslayer) don't drop yet; a gem goes only into an item it has a working bonus
  for.
- The effect and ability affixes and gem bonuses (bleeding, executing, enchantments, durability...).
- Removing gems and adding sockets (the sigils), gem cutting, bosses and elites, the tier screen and its unlocks,
  and the reforging, salvaging and augmenting tables.

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
`apply_enchantment_attributes` (for the stacks' modifiers) and the anvil's `native_anvil_update`.

## License

MIT, see [LICENSE](LICENSE).

Apotheosis is by Shadows_of_Fire (Stormraven Studios, LLC). Its code and data are MIT
([NOTICE-apotheosis.txt](NOTICE-apotheosis.txt)), and this port carries its data files. Its assets
(textures, models, sounds and lang) are all rights reserved and none are used. The names and words in this
port are its own. Not affiliated with or endorsed by Apotheosis' authors.
