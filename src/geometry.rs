use itertools::Itertools;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};
//use num_traits::FromPrimitive;
use num_derive::FromPrimitive;

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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec2f(pub f32, pub f32);

#[derive(Debug, Copy, Clone)]
/// Both minimum and maximum are inclusive
pub struct Rect {
    pub min: Column,
    pub max: Column,
}

#[derive(Debug, Copy, Clone)]
pub struct Cuboid {
    pub min: Pos,
    pub max: Pos,
}

pub struct Polyline(pub Vec<Column>);
// Note: only valid with multiple points
pub struct Polygon(pub Vec<Column>);
// Todo: areas with shared borders/corners

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive, Hash)]
#[repr(u8)]
pub enum Axis {
    Y,
    X,
    Z,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive, Hash)]
#[repr(u8)]
pub enum HDir {
    ZPos,
    XNeg,
    ZNeg,
    XPos,
}

impl HDir {
    pub fn iter() -> impl Iterator<Item = HDir> {
        [HDir::ZPos, HDir::XNeg, HDir::ZNeg, HDir::XPos]
            .iter()
            .cloned()
    }

    pub fn clockwise(self) -> Self {
        match self {
            HDir::ZPos => HDir::XNeg,
            HDir::XNeg => HDir::ZNeg,
            HDir::ZNeg => HDir::XPos,
            HDir::XPos => HDir::ZPos,
        }
    }

