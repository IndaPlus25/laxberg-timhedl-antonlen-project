use rand::{self, RngExt};
use super::file_parser::{Mesh, Face};

/// Checks if any of a triangles vertecies are within a given box
fn verticies_in_cube(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    for vertex in vertecies {
        let dx = vertex[0] - cube_center[0];
        let dy = vertex[1] - cube_center[1];
        let dz = vertex[2] - cube_center[2];

        if dx.abs() > cube_width { continue; }
        if dy.abs() > cube_width { continue; }
        if dz.abs() > cube_width { continue; }

        return true;
    };

    return false;
}

fn vertecies_outside_same_face(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    // Iterate over the 3 axis x,y,z
    for axis in 0..3 {
        
        // Calculate distance from center along 'axis' for all 3 vertecies
        let dv1 = vertecies[0][axis] - cube_center[axis];
        let dv2 = vertecies[1][axis] - cube_center[axis];
        let dv3 = vertecies[2][axis] - cube_center[axis];

        if dv1 > cube_width && dv2 > cube_width && dv3 > cube_width {
            return true;
        }

        if -dv1 > cube_width && -dv2 > cube_width && -dv3 > cube_width {
            return true;
        }
    };

    return false;
}

fn vertecies_outside_same_edge(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    let threshold = 2.0 * cube_width;

    // Array of possible faces, remains true if all vertecies are outside it
    let mut outsides = [true; 12];

    for vertex in vertecies {
        let dx = vertex[0] - cube_center[0];
        let dy = vertex[1] - cube_center[1];
        let dz = vertex[2] - cube_center[2];

        // Planes independent of the z-axis
        if dx + dy <= threshold { outsides[0] = false; }
        if dx - dy <= threshold { outsides[1] = false; }
        if -dx + dy <= threshold { outsides[2] = false; }
        if -dx - dy <= threshold { outsides[3] = false; }
        
        // Planes independent of the x-axis
        if dy + dz <= threshold { outsides[4] = false; }
        if dy - dz <= threshold { outsides[5] = false; }
        if -dy + dz <= threshold { outsides[6] = false; }
        if -dy - dz <= threshold { outsides[7] = false; }

        // Planes independent of the y-axis
        if dx + dz <= threshold { outsides[8] = false; }
        if dx - dz <= threshold { outsides[9] = false; }
        if -dx + dz <= threshold { outsides[10] = false; }
        if -dx - dz <= threshold { outsides[11] = false; }

        
    }

    outsides.iter().any(|x| *x )
}

fn vertecies_outside_same_corner(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    let threshold = 3.0  * cube_width;
    
    // Array of possible corner planes, element is true if all vertecies outside it
    let mut outsides = [true; 8];

    for vertex in vertecies {
        let dx = vertex[0] - cube_center[0];
        let dy = vertex[1] - cube_center[1];
        let dz = vertex[2] - cube_center[2];

        // Positive x
        if dx + dy + dz <= threshold { outsides[0] = false; }
        if dx + dy - dz <= threshold { outsides[1] = false; }
        if dx - dy + dz <= threshold { outsides[2] = false; }
        if dx - dy - dz <= threshold { outsides[3] = false; }

        // Negative x
        if -dx + dy + dz <= threshold { outsides[4] = false; }
        if -dx + dy - dz <= threshold { outsides[5] = false; }
        if -dx - dy + dz <= threshold { outsides[6] = false; }
        if -dx - dy - dz <= threshold { outsides[7] = false; }
    }

    outsides.iter().any(|x| *x )
}

fn triangle_edge_pierces_cube_face(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    let edge1 = [vertecies[0], vertecies[1]];
    let edge2 = [vertecies[1], vertecies[2]];
    let edge3 = [vertecies[2], vertecies[0]];
    let edges = [edge1, edge2, edge3];

    // Iterate over the planes and calculate the axis and axis value for the plane
    for i in 0..6 {
        let axis = i % 3;
        let sign = if i < 3 { 1.0 } else { -1.0 };
        let plane_axis_value = cube_center[axis] + (cube_width * sign);

        // Iterate over the edges and check if any intersect with the current plane
        for edge in 0..3 {
            let Some(vertex) = line_intersect_point_with_plane(edges[edge][0], edges[edge][1], axis, plane_axis_value) else {
                continue;
            };

            let dx = vertex[0] - cube_center[0];
            let dy = vertex[1] - cube_center[1];
            let dz = vertex[2] - cube_center[2];

            if dx > cube_width || -dx > cube_width { continue; }
            if dy > cube_width || -dy > cube_width { continue; }
            if dz > cube_width || -dz > cube_width { continue; }

            return true
        }
    };

    false
}

