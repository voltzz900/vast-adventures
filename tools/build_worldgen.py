"""Apotheosis' rogue spawners and boss dungeons as Horizon (26.3) world generation data, for the content mod.

Apotheosis places them with Java features of its own. Here they are structure templates placed by the game's
`minecraft:template` feature, with block predicates in place of the features' checks:

- A **rogue spawner** (one of Apotheosis' nine, `vendor/apotheosis/rogue_spawners`) stands on a cave floor: its
  chest below (the spawner kind's loot, now and then the valuable chest's), the spawner, and a cover block above.
- A **boss dungeon** is a stone brick room set into solid rock with an opening or more, two dungeon chests and a
  boss spawner in the middle; the second kind is Apotheosis' own room (`boss_1.nbt`). The boss spawner is a vanilla
  spawner whose mob carries the tag `apotheosis.boss_spawner`: the native mod turns that mob into a boss and takes
  the spawner away, as Apotheosis' boss spawner does when a player comes near.

`files()` returns {path in the content mod: bytes or JSON}.
"""
import json
from pathlib import Path

import nbtw as n

ROOT = Path(__file__).resolve().parent.parent
VENDOR = ROOT / "vendor/apotheosis"
DATA_VERSION = 5023  # 26.3's, as its own structure templates carry
BOSS_TAG = "apotheosis.boss_spawner"
VALUABLE_CHANCE = 0.11  # AdventureConfig.spawnerValueChance
COVERS = ["minecraft:mossy_cobblestone", "minecraft:cracked_stone_bricks", "minecraft:crying_obsidian",
          "minecraft:lodestone"]
RENAMED = {"minecraft:chain": "minecraft:iron_chain"}  # renamed since the template was saved


# ---- NBT from Apotheosis' JSON --------------------------------------------------------------------------------

BYTES = {"amplifier", "show_particles", "show_icon", "ambient"}
FLOATS = {"Health"}


def typed(value, key=None):
    if isinstance(value, dict):
        renamed = {("show_icon" if k == "visible" else k): v for k, v in value.items()}
        return {k: typed(v, k) for k, v in renamed.items()}
    if isinstance(value, list):
        items = [typed(v) for v in value]
        return n.List(n.tag_id(items[0]) if items else 0, items)
    if isinstance(value, bool):
        return n.Byte(value)
    if isinstance(value, int):
        return n.Byte(value) if key in BYTES else n.Int(value)
    if isinstance(value, float):
        return n.Float(value) if key in FLOATS else n.Double(value)
    return value


def spawner(potentials, stats, entity_extra=None):
    """A spawner's block entity: weighted `potentials` [(entity JSON, weight)] and Apothic Spawners' `stats`."""
    def entity(e):
        e = dict(e)
        if entity_extra:
            e.update(entity_extra)
        if stats.get("apothic_spawners:youthful"):
            e["IsBaby"] = True
        return typed(e)

    stat = lambda key, default: n.Short(int(stats.get(f"apothic_spawners:{key}", default)))
    first = entity(potentials[0][0])
    return {
        "id": "minecraft:mob_spawner",
        "SpawnData": {"entity": first},
        "SpawnPotentials": n.List(10, [{"data": {"entity": entity(e)}, "weight": n.Int(w)} for e, w in potentials]),
        "Delay": n.Short(20),
        "MinSpawnDelay": stat("min_delay", 200),
        "MaxSpawnDelay": stat("max_delay", 800),
        "SpawnCount": stat("spawn_count", 4),
        "MaxNearbyEntities": stat("max_nearby_entities", 6),
        "RequiredPlayerRange": stat("req_player_range", 16),
        "SpawnRange": stat("spawn_range", 4),
    }


def template(size, blocks):
    """A structure template: `blocks` [(pos, block id, properties or None, block entity or None)]."""
    palette, states, out = [], {}, []
    for pos, block, props, nbt in blocks:
        block = RENAMED.get(block, block)
        key = (block, tuple(sorted((props or {}).items())))
        if key not in states:
            states[key] = len(palette)
            entry = {"id": block}
            if props:
                entry["properties"] = dict(sorted(props.items()))
            palette.append(entry)
        info = {"pos": n.List(3, [n.Int(c) for c in pos]), "state": n.Int(states[key])}
        if nbt is not None:
            info["nbt"] = nbt
        out.append(info)
    return n.write({
        "DataVersion": n.Int(DATA_VERSION),
        "size": n.List(3, [n.Int(c) for c in size]),
        "palette": n.List(10, palette),
        "blocks": n.List(10, out),
        "entities": n.List(0),
    })


