use itertools::Itertools;
use std::{
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign},
    str::FromStr,
};
//use num_traits::FromPrimitive;
use num_derive::FromPrimitive;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ChunkIndex(pub i32, pub i32);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Vec3(pub i32, pub i32, pub i32);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Vec2(pub i32, pub i32);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec2f(pub f32, pub f32);

#[derive(Debug, Copy, Clone)]
/// Both minimum and maximum are inclusive
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Debug, Copy, Clone)]
pub struct Cuboid {
    pub min: Vec3,
    pub max: Vec3,
}

pub struct Polyline(pub Vec<Vec2>);
// Note: only valid with multiple points
pub struct Polygon(pub Vec<Vec2>);
// Todo: areas with shared borders/corners

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive, Hash)]
#[repr(u8)]
pub enum Axis {
    Y,
    X,
    Z,
}

impl Axis {
    pub fn to_str(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive, Hash)]
#[repr(u8)]
pub enum HDir {
    ZVec3,
    XNeg,
    ZNeg,
    XVec3,
}

impl HDir {
    pub fn iter() -> impl Iterator<Item = HDir> {
        [HDir::ZVec3, HDir::XNeg, HDir::ZNeg, HDir::XVec3]
            .iter()
            .cloned()
    }

    pub fn rotated(self, turns: u8) -> Self {
        match (self as u8 + turns) % 4 {
            1 => HDir::XNeg,
            2 => HDir::ZNeg,
            3 => HDir::XVec3,
            _ => HDir::ZVec3,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            HDir::ZNeg => "north",
            HDir::XVec3 => "east",
            HDir::ZVec3 => "south",
            HDir::XNeg => "west",
        }
    }
}

impl FromStr for HDir {
    type Err = ();

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "north" => Ok(HDir::ZNeg),
            "east" => Ok(HDir::XVec3),
            "south" => Ok(HDir::ZVec3),
            "west" => Ok(HDir::XNeg),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum FullDir {
    XVec3,
    XNeg,
    YVec3,
    YNeg,
    ZVec3,
    ZNeg,
}

impl Vec2 {
    pub fn clockwise(self) -> Vec2 {
        Vec2(self.1, -self.0)
    }

    pub fn counterclockwise(self) -> Vec2 {
        Vec2(-self.1, self.0)
    }

    pub fn len(self) -> f32 {
        ((self.0.pow(2) + self.1.pow(2)) as f32).powf(0.5)
    }
}

impl Vec3 {
    /// Turns around the y axis
    pub fn rotated(self, turns: u8) -> Self {
        match turns % 4 {
            1 => Vec3(-self.2, self.1, self.0),
            2 => Vec3(-self.0, self.1, -self.2),
            3 => Vec3(self.2, self.1, -self.0),
            _ => self,
        }
    }

    pub fn mirrord(self, axis: Axis) -> Self {
        match axis {
            Axis::X => Vec3(-self.0, self.1, self.2),
            Axis::Y => Vec3(self.0, -self.1, self.2),
            Axis::Z => Vec3(self.0, self.1, -self.2),
        }
    }
}

impl From<HDir> for Vec2 {
    fn from(dir: HDir) -> Self {
        match dir {
            HDir::XVec3 => Vec2(1, 0),
            HDir::XNeg => Vec2(-1, 0),
            HDir::ZVec3 => Vec2(0, 1),
            HDir::ZNeg => Vec2(0, -1),
        }
    }
}

impl From<FullDir> for Vec3 {
    fn from(dir: FullDir) -> Self {
        match dir {
            FullDir::XVec3 => Vec3(1, 0, 0),
            FullDir::XNeg => Vec3(-1, 0, 0),
            FullDir::YVec3 => Vec3(0, 1, 0),
            FullDir::YNeg => Vec3(0, -1, 0),
            FullDir::ZVec3 => Vec3(0, 0, 1),
            FullDir::ZNeg => Vec3(0, 0, -1),
        }
    }
}

impl From<HDir> for Vec3 {
    fn from(dir: HDir) -> Self {
        Vec2::from(dir).into()
    }
}

impl From<Vec2> for Vec3 {
    fn from(vec: Vec2) -> Self {
        Vec3(vec.0, 0, vec.1)
    }
}

impl Rect {
    pub fn new_centered(center: Vec2, size: Vec2) -> Rect {
        Rect {
            min: center - size / 2,
            max: center + size / 2,
        }
    }

    pub fn size(self) -> Vec2 {
        self.max + Vec2(1, 1) - self.min
    }