    pub fn opposite(self) -> Self {
        self.clockwise().clockwise()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum FullDir {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg,
}

impl Vec2 {
    pub fn clockwise(self) -> Vec2 {
        Vec2(self.1, -self.0)
    }

    pub fn counterclockwise(self) -> Vec2 {
        Vec2(-self.1, self.0)
    }
}

impl From<HDir> for Vec2 {
    fn from(dir: HDir) -> Self {
        match dir {
            HDir::XPos => Vec2(1, 0),
            HDir::XNeg => Vec2(-1, 0),
            HDir::ZPos => Vec2(0, 1),
            HDir::ZNeg => Vec2(0, -1),
        }
    }
}

impl From<FullDir> for Vec3 {
    fn from(dir: FullDir) -> Self {
        match dir {
            FullDir::XPos => Vec3(1, 0, 0),
            FullDir::XNeg => Vec3(-1, 0, 0),
            FullDir::YPos => Vec3(0, 1, 0),
            FullDir::YNeg => Vec3(0, -1, 0),
            FullDir::ZPos => Vec3(0, 0, 1),
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
    pub fn size(self) -> Vec2 {
        self.max + Vec2(1, 1) - self.min
    }

    pub fn center(self) -> Column {
        self.min + self.size() * 0.5
    }

    pub fn contains(self, column: Column) -> bool {
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

    pub fn grow(self, amount: i32) -> Self {
        self.shrink(-amount)
    }

    pub fn shrink(self, amount: i32) -> Self {
        Self {
            min: self.min + Vec2(amount, amount),
            max: self.max - Vec2(amount, amount),
        }
    }

    pub fn iter(self) -> impl Iterator<Item = Column> {
        (self.min.1..=self.max.1)
            .flat_map(move |z| (self.min.0..=self.max.0).map(move |x| Column(x, z)))
    }

    pub fn border(self) -> impl Iterator<Item = Column> {
        (self.min.0..=self.max.0)
            .map(move |x| Column(x, self.min.1))
            .chain((self.min.1..=self.max.1).map(move |z| Column(self.max.0, z)))
            .chain(
                (self.min.0..=self.max.0)
                    .rev()
                    .map(move |x| Column(x, self.max.1)),
            )
            .chain(
                (self.min.1..=self.max.1)
                    .rev()
                    .map(move |z| Column(self.min.0, z)),
            )
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

    pub fn segments(&self) -> impl Iterator<Item = (Column, Column)> + '_ {
        self.0.iter().cloned().tuple_windows()
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

    // TODO important: Make sure this doesn't include borders! (also add version that does)
    pub fn contains(&self, column: Column) -> bool {
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
                } else {
                    if x > column.0 as f32 + 0.5 {
                        inside = true;
                    }
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

    pub fn segments(&self) -> impl Iterator<Item = (Column, Column)> + '_ {
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
    current: Column,
}

// Todo: look for a more performant solution
impl Iterator for PolygonIterator<'_> {
    type Item = Column;

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
    points: &'a [Column],
    i: usize,
    current_iter: ColumnLineIter,
    closed: bool,
}

impl Iterator for BorderIterator<'_> {
    type Item = Column;

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

impl Column {
    pub fn at(self, y: u8) -> Pos {
        Pos(self.0, y, self.1)
    }

    // Unrelated to std::cmp::min
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0), self.1.min(other.1))
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0), self.1.max(other.1))
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
        ChunkIndex(column.0.div_euclid(16), column.1.div_euclid(16))
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

impl ChunkIndex {
    pub fn area(self) -> Rect {
        Rect {
            min: Column(self.0 * 16, self.1 * 16),
            max: Column(self.0 * 16, self.1 * 16) + Vec2(15, 15),
        }
    }
}

impl Pos {
    // Unrelated to std::cmp::min
    pub fn min(self, other: Pos) -> Pos {
        Pos(
            self.0.min(other.0),
            self.1.min(other.1),
            self.2.min(other.2),
        )
    }

    pub fn max(self, other: Pos) -> Pos {
        Pos(
            self.0.max(other.0),
            self.1.max(other.1),
            self.2.max(other.2),
        )
    }
}

impl Cuboid {
    pub fn new(corner_a: Pos, corner_b: Pos) -> Cuboid {
        Cuboid {
            min: corner_a,
            max: corner_b,
        }
        .extend_to(corner_b)
    }

    pub fn extend_to(self, pos: Pos) -> Self {
        Cuboid {
            min: self.min.min(pos),
            max: self.max.max(pos),
        }
    }

    pub fn size(self) -> Vec3 {
        self.max - self.min + Vec3(1, 1, 1)
    }

    pub fn iter(self) -> impl Iterator<Item = Pos> {
        (self.min.1..=self.max.1)
            .flat_map(move |y| (self.min.2..=self.max.2).map(move |z| (y, z)))
            .flat_map(move |(y, z)| (self.min.0..=self.max.0).map(move |x| Pos(x, y, z)))
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
        Pos(
            self.0 + rhs.0,
            (self.1 as i32 + rhs.1) as u8,
            self.2 + rhs.2,
        )
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
        Pos(
            self.0 - rhs.0,
            (self.1 as i32 - rhs.1) as u8,
            self.2 - rhs.2,
        )
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

impl Add<Vec2> for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = *self + rhs;
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
        Self((self.0 * rhs) as i32, (self.1 * rhs) as i32)
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
        Self((self.0 % rhs) as i32, (self.1 % rhs) as i32)
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
        Self((self.0 % rhs) as i32, (self.1 % rhs) as i32)
    }
}

impl RemAssign<i32> for Vec2 {
    fn rem_assign(&mut self, rhs: i32) {
        *self = *self % rhs;
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

// TODO important: bool/enum whether to column should neighbor directly or whether diagonally is enough
pub struct ColumnLineIter {
    inner: bresenham::Bresenham,
    style: LineStyle,
    prev: Column,
    enqueued: Option<Column>,
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
    type Item = Column;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(column) = self.enqueued.take() {
            return Some(column);
        }
        let next = self.inner.next().map(|(x, z)| Column(x as i32, z as i32))?;

        if self.style == LineStyle::Thin {
            return Some(next);
        }

        if (self.prev.0 != next.0) & (self.prev.1 != next.1) {
            let interpol = if self.style == LineStyle::ThickWobbly {
                if rand(0.5) {
                    Column(self.prev.0, next.1)
                } else {
                    Column(next.0, self.prev.1)
                }
            } else {
                if self.prev.0 < next.0 {
                    Column(self.prev.0, next.1)
                } else if self.prev.0 > next.0 {
                    Column(next.0, self.prev.1)
                } else if self.prev.1 < next.1 {
                    Column(self.prev.0, next.1)
                } else {
                    Column(next.0, self.prev.1)
                }
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
    pub fn new(start: Column, end: Column, style: LineStyle) -> ColumnLineIter {
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
