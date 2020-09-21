use std::ops::{Add, AddAssign, Sub, SubAssign};
use itertools::Itertools;


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
// Note: only valid with multiple points
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

impl Polyline {
    pub fn iter(&self) -> BorderIterator {
        BorderIterator {
            points: &self.0,
            i: 0,
            current_iter: ColumnLineIter::new(self.0[0], self.0[1]),
            closed: false
        }
    } 
}

impl Polygon {
    // Test code (not working):
    /*
    let test_polygon = Polygon(vec![
        Column(-10, -10),
        Column(0, 10),
        Column(10, -10),
        Column(2, -4),
        Column(-3, 0),
    ]);

    for column in test_polygon.border() {
        world[column.at_height(91)] = Block::Debug(1)
    }
    for column in test_polygon.iter() {
        world[column.at_height(90)] = Block::Debug(0);
    }
    */

    // This doesn't include borders
    pub fn contains(&self, column: Column) -> bool {
        let mut inside = false;

        // Cast ray in x+ direction
        // Iterate through edges, check if we cross them
        for i in 0..self.0.len() {
            let line_start = self.0[i];
            let line_end = self.0[(i+1)%self.0.len()];
            // Todo: fix corners
            if (line_start.1 <= column.1) & (line_end.1 > column.1)
             | (line_start.1 >= column.1) & (line_end.1 < column.1)
            {
                // Calculate possible intersection
                let angle = (line_end.1-line_start.1) as f32 / (line_end.0-line_start.0) as f32;
                let x = line_start.0 as f32 + ((column.1-line_start.1) as f32 / angle + 0.5);
                if inside {
                    if x >= column.0 as f32 {
                        inside = false;
                    }
                } else {
                    if x > column.0 as f32 + 0.5 {
                        inside = true;
                    }
                }
            } else if (line_start.1 == column.1) & (line_end.1 == column.1) {
                if (line_start.0 <= column.0) & (line_end.0 >= column.0)
                 | (line_end.0 <= column.0) & (line_start.0 >= column.0) {
                    return false
                } else {
                    // Todo: what if there are multiple segments right after another on the same z coord?
                    let before = self.0[(i-1)%self.0.len()];
                    let after = self.0[(i+2)%self.0.len()];
                    if before.1.signum() != after.1.signum() {
                        inside ^= true;
                    }
                }
            }
        }

        inside
    }

    pub fn iter(&self) -> PolygonIterator {
        let mut min = self.0[0];
        let mut max = self.0[0];
        for column in self.0.iter() {
            min = min.min(*column);
            max = max.max(*column);
        }
        PolygonIterator {
            polygon: self,
            bounds: Rect {min, max},
            current: min
        }
    }

    pub fn border(&self) -> BorderIterator {
        BorderIterator {
            points: &self.0,
            i: 0,
            current_iter: ColumnLineIter::new(self.0[0], self.0[1]),
            closed: true
        }
    }

    pub fn segments(&self) -> impl Iterator<Item=(Column, Column)> + '_ {
        self.0.iter().cloned().tuple_windows()
            .chain(if self.0.len() <= 1 {None} else {Some((self.0[self.0.len()-1], self.0[0]))})
    }
}

pub struct PolygonIterator<'a> {
    polygon: &'a Polygon,
    bounds: Rect, // We can't use Rect::iter() here :(
    current: Column
}

// Todo: look for a more performant solution
impl Iterator for PolygonIterator<'_> {
    type Item = Column;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current != self.bounds.max {
            let column = self.current;
            self.current.0 += 1;
            if self.current.0 > self.bounds.max.0 {
                self.current.0 = self.bounds.min.0;
                self.current.1 += 1;
            }
            if self.polygon.contains(column) {
                return Some(column)
            }
        }
        None
    }
}

pub struct BorderIterator<'a> {
    points: &'a [Column],
    i: usize,
    current_iter: ColumnLineIter,
    closed: bool
}

impl Iterator for BorderIterator<'_> {
    type Item = Column;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_iter.next() {
            None => if self.i < if self.closed {self.points.len()-1} else {self.points.len()-2} {
                self.i += 1;
                self.current_iter = ColumnLineIter::new(
                    self.points[self.i], 
                    self.points[(self.i+1)%self.points.len()]
                );
                self.current_iter.next()
            } else {
                None
            },
            Some(next) => Some(next)
        }
    }
}




impl Column {
    pub fn at_height(self, y: u8) -> Pos {
        Pos(self.0, y, self.1)
    }

    // Unrelated to std::cmp::min
    pub fn min(self, other: Self) -> Self {
        Self (
            self.0.min(other.0),
            self.1.min(other.1),
        )
    }

    pub fn max(self, other: Self) -> Self {
        Self (
            self.0.max(other.0),
            self.1.max(other.1),
        )
    }
}

impl From<Pos> for Column {
    fn from(pos: Pos) -> Self {
        Column(pos.0, pos.2)
    }
}

impl From<(i32, i32)> for Column {
    fn from((x, z): (i32, i32)) -> Self {
        Column(x, z)
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

impl From<(i32, i32)> for ChunkIndex {
    fn from((x, z): (i32, i32)) -> Self {
        ChunkIndex(x, z)
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
    pub fn new(corner_a: Pos, corner_b: Pos) -> Cuboid {
        Cuboid { min: corner_a, max: corner_b}
        .extend_to(corner_b)
    }

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

impl Add<Vec2> for Pos {
    type Output = Pos;
    fn add(self, rhs: Vec2) -> Self::Output {
        Pos(self.0 + rhs.0, self.1, self.2 + rhs.1)
    }
}

impl AddAssign<Vec2> for Pos {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = *self + rhs;
    }
}

impl Sub<Vec2> for Pos {
    type Output = Pos;
    fn sub(self, rhs: Vec2) -> Self::Output {
        Pos(self.0 - rhs.0, self.1, self.2 - rhs.1)
    }
}

impl SubAssign<Vec2> for Pos {
    fn sub_assign(&mut self, rhs: Vec2) {
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



pub struct ColumnLineIter(bresenham::Bresenham);

impl Iterator for ColumnLineIter {
    type Item = Column;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(x, z)|Column(x as i32, z as i32))
    }
}

impl ColumnLineIter {
    pub fn new(start: Column, end: Column) -> ColumnLineIter {
        ColumnLineIter(
            bresenham::Bresenham::new((start.0 as isize, start.1 as isize), (end.0 as isize, end.1 as isize))
        )
    }
}






pub fn rand(prob: f32) -> bool {
    rand::random::<f32>() < prob
} 

pub fn rand_1(prob: f32) -> i32 {
    if rand::random::<f32>() < prob {
        rand::Rng::gen_range(&mut rand::thread_rng(), -1, 2)
    } else {
        0
    }
}

pub fn rand_2(prob: f32) -> Vec2 {
    Vec2(rand_1(prob), rand_1(prob))
}

pub fn rand_3(prob: f32) -> Vec3 {
    Vec3(rand_1(prob), rand_1(prob), rand_1(prob))
}