    pub fn center(self) -> Vec2 {
        self.min + self.size() * 0.5
    }

    pub fn contains(self, column: Vec2) -> bool {
        (self.min.0 <= column.0)
            & (self.min.1 <= column.1)
            & (self.max.0 >= column.0)
            & (self.max.1 >= column.1)
    }

    pub fn overlapps(self, other: Rect) -> bool {
        (self.min.0 <= other.max.0)
            & (self.max.0 >= other.min.0)
            & (self.min.1 <= other.max.1)
            & (self.max.1 >= other.min.1)
    }

    pub fn overlap(self, other: Rect) -> Rect {
        Rect {
            min: Vec2(self.min.0.max(other.min.0), self.min.1.max(other.min.1)),
            max: Vec2(self.max.0.min(other.max.0), self.max.1.min(other.max.1)),
        }
    }

    pub fn grow(self, amount: i32) -> Self {
        self.shrink(-amount)
    }

    pub fn shrink(self, amount: i32) -> Self {
        Self {
            min: self.min + Vec2(amount, amount),
            max: self.max - Vec2(amount, amount),
        }
    }

    pub fn border(self) -> impl Iterator<Item = Vec2> {
        (self.min.0..=self.max.0)
            .map(move |x| Vec2(x, self.min.1))
            .chain((self.min.1..=self.max.1).map(move |z| Vec2(self.max.0, z)))
            .chain(
                (self.min.0..=self.max.0)
                    .rev()
                    .map(move |x| Vec2(x, self.max.1)),
            )
            .chain(
                (self.min.1..=self.max.1)
                    .rev()
                    .map(move |z| Vec2(self.min.0, z)),
            )
    }
}

pub struct RectIter {
    area: Rect,
    column: Vec2,
}

impl Iterator for RectIter {
    type Item = Vec2;

    fn next(&mut self) -> Option<Self::Item> {
        if self.area.contains(self.column) {
            let column = self.column;
            self.column.0 += 1;
            if self.column.0 > self.area.max.0 {
                self.column.0 = self.area.min.0;
                self.column.1 += 1;
            }
            Some(column)
        } else {
            None
        }
    }
}

impl IntoIterator for Rect {
    type Item = Vec2;

    type IntoIter = RectIter;

    fn into_iter(self) -> Self::IntoIter {
        RectIter {
            area: self,
            column: self.min,
        }
    }
}

impl Polyline {
    pub fn iter(&self, style: LineStyle) -> BorderIterator {
        BorderIterator {
            points: &self.0,
            i: 0,
            current_iter: ColumnLineIter::new(self.0[0], self.0[1], style),
            closed: false,
        }
    }

    pub fn segments(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.0.iter().cloned().tuple_windows()
    }
}

impl Polygon {
    // Test code (not working):
    /*
    let test_polygon = Polygon(vec![
        Vec2(-10, -10),
        Vec2(0, 10),
        Vec2(10, -10),
        Vec2(2, -4),
        Vec2(-3, 0),
    ]);

    for column in test_polygon.border() {
        world[column.at_height(91)] = Block::Debug(1)
    }
    for column in test_polygon.iter() {
        world[column.at_height(90)] = Block::Debug(0);
    }
    */

