//! This is not a good idea, not at all! I could just have a method. But I'm not here
//! to make sensible engineering choices, I'm purely here to have fun mwahaha

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
        let section_index = self.section_index(pos);
        match &self.sections.get(section_index) {
            Some(Some(section)) => section.blocks[Self::block_in_section_index(pos)],
            Some(None) => Air,
            None => {
                eprintln!("Out of bounds access at {pos}");
                Barrier
            }
        }
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
        let chunk_index = self.chunk_index(pos.into());
        self.dirty_chunks[chunk_index] = true;
        let index = self.section_index(pos);
        let section = self.sections[index].get_or_insert_default();
        let previous = &mut section.blocks[Self::block_in_section_index(pos)];
        self.setblock_recording.push(SetBlock {
            pos,
            previous: *previous,
            block,
        });
        *previous = block;
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
        let chunk_index = self.chunk_index(pos.into());
        self.dirty_chunks[chunk_index] = true;
        let index = self.section_index(pos);
        let section = self.sections[index].get_or_insert_default();
        let previous = &mut section.blocks[Self::block_in_section_index(pos)];
        let block = fun(*previous);
        self.setblock_recording.push(SetBlock {
            pos,
            previous: *previous,
            block,
        });
        *previous = block;
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
