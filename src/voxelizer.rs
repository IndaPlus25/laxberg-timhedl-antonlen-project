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

fn vertecies_on_same_side(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
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
        
        assert!(vertecies_on_same_side(vertecies, center, width))
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
        
        assert!(vertecies_on_same_side(vertecies, center, width))
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
        
        assert_eq!(vertecies_on_same_side(vertecies, center, width), false)
    }
}
