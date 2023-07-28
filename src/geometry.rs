use itertools::Itertools;
use num_derive::FromPrimitive;
use std::str::FromStr;

pub use bevy_math::{ivec2, ivec3, vec2, vec3, IVec2, IVec3, Vec2, Vec3};

//
// TODO: Swap Y and Z??
// Makes things simpler in some cases, adds confusion in others
//

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ChunkIndex(pub i32, pub i32);

#[derive(Debug, Copy, Clone)]
/// Both minimum and maximum are inclusive
pub struct Rect {
    pub min: IVec2,
    pub max: IVec2,
}

#[derive(Debug, Copy, Clone)]
pub struct Cuboid {
    pub min: IVec3,
    pub max: IVec3,
}

pub struct Polyline(pub Vec<IVec2>);
// Note: only valid with multiple points
pub struct Polygon(pub Vec<IVec2>);
// Todo: areas with shared borders/corners

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive, Hash)]
#[repr(u8)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn to_str(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "z",
            Axis::Z => "y",
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive, Hash)]
#[repr(u8)]
pub enum HDir {
    YPos,
    XNeg,
    YNeg,
    XPos,
}

impl HDir {
    pub fn iter() -> impl Iterator<Item = HDir> {
        [HDir::YPos, HDir::XNeg, HDir::YNeg, HDir::XPos]
            .iter()
            .cloned()
    }

    pub fn rotated(self, turns: u8) -> Self {
        match (self as u8 + turns) % 4 {
            1 => HDir::XNeg,
            2 => HDir::YNeg,
            3 => HDir::XPos,
            _ => HDir::YPos,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            HDir::YNeg => "north",
            HDir::XPos => "east",
            HDir::YPos => "south",
            HDir::XNeg => "west",
        }
    }
}

impl FromStr for HDir {
    type Err = ();

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "north" => Ok(HDir::YNeg),
            "east" => Ok(HDir::XPos),
            "south" => Ok(HDir::YPos),
            "west" => Ok(HDir::XNeg),
            _ => Err(()),
        }
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

pub trait IVec2Ext {
    fn clockwise(self) -> Self;
    fn counterclockwise(self) -> Self;
    fn length(self) -> f32;
}

impl IVec2Ext for IVec2 {
    fn clockwise(self) -> IVec2 {
        ivec2(self.x, -self.y)
    }

    fn counterclockwise(self) -> IVec2 {
        ivec2(-self.x, self.y)
    }

    fn length(self) -> f32 {
        ((self.x.pow(2) + self.y.pow(2)) as f32).powf(0.5)
    }
}

pub trait IVec3Ext {
    fn rotated(self, turns: u8) -> Self;
    fn mirrord(self, axis: Axis) -> Self;
}

impl IVec3Ext for IVec3 {
    /// Turns around the y axis
    fn rotated(self, turns: u8) -> Self {
        match turns % 4 {
            1 => ivec3(-self.y, self.x, self.z),
            2 => ivec3(-self.x, -self.y, self.z),
            3 => ivec3(self.y, -self.x, self.z),
            _ => self,
        }
    }

    fn mirrord(self, axis: Axis) -> Self {
        match axis {
            Axis::X => ivec3(-self.x, self.y, self.z),
            Axis::Y => ivec3(self.x, -self.y, self.z),
            Axis::Z => ivec3(self.x, self.y, -self.z),
        }
    }
}

impl From<HDir> for IVec2 {
    fn from(dir: HDir) -> Self {
        match dir {
            HDir::XPos => ivec2(1, 0),
            HDir::XNeg => ivec2(-1, 0),
            HDir::YPos => ivec2(0, 1),
            HDir::YNeg => ivec2(0, -1),
        }
    }
}

impl From<FullDir> for IVec3 {
    fn from(dir: FullDir) -> Self {
        match dir {
            FullDir::XPos => ivec3(1, 0, 0),
            FullDir::XNeg => ivec3(-1, 0, 0),
            FullDir::YPos => ivec3(0, 1, 0),
            FullDir::YNeg => ivec3(0, -1, 0),
            FullDir::ZPos => ivec3(0, 0, 1),
            FullDir::ZNeg => ivec3(0, 0, -1),
        }
    }
}

