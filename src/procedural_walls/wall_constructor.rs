//! Wall constructor for procedural brick wall generation along curves.
//!
//! This module implements procedural brick wall generation using seeded randomization.
//! Bricks are positioned along a curve path with randomized dimensions and optional
//! horizontal splitting for realistic brick patterns.

use super::brick::Brick;
use super::curve::Curve;
use bevy::math::{Mat3, Quat};
use bevy::prelude::{Transform, Vec2, Vec3};
use fastrand::Rng;

// High-fidelity rectangular chiseled dimensions (approx 2.5:1 ratio)
#[allow(dead_code)]
const BRICK_WIDTH: f32 = 0.55;
#[allow(dead_code)]
const BRICK_WIDTH_VARIANCE: f32 = 0.15;

#[allow(dead_code)]
const BRICK_HEIGHT: f32 = 0.22;
#[allow(dead_code)]
const BRICK_HEIGHT_VARIANCE: f32 = 0.05;

#[allow(dead_code)]
const BRICK_DEPTH: f32 = 0.35;
#[allow(dead_code)]
const BRICK_DEPTH_VARIANCE: f32 = 0.08;

pub struct WallConstructor;

impl WallConstructor {
    pub fn from_curve(
        curve: &Curve,
        wall_height: f32,
        get_ground_y: impl Fn(Vec3) -> f32,
    ) -> Vec<Brick> {
        Self::from_curve_with_style(
            curve,
            wall_height,
            super::WallStyle::ClassicBrick,
            get_ground_y,
        )
    }