    // TODO important: Make sure this doesn't include borders! (also add version that does)
    pub fn contains(&self, column: Vec2) -> bool {
        let mut inside = false;

        // Cast ray in x+ direction
        // Iterate through edges, check if we cross them
        for i in 0..self.0.len() {
            let line_start = self.0[i];
            let line_end = self.0[(i + 1) % self.0.len()];
            // Todo: fix corners
            if (line_start.1 <= column.1) & (line_end.1 > column.1)
                | (line_start.1 >= column.1) & (line_end.1 < column.1)
            {
                // Calculate possible intersection
                let angle = (line_end.1 - line_start.1) as f32 / (line_end.0 - line_start.0) as f32;
                let x = line_start.0 as f32 + ((column.1 - line_start.1) as f32 / angle + 0.5);
                if inside {
                    if x >= column.0 as f32 {
                        inside = false;
                    }
                } else if x > column.0 as f32 + 0.5 {
                    inside = true;
                }
            } else if (line_start.1 == column.1) & (line_end.1 == column.1) {
                if (line_start.0 <= column.0) & (line_end.0 >= column.0)
                    | (line_end.0 <= column.0) & (line_start.0 >= column.0)
                {
                    return false;
                } else {
                    // Todo: what if there are multiple segments right after another on the same z coord?
                    let before = self.0[(i - 1) % self.0.len()];
                    let after = self.0[(i + 2) % self.0.len()];
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
            bounds: Rect { min, max },
            current: min,
        }
    }

    pub fn border(&self, style: LineStyle) -> BorderIterator {
        BorderIterator {
            points: &self.0,
            i: 0,
            current_iter: ColumnLineIter::new(self.0[0], self.0[1], style),
            closed: true,
        }
    }

    pub fn segments(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.0
            .iter()
            .cloned()
            .tuple_windows()
            .chain(if self.0.len() <= 1 {
                None
            } else {
                Some((self.0[self.0.len() - 1], self.0[0]))
            })
    }
}

pub struct PolygonIterator<'a> {
    polygon: &'a Polygon,
    bounds: Rect, // We can't use Rect::iter() here :(
    current: Vec2,
}

// Todo: look for a more performant solution
impl Iterator for PolygonIterator<'_> {
    type Item = Vec2;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let column = self.current;
            self.current.0 += 1;
            if self.current.0 > self.bounds.max.0 {
                if self.current.1 == self.bounds.max.1 {
                    break;
                }
                self.current.0 = self.bounds.min.0;
                self.current.1 += 1;
            }
            if self.polygon.contains(column) {
                return Some(column);
            }
        }
        None
    }
}

pub struct BorderIterator<'a> {
    points: &'a [Vec2],
    i: usize,
    current_iter: ColumnLineIter,
    closed: bool,
}

impl Iterator for BorderIterator<'_> {
    type Item = Vec2;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_iter.next() {
            None => {
                if self.i
                    < if self.closed {
                        self.points.len() - 1
                    } else {
                        self.points.len() - 2
                    }
                {
                    self.i += 1;
                    self.current_iter = ColumnLineIter::new(
                        self.points[self.i],
                        self.points[(self.i + 1) % self.points.len()],
                        self.current_iter.style,
                    );
                    self.current_iter.next()
                } else {
                    None
                }
            }
            Some(next) => Some(next),
        }
    }
}

impl Vec2 {
    pub fn at(self, y: i32) -> Vec3 {
        Vec3(self.0, y, self.1)
    }

    // Unrelated to std::cmp::min
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0), self.1.min(other.1))
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0), self.1.max(other.1))
    }
}

impl From<Vec3> for Vec2 {
    fn from(pos: Vec3) -> Self {
        Vec2(pos.0, pos.2)
    }
}

impl From<(i32, i32)> for Vec2 {
    fn from((x, z): (i32, i32)) -> Self {
        Vec2(x, z)
    }
}

impl From<Vec2> for ChunkIndex {
    fn from(column: Vec2) -> Self {
        ChunkIndex(column.0.div_euclid(16), column.1.div_euclid(16))
    }
}

impl From<Vec3> for ChunkIndex {
    fn from(pos: Vec3) -> Self {
        Vec2(pos.0, pos.2).into()
    }
}

impl From<(i32, i32)> for ChunkIndex {
    fn from((x, z): (i32, i32)) -> Self {
        ChunkIndex(x, z)
    }
}

impl ChunkIndex {
    pub fn area(self) -> Rect {
        Rect {
            min: Vec2(self.0 * 16, self.1 * 16),
            max: Vec2(self.0 * 16, self.1 * 16) + Vec2(15, 15),
        }
    }
}

impl Vec3 {
    // Unrelated to std::cmp::min
    pub fn min(self, other: Vec3) -> Vec3 {
        Vec3(
            self.0.min(other.0),
            self.1.min(other.1),
            self.2.min(other.2),
        )
    }

    pub fn max(self, other: Vec3) -> Vec3 {
        Vec3(
            self.0.max(other.0),
            self.1.max(other.1),
            self.2.max(other.2),
        )
    }
}

impl Cuboid {
    pub fn new(corner_a: Vec3, corner_b: Vec3) -> Cuboid {
        Cuboid {
            min: corner_a,
            max: corner_b,
        }
        .extend_to(corner_b)
    }

    pub fn extend_to(self, pos: Vec3) -> Self {
        Cuboid {
            min: self.min.min(pos),
            max: self.max.max(pos),
        }
    }

    pub fn size(self) -> Vec3 {
        self.max - self.min + Vec3(1, 1, 1)
    }

