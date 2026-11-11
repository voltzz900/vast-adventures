#!/usr/bin/env python3
"""Builds content/apotheosis: the content mod beside the native mod (apotheosis_native).

It carries the gem items (one per gem, each with a sprite drawn pixel by pixel below), a creative tab, and the
port's own names in English and Brazilian Portuguese (Apotheosis' lang is all rights reserved, so none of its words
are used). The native mod works without it; names then show in English and gems can't exist.

Usage: build_content.py
"""
import colorsys
import json
import re
import shutil
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "content/apotheosis"
RUNTIME = ROOT / "mixins/apotheosis_native/runtime.rs"
DATA = ROOT / "mixins/apotheosis_native/data.rs"
VERSION = "0.5.0"

# ---- sprites ------------------------------------------------------------------

# Digits pick a tone of the gem's ramp, 0 the outline to 6 the highlight; `.` is clear. Light from the top left.
SHAPES = {
    "round": [
        "................",
        "................",
        ".....000000.....",
        "....06665550....",
        "...0665555440...",
        "..066555544330..",
        "..065554443320..",
        "..055444333220..",
        "..054443332210..",
        "..044433322110..",
        "...0333222110...",
        "....02221110....",
        ".....000000.....",
        "................",
        "................",
        "................",
    ],
    "square": [
        "................",
        "................",
        "....00000000....",
        "...0666666550...",
        "..066555555440..",
        "..065544444330..",
        "..065543334330..",
        "..065543334330..",
        "..055433333220..",
        "..054333333210..",
        "..044222222110..",
        "...0222211110...",
        "....00000000....",
        "................",
        "................",
        "................",
    ],
    "tear": [
        "................",
        ".......00.......",
        "......0650......",
        ".....065540.....",
        "....06555440....",
        "...0655544430...",
        "...0655444330...",
        "..065554443320..",
        "..055544433320..",
        "..054444333210..",
        "..044433322110..",
        "...0333222110...",
        "....02221110....",
        ".....000000.....",
        "................",
        "................",
    ],
    "shard": [
        "................",
        "........00......",
        ".......0650.....",
        "...00..06540....",
        "..0650.065430...",
        "..06540055430...",
        "..05543054430...",
        "...0543054330...",
        "...05443054330..",
        "....0543054320..",
        "....054305432...",
        ".....04300420...",
        ".....0430.010...",
        "......010..0....",
        ".......0........",
        "................",
    ],
}

SHAPES.update({
    "pile": [
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        ".......00.......",
        "......0650......",
        ".....065440.....",
        "....06564430....",
        "...0654643320...",
        "..065443362210..",
        ".05443322221110.",
        "..000000000000..",
        "................",
        "................",
    ],
    "scrap": [
        "................",
        "................",
        "................",
        "......000.......",
        ".....0654000....",
        "....065543650...",
        "...0655443430...",
        "...054432230....",
        "..0654322110....",
        "..0543210.000...",
        "...03210.06540..",
        "....000.0544320.",
        "........0433210.",
        ".........00000..",
        "................",
        "................",
    ],
    "cloth": [
        "................",
        "................",
        "...0000000000...",
        "..066655555540..",
        "..065544444430..",
        "..054433333320..",
        "...0433322220...",
        "...0544433330...",
        "..06554444430...",
        "..05443333320...",
        "..04332222210...",
        "...0322111110...",
        "....00000000....",
        "................",
        "................",
        "................",
    ],
})

SHAPES.update({
    "sigil": [
        "................",
        "................",
        ".....000000.....",
        "....06655540....",
        "...0654444430...",
        "...0543..3420...",
        "...054.66.420...",
        "...054.66.420...",
        "...0543..3320...",
        "...0544444320...",
        "...0433333220...",
        "....03222210....",
        ".....000000.....",
        "................",
        "................",
        "................",
    ],
    "slab": [
        "................",
        "................",
        "................",
        "...0000000000...",
        "..066565656550..",
        "..065444544430..",
        "..054433343320..",
        "..054344443420..",
        "..043333333310..",
        "..043223332210..",
        "..032222222110..",
        "...0000000000...",
        "................",
        "................",
        "................",
        "................",
    ],
})

