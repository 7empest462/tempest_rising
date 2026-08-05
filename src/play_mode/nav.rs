//! Creature navigation: hybrid ray steering + fallback A* pathfinding.
//!
//! **Ray steering** runs every frame and handles 90% of obstacle avoidance.
//! **A\*** only fires when a creature is stuck (hasn't progressed in ~2 seconds),
//! building a small local grid on-demand from live `WallCollider` data.

use bevy::prelude::*;

use crate::map_editor::data::TempestMap;

// ─── Ray Steering ────────────────────────────────────────────

/// Result of a single ray probe against wall colliders.
struct RayProbe {
    direction: Vec3,
    clear_distance: f32, // how far we can go before hitting a wall
}

/// Casts a ray from `origin` in `dir` (XZ only) and returns the distance
/// to the nearest `WallCollider` AABB intersection, or `max_dist` if clear.
fn probe_ray(
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    creature_radius: f32,
    colliders: &[(Vec3, Vec3, bool)], // (center, half_extents, is_open_door)
) -> f32 {
    let mut closest = max_dist;
    let step = creature_radius * 0.8; // step size for ray marching
    let steps = (max_dist / step).ceil() as usize;

    for i in 1..=steps {
        let t = (i as f32) * step;
        if t > closest {
            break;
        }
        let sample = Vec3::new(origin.x + dir.x * t, origin.y, origin.z + dir.z * t);

        for &(center, extents, is_open) in colliders {
            if is_open {
                continue;
            }
            // Inflate the AABB by creature_radius for Minkowski-sum style check
            let min_x = center.x - extents.x - creature_radius;
            let max_x = center.x + extents.x + creature_radius;
            let min_z = center.z - extents.z - creature_radius;
            let max_z = center.z + extents.z + creature_radius;

            // Y overlap check (creature should be at roughly the same height as wall)
            let min_y = center.y - extents.y;
            let max_y = center.y + extents.y;
            if origin.y > max_y + 1.0 || origin.y < min_y - 1.0 {
                continue;
            }

            if sample.x >= min_x && sample.x <= max_x && sample.z >= min_z && sample.z <= max_z {
                closest = closest.min(t - step); // back up one step
                break;
            }
        }
    }

    closest.max(0.0)
}