    pub fn iter(self) -> impl Iterator<Item = Vec3> {
        (self.min.1..=self.max.1)
            .flat_map(move |y| (self.min.2..=self.max.2).map(move |z| (y, z)))
            .flat_map(move |(y, z)| (self.min.0..=self.max.0).map(move |x| Vec3(x, y, z)))
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Self::Output {
        Vec3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
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

impl SubAssign<Vec3> for Vec3 {
    fn sub_assign(&mut self, rhs: Vec3) {
        *self = *self - rhs;
    }
}

impl Add<Vec2> for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec2) -> Self::Output {
        Vec3(self.0 + rhs.0, self.1, self.2 + rhs.1)
    }
}

impl AddAssign<Vec2> for Vec3 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = *self + rhs;
    }
}

impl Sub<Vec2> for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec2) -> Self::Output {
        Vec3(self.0 - rhs.0, self.1, self.2 - rhs.1)
    }
}

impl SubAssign<Vec2> for Vec3 {
    fn sub_assign(&mut self, rhs: Vec2) {
        *self = *self - rhs;
    }
}

impl Sub<Vec2> for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Self::Output {
        Vec2(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl Add<Vec2> for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Self::Output {
        Vec2(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = *self + rhs;
    }
}

impl SubAssign<Vec2> for Vec2 {
    fn sub_assign(&mut self, rhs: Vec2) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Self::Output {
        Self((self.0 as f32 * rhs) as i32, (self.1 as f32 * rhs) as i32)
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Mul<i32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: i32) -> Self::Output {
        Self(self.0 * rhs, self.1 * rhs)
    }
}

impl MulAssign<i32> for Vec2 {
    fn mul_assign(&mut self, rhs: i32) {
        *self = *self * rhs;
    }
}

impl Div<i32> for Vec2 {
    type Output = Vec2;
    fn div(self, rhs: i32) -> Self::Output {
        Self(self.0 / rhs, self.1 / rhs)
    }
}

impl DivAssign<i32> for Vec2 {
    fn div_assign(&mut self, rhs: i32) {
        *self = *self / rhs;
    }
}

impl Rem<i32> for Vec2 {
    type Output = Vec2;
    fn rem(self, rhs: i32) -> Self::Output {
        Self(self.0 % rhs, self.1 % rhs)
    }
}

impl RemAssign<i32> for Vec2 {
    fn rem_assign(&mut self, rhs: i32) {
        *self = *self % rhs;
    }
}

// TODO important: bool/enum whether to column should neighbor directly or whether diagonally is enough
pub struct ColumnLineIter {
    inner: bresenham::Bresenham,
    style: LineStyle,
    prev: Vec2,
    enqueued: Option<Vec2>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum LineStyle {
    /// Allow column only connecting via corners
    Thin,
    /// Make full connections
    Thick,
    /// Like Thick, but chose side to insert filler columns randomly
    ThickWobbly,
}

impl Iterator for ColumnLineIter {
    type Item = Vec2;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(column) = self.enqueued.take() {
            return Some(column);
        }
        let next = self.inner.next().map(|(x, z)| Vec2(x as i32, z as i32))?;

        if self.style == LineStyle::Thin {
            return Some(next);
        }

        if (self.prev.0 != next.0) & (self.prev.1 != next.1) {
            let interpol = if self.style == LineStyle::ThickWobbly {
                if rand(0.5) {
                    Vec2(self.prev.0, next.1)
                } else {
                    Vec2(next.0, self.prev.1)
                }
            } else if self.prev.0 < next.0 {
                Vec2(self.prev.0, next.1)
            } else if self.prev.0 > next.0 {
                Vec2(next.0, self.prev.1)
            } else if self.prev.1 < next.1 {
                Vec2(self.prev.0, next.1)
            } else {
                Vec2(next.0, self.prev.1)
            };
            self.enqueued = Some(next);
            self.prev = next;
            Some(interpol)
        } else {
            self.prev = next;
            Some(next)
        }
    }
}

impl ColumnLineIter {
    pub fn new(start: Vec2, end: Vec2, style: LineStyle) -> ColumnLineIter {
        ColumnLineIter {
            inner: bresenham::Bresenham::new(
                (start.0 as isize, start.1 as isize),
                (end.0 as isize, end.1 as isize),
            ),
            style,
            prev: start,
            enqueued: None,
        }
    }
}

/* These don't really fit here, but oh well. */
// TODO: Use world seed (and move to World?)
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

// Inclusive range
pub fn rand_range(min: i32, max: i32) -> i32 {
    rand::Rng::gen_range(&mut rand::thread_rng(), min, max)
}
