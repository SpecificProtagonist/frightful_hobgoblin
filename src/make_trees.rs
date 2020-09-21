use crate::world::*;
use crate::geometry::*;

pub fn make_tiny(world: &mut World, base_pos: Pos, species: TreeSpecies) {
    let log_block = Block::Log(species, LogType::FullBark, LogOrigin::Natural);
    let leaf_block = Block::Leaves(species);

    let base_pos = base_pos + Vec3(0,1,0);
    world[base_pos] = log_block;

    let mut pos = base_pos + Vec3(0,1,0) + rand_2(0.2);
    world[pos] = log_block;

    pos.1 += 1;
    if rand(0.8) {
        if Column::from(pos) == base_pos.into() {
            pos += rand_2(0.3);
        }
        world[pos] = log_block;

        if rand(0.2) {
            pos.1 += 1;
            world[pos] = log_block;
        }
    }

    world.set_if_not_solid(pos+Vec3(1,0,0), leaf_block);
    world.set_if_not_solid(pos+Vec3(-1,0,0), leaf_block);
    world.set_if_not_solid(pos+Vec3(0,1,0), leaf_block);
    world.set_if_not_solid(pos+Vec3(0,-1,0), leaf_block);
    world.set_if_not_solid(pos+Vec3(0,0,1), leaf_block);
    world.set_if_not_solid(pos+Vec3(0,0,-1), leaf_block);

    for leaf_pos in Cuboid::new(pos-Vec3(2,2,2), pos+Vec3(2,2,2)).iter() {
        let distance_squared = (
            (leaf_pos-pos).0 * (leaf_pos-pos).0
          + (leaf_pos-pos).1 * (leaf_pos-pos).1
          + (leaf_pos-pos).2 * (leaf_pos-pos).2) as f32;
        if rand(1.0 - (distance_squared / 3.0)) {
            world.set_if_not_solid(leaf_pos, leaf_block);
        }
    }
}

pub fn make_straight(world: &mut World, pos: Pos, species: TreeSpecies) {
    let log_block = Block::Log(species, LogType::FullBark, LogOrigin::Natural);
    let leaf_block = Block::Leaves(species);

    let log_height = 5 + rand_1(0.5) + rand_1(0.5);

    for y in 1 ..= log_height {
        world[pos + Vec3(0,y,0)] = log_block;
    }

    for Vec2(x, z) in &[Vec2(1,0), Vec2(-1,0), Vec2(0,1), Vec2(0,-1)] {
        for y in 3 ..= log_height + 1 {
            world[pos + Vec3(*x,y,*z)] = leaf_block;
        }
    }

    world.set_if_not_solid(pos + Vec3(0,log_height + 1,0), leaf_block);
    world.set_if_not_solid(pos + Vec3(0,log_height + 2,0), leaf_block);
    if (log_height == 5) & rand(0.75) | (log_height > 5) {
        world.set_if_not_solid(pos + Vec3(0,log_height + 3,0), leaf_block);
    }

    for Vec2(x, z) in &[Vec2(1,1), Vec2(-1,1), Vec2(1,-1), Vec2(-1,-1)] {
        for y in 4 + rand_1(0.5) ..= log_height - 1 + rand_1(0.5) {
            world.set_if_not_solid(pos + Vec3(*x,y,*z), leaf_block);
        }
    }

}