/// Primary steering function. Takes a desired movement direction and returns
/// a steered direction that avoids nearby wall colliders.
///
/// Casts 5 rays in a fan pattern ahead of the creature:
/// - Center (0°)
/// - Left/Right (±30°)
/// - Wide Left/Right (±60°)
///
/// Returns a velocity vector with the same speed but adjusted direction.
pub fn steer_around_obstacles(
    origin: Vec3,
    desired_dir: Vec3,
    speed: f32,
    creature_radius: f32,
    colliders: &[(Vec3, Vec3, bool)], // pre-collected (center, half_extents, is_open_door)
    map: &TempestMap,
    water_level: f32,
) -> Vec3 {
    let dir_2d = Vec3::new(desired_dir.x, 0.0, desired_dir.z);
    if dir_2d.length_squared() < 0.0001 {
        return Vec3::ZERO;
    }
    let forward = dir_2d.normalize();
    let probe_dist = (speed * 0.8).clamp(2.0, 6.0); // look-ahead distance

    // Generate fan rays by rotating the forward vector around Y
    let angles = [0.0_f32, 30.0, -30.0, 60.0, -60.0];
    let mut probes: Vec<RayProbe> = Vec::with_capacity(5);

    for &angle_deg in &angles {
        let angle_rad = angle_deg.to_radians();
        let rot = Quat::from_rotation_y(angle_rad);
        let ray_dir = rot * forward;
        let clear = probe_ray(origin, ray_dir, probe_dist, creature_radius, colliders);

        // Also check for deep water ahead
        let sample_pos = origin + ray_dir * clear.min(probe_dist * 0.5);
        let terrain_h = crate::play_mode::get_bilinear_height(sample_pos.x, sample_pos.z, map);
        let ground_h = crate::play_mode::get_effective_floor_height(sample_pos, terrain_h);
        let water_depth = (water_level - ground_h).max(0.0);

        // Penalize directions that lead into deep water (unless creature is already in water)
        let origin_terrain = crate::play_mode::get_bilinear_height(origin.x, origin.z, map);
        let origin_ground = crate::play_mode::get_effective_floor_height(origin, origin_terrain);
        let origin_water = (water_level - origin_ground).max(0.0);

        let effective_clear = if water_depth > 1.2 && origin_water < 0.8 {
            clear.min(0.5) // heavily penalize entering deep water from land
        } else {
            clear
        };

        probes.push(RayProbe {
            direction: Vec3::new(ray_dir.x, 0.0, ray_dir.z).normalize(),
            clear_distance: effective_clear,
        });
    }

    // If center ray is mostly clear, use the desired direction (no steering needed)
    if probes[0].clear_distance > probe_dist * 0.7 {
        return forward * speed;
    }

    // Center is blocked — find the best alternative ray
    let mut best_idx = 0;
    let mut best_clear = 0.0_f32;
    for (i, probe) in probes.iter().enumerate() {
        if probe.clear_distance > best_clear {
            best_clear = probe.clear_distance;
            best_idx = i;
        }
    }

    // If even the best ray is very short, slow down to avoid ramming
    let speed_factor = (best_clear / probe_dist).clamp(0.3, 1.0);

    // Blend the best ray with the forward direction for smoother steering
    let blend = if best_clear > probe_dist * 0.5 {
        0.6
    } else {
        0.9
    };
    let steered = Vec3::lerp(forward, probes[best_idx].direction, blend).normalize_or_zero();

    steered * speed * speed_factor
}

// ─── Stuck Detection ─────────────────────────────────────────

/// Returns true if the creature has been stuck (making no progress toward its
/// goal) for longer than `threshold_secs`.
pub fn is_creature_stuck(
    current_pos: Vec3,
    last_progress_pos: Vec3,
    stuck_timer: f32,
    threshold_secs: f32,
) -> bool {
    let progress = Vec2::new(
        current_pos.x - last_progress_pos.x,
        current_pos.z - last_progress_pos.z,
    )
    .length();

    // Has moved less than 0.5m in the accumulated stuck time
    progress < 0.5 && stuck_timer >= threshold_secs
}

/// Update the stuck timer. Returns (new_stuck_timer, new_last_progress_pos).
/// Call this every frame for chasing creatures.
pub fn update_stuck_tracking(
    current_pos: Vec3,
    last_progress_pos: Vec3,
    stuck_timer: f32,
    dt: f32,
) -> (f32, Vec3) {
    let progress = Vec2::new(
        current_pos.x - last_progress_pos.x,
        current_pos.z - last_progress_pos.z,
    )
    .length();

    if progress > 1.0 {
        // Made good progress — reset
        (0.0, current_pos)
    } else {
        // Still near the same spot
        (stuck_timer + dt, last_progress_pos)
    }
}

// ─── A* Pathfinding (Fallback) ───────────────────────────────

/// A* grid cell
#[derive(Clone, Copy)]
struct Cell {
    cost: f32,      // g-cost (distance from start)
    heuristic: f32, // f-cost (g + h)
    parent: Option<(usize, usize)>,
    closed: bool,
    blocked: bool,
}

