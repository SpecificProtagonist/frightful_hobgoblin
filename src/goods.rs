use crate::{sim::ConsList, *};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;

use self::sim::ConsItem;

// Material for construction
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum Good {
    Stone,
    Wood,
    Soil,
    Brick,
}

impl Good {
    pub fn display_as_block(self) -> Block {
        match self {
            Self::Stone => Full(Cobble),
            Self::Wood => Full(Wood(Oak)),
            Self::Soil => PackedMud,
            Self::Brick => Full(Brick),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Stack {
    pub good: Good,
    pub amount: f32,
}

impl Stack {
    pub fn new(kind: Good, amount: f32) -> Self {
        Self { good: kind, amount }
    }
}

impl std::fmt::Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}×{}", self.good, self.amount)
    }
}

impl std::ops::Neg for Stack {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            good: self.good,
            amount: -self.amount,
        }
    }
}

pub const CARRY_CAPACITY: f32 = 64.;

pub fn next_stack(list: &ConsList) -> Option<Stack> {
    let mut stack: Option<Stack> = None;
    for cons in list {
        let ConsItem::Set(set) = cons else {
            continue;
        };
        let Some(next) = goods_for_block(set.block) else {
            continue;
        };
        if let Some(stack) = &mut stack {
            if stack.good == next.good {
                stack.amount += next.amount;
                if stack.amount >= CARRY_CAPACITY {
                    return Some(Stack {
                        good: stack.good,
                        amount: CARRY_CAPACITY,
                    });
                }
            } else {
                break;
            }
        } else {
            stack = Some(next)
        }
    }
    stack
}

pub fn goods_for_block(block: Block) -> Option<Stack> {
    fn get_blockmaterial(mat: BlockMaterial) -> Good {
        match mat {
            Wood(_) => Good::Wood,
            Cobble
            | Stone
            | Granite
            | Diorite
            | Andesite
            | SmoothStone
            | PolishedGranite
            | PolishedDiorite
            | PolishedAndesite
            | MossyCobble
            | StoneBrick
            | MossyStonebrick
            | Blackstone
            | PolishedBlackstone
            | PolishedBlackstoneBrick
            | CobbledDeepslate
            | PolishedDeepslate
            | DeepslateBrick
            | DeepslateTile
            | Prismarine
            | DarkPrismarine
            | Sandstone
            | SmoothSandstone
            | RedSandstone
            | SmoothRedSandstone => Good::Stone,
            MudBrick => Good::Soil,
            Brick => Good::Brick,
        }
    }

    match block {
        // This doesn't follow Minecraft's exact ratios
        Log(..) => Some(Stack::new(Good::Wood, 2.)),
        Full(mat) => Some(Stack::new(get_blockmaterial(mat), 1.)),
        Stair(mat, ..) => Some(Stack::new(get_blockmaterial(mat), 0.5)),
        Slab(mat, ..) => Some(Stack::new(get_blockmaterial(mat), 0.5)),
        Fence(mat) => Some(Stack::new(get_blockmaterial(mat), 0.5)),
        Barrel => Some(Stack::new(Good::Wood, 1.)),
        Trapdoor(..) => Some(Stack::new(Good::Wood, 0.25)),
        Door(..) => Some(Stack::new(Good::Wood, 0.25)),
        MangroveRoots => Some(Stack::new(Good::Wood, 0.1875)),
        MuddyMangroveRoots => Some(Stack::new(Good::Soil, 0.8125)),
        _ if block.dirtsoil() => Some(Stack::new(Good::Soil, 1.)),
        _ => None,
    }
}

#[derive(Component, Debug, Clone, Default, Deref, DerefMut)]
pub struct Goods(HashMap<Good, f32>);

impl Goods {
    pub fn has(&self, stack: Stack) -> bool {
        self.get(&stack.good)
            .map(|&a| a >= stack.amount)
            .unwrap_or(false)
    }

    pub fn add(&mut self, stack: Stack) {
        *self.entry(stack.good).or_default() += stack.amount;
    }

    pub fn remove(&mut self, stack: Stack) {
        if let Some(entry) = self.get_mut(&stack.good) {
            *entry -= stack.amount;
            if *entry <= 0. {
                self.0.remove(&stack.good);
            }
        }
    }

    pub fn remove_up_to(&mut self, mut stack: Stack) -> Stack {
        let available = self.entry(stack.good).or_default();
        stack.amount = stack.amount.min(*available);
        *available -= stack.amount;
        stack
    }

    /// Try consume the goods required for block, return what good is missing
    pub fn try_consume(&mut self, block: Block) -> Option<Good> {
        let needed = goods_for_block(block)?;
        let stored = self.entry(needed.good).or_default();
        if *stored >= needed.amount {
            *stored -= needed.amount;
            None
        } else {
            Some(needed.good)
        }
    }
}
