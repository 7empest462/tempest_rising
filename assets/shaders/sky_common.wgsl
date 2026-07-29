// Shared sky gradient function used by water reflections
fn get_sky_color(direction: vec3<f32>) -> vec3<f32> {
    let gradient_pos = clamp((direction.y + 0.6) / 1.8, 0.0, 1.0);
    
    // Base daytime palette
    let bottom_color = vec3<f32>(0.58, 0.74, 0.88);
    let horizon_color = vec3<f32>(0.42, 0.68, 0.92);
    let mid_sky_color = vec3<f32>(0.18, 0.48, 0.85);
    let top_color = vec3<f32>(0.06, 0.22, 0.65);

    var color: vec3<f32>;
    
    if gradient_pos < 0.35 {
        color = mix(bottom_color, horizon_color, smoothstep(0.0, 1.0, gradient_pos / 0.35));
    } else if gradient_pos < 0.75 {
        let t = (gradient_pos - 0.35) / 0.4;
        color = mix(horizon_color, mid_sky_color, smoothstep(0.0, 1.0, t));
    } else {
        let t = (gradient_pos - 0.75) / 0.25;
        color = mix(mid_sky_color, top_color, smoothstep(0.0, 1.0, t));
    }

    // Optional: Add very subtle horizon glow
    color += vec3<f32>(0.08, 0.12, 0.18) * pow(max(0.0, 1.0 - abs(direction.y)), 4.0) * 0.6;

    return color;
}
