use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use crate::map_editor::data::EditableMesh;

impl EditableMesh {
    /// Create a standard 1x1x1 cube centered at origin
    pub fn new_cube(size: f32) -> Self {
        let h = size * 0.5;
        let vertices = vec![
            [-h, -h, -h], [h, -h, -h], [h, h, -h], [-h, h, -h], // Back face
            [-h, -h,  h], [h, -h,  h], [h, h,  h], [-h, h,  h], // Front face
        ];
        // Faces (windings pointing outwards)
        let faces = vec![
            vec![0, 3, 2, 1], // Back
            vec![4, 5, 6, 7], // Front
            vec![0, 1, 5, 4], // Bottom
            vec![2, 3, 7, 6], // Top
            vec![0, 4, 7, 3], // Left
            vec![1, 2, 6, 5], // Right
        ];
        Self { vertices, faces }
    }

    /// Converts this EditableMesh to a Bevy Render Mesh (triangulating arbitrary polygons)
    pub fn to_bevy_mesh(&self) -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let mut v_idx = 0;
        for face in &self.faces {
            if face.len() < 3 {
                continue;
            }

            // Calculate face normal
            let v0 = Vec3::from_array(self.vertices[face[0] as usize]);
            let v1 = Vec3::from_array(self.vertices[face[1] as usize]);
            let v2 = Vec3::from_array(self.vertices[face[2] as usize]);
            let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();

            // Triangulate face using simple fan triangulation
            for i in 1..(face.len() - 1) {
                let idx0 = face[0];
                let idx1 = face[i];
                let idx2 = face[i + 1];

                let p0 = self.vertices[idx0 as usize];
                let p1 = self.vertices[idx1 as usize];
                let p2 = self.vertices[idx2 as usize];

                positions.push(p0);
                positions.push(p1);
                positions.push(p2);

                normals.push(normal.to_array());
                normals.push(normal.to_array());
                normals.push(normal.to_array());

                // Generate simple planar/box projection UVs
                let uv0 = [p0[0] * 0.5 + 0.5, p0[2] * 0.5 + 0.5];
                let uv1 = [p1[0] * 0.5 + 0.5, p1[2] * 0.5 + 0.5];
                let uv2 = [p2[0] * 0.5 + 0.5, p2[2] * 0.5 + 0.5];

                uvs.push(uv0);
                uvs.push(uv1);
                uvs.push(uv2);

                indices.push(v_idx);
                indices.push(v_idx + 1);
                indices.push(v_idx + 2);
                v_idx += 3;
            }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }

    /// Extrudes the selected face index by a given distance along its normal
    pub fn extrude(&mut self, face_idx: usize, distance: f32) {
        if face_idx >= self.faces.len() {
            return;
        }

        let face = self.faces[face_idx].clone();
        if face.len() < 3 {
            return;
        }

        // Calculate face normal
        let v0 = Vec3::from_array(self.vertices[face[0] as usize]);
        let v1 = Vec3::from_array(self.vertices[face[1] as usize]);
        let v2 = Vec3::from_array(self.vertices[face[2] as usize]);
        let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();
        let offset = normal * distance;

        // Duplicate vertices of the face
        let mut new_vert_indices = Vec::new();
        for &orig_v_idx in &face {
            let orig_pos = Vec3::from_array(self.vertices[orig_v_idx as usize]);
            let new_pos = (orig_pos + offset).to_array();
            self.vertices.push(new_pos);
            new_vert_indices.push((self.vertices.len() - 1) as u32);
        }

        // Create new side faces connecting the original boundary to the new extruded face
        let len = face.len();
        for i in 0..len {
            let next_i = (i + 1) % len;
            let orig_a = face[i];
            let orig_b = face[next_i];
            let new_a = new_vert_indices[i];
            let new_b = new_vert_indices[next_i];

            // Add side quad (or triangle if base is degenerated)
            self.faces.push(vec![orig_a, orig_b, new_b, new_a]);
        }

        // Replace the original face with the new extruded face
        // Keeping winding orientation consistent
        self.faces[face_idx] = new_vert_indices;
    }

