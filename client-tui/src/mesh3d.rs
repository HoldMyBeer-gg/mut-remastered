//! 3D mesh renderer using z-buffer projection.
//!
//! Renders wireframe/solid 3D meshes into a character buffer using
//! the painter's algorithm with z-depth sorting. Meshes are defined
//! as vertices + faces (triangles/quads).

use std::f64::consts::PI;

/// A 3D vertex.
#[derive(Clone, Copy)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn rotate_x(self, angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Vec3 {
            x: self.x,
            y: self.y * cos - self.z * sin,
            z: self.y * sin + self.z * cos,
        }
    }

    pub fn rotate_y(self, angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Vec3 {
            x: self.x * cos + self.z * sin,
            y: self.y,
            z: -self.x * sin + self.z * cos,
        }
    }

    pub fn rotate_z(self, angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Vec3 {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
            z: self.z,
        }
    }

    pub fn sub(self, other: Vec3) -> Vec3 {
        Vec3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn dot(self, other: Vec3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        if len < 1e-10 { return Vec3::new(0.0, 0.0, 1.0); }
        Vec3::new(self.x / len, self.y / len, self.z / len)
    }
}

/// A face (triangle) defined by 3 vertex indices.
pub struct Face {
    pub v: [usize; 3],
    pub shade_char: char,
}

/// A 3D mesh: vertices + faces.
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Face>,
}

/// Project a 3D point to 2D screen coordinates.
/// Returns (screen_x, screen_y, depth) or None if behind camera.
fn project(v: &Vec3, screen_w: usize, screen_h: usize, fov: f64) -> Option<(f64, f64, f64)> {
    // Camera is at origin looking down +Z
    let z = v.z;
    if z < 0.5 { return None; } // Behind camera

    let aspect = screen_w as f64 / (screen_h as f64 * 2.0); // *2 because chars are ~2x tall
    let scale = fov / z;

    let sx = v.x * scale * aspect + screen_w as f64 / 2.0;
    let sy = -v.y * scale + screen_h as f64 / 2.0;

    Some((sx, sy, z))
}

/// Render a mesh into a character buffer with z-buffer.
///
/// `buf` is (height, width) of (char, fg_color as u8 RGB).
/// `zbuf` is the z-buffer (same dimensions), initialized to f64::MAX.
/// `rotation` is (rx, ry, rz) in radians.
/// `position` is the mesh center in world space.
/// `scale` is the uniform scale factor.
/// `color` is the RGB color for the mesh.
pub fn render_mesh(
    buf: &mut Vec<Vec<(char, [u8; 3])>>,
    zbuf: &mut Vec<Vec<f64>>,
    mesh: &Mesh,
    rotation: (f64, f64, f64),
    position: Vec3,
    scale: f64,
    color: [u8; 3],
) {
    let h = buf.len();
    let w = if h > 0 { buf[0].len() } else { return };
    let fov = 60.0;

    // Transform vertices
    let transformed: Vec<Vec3> = mesh
        .vertices
        .iter()
        .map(|v| {
            let mut p = Vec3::new(v.x * scale, v.y * scale, v.z * scale);
            p = p.rotate_x(rotation.0);
            p = p.rotate_y(rotation.1);
            p = p.rotate_z(rotation.2);
            Vec3::new(p.x + position.x, p.y + position.y, p.z + position.z)
        })
        .collect();

    // For each face, rasterize the triangle
    for face in &mesh.faces {
        let v0 = transformed[face.v[0]];
        let v1 = transformed[face.v[1]];
        let v2 = transformed[face.v[2]];

        // Back-face culling
        let normal = v1.sub(v0).cross(v2.sub(v0)).normalize();
        let view_dir = Vec3::new(0.0, 0.0, 1.0);
        if normal.dot(view_dir) < 0.0 {
            continue;
        }

        // Project to screen
        let p0 = match project(&v0, w, h, fov) { Some(p) => p, None => continue };
        let p1 = match project(&v1, w, h, fov) { Some(p) => p, None => continue };
        let p2 = match project(&v2, w, h, fov) { Some(p) => p, None => continue };

        // Light direction (from top-left)
        let light = Vec3::new(-0.5, 1.0, -0.3).normalize();
        let brightness = normal.dot(light).max(0.1);

        // Choose shade character based on brightness
        let shade = if brightness > 0.8 { '@' }
            else if brightness > 0.6 { '#' }
            else if brightness > 0.45 { '&' }
            else if brightness > 0.3 { '*' }
            else if brightness > 0.15 { '+' }
            else { '.' };

        // Shade color based on brightness
        let r = (color[0] as f64 * brightness).min(255.0) as u8;
        let g = (color[1] as f64 * brightness).min(255.0) as u8;
        let b = (color[2] as f64 * brightness).min(255.0) as u8;

        // Rasterize triangle using scanline
        rasterize_triangle(buf, zbuf, w, h, p0, p1, p2, shade, [r, g, b]);
    }
}