def chest(loot, facing="north"):
    return ("minecraft:chest", {"facing": facing, "type": "single", "waterlogged": "false"},
            {"id": "minecraft:chest", "LootTable": loot})


# ---- placement -----------------------------------------------------------------------------------------------

def height(low, high):
    return {"type": "minecraft:height_range", "height": {"type": "minecraft:uniform", "min_inclusive": low,
                                                          "max_inclusive": high}}


SHALLOW = ({"absolute": 10}, {"below_top": 0})
SHALLOW_BOSS = ({"absolute": 0}, {"below_top": 0})
DEEP = ({"above_bottom": 6}, {"absolute": -1})


def solid(dx, dy, dz):
    return {"type": "minecraft:solid", "offset": [dx, dy, dz]}


def air(dx, dy, dz):
    return {"type": "minecraft:matching_blocks", "blocks": ["minecraft:air", "minecraft:cave_air"], "offset": [dx, dy, dz]}


def placed(feature, count, heights, predicate, rarity=None):
    placement = [{"type": "minecraft:count", "count": count}]
    if rarity:
        placement.append({"type": "minecraft:rarity_filter", "chance": rarity})
    placement += [{"type": "minecraft:in_square"}, height(*heights),
                  {"type": "minecraft:block_predicate_filter", "predicate": predicate}, {"type": "minecraft:biome"}]
    return {"feature": feature, "placement": placement}


# ---- rogue spawners ------------------------------------------------------------------------------------------

def rogue_spawners(files):
    entries = []
    for path in sorted((VENDOR / "rogue_spawners").glob("*/*.json")):
        d = json.loads(path.read_text())
        name = f"{path.parent.name}/{path.stem}"
        potentials = [(p["data"]["entity"], p.get("weight", 1)) for p in d["spawn_potentials"]]
        # Now and then the chest holds the valuable chest's loot instead: a template of its own.
        weight = d.get("weight", 1) * 100
        for suffix, loot, share in (("", d["loot_table"], 1 - VALUABLE_CHANCE),
                                    ("_valuable", "apotheosis:chests/chest_valuable", VALUABLE_CHANCE)):
            blocks = [
                ((0, 0, 0), *chest(loot)),
                ((0, 1, 0), "minecraft:spawner", None, spawner(potentials, d.get("stats", {}))),
                ((0, 2, 0), COVERS[0], None, None),
            ]
            files[f"data/apotheosis/structure/rogue_spawner/{name}{suffix}.nbt"] = template((1, 3, 1), blocks)
            entries.append({"data": {"id": f"apotheosis:rogue_spawner/{name}{suffix}"}, "weight": round(weight * share)})
    # The cover is one of four (the rules apply in turn).
    rules = [{"input_predicate": {"predicate_type": "minecraft:random_block_match", "block": COVERS[0],
                                  "probability": 1 / (len(COVERS) - i)},
              "location_predicate": {"predicate_type": "minecraft:always_true"}, "output_state": cover}
             for i, cover in enumerate(COVERS[1:])]
    files["data/apotheosis/worldgen/processor_list/rogue_spawner.json"] = {
        "processors": [{"processor_type": "minecraft:rule", "rules": rules}]}
    files["data/apotheosis/worldgen/feature/rogue_spawner.json"] = {
        "type": "minecraft:template", "templates": entries, "processors": "apotheosis:rogue_spawner"}
    # RogueSpawnerFeature: stone below the spawner, the spawner's place air or stone, air above it. The template's
    # origin is the chest, on the floor.
    stone = {"type": "minecraft:matching_block_tag", "tag": "minecraft:base_stone_overworld"}
    predicate = {"type": "minecraft:all_of", "predicates": [
        stone,
        {"type": "minecraft:any_of", "predicates": [air(0, 1, 0), dict(stone, offset=[0, 1, 0])]},
        air(0, 2, 0),
    ]}
    files["data/apotheosis/worldgen/placed_feature/rogue_spawner.json"] = placed(
        "apotheosis:rogue_spawner", 65, SHALLOW, predicate, rarity=100)
    files["data/apotheosis/worldgen/placed_feature/rogue_spawner_deep.json"] = placed(
        "apotheosis:rogue_spawner", 85, DEEP, predicate, rarity=100)


# ---- boss dungeons -------------------------------------------------------------------------------------------