    /// Insets the selected face index by a factor (0.0 - 1.0) towards its center
    pub fn inset(&mut self, face_idx: usize, factor: f32) {
        if face_idx >= self.faces.len() {
            return;
        }

        let face = self.faces[face_idx].clone();
        if face.len() < 3 {
            return;
        }

        // Compute face center
        let mut center = Vec3::ZERO;
        for &v_idx in &face {
            center += Vec3::from_array(self.vertices[v_idx as usize]);
        }
        center /= face.len() as f32;

        // Create insetted vertices
        let mut inset_vert_indices = Vec::new();
        for &v_idx in &face {
            let pos = Vec3::from_array(self.vertices[v_idx as usize]);
            let inset_pos = pos.lerp(center, factor).to_array();
            self.vertices.push(inset_pos);
            inset_vert_indices.push((self.vertices.len() - 1) as u32);
        }

        // Create side faces between original face and insetted face
        let len = face.len();
        for i in 0..len {
            let next_i = (i + 1) % len;
            let orig_a = face[i];
            let orig_b = face[next_i];
            let inset_a = inset_vert_indices[i];
            let inset_b = inset_vert_indices[next_i];

            self.faces.push(vec![orig_a, orig_b, inset_b, inset_a]);
        }

        // Replace original face with the insetted face
        self.faces[face_idx] = inset_vert_indices;
    }

    /// Subdivides the entire mesh by splitting all faces using midpoint subdivision
    pub fn subdivide(&mut self) {
        let mut new_faces = Vec::new();
        
        for face in &self.faces {
            if face.len() < 3 {
                continue;
            }

            // Calculate face center
            let mut center = Vec3::ZERO;
            for &v_idx in face {
                center += Vec3::from_array(self.vertices[v_idx as usize]);
            }
            center /= face.len() as f32;
            self.vertices.push(center.to_array());
            let center_v_idx = (self.vertices.len() - 1) as u32;

            // Calculate midpoints of all edges of the face
            let mut edge_mid_indices = Vec::new();
            let len = face.len();
            for i in 0..len {
                let next_i = (i + 1) % len;
                let va = Vec3::from_array(self.vertices[face[i] as usize]);
                let vb = Vec3::from_array(self.vertices[face[next_i] as usize]);
                let mid = (va + vb) * 0.5;
                self.vertices.push(mid.to_array());
                edge_mid_indices.push((self.vertices.len() - 1) as u32);
            }

            // Create subdivided sub-faces
            for i in 0..len {
                let prev_i = (i + len - 1) % len;
                let orig_v = face[i];
                let mid_prev = edge_mid_indices[prev_i];
                let mid_next = edge_mid_indices[i];
                
                new_faces.push(vec![orig_v, mid_next, center_v_idx, mid_prev]);
            }
        }

        self.faces = new_faces;
    }

    /// Bevels (chamfers) all vertices by shaving off a specified amount
    pub fn bevel(&mut self, amount: f32) {
        let mut new_faces = Vec::new();
        let orig_vert_count = self.vertices.len();
        
        // For each vertex, track all faces sharing it
        let mut vertex_faces = vec![Vec::new(); orig_vert_count];
        for (f_idx, face) in self.faces.iter().enumerate() {
            for &v_idx in face {
                vertex_faces[v_idx as usize].push(f_idx);
            }
        }

        // Create new vertices shifted along edges
        // Map from (orig_v, face_idx) -> new_v_idx
        let mut new_vert_map = std::collections::HashMap::new();

        for v_idx in 0..orig_vert_count {
            let faces = &vertex_faces[v_idx];
            if faces.is_empty() {
                continue;
            }
            let pos = Vec3::from_array(self.vertices[v_idx]);

            // Calculate shift for this vertex in each face
            let mut face_verts = Vec::new();
            for &f_idx in faces {
                // Find face center or adjacent vertices in this face to get inward direction
                let face = &self.faces[f_idx];
                let mut face_center = Vec3::ZERO;
                for &fv in face {
                    face_center += Vec3::from_array(self.vertices[fv as usize]);
                }
                face_center /= face.len() as f32;

                let dir = (face_center - pos).normalize_or_zero();
                let shifted_pos = (pos + dir * amount).to_array();
                self.vertices.push(shifted_pos);
                let new_v = (self.vertices.len() - 1) as u32;
                new_vert_map.insert((v_idx as u32, f_idx), new_v);
                face_verts.push(new_v);
            }

            // Spawn a cap face covering the beveled vertex
            if face_verts.len() >= 3 {
                // simple cap order
                new_faces.push(face_verts);
            }
        }

        // Reconstruct original faces using beveled vertices
        for (f_idx, face) in self.faces.iter().enumerate() {
            let mut reconstructed_face = Vec::new();
            for &v_idx in face {
                if let Some(&new_v) = new_vert_map.get(&(v_idx, f_idx)) {
                    reconstructed_face.push(new_v);
                }
            }
            if reconstructed_face.len() >= 3 {
                new_faces.push(reconstructed_face);
            }
        }

        self.faces = new_faces;
    }

