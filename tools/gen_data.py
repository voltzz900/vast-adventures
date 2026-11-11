#!/usr/bin/env python3
"""Writes mixins/apotheosis_native/data.rs from Apotheosis' data files.

Apotheosis' code and data are MIT (Copyright (c) 2018-2025 Stormraven Studios, LLC); its assets (textures, models,
sounds, lang) are all rights reserved, so only data/ is read here, and the names this port shows are its own.

Usage: gen_data.py <the Apotheosis jar or its unpacked data/apotheosis folder>
"""
import json
import sys
import zipfile
from pathlib import Path

ROOT = "data/apotheosis/apotheosis/"
OUT = Path(__file__).resolve().parent.parent / "mixins/apotheosis_native/data.rs"


def files(source):
    """(path under data/apotheosis/apotheosis/, parsed JSON) for every JSON file."""
    source = Path(source)
    if source.suffix == ".jar":
        with zipfile.ZipFile(source) as jar:
            for name in sorted(jar.namelist()):
                if name.startswith(ROOT) and name.endswith(".json"):
                    yield name[len(ROOT):], json.loads(jar.read(name))
    else:
        for path in sorted(source.rglob("*.json")):
            yield path.relative_to(source).as_posix(), json.loads(path.read_text())


ALL = "data/apotheosis/"


def everything(source):
    """(path under data/apotheosis/, parsed JSON) for the gear sets, their tags and the spawn rules."""
    with zipfile.ZipFile(source) as jar:
        for name in sorted(jar.namelist()):
            if name.startswith(ALL) and name.endswith(".json"):
                yield name[len(ALL):], json.loads(jar.read(name))


def main():
    rarities, overrides, affixes, entries, gems, purity = {}, {}, [], [], [], {}
    invaders, elites, gear_sets, gear_tags, spawn_rules = [], [], {}, {}, {}
    for path, value in everything(sys.argv[1]):
        if path.startswith("placebo/gear_sets/"):
            gear_sets["apotheosis:" + path[len("placebo/gear_sets/"):-5]] = value
        elif path.startswith("tags/placebo/gear_sets/"):
            gear_tags["apotheosis:" + path[len("tags/placebo/gear_sets/"):-5]] = [
                v if isinstance(v, str) else v["id"] for v in value["values"]]
        elif path == "data_maps/dimension_type/invader_spawn_rules.json":
            spawn_rules = value["values"]
    for path, value in files(sys.argv[1]):
        kind, _, rest = path.partition("/")
        name = rest[:-len(".json")]
        if kind == "rarities":
            rarities[f"apotheosis:{name}"] = {k: value[k] for k in ("color", "weights", "rules", "sort_index") if k in value}
        elif kind == "rarity_override":
            overrides[value["category"]] = value["overrides"]
        elif kind == "affixes" and value.get("type") == "apotheosis:attribute":
            affixes.append({"id": f"apotheosis:{name}", **{k: value[k] for k in
                            ("attribute", "operation", "values", "categories", "definition") if k in value}})
        elif kind == "gems":
            bonuses = []
            for b in value["bonuses"]:
                gc = b.get("gem_class")
                if isinstance(gc, str):
                    key, types = gc.split(":")[-1], [gc.split(":")[-1]]
                elif isinstance(gc, dict):
                    key = gc["key"]
                    raw = gc.get("types")
                    types = ["*"] if isinstance(raw, dict) else [t.split(":")[-1] for t in raw]
                else:
                    key, types = None, []
                bonus = {"type": b["type"], "key": key, "types": types}
                for k in ("attribute", "operation", "values", "modifiers", "desc"):
                    if k in b:
                        bonus[k] = b[k]
                bonuses.append(bonus)
            gems.append({"id": f"apotheosis:{name}", "weights": value["weights"], "constraints": value.get("constraints", {}),
                         "min_purity": value.get("min_purity", "cracked"), "unique": value.get("unique", False), "bonuses": bonuses})
        elif kind == "apothic_invaders":
            b = value["basic_data"]
            invaders.append({"id": f"apotheosis:{name}", "entity": value["entity"], "weights": b["weights"],
                             "constraints": b.get("constraints", {}), "gear_sets": b.get("valid_gear_sets", {}),
                             "stats": value["stats"]})
        elif kind == "apothic_elites":
            b = value["basic_data"]
            ents = value["entities"]
            elites.append({"id": f"apotheosis:{name}", "entities": [ents] if isinstance(ents, str) else ents,
                           "success_chance": value["success_chance"], "weights": b["weights"],
                           "constraints": b.get("constraints", {}), "gear_sets": b.get("valid_gear_sets", {}),
                           "name": b.get("name"), "stats": value["stats"],
                           "affix_chance": value.get("affix_data", {}).get("affix_chance", -1),
                           "rarities": value.get("affix_data", {}).get("rarities", [])})
        elif kind == "purity_weights":
            purity = value["weights"]
        elif kind == "affix_loot_entries":
            entries.append({"item": value["stack"]["id"], **{k: value[k] for k in ("weights", "rarities", "constraints") if k in value}})
    rarities = dict(sorted(rarities.items(), key=lambda kv: kv[1].get("sort_index", 1000)))

    def const(name, value):
        text = json.dumps(value, separators=(",", ":"), sort_keys=True)
        assert '"##' not in text
        return f'pub const {name}: &str = r##"{text}"##;\n'

    OUT.write_text(
        "// Generated by tools/gen_data.py from Apotheosis' data files (MIT, Copyright (c) 2018-2025 Stormraven\n"
        "// Studios, LLC). Do not edit: run the script again.\n\n"
        "/// Rarities by id, lowest first: colour, per-tier weights and the rules that roll an item.\n"
        + const("RARITIES", rarities)
        + "\n/// Per loot category, the rules a rarity uses instead of its own.\n"
        + const("RARITY_OVERRIDES", overrides)
        + "\n/// Attribute affixes: attribute, operation, values per rarity, categories and definition.\n"
        + const("AFFIXES", affixes)
        + "\n/// The items a chest's affix loot is drawn from, with their weights.\n"
        + const("LOOT_ENTRIES", entries)
        + "\n/// Gems: weights, constraints, lowest purity, whether one per item, and bonuses by item class.\n"
        + const("GEMS", gems)
        + "\n/// Purity weights per world tier.\n"
        + const("PURITY_WEIGHTS", purity)
        + "\n/// Bosses (Apotheosis' invaders): entity, weights, constraints, gear sets per tier and stats per rarity.\n"
        + const("INVADERS", invaders)
        + "\n/// Elites: the mobs they turn, chance, weights, constraints, gear sets, name, stats and affix chance.\n"
        + const("ELITES", elites)
        + "\n/// Gear sets by id, and the tags that group them by tier.\n"
        + const("GEAR_SETS", gear_sets)
        + const("GEAR_TAGS", gear_tags)
        + "\n/// Boss spawn chances per dimension and tier.\n"
        + const("SPAWN_RULES", spawn_rules))
    print(f"{OUT}: {len(rarities)} rarities, {len(overrides)} overrides, {len(affixes)} attribute affixes, {len(entries)} loot entries, {len(gems)} gems, {len(invaders)} invaders, {len(elites)} elites, {len(gear_sets)} gear sets")


if __name__ == "__main__":
    main()