def boss_spawner():
    """The boss dungeon's spawner: one marked zombie when a player comes within 8 blocks."""
    rules = {"block_light_limit": n.List(3, [n.Int(0), n.Int(15)]), "sky_light_limit": n.List(3, [n.Int(0), n.Int(15)])}
    nbt = spawner([({"id": "minecraft:zombie"}, 1)],
                  {"apothic_spawners:min_delay": 20, "apothic_spawners:max_delay": 40,
                   "apothic_spawners:spawn_count": 1, "apothic_spawners:max_nearby_entities": 1,
                   "apothic_spawners:req_player_range": 8, "apothic_spawners:spawn_range": 1},
                  {"Tags": [BOSS_TAG], "PersistenceRequired": True, "Silent": True})
    nbt["Delay"] = n.Short(0)
    nbt["SpawnData"]["custom_spawn_rules"] = rules
    for p in nbt["SpawnPotentials"]:
        p["data"]["custom_spawn_rules"] = rules
    return nbt


def dungeon_room(rx, rz):
    """BossDungeonFeature's room, `rx` by `rz` from its middle: floor y 0, roof y 5, walls of stone bricks (the
    processor keeps the rock's openings and weathers the bricks), two chests against the walls, the spawner."""
    blocks = []
    for x in range(2 * rx + 1):
        for z in range(2 * rz + 1):
            for y in range(6):
                edge = x in (0, 2 * rx) or z in (0, 2 * rz) or y in (0, 5)
                blocks.append(((x, y, z), "minecraft:stone_bricks" if edge else "minecraft:cave_air", None, None))
    blocks = [b for b in blocks if b[0] not in ((1, 1, rz), (2 * rx - 1, 1, rz), (rx, 1, rz))]
    blocks.append(((1, 1, rz), *chest("minecraft:chests/simple_dungeon", "east")))
    blocks.append(((2 * rx - 1, 1, rz), *chest("minecraft:chests/simple_dungeon", "west")))
    blocks.append(((rx, 1, rz), "minecraft:spawner", None, boss_spawner()))
    return template((2 * rx + 1, 6, 2 * rz + 1), blocks)


def boss_room_2(flip):
    """Apotheosis' second room (`boss_1.nbt`, 9 by 9), with its two chests in opposite corners and the spawner."""
    root = n.read((VENDOR / "structure/boss_1.nbt").read_bytes())
    palette = [{"id": RENAMED.get(p.get("Name") or p["id"], p.get("Name") or p["id"]),
                **({"properties": p.get("Properties") or p.get("properties")} if (p.get("Properties") or p.get("properties")) else {})}
               for p in root["palette"]]
    blocks = []
    for b in root["blocks"]:
        entry = palette[b["state"]]
        blocks.append((tuple(b["pos"]), entry["id"], entry.get("properties"), b.get("nbt")))
    corners = [(1, 1, 1), (7, 1, 7)] if not flip else [(7, 1, 1), (1, 1, 7)]
    special = {(4, 1, 4): ("minecraft:spawner", None, boss_spawner())}
    for i, c in enumerate(corners):
        special[c] = chest("minecraft:chests/simple_dungeon", "south" if c[2] == 1 else "north")
    blocks = [b for b in blocks if b[0] not in special] + [(p, *v) for p, v in special.items()]
    return template(tuple(root["size"]), blocks)