/// Rasterize a projected triangle into the buffer.
fn rasterize_triangle(
    buf: &mut Vec<Vec<(char, [u8; 3])>>,
    zbuf: &mut Vec<Vec<f64>>,
    w: usize, h: usize,
    p0: (f64, f64, f64),
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    shade: char,
    color: [u8; 3],
) {
    // Bounding box
    let min_x = (p0.0.min(p1.0).min(p2.0).floor() as i32).max(0) as usize;
    let max_x = (p0.0.max(p1.0).max(p2.0).ceil() as i32).min(w as i32 - 1) as usize;
    let min_y = (p0.1.min(p1.1).min(p2.1).floor() as i32).max(0) as usize;
    let max_y = (p0.1.max(p1.1).max(p2.1).ceil() as i32).min(h as i32 - 1) as usize;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f64 + 0.5;
            let py = y as f64 + 0.5;

            // Barycentric coordinates
            let (w0, w1, w2) = barycentric(px, py, p0, p1, p2);

            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                // Interpolate depth
                let z = w0 * p0.2 + w1 * p1.2 + w2 * p2.2;

                if z < zbuf[y][x] {
                    zbuf[y][x] = z;
                    buf[y][x] = (shade, color);
                }
            }
        }
    }
}

fn barycentric(
    px: f64, py: f64,
    p0: (f64, f64, f64),
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
) -> (f64, f64, f64) {
    let d = (p1.1 - p2.1) * (p0.0 - p2.0) + (p2.0 - p1.0) * (p0.1 - p2.1);
    if d.abs() < 1e-10 { return (-1.0, -1.0, -1.0); }
    let w0 = ((p1.1 - p2.1) * (px - p2.0) + (p2.0 - p1.0) * (py - p2.1)) / d;
    let w1 = ((p2.1 - p0.1) * (px - p2.0) + (p0.0 - p2.0) * (py - p2.1)) / d;
    let w2 = 1.0 - w0 - w1;
    (w0, w1, w2)
}

// ── Predefined Meshes ────────────────────────────────────────────

/// Create a unit cube mesh centered at origin.
pub fn create_cube() -> Mesh {
    let s = 1.0;
    let vertices = vec![
        Vec3::new(-s, -s, -s), Vec3::new( s, -s, -s),
        Vec3::new( s,  s, -s), Vec3::new(-s,  s, -s),
        Vec3::new(-s, -s,  s), Vec3::new( s, -s,  s),
        Vec3::new( s,  s,  s), Vec3::new(-s,  s,  s),
    ];
    let faces = vec![
        // Front
        Face { v: [0, 1, 2], shade_char: '#' }, Face { v: [0, 2, 3], shade_char: '#' },
        // Back
        Face { v: [5, 4, 7], shade_char: '#' }, Face { v: [5, 7, 6], shade_char: '#' },
        // Left
        Face { v: [4, 0, 3], shade_char: '#' }, Face { v: [4, 3, 7], shade_char: '#' },
        // Right
        Face { v: [1, 5, 6], shade_char: '#' }, Face { v: [1, 6, 2], shade_char: '#' },
        // Top
        Face { v: [3, 2, 6], shade_char: '#' }, Face { v: [3, 6, 7], shade_char: '#' },
        // Bottom
        Face { v: [4, 5, 1], shade_char: '#' }, Face { v: [4, 1, 0], shade_char: '#' },
    ];
    Mesh { vertices, faces }
}