# Apotheosis' crafting materials: what salvaging gives and reforging and gem cutting cost. Shape, colour, English
# and Portuguese names are this port's own.
MATERIALS = {
    "mysterious_scrap_metal": ("scrap", 0x9AA0A6, "Mysterious Scrap Metal", "Sucata Misteriosa"),
    "timeworn_fabric": ("cloth", 0x8E6E4A, "Timeworn Fabric", "Tecido Gasto"),
    "luminous_crystal_shard": ("shard", 0x7FD6F5, "Luminous Crystal Shard", "Fragmento de Cristal Luminoso"),
    "arcane_sands": ("pile", 0xC77DFF, "Arcane Sands", "Areia Arcana"),
    "godforged_pearl": ("round", 0xFFE7A8, "Godforged Pearl", "Pérola Forjada pelos Deuses"),
    "gem_dust": ("pile", 0xD06AA8, "Gem Dust", "Pó de Gema"),
    "gem_fused_slate": ("slab", 0x5B5266, "Gem-Fused Slate", "Ardósia Fundida com Gema"),
    "sigil_of_socketing": ("sigil", 0x3FA7D6, "Sigil of Socketing", "Sigilo de Encaixe"),
    "sigil_of_withdrawal": ("sigil", 0xD6643F, "Sigil of Withdrawal", "Sigilo de Retirada"),
    "sigil_of_unnaming": ("sigil", 0x9A9A9A, "Sigil of Unnaming", "Sigilo de Desnomear"),
}
# Crafting recipes for the slate and the sigils (Apotheosis' own shapes).
RECIPES = {
    "gem_fused_slate": (["DDD", "DGD", "DDD"], {"D": "minecraft:deepslate", "G": "apotheosis:gem_dust"}, 8),
    "sigil_of_socketing": (["GUG", "EEE", "GAG"], {"A": "minecraft:amethyst_shard", "E": "apotheosis:gem_fused_slate",
                                                   "G": "apotheosis:gem_dust", "U": "minecraft:gunpowder"}, 3),
    "sigil_of_withdrawal": (["GBG", "ELE", "GMG"], {"B": "minecraft:blaze_rod", "E": "minecraft:ender_pearl",
                                                    "G": "apotheosis:gem_fused_slate", "L": "minecraft:lava_bucket",
                                                    "M": "apotheosis:gem_dust"}, 4),
    "sigil_of_unnaming": (["GGG", "FFF", "GGG"], {"F": "minecraft:flint", "G": "apotheosis:gem_fused_slate"}, 6),
}

# Each gem's shape and base colour, picked by hand; English and Portuguese names are this port's own.
GEMS = {
    "ballast": ("round", 0x8A8F99, "Ballast", "do Lastro"),
    "brawlers": ("square", 0xC0392B, "the Brawler", "do Brigão"),
    "breach": ("shard", 0x7D3C98, "Breach", "da Ruptura"),
    "combatant": ("tear", 0x2E86C1, "the Combatant", "do Combatente"),
    "guardian": ("square", 0x17A589, "the Guardian", "do Guardião"),
    "lightning": ("shard", 0xF4D03F, "Lightning", "do Relâmpago"),
    "lunar": ("round", 0xAEB6BF, "the Moon", "Lunar"),
    "samurai": ("tear", 0xE74C3C, "the Samurai", "do Samurai"),
    "slipstream": ("tear", 0x48C9B0, "the Slipstream", "da Correnteza"),
    "solar": ("round", 0xF39C12, "the Sun", "Solar"),
    "splendor": ("square", 0xF7DC6F, "Splendor", "do Esplendor"),
    "tyrannical": ("shard", 0x6C3483, "the Tyrant", "do Tirano"),
    "warlord": ("square", 0x922B21, "the Warlord", "do Senhor da Guerra"),
    "earth": ("round", 0x229954, "the Earth", "da Terra"),
    "royalty": ("square", 0xD4AC0D, "Royalty", "da Realeza"),
    "verdant_ruin": ("shard", 0x52BE80, "the Verdant Ruin", "da Ruína Verdejante"),
    "endersurge": ("tear", 0x1ABC9C, "the Ender Surge", "do Surto Ender"),
    "mageslayer": ("shard", 0x5B2C6F, "the Mageslayer", "do Matamagos"),
    "blood_lord": ("tear", 0x78281F, "the Blood Lord", "do Senhor do Sangue"),
    "inferno": ("shard", 0xE67E22, "the Inferno", "do Inferno"),
    "molten_breach": ("round", 0xDC7633, "the Molten Breach", "da Ruptura Derretida"),
}