def boss_dungeons(files):
    rooms = []
    for rx in (3, 4, 5):
        for rz in (3, 4, 5):
            files[f"data/apotheosis/structure/boss_dungeon/room_{rx}_{rz}.nbt"] = dungeon_room(rx, rz)
            rooms.append((rx, rz))
    weather = [{"input_predicate": {"predicate_type": "minecraft:block_match", "block": "minecraft:stone_bricks"},
                "location_predicate": {"predicate_type": "minecraft:block_match", "block": block},
                "output_state": "minecraft:cave_air"}
               for block in ("minecraft:air", "minecraft:cave_air")]
    weather += [{"input_predicate": {"predicate_type": "minecraft:random_block_match", "block": "minecraft:stone_bricks",
                                     "probability": p},
                 "location_predicate": {"predicate_type": "minecraft:always_true"}, "output_state": out}
                for out, p in (("minecraft:mossy_stone_bricks", 1 / 3), ("minecraft:cracked_stone_bricks", 1 / 4))]
    files["data/apotheosis/worldgen/processor_list/boss_dungeon.json"] = {
        "processors": [{"processor_type": "minecraft:rule", "rules": weather}]}
    # One feature per room size, as the predicates follow the room.
    for rx, rz in rooms:
        files[f"data/apotheosis/worldgen/feature/boss_dungeon_{rx}_{rz}.json"] = {
            "type": "minecraft:template", "processors": "apotheosis:boss_dungeon",
            "templates": [{"data": {"id": f"apotheosis:boss_dungeon/room_{rx}_{rz}",
                                    "rotations": ["none", "180"]}, "weight": 1}]}
    for flip in (False, True):
        files[f"data/apotheosis/structure/boss_dungeon/room_2_{int(flip)}.nbt"] = boss_room_2(flip)
    files["data/apotheosis/worldgen/feature/boss_dungeon_2.json"] = {
        "type": "minecraft:template",
        "templates": [{"data": {"id": f"apotheosis:boss_dungeon/room_2_{i}"}, "weight": 1} for i in (0, 1)]}

    def room_predicate(rx, rz, roof):
        # Solid floor and roof (a grid of points across them) and an opening at the height of a door in a wall.
        xs, zs = (-rx, 0, rx), (-rz, 0, rz)
        checks = [solid(x, 0, z) for x in xs for z in zs] + [solid(x, roof, z) for x in xs for z in zs]
        doors = [{"type": "minecraft:all_of", "predicates": [air(x, 1, z), air(x, 2, z)]}
                 for x, z in ((-rx, 0), (rx, 0), (0, -rz), (0, rz))]
        return {"type": "minecraft:all_of", "predicates": checks + [{"type": "minecraft:any_of", "predicates": doors}]}

    biome_features = {"boss_dungeon": [], "boss_dungeon_deep": []}
    for rx, rz in rooms:
        for suffix, count, heights in (("", 6, SHALLOW_BOSS), ("_deep", 8, DEEP)):
            # The room's size is rolled per attempt in Apotheosis: its count is shared among the nine sizes.
            name = f"boss_dungeon_{rx}_{rz}{suffix}"
            # The predicates are looser than BossDungeonFeature's (a solid floor and roof all across, at most five
            # openings), so fewer attempts: about as many dungeons as Apotheosis makes, from what the game shows.
            files[f"data/apotheosis/worldgen/placed_feature/{name}.json"] = placed(
                f"apotheosis:boss_dungeon_{rx}_{rz}", 1, heights, room_predicate(rx, rz, 5), rarity=round(27 / count))
            biome_features[f"boss_dungeon{suffix}"].append(f"apotheosis:{name}")
    for suffix, count, heights in (("", 6, SHALLOW_BOSS), ("_deep", 8, DEEP)):
        name = f"boss_dungeon_2{suffix}"
        # BossDungeonFeature2: the room is placed from 4 west, 1 down and 4 north of its middle.
        files[f"data/apotheosis/worldgen/placed_feature/{name}.json"] = placed(
            "apotheosis:boss_dungeon_2", count, heights, room_predicate(4, 4, 4), rarity=3)
        biome_features[f"boss_dungeon{suffix}"].append(f"apotheosis:{name}")
    return biome_features


def loot_26_3(value):
    """A 1.21 loot table in 26.3's shape: `functions` become `modifier` (with `type`), `conditions` `condition`."""
    if isinstance(value, list):
        return [loot_26_3(v) for v in value]
    if not isinstance(value, dict):
        return value
    out = {}
    for key, v in value.items():
        if key == "functions":
            mods = [loot_26_3(f) for f in v]
            out["modifier"] = mods[0] if len(mods) == 1 else mods
        elif key == "conditions":
            conds = [loot_26_3(c) for c in v]
            out["condition"] = conds[0] if len(conds) == 1 else {"type": "minecraft:all_of", "terms": conds}
        elif key in ("function", "condition") and isinstance(v, str):
            out["type"] = v
        else:
            out[key] = loot_26_3(v)
    return out


def files():
    out = {}
    rogue_spawners(out)
    dungeons = boss_dungeons(out)
    for path in sorted((VENDOR / "loot_table").glob("*.json")):
        out[f"data/apotheosis/loot_table/chests/{path.name}"] = loot_26_3(json.loads(path.read_text()))
    modifiers = {"rogue_spawner": ["apotheosis:rogue_spawner", "apotheosis:rogue_spawner_deep"], **dungeons}
    for name, features in modifiers.items():
        out[f"data/apotheosis/neoforge/biome_modifier/{name}.json"] = {
            "type": "neoforge:add_features", "biomes": "#minecraft:is_overworld", "features": features,
            "step": "underground_structures"}
    return out
