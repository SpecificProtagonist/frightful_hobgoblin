use std::ops::{Add, AddAssign, Sub, SubAssign};


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Pos(pub i32, pub u8, pub i32);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Column(pub i32, pub i32);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ChunkIndex(pub i32, pub i32);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Vec3(pub i32, pub i32, pub i32);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Vec2(pub i32, pub i32);

#[derive(Debug, Copy, Clone)]
pub struct Rect {
    pub min: Column,
    pub max: Column
}

#[derive(Debug, Copy, Clone)]
pub struct Cuboid {
    pub min: Pos,
    pub max: Pos
}

pub struct Polyline(pub Vec<Column>);
pub struct Polygon(pub Vec<Column>);
// Todo: areas with shared borders/corners



impl Rect {
    pub fn contains(self, column: Column) -> bool {
        (self.min.0 <= column.0) &
        (self.min.1 <= column.1) &
        (self.max.0 >= column.0) &
        (self.max.1 >= column.1)
    }

    pub fn iter(self) -> impl Iterator<Item=Column> {
        (self.min.1 ..= self.max.1)
        .flat_map(move |z|(self.min.0 ..= self.max.0).map(move |x|Column(x,z)))
    }
}



impl Column {
    pub fn with_height(self, y: u8) -> Pos {
        Pos(self.0, y, self.1)
    }
}

impl From<Pos> for Column {
    fn from(pos: Pos) -> Self {
        Column(pos.0, pos.2)
    }
}



impl From<Column> for ChunkIndex {
    fn from(column: Column) -> Self {
        ChunkIndex(
            column.0.div_euclid(16),
            column.1.div_euclid(16)
        )
    }
}

impl From<Pos> for ChunkIndex {
    fn from(pos: Pos) -> Self {
        Column(pos.0, pos.2).into()
    }
}



impl Pos {
    // Unrelated to std::cmp::min
    pub fn min(self, other: Pos) -> Pos {
        Pos(
            self.0.min(other.0),
            self.1.min(other.1),
            self.2.min(other.2)
        )
    }

    pub fn max(self, other: Pos) -> Pos {
        Pos(
            self.0.max(other.0),
            self.1.max(other.1),
            self.2.max(other.2)
        )
    }
}



impl Cuboid {
    pub fn extend_to(self, pos: Pos) -> Self {
        Cuboid {
            min: self.min.min(pos),
            max: self.max.max(pos)
        }
    }

    pub fn size(self) -> Vec3 {
        self.max - self.min + Vec3(1,1,1)
    }

    pub fn iter(self) -> impl Iterator<Item=Pos> {
        (self.min.1 ..= self.max.1)
        .flat_map(move |y|(self.min.2 ..= self.max.2).map(move |z|(y,z)))
        .flat_map(move |(y, z)|(self.min.0 ..= self.max.0).map(move |x|Pos(x, y, z)))
    }
}



impl Sub<Pos> for Pos {
    type Output = Vec3;
    fn sub(self, rhs: Pos) -> Self::Output {
        Vec3(self.0 - rhs.0, self.1 as i32 - rhs.1 as i32, self.2 - rhs.2)
    }
}

impl Add<Vec3> for Pos {
    type Output = Pos;
    fn add(self, rhs: Vec3) -> Self::Output {
        Pos(self.0 + rhs.0, (self.1 as i32 + rhs.1) as u8, self.2 + rhs.2)
    }
}

impl AddAssign<Vec3> for Pos {
    fn add_assign(&mut self, rhs: Vec3) {
        *self = *self + rhs;
    }
}

impl Sub<Vec3> for Pos {
    type Output = Pos;
    fn sub(self, rhs: Vec3) -> Self::Output {
        Pos(self.0 - rhs.0, (self.1 as i32 - rhs.1) as u8, self.2 - rhs.2)
    }
}

impl SubAssign<Vec3> for Pos {
    fn sub_assign(&mut self, rhs: Vec3) {
        *self = *self - rhs;
    }
}



impl Sub<Column> for Column {
    type Output = Vec2;
    fn sub(self, rhs: Column) -> Self::Output {
        Vec2(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl Add<Vec2> for Column {
    type Output = Column;
    fn add(self, rhs: Vec2) -> Self::Output {
        Column(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign<Vec2> for Column {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = *self + rhs;
    }
}

impl Sub<Vec2> for Column {
    type Output = Column;
    fn sub(self, rhs: Vec2) -> Self::Output {
        Column(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl SubAssign<Vec2> for Column {
    fn sub_assign(&mut self, rhs: Vec2) {
        *self = *self - rhs;
    }
}


impl Add<Vec3> for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Self::Output {
        Vec3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl AddAssign<Vec3> for Vec3 {
    fn add_assign(&mut self, rhs: Vec3) {
        *self = *self + rhs;
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Self::Output {
        Vec3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }
}

impl SubAssign<Vec3> for Vec3 {
    fn sub_assign(&mut self, rhs: Vec3) {
        *self = *self - rhs;
    }
}