impl From<HDir> for IVec3 {
    fn from(dir: HDir) -> Self {
        IVec2::from(dir).extend(0)
    }
}

impl Rect {
    pub fn new_centered(center: IVec2, size: IVec2) -> Rect {
        Rect {
            min: center - size / 2,
            max: center + size / 2,
        }
    }

    pub fn size(self) -> IVec2 {
        self.max + ivec2(1, 1) - self.min
    }

    pub fn center(self) -> IVec2 {
        self.min + self.size() / 2
    }

    pub fn contains(self, column: IVec2) -> bool {
        (self.min.x <= column.x)
            & (self.min.y <= column.y)
            & (self.max.x >= column.x)
            & (self.max.y >= column.y)
    }

    pub fn overlapps(self, other: Rect) -> bool {
        (self.min.x <= other.max.x)
            & (self.max.x >= other.min.x)
            & (self.min.y <= other.max.y)
            & (self.max.y >= other.min.y)
    }

    pub fn overlap(self, other: Rect) -> Rect {
        Rect {
            min: ivec2(self.min.x.max(other.min.x), self.min.x.max(other.min.y)),
            max: ivec2(self.max.x.min(other.max.x), self.max.x.min(other.max.y)),
        }
    }

    pub fn grow(self, amount: i32) -> Self {
        self.shrink(-amount)
    }

    pub fn shrink(self, amount: i32) -> Self {
        Self {
            min: self.min + ivec2(amount, amount),
            max: self.max - ivec2(amount, amount),
        }
    }

    pub fn border(self) -> impl Iterator<Item = IVec2> {
        (self.min.x..=self.max.x)
            .map(move |x| ivec2(x, self.min.y))
            .chain((self.min.y..=self.max.y).map(move |z| ivec2(self.max.x, z)))
            .chain(
                (self.min.x..=self.max.x)
                    .rev()
                    .map(move |x| ivec2(x, self.max.x)),
            )
            .chain(
                (self.min.y..=self.max.y)
                    .rev()
                    .map(move |z| ivec2(self.min.x, z)),
            )
    }
}

pub struct RectIter {
    area: Rect,
    column: IVec2,
}

impl Iterator for RectIter {
    type Item = IVec2;

    fn next(&mut self) -> Option<Self::Item> {
        if self.area.contains(self.column) {
            let column = self.column;
            self.column.x += 1;
            if self.column.x > self.area.max.x {
                self.column.x = self.area.min.x;
                self.column.x += 1;
            }
            Some(column)
        } else {
            None
        }
    }
}

impl IntoIterator for Rect {
    type Item = IVec2;

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

