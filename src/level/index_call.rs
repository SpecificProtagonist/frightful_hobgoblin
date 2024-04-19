//! This is not a good idea, not at all! I could just have a method. But I'm not here
//! to make sensible engineering choices, I'm here to have fun

use std::mem;

use crate::*;

impl FnOnce<(IVec3,)> for Level {
    type Output = Block;

    extern "rust-call" fn call_once(self, pos: (IVec3,)) -> Self::Output {
        self.call(pos)
    }
}
impl FnMut<(IVec3,)> for Level {
    extern "rust-call" fn call_mut(&mut self, pos: (IVec3,)) -> Self::Output {
        self.call(pos)
    }
}

impl Fn<(IVec3,)> for Level {
    extern "rust-call" fn call(&self, (pos,): (IVec3,)) -> Self::Output {
        (self.blocks)(pos)
    }
}

impl FnOnce<(IVec3, Block)> for Level {
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (IVec3, Block)) -> Self::Output {
        self.call_mut(args)
    }
}

impl FnMut<(IVec3, Block)> for Level {
    extern "rust-call" fn call_mut(&mut self, (pos, block): (IVec3, Block)) {
        let previous = mem::replace(self.block_mut(pos), block);
        if previous != block {
            self.setblock_recording.push(SetBlock {
                pos,
                previous,
                block,
            });
        }
    }
}

impl<F: FnOnce(Block) -> Block> FnOnce<(IVec3, F)> for Level {
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (IVec3, F)) -> Self::Output {
        self.call_mut(args)
    }
}

impl<F: FnOnce(Block) -> Block> FnMut<(IVec3, F)> for Level {
    extern "rust-call" fn call_mut(&mut self, (pos, fun): (IVec3, F)) {
        let previous = self.block_mut(pos);
        let block = fun(*previous);
        let previous = mem::replace(previous, block);
        if previous != block {
            self.setblock_recording.push(SetBlock {
                pos,
                previous,
                block,
            });
        }
    }
}

impl FnOnce<(Vec3, Block)> for Level {
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (Vec3, Block)) -> Self::Output {
        self.call_mut(args)
    }
}

impl FnMut<(Vec3, Block)> for Level {
    extern "rust-call" fn call_mut(&mut self, (pos, block): (Vec3, Block)) {
        self(pos.block(), block)
    }
}

impl FnOnce<(IVec2, i32, Block)> for Level {
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (IVec2, i32, Block)) -> Self::Output {
        self.call_mut(args)
    }
}

impl FnMut<(IVec2, i32, Block)> for Level {
    extern "rust-call" fn call_mut(&mut self, (column, z, block): (IVec2, i32, Block)) {
        self(column.extend(z), block)
    }
}

impl<F: FnOnce(Block) -> Block> FnOnce<(IVec2, i32, F)> for Level {
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (IVec2, i32, F)) -> Self::Output {
        self.call_mut(args)
    }
}

impl<F: FnOnce(Block) -> Block> FnMut<(IVec2, i32, F)> for Level {
    extern "rust-call" fn call_mut(&mut self, (column, z, fun): (IVec2, i32, F)) {
        self(column.extend(z), fun)
    }
}
