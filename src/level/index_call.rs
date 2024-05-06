//! This is not a good idea, not at all! I could just have a method. But I'm not here
//! to make sensible engineering choices, I'm here to have fun

use std::mem;

use crate::*;

impl<P: MaybeRef<IVec3>> FnOnce<(P,)> for Level {
    type Output = Block;

    extern "rust-call" fn call_once(self, pos: (P,)) -> Self::Output {
        self.call(pos)
    }
}
impl<P: MaybeRef<IVec3>> FnMut<(P,)> for Level {
    extern "rust-call" fn call_mut(&mut self, pos: (P,)) -> Self::Output {
        self.call(pos)
    }
}

impl<P: MaybeRef<IVec3>> Fn<(P,)> for Level {
    extern "rust-call" fn call(&self, (pos,): (P,)) -> Self::Output {
        self.blocks[pos.get_val()]
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
                nbt: None,
            });
        }
    }
}

impl FnOnce<(IVec3, Block, String)> for Level {
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: (IVec3, Block, String)) -> Self::Output {
        self.call_mut(args)
    }
}

impl FnMut<(IVec3, Block, String)> for Level {
    extern "rust-call" fn call_mut(&mut self, (pos, block, nbt): (IVec3, Block, String)) {
        let previous = mem::replace(self.block_mut(pos), block);
        if previous != block {
            self.setblock_recording.push(SetBlock {
                pos,
                previous,
                block,
                nbt: Some(nbt),
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
                nbt: None,
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
