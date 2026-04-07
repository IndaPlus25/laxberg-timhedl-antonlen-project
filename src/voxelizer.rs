/// Checks if any of a triangles vertecies are within a given box
fn verticies_in_cube(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    let min_x = cube_center[0] - cube_width;
    let max_x = cube_center[0] + cube_width;

    let min_y = cube_center[1] - cube_width;
    let max_y = cube_center[1] + cube_width;

    let min_z = cube_center[2] - cube_width;
    let max_z = cube_center[2] + cube_width;

    for vertex in vertecies {
        let vertex_x = vertex[0];
        let vertex_y = vertex[1];
        let vertex_z = vertex[2];

        if vertex_x > max_x || vertex_x < min_x {
            continue;
        }

        if vertex_y > max_y || vertex_y < min_y {
            continue;
        }

        if vertex_z > max_z || vertex_y < min_z {
            continue;
        }

        return true;
    };

    return false;
}

fn vertecies_outside_same_face(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    let min_x = cube_center[0] - cube_width;
    let max_x = cube_center[0] + cube_width;

    let min_y = cube_center[1] - cube_width;
    let max_y = cube_center[1] + cube_width;

    let min_z = cube_center[2] - cube_width;
    let max_z = cube_center[2] + cube_width;

    let min_directions = [min_x, min_y, min_z];
    let max_directions = [max_x, max_y, max_z];

    // Iterate over the x,y,z components of the vertecies and compare agains cube edges
    for direction in 0..3 {
        let vertex1_dir = vertecies[0][direction];
        let vertex2_dir = vertecies[1][direction];
        let vertex3_dir = vertecies[2][direction];

        let min_dir = min_directions[direction];
        if vertex1_dir < min_dir && vertex2_dir < min_dir && vertex3_dir < min_dir {
            return true;
        }

        let max_dir = max_directions[direction];
        if vertex1_dir > max_dir && vertex2_dir > max_dir && vertex3_dir > max_dir {
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
    todo!();
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
}
