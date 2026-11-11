# Vast Adventures

A native mod and a content mod for [Horizon](https://github.com/horizon-rust/horizon): loot with rarities and
affixes, gems and sockets, bosses, elites, rogue spawners and boss dungeons. It is built from the MIT code and data
of [Apotheosis](https://github.com/Shadows-of-Fire/Apotheosis)' adventure module (by Shadows_of_Fire), with names,
words and art of its own. Until 0.8 it was called "Apotheosis for Horizon".

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
- **World tier**: `/vastadventures tier` shows it, and `/vastadventures tier <calm|wilds|rising|peril|apex>`
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
  the time by dimension and world tier (none in Calm), at most one every three minutes. A boss is one of the
  dimension's bosses (some only in their biomes, from 0.8), at a rarity its stats list, with:
  - more health, speed, damage and knockback, rolled from Apotheosis' stats for that rarity;
  - fire resistance, and a glow for three minutes;
  - its tier's gear set (iron to netherite), one piece of it made affix loot at the boss's rarity, which
    always drops;
  - a name in the rarity's colour, such as *Koreth the Cruel* (this port's own words);
  - 30% of the time a gem in its off hand, which also always drops.
  The chat says where it appeared. `/vastadventures boss` (operators) summons one at the player.
- **Elites**: some skeletons (the Grave Knight and the Hive Archer) and wither skeletons (the Withering
  Archer) spawn stronger, named, in their tier's gear, sometimes with one affix item that always drops.

## 0.4: salvaging, reforging and gem cutting

At the game's own stations, with the materials (items drawn for this port):

- **Salvaging** (grindstone): an affix item alone salvages into one to four of its rarity's material:
  Rusted Scrap (Common), Faded Cloth (Uncommon), Glow Shard (Rare), Starsand
  (Epic), Titan Pearl (Mythic). A gem salvages into Gem Dust, more the purer it is.
- **Reforging** (anvil): an affix item on the left and its rarity's material on the right (1 to 3) reroll its
  affixes at the same rarity, for 2 to 50 levels. Its sockets and gems stay.
- **Gem cutting** (anvil): a gem and Gem Dust (1, 3, 5, 7 or 9) make it one purity better, for 3 to 15 levels.

The result is worked out from the item itself, so it is the same on every look (and a reforged item rerolls
differently next time). Apotheosis' sigils for reforging and cutting aren't ported yet.

## 0.5: seals and the tier's luck

- **Runed Slate**: eight deepslate around a Gem Dust make eight; the seals are made from it.
- **Seal of Sockets** (anvil, with the item): one more socket, up to two.
- **Seal of Extraction** (anvil): takes every gem out of an item; the gems pop out on top of the anvil.
- **Seal of Erasure** (anvil): takes the affix name off an item.
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

## 0.7: Apothic attributes

Affixes and gem bonuses on the attributes Apotheosis takes from Apothic Attributes now roll and work, carried out
by this mod (the tooltip lists them in blue):

- **Critical chance and damage**: a hit's chance to deal more, rolled again past 100% (each further crit weaker).
  A player without such gear never crits this way (Apothic gives everyone 5%; this port keeps the vanilla game).
- **Life steal**: melee damage dealt heals you by a share.
- **Damage from the target's health**, **fire damage** and **cold damage**: more melee damage, scaled by the swing.
- **Armour pierce** and **shred**: melee hits go through some of the target's armour.
- **Dodge chance**: a share of the hits from other creatures miss you.
- **Healing received**, **experience gained** (from blocks), **arrow and projectile damage** and **draw speed**.

## 0.8: rogue spawners, boss dungeons and biome bosses

New chunks of the overworld get Apotheosis' underground finds, as structure templates and world generation data
in the content mod:

- **Rogue spawners**: a spawner on a cave floor with a chest under it and a cover block over it. It is one of
  Apotheosis' nine (brutal zombies, husks and pillagers, or swarms of spiders, cave spiders, silverfish and baby
  zombies), with their stronger mobs and faster spawning, and the chest holds its kind's loot or, now and then,
  the valuable chest's.
