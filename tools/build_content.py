#!/usr/bin/env python3
"""Builds content/vastadventures: the content mod beside the native mod (vastadventures_native).

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
OUT = ROOT / "content/vastadventures"
RUNTIME = ROOT / "mixins/vastadventures_native/runtime.rs"
DATA = ROOT / "mixins/vastadventures_native/data.rs"
VERSION = "0.9.0"

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
    "rusted_scrap": ("scrap", 0x9AA0A6, "Rusted Scrap", "Sucata Enferrujada"),
    "faded_cloth": ("cloth", 0x8E6E4A, "Faded Cloth", "Pano Desbotado"),
    "glow_shard": ("shard", 0x7FD6F5, "Glow Shard", "Lasca Luminosa"),
    "starsand": ("pile", 0xC77DFF, "Starsand", "Areia Estelar"),
    "titan_pearl": ("round", 0xFFE7A8, "Titan Pearl", "Pérola Titânica"),
    "gem_dust": ("pile", 0xD06AA8, "Gem Dust", "Pó de Gema"),
    "runed_slate": ("slab", 0x5B5266, "Runed Slate", "Ardósia Rúnica"),
    "seal_of_sockets": ("sigil", 0x3FA7D6, "Seal of Sockets", "Selo dos Encaixes"),
    "seal_of_extraction": ("sigil", 0xD6643F, "Seal of Extraction", "Selo da Extração"),
    "seal_of_erasure": ("sigil", 0x9A9A9A, "Seal of Erasure", "Selo do Apagamento"),
}
# Crafting recipes for the slate and the sigils (Apotheosis' own shapes).
RECIPES = {
    "runed_slate": (["DDD", "DGD", "DDD"], {"D": "minecraft:deepslate", "G": "vastadventures:gem_dust"}, 8),
    "seal_of_sockets": (["GUG", "EEE", "GAG"], {"A": "minecraft:amethyst_shard", "E": "vastadventures:runed_slate",
                                                   "G": "vastadventures:gem_dust", "U": "minecraft:gunpowder"}, 3),
    "seal_of_extraction": (["GBG", "ELE", "GMG"], {"B": "minecraft:blaze_rod", "E": "minecraft:ender_pearl",
                                                    "G": "vastadventures:runed_slate", "L": "minecraft:lava_bucket",
                                                    "M": "vastadventures:gem_dust"}, 4),
    "seal_of_erasure": (["GGG", "FFF", "GGG"], {"F": "minecraft:flint", "G": "vastadventures:runed_slate"}, 6),
}

# Each gem's shape and base colour, picked by hand; English and Portuguese names are this port's own.
GEMS = {
    "ballast": ("round", 0x8A8F99, "Heft", "do Peso"),
    "brawlers": ("square", 0xC0392B, "Knuckles", "dos Punhos"),
    "breach": ("shard", 0x7D3C98, "Cracks", "das Rachaduras"),
    "combatant": ("tear", 0x2E86C1, "the Duelist", "do Duelista"),
    "guardian": ("square", 0x17A589, "the Sentinel", "da Sentinela"),
    "lightning": ("shard", 0xF4D03F, "Storms", "das Tempestades"),
    "lunar": ("round", 0xAEB6BF, "Moonlight", "do Luar"),
    "samurai": ("tear", 0xE74C3C, "the Blade", "da Lâmina"),
    "slipstream": ("tear", 0x48C9B0, "Drift", "da Deriva"),
    "solar": ("round", 0xF39C12, "Daylight", "da Luz do Dia"),
    "splendor": ("square", 0xF7DC6F, "Glory", "da Glória"),
    "tyrannical": ("shard", 0x6C3483, "the Despot", "do Déspota"),
    "warlord": ("square", 0x922B21, "the Marshal", "do Marechal"),
    "earth": ("round", 0x229954, "Bedrock", "da Rocha-Mãe"),
    "royalty": ("square", 0xD4AC0D, "the Crown", "da Coroa"),
    "verdant_ruin": ("shard", 0x52BE80, "Overgrowth", "do Matagal"),
    "endersurge": ("tear", 0x1ABC9C, "the Void Tide", "da Maré do Vazio"),
    "mageslayer": ("shard", 0x5B2C6F, "Wardbreaking", "Quebra-Feitiço"),
    "blood_lord": ("tear", 0x78281F, "the Bloodline", "da Linhagem"),
    "inferno": ("shard", 0xE67E22, "Embers", "das Brasas"),
    "molten_breach": ("round", 0xDC7633, "Magma", "do Magma"),
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
    "armor/dmg_reduction/blast_forged": ("Antiexplosivo", "da Explosão"),
    "armor/dmg_reduction/blockading": ("Bloqueador", "do Bastião"),
    "armor/dmg_reduction/deflective": ("Defletor", "da Deflexão"),
    "armor/dmg_reduction/feathery": ("Plumoso", "da Queda"),
    "armor/dmg_reduction/grounded": ("Aterrado", "da Tempestade"),
    "armor/dmg_reduction/runed": ("Rúnico", "das Runas"),
    "armor/mob_effect/blinding": ("Ofuscante", "da Escuridão"),
    "armor/mob_effect/bolstering": ("Fortificante", "da Resistência"),
    "armor/mob_effect/nimble": ("Ligeiro", "da Fuga"),
    "armor/mob_effect/revitalizing": ("Revigorante", "da Renovação"),
    "breaker/mob_effect/spelunkers": ("Espeleólogo", "das Cavernas"),
    "breaker/mob_effect/swift": ("Apressado", "da Pressa"),
    "breaker/enchantment/prosperous": ("Próspero", "da Fartura"),
    "breaker/effect/radial": ("Varredor", "da Pedreira"),
    "breaker/ability/supermassive": ("Colossal", "da Montanha"),
    "armor/attribute/spiritual": ("Restaurador", "da Recuperação"),
    "breaker/attribute/experienced": ("Erudito", "do Saber"),
    "melee/attribute/giant_slaying": ("Derrubador", "dos Colossos"),
    "melee/attribute/glacial": ("Gélido", "do Inverno"),
    "melee/attribute/infernal": ("Escaldante", "da Pira"),
    "melee/attribute/intricate": ("Preciso", "da Precisão"),
    "melee/attribute/lacerating": ("Dilacerante", "das Feridas"),
    "melee/attribute/piercing": ("Perfurante", "da Agulha"),
    "melee/attribute/vampiric": ("Sanguinário", "da Sanguessuga"),
    "ranged/attribute/agile": ("Destro", "da Aljava"),
    "ranged/attribute/elven": ("Aguçado", "da Caça"),
    "weapon/attribute/shredding": ("Estilhaçante", "dos Estilhaços"),
    "generic/telepathic": ("Telepático", "da Mente"),
    "melee/cleaving": ("Cortante", "do Cutelo"),
    "melee/executing": ("Executor", "do Carrasco"),
    "melee/festive": ("Festivo", "do Banquete"),
    "melee/thunderstruck": ("Fulminante", "do Trovão"),
    "melee/mob_effect/elusive": ("Esquivo", "da Brisa"),
    "melee/mob_effect/weakening": ("Debilitante", "da Fraqueza"),
    "ranged/enchantment/prosperous": ("Próspero", "dos Espólios"),
    "ranged/mob_effect/blighted": ("Pestilento", "da Praga"),
    "ranged/mob_effect/deathbound": ("Mortal", "da Cova"),
    "ranged/mob_effect/ensnaring": ("Enredante", "das Armadilhas"),
    "ranged/mob_effect/fleeting": ("Fugaz", "do Passo do Vento"),
    "ranged/mob_effect/ivy_laced": ("Venenoso", "do Veneno"),
    "ranged/mob_effect/shulkers": ("Levitante", "do Shulker"),
}
RARITIES = [("common", "Common", "Comum"), ("uncommon", "Uncommon", "Incomum"), ("rare", "Rare", "Raro"),
            ("epic", "Epic", "Épico"), ("mythic", "Mythic", "Mítico")]


def lang():
    names = re.findall(r'\("([a-z_/]+)", "([^"]+)", "([^"]+)"\),', RUNTIME.read_text())
    assert {n[0] for n in names} == set(AFFIXES_PT), set(AFFIXES_PT) ^ {n[0] for n in names}
    en = {"misc.vastadventures.affix_name.two": "%s %s", "misc.vastadventures.affix_name.three": "%s %s %s",
          "item.vastadventures.gem_name": "%s %s", "socket.vastadventures.empty": "Empty Socket",
          "text.vastadventures.unique": "Unique: one per item", "text.vastadventures.when_socketed_in": "When socketed in:",
          "itemGroup.vastadventures.gems": "Vast Adventures"}
    br = {"misc.vastadventures.affix_name.two": "%2$s %1$s", "misc.vastadventures.affix_name.three": "%2$s %1$s %3$s",
          "item.vastadventures.gem_name": "%2$s %1$s", "socket.vastadventures.empty": "Encaixe vazio",
          "text.vastadventures.unique": "Única: uma por item", "text.vastadventures.when_socketed_in": "Encaixada em:",
          "itemGroup.vastadventures.gems": "Vast Adventures"}
    for key, e, p in RARITIES:
        en[f"rarity.vastadventures:{key}"], br[f"rarity.vastadventures:{key}"] = e, p
    for path, name, suffix in names:
        en[f"affix.vastadventures:{path}"], en[f"affix.vastadventures:{path}.suffix"] = name, suffix
        br[f"affix.vastadventures:{path}"], br[f"affix.vastadventures:{path}.suffix"] = AFFIXES_PT[path]
    for key, e, p in PURITIES:
        en[f"purity.vastadventures.{key}"], br[f"purity.vastadventures.{key}"] = e, p
    for key, (e, p) in CLASSES.items():
        en[f"gem_class.vastadventures.{key}"], br[f"gem_class.vastadventures.{key}"] = e, p
    titles = re.findall(r'\("([a-z]+)", "(the [^"]+)", "([^"]+)"\),', (ROOT / "mixins/vastadventures_native/bosses.rs").read_text())
    assert len(titles) >= 10, titles
    en["vastadventures.boss_name"], br["vastadventures.boss_name"] = "%s %s", "%s, %s"
    for key, e, p in titles:
        en[f"vastadventures.boss_title.{key}"], br[f"vastadventures.boss_title.{key}"] = e, p
    for key, e, p in (("on_hit_target", "on hit, to the target", "ao acertar, no alvo"), ("on_hit_self", "on hit, to you", "ao acertar, em você"),
                      ("hurt_self", "when hurt, to you", "ao levar dano, em você"), ("hurt_attacker", "when hurt, to the attacker", "ao levar dano, no atacante"),
                      ("arrow_target", "on an arrow hit, to the target", "ao acertar uma flecha, no alvo"),
                      ("arrow_self", "on an arrow hit, to you", "ao acertar uma flecha, em você"),
                      ("break_self", "on breaking a block, to you", "ao quebrar um bloco, em você")):
        en[f"vastadventures.when.{key}"], br[f"vastadventures.when.{key}"] = e, p
    for kind, e, p in (("physical", "physical", "físico"), ("projectile", "projectile", "de projétil"), ("fall", "fall", "de queda"),
                       ("explosion", "explosion", "de explosão"), ("magic", "magic", "mágico"), ("lightning", "lightning", "de raio"),
                       ("fire", "fire", "de fogo")):
        en[f"vastadventures.reduction.{kind}"] = f"-%s {e} damage taken"
        br[f"vastadventures.reduction.{kind}"] = f"-%s de dano {p} recebido"
    for key, e, p in (("executing", "Executes below %s health", "Executa abaixo de %s de vida"),
                      ("thunderstruck", "Lightning deals %s to nearby enemies", "Raios causam %s aos inimigos próximos"),
                      ("cleaving", "%s chance to hit %s more enemies", "%s de chance de acertar mais %s inimigos"),
                      ("telepathic", "Drops go to your inventory", "Os drops vão direto pro inventário"),
                      ("festive", "%s chance of a shower of loot", "%s de chance de uma chuva de loot"),
                      ("radial", "Mines %sx%s", "Minera %sx%s")):
        en[f"vastadventures.{key}"], br[f"vastadventures.{key}"] = e, p
    for key, e, p in (("crit_chance", "Critical Chance", "Chance de Crítico"), ("crit_damage", "Critical Damage", "Dano Crítico"),
                      ("life_steal", "Life Steal", "Roubo de Vida"),
                      ("current_hp_damage", "Damage from Current Health", "Dano pela Vida Atual"),
                      ("fire_damage", "Fire Damage", "Dano de Fogo"), ("cold_damage", "Cold Damage", "Dano de Gelo"),
                      ("armor_pierce", "Armor Pierce", "Perfuração de Armadura"), ("armor_shred", "Armor Shred", "Rasgo de Armadura"),
                      ("dodge_chance", "Dodge Chance", "Chance de Esquiva"), ("healing_received", "Healing Received", "Cura Recebida"),
                      ("experience_gained", "Experience Gained", "Experiência Ganha"), ("arrow_damage", "Arrow Damage", "Dano de Flecha"),
                      ("projectile_damage", "Projectile Damage", "Dano de Projétil"), ("draw_speed", "Draw Speed", "Velocidade de Puxada")):
        en[f"vastadventures.attribute.{key}"], br[f"vastadventures.attribute.{key}"] = e, p
    en["info.vastadventures.boss_spawn"] = "A %s boss has appeared: %s at %s %s %s"
    br["info.vastadventures.boss_spawn"] = "Um chefe %s apareceu: %s em %s %s %s"
    for key, e, p in (("undead_knight", "Grave Knight", "Cavaleiro da Cova"), ("honeyed_archer", "Hive Archer", "Arqueira da Colmeia"),
                      ("withering_archer", "Blight Archer", "Arqueiro da Praga"), ("craig", "Lucky Lou", "Lou Sortudo")):
        en[f"vastadventures.elite.{key}"], br[f"vastadventures.elite.{key}"] = e, p
    for item, (_, _, e, p) in MATERIALS.items():
        en[f"item.vastadventures.{item}"], br[f"item.vastadventures.{item}"] = e, p
    for gem, (_, _, e, p) in GEMS.items():
        en[f"item.vastadventures.gem_{gem}"] = f"Gem of {e}"
        br[f"item.vastadventures.gem_{gem}"] = f"Gema {p}"
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
        files[f"data/vastadventures/horizon/item/{item}.json"] = {"template": "minecraft:diamond"}
        files[f"assets/vastadventures/models/item/{item}.json"] = {"parent": "minecraft:item/generated",
                                                               "textures": {"layer0": f"vastadventures:item/{item}"}}
        files[f"assets/vastadventures/items/{item}.json"] = {"model": {"type": "minecraft:model", "model": f"vastadventures:item/{item}"}}
        files[f"assets/vastadventures/textures/item/{item}.png"] = png(SHAPES[shape], ramp(colour))
    for item, (shape, colour, _, _) in MATERIALS.items():
        files[f"data/vastadventures/horizon/item/{item}.json"] = {"template": "minecraft:amethyst_shard"}
        files[f"assets/vastadventures/models/item/{item}.json"] = {"parent": "minecraft:item/generated",
                                                               "textures": {"layer0": f"vastadventures:item/{item}"}}
        files[f"assets/vastadventures/items/{item}.json"] = {"model": {"type": "minecraft:model", "model": f"vastadventures:item/{item}"}}
        files[f"assets/vastadventures/textures/item/{item}.png"] = png(SHAPES[shape], ramp(colour))
    for item, (pattern, key, count) in RECIPES.items():
        files[f"data/vastadventures/recipe/{item}.json"] = {"type": "minecraft:crafting_shaped", "category": "misc",
                                                        "key": key, "pattern": pattern,
                                                        "result": {"count": count, "id": f"vastadventures:{item}"}}
    files["data/vastadventures/horizon/creative_tab/gems.json"] = {
        "title": "Vast Adventures", "icon": "vastadventures:gem_ballast",
        "items": [f"vastadventures:gem_{g}" for g in GEMS] + [f"vastadventures:{m}" for m in MATERIALS]}
    import build_worldgen
    files.update(build_worldgen.files())
    en, br = lang()
    files["assets/vastadventures/lang/en_us.json"] = dict(sorted(en.items()))
    files["assets/vastadventures/lang/pt_br.json"] = dict(sorted(br.items()))
    for path, value in files.items():
        target = OUT / path
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(value, bytes):
            target.write_bytes(value)
        else:
            target.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
    (OUT / "horizon-mod.toml").write_text(
        'id = "vastadventures"\n'
        'name = "Vast Adventures (content)"\n'
        f'version = "{VERSION}"\n'
        'description = "Vast Adventures\' gems, materials, seals, dungeons and names in English and Brazilian Portuguese. Install it with the native mod."\n'
        'authors = ["voltzz900"]\n'
        'api = "0.2"\n'
        'side = "both"\n'
        'capabilities = ["registries"]\n'
        )
    print(f"{OUT}: {len(GEMS)} gems, {len(en)} lang keys")


if __name__ == "__main__":
    main()