    pub fn segments(&self) -> impl Iterator<Item = (IVec2, IVec2)> + '_ {
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
    pub fn contains(&self, column: IVec2) -> bool {
        let mut inside = false;

        // Cast ray in x+ direction
        // Iterate through edges, check if we cross them
        for i in 0..self.0.len() {
            let line_start = self.0[i];
            let line_end = self.0[(i + 1) % self.0.len()];
            // Todo: fix corners
            if (line_start.y <= column.y) & (line_end.y > column.y)
                | (line_start.y >= column.y) & (line_end.y < column.y)
            {
                // Calculate possible intersection
                let angle = (line_end.y - line_start.y) as f32 / (line_end.x - line_start.x) as f32;
                let x = line_start.x as f32 + ((column.y - line_start.y) as f32 / angle + 0.5);
                if inside {
                    if x >= column.x as f32 {
                        inside = false;
                    }
                } else if x > column.x as f32 + 0.5 {
                    inside = true;
                }
            } else if (line_start.y == column.y) & (line_end.y == column.y) {
                if (line_start.x <= column.x) & (line_end.x >= column.x)
                    | (line_end.x <= column.x) & (line_start.x >= column.x)
                {
                    return false;
                } else {
                    // Todo: what if there are multiple segments right after another on the same z coord?
                    let before = self.0[(i - 1) % self.0.len()];
                    let after = self.0[(i + 2) % self.0.len()];
                    if before.y.signum() != after.y.signum() {
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

    pub fn segments(&self) -> impl Iterator<Item = (IVec2, IVec2)> + '_ {
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
    current: IVec2,
}

// Todo: look for a more performant solution
impl Iterator for PolygonIterator<'_> {
    type Item = IVec2;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let column = self.current;
            self.current.x += 1;
            if self.current.x > self.bounds.max.x {
                if self.current.y == self.bounds.max.y {
                    break;
                }
                self.current.x = self.bounds.min.x;
                self.current.y += 1;
            }
            if self.polygon.contains(column) {
                return Some(column);
            }
        }
        None
    }
}

pub struct BorderIterator<'a> {
    points: &'a [IVec2],
    i: usize,
    current_iter: ColumnLineIter,
    closed: bool,
}

impl Iterator for BorderIterator<'_> {
    type Item = IVec2;

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

impl From<IVec2> for ChunkIndex {
    fn from(column: IVec2) -> Self {
        ChunkIndex(column.x.div_euclid(16), column.y.div_euclid(16))
    }
}

impl From<IVec3> for ChunkIndex {
    fn from(pos: IVec3) -> Self {
        ivec2(pos.x, pos.y).into()
    }
}

impl From<(i32, i32)> for ChunkIndex {
    fn from((x, y): (i32, i32)) -> Self {
        ChunkIndex(x, y)
    }
}

impl ChunkIndex {
    pub fn area(self) -> Rect {
        Rect {
            min: ivec2(self.0 * 16, self.1 * 16),
            max: ivec2(self.0 * 16 + 15, self.1 * 16 + 15),
        }
    }
}

impl Cuboid {
    pub fn new(corner_a: IVec3, corner_b: IVec3) -> Cuboid {
        Cuboid {
            min: corner_a,
            max: corner_b,
        }
        .extend_to(corner_b)
    }

    pub fn extend_to(self, pos: IVec3) -> Self {
        Cuboid {
            min: self.min.min(pos),
            max: self.max.max(pos),
        }
    }

    pub fn size(self) -> IVec3 {
        self.max - self.min + IVec3::splat(1)
    }

    pub fn iter(self) -> impl Iterator<Item = IVec3> {
        (self.min.z..=self.max.z)
            .flat_map(move |y| (self.min.y..=self.max.y).map(move |z| (y, z)))
            .flat_map(move |(y, z)| (self.min.x..=self.max.x).map(move |x| ivec3(x, y, z)))
    }
}

// TODO important: bool/enum whether to column should neighbor directly or whether diagonally is enough
pub struct ColumnLineIter {
    inner: bresenham::Bresenham,
    style: LineStyle,
    prev: IVec2,
    enqueued: Option<IVec2>,
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
    type Item = IVec2;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(column) = self.enqueued.take() {
            return Some(column);
        }
        let next = self.inner.next().map(|(x, z)| ivec2(x as i32, z as i32))?;

        if self.style == LineStyle::Thin {
            return Some(next);
        }

        if (self.prev.x != next.x) & (self.prev.y != next.y) {
            let interpol = if self.style == LineStyle::ThickWobbly {
                if rand(0.5) {
                    ivec2(self.prev.x, next.x)
                } else {
                    ivec2(next.x, self.prev.x)
                }
            } else if self.prev.x < next.x {
                ivec2(self.prev.x, next.y)
            } else if self.prev.x > next.x {
                ivec2(next.x, self.prev.y)
            } else if self.prev.y < next.y {
                ivec2(self.prev.x, next.y)
            } else {
                ivec2(next.x, self.prev.y)
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
    pub fn new(start: IVec2, end: IVec2, style: LineStyle) -> ColumnLineIter {
        ColumnLineIter {
            inner: bresenham::Bresenham::new(
                (start.x as isize, start.y as isize),
                (end.x as isize, end.y as isize),
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

pub fn rand_2(prob: f32) -> IVec2 {
    ivec2(rand_1(prob), rand_1(prob))
}

pub fn rand_3(prob: f32) -> IVec3 {
    ivec3(rand_1(prob), rand_1(prob), rand_1(prob))
}

// Inclusive range
pub fn rand_range(min: i32, max: i32) -> i32 {
    rand::Rng::gen_range(&mut rand::thread_rng(), min, max)
}
