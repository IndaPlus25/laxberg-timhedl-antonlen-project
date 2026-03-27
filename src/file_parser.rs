use std::fs::File;
use std::io::{self, BufRead, BufReader, Error, ErrorKind};
use std::path::{Path};

pub struct Vertix{
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Face {
    pub v1: usize,
    pub v2: usize,
    pub v3: usize,
}

pub struct Mesh {
    pub name: String,
    pub vertices: Vec<Vertix>,
    pub faces: Vec<Face>,
}

trait FileFormat{
    fn parse_vertices(&self, coordinates: &str) -> Option<Vertix>;

    fn parse_faces(&self, vertices: &str)  -> Option<Vec<Face>>;
} 

struct ObjParser;

impl FileFormat for ObjParser {
    fn parse_vertices(&self, coordinates: &str) -> Option<Vertix> {
        let mut parts = coordinates.split_whitespace();

        let x = parts.next()?.parse::<f32>().ok()?;
        let y = parts.next()?.parse::<f32>().ok()?;
        let z = parts.next()?.parse::<f32>().ok()?;

        Some(Vertix { x, y, z })
    }

    fn parse_faces(&self, vertices: &str)  -> Option<Vec<Face>> {
        let mut parts = vertices.split_whitespace();

        let a = parts.next()?.split('/').next()?.parse::<usize>().ok()? - 1;
        let b = parts.next()?.split('/').next()?.parse::<usize>().ok()? - 1;
        let c = parts.next()?.split('/').next()?.parse::<usize>().ok()? - 1;

        let d_option = parts.next()
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.parse::<usize>().ok());

        if let Some(d) = d_option{
            let d = d - 1;
            return Some(vec![
                Face {v1: a, v2: b, v3: c},
                Face {v1: a, v2: c, v3: d},   
            ])
        }

        Some(vec![
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
            Some(Box::new(ObjParser))
        },
        _ => {None},
    }
}

fn parse_obj_file(filename: &str) -> io::Result<Vec<Mesh>> {
    let path = Path::new(filename);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let formatter = match get_file_format(path) {
        Some(f) => {f},
        None => {      
            return Err(Error::new(
                ErrorKind::Other, 
                "Placeholder: Unsupported file format!"
            ));
        }  
    };

    let mut vertices: Vec<Vertix> = vec![]; 
    let mut faces: Vec<Face> = vec![];
    let mut objects: Vec<Mesh> = vec![];

    for line_result in reader.lines(){
        let line = line_result?;
        let trimmed_line = line.trim();

        match trimmed_line.to_lowercase() {
            x if x.starts_with("f ") => {
                let raw_vertices = x[2..].trim();

                match formatter.parse_faces(raw_vertices) {
                    Some(mut f) => faces.append(&mut f),
                    None => {println!("Error: corrupted parse")} // PLACEHOLDER ERROR
                }
            },
            x if x.starts_with("v ") => {
                let coordinates = x[2..].trim();

                match formatter.parse_vertices(coordinates) {
                    Some(v) => vertices.push(v),
                    None => {println!("Error: corrupted parse")} // PLACEHOLDER ERROR
                }
            },
            x if x.starts_with("o ") => {
                let name = x[2..].to_string();
                objects.push(Mesh {name, vertices, faces,});

                (vertices, faces) = (vec![], vec![]);
            },
            _ => {},
        }

    }

    Ok(objects)
}

pub fn file_parse_interface(filename: &str) -> Option<Vec<Mesh>> {
    if !filename.ends_with(".obj"){
        return None;
    }

    match parse_obj_file(filename) { 
        Ok(mesh) => {
            println!("Successful mesh"); // PLACEHOLDER ERROR
            return Some(mesh);
        }
        Err(_) => {
            println!("not successful mesh"); // PLACEHOLDER ERROR
            return None;
        }
    }
}