use std::fs::File;
use std::io::{self, BufRead, BufReader, Error, ErrorKind};
use std::path::{Path};

#[derive(PartialEq, Debug, Clone)]
pub struct Vertex{
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Face {
    pub v1: usize,
    pub v2: usize,
    pub v3: usize,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
}

trait FileFormat{
    fn handle_input(&mut self, reader: &mut BufReader<File>) -> Result<Mesh, Error>;

    fn parse_line(&mut self, line_result: Result<String, Error>) -> io::Result<()>; 

    fn parse_vertices(&self, coordinates: &str) -> Result<Vertex, Error>;

    fn parse_faces(&self, vertices: &str)  -> Result<Vec<Face>, Error> ;
} 

struct ObjParser{
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
}

impl ObjParser {
    fn new() -> Self {
        Self {
            vertices: vec![],
            faces: vec![],
        }
    }

    fn parse_face_obj_format(part: Option<&str>) -> Result<usize, Error> {
        let Some(point_data) = part else {
            return Err(Error::new(ErrorKind::InvalidData, "Missing data"));
        };

        let Some(point_str) = point_data.split('/').next() else {
            return Err(Error::new(ErrorKind::InvalidData, "Missing point"));
        };

        let Ok(parsed_point) = point_str.parse::<usize>() else {
            return Err(Error::new(ErrorKind::InvalidData, format!("Parsing point failed, '{}' not valid input", point_str)))
        };

        let Some(point) = parsed_point.checked_sub(1) else {
            return Err(Error::new(ErrorKind::InvalidData, format!("Invalid index, point index is negative")))
        };

        Ok(point)
    }

    fn parse_vertex_obj_format(part: Option<&str>) -> Result<f32, Error> { 
        let Some(coordinate_str) = part else {
            return Err(Error::new(ErrorKind::InvalidData, "Missing coordinate"));
        };

        let Ok(coordinate) = coordinate_str.parse::<f32>() else {
            return Err(Error::new(ErrorKind::InvalidData, format!("Parsing coordinate failed, '{}' not valid input", coordinate_str)))
        };

        Ok(coordinate)
    }
}

impl FileFormat for ObjParser { 
    fn handle_input(&mut self, reader: &mut BufReader<File>) -> Result<Mesh, Error> {
        for (i, line_result) in reader.lines().enumerate(){
            match self.parse_line(line_result) {
                Ok(_) => {},
                Err(e) => {return Err(Error::new(e.kind(), format!("{}, failed on line {}", e, i + 1)));}
            }
        }  

        let mesh = Mesh {
            vertices: std::mem::take(&mut self.vertices),
            faces: std::mem::take(&mut self.faces),
        };

        Ok(mesh)
    }

    fn parse_line(&mut self, line_result: Result<String, Error>) -> Result<(), Error> {
        let line = line_result?;
        let trimmed_line = line.trim();

        match trimmed_line.to_lowercase() {
            x if x.starts_with("f ") => {
                let raw_vertices = x[2..].trim();

                match self.parse_faces(raw_vertices) {
                    Ok(mut f) => self.faces.append(&mut f),
                    Err(e) => {return Err(e);}
                }
            },
            x if x.starts_with("v ") => {
                let coordinates = x[2..].trim();

                match self.parse_vertices(coordinates) {
                    Ok(v) => self.vertices.push(v),
                    Err(e) => {return Err(e);}
                }
            },
            _ => {},
        }

        Ok(())
    }

    fn parse_vertices(&self, coordinates: &str) -> Result<Vertex, Error> {
        let mut parts = coordinates.split_whitespace();

        let x =  ObjParser::parse_vertex_obj_format(parts.next())?;
        let y = ObjParser::parse_vertex_obj_format(parts.next())?;
        let z = ObjParser::parse_vertex_obj_format(parts.next())?;

        Ok(Vertex { x, y, z })
    }

    fn parse_faces(&self, vertices: &str)  -> Result<Vec<Face>, Error> {
        let mut parts = vertices.split_whitespace();

        let a = ObjParser::parse_face_obj_format(parts.next())?;
        let b = ObjParser::parse_face_obj_format(parts.next())?;
        let c = ObjParser::parse_face_obj_format(parts.next())?;

        match ObjParser::parse_face_obj_format(parts.next()) {
            Ok(d) => {
                return Ok(vec![
                    Face {v1: a, v2: b, v3: c},
                    Face {v1: a, v2: c, v3: d},   
                ])
            }
            Err(_) => {}
        };

        Ok(vec![
            Face {v1: a, v2: b, v3: c}
        ])
    }

}

fn get_file_format(path: &Path) -> Option<Box<dyn FileFormat>>{
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext|ext.to_lowercase());
    
    match extension.as_deref() {
        Some("obj") => {
            Some(Box::new(ObjParser::new()))
        },
        _ => {None},
    }
}