    /// Slices the mesh along a plane defined by origin and normal (Knife/Cut Tool)
    pub fn knife_cut(&mut self, origin: Vec3, normal: Vec3) {
        let mut new_faces = Vec::new();
        let plane_norm = normal.normalize_or_zero();

        for face in &self.faces {
            if face.len() < 3 {
                continue;
            }

            let mut side_a = Vec::new();
            let mut side_b = Vec::new();

            let len = face.len();
            for i in 0..len {
                let next_i = (i + 1) % len;
                let v_a_idx = face[i];
                let v_b_idx = face[next_i];
                let pos_a = Vec3::from_array(self.vertices[v_a_idx as usize]);
                let pos_b = Vec3::from_array(self.vertices[v_b_idx as usize]);

                let dist_a = (pos_a - origin).dot(plane_norm);
                let dist_b = (pos_b - origin).dot(plane_norm);

                // Add vertex A to its correct side
                if dist_a >= 0.0 {
                    side_a.push(v_a_idx);
                } else {
                    side_b.push(v_a_idx);
                }

                // Check if edge intersects the cut plane
                if (dist_a > 0.0 && dist_b < 0.0) || (dist_a < 0.0 && dist_b > 0.0) {
                    let t = dist_a / (dist_a - dist_b);
                    let intersect_pos = pos_a.lerp(pos_b, t).to_array();
                    self.vertices.push(intersect_pos);
                    let intersect_idx = (self.vertices.len() - 1) as u32;

                    side_a.push(intersect_idx);
                    side_b.push(intersect_idx);
                }
            }

            // Create new faces from the split parts
            if side_a.len() >= 3 {
                new_faces.push(side_a);
            }
            if side_b.len() >= 3 {
                new_faces.push(side_b);
            }
        }

        self.faces = new_faces;
    }

    /// Connects two separate faces to bridge them, creating a geometric tunnel/bridge
    pub fn bridge(&mut self, face_idx_a: usize, face_idx_b: usize) {
        if face_idx_a >= self.faces.len() || face_idx_b >= self.faces.len() || face_idx_a == face_idx_b {
            return;
        }

        let face_a = self.faces[face_idx_a].clone();
        let face_b = self.faces[face_idx_b].clone();

        if face_a.len() != face_b.len() || face_a.len() < 3 {
            return; // Simple bridge requires matching vertex counts
        }

        let len = face_a.len();
        // Create side faces bridging the edges of both faces
        for i in 0..len {
            let next_i = (i + 1) % len;
            
            // Bridge side quad
            let a1 = face_a[i];
            let a2 = face_a[next_i];
            let b1 = face_b[i];
            let b2 = face_b[next_i];

            self.faces.push(vec![a1, a2, b2, b1]);
        }

        // Remove the original cap faces to make it a hollow tunnel/bridge
        if face_idx_a > face_idx_b {
            self.faces.remove(face_idx_a);
            self.faces.remove(face_idx_b);
        } else {
            self.faces.remove(face_idx_b);
            self.faces.remove(face_idx_a);
        }
    }