- **Boss dungeons**: a stone brick room set into the rock, open where caves meet it, with two dungeon chests
  and a boss spawner in the middle; or Apotheosis' own second room. When a player comes within 8 blocks the
  spawner goes and a boss of the dimension's stands in its place.
- **Bosses tied to a biome** now come too: the bogged in wet biomes, the husk in dry ones, the stray in cold ones
  and the wolf pack leader in snowy ones (this port's own lists of the game's biomes for Apotheosis' tags).

## 0.9: Vast Adventures

The port takes a name of its own. Its mods are now `vastadventures_native` and `vastadventures` (items
`vastadventures:...`, the command `/vastadventures`), its world tiers Calm, Wilds, Rising, Peril and Apex, and its
materials, seals, gems and elites have names of their own. The second kind of boss dungeon is a deepslate hall of
this port's own. Items and worlds from 0.8 and before carry the old ids.

## Not yet

- The Apothic attributes still missing: overheal, protection pierce and shred, arrow velocity, cooldown
  reduction, elytra and creative flight, and those Apotheosis gives only to bosses. Their affixes don't roll, and
  a gem goes only into an item it has a working bonus for.
- The effects that need the Apothic attributes or effects (bleeding, sundering, knowledge...), shield affixes,
  durability affixes, the spectral and magical arrows, and Psychic, Retreating, Catalyzing, Omnetic,
  Stoneforming and Enlightened.
- Seals like Apotheosis' Sigils of Rebirth and Enhancement (they fuel the reforging and augmenting tables, here the anvil),
  Malice and Supremacy (from Gateways), the tier screen and its unlocks, and the augmenting table.
- Tier augments for monsters (armour, toughness): Horizon's mobs have no such attributes yet.
- For bosses and elites: scale, armour and the Apothic attributes in their stats, the enchanting of their gear,
  the beam over a new boss, and the tier augments that toughen every monster.
- Apotheosis' towers (with the tomes and its painting) and its other world generation; the spawners'
  rules beyond their timing (Apothic Spawners is a mod of its own).
- The vines around a rogue spawner.

## Install

- **The native mod:** `vastadventures_native-<version>.hnmod`. Install it, then build natives in the launcher.
- **The content mod:** `vastadventures-<version>.hmod`, with the gem items and the names in English and Brazilian
  Portuguese. Install both.

## Building

- `tools/gen_data.py <Apotheosis jar>` regenerates `mixins/vastadventures_native/data.rs` from Apotheosis' data files.
- `tools/build_content.py` builds `content/vastadventures` (gem items, sprites, lang, and with `tools/build_worldgen.py`
  the rogue spawners and boss dungeons, from Apotheosis' data in `vendor/vastadventures`).
- `cargo test` runs the tests: `src/lib.rs` builds the injected modules as an ordinary crate.
- `tools/package.py` packs both files into `dist/`.

It is pinned to Horizon `fdc87ae` (Nightly 2026-11-08): besides the native hook points it hooks the game's own
`apply_enchantment_attributes` (for the stacks' modifiers), the anvil's `native_anvil_update` and
`native_mobs_joined` (for bosses and elites; the boss spawner's mob is found in the server tick), the grindstone's `grindstone_result` and `grindstone_accepts`,
the anvil's `anvil_take` and `anvil_used` (for seals) and `ItemStack::enchantment_level` (for enchantment bonuses).
Its `api` folder is an empty trait mod: the game asks some hook points, the player's melee hit among them, only
while a trait mod is loaded.

## License

MIT, see [LICENSE](LICENSE).

Apotheosis is by Shadows_of_Fire (Stormraven Studios, LLC). Its code and data are MIT
([NOTICE-apotheosis.txt](NOTICE-apotheosis.txt)), and Vast Adventures carries its data files (with its ids renamed). Its assets
(textures, models, sounds and lang) are all rights reserved and none are used. The names and words in this
port are its own. Not affiliated with or endorsed by Apotheosis' authors.