STEPS = [(-0.42, -14, 1.1), (-0.30, -10, 1.15), (-0.17, -5, 1.1), (0.0, 0, 1.0), (0.12, 4, 0.95), (0.24, 8, 0.85), (0.36, 12, 0.5)]


def ramp(colour):
    """Seven tones from the colour, shadows leaning blue and highlights warm."""
    r, g, b = ((colour >> s & 255) / 255 for s in (16, 8, 0))
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    room = min(1.0, (0.97 - l) / 0.36) if l > 0.6 else 1.0
    out = []
    for dl, dh, fs in STEPS:
        dl = dl * room if dl > 0 else dl
        goal = 240 if dh < 0 else 60
        delta = ((goal - h * 360 + 180) % 360) - 180
        nh = (h * 360 + max(-abs(dh), min(abs(dh), delta)) * (1 if s > 0.08 else 0.3)) % 360 / 360
        nr, ng, nb = colorsys.hls_to_rgb(nh, max(0.04, min(0.97, l + dl)), max(0.0, min(1.0, s * fs)))
        out.append((round(nr * 255), round(ng * 255), round(nb * 255), 255))
    return out


def png(grid, tones):
    rows = [[(0, 0, 0, 0) if ch == "." else tones[int(ch)] for ch in row] for row in grid]
    raw = b"".join(b"\x00" + bytes(c for px in row for c in px) for row in rows)
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 16, 16, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


for name, grid in SHAPES.items():
    assert len(grid) == 16 and all(len(r) == 16 for r in grid), (name, [len(r) for r in grid])

# ---- names --------------------------------------------------------------------

PURITIES = [("cracked", "Cracked", "Rachada"), ("chipped", "Chipped", "Lascada"), ("flawed", "Flawed", "Imperfeita"),
            ("normal", "", ""), ("flawless", "Flawless", "Impecável"), ("perfect", "Perfect", "Perfeita")]
CLASSES = {
    "light_weapon": ("Melee Weapons", "Armas Corpo a Corpo"), "melee_weapon": ("Melee Weapons", "Armas Corpo a Corpo"),
    "weapons": ("Weapons", "Armas"), "ranged_weapon": ("Ranged Weapons", "Armas de Longo Alcance"),
    "bow": ("Bows", "Arcos"), "trident": ("Tridents", "Tridentes"), "shield": ("Shields", "Escudos"),
    "tools": ("Tools", "Ferramentas"), "breaker": ("Pickaxes and Shovels", "Picaretas e Pás"),
    "weapon_or_tool": ("Weapons and Tools", "Armas e Ferramentas"), "armor": ("Armour", "Armaduras"),
    "core_armor": ("Chestplates and Leggings", "Peitorais e Calças"), "lower_armor": ("Leggings and Boots", "Calças e Botas"),
    "helmet": ("Helmets", "Capacetes"), "chestplate": ("Chestplates", "Peitorais"), "leggings": ("Leggings", "Calças"),
    "boots": ("Boots", "Botas"), "anything": ("Anything", "Qualquer Item"),
}
# Portuguese for the affix names in runtime.rs (English comes from there).
AFFIXES_PT = {
    "armor/attribute/adamantine": ("Impenetrável", "da Muralha"), "armor/attribute/blessed": ("Vital", "da Vida"),
    "armor/attribute/elastic": ("Flexível", "das Passadas"), "armor/attribute/fireproof": ("Incombustível", "das Brasas"),
    "armor/attribute/fortunate": ("Estelar", "das Estrelas"), "armor/attribute/gravitational": ("Leve", "da Pluma"),
    "armor/attribute/ironforged": ("Resistente", "do Ferro"), "armor/attribute/oxygenated": ("Submersível", "das Profundezas"),
    "armor/attribute/stalwart": ("Inabalável", "da Rocha"), "armor/attribute/steel_touched": ("Tenaz", "do Aço"),
    "armor/attribute/windswept": ("Veloz", "do Vento"), "breaker/attribute/destructive": ("Voraz", "da Ruína"),
    "breaker/attribute/lengthy": ("Distante", "do Alcance"), "breaker/attribute/submerged": ("Fluvial", "das Marés"),
    "generic/attribute/lucky": ("Feliz", "da Sorte"), "melee/attribute/forceful": ("Potente", "do Impacto"),
    "melee/attribute/graceful": ("Ágil", "da Destreza"), "melee/attribute/lengthy": ("Extensível", "da Lança"),
    "melee/attribute/murderous": ("Cruel", "da Matança"), "melee/attribute/violent": ("Feroz", "da Fúria"),
    "ranged/attribute/windswept": ("Veloz", "do Vento"), "shield/attribute/ironforged": ("Resistente", "do Ferro"),
    "shield/attribute/stalwart": ("Inabalável", "da Rocha"), "shield/attribute/steel_touched": ("Tenaz", "do Aço"),
}
RARITIES = [("common", "Common", "Comum"), ("uncommon", "Uncommon", "Incomum"), ("rare", "Rare", "Raro"),
            ("epic", "Epic", "Épico"), ("mythic", "Mythic", "Mítico")]


