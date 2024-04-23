use std::{
    ops::{Add, AddAssign},
    str::FromStr,
};

use bevy_math::Vec2Swizzles;
pub use bevy_math::{ivec2, ivec3, vec2, vec3, IVec2, IVec3, Vec2, Vec3};
use itertools::Itertools;
use num_derive::FromPrimitive;

use crate::*;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ChunkIndex(pub i32, pub i32);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// Both minimum and maximum are inclusive
pub struct Rect {
    pub min: IVec2,
    pub max: IVec2,
}

impl Rect {
    /// Corners don't need to be min/max ones
    pub fn new(corner_a: IVec2, corner_b: IVec2) -> Rect {
        Self {
            min: corner_a.min(corner_b),
            max: corner_a.max(corner_b),
        }
    }

    pub fn new_centered(center: IVec2, size: IVec2) -> Rect {
        Rect {
            min: center - size / 2,
            max: center + (size + IVec2::ONE) / 2,
        }
    }

    pub fn size(self) -> IVec2 {
        self.max + ivec2(1, 1) - self.min
    }

    pub fn total(self) -> i32 {
        self.size().x * self.size().y
    }

    pub fn center(self) -> IVec2 {
        self.min + self.size() / 2
    }

    pub fn center_vec2(self) -> Vec2 {
        self.min.as_vec2() + self.size().as_vec2() / 2.
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

    pub fn has_subrect(self, subrect: Rect) -> bool {
        (self.min.x <= subrect.min.x)
            & (self.min.y <= subrect.min.y)
            & (self.max.x >= subrect.max.x)
            & (self.max.y >= subrect.max.y)
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

    pub fn grow2(self, amount: IVec2) -> Self {
        Self {
            min: self.min - amount,
            max: self.max + amount,
        }
    }

    pub fn transposed(self) -> Self {
        Self::new_centered(self.center(), self.size().yx())
    }

    pub fn border(self) -> impl Iterator<Item = IVec2> {
        (self.min.x..=self.max.x)
            .map(move |x| ivec2(x, self.min.y))
            .chain((self.min.y..=self.max.y).map(move |y| ivec2(self.max.x, y)))
            .chain(
                (self.min.x..=self.max.x)
                    .rev()
                    .map(move |x| ivec2(x, self.max.y)),
            )
            .chain(
                (self.min.y..=self.max.y)
                    .rev()
                    .map(move |y| ivec2(self.min.x, y)),
            )
    }

    pub fn corners(self) -> impl Iterator<Item = IVec2> {
        Some(self.min)
            .into_iter()
            .chain(Some(ivec2(self.min.x, self.max.y)))
            .chain(Some(self.max))
            .chain(Some(ivec2(self.max.x, self.min.y)))
    }
}

impl Add<IVec2> for Rect {
    type Output = Self;

    fn add(self, offset: IVec2) -> Self::Output {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }
}

impl AddAssign<IVec2> for Rect {
    fn add_assign(&mut self, rhs: IVec2) {
        *self = *self + rhs
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
                self.column.y += 1;
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Cuboid {
    pub min: IVec3,
    pub max: IVec3,
}

pub struct Polyline(pub Vec<IVec2>);
// Note: only valid with multiple points
pub struct Polygon(pub Vec<IVec2>);
// Todo: areas with shared borders/corners

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum HAxis {
    X,
    Y,
}

impl HAxis {
    pub fn rotated(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }

    // Positive direction
    pub fn pos(self) -> IVec2 {
        match self {
            Self::X => ivec2(1, 0),
            Self::Y => ivec2(0, 1),
        }
    }
}

impl From<HAxis> for Axis {
    fn from(value: HAxis) -> Self {
        match value {
            HAxis::X => Axis::X,
            HAxis::Y => Axis::Y,
        }
    }
}

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

impl FromStr for Axis {
    type Err = ();

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "x" => Ok(Axis::X),
            "y" => Ok(Axis::Z),
            "z" => Ok(Axis::Y),
            _ => Err(()),
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
pub use HDir::*;

impl HDir {
    pub const ALL: [Self; 4] = [YPos, XNeg, YNeg, XPos];

    pub fn rotated(self, steps: i32) -> Self {
        match (self as i32 + steps).rem_euclid(4) {
            1 => XNeg,
            2 => YNeg,
            3 => XPos,
            _ => YPos,
        }
    }

    pub fn flipped(self, x: bool, y: bool) -> Self {
        match (self, x, y) {
            (XNeg, true, _) => XPos,
            (XPos, true, _) => XNeg,
            (YNeg, _, true) => YPos,
            (YPos, _, true) => YNeg,
            _ => self,
        }
    }

    pub fn difference(self, to: Self) -> i32 {
        (to as i32 - self as i32).rem_euclid(4)
    }

    pub fn to_str(self) -> &'static str {
        match self {
            YNeg => "north",
            XPos => "east",
            YPos => "south",
            XNeg => "west",
        }
    }
}

impl FromStr for HDir {
    type Err = ();

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "north" => Ok(YNeg),
            "east" => Ok(XPos),
            "south" => Ok(YPos),
            "west" => Ok(XNeg),
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

impl From<HDir> for FullDir {
    fn from(value: HDir) -> Self {
        match value {
            YPos => Self::YPos,
            XNeg => Self::XNeg,
            YNeg => Self::YNeg,
            XPos => Self::XPos,
        }
    }
}

pub const NEIGHBORS_2D: [IVec2; 4] = [ivec2(1, 0), ivec2(-1, 0), ivec2(0, 1), ivec2(0, -1)];

pub const NEIGHBORS_3D: [IVec3; 6] = [
    ivec3(1, 0, 0),
    ivec3(-1, 0, 0),
    ivec3(0, 1, 0),
    ivec3(0, -1, 0),
    ivec3(0, 0, 1),
    ivec3(0, 0, -1),
];

pub trait IVec2Ext {
    fn clockwise(self) -> Self;
    fn counterclockwise(self) -> Self;
    fn length(self) -> f32;
    fn touch_face(self, other: Self) -> bool;
    fn rotated(self, steps: i32) -> Self;
    // fn rotated(self, steps: i32) -> Self;
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

    fn touch_face(self, other: Self) -> bool {
        let diff = (self - other).abs();
        (diff == IVec2::X) | (diff == IVec2::Y)
    }

    /// Clockwise, 90° steps
    fn rotated(self, steps: i32) -> Self {
        match steps.rem_euclid(4) {
            1 => ivec2(-self.y, self.x),
            2 => ivec2(-self.x, -self.y),
            3 => ivec2(self.y, -self.x),
            _ => self,
        }
    }
}

pub trait IVec3Ext {
    fn rotated(self, steps: i32) -> Self;
    fn mirrored(self, axis: Axis) -> Self;
    fn add(self, offset: impl Into<IVec2>) -> Self;
}

impl IVec3Ext for IVec3 {
    /// Turns around the y axis
    fn rotated(self, steps: i32) -> Self {
        match steps % 4 {
            1 => ivec3(-self.y, self.x, self.z),
            2 => ivec3(-self.x, -self.y, self.z),
            3 => ivec3(self.y, -self.x, self.z),
            _ => self,
        }
    }

    fn mirrored(self, axis: Axis) -> Self {
        match axis {
            Axis::X => ivec3(-self.x, self.y, self.z),
            Axis::Y => ivec3(self.x, -self.y, self.z),
            Axis::Z => ivec3(self.x, self.y, -self.z),
        }
    }

    fn add(self, offset: impl Into<IVec2>) -> Self {
        (self.truncate() + offset.into()).extend(self.z)
    }
}

pub trait Vec3Ext {
    fn block(self) -> IVec3;
}

impl Vec3Ext for Vec3 {
    fn block(self) -> IVec3 {
        (self + Vec3::splat(0.5)).floor().as_ivec3()
    }
}

pub trait Vec2Ext {
    fn block(self) -> IVec2;
}

impl Vec2Ext for Vec2 {
    fn block(self) -> IVec2 {
        self.floor().as_ivec2()
    }
}

impl From<HDir> for IVec2 {
    fn from(dir: HDir) -> Self {
        match dir {
            XPos => ivec2(1, 0),
            XNeg => ivec2(-1, 0),
            YPos => ivec2(0, 1),
            YNeg => ivec2(0, -1),
        }
    }
}

impl Add<HDir> for IVec2 {
    type Output = IVec2;

    fn add(self, rhs: HDir) -> Self::Output {
        self + IVec2::from(rhs)
    }
}

impl AddAssign<HDir> for IVec2 {
    fn add_assign(&mut self, rhs: HDir) {
        *self = *self + rhs
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
    pub fn new(corner_a: IVec3, corner_b: IVec3) -> Self {
        Cuboid {
            min: corner_a,
            max: corner_b,
        }
        .extend_to(corner_b)
    }

    pub fn around(center: IVec3, radius: i32) -> Self {
        Cuboid {
            min: center - IVec3::splat(radius),
            max: center + IVec3::splat(radius),
        }
    }

    pub fn shrink(self, amount: i32) -> Self {
        self.grow(-amount)
    }

    pub fn grow(self, amount: i32) -> Self {
        Self {
            min: self.min - IVec3::ONE * amount,
            max: self.max + IVec3::ONE * amount,
        }
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

    pub fn d2(self) -> Rect {
        Rect {
            min: self.min.truncate(),
            max: self.max.truncate(),
        }
    }

    pub fn volume(self) -> i32 {
        self.size().x * self.size().y * self.size().z
    }
}

pub struct CuboidIter {
    z: i32,
    max_z: i32,
    layer_iter: RectIter,
}

impl Iterator for CuboidIter {
    type Item = IVec3;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(xy) = self.layer_iter.next() {
            Some(xy.extend(self.z))
        } else if self.z < self.max_z {
            self.z += 1;
            self.layer_iter.column = self.layer_iter.area.min;
            self.next()
        } else {
            None
        }
    }
}

impl IntoIterator for Cuboid {
    type Item = IVec3;

    type IntoIter = CuboidIter;

    fn into_iter(self) -> Self::IntoIter {
        CuboidIter {
            z: self.min.z,
            max_z: self.max.z,
            layer_iter: self.d2().into_iter(),
        }
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
                if rand() {
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
