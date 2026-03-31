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
    let min_x = cube_center[0] - cube_width;
    let max_x = cube_center[0] + cube_width;

    let min_y = cube_center[1] - cube_width;
    let max_y = cube_center[1] + cube_width;

    let min_z = cube_center[2] - cube_width;
    let max_z = cube_center[2] + cube_width;

    // Array of possible faces, remains true if all vertecies are outside it
    let mut outsides = [true; 12];

    for vertex in vertecies {
        let x = vertex[0];
        let y = vertex[1];
        let z = vertex[2];

        // Planes independent of the z-axis
        if x+y <= max_x + max_y {
            outsides[0] = false;
        }
        if x-y <= max_x + min_y {
            outsides[1] = false;
        }
        if -x+y <= min_x + max_y{
            outsides[2] = false;
        }
        if -x-y <= -1.0 * (min_x + min_y) {
            outsides[3] = false;
        }

        // Planes independent of the x-axis
        if y+z <= max_y + max_z {
            outsides[4] = false;
        }
        if y-z <= max_y + min_z {
            outsides[5] = false;
        }
        if -y+z <= min_y + max_z {
            outsides[6] = false;
        }
        if -y-z <= -1.0 * (min_y + min_z) {
            outsides[7] = false;
        }

        // Planes independent of the y-axis
        if x+z <= max_x + max_z {
            outsides[8] = false;
        }
        if x-z <= max_x + min_z {
            outsides[9] = false;
        }
        if -x+z <= min_x + max_z {
            outsides[10] = false;
        }
        if -x-z <= -1.0 * (min_x + min_z) {
            outsides[11] = false;
        }
        
    }

    if outsides.iter().any(|x| *x == true ) {
        return true;
    }

    return false;
}

fn vertecies_outside_same_corner(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
    todo!();
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
}