def lang():
    names = re.findall(r'\("([a-z_/]+)", "([^"]+)", "([^"]+)"\),', RUNTIME.read_text())
    assert {n[0] for n in names} == set(AFFIXES_PT), set(AFFIXES_PT) ^ {n[0] for n in names}
    en = {"misc.apotheosis.affix_name.two": "%s %s", "misc.apotheosis.affix_name.three": "%s %s %s",
          "item.apotheosis.gem_name": "%s %s", "socket.apotheosis.empty": "Empty Socket",
          "text.apotheosis.unique": "Unique: one per item", "text.apotheosis.when_socketed_in": "When socketed in:",
          "itemGroup.apotheosis.gems": "Apotheosis Gems"}
    br = {"misc.apotheosis.affix_name.two": "%2$s %1$s", "misc.apotheosis.affix_name.three": "%2$s %1$s %3$s",
          "item.apotheosis.gem_name": "%2$s %1$s", "socket.apotheosis.empty": "Encaixe vazio",
          "text.apotheosis.unique": "Única: uma por item", "text.apotheosis.when_socketed_in": "Encaixada em:",
          "itemGroup.apotheosis.gems": "Gemas do Apotheosis"}
    for key, e, p in RARITIES:
        en[f"rarity.apotheosis:{key}"], br[f"rarity.apotheosis:{key}"] = e, p
    for path, name, suffix in names:
        en[f"affix.apotheosis:{path}"], en[f"affix.apotheosis:{path}.suffix"] = name, suffix
        br[f"affix.apotheosis:{path}"], br[f"affix.apotheosis:{path}.suffix"] = AFFIXES_PT[path]
    for key, e, p in PURITIES:
        en[f"purity.apotheosis.{key}"], br[f"purity.apotheosis.{key}"] = e, p
    for key, (e, p) in CLASSES.items():
        en[f"gem_class.apotheosis.{key}"], br[f"gem_class.apotheosis.{key}"] = e, p
    titles = re.findall(r'\("([a-z]+)", "(the [^"]+)", "([^"]+)"\),', (ROOT / "mixins/apotheosis_native/bosses.rs").read_text())
    assert len(titles) >= 10, titles
    en["apotheosis.boss_name"], br["apotheosis.boss_name"] = "%s %s", "%s, %s"
    for key, e, p in titles:
        en[f"apotheosis.boss_title.{key}"], br[f"apotheosis.boss_title.{key}"] = e, p
    en["info.apotheosis.boss_spawn"] = "A %s boss has appeared: %s at %s %s %s"
    br["info.apotheosis.boss_spawn"] = "Um chefe %s apareceu: %s em %s %s %s"
    for key, e, p in (("undead_knight", "Undead Knight", "Cavaleiro Morto-Vivo"), ("honeyed_archer", "Honeyed Archer", "Arqueira do Mel"),
                      ("withering_archer", "Withering Archer", "Arqueiro Fenecido"), ("craig", "Craig", "Craig")):
        en[f"apotheosis.elite.{key}"], br[f"apotheosis.elite.{key}"] = e, p
    for item, (_, _, e, p) in MATERIALS.items():
        en[f"item.apotheosis.{item}"], br[f"item.apotheosis.{item}"] = e, p
    for gem, (_, _, e, p) in GEMS.items():
        en[f"item.apotheosis.gem_{gem}"] = f"Gem of {e}"
        br[f"item.apotheosis.gem_{gem}"] = f"Gema {p}"
    return en, br


