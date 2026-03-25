pub struct Vertix{
    x: f32,
    y: f32,
    z: f32,
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

pub fn file_parse_interface(filename: String){

}

fn parse_obj_file(filepath: &str){

}