/// Finds a path from `start` to `goal` using A* over a local grid.
/// The grid is built on-demand around the creature's position.
///
/// Returns a `Vec<Vec3>` of world-space waypoints, or empty if no path found.
pub fn find_path_astar(
    start: Vec3,
    goal: Vec3,
    creature_radius: f32,
    colliders: &[(Vec3, Vec3, bool)],
    map: &TempestMap,
    water_level: f32,
) -> Vec<Vec3> {
    let cell_size = 1.0_f32; // 1 meter per cell
    let grid_radius = 20; // 20 cells in each direction = 40×40 grid
    let grid_size = grid_radius * 2 + 1;

    // Grid origin (center of the local grid in world space)
    let center_x = (start.x + goal.x) * 0.5;
    let center_z = (start.z + goal.z) * 0.5;
    let origin_x = center_x - grid_radius as f32 * cell_size;
    let origin_z = center_z - grid_radius as f32 * cell_size;

    // Build the grid
    let total_cells = grid_size * grid_size;
    let mut grid = vec![
        Cell {
            cost: f32::MAX,
            heuristic: f32::MAX,
            parent: None,
            closed: false,
            blocked: false,
        };
        total_cells
    ];

    // Mark blocked cells (walls + deep water + steep terrain)
    for gz in 0..grid_size {
        for gx in 0..grid_size {
            let world_x = origin_x + gx as f32 * cell_size + cell_size * 0.5;
            let world_z = origin_z + gz as f32 * cell_size + cell_size * 0.5;
            let idx = gz * grid_size + gx;

            // Check wall colliders
            for &(center, extents, is_open) in colliders {
                if is_open {
                    continue;
                }
                let min_x = center.x - extents.x - creature_radius;
                let max_x = center.x + extents.x + creature_radius;
                let min_z = center.z - extents.z - creature_radius;
                let max_z = center.z + extents.z + creature_radius;

                if world_x >= min_x && world_x <= max_x && world_z >= min_z && world_z <= max_z {
                    grid[idx].blocked = true;
                    break;
                }
            }

            // Check deep water
            if !grid[idx].blocked {
                let hw = map.width as f32 / 2.0;
                let hh = map.height as f32 / 2.0;
                let map_x = ((world_x + hw) as u32).min(map.width.saturating_sub(1));
                let map_z = ((world_z + hh) as u32).min(map.height.saturating_sub(1));
                let terrain_h = map.get_height(map_x, map_z);
                if water_level - terrain_h > 1.5 {
                    grid[idx].blocked = true;
                }
            }
        }
    }

    // Convert world positions to grid coordinates
    let world_to_grid = |wx: f32, wz: f32| -> (usize, usize) {
        let gx = ((wx - origin_x) / cell_size).floor() as isize;
        let gz = ((wz - origin_z) / cell_size).floor() as isize;
        (
            gx.clamp(0, grid_size as isize - 1) as usize,
            gz.clamp(0, grid_size as isize - 1) as usize,
        )
    };

    let (start_gx, start_gz) = world_to_grid(start.x, start.z);
    let (goal_gx, goal_gz) = world_to_grid(goal.x, goal.z);

    let start_idx = start_gz * grid_size + start_gx;
    grid[start_idx].cost = 0.0;
    grid[start_idx].heuristic = heuristic(start_gx, start_gz, goal_gx, goal_gz);

    // Open list: (f_cost, gx, gz)
    let mut open: Vec<(f32, usize, usize)> = vec![(grid[start_idx].heuristic, start_gx, start_gz)];

    let neighbors: [(isize, isize); 8] = [
        (-1, 0),
        (1, 0),
        (0, -1),
        (0, 1),
        (-1, -1),
        (-1, 1),
        (1, -1),
        (1, 1),
    ];

    let mut found = false;
    let max_iterations = 2000;
    let mut iterations = 0;

    while let Some(pos) = pop_min(&mut open) {
        let (_, cx, cz) = pos;
        let cidx = cz * grid_size + cx;

        if grid[cidx].closed {
            continue;
        }
        grid[cidx].closed = true;

        if cx == goal_gx && cz == goal_gz {
            found = true;
            break;
        }

        iterations += 1;
        if iterations > max_iterations {
            break;
        }

        for &(dx, dz) in &neighbors {
            let nx = cx as isize + dx;
            let nz = cz as isize + dz;

            if nx < 0 || nz < 0 || nx >= grid_size as isize || nz >= grid_size as isize {
                continue;
            }

            let nx = nx as usize;
            let nz = nz as usize;
            let nidx = nz * grid_size + nx;

            if grid[nidx].closed || grid[nidx].blocked {
                continue;
            }

            let move_cost = if dx.abs() + dz.abs() == 2 { 1.414 } else { 1.0 };
            let new_cost = grid[cidx].cost + move_cost;

            if new_cost < grid[nidx].cost {
                grid[nidx].cost = new_cost;
                grid[nidx].heuristic = new_cost + heuristic(nx, nz, goal_gx, goal_gz);
                grid[nidx].parent = Some((cx, cz));
                open.push((grid[nidx].heuristic, nx, nz));
            }
        }
    }

    if !found {
        return Vec::new();
    }

    // Backtrack to build waypoint path
    let mut path_cells = Vec::new();
    let mut cur = (goal_gx, goal_gz);
    while cur != (start_gx, start_gz) {
        path_cells.push(cur);
        let idx = cur.1 * grid_size + cur.0;
        match grid[idx].parent {
            Some(p) => cur = p,
            None => break,
        }
    }
    path_cells.reverse();

    // Convert grid cells back to world positions
    let path: Vec<Vec3> = path_cells
        .iter()
        .map(|&(gx, gz)| {
            let wx = origin_x + gx as f32 * cell_size + cell_size * 0.5;
            let wz = origin_z + gz as f32 * cell_size + cell_size * 0.5;
            Vec3::new(wx, start.y, wz)
        })
        .collect();

    // Simplify: skip waypoints that have clear line-of-sight to a later one
    simplify_path(path, creature_radius, colliders)
}

