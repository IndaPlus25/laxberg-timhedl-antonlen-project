use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub struct Vertix{
    x: f32,
    y: f32,
    z: f32,
}

impl Vertix {
    fn parse(coordinates: &str) -> Option<Vertix> {
        let mut parts = coordinates.split_whitespace();

        let x = parts.next()?.parse::<f32>().ok()?;
        let y = parts.next()?.parse::<f32>().ok()?;
        let z = parts.next()?.parse::<f32>().ok()?;

        Some(Vertix { x, y, z })
    }
}

pub struct Face {
    v1: usize,
    v2: usize,
    v3: usize,
}

impl Face {
    fn parse(vertices: &str)  -> Option<Vec<Face>> {
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

pub struct Mesh {
    name: String,
    vertices: Vec<Vertix>,
    faces: Vec<Face>,
}

pub fn file_parse_interface(filename: &str){
    if !filename.ends_with(".obj"){
        return;
    }

    let result = parse_obj_file(filename);
}

fn parse_obj_file(filename: &str) -> io::Result<Vec<Mesh>> {
    let path = Path::new(filename);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut vertices: Vec<Vertix> = vec![]; 
    let mut faces: Vec<Face> = vec![];
    let mut objects: Vec<Mesh> = vec![];

    for line_result in reader.lines(){
        let line = line_result?;
        let trimmed_line = line.trim();

        match trimmed_line.to_lowercase() {
            x if x.starts_with("f ") => {
                let raw_vertices = x[2..].trim();

                match Face::parse(raw_vertices) {
                    Some(mut f) => faces.append(&mut f),
                    None => {println!("Error: corrupted parse")} // PLACEHOLDER ERROR
                }
            },
            x if x.starts_with("v ") => {
                let coordinates = x[2..].trim();

                match Vertix::parse(coordinates) {
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