    pub fn from_curve_with_style(
        curve: &Curve,
        wall_height: f32,
        style: super::WallStyle,
        get_ground_y: impl Fn(Vec3) -> f32,
    ) -> Vec<Brick> {
        let mut rng = fastrand::Rng::with_seed(0);

        let (brick_w, brick_h, _brick_d, width_variance, height_variance) = match style {
            super::WallStyle::ClassicBrick => (0.55, 0.22, 0.35, 0.15, 0.05),
            super::WallStyle::PalisadeFence => (0.28, wall_height, 0.28, 0.04, 0.0),
            super::WallStyle::GraniteFortress => (0.75, 0.38, 0.48, 0.15, 0.05),
            super::WallStyle::LogTimber => (0.95, 0.26, 0.32, 0.12, 0.03),
            super::WallStyle::CyberMetal => (0.55, 0.32, 0.32, 0.08, 0.02),
        };

        let wall_length: f32 = curve.length;
        let bricks_per_row = (wall_length / brick_w).ceil().max(1.0) as usize;

        // Calculate curve span and vertical top in world space using true ground heights
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for p in &curve.points {
            let gy = get_ground_y(*p);
            if gy < min_y {
                min_y = gy;
            }
            if p.y > max_y {
                max_y = p.y;
            }
        }
        let top_y = max_y + wall_height;
        let max_span = (top_y - min_y).max(wall_height);

        // Calculate rows globally from the flat top down to the lowest ground
        let max_row_count = (max_span / brick_h).ceil().max(1.0) as usize;

        let mut bricks = Vec::with_capacity(
            max_row_count * bricks_per_row + (max_row_count * bricks_per_row) / 3,
        );

        let mut split_points: Vec<f32> = Vec::with_capacity(bricks_per_row + 2);
        let mut perturbed: Vec<f32> = Vec::with_capacity(bricks_per_row + 2);
        let mut brick_row: Vec<Brick> = Vec::with_capacity(bricks_per_row + bricks_per_row / 2);

        let rows = random_splits(max_row_count, height_variance / max_span, &mut rng);

        for r in 0..max_row_count {
            let row_u = rows[r];

            // Stagger alternate rows (running bond pattern!)
            let is_odd = r % 2 == 1;
            split_points.clear();
            if is_odd && style != super::WallStyle::PalisadeFence {
                split_points.push(0.0);
                for k in 1..=bricks_per_row {
                    let u = (k as f32 - 0.5) / (bricks_per_row as f32);
                    if u < 1.0 {
                        split_points.push(u);
                    }
                }
                split_points.push(1.0);
            } else {
                for k in 0..=bricks_per_row {
                    split_points.push(k as f32 / bricks_per_row as f32);
                }
            }

            perturbed.clear();
            perturb_splits_into(
                &split_points,
                width_variance / wall_length,
                &mut rng,
                &mut perturbed,
            );
            let brick_widths = &perturbed;

            let brick_height = if let Some(&next_row_u) = rows.get(r + 1) {
                (next_row_u - row_u) * max_span
            } else {
                brick_h + (rng.f32() - 0.5) * height_variance
            };

            brick_row.clear();
            for j in 0..brick_widths.len() {
                if let Some(&next_u) = brick_widths.get(j + 1) {
                    let this_u = brick_widths[j];

                    // Skip some top-most row bricks for weathered crenellation/castle effect
                    if r == 0 && rng.f32() < 0.35 {
                        continue;
                    }

                    let brick_depth = BRICK_DEPTH + (rng.f32() - 0.5) * BRICK_DEPTH_VARIANCE;

                    // Vertically split brick chance (except top row) for stone masonry variety
                    if rng.f32() < 0.4 && r != 0 {
                        let range = (0.3, 0.7);
                        let random_split = rng.f32() * (range.1 - range.0) + range.0;
                        let pivot_u = ((next_u + this_u) / 2.0).clamp(0.0, 1.0);

                        let height_u_1 = brick_height / max_span * random_split;
                        let height_u_2 = brick_height / max_span * (1.0 - random_split);

                        let pivot_v_1 = row_u + height_u_1 / 2.0;
                        let pivot_v_2 = (row_u + brick_height / max_span) - height_u_2 / 2.0;

                        let width_u = next_u - this_u;
                        let width_ws = width_u * wall_length;

                        for (height, pivot_v) in [(height_u_1, pivot_v_1), (height_u_2, pivot_v_2)]
                        {
                            let brick_center_y = top_y - pivot_v * max_span;
                            let curve_pos = curve.get_pos_at_u(pivot_u);
                            let ground_y = get_ground_y(curve_pos);

                            // Skip if entirely below ground
                            let half_height = (height * max_span) / 2.0;
                            if brick_center_y + half_height < ground_y {
                                continue;
                            }

                            brick_row.push(Brick {
                                pivot_uv: Vec2::new(pivot_u, pivot_v),
                                bounds_uv: Vec2::new(width_u, height),
                                transform: Transform {
                                    translation: Vec3::new(pivot_u * wall_length, 0.0, 0.0),
                                    rotation: Quat::IDENTITY,
                                    scale: Vec3::new(width_ws, height * max_span, brick_depth),
                                },
                            });
                        }
                    } else {
                        let pivot_u = ((next_u + this_u) / 2.0).clamp(0.0, 1.0);
                        let width_u = next_u - this_u;
                        let width_ws = width_u * wall_length;
                        let pivot_v = row_u + brick_height / max_span / 2.0;

                        let brick_center_y = top_y - pivot_v * max_span;
                        let curve_pos = curve.get_pos_at_u(pivot_u);
                        let ground_y = get_ground_y(curve_pos);

                        // Skip if entirely below ground
                        let half_height = brick_height / 2.0;
                        if brick_center_y + half_height < ground_y {
                            continue;
                        }

                        brick_row.push(Brick {
                            pivot_uv: Vec2::new(pivot_u, pivot_v),
                            bounds_uv: Vec2::new(width_u, brick_height / max_span),
                            transform: Transform {
                                scale: Vec3::new(width_ws, brick_height, brick_depth),
                                translation: Vec3::new(pivot_u * wall_length, 0.0, 0.0),
                                rotation: Quat::IDENTITY,
                            },
                        });
                    }
                }
            }

            // Transform bricks into world space
            for brick in &mut brick_row {
                let curve_pos = curve.get_pos_at_u(brick.pivot_uv.x);
                brick.transform.translation = curve_pos;
                // Calculate absolute vertical translation
                brick.transform.translation.y = top_y - brick.pivot_uv.y * max_span;

                let curve_tangent = curve.get_tangent_at_u(brick.pivot_uv.x).normalize_or_zero();
                let up = Vec3::Y;
                let normal = curve_tangent.cross(up).normalize_or_zero();
                brick.transform.rotation =
                    Quat::from_mat3(&Mat3::from_cols(curve_tangent, up, normal));
            }

            bricks.append(&mut brick_row);
        }

        bricks
    }
}

