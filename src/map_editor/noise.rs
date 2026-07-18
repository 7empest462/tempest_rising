pub struct PerlinNoise {
    permutation: [u8; 512],
}

impl PerlinNoise {
    pub fn new(seed: u32) -> Self {
        // Initialize permutation table 0..255
        let mut perm: [u8; 256] = std::array::from_fn(|i| i as u8);

        // Shuffle deterministically based on seed
        let mut s = seed;
        let mut next_random = move || {
            s = s.wrapping_mul(1103515245).wrapping_add(12345);
            s
        };

        for i in (1..256).rev() {
            let j = (next_random() as usize) % (i + 1);
            perm.swap(i, j);
        }

        // Double the permutation table
        let mut permutation = [0u8; 512];
        permutation[0..256].copy_from_slice(&perm);
        permutation[256..512].copy_from_slice(&perm);

        Self { permutation }
    }

    fn fade(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    fn lerp(t: f32, a: f32, b: f32) -> f32 {
        a + t * (b - a)
    }

    fn grad(hash: u8, x: f32, y: f32) -> f32 {
        let h = hash & 7;
        let u = if h < 4 { x } else { y };
        let v = if h < 4 { y } else { x };
        let g_u = if (h & 1) != 0 { -u } else { u };
        let g_v = if (h & 2) != 0 { -v } else { v };
        g_u + g_v
    }

    pub fn noise(&self, x: f32, y: f32) -> f32 {
        let ix = x.floor() as i32 & 255;
        let iy = y.floor() as i32 & 255;

        let fx = x - x.floor();
        let fy = y - y.floor();

        let u = Self::fade(fx);
        let v = Self::fade(fy);

        let aa = self.permutation[self.permutation[ix as usize] as usize + iy as usize];
        let ab = self.permutation[self.permutation[ix as usize] as usize + iy as usize + 1];
        let ba = self.permutation[self.permutation[(ix + 1) as usize] as usize + iy as usize];
        let bb = self.permutation[self.permutation[(ix + 1) as usize] as usize + iy as usize + 1];

        let x1 = Self::lerp(u, Self::grad(aa, fx, fy), Self::grad(ba, fx - 1.0, fy));
        let x2 = Self::lerp(
            u,
            Self::grad(ab, fx, fy - 1.0),
            Self::grad(bb, fx - 1.0, fy - 1.0),
        );

        Self::lerp(v, x1, x2)
    }

    pub fn fbm(&self, x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        for _ in 0..octaves {
            total += self.noise(x * frequency, y * frequency) * amplitude;
            max_value += amplitude;
            amplitude *= gain;
            frequency *= lacunarity;
        }

        total / max_value
    }
}
