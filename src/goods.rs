use crate::{sim::PlaceList, *};
use bevy_ecs::prelude::*;

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
            Self::Soil => CoarseDirt,
            Self::Brick => Full(Brick),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Stack {
    pub kind: Good,
    pub amount: f32,
}

impl Stack {
    pub fn new(kind: Good, amount: f32) -> Self {
        Self { kind, amount }
    }
}

impl std::fmt::Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}×{}", self.kind, self.amount)
    }
}

pub const CARRY_CAPACITY: f32 = 64.;

pub fn next_stack(list: &PlaceList) -> Option<Stack> {
    let mut stack: Option<Stack> = None;
    for set in list {
        if let Some(next) = goods_for_block(set.block) {
            if let Some(stack) = &mut stack {
                if stack.kind == next.kind {
                    stack.amount += next.amount;
                    if stack.amount >= CARRY_CAPACITY {
                        return Some(Stack {
                            kind: stack.kind,
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
            | PolishedGranite
            | PolishedDiorite
            | PolishedAndesite
            | MossyCobble
            | StoneBrick
            | MossyStonebrick
            | Blackstone
            | PolishedBlackstone
            | PolishedBlackstoneBrick
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

#[derive(Component, Default, Debug)]
pub struct Pile(pub HashMap<Good, f32>);

impl Pile {
    pub fn has(&self, stack: Stack) -> bool {
        self.0
            .get(&stack.kind)
            .map(|&a| a >= stack.amount)
            .unwrap_or(false)
    }

    pub fn add(&mut self, stack: Stack) {
        *self.0.entry(stack.kind).or_default() += stack.amount;
    }

    pub fn remove(&mut self, stack: Stack) {
        if let Some(entry) = self.0.get_mut(&stack.kind) {
            *entry -= stack.amount;
            if *entry <= 0. {
                self.0.remove(&stack.kind);
            }
        }
    }

    pub fn remove_up_to(&mut self, mut stack: Stack) -> Stack {
        let available = self.0.entry(stack.kind).or_default();
        stack.amount = stack.amount.min(*available);
        *available -= stack.amount;
        stack
    }

    pub fn build(&mut self, block: Block) -> Option<Block> {
        let Some(needed) = goods_for_block(block) else {
            return Some(block);
        };
        let stored = self.0.entry(needed.kind).or_default();
        if *stored >= needed.amount {
            *stored -= needed.amount;
            Some(block)
        } else {
            None
        }
    }
}
