//! Curve module providing a spline-like path representation for procedural geometry.
//!
//! This module implements a piecewise-linear curve that can store points, compute arc lengths,
//! and interpolate positions and tangents along the curve using normalized parameter values (0.0 to 1.0).
//! It includes smoothing and resampling operations for flexible geometry generation.

use bevy::prelude::Vec3;

#[derive(Clone)]
pub struct Curve {
    pub points: Vec<Vec3>,
    // cache u values upon creation
    pub points_u: Vec<f32>,
    pub length: f32,
}

impl Default for Curve {
    fn default() -> Self {
        Self::new()
    }
}

impl Curve {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            points_u: Vec::new(),
            length: 0.0,
        }
    }

    /// Incrementally append a point in O(n) with no clone or distance recompute.
    #[allow(dead_code)]
    pub fn add(&mut self, pt: Vec3) {
        if self.points.is_empty() {
            self.points.push(pt);
            self.points_u.push(0.0);
            self.length = 0.0;
            return;
        }

        let new_seg = (pt - *self.points.last().unwrap()).length();
        let old_length = self.length;
        self.length += new_seg;

        // Rescale existing u values to fit the new total arc length
        if self.length > f32::EPSILON {
            let scale = old_length / self.length;
            for u in self.points_u.iter_mut() {
                *u *= scale;
            }
        }

        self.points.push(pt);
        self.points_u
            .push(if self.length > f32::EPSILON { 1.0 } else { 0.0 });
    }

    /// Incrementally prepend a point with a single rescale pass (no clone or distance recompute).
    #[allow(dead_code)]
    pub fn add_to_front(&mut self, pt: Vec3) {
        if self.points.is_empty() {
            self.points.push(pt);
            self.points_u.push(0.0);
            self.length = 0.0;
            return;
        }

        let new_seg = (self.points[0] - pt).length();
        let old_length = self.length;
        self.length += new_seg;

        // Shift and rescale: old_u -> (old_u * old_length + new_seg) / new_length
        if self.length > f32::EPSILON {
            for u in self.points_u.iter_mut() {
                *u = (*u * old_length + new_seg) / self.length;
            }
        }

        self.points.insert(0, pt);
        self.points_u.insert(0, 0.0);
    }

    pub fn from(points: Vec<Vec3>) -> Self {
        if points.is_empty() {
            return Self::new();
        }

        let mut cumulative_lengths = Vec::with_capacity(points.len());
        let mut length = 0.0;
        cumulative_lengths.push(0.0);

        for segment in points.windows(2) {
            length += (segment[1] - segment[0]).length();
            cumulative_lengths.push(length);
        }

        let points_u = if length > f32::EPSILON {
            cumulative_lengths
                .iter()
                .map(|distance| distance / length)
                .collect()
        } else {
            vec![0.0; points.len()]
        };

        Self {
            points,
            points_u,
            length,
        }
    }

    pub fn smooth(mut self, smoothing_steps: usize) -> Self {
        if self.points.len() < 3 || smoothing_steps == 0 {
            return self;
        }

        for _ in 0..smoothing_steps {
            let mut current_iter_smooth = self.points.clone();
            for (i, smoothed_point) in current_iter_smooth
                .iter_mut()
                .enumerate()
                .take(self.points.len() - 1)
                .skip(1)
            {
                let current_pos = self.points[i];
                let avg = (self.points[i - 1] + self.points[i + 1]) * 0.5;
                *smoothed_point = current_pos + (avg - current_pos) * 0.5;
            }
            self.points = current_iter_smooth;
        }

        Curve::from(self.points)
    }

    /// Resample the curve at uniform arc-length intervals using an O(n+m) two-pointer walk.
    pub fn resample(self, segment_length: f32) -> Self {
        if self.points.len() < 2 || self.length <= f32::EPSILON || segment_length <= f32::EPSILON {
            return self;
        }

        if segment_length >= self.length {
            return Curve::from(vec![self.points[0], *self.points.last().unwrap()]);
        }

        let u_spacing = segment_length / self.length;
        let target_points = (1.0 / u_spacing).round() as usize;
        let target_u_spacing = 1.0 / (target_points as f32);

        // Two-pointer walk: advance segment cursor forward with samples (never restart from 0)
        let mut result = Vec::with_capacity(target_points + 1);
        let mut seg_start = 0usize;

        for i in 0..=target_points {
            let u = ((i as f32) * target_u_spacing).min(1.0);

            // Advance segment pointer forward (segments are sorted by u)
            while seg_start + 2 < self.points.len() && self.points_u[seg_start + 1] < u {
                seg_start += 1;
            }

            let idx1 = seg_start;
            let idx2 = (seg_start + 1).min(self.points.len() - 1);
            let u_range = (self.points_u[idx1], self.points_u[idx2]);
            let dir = self.points[idx2] - self.points[idx1];

            let mag = if (u_range.1 - u_range.0).abs() < f32::EPSILON {
                0.0
            } else {
                (u - u_range.0) / (u_range.1 - u_range.0)
            };

            result.push(self.points[idx1] + dir * mag);
        }

        Curve::from(result)
    }

    /// O(log n) binary search for the segment containing parameter u.
    fn get_curve_segment_from_u(&self, u: f32) -> (usize, usize) {
        let last = self.points.len() - 1;
        if u >= 1.0 {
            return (last - 1, last);
        }
        if u <= 0.0 {
            return (0, 1);
        }

        // Find the first index i (starting from 1) where points_u[i] >= u
        let mut lo = 1usize;
        let mut hi = last;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.points_u[mid] >= u {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        (lo - 1, lo)
    }

    pub fn get_pos_at_u(&self, u: f32) -> Vec3 {
        if self.points.is_empty() {
            return Vec3::ZERO;
        }
        if self.points.len() == 1 || self.length <= f32::EPSILON {
            return self.points[0];
        }

        let u = u.clamp(0.0, 1.0);
        let (idx1, idx2) = self.get_curve_segment_from_u(u);
        let dir = self.points[idx2] - self.points[idx1];
        let u_range = (self.points_u[idx1], self.points_u[idx2]);
        let u_span = u_range.1 - u_range.0;

        if u_span.abs() <= f32::EPSILON {
            return self.points[idx1];
        }

        self.points[idx1] + dir * ((u - u_range.0) / u_span)
    }

    pub fn get_tangent_at_u(&self, u: f32) -> Vec3 {
        if self.points.len() < 2 || self.length <= f32::EPSILON {
            return Vec3::X;
        }

        let u = u.clamp(0.0, 1.0);
        let (idx1, idx2) = self.get_curve_segment_from_u(u);
        let tangent = (self.points[idx2] - self.points[idx1]).normalize_or_zero();
        if tangent.length_squared() > f32::EPSILON {
            return tangent;
        }

        self.points
            .windows(2)
            .map(|segment| (segment[1] - segment[0]).normalize_or_zero())
            .find(|candidate| candidate.length_squared() > f32::EPSILON)
            .unwrap_or(Vec3::X)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_from_points() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];

        let curve = Curve::from(points);

        assert_eq!(curve.points.len(), 3);
        assert!(curve.length > 0.0);

        let expected_length = 2.0;
        assert!((curve.length - expected_length).abs() < 0.001);

        assert_eq!(curve.points_u[0], 0.0);
        assert!(curve.points_u[1] > 0.0 && curve.points_u[1] < 1.0);
        assert_eq!(curve.points_u[2], 1.0);
    }

    #[test]
    fn test_curve_smooth() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 1.0, 0.0),
        ];

        let curve = Curve::from(points.clone());
        let smoothed = curve.smooth(1);

        assert_eq!(smoothed.points.len(), 4);

        // First point is not smoothed
        assert_eq!(smoothed.points[0], points[0]);

        // Point 1: prev=(0,0,0), current=(1,1,0), next=(2,0,0)
        // avg = (1, 0, 0)
        // smoothed = (1,1,0) + ((1,0,0) - (1,1,0)) * 0.5 = (1, 0.5, 0)
        let expected_p1 = Vec3::new(1.0, 0.5, 0.0);
        assert!((smoothed.points[1] - expected_p1).length() < 0.001);

        // Point 2: prev=(1,1,0), current=(2,0,0), next=(3,1,0)
        // avg = (2, 1, 0)
        // smoothed = (2,0,0) + ((2,1,0) - (2,0,0)) * 0.5 = (2, 0.5, 0)
        let expected_p2 = Vec3::new(2.0, 0.5, 0.0);
        assert!((smoothed.points[2] - expected_p2).length() < 0.001);
    }

    #[test]
    fn test_curve_resample() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        ];

        let curve = Curve::from(points.clone());

        let resampled = curve.resample(1.0);

        assert!(resampled.points.len() > 2);

        assert_eq!(resampled.points[0], points[0]);
        assert_eq!(
            resampled.points[resampled.points.len() - 1],
            points[points.len() - 1]
        );

        for (i, pt) in resampled.points.iter().enumerate().skip(1) {
            assert!(resampled.points[i - 1].distance(*pt) <= 1.01);
        }
    }

    #[test]
    fn test_curve_get_pos_at_u() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];

        let curve = Curve::from(points.clone());

        let pos_at_0 = curve.get_pos_at_u(0.0);
        assert!((pos_at_0 - points[0]).length() < 0.001);

        let pos_at_1 = curve.get_pos_at_u(1.0);
        assert!((pos_at_1 - points[2]).length() < 0.001);

        let pos_at_half = curve.get_pos_at_u(0.5);
        assert!(pos_at_half.x > 0.0 && pos_at_half.x < 2.0);
        assert_eq!(pos_at_half.y, 0.0);
        assert_eq!(pos_at_half.z, 0.0);
    }

    #[test]
    fn test_curve_get_tangent_at_u() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];

        let curve = Curve::from(points);

        let tangent_at_0 = curve.get_tangent_at_u(0.0);
        assert!((tangent_at_0.length() - 1.0).abs() < 0.001);
        assert!(tangent_at_0.x > 0.99);

        let tangent_at_1 = curve.get_tangent_at_u(1.0);
        assert!((tangent_at_1.length() - 1.0).abs() < 0.001);
        assert!(tangent_at_1.x > 0.99);

        let tangent_at_half = curve.get_tangent_at_u(0.5);
        assert!((tangent_at_half.length() - 1.0).abs() < 0.001);
        assert!(tangent_at_half.x > 0.99);
    }

    #[test]
    fn test_curve_from_points_various_lengths() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(3.0, 4.0, 0.0),
            Vec3::new(6.0, 8.0, 0.0),
        ];

        let curve = Curve::from(points);

        assert_eq!(curve.points.len(), 3);

        let expected_length = 5.0 + 5.0;
        assert!((curve.length - expected_length).abs() < 0.001);
    }

    #[test]
    fn test_curve_smooth_with_less_than_3_points() {
        let points = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];

        let curve = Curve::from(points.clone());
        let smoothed = curve.smooth(1);

        assert_eq!(smoothed.points.len(), 2);
        assert_eq!(smoothed.points, points);
    }

    #[test]
    fn test_curve_resample_with_large_segment_length() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];

        let curve = Curve::from(points.clone());

        let resampled = curve.resample(100.0);

        assert_eq!(resampled.points.len(), 2);
        assert_eq!(resampled.points[0], points[0]);
        assert_eq!(resampled.points[1], points[points.len() - 1]);
    }

    #[test]
    fn test_curve_smooth_recomputes_arc_length() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let original = Curve::from(points);
        let smoothed = original.clone().smooth(1);

        assert!(smoothed.length < original.length);
        assert_eq!(smoothed.points_u[0], 0.0);
        assert_eq!(smoothed.points_u[smoothed.points_u.len() - 1], 1.0);
    }

    #[test]
    fn test_curve_handles_zero_length_points() {
        let point = Vec3::new(1.0, 2.0, 3.0);
        let curve = Curve::from(vec![point, point, point]);

        assert_eq!(curve.length, 0.0);
        assert_eq!(curve.get_pos_at_u(0.5), point);
        assert_eq!(curve.get_tangent_at_u(0.5), Vec3::X);
        assert_eq!(curve.resample(1.0).points.len(), 3);
    }
}