fn line_intersect_point_with_plane(point1: [f32; 3], point2: [f32; 3], axis: usize, plane_axis_value: f32) -> Option<[f32; 3]> {
    if point1[axis] == point2[axis] { return None; }

    let alpha = (plane_axis_value - point2[axis]) / (point1[axis] - point2[axis]);
    if alpha < 0.0 || alpha > 1.0 { return None; }

    let x = (alpha * point1[0]) + ((1.0 - alpha) * point2[0]);
    let y = (alpha * point1[1]) + ((1.0 - alpha) * point2[1]);
    let z = (alpha * point1[2]) + ((1.0 - alpha) * point2[2]);

    let mut vertex = [x, y, z];
    vertex[axis] = plane_axis_value;

    return Some(vertex)
}

fn cube_corner_pierce_triangle(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    let directions = [
        // Positive x diagonals
        [cube_width, cube_width, cube_width],
        [cube_width, cube_width, -cube_width],
        [cube_width, -cube_width, cube_width],
        [cube_width, -cube_width, -cube_width],

        // Negative x diagonals
        [-cube_width, cube_width, cube_width],
        [-cube_width, cube_width, -cube_width],
        [-cube_width, -cube_width, cube_width],
        [-cube_width, -cube_width, -cube_width],
    ];
    
    let edge1 = [
        vertecies[1][0] - vertecies[0][0],
        vertecies[1][1] - vertecies[0][1],
        vertecies[1][2] - vertecies[0][2]
    ];

    let edge2 = [
        vertecies[2][0] - vertecies[0][0],
        vertecies[2][1] - vertecies[0][1],
        vertecies[2][2] - vertecies[0][2]
    ];

    let mut intersections = [true; 8];

    for i in 0..8 {
        let direction = directions[i];

        let h = [
            direction[1] * edge2[2] - direction[2] * edge2[1],
            -(direction[0] * edge2[2] - direction[2] * edge2[0]),
            direction[0] * edge2[1] - direction[1] * edge2[0],
        ];

        let a = edge1[0] * h[0] + edge1[1] * h[1] + edge1[2] * h[2];
        if a > -1e-7 && a < 1e-7 {
            intersections[i] = false;
            continue;
        };

        let f = 1.0 / a;
        let s = [
            cube_center[0] - vertecies[0][0],
            cube_center[1] - vertecies[0][1],
            cube_center[2] - vertecies[0][2],
        ];
        let u = f * (s[0] * h[0] + s[1] * h[1] + s[2] * h[2]);
        if u < 0.0 || u > 1.0 {
            intersections[i] = false;
            continue;
        };

        let q = [
            s[1] * edge1[2] - s[2] * edge1[1],
            -(s[0] * edge1[2] - s[2] * edge1[0]),
            s[0] * edge1[1] - s[1] * edge1[0],
        ];
        let v = f * (direction[0] * q[0] + direction[1] * q[1] + direction[2] * q[2]);
        if v < 0.0 || u + v > 1.0 {
            intersections[i] = false;
            continue;
        }

        let t = f * (edge2[0] * q[0] + edge2[1] * q[1] + edge2[2] * q[2]);
        if t < 0.0 || t > 1.0 {
            intersections[i] = false;
        }
    };

    intersections.iter().any(|x| *x )
}

fn triangle_cube_intersection(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    if verticies_in_cube(vertecies, cube_center, cube_width) {
        return true;
    }

    if vertecies_outside_same_face(vertecies, cube_center, cube_width) {
        return false;
    }

    if vertecies_outside_same_edge(vertecies, cube_center, cube_width) {
        return false;
    }

    if vertecies_outside_same_corner(vertecies, cube_center, cube_width) {
        return false;
    }

    if triangle_edge_pierces_cube_face(vertecies, cube_center, cube_width) {
        return true;
    }

    if cube_corner_pierce_triangle(vertecies, cube_center, cube_width) {
        return true;
    }

    // Default return value, should be unreachable
    false
}

