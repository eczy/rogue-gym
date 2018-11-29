use super::{InitItem, Item, ItemAttr, ItemHandler, ItemToken};
use crate::character::{Dice, HitPoint, Level};
use crate::rng::{Parcent, RngHandle};
use crate::SmallStr;
use std::fmt;
use std::ops::Range;
use tuple_map::TupleMap3;

/// Weapon configuration
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Config {
    #[serde(default)]
    #[serde(flatten)]
    pub weapons: Weapons,
    #[serde(default = "default_cursed_rate")]
    #[serde(skip_serializing_if = "is_default_cursed_rate")]
    pub cursed_rate: Parcent,
    #[serde(default = "default_powerup_rate")]
    #[serde(skip_serializing_if = "is_default_powerup_rate")]
    pub powerup_rate: Parcent,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            weapons: Default::default(),
            cursed_rate: default_cursed_rate(),
            powerup_rate: default_powerup_rate(),
        }
    }
}

impl Config {
    pub(super) fn build(self) -> WeaponHandler {
        let Config {
            weapons,
            cursed_rate,
            powerup_rate,
        } = self;
        WeaponHandler {
            cursed_rate,
            powerup_rate,
            weapons: weapons.build(),
        }
    }
}

const fn default_cursed_rate() -> Parcent {
    Parcent::new(10)
}

const fn default_powerup_rate() -> Parcent {
    Parcent::new(5)
}

fn is_default_cursed_rate(u: &Parcent) -> bool {
    cfg!(not(test)) && *u == default_cursed_rate()
}

