// Apotheosis for Horizon: the effects of affixes and gems beyond attributes (slice 0.6).
//
// Injected into horizon-world as `pub mod apotheosis_effects`. Given an item (its id and `custom_data`), it lists
// what its affixes and socketed gems do: mob effects on a hit, an arrow, a hurt or a broken block, damage
// reduction, enchantment levels, executing, thunderstruck, cleaving, telepathic and festive drops, and radial
// mining. The game's side applies them at the matching hooks.

use crate::apotheosis::{self as core, step_value};
use crate::apotheosis_gems as gems;
use serde_json::Value;

/// When a mob effect applies, and to whom (`MobEffectAffix.Target`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// Hitting a mob: the mob, or the wielder.
    AttackTarget,
    AttackSelf,
    /// Being hurt: the wearer, or what hurt them.
    HurtSelf,
    HurtAttacker,
    /// An arrow hitting a mob: the mob, or the shooter.
    ArrowTarget,
    ArrowSelf,
    /// Breaking a block: the miner.
    BreakSelf,
}

impl Target {
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "attack_target" => Self::AttackTarget,
            "attack_self" => Self::AttackSelf,
            "hurt_self" => Self::HurtSelf,
            "hurt_attacker" => Self::HurtAttacker,
            "arrow_target" => Self::ArrowTarget,
            "arrow_self" => Self::ArrowSelf,
            "break_self" => Self::BreakSelf,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// A mob effect (`minecraft:` ones only) for `duration` ticks, at most once every `cooldown` ticks.
    MobEffect { source: String, target: Target, effect: String, duration: i32, amplifier: i32, cooldown: i32 },
    /// A share of incoming damage of a kind taken off.
    DamageReduction { kind: String, amount: f64 },
    /// Levels added to an enchantment: to it alone (`single`), only when the item has it (`existing`), or to
    /// every enchantment it has (`global`).
    Enchantment { enchantment: String, mode: String, levels: u32 },
    /// A mob left below this share of its health dies.
    Executing { threshold: f64 },
    /// Lightning damage to the mobs around the one hit.
    Thunderstruck { damage: f64 },
    /// A chance that the hit also lands on up to `targets` mobs around.
    Cleaving { chance: f64, targets: u32 },
    /// Drops go straight to the player.
    Telepathic,
    /// A chance that a kill drops its loot many more times.
    Festive { chance: f64, rolls: u32 },
    /// Mining an area: `width` by `height`, offset.
    Radial { width: i32, x_off: i32, height: i32, y_off: i32 },
}

/// The Apothic effects this port can't apply yet (they come with Apothic Attributes).
fn vanilla(effect: &str) -> bool {
    effect.starts_with("minecraft:")
}

/// The effect an affix or a gem bonus gives, from its own fields and its value (`v`, already for the item's
/// rarity or purity) at `level` (an affix's roll; 0.5 for a gem).
fn effect_of(typ: &str, raw: &Value, v: &Value, level: f32, source: &str) -> Option<Effect> {
    let num = |x: &Value| step_value(x, level);
    Some(match typ {
        "apotheosis:mob_effect" => {
            let effect = raw["mob_effect"].as_str()?;
            if !vanilla(effect) {
                return None;
            }
            Effect::MobEffect {
                source: source.to_owned(),
                target: Target::named(raw["target"].as_str()?)?,
                effect: effect.to_owned(),
                duration: num(&v["duration"])? as i32,
                amplifier: num(&v["amplifier"]).unwrap_or(0.0) as i32,
                cooldown: v["cooldown"].as_f64().unwrap_or(0.0) as i32,
            }
        }
        "apotheosis:damage_reduction" => Effect::DamageReduction { kind: raw["damage_type"].as_str()?.to_owned(), amount: num(v)? },
        "apotheosis:enchantment" => Effect::Enchantment {
            enchantment: raw["enchantment"].as_str()?.to_owned(),
            mode: raw["mode"].as_str().unwrap_or("single").to_owned(),
            levels: num(v)? as u32,
        },
        "apotheosis:executing" => Effect::Executing { threshold: num(v)? },
        "apotheosis:thunderstruck" => Effect::Thunderstruck { damage: num(v)? },
        "apotheosis:cleaving" => Effect::Cleaving { chance: num(&v["chance"])?, targets: num(&v["targets"])? as u32 },
        "apotheosis:telepathic" => Effect::Telepathic,
        "apotheosis:festive" => Effect::Festive { chance: num(&v["chance"])?, rolls: v["rolls"].as_u64().unwrap_or(1) as u32 },
        "apotheosis:radial" => {
            // A list of shapes for an affix (the roll picks one), one shape for a gem.
            let shape = match v {
                Value::Array(list) if !list.is_empty() => &list[((level * list.len() as f32) as usize).min(list.len() - 1)],
                other => other,
            };
            Effect::Radial {
                width: shape["x"].as_i64()? as i32,
                x_off: shape["xOff"].as_i64().unwrap_or(0) as i32,
                height: shape["y"].as_i64()? as i32,
                y_off: shape["yOff"].as_i64().unwrap_or(0) as i32,
            }
        }
        _ => return None,
    })
}

/// Text components for an effect (for gem tooltips and the item tooltip's lines), with English fallbacks.
pub fn describe(effect: &Effect) -> Vec<Value> {
    use serde_json::json;
    let pct = |v: f64| format!("{}%", (v * 1000.0).round() / 10.0);
    match effect {
        Effect::MobEffect { target, effect, duration, amplifier, .. } => {
            let when = match target {
                Target::AttackTarget => ("on_hit_target", "on hit, to the target"),
                Target::AttackSelf => ("on_hit_self", "on hit, to you"),
                Target::HurtSelf => ("hurt_self", "when hurt, to you"),
                Target::HurtAttacker => ("hurt_attacker", "when hurt, to the attacker"),
                Target::ArrowTarget => ("arrow_target", "on an arrow hit, to the target"),
                Target::ArrowSelf => ("arrow_self", "on an arrow hit, to you"),
                Target::BreakSelf => ("break_self", "on breaking a block, to you"),
            };
            let path = effect.strip_prefix("minecraft:").unwrap_or(effect);
            let level = if *amplifier > 0 { format!(" {}", amplifier + 1) } else { String::new() };
            vec![
                json!({"translate": format!("effect.minecraft.{path}")}),
                json!({"text": format!("{level} ({}s) ", duration / 20)}),
                json!({"translate": format!("apotheosis.when.{}", when.0), "fallback": when.1}),
            ]
        }
        Effect::DamageReduction { kind, amount } => vec![json!({"translate": format!("apotheosis.reduction.{kind}"),
            "fallback": format!("-%s {kind} damage taken"), "with": [pct(*amount)]})],
        Effect::Enchantment { enchantment, levels, .. } => {
            let path = enchantment.strip_prefix("minecraft:").unwrap_or(enchantment);
            vec![json!({"text": format!("+{levels} ")}), json!({"translate": format!("enchantment.minecraft.{path}")})]
        }
        Effect::Executing { threshold } => vec![json!({"translate": "apotheosis.executing", "fallback": "Executes below %s health", "with": [pct(*threshold)]})],
        Effect::Thunderstruck { damage } => vec![json!({"translate": "apotheosis.thunderstruck", "fallback": "Lightning deals %s to nearby enemies", "with": [format!("{damage}")]})],
        Effect::Cleaving { chance, targets } => vec![json!({"translate": "apotheosis.cleaving", "fallback": "%s chance to hit %s more enemies", "with": [pct(*chance), format!("{targets}")]})],
        Effect::Telepathic => vec![json!({"translate": "apotheosis.telepathic", "fallback": "Drops go to your inventory"})],
        Effect::Festive { chance, .. } => vec![json!({"translate": "apotheosis.festive", "fallback": "%s chance of a shower of loot", "with": [pct(*chance)]})],
        Effect::Radial { width, height, .. } => vec![json!({"translate": "apotheosis.radial", "fallback": "Mines %sx%s", "with": [format!("{width}"), format!("{height}")]})],
    }
}

/// The text for a gem bonus of a ported type at a purity's value.
pub fn describe_raw(typ: &str, raw: &Value, value: &Value) -> Vec<Value> {
    effect_of(typ, raw, value, 0.5, "").map(|e| describe(&e)).unwrap_or_default()
}