/// Octile distance heuristic for A*
fn heuristic(ax: usize, az: usize, bx: usize, bz: usize) -> f32 {
    let dx = (ax as f32 - bx as f32).abs();
    let dz = (az as f32 - bz as f32).abs();
    let (min, max) = if dx < dz { (dx, dz) } else { (dz, dx) };
    max + min * 0.414 // octile distance
}

/// Pop the element with the smallest f-cost from the open list.
fn pop_min(open: &mut Vec<(f32, usize, usize)>) -> Option<(f32, usize, usize)> {
    if open.is_empty() {
        return None;
    }
    let mut min_idx = 0;
    let mut min_cost = open[0].0;
    for (i, &(cost, _, _)) in open.iter().enumerate().skip(1) {
        if cost < min_cost {
            min_cost = cost;
            min_idx = i;
        }
    }
    Some(open.swap_remove(min_idx))
}

/// Removes redundant waypoints by checking line-of-sight between non-adjacent points.
fn simplify_path(
    path: Vec<Vec3>,
    creature_radius: f32,
    colliders: &[(Vec3, Vec3, bool)],
) -> Vec<Vec3> {
    if path.len() <= 2 {
        return path;
    }

    let mut simplified = vec![path[0]];
    let mut current = 0;

    while current < path.len() - 1 {
        let mut furthest = current + 1;
        for i in (current + 2)..path.len() {
            if has_clear_line(path[current], path[i], creature_radius, colliders) {
                furthest = i;
            }
        }
        simplified.push(path[furthest]);
        current = furthest;
    }

    simplified
}

/// Checks if a straight line between two points is clear of wall colliders.
pub fn has_clear_line(
    from: Vec3,
    to: Vec3,
    creature_radius: f32,
    colliders: &[(Vec3, Vec3, bool)],
) -> bool {
    let delta = to - from;
    let dist = Vec2::new(delta.x, delta.z).length();
    let step = creature_radius * 0.6;
    let steps = (dist / step).ceil() as usize;
    let dir = Vec3::new(delta.x, 0.0, delta.z).normalize_or_zero();

    for i in 1..=steps {
        let t = (i as f32) * step;
        let sample = Vec3::new(from.x + dir.x * t, from.y, from.z + dir.z * t);

        for &(center, extents, is_open) in colliders {
            if is_open {
                continue;
            }
            let min_x = center.x - extents.x - creature_radius;
            let max_x = center.x + extents.x + creature_radius;
            let min_z = center.z - extents.z - creature_radius;
            let max_z = center.z + extents.z + creature_radius;

            if sample.x >= min_x && sample.x <= max_x && sample.z >= min_z && sample.z <= max_z {
                return false;
            }
        }
    }

    true
}

