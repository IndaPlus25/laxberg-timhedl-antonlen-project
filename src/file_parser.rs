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

fn parse_obj_file(filename: &str) -> io::Result<()> {
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

            },
            x if x.starts_with("v ") => {
                let coordinates = x[2..].trim();

                match Vertix::parse(coordinates) {
                    Some(v) => vertices.push(v),
                    None => {println!("Error: corrupted parse")} // PLACEHOLDER ERROR
                }
            },
            x if x.starts_with("o ") => {

            },
            _ => {},
        }

    }

    Ok(())
}