/// Create a diamond/gem shape (octahedron).
pub fn create_diamond() -> Mesh {
    let vertices = vec![
        Vec3::new( 0.0,  1.2,  0.0), // top
        Vec3::new( 1.0,  0.0,  0.0), // right
        Vec3::new( 0.0,  0.0,  1.0), // front
        Vec3::new(-1.0,  0.0,  0.0), // left
        Vec3::new( 0.0,  0.0, -1.0), // back
        Vec3::new( 0.0, -1.2,  0.0), // bottom
    ];
    let faces = vec![
        Face { v: [0, 1, 2], shade_char: '#' },
        Face { v: [0, 2, 3], shade_char: '#' },
        Face { v: [0, 3, 4], shade_char: '#' },
        Face { v: [0, 4, 1], shade_char: '#' },
        Face { v: [5, 2, 1], shade_char: '#' },
        Face { v: [5, 3, 2], shade_char: '#' },
        Face { v: [5, 4, 3], shade_char: '#' },
        Face { v: [5, 1, 4], shade_char: '#' },
    ];
    Mesh { vertices, faces }
}

/// Create a simple humanoid/monster shape (blocky golem).
pub fn create_monster() -> Mesh {
    let mut verts = Vec::new();
    let mut faces = Vec::new();

    // Body (tall box)
    add_box(&mut verts, &mut faces, 0.0, 0.0, 0.0, 0.6, 1.0, 0.4);
    // Head (small box on top)
    add_box(&mut verts, &mut faces, 0.0, 1.3, 0.0, 0.4, 0.4, 0.4);
    // Left arm
    add_box(&mut verts, &mut faces, -0.9, 0.2, 0.0, 0.25, 0.8, 0.25);
    // Right arm
    add_box(&mut verts, &mut faces, 0.9, 0.2, 0.0, 0.25, 0.8, 0.25);
    // Left leg
    add_box(&mut verts, &mut faces, -0.3, -1.3, 0.0, 0.25, 0.6, 0.3);
    // Right leg
    add_box(&mut verts, &mut faces, 0.3, -1.3, 0.0, 0.25, 0.6, 0.3);

    Mesh { vertices: verts, faces }
}

/// Create a sword shape.
pub fn create_sword() -> Mesh {
    let mut verts = Vec::new();
    let mut faces = Vec::new();
    // Blade (thin tall box)
    add_box(&mut verts, &mut faces, 0.0, 0.8, 0.0, 0.08, 1.2, 0.02);
    // Guard (wide short box)
    add_box(&mut verts, &mut faces, 0.0, -0.1, 0.0, 0.4, 0.08, 0.08);
    // Handle
    add_box(&mut verts, &mut faces, 0.0, -0.6, 0.0, 0.06, 0.4, 0.06);
    Mesh { vertices: verts, faces }
}

/// Helper: add a box centered at (cx, cy, cz) with half-extents (hx, hy, hz).
fn add_box(
    verts: &mut Vec<Vec3>,
    faces: &mut Vec<Face>,
    cx: f64, cy: f64, cz: f64,
    hx: f64, hy: f64, hz: f64,
) {
    let base = verts.len();
    verts.push(Vec3::new(cx - hx, cy - hy, cz - hz));
    verts.push(Vec3::new(cx + hx, cy - hy, cz - hz));
    verts.push(Vec3::new(cx + hx, cy + hy, cz - hz));
    verts.push(Vec3::new(cx - hx, cy + hy, cz - hz));
    verts.push(Vec3::new(cx - hx, cy - hy, cz + hz));
    verts.push(Vec3::new(cx + hx, cy - hy, cz + hz));
    verts.push(Vec3::new(cx + hx, cy + hy, cz + hz));
    verts.push(Vec3::new(cx - hx, cy + hy, cz + hz));

    let f = |a, b, c| Face { v: [base + a, base + b, base + c], shade_char: '#' };
    faces.push(f(0, 1, 2)); faces.push(f(0, 2, 3));
    faces.push(f(5, 4, 7)); faces.push(f(5, 7, 6));
    faces.push(f(4, 0, 3)); faces.push(f(4, 3, 7));
    faces.push(f(1, 5, 6)); faces.push(f(1, 6, 2));
    faces.push(f(3, 2, 6)); faces.push(f(3, 6, 7));
    faces.push(f(4, 5, 1)); faces.push(f(4, 1, 0));
}