/// Generate random splits in [0;1] range with variance perturbation.
fn random_splits(splits: usize, variance_u: f32, rng: &mut Rng) -> Vec<f32> {
    let row_u: Vec<f32> = (0..(splits + 1))
        .map(|i| (i as f32) / (splits as f32))
        .collect();

    row_u
        .iter()
        .enumerate()
        .map(|(i, u)| {
            if i != 0 && i != row_u.len() - 1 {
                (u + (rng.f32() - 0.5) * variance_u).clamp(0.0, 1.0)
            } else {
                *u
            }
        })
        .collect()
}

/// Perturbs the staggered split points along a row, writing into an output buffer to avoid allocations.
fn perturb_splits_into(splits: &[f32], variance_u: f32, rng: &mut Rng, out: &mut Vec<f32>) {
    out.reserve(splits.len());
    for (i, &u) in splits.iter().enumerate() {
        if i != 0 && i != splits.len() - 1 {
            out.push((u + (rng.f32() - 0.5) * variance_u).clamp(0.0, 1.0));
        } else {
            out.push(u);
        }
    }
}

/// A chunk of wall bricks in a contiguous span along the curve (u-space).
#[allow(dead_code)]
pub struct BrickSegment {
    pub u_start: f32,
    pub u_end: f32,
    pub bricks: Vec<Brick>,
}

/// Axis-aligned bounding box for a wall segment.
#[allow(dead_code)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl BrickSegment {
    /// Compute a conservative AABB in world space that contains all bricks in this segment.
    #[allow(dead_code)]
    pub fn compute_aabb(&self) -> Option<Aabb> {
        if self.bricks.is_empty() {
            return None;
        }
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for b in &self.bricks {
            let t = b.transform.translation;
            let rot = b.transform.rotation;
            let half = (b.transform.scale.abs()) * 0.5;
            let right = (rot * Vec3::X).abs();
            let up = (rot * Vec3::Y).abs();
            let forward = (rot * Vec3::Z).abs();
            let ext = right * half.x + up * half.y + forward * half.z;
            let bmin = t - ext;
            let bmax = t + ext;
            min.x = min.x.min(bmin.x);
            min.y = min.y.min(bmin.y);
            min.z = min.z.min(bmin.z);
            max.x = max.x.max(bmax.x);
            max.y = max.y.max(bmax.y);
            max.z = max.z.max(bmax.z);
        }
        Some(Aabb { min, max })
    }
}

impl WallConstructor {
    /// Construct wall bricks and group them into segments for culling/spatial batching.
    /// `segment_length` is measured in world units along the curve.
    #[allow(dead_code)]
    pub fn from_curve_chunked(
        curve: &Curve,
        wall_height: f32,
        get_ground_y: impl Fn(Vec3) -> f32,
        segment_length: f32,
    ) -> Vec<BrickSegment> {
        let bricks = Self::from_curve(curve, wall_height, get_ground_y);
        let wall_length = curve.length.max(0.0001);
        let segments_count = ((wall_length / segment_length).ceil() as usize).max(1);

        let mut segments: Vec<BrickSegment> = Vec::with_capacity(segments_count);
        for i in 0..segments_count {
            let u_start = ((i as f32) * segment_length / wall_length).clamp(0.0, 1.0);
            let u_end = (((i + 1) as f32) * segment_length / wall_length).min(1.0);
            segments.push(BrickSegment {
                u_start,
                u_end,
                bricks: Vec::new(),
            });
        }

        for brick in bricks {
            let u = brick.pivot_uv.x.clamp(0.0, 1.0);
            let mut idx = ((u * wall_length) / segment_length).floor() as usize;
            if idx >= segments_count {
                idx = segments_count - 1;
            }
            segments[idx].bricks.push(brick);
        }

        segments
    }
}