def main():
    data_gems = re.search(r'GEMS: &str = r##"(.*?)"##', DATA.read_text()).group(1)
    in_data = {g["id"].rsplit("/", 1)[1] for g in json.loads(data_gems)}
    assert in_data == set(GEMS), in_data ^ set(GEMS)
    if OUT.exists():
        shutil.rmtree(OUT)
    files = {}
    for gem, (shape, colour, _, _) in GEMS.items():
        item = f"gem_{gem}"
        files[f"data/apotheosis/horizon/item/{item}.json"] = {"template": "minecraft:diamond"}
        files[f"assets/apotheosis/models/item/{item}.json"] = {"parent": "minecraft:item/generated",
                                                               "textures": {"layer0": f"apotheosis:item/{item}"}}
        files[f"assets/apotheosis/items/{item}.json"] = {"model": {"type": "minecraft:model", "model": f"apotheosis:item/{item}"}}
        files[f"assets/apotheosis/textures/item/{item}.png"] = png(SHAPES[shape], ramp(colour))
    for item, (shape, colour, _, _) in MATERIALS.items():
        files[f"data/apotheosis/horizon/item/{item}.json"] = {"template": "minecraft:amethyst_shard"}
        files[f"assets/apotheosis/models/item/{item}.json"] = {"parent": "minecraft:item/generated",
                                                               "textures": {"layer0": f"apotheosis:item/{item}"}}
        files[f"assets/apotheosis/items/{item}.json"] = {"model": {"type": "minecraft:model", "model": f"apotheosis:item/{item}"}}
        files[f"assets/apotheosis/textures/item/{item}.png"] = png(SHAPES[shape], ramp(colour))
    for item, (pattern, key, count) in RECIPES.items():
        files[f"data/apotheosis/recipe/{item}.json"] = {"type": "minecraft:crafting_shaped", "category": "misc",
                                                        "key": key, "pattern": pattern,
                                                        "result": {"count": count, "id": f"apotheosis:{item}"}}
    files["data/apotheosis/horizon/creative_tab/gems.json"] = {
        "title": "Apotheosis", "icon": "apotheosis:gem_ballast",
        "items": [f"apotheosis:gem_{g}" for g in GEMS] + [f"apotheosis:{m}" for m in MATERIALS]}
    en, br = lang()
    files["assets/apotheosis/lang/en_us.json"] = dict(sorted(en.items()))
    files["assets/apotheosis/lang/pt_br.json"] = dict(sorted(br.items()))
    for path, value in files.items():
        target = OUT / path
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(value, bytes):
            target.write_bytes(value)
        else:
            target.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
    (OUT / "horizon-mod.toml").write_text(
        'id = "apotheosis"\n'
        'name = "Apotheosis (content)"\n'
        f'version = "{VERSION}"\n'
        'description = "The Apotheosis port\'s gem items and its names in English and Brazilian Portuguese. Install it with the native mod."\n'
        'authors = ["voltzz900"]\n'
        'api = "0.2"\n'
        'side = "both"\n'
        'capabilities = ["registries"]\n'
        )
    print(f"{OUT}: {len(GEMS)} gems, {len(en)} lang keys")


if __name__ == "__main__":
    main()
