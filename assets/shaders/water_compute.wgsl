// Water physics compute shader
// Simulates shallow water equations on GPU for real-time wave propagation

// Grid dimensions (must match Rust code)
const GRID_SIZE: u32 = 256u;

struct WaterInteractorData {
    grid_x: f32,
    grid_z: f32,
    push_force: f32,
    push_radius: f32,
    swim_add_height: f32,
    swim_radius: f32,
    _pad1: f32,
    _pad2: f32,
};

struct WaterImpulseData {
    grid_x: f32,
    grid_z: f32,
    force: f32,
    radius: f32,
};

struct SimParams {
    delta_time: f32,
    gravity: f32,
    friction: f32,
    interactor_count: u32,
    interactors: array<WaterInteractorData, 16>,
    impulse_count: u32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
    impulses: array<WaterImpulseData, 8>,
};

@group(0) @binding(0) var<uniform> params: SimParams;

// Storage buffers for simulation data
@group(0) @binding(1) var<storage, read_write> height_current: array<f32>;
@group(0) @binding(2) var<storage, read_write> height_next: array<f32>;
@group(0) @binding(3) var<storage, read_write> flow_x: array<f32>;
@group(0) @binding(4) var<storage, read_write> flow_y: array<f32>;
@group(0) @binding(5) var<storage, read_write> flow_x_next: array<f32>;
@group(0) @binding(6) var<storage, read_write> flow_y_next: array<f32>;
@group(0) @binding(7) var<storage, read> wall_mask: array<u32>; // Packed bits for faster access

// Helper function to convert 2D grid coordinates to linear index
fn grid_idx(x: u32, y: u32) -> u32 {
    return x * GRID_SIZE + y;
}

// Helper function to unpack wall bit
fn is_wall(x: u32, y: u32) -> bool {
    let idx = grid_idx(x, y);
    let packed_idx = idx / 32u;
    let bit_idx = idx % 32u;
    return ((wall_mask[packed_idx] >> bit_idx) & 1u) != 0u;
}

// Workgroup size: 8x8 = 64 threads
@compute @workgroup_size(8, 8, 1)
fn water_flow_pass(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= GRID_SIZE || y >= GRID_SIZE) {
        return;
    }

    let idx = grid_idx(x, y);
    
    // Clear border flows
    if (x == 0u || x == GRID_SIZE - 1u || y == 0u || y == GRID_SIZE - 1u) {
        flow_x_next[idx] = 0.0;
        flow_y_next[idx] = 0.0;
        return;
    }

    var flow_x_val = flow_x[idx];
    var flow_y_val = flow_y[idx];
    
    // Calculate X-direction flow
    if (x > 0u) {
        let source_has_wall = is_wall(x - 1u, y);
        let dest_has_wall = is_wall(x, y);
        let height_diff = height_current[grid_idx(x - 1u, y)] - height_current[idx];

        if (!source_has_wall && !dest_has_wall) {
            let friction_factor = pow(params.friction, params.delta_time);
            flow_x_val = flow_x_val * friction_factor + height_diff * params.gravity * params.delta_time;
        } else {
            flow_x_val = 0.0;
        }
    } else {
        flow_x_val = 0.0;
    }

    // Calculate Y-direction flow
    if (y > 0u) {
        let source_has_wall = is_wall(x, y - 1u);
        let dest_has_wall = is_wall(x, y);
        let height_diff = height_current[grid_idx(x, y - 1u)] - height_current[idx];

        if (!source_has_wall && !dest_has_wall) {
            let friction_factor = pow(params.friction, params.delta_time);
            flow_y_val = flow_y_val * friction_factor + height_diff * params.gravity * params.delta_time;
        } else {
            flow_y_val = 0.0;
        }
    } else {
        flow_y_val = 0.0;
    }
    // Apply impulses (like block breaks) outward
    for (var i = 0u; i < params.impulse_count; i++) {
        let impulse = params.impulses[i];
        if (impulse.force != 0.0) {
            let dx = f32(x) - impulse.grid_x;
            let dy = f32(y) - impulse.grid_z; // Z is Y in 2D grid
            let dist = sqrt(dx * dx + dy * dy);
            
            if (dist > 0.1 && dist < impulse.radius) {
                let weight = max(0.0, 1.0 - (dist / impulse.radius));
                let push_strength = impulse.force * weight * params.delta_time;
                flow_x_val += (dx / dist) * push_strength;
                flow_y_val += (dy / dist) * push_strength;
            }
        }
    }

    // Apply all interactors (animals, players)
    for (var i = 0u; i < params.interactor_count; i++) {
        let interactor = params.interactors[i];
        
        let dx = f32(x) - interactor.grid_x;
        let dy = f32(y) - interactor.grid_z;
        let dist = sqrt(dx * dx + dy * dy);
        let max_dist = max(dist, 0.01);
        let dir_x = dx / max_dist;
        let dir_y = dy / max_dist;

        if (interactor.push_force != 0.0 && dist < interactor.push_radius) {
            let weight = max(0.0, 1.0 - (dist / interactor.push_radius));
            let push_strength = interactor.push_force * weight * params.delta_time;
            flow_x_val += dir_x * push_strength;
            flow_y_val += dir_y * push_strength;
        }

        if (interactor.swim_add_height != 0.0 && dist < interactor.swim_radius) {
            let weight = max(0.0, 1.0 - (dist / interactor.swim_radius));
            let swim_push = interactor.swim_add_height * weight * 4.5; 
            flow_x_val += dir_x * swim_push;
            flow_y_val += dir_y * swim_push;
        }
    }
    
    flow_x_next[idx] = flow_x_val;
    flow_y_next[idx] = flow_y_val;
}