/// Whether this port applies an affix or gem bonus of this type with these fields.
pub fn ported(typ: &str, raw: &Value) -> bool {
    match typ {
        "apotheosis:mob_effect" => {
            raw["mob_effect"].as_str().is_some_and(vanilla) && raw["target"].as_str().and_then(Target::named).is_some()
        }
        "apotheosis:damage_reduction" | "apotheosis:enchantment" | "apotheosis:executing" | "apotheosis:thunderstruck"
        | "apotheosis:cleaving" | "apotheosis:telepathic" | "apotheosis:festive" | "apotheosis:radial" => true,
        _ => false,
    }
}

/// Everything an item's affixes and gems do beyond attributes.
pub fn effects(item: &str, custom: &Value) -> Vec<Effect> {
    let mut out = Vec::new();
    let Some(category) = core::category(item) else { return out };
    let a = &custom["apotheosis"];
    if let Some(rarity) = a["rarity"].as_str() {
        for (id, level) in a["affixes"].as_object().into_iter().flatten() {
            let Some(affix) = core::affix_named(id) else { continue };
            let level = level.as_f64().unwrap_or(0.0) as f32;
            let value = affix.raw["values"].get(rarity).cloned().unwrap_or(Value::Null);
            out.extend(effect_of(&affix.typ, &affix.raw, &value, level, id));
        }
    }
    for g in a["gems"].as_array().into_iter().flatten() {
        let Some(gem) = g["gem"].as_str().and_then(gems::gem_named) else { continue };
        let purity = g["purity"].as_str().and_then(gems::purity_named).unwrap_or(0);
        for (typ, raw) in gem.raw_bonuses_for(category) {
            let value = raw["values"].get(gems::PURITIES[purity]).cloned().unwrap_or(Value::Null);
            if !value.is_null() {
                out.extend(effect_of(typ, raw, &value, 0.5, &gem.id));
            }
        }
    }
    out
}

/// What kind of damage a damage type is, as Apotheosis' damage reduction groups them.
pub fn damage_kinds(damage_type: &str) -> &'static [&'static str] {
    match damage_type.strip_prefix("minecraft:").unwrap_or(damage_type) {
        "fall" | "stalagmite" | "fly_into_wall" => &["fall", "physical"],
        "explosion" | "player_explosion" | "fireworks" | "bad_respawn_point" => &["explosion"],
        "arrow" | "trident" | "thrown" | "mob_projectile" | "spit" | "wind_charge" | "fireball" | "unattributed_fireball"
        | "wither_skull" => &["projectile", "physical"],
        "lightning_bolt" => &["lightning"],
        "magic" | "indirect_magic" | "thorns" | "sonic_boom" | "wither" | "dragon_breath" => &["magic"],
        "in_fire" | "on_fire" | "lava" | "hot_floor" | "campfire" => &["fire"],
        "mob_attack" | "mob_attack_no_aggro" | "player_attack" | "sting" | "mace_smash" | "spear" | "falling_block"
        | "falling_anvil" | "falling_stalactite" | "cactus" | "sweet_berry_bush" => &["physical"],
        _ => &[],
    }
}

/// The share of a hit of `damage_type` that `effects` take off, at most 95%.
pub fn reduction(effects: &[Effect], damage_type: &str) -> f64 {
    let kinds = damage_kinds(damage_type);
    let total: f64 = effects
        .iter()
        .filter_map(|e| match e {
            Effect::DamageReduction { kind, amount } if kinds.contains(&kind.as_str()) => Some(*amount),
            _ => None,
        })
        .sum();
    total.min(0.95)
}

/// The levels `effects` add to enchantment `id` on an item that has it at `base`.
pub fn enchantment_bonus(effects: &[Effect], id: &str, base: u32) -> u32 {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::Enchantment { enchantment, mode, levels } => match mode.as_str() {
                "global" if base > 0 => Some(*levels),
                "existing" if base > 0 && enchantment == id => Some(*levels),
                "single" if enchantment == id => Some(*levels),
                _ => None,
            },
            _ => None,
        })
        .sum()
}