fn parse_file(filename: &str) -> Result<Mesh, Error> {
    let path = Path::new(filename);

    let Ok(file) = File::open(&path) else {
        return Err(Error::new(ErrorKind::InvalidInput, "File does not exist"));
    };

    let mut reader = BufReader::new(file);

    let mut formatter = match get_file_format(path) {
        Some(f) => {f},
        None => {      
            return Err(Error::new(
                ErrorKind::Other, 
                "Placeholder: Unsupported file format!"
            ));
        },
    };

    let object = match formatter.handle_input(&mut reader){
        Ok(object) => {object},
        Err(e) => {return Err(e);}
    };

    Ok(object)

}

/// The interface for the file parser. 
/// 
/// Input: Takes a &str that is the filename that is going to be parsed, need to contain the fileformat. 
/// 
/// Output: Gives a result, either Error to handle or a Vec of Meshes. One Mesh is one object in the obj file. A Mesh contains a list of faces and vertecies that descirbe the object.
pub fn file_parse_interface(filename: &str) -> Result<Mesh, Error> {
    let mesh = parse_file(filename);
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verticies_finding_test(){
        let filename = "test-resources/file-parsing/correct-bugatti.obj";

        let mesh = file_parse_interface(filename).expect("Failed to parse file");

        let vertices = vec![
            // o alights.014_Plane.051
            Vertex { x: -6.070838, y: 1.759535, z: -26.802847 },
            Vertex { x: -5.678808, y: 5.600095, z: -26.429026 },
            Vertex { x: 3.241621, y: 0.874194, z: -27.473124 },
            Vertex { x: 3.633651, y: 4.714754, z: -27.099302 },
            
            // o alights.000_Plane.049
            Vertex { x: -18.872726, y: 1.170217, z: -5.146016 },
            Vertex { x: -18.331558, y: 5.010777, z: -5.169862 },
            Vertex { x: -12.906070, y: 0.284877, z: -12.327253 },
            Vertex { x: -12.364902, y: 4.125437, z: -12.351101 },
            
            // o alights.015_Plane.050
            Vertex { x: 18.468174, y: 7.681436, z: 20.833883 },
            Vertex { x: 19.072819, y: 11.192630, z: 19.663589 },
            Vertex { x: 19.423908, y: 7.429442, z: 20.571625 },
            Vertex { x: 20.028553, y: 10.940637, z: 19.401331 },
            Vertex { x: 15.553186, y: 8.450017, z: 21.633774 },
            Vertex { x: 16.157831, y: 11.961211, z: 20.463480 },
            Vertex { x: 16.508921, y: 8.198023, z: 21.371515 },
            Vertex { x: 17.113564, y: 11.709217, z: 20.201221 },
            Vertex { x: 12.638199, y: 9.218597, z: 22.433664 },
            Vertex { x: 13.242843, y: 12.729792, z: 21.263371 },
            Vertex { x: 13.593933, y: 8.966604, z: 22.171406 },
            Vertex { x: 14.198576, y: 12.477798, z: 21.001112 },
            Vertex { x: 9.723210, y: 9.987179, z: 23.233555 },
            Vertex { x: 10.327855, y: 13.498373, z: 22.063261 },
            Vertex { x: 10.678944, y: 9.735185, z: 22.971296 },
            Vertex { x: 11.283588, y: 13.246380, z: 21.801003 },
            Vertex { x: 6.808223, y: 10.755759, z: 24.033445 },
            Vertex { x: 7.412868, y: 14.266953, z: 22.863152 },
            Vertex { x: 7.763956, y: 10.503765, z: 23.771187 },
            Vertex { x: 8.368601, y: 14.014959, z: 22.600893 },
            Vertex { x: 3.893235, y: 11.524340, z: 24.833336 },
            Vertex { x: 4.497880, y: 15.035534, z: 23.663042 },
            Vertex { x: 4.848969, y: 11.272346, z: 24.571075 },
            Vertex { x: 5.453613, y: 14.783541, z: 23.400784 },
            Vertex { x: 0.978247, y: 12.292921, z: 25.633226 },
            Vertex { x: 1.582891, y: 15.804115, z: 24.462933 },
            Vertex { x: 1.933981, y: 12.040927, z: 25.370968 },
            Vertex { x: 2.538626, y: 15.552121, z: 24.200672 },

            // o Plane.047_Plane.042
            Vertex { x: -18.905333, y: 26.318699, z: -10.118521 },
            Vertex { x: -30.899866, y: 12.653121, z: -8.870247 },
            Vertex { x: -16.865187, y: 26.182327, z: 7.992091 },
            Vertex { x: -28.859720, y: 12.516749, z: 9.240364 },
        ];

        assert_eq!(vertices, mesh.vertices)
    }

    #[test]
    fn faces_finding_test(){
        let filename = "test-resources/file-parsing/correct-bugatti.obj";

        let mesh = file_parse_interface(filename).expect("Failed to parse file");

        let faces = vec![
            // o alights.014_Plane.051
            // f 1//1 2//1 4//1 3//1
            Face { v1: 0, v2: 1, v3: 3 },
            Face { v1: 0, v2: 3, v3: 2 },

            // o alights.000_Plane.049
            // f 5//2 6//2 8//2 7//2
            Face { v1: 4, v2: 5, v3: 7 },
            Face { v1: 4, v2: 7, v3: 6 },

            // o alights.015_Plane.050
            // f 9//3 10//3 12//3 11//3
            Face { v1: 8, v2: 9, v3: 11 },
            Face { v1: 8, v2: 11, v3: 10 },

            // f 13//3 14//3 16//3 15//3
            Face { v1: 12, v2: 13, v3: 15 },
            Face { v1: 12, v2: 15, v3: 14 },

            // f 17//3 18//3 20//3 19//3
            Face { v1: 16, v2: 17, v3: 19 },
            Face { v1: 16, v2: 19, v3: 18 },

            // f 21//3 22//3 24//3 23//3
            Face { v1: 20, v2: 21, v3: 23 },
            Face { v1: 20, v2: 23, v3: 22 },

            // f 25//3 26//3 28//3 27//3
            Face { v1: 24, v2: 25, v3: 27 },
            Face { v1: 24, v2: 27, v3: 26 },

            // f 29//3 30//3 32//3 31//3
            Face { v1: 28, v2: 29, v3: 31 },
            Face { v1: 28, v2: 31, v3: 30 },

            // f 33//3 34//3 36//3 35//3
            Face { v1: 32, v2: 33, v3: 35 },
            Face { v1: 32, v2: 35, v3: 34 },

            // o o Plane.047_Plane.042
            // f 37//4 38//4 40//4 39//4
            Face { v1: 36, v2: 37, v3: 39 },
            Face { v1: 36, v2: 39, v3: 38 },
        ];

        assert_eq!(faces, mesh.faces)
    }

#[test]
    fn complete_structure_test() {
        let filename = "test-resources/file-parsing/correct-bugatti.obj";
        
        let mesh = file_parse_interface(filename).expect("Failed to parse valid obj file");

        let expected_mesh = Mesh {
            vertices: vec![
                // From alights.014_plane.051
                Vertex { x: -6.070838, y: 1.759535, z: -26.802847 },
                Vertex { x: -5.678808, y: 5.600095, z: -26.429026 },
                Vertex { x: 3.241621, y: 0.874194, z: -27.473124 },
                Vertex { x: 3.633651, y: 4.714754, z: -27.099302 },
                // From alights.000_plane.049
                Vertex { x: -18.872726, y: 1.170217, z: -5.146016 },
                Vertex { x: -18.331558, y: 5.010777, z: -5.169862 },
                Vertex { x: -12.906070, y: 0.284877, z: -12.327253 },
                Vertex { x: -12.364902, y: 4.125437, z: -12.351101 },
                // From alights.015_plane.050
                Vertex { x: 18.468174, y: 7.681436, z: 20.833883 },
                Vertex { x: 19.072819, y: 11.192630, z: 19.663589 },
                Vertex { x: 19.423908, y: 7.429442, z: 20.571625 },
                Vertex { x: 20.028553, y: 10.940637, z: 19.401331 },
                Vertex { x: 15.553186, y: 8.450017, z: 21.633774 },
                Vertex { x: 16.157831, y: 11.961211, z: 20.463480 },
                Vertex { x: 16.508921, y: 8.198023, z: 21.371515 },
                Vertex { x: 17.113564, y: 11.709217, z: 20.201221 },
                Vertex { x: 12.638199, y: 9.218597, z: 22.433664 },
                Vertex { x: 13.242843, y: 12.729792, z: 21.263371 },
                Vertex { x: 13.593933, y: 8.966604, z: 22.171406 },
                Vertex { x: 14.198576, y: 12.477798, z: 21.001112 },
                Vertex { x: 9.723210, y: 9.987179, z: 23.233555 },
                Vertex { x: 10.327855, y: 13.498373, z: 22.063261 },
                Vertex { x: 10.678944, y: 9.735185, z: 22.971296 },
                Vertex { x: 11.283588, y: 13.246380, z: 21.801003 },
                Vertex { x: 6.808223, y: 10.755759, z: 24.033445 },
                Vertex { x: 7.412868, y: 14.266953, z: 22.863152 },
                Vertex { x: 7.763956, y: 10.503765, z: 23.771187 },
                Vertex { x: 8.368601, y: 14.014959, z: 22.600893 },
                Vertex { x: 3.893235, y: 11.524340, z: 24.833336 },
                Vertex { x: 4.497880, y: 15.035534, z: 23.663042 },
                Vertex { x: 4.848969, y: 11.272346, z: 24.571075 },
                Vertex { x: 5.453613, y: 14.783541, z: 23.400784 },
                Vertex { x: 0.978247, y: 12.292921, z: 25.633226 },
                Vertex { x: 1.582891, y: 15.804115, z: 24.462933 },
                Vertex { x: 1.933981, y: 12.040927, z: 25.370968 },
                Vertex { x: 2.538626, y: 15.552121, z: 24.200672 },
                // From plane.047_plane.042
                Vertex { x: -18.905333, y: 26.318699, z: -10.118521 },
                Vertex { x: -30.899866, y: 12.653121, z: -8.870247 },
                Vertex { x: -16.865187, y: 26.182327, z: 7.992091 },
                Vertex { x: -28.859720, y: 12.516749, z: 9.240364 },
            ],
            faces: vec![
                Face { v1: 0, v2: 1, v3: 3 },
                Face { v1: 0, v2: 3, v3: 2 },
                Face { v1: 4, v2: 5, v3: 7 },
                Face { v1: 4, v2: 7, v3: 6 },
                Face { v1: 8, v2: 9, v3: 11 },
                Face { v1: 8, v2: 11, v3: 10 },
                Face { v1: 12, v2: 13, v3: 15 },
                Face { v1: 12, v2: 15, v3: 14 },
                Face { v1: 16, v2: 17, v3: 19 },
                Face { v1: 16, v2: 19, v3: 18 },
                Face { v1: 20, v2: 21, v3: 23 },
                Face { v1: 20, v2: 23, v3: 22 },
                Face { v1: 24, v2: 25, v3: 27 },
                Face { v1: 24, v2: 27, v3: 26 },
                Face { v1: 28, v2: 29, v3: 31 },
                Face { v1: 28, v2: 31, v3: 30 },
                Face { v1: 32, v2: 33, v3: 35 },
                Face { v1: 32, v2: 35, v3: 34 },
                Face { v1: 36, v2: 37, v3: 39 },
                Face { v1: 36, v2: 39, v3: 38 },
            ],
        };

        assert_eq!(mesh, expected_mesh)
    }

    #[test]
    fn missing_coordinate_test() {
        let filename = "test-resources/file-parsing/bugatti-missing-coordinate-line-6.obj";

        let result: Result<Mesh, Error> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert_eq!(actual_err.kind(), ErrorKind::InvalidData);
        assert_eq!(actual_err.to_string(), "Missing coordinate, failed on line 6");
    }

    #[test]
    fn invalid_coordinate_test() {
        let filename = "test-resources/file-parsing/bugatti-invalid-coordinate-line-6.obj";

        let result: Result<Mesh, Error> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert_eq!(actual_err.kind(), ErrorKind::InvalidData);
        assert_eq!(actual_err.to_string(), "Parsing coordinate failed, 'apple' not valid input, failed on line 6");
    }

    #[test]
    fn missing_file_test() {
        let filename = "completed-project-with-no-errors.obj";

        let result: Result<Mesh, Error> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert_eq!(actual_err.kind(), ErrorKind::InvalidInput);
        assert_eq!(actual_err.to_string(), "File does not exist");
    }

    #[test]
    fn not_supported_fileformat_test() {
        let filename = "test-resources/file-parsing/invalid-fileformat.txt";

        let result: Result<Mesh, Error> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert_eq!(actual_err.kind(), ErrorKind::Other);
        assert_eq!(actual_err.to_string(), "Placeholder: Unsupported file format!");
    }

    #[test]
    fn missing_data_test() {
        let filename = "test-resources/file-parsing/bugatti-missing-data-line-21.obj";

        let result: Result<Mesh, Error> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert_eq!(actual_err.kind(), ErrorKind::InvalidData);
        assert_eq!(actual_err.to_string(), "Missing data, failed on line 21");
    }

    #[test]
    fn invalid_vertex_test() {
        let filename = "test-resources/file-parsing/bugatti-invalid-vertex-line-21.obj";

        let result: Result<Mesh, Error> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert_eq!(actual_err.kind(), ErrorKind::InvalidData);
        assert_eq!(actual_err.to_string(), "Parsing point failed, 'apple' not valid input, failed on line 21");
    }

    #[test]
    fn missing_point_test() {
        let filename = "test-resources/file-parsing/bugatti-invalid-index-line-21.obj";

        let result: Result<Mesh, Error> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert_eq!(actual_err.kind(), ErrorKind::InvalidData);
        assert_eq!(actual_err.to_string(), "Invalid index, point index is negative, failed on line 21");
    }
}