// ─── High-Level Navigation Helper ────────────────────────────

/// All-in-one navigation function for a chasing creature.
///
/// Call this instead of raw `velocity = dir * speed` in chase/attack branches.
/// It handles:
/// 1. Following an existing A* path (if one exists)
/// 2. Stuck detection → triggers A* repathing
/// 3. Ray steering for moment-to-moment avoidance
///
/// Returns `(steered_velocity, updated_nav_path, updated_nav_index, updated_stuck_timer, updated_last_pos)`
#[allow(clippy::too_many_arguments)]
pub fn navigate_toward(
    creature_pos: Vec3,
    target_pos: Vec3,
    speed: f32,
    creature_radius: f32,
    nav_path: &[Vec3],
    nav_path_index: usize,
    stuck_timer: f32,
    last_progress_pos: Vec3,
    dt: f32,
    colliders: &[(Vec3, Vec3, bool)],
    map: &TempestMap,
    water_level: f32,
) -> (Vec3, Vec<Vec3>, usize, f32, Vec3) {
    // Update stuck tracking
    let (new_stuck, new_last) =
        update_stuck_tracking(creature_pos, last_progress_pos, stuck_timer, dt);

    // If following a path, navigate to next waypoint
    if !nav_path.is_empty() && nav_path_index < nav_path.len() {
        let waypoint = nav_path[nav_path_index];
        let to_wp = waypoint - creature_pos;
        let dist_to_wp = Vec2::new(to_wp.x, to_wp.z).length();

        if dist_to_wp < 1.5 {
            // Reached waypoint — advance to next
            let new_index = nav_path_index + 1;
            if new_index >= nav_path.len() {
                // Path complete — switch to direct pursuit
                let dir = (target_pos - creature_pos).normalize_or_zero();
                let vel = steer_around_obstacles(
                    creature_pos,
                    dir,
                    speed,
                    creature_radius,
                    colliders,
                    map,
                    water_level,
                );
                return (vel, Vec::new(), 0, 0.0, creature_pos);
            }
            // Continue to next waypoint
            let next_wp = nav_path[new_index];
            let dir = (next_wp - creature_pos).normalize_or_zero();
            let vel = steer_around_obstacles(
                creature_pos,
                dir,
                speed,
                creature_radius,
                colliders,
                map,
                water_level,
            );
            return (vel, nav_path.to_vec(), new_index, new_stuck, new_last);
        }

        // Still moving toward current waypoint
        let dir = Vec3::new(to_wp.x, 0.0, to_wp.z).normalize_or_zero();
        let vel = steer_around_obstacles(
            creature_pos,
            dir,
            speed,
            creature_radius,
            colliders,
            map,
            water_level,
        );
        return (vel, nav_path.to_vec(), nav_path_index, new_stuck, new_last);
    }

    // No path — use direct steering
    let dir = (target_pos - creature_pos).normalize_or_zero();
    let vel = steer_around_obstacles(
        creature_pos,
        dir,
        speed,
        creature_radius,
        colliders,
        map,
        water_level,
    );

    // Check if stuck — trigger A* repathing
    if is_creature_stuck(creature_pos, new_last, new_stuck, 2.0) {
        let path = find_path_astar(
            creature_pos,
            target_pos,
            creature_radius,
            colliders,
            map,
            water_level,
        );
        if !path.is_empty() {
            return (vel, path, 0, 0.0, creature_pos);
        }
    }

    (vel, Vec::new(), 0, new_stuck, new_last)
}