/// The blocks a radial miner breaks around `pos`, besides it: `width` across and `height` up the face the
/// player looks at (a wall when they look ahead, the floor or ceiling when they look up or down).
pub fn radial_area(pos: [i32; 3], yaw: f32, pitch: f32, width: i32, x_off: i32, height: i32, y_off: i32) -> Vec<[i32; 3]> {
    let mut out = Vec::new();
    // `RadialState`: from -(size / 2) + offset, `size` blocks.
    for i in 0..width {
        for j in 0..height {
            let (a, b) = (-(width / 2) + x_off + i, -(height / 2) + y_off + j);
            if a == 0 && b == 0 {
                continue;
            }
            let p = if pitch.abs() > 50.0 {
                // looking down or up: the floor or ceiling plane
                [pos[0] + a, pos[1], pos[2] + b]
            } else {
                // looking ahead: the wall across the view
                let facing_x = (yaw.to_radians().sin()).abs() > (yaw.to_radians().cos()).abs();
                if facing_x { [pos[0], pos[1] + b, pos[2] + a] } else { [pos[0] + a, pos[1] + b, pos[2]] }
            };
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn affixes_give_effects() {
        let custom = json!({"apotheosis": {"rarity": "apotheosis:rare", "affixes": {
            "apotheosis:melee/mob_effect/weakening": 0.5,
            "apotheosis:armor/dmg_reduction/feathery": 0.5,
        }}});
        let sword = effects("minecraft:diamond_sword", &custom);
        assert!(sword.iter().any(|e| matches!(e, Effect::MobEffect { target: Target::AttackTarget, effect, .. } if effect == "minecraft:weakness")), "{sword:?}");
        let boots = effects("minecraft:diamond_boots", &custom);
        assert!(reduction(&boots, "minecraft:fall") > 0.1);
        assert_eq!(reduction(&boots, "minecraft:magic"), 0.0);
    }

    #[test]
    fn gems_give_effects() {
        let custom = json!({"apotheosis": {"sockets": 1, "gems": [{"gem": "apotheosis:core/combatant", "purity": "normal"}]}});
        let chest = effects("minecraft:diamond_chestplate", &custom);
        assert!((reduction(&chest, "minecraft:mob_attack") - 0.175).abs() < 1e-9, "{chest:?}");
        let earth = json!({"apotheosis": {"sockets": 1, "gems": [{"gem": "apotheosis:overworld/earth", "purity": "perfect"}]}});
        let sword = effects("minecraft:diamond_sword", &earth);
        assert_eq!(enchantment_bonus(&sword, "minecraft:sharpness", 2), 4);
        assert_eq!(enchantment_bonus(&sword, "minecraft:sharpness", 0), 0, "existing only");
    }

    #[test]
    fn enchantment_modes() {
        let single = vec![Effect::Enchantment { enchantment: "minecraft:looting".into(), mode: "single".into(), levels: 7 }];
        assert_eq!(enchantment_bonus(&single, "minecraft:looting", 0), 7);
        let global = vec![Effect::Enchantment { enchantment: "minecraft:sharpness".into(), mode: "global".into(), levels: 1 }];
        assert_eq!(enchantment_bonus(&global, "minecraft:unbreaking", 3), 1);
        assert_eq!(enchantment_bonus(&global, "minecraft:mending", 0), 0);
    }

    #[test]
    fn radial_areas() {
        assert_eq!(radial_area([0, 64, 0], 0.0, 0.0, 3, 0, 3, 0).len(), 8);
        let floor = radial_area([0, 64, 0], 0.0, 80.0, 3, 0, 3, 0);
        assert!(floor.iter().all(|p| p[1] == 64));
        assert_eq!(radial_area([0, 64, 0], 0.0, 0.0, 1, 0, 2, 1).len(), 1, "one more block above");
    }

    #[test]
    fn kinds() {
        assert!(damage_kinds("minecraft:arrow").contains(&"projectile"));
        assert!(damage_kinds("minecraft:fall").contains(&"fall"));
        assert!(damage_kinds("minecraft:magic").contains(&"magic"));
    }
}