@compute @workgroup_size(8, 8, 1)
fn water_outflow_pass(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= GRID_SIZE || y >= GRID_SIZE) {
        return;
    }

    let idx = grid_idx(x, y);
    
    if (is_wall(x, y)) {
        return;
    }

    var outflow_x = flow_x_next[idx];
    var outflow_y = flow_y_next[idx];
    var total_outflow = 0.0;
    
    total_outflow += max(0.0, -outflow_x);
    total_outflow += max(0.0, -outflow_y);

    if (x < GRID_SIZE - 1u) {
        total_outflow += max(0.0, flow_x_next[grid_idx(x + 1u, y)]);
    }
    if (y < GRID_SIZE - 1u) {
        total_outflow += max(0.0, flow_y_next[grid_idx(x, y + 1u)]);
    }

    // Apply exact CPU scaling equation
    let max_outflow = height_current[idx] / params.delta_time;

    if (total_outflow > 0.0) {
        let scale = min(1.0, max_outflow / total_outflow);
        
        if (outflow_x < 0.0) {
            flow_x_next[idx] *= scale;
        }
        if (outflow_y < 0.0) {
            flow_y_next[idx] *= scale;
        }
        if (x < GRID_SIZE - 1u && flow_x_next[grid_idx(x + 1u, y)] > 0.0) {
            flow_x_next[grid_idx(x + 1u, y)] *= scale;
        }
        if (y < GRID_SIZE - 1u && flow_y_next[grid_idx(x, y + 1u)] > 0.0) {
            flow_y_next[grid_idx(x, y + 1u)] *= scale;
        }
    }
}

@compute @workgroup_size(8, 8, 1)
fn water_height_pass(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= GRID_SIZE || y >= GRID_SIZE) {
        return;
    }

    let idx = grid_idx(x, y);
    var new_height = height_current[idx];
    
    if (is_wall(x, y)) {
        height_next[idx] = 1.0;
        return;
    }

    var height_change = 0.0;

    let can_receive_from_left = x > 0u && !is_wall(x - 1u, y);
    if (can_receive_from_left) {
        height_change += flow_x_next[idx];
    }

    let can_receive_from_top = y > 0u && !is_wall(x, y - 1u);
    if (can_receive_from_top) {
        height_change += flow_y_next[idx];
    }

    let can_flow_right = x < GRID_SIZE - 1u && !is_wall(x + 1u, y);
    if (can_flow_right) {
        height_change -= flow_x_next[grid_idx(x + 1u, y)];
    }

    let can_flow_bottom = y < GRID_SIZE - 1u && !is_wall(x, y + 1u);
    if (can_flow_bottom) {
        height_change -= flow_y_next[grid_idx(x, y + 1u)];
    }

    new_height += height_change * params.delta_time;
    new_height = max(new_height, 0.1);

    // === Apply impulses ===
    for (var i = 0u; i < params.impulse_count; i++) {
        let impulse = params.impulses[i];
        if (impulse.force != 0.0) {
            let dx = f32(x) - impulse.grid_x;
            let dy = f32(y) - impulse.grid_z;
            let dist = sqrt(dx * dx + dy * dy);
            
            if (dist < impulse.radius) {
                if (impulse.force < 0.0) {
                    new_height = max(0.2, new_height + impulse.force * 0.1);
                } else {
                    new_height += impulse.force * 0.1;
                }
            }
        }
    }

    // === Interactor depression (animals/players) ===
    for (var i = 0u; i < params.interactor_count; i++) {
        let interactor = params.interactors[i];
        if (interactor.push_force != 0.0) {
            let dx = f32(x) - interactor.grid_x;
            let dy = f32(y) - interactor.grid_z;
            let dist = sqrt(dx * dx + dy * dy);
            
            if (dist < interactor.push_radius) {
                let weight = max(0.0, 1.0 - (dist / interactor.push_radius));
                let displacement = (interactor.push_force / 25.0) * 0.45 * weight * params.delta_time;
                new_height = max(0.2, new_height - displacement);
            }
        }
    }

    // Gentle damping + clamping
    new_height = mix(new_height, 1.0, 0.016 * params.delta_time * 60.0);
    new_height = clamp(new_height, 0.05, 4.5);

    height_next[idx] = new_height;
}
