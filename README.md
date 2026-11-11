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

## 0.4: salvaging, reforging and gem cutting

At the game's own stations, with Apotheosis' materials (items drawn for this port):

- **Salvaging** (grindstone): an affix item alone salvages into one to four of its rarity's material:
  Mysterious Scrap Metal (Common), Timeworn Fabric (Uncommon), Luminous Crystal Shard (Rare), Arcane Sands
  (Epic), Godforged Pearl (Mythic). A gem salvages into Gem Dust, more the purer it is.
- **Reforging** (anvil): an affix item on the left and its rarity's material on the right (1 to 3) reroll its
  affixes at the same rarity, for 2 to 50 levels. Its sockets and gems stay.
- **Gem cutting** (anvil): a gem and Gem Dust (1, 3, 5, 7 or 9) make it one purity better, for 3 to 15 levels.

The result is worked out from the item itself, so it is the same on every look (and a reforged item rerolls
differently next time). Apotheosis' sigils, which reforging and cutting also take there, aren't ported yet.

## 0.5: sigils and the tier's luck

- **Gem-Fused Slate**: eight deepslate around a Gem Dust make eight; the sigils are made from it.
- **Sigil of Socketing** (anvil, with the item): one more socket, up to two.
- **Sigil of Withdrawal** (anvil): takes every gem out of an item; the gems pop out on top of the anvil.
- **Sigil of Unnaming** (anvil): takes the affix name off an item.
- **World tier**: the player's luck grows by the tier (+20% to +100%), as Apotheosis' tier augments do.

## 0.6: effect and ability affixes

Affixes and gem bonuses that do more than add an attribute, from Apotheosis' data, shown in the tooltip in green:

- **Status effects**: a weapon's hit gives the target (or the wielder) an effect such as Weakness or Slowness, a
  bow's arrow does the same where it lands, armour gives one when its wearer is hurt (or to the attacker), and a
  tool when it breaks a block. Each has Apotheosis' cooldown. Only the game's own effects; Apothic ones wait.
- **Damage reduction** (armour): less fall, fire, magic, explosion or projectile damage, up to 95% in all.
- **Enchantment bonuses**: extra levels of an enchantment the item has (such as *Prosperous* adding Fortune), of
  one it hasn't, or of all of them, as the affix says.
- **Executing** (a hit that leaves a monster below a share of its health kills it), **Thunderstruck** (lightning
  that hurts the monsters around the target), **Cleaving** (the hit also lands on monsters nearby).
- **Radial** (tools): breaks the blocks around the one mined, in Apotheosis' shapes, but only those the tool is
  made for and none that can't be broken.
- **Telepathic**: what the item breaks or kills goes straight into the inventory.
- **Festive**: now and then a kill drops its loot more than once.

## Not yet

- Affixes and gem bonuses on attributes Horizon doesn't have: crit chance, life steal, fire and cold damage,
  armour pierce/shred, draw speed and the other Apothic Attributes ones. Gems whose bonuses are all of these
  (Earth, Ender Surge, Inferno, Mageslayer) don't drop yet; a gem goes only into an item it has a working bonus
  for.
- The effects that need the Apothic attributes or effects (bleeding, sundering, knowledge...), shield affixes,
  durability affixes, the spectral and magical arrows, and Psychic, Retreating, Catalyzing, Omnetic,
  Stoneforming and Enlightened.
- The Sigils of Rebirth and Enhancement (they fuel the reforging and augmenting tables, here the anvil),
  Malice and Supremacy (from Gateways), the tier screen and its unlocks, and the augmenting table.
- Tier augments for monsters (armour, toughness): Horizon's mobs have no such attributes yet.
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
`native_mobs_joined` (for bosses and elites), the grindstone's `grindstone_result` and `grindstone_accepts`,
the anvil's `anvil_take` and `anvil_used` (for sigils) and `ItemStack::enchantment_level` (for enchantment bonuses).
Its `api` folder is an empty trait mod: the game asks some hook points, the player's melee hit among them, only
while a trait mod is loaded.

## License

MIT, see [LICENSE](LICENSE).

Apotheosis is by Shadows_of_Fire (Stormraven Studios, LLC). Its code and data are MIT
([NOTICE-apotheosis.txt](NOTICE-apotheosis.txt)), and this port carries its data files. Its assets
(textures, models, sounds and lang) are all rights reserved and none are used. The names and words in this
port are its own. Not affiliated with or endorsed by Apotheosis' authors.
