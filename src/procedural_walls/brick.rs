//! Brick structure for procedural wall geometry.
//!
//! This module defines the `Brick` struct that represents individual bricks in a procedural wall.
//! Each brick has UV coordinates for positioning and bounds within the wall pattern, as well as
//! a world-space transform for rendering.

use bevy::prelude::{Transform, Vec2};

/// Represents a single brick in a procedurally generated wall.
///
/// The brick maintains both parametric UV coordinates (for pattern definition) and
/// a world-space transform (for rendering). The `bounds_uv` define the size of the brick
/// in normalized coordinates, while `pivot_uv` marks its center position.
#[derive(Clone)]
pub struct Brick {
    /// Normalized UV bounds (width and height) of the brick in wall-space [0, 1]
    pub bounds_uv: Vec2,
    /// Normalized UV pivot point (center) of the brick in wall-space [0, 1]
    pub pivot_uv: Vec2,
    /// World-space transform including translation, rotation, and scale
    pub transform: Transform,
}