fn is_default_powerup_rate(u: &Parcent) -> bool {
    cfg!(not(test)) && *u == default_powerup_rate()
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Weapons {
    Builtin {
        typ: BuiltinKind,
        include: Vec<usize>,
    },
    Custom(Vec<WeaponStatus>),
}

impl Default for Weapons {
    fn default() -> Self {
        Weapons::Builtin {
            typ: BuiltinKind::Rogue,
            include: (0..ROGUE_WEAPONS.len()).collect(),
        }
    }
}

impl Weapons {
    fn build(self) -> Vec<WeaponStatus> {
        match self {
            Weapons::Builtin { typ, include } => match typ {
                BuiltinKind::Rogue => include
                    .into_iter()
                    .filter_map(|i| {
                        if i >= ROGUE_WEAPONS.len() {
                            return None;
                        }
                        Some(ROGUE_WEAPONS[i].to_weapon())
                    })
                    .collect(),
            },
            Weapons::Custom(v) => v,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum BuiltinKind {
    Rogue,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Weapon {
    at_weild: Dice<HitPoint>,
    at_throw: Dice<HitPoint>,
    name: SmallStr,
    hit_plus: Level,
    dam_plus: HitPoint,
}

fn display_plus_types(i: i64, f: &mut fmt::Formatter) -> fmt::Result {
    if i < 0 {
        write!(f, "-{}", -i)
    } else {
        write!(f, "+{}", i)
    }
}

impl fmt::Display for Weapon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_plus_types(self.hit_plus.0, f)?;
        write!(f, ",")?;
        display_plus_types(self.dam_plus.0, f)?;
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct WeaponStatus {
    at_weild: Dice<HitPoint>,
    at_throw: Dice<HitPoint>,
    name: SmallStr,
    init_num: Range<u32>,
    attr: ItemAttr,
}

impl WeaponStatus {
    pub(super) fn into_item(
        self,
        rng: &mut RngHandle,
        initialize: impl FnOnce(&mut Weapon, &mut ItemAttr, &mut RngHandle),
    ) -> Item {
        let WeaponStatus {
            at_weild,
            at_throw,
            name,
            mut attr,
            init_num,
        } = self;
        let num = rng.range(init_num);
        let mut weapon = Weapon {
            at_weild,
            at_throw,
            name,
            hit_plus: 0.into(),
            dam_plus: 0.into(),
        };
        initialize(&mut weapon, &mut attr, rng);
        Item::weapon(weapon, attr, num)
    }
}

pub struct WeaponHandler {
    weapons: Vec<WeaponStatus>,
    cursed_rate: Parcent,
    powerup_rate: Parcent,
}

impl WeaponHandler {
    pub fn gen_weapon(&self, item_handle: &mut ItemHandler) -> ItemToken {
        let idx = item_handle.rng.range(0..self.weapons.len());
        let status = self.weapons[idx].clone();
        let item = status.into_item(&mut item_handle.rng, |weapon, attr, rng| {
            if rng.parcent(self.cursed_rate) {
                attr.or(ItemAttr::IS_CURSED);
                weapon.hit_plus -= Level(rng.range(1..=4));
            } else if rng.parcent(self.powerup_rate) {
                weapon.hit_plus += Level(rng.range(1..=4));
            }
        });
        item_handle.gen_item(item)
    }
}

pub(crate) fn rogue_init_weapons(vec: &mut Vec<InitItem>) {
    (0, 2, 3).for_each(|i| vec.push(InitItem::Weapon(ROGUE_WEAPONS[i].to_weapon())));
}

struct StaticWeapon {
    at_weild: Dice<HitPoint>,
    at_throw: Dice<HitPoint>,
    name: &'static str,
    attr: ItemAttr,
    min: u32,
    max: u32,
}

impl StaticWeapon {
    fn to_weapon(&self) -> WeaponStatus {
        let &StaticWeapon {
            at_weild,
            at_throw,
            name,
            attr,
            min,
            max,
        } = self;
        WeaponStatus {
            at_weild,
            at_throw,
            name: SmallStr::from_str(name),
            init_num: min..max + 1,
            attr,
        }
    }
}

const MANY_AND_THROW: ItemAttr = ItemAttr::IS_MANY.merge(ItemAttr::CAN_THROW);

macro_rules! hp_dice {
    ($n: expr, $m: expr) => {
        Dice::new($n, HitPoint($m))
    };
}

const ROGUE_WEAPONS: [StaticWeapon; 9] = [
    StaticWeapon {
        at_weild: hp_dice!(2, 4),
        at_throw: hp_dice!(1, 3),
        name: "mace",
        attr: ItemAttr::empty(),
        min: 1,
        max: 1,
    },
    StaticWeapon {
        at_weild: hp_dice!(3, 4),
        at_throw: hp_dice!(1, 2),
        name: "long-sword",
        attr: ItemAttr::empty(),
        min: 1,
        max: 1,
    },
    StaticWeapon {
        at_weild: hp_dice!(1, 1),
        at_throw: hp_dice!(1, 1),
        name: "short-bow",
        attr: ItemAttr::empty(),
        min: 1,
        max: 1,
    },
    StaticWeapon {
        at_weild: hp_dice!(1, 1),
        at_throw: hp_dice!(2, 3),
        name: "arrow",
        attr: MANY_AND_THROW,
        min: 8,
        max: 16,
    },
    StaticWeapon {
        at_weild: hp_dice!(1, 6),
        at_throw: hp_dice!(1, 4),
        name: "dagger",
        attr: ItemAttr::CAN_THROW,
        min: 2,
        max: 6,
    },
    StaticWeapon {
        at_weild: hp_dice!(4, 4),
        at_throw: hp_dice!(1, 2),
        name: "two-handed-sword",
        attr: ItemAttr::empty(),
        min: 1,
        max: 1,
    },
    StaticWeapon {
        at_weild: hp_dice!(1, 1),
        at_throw: hp_dice!(1, 3),
        name: "dart",
        attr: MANY_AND_THROW,
        min: 8,
        max: 16,
    },
    StaticWeapon {
        at_weild: hp_dice!(1, 2),
        at_throw: hp_dice!(2, 4),
        name: "shuriken",
        attr: MANY_AND_THROW,
        min: 8,
        max: 16,
    },
    StaticWeapon {
        at_weild: hp_dice!(2, 3),
        at_throw: hp_dice!(1, 6),
        name: "spear",
        attr: ItemAttr::IS_MANY,
        min: 8,
        max: 16,
    },
];