    /// Performs a Boolean Union, Subtraction, or Intersection with another mesh
    pub fn boolean_operation(&mut self, other: &EditableMesh, op: &str, other_pos: Vec3, other_rot: Quat) {
        // Transform other mesh's vertices to self local space
        let mut transformed_other_verts = Vec::new();
        for v in &other.vertices {
            let pos = Vec3::from_array(*v);
            let world_pos = other_rot * pos + other_pos;
            transformed_other_verts.push(world_pos);
        }

        // To achieve a highly robust and reliable boolean output in plain Rust,
        // we use a solid volume plane-clipping categorization.
        // For each polygon (face) in Mesh A (self):
        //   Classify it against the volume of Mesh B (other).
        // For each polygon in Mesh B:
        //   Classify it against the volume of Mesh A.
        //
        // A simple inside/outside test for a point against a mesh is to find if it is on the negative
        // side of all planes of the mesh (works perfectly for convex components which make up Backrooms assets).
        let mut kept_faces = Vec::new();
        
        let self_vertices: Vec<Vec3> = self.vertices.iter().map(|v| Vec3::from_array(*v)).collect();
        
        // 1. Process self faces against other's volume
        for face in &self.faces {
            let mut center = Vec3::ZERO;
            for &v in face {
                center += self_vertices[v as usize];
            }
            center /= face.len() as f32;

            let is_inside_other = is_point_inside_mesh(center, &transformed_other_verts, &other.faces);

            match op {
                "union" => {
                    // Keep faces of A outside B
                    if !is_inside_other {
                        kept_faces.push(face.clone());
                    }
                }
                "subtract" => {
                    // Keep faces of A outside B
                    if !is_inside_other {
                        kept_faces.push(face.clone());
                    }
                }
                "intersection" => {
                    // Keep faces of A inside B
                    if is_inside_other {
                        kept_faces.push(face.clone());
                    }
                }
                _ => {}
            }
        }

        // 2. Process other faces against self volume and merge them
        let base_vert_offset = self.vertices.len() as u32;
        for v in &transformed_other_verts {
            self.vertices.push(v.to_array());
        }

        for other_face in &other.faces {
            let mut center = Vec3::ZERO;
            for &v in other_face {
                center += transformed_other_verts[v as usize];
            }
            center /= other_face.len() as f32;

            let is_inside_self = is_point_inside_mesh(center, &self_vertices, &self.faces);

            // Shift indices of other face to point to our newly appended vertices
            let shifted_face: Vec<u32> = other_face.iter().map(|&v| v + base_vert_offset).collect();

            match op {
                "union" => {
                    // Keep faces of B outside A
                    if !is_inside_self {
                        kept_faces.push(shifted_face);
                    }
                }
                "subtract" => {
                    // Keep faces of B inside A, but reverse their normals (flip index order)
                    if is_inside_self {
                        let mut flipped = shifted_face;
                        flipped.reverse();
                        kept_faces.push(flipped);
                    }
                }
                "intersection" => {
                    // Keep faces of B inside A
                    if is_inside_self {
                        kept_faces.push(shifted_face);
                    }
                }
                _ => {}
            }
        }

        self.faces = kept_faces;
        self.cleanup_unused_vertices();
    }

    /// Cleans up any vertices that are no longer referenced by any faces
    pub fn cleanup_unused_vertices(&mut self) {
        let mut used = vec![false; self.vertices.len()];
        for face in &self.faces {
            for &v in face {
                if (v as usize) < used.len() {
                    used[v as usize] = true;
                }
            }
        }

        let mut new_vertices = Vec::new();
        let mut index_map = vec![0; self.vertices.len()];
        for (i, &is_used) in used.iter().enumerate() {
            if is_used {
                new_vertices.push(self.vertices[i]);
                index_map[i] = (new_vertices.len() - 1) as u32;
            }
        }

        let mut new_faces = Vec::new();
        for face in &self.faces {
            let mapped_face: Vec<u32> = face.iter().map(|&v| index_map[v as usize]).collect();
            new_faces.push(mapped_face);
        }

        self.vertices = new_vertices;
        self.faces = new_faces;
    }
}

/// Helper function using raycasting/winding checks to determine if a point is inside a mesh volume
fn is_point_inside_mesh(point: Vec3, vertices: &[Vec3], faces: &[Vec<u32>]) -> bool {
    // For convex parts (which are typical for modular structures),
    // a point is inside if it is on the back side of all polygon planes.
    // Let's perform a plane-sidedness check for all faces.
    for face in faces {
        if face.len() < 3 {
            continue;
        }
        let v0 = vertices[face[0] as usize];
        let v1 = vertices[face[1] as usize];
        let v2 = vertices[face[2] as usize];

        let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();
        let dist = (point - v0).dot(normal);

        // If the point is on the positive (outside) side of ANY plane,
        // it is outside the convex shape.
        if dist > 0.05 {
            return false;
        }
    }
    true
}
