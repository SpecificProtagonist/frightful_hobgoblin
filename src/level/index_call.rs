//! This is not a good idea, not at all! I could just have a method. But I'm not here
//! to make sensible engineering choices, I'm purely here to have fun mwahaha

use std::ops::Index;

use crate::*;

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

impl Index<IVec3> for Level {
    type Output = Block;

    // TODO: On out of bounds, print warning (only once) and return dummy
    fn index(&self, pos: IVec3) -> &Self::Output {
        if let Some(section) = &self.sections[self.section_index(pos)] {
            &section.blocks[Self::block_in_section_index(pos)]
        } else {
            &Block::Air
        }
    }
}

impl Index<Vec3> for Level {
    type Output = Block;

    fn index(&self, pos: Vec3) -> &Self::Output {
        &self[pos.block()]
    }
}
