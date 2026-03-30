fn verticies_in_cube(vertecies: [[f32; 3]; 3], cube_center: [f32; 3], cube_width: f32) -> bool {
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
}