fn vertecies_from_mesh_face(mesh: &Mesh, face: &Face) -> [[f32; 3]; 3] {
    [
        [
            mesh.vertices[face.v1].x,
            mesh.vertices[face.v1].y,
            mesh.vertices[face.v1].z,
        ],
        [
            mesh.vertices[face.v2].x,
            mesh.vertices[face.v2].y,
            mesh.vertices[face.v2].z,
        ],
        [
            mesh.vertices[face.v3].x,
            mesh.vertices[face.v3].y,
            mesh.vertices[face.v3].z,
        ],
    ]
}

pub fn voxel_grid_from_triangles(mesh: Mesh, min_width: usize) -> Vec<Vec<Vec<u32>>> {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];

    // Set min and max values for each axis
    for triangle in &mesh.faces {
        let vertecies = vertecies_from_mesh_face(&mesh, triangle);
        for vertex in vertecies {
            let x = vertex[0];
            let y = vertex[1];
            let z = vertex[2];

            if x < min[0] { min[0] = x; }
            if x > max[0] { max[0] = x; }

            if y < min[1] { min[1] = y; }
            if y > max[1] { max[1] = y; }

            if z < min[2] { min[2] = z; }
            if z > max[2] { max[2] = z; }
        }
    };

    // Calculate the size and amount of cubes using the shortest axis and the 'min_width' argument 
    let axis_width = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let min_width_axis = axis_width.iter().zip([0, 1, 2]).reduce(|x, y| if x.0 < y.0 { x } else { y }).unwrap().1;
    let cube_width = axis_width[min_width_axis] / (min_width as f32);
    let cubes_per_axis: [usize; 3] = [
        (axis_width[0] / cube_width).ceil() as usize,
        (axis_width[1] / cube_width).ceil() as usize,
        (axis_width[2] / cube_width).ceil() as usize,
    ];

    let mut voxel_grid = vec![vec![vec![0; cubes_per_axis[0]]; cubes_per_axis[1]]; cubes_per_axis[2]];

    let mut rng = rand::rng();

    // Iterate over x,y,z and calculate the cube's position in the "triangle world"
    for z_step in 0..cubes_per_axis[2] {
        let z = max[2] - (cube_width * (z_step as f32) + cube_width * 0.5);

        for y_step in 0..cubes_per_axis[1] {
            let y = min[1] + (cube_width * (y_step as f32) + cube_width * 0.5);

            for x_step in 0..cubes_per_axis[0] {
                let x = min[0] + (cube_width * (x_step as f32) + cube_width * 0.5);

                // Iterate over the triangles and check if any intersect with the cube 
                for triangle in &mesh.faces {
                    let vertecies = vertecies_from_mesh_face(&mesh, triangle);

                    if triangle_cube_intersection(vertecies, [x, y, z], cube_width * 0.5) {
                        voxel_grid[z_step][y_step][x_step] = rng.random_range(1..=6); 
                        break;
                    }
                }
            }
        }
    }

    return voxel_grid;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_vertex_in_cube_return_true() {
        let vertecies = [
            [3.0, 3.0, 3.0],
            [0.5, 0.5, 0.5],
            [2.0, 2.0, 2.0]
        ];
        let center = [0.5, 0.5, 0.5];
        let width = 0.5;

        assert!(verticies_in_cube(vertecies, center, width))
    }

    #[test]
    fn all_vertecies_in_cube_return_true() {
        let vertecies = [
            [0.6, 0.6, 0.6],
            [0.5, 0.5, 0.5],
            [0.4, 0.4, 0.4]
        ];
        let center = [0.5, 0.5, 0.5];
        let width = 0.5;
        
        assert!(verticies_in_cube(vertecies, center, width))
    }

    #[test]
    fn no_vertex_in_cube_return_false() {
        let vertecies = [
            [1.6, 1.6, 1.6],
            [2.5, 2.5, 2.5],
            [3.4, 3.4, 3.4]
        ];
        let center = [0.5, 0.5, 0.5];
        let width = 0.5;
        
        assert_eq!(verticies_in_cube(vertecies, center, width), false)
    }

    #[test]
    fn negative_vertex_value_inside_cube_return_true() {
        
        let vertecies = [
            [-1.6, 1.6, -1.6],
            [2.5, 2.5, 2.5],
            [3.4, 3.4, 3.4]
        ];
        let center = [0.0, 0.0, 0.0];
        let width = 2.0;
        
        assert!(verticies_in_cube(vertecies, center, width))
    }

    #[test]
    fn all_vertecies_outside_singular_face_plane_return_true() {
        let vertecies = [
            [1.6, 1.6, -1.6],
            [2.5, 2.5, 2.5],
            [3.4, -3.4, 3.4]
        ];
        let center = [0.5, 0.5, 0.5];
        let width = 0.5;
        
        assert!(vertecies_outside_same_face(vertecies, center, width))
    }

    #[test]
    fn all_vertecies_outside_multiple_face_planes_return_true() {
        let vertecies = [
            [1.6, 1.6, -1.6],
            [2.5, 2.5, 2.5],
            [3.4, 3.4, 3.4]
        ];
        let center = [0.5, 0.5, 0.5];
        let width = 0.5;
        
        assert!(vertecies_outside_same_face(vertecies, center, width))
    }

    #[test]
    fn not_all_vertecies_outside_singular_face_planes_return_false() {
        let vertecies = [
            [-1.6, 1.6, -1.6],
            [2.5, -2.5, 2.5],
            [3.4, 3.4, 3.4]
        ];
        let center = [0.5, 0.5, 0.5];
        let width = 0.5;
        
        assert_eq!(vertecies_outside_same_face(vertecies, center, width), false)
    }

    #[test]
    fn all_vertecies_outside_singular_edge_plane_return_true() {
        let vertecies = [
            [-2.0, 1.0, 1.5],
            [-1.0, -1.0, 1.5],
            [-2.5, 2.0, 0.0]
        ];
        let center = [0.0, 0.0, 0.0];
        let width = 1.0;
        
        assert!(vertecies_outside_same_edge(vertecies, center, width))
    }

    #[test]
    fn all_vertecies_outside_multiple_edge_planes_return_true() {
        let vertecies = [
            [-2.0, 1.0, 1.5],
            [-1.0, 1.0, 1.5],
            [-2.5, 3.0, 0.0]
        ];
        let center = [0.0, 0.0, 0.0];
        let width = 1.0;
        
        assert!(vertecies_outside_same_edge(vertecies, center, width))
    }

    #[test]
    fn not_all_vertecies_outside_singular_edge_planes_return_false() {
        let vertecies = [
            [2.0, -1.0, 1.5],
            [1.0, -1.0, 1.5],
            [-2.5, 3.0, 0.0]
        ];
        let center = [0.0, 0.0, 0.0];
        let width = 1.0;
        
        assert_eq!(vertecies_outside_same_edge(vertecies, center, width), false)
    }

    #[test]
    fn all_vertecies_outside_singular_corner_plane_return_true() {
        let vertecies = [
            [1.2, 1.2, 1.2],
            [2.0, 0.5, 1.2],
            [0.5, 2.0, 1.2],
        ];

        let center = [0.0, 0.0, 0.0];
        let width = 1.0;
        
        assert!(vertecies_outside_same_corner(vertecies, center, width))
    }

    #[test]
    fn all_vertecies_outside_multiple_corner_planes_return_true() {
        let vertecies = [
            [6.0, 0.5, 1.2],
            [6.0, -0.5, 1.2],
            [8.0, 0.0, 1.2],
        ];

        let center = [0.0, 0.0, 0.0];
        let width = 1.0;
        
        assert!(vertecies_outside_same_corner(vertecies, center, width))
    }

    #[test]
    fn not_all_vertecies_outside_singular_corner_plane_return_false() {
        let vertecies = [
            [3.0, 3.0, 3.0], 
            [0.5, 0.5, 0.5], 
            [0.0, 0.5, 0.5], 
        ];
        let center = [0.0, 0.0, 0.0];
        let width = 1.0;
        
        assert_eq!(vertecies_outside_same_corner(vertecies, center, width), false)
    }

    #[test]
    fn line_intersects_with_plane_returns_intersection_point() {
        let points = [
            [-2.0, -4.0, 1.0],
            [5.0, -2.0, 1.0]
        ];
        let axis = 0;
        let plane_position = 2.0;

        assert_eq!(line_intersect_point_with_plane(points[0], points[1], axis, plane_position), Some([2.0, -2.857143, 1.0]))
    }

    #[test]
    fn line_not_intersecting_with_plane_returns_none() {
        let points = [
            [5.0, -4.0, 1.0],
            [5.0, -2.0, 1.0]
        ];
        let axis = 0;
        let plane_position = 2.0;

        assert_eq!(line_intersect_point_with_plane(points[0], points[1], axis, plane_position), None)
        
    }

    #[test]
    fn triangle_pierces_positive_x_and_positive_y_faces_return_true() {
        let vertecies = [
            [0.0, 0.0, 0.0], 
            [4.0, -2.0, 1.0], 
            [4.0, 0.0, 2.0], 
        ];
        let center = [2.0, -2.0, 1.0];
        let width = 1.0;
        
        assert!(triangle_edge_pierces_cube_face(vertecies, center, width))
    }

    #[test]
    fn triangle_pierces_x_and_positive_y_faces_return_true() {
        let vertecies = [
            [-2.0, -2.0, 0.0], 
            [4.0, -2.0, 1.0], 
            [4.0, 0.0, 2.0], 
        ];
        let center = [2.0, -2.0, 1.0];
        let width = 1.0;
        
        assert!(triangle_edge_pierces_cube_face(vertecies, center, width))
    }

    #[test]
    fn triangle_pierces_x_and_y_faces_return_true() {
        let vertecies = [
            [0.0, -4.0, 1.0], 
            [5.0, -1.0, 1.0], 
            [2.0, 0.0, 1.0], 
        ];
        let center = [2.0, -2.0, 1.0];
        let width = 1.0;
        
        assert!(triangle_edge_pierces_cube_face(vertecies, center, width))
    }

    #[test]
    fn no_edge_intersects_cube_face_return_false() {
        let vertecies = [
            [-2.0, -4.0, 1.0], 
            [5.0, -4.0, 1.0], 
            [2.0, 2.0, 1.0], 
        ];
        let center = [2.0, -2.0, 1.0];
        let width = 1.0;
        
        assert_eq!(triangle_edge_pierces_cube_face(vertecies, center, width), false)
    } 

    #[test]
    fn cube_at_center_corner_pierces_triangle_return_true() {
        let vertecies = [
            [3.0, 0.0, 0.0], 
            [0.0, 3.0, 0.0], 
            [0.0, 0.0, 1.75], 
        ];
        let center = [0.0, 0.0, 0.0];
        let width = 1.0;
        
        assert!(cube_corner_pierce_triangle(vertecies, center, width));
    }

    #[test]
    fn cube_not_at_center_corner_pierces_triangle_return_true() {
        let vertecies = [
            [8.0, -4.0, 5.0], 
            [3.0, 1.0, 5.0], 
            [3.0, -4.0, 9.0], 
        ];
        let center = [3.0, -4.0, 5.0];
        let width = 2.0;
        
        assert!(cube_corner_pierce_triangle(vertecies, center, width));
    }

    #[test]
    fn cube_corner_not_pierces_triangle_return_false() {
        let vertecies = [
            [8.0, -4.0, 5.0], 
            [3.0, 1.0, 5.0], 
            [4.0, -1.0, 9.0], 
        ];
        let center = [3.0, -4.0, 5.0];
        let width = 2.0;
        
        assert_eq!(cube_corner_pierce_triangle(vertecies, center, width), false);
    }

    #[test]
    fn triangle_through_cube_diagonal_returns_true() {
        let vertecies = [
            [8.0, -9.0, 0.0], 
            [-1.0, 1.0, 0.0], 
            [3.0, -4.0, 14.0], 
        ];
        let center = [3.0, -4.0, 5.0];
        let width = 2.0;
        
        assert!(cube_corner_pierce_triangle(vertecies, center, width));
    }
}
