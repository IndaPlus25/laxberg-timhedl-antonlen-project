use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use gltf::buffer::Data;
use gltf::image::Format;
use image::{DynamicImage, RgbImage, RgbaImage};
use gltf::{Gltf, Primitive};

const DEFAULT_COLOR: Vertex = Vertex {x: 1.0, y: 1.0, z: 1.0, u: 0.0, v: 0.0};

use crate::error::{FileParseError};

#[derive(PartialEq, Debug, Clone)]
pub struct Vertex{
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub u: f32,
    pub v: f32,
}

impl Vertex {
    fn to_bits(self) -> [u32; 3]{
        [self.x.to_bits(), self.y.to_bits(), self.z.to_bits()]
    }

    fn convert_to_real_color(&mut self) {
        let gamma: f32 = 2.2;
        let inverse_gamma = 1.0 / gamma;

        self.x = self.x.powf(inverse_gamma);
        self.y = self.y.powf(inverse_gamma);
        self.z = self.z.powf(inverse_gamma);
        self.u = self.u;
        self.v = self.v;
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Face {
    pub v1: usize,
    pub v2: usize,
    pub v3: usize,
    pub color_id: usize
}

#[derive(PartialEq, Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
    pub colors: Vec<Vertex>
}

#[derive(Debug)]
struct PaletteManager {
    colors: Vec<Vertex>,
    v_textures: Vec<(f32, f32)>,
    
    color_name_translator: HashMap<String, usize>,
    color_translator: HashMap<[u32; 3], usize>,
    palette_translator: HashMap<String, String>,
    palette_storer: HashMap<String, Option<RgbaImage>>,

    current_color: String,
    current_index: usize,

    folder: PathBuf
}

impl PaletteManager {
    fn new() -> Self {
        Self {
            colors: vec![DEFAULT_COLOR, DEFAULT_COLOR],
            v_textures: vec![(0.0, 0.0)],

            color_translator: HashMap::new(),
            color_name_translator: HashMap::new(),
            palette_translator: HashMap::new(),
            palette_storer: HashMap::new(),

            current_color: String::new(),
            current_index: 2,

            folder: PathBuf::new()
        }
    }

    fn add_material(&mut self, name: String, color: Vertex) {
        let parsed_color = color.clone().to_bits();

        self.color_name_translator.insert(name, self.current_index);
        self.color_translator.insert(parsed_color, self.current_index);
        self.colors.push(color);

        self.current_index += 1;
    }

    fn add_palette(&mut self, name: String, palette: String) {
        if self.palette_translator.get(&name).is_some() {
            return;
        }

        self.palette_translator.insert(name, palette.clone());

        if self.palette_storer.get(&palette).is_some(){
            return;
        }

        let palette_image = self.get_palette(palette.clone());
        self.palette_storer.insert(palette, palette_image);        
    }

    fn add_color(&mut self, color: Vertex) {
        let parsed_color = color.clone().to_bits();

        if self.color_translator.get(&parsed_color).is_some(){
            return;
        }

        self.color_translator.insert(parsed_color, self.current_index);
        self.colors.push(color);

        self.current_index += 1;
    }

    fn get_current_color(&self) -> Option<&usize>{
        self.color_name_translator.get(&self.current_color)
    }

    fn get_current_palette(&self) -> Option<&RgbaImage>{
        let palette_name = self.palette_translator.get(&self.current_color)?;
        self.palette_storer.get(palette_name)?.as_ref()
    }

    fn get_index_from_color(&self, color: Vertex) -> Option<&usize> {
        let formated_color = color.clone().to_bits();
        self.color_translator.get(&formated_color)
    }

    fn get_palette(&self, palette: String) -> Option<RgbaImage> {
        match self.palette_storer.get(&palette) {
            Some(cached) => cached.as_ref().cloned(),
            None => {
                let mut folder = self.folder.clone();
                folder.push(PathBuf::from(palette));

                match image::open(folder) {
                    Ok(img) => Some(img.into_rgba8()),
                    Err(_) => None,
                }
            }
        }
    }

    fn get_color_from_position(img: &RgbaImage, position: (f32, f32)) -> Vertex {
        let (width, height) = img.dimensions();

        let pixel_x = (position.0 * width as f32).floor() as u32;
        let pixel_y = ((1.0 - position.1) * height as f32).floor() as u32;

        let safe_x = pixel_x.clamp(0, width - 1);
        let safe_y = pixel_y.clamp(0, height - 1);

        let pixel = img.get_pixel(safe_x, safe_y);

        Vertex {
            x: pixel[0] as f32 / 255.0,
            y: pixel[1] as f32 / 255.0,
            z: pixel[2] as f32 / 255.0,
            u: 0.0,
            v: 0.0,
        }
    }
}

trait FileFormat{
    fn handle_input(&mut self, reader: &mut BufReader<File>, folder: Option<&Path>) -> Result<Mesh, FileParseError>;
} 

struct ObjParser{
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
    palette_manager: PaletteManager
}

impl ObjParser {
    fn new() -> Self {
        Self {
            vertices: vec![],
            faces: vec![],
            palette_manager: PaletteManager::new(),
        }
    }

    fn parse_line(&mut self, line_result: Result<String, Error>, folder: Option<&Path>) -> Result<(), FileParseError> {
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
            x if x.starts_with("vt ") => {
                let coordinates = x[3..].trim();

                match ObjParser::parse_v_texture(coordinates) {
                    Ok(v) => self.palette_manager.v_textures.push(v),
                    Err(e) => {return Err(e);}
                }
            },
            x if x.starts_with("mtllib ") => {
                let color_scheme_path = trimmed_line[7..].trim();

                let full_path = match folder{
                    Some(folder_path) => folder_path.join(color_scheme_path),
                    None => PathBuf::from(color_scheme_path),
                };

                let color_file = match File::open(full_path){
                    Ok(file) => file,
                    Err(_) => return Ok(()),
                };

                self.parse_color_file(&color_file)?;
            },
            x if x.starts_with("usemtl ") => {
                self.palette_manager.current_color = x[7..].trim().to_string();
            }
            _ => {},
        }

        Ok(())
    }

    fn parse_vertices(&self, coordinates: &str) -> Result<Vertex, FileParseError> {
        let mut parts = coordinates.split_whitespace();

        let x =  ObjParser::parse_vertex_obj_format(parts.next())?;
        let y = ObjParser::parse_vertex_obj_format(parts.next())?;
        let z = ObjParser::parse_vertex_obj_format(parts.next())?;

        Ok(Vertex { x, y, z, u: 0.0, v: 0.0 })
    }

    fn parse_faces(&mut self, vertices: &str)  -> Result<Vec<Face>, FileParseError> {
        let mut parts = vertices.split_whitespace();

        let (a, texture_a) = ObjParser::parse_face_obj_format(parts.next())?;
        let (b, texture_b) = ObjParser::parse_face_obj_format(parts.next())?;
        let (c, texture_c) = ObjParser::parse_face_obj_format(parts.next())?;

        let (d, texture_d) = match ObjParser::parse_face_obj_format(parts.next()) {
            Ok((d, texture_d)) => (Some(d), texture_d),
            Err(_) => (None, None),
        };
        let first_some = texture_a.or(texture_b).or(texture_c).or(texture_d);

        let mut color_id: usize = self.palette_manager.get_current_color().copied().unwrap_or(1);

        if let (Some(point), Some(palette_image)) = (first_some, self.palette_manager.get_current_palette()){
            let position = self.palette_manager.v_textures[point];

            let color = PaletteManager::get_color_from_position(&palette_image, position);
            self.palette_manager.add_color(color.clone());

            color_id = self.palette_manager.get_index_from_color(color).unwrap_or(&1).to_owned();
        }

        if let Some(d) = d {
            Ok(vec![
                Face { v1: a, v2: b, v3: c, color_id },
                Face { v1: a, v2: c, v3: d, color_id },   
            ])
        } else {
            Ok(vec![
                Face { v1: a, v2: b, v3: c, color_id }
            ])
        }
    }

    fn parse_face_obj_format(part: Option<&str>) -> Result<(usize, Option<usize>), FileParseError> {
        let Some(point_data) = part else {
            return Err(FileParseError::MissingData);
        };

        let mut parts = point_data.split('/');

        let Some(point_str) = parts.next() else {
            return Err(FileParseError::MissingPoint);
        };

        let texture = parts.next().and_then(|s| s.parse::<usize>().ok());

        let Ok(parsed_point) = point_str.parse::<usize>() else {
            return Err(FileParseError::InvalidDataType(point_str.to_string()));
        };

        let Some(point) = parsed_point.checked_sub(1) else {
            return Err(FileParseError::DataOutOfBounds(parsed_point));
        };

        Ok((point, texture))
    }

    fn parse_vertex_obj_format(part: Option<&str>) -> Result<f32, FileParseError> { 
        let Some(coordinate_str) = part else {
            return Err(FileParseError::MissingCoordinate);
        };

        let Ok(coordinate) = coordinate_str.parse::<f32>() else {
            return Err(FileParseError::InvalidDataType(coordinate_str.to_string()));
        };

        Ok(coordinate)
    }

    fn parse_color_file(&mut self, file: &File) -> Result<(), FileParseError>{
        let reader = BufReader::new(file);

        let mut current_material = String::new();

        let gamma: f32 = 2.2;
        let inverse_gamma = 1.0 / gamma;

        for line_result in reader.lines() {
            let line = line_result?;
            
            match line.to_lowercase() {
                x if x.starts_with("newmtl ") => current_material = x[7..].trim().to_owned(),
                x if x.starts_with("kd ") => {
                    let color = x[3..].trim();

                    let vertex = self.parse_vertices(color)?;
                    let parsed_color = Vertex {
                        x: vertex.x.powf(inverse_gamma),
                        y: vertex.y.powf(inverse_gamma),
                        z: vertex.z.powf(inverse_gamma),
                        u: 0.0,
                        v: 0.0,
                    };

                    self.palette_manager.add_material(current_material.clone(), parsed_color);
                },
                x if x.starts_with("map_kd ") => {
                    let palette = line[7..].trim();

                    self.palette_manager.add_palette(current_material.clone(), palette.to_string());
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn parse_v_texture(coordinates: &str) -> Result<(f32, f32), FileParseError> {
        let mut parts = coordinates.split_whitespace();

        let x =  ObjParser::parse_vertex_obj_format(parts.next())?;
        let y = ObjParser::parse_vertex_obj_format(parts.next())?;

        Ok((x, y))
    }
}

impl FileFormat for ObjParser { 
    fn handle_input(&mut self, reader: &mut BufReader<File>, folder: Option<&Path>) -> Result<Mesh, FileParseError> {
        if let Some(folder) = folder {
            self.palette_manager.folder = folder.to_path_buf()
        }        
        
        for (i, line_result) in reader.lines().enumerate(){
            match self.parse_line(line_result, folder) {
                Ok(_) => {},
                Err(error) => {return Err(FileParseError::FailedLineParse(i, Box::new(error)));}
            }
        }  
        let mesh = Mesh {
            vertices: std::mem::take(&mut self.vertices),
            faces: std::mem::take(&mut self.faces),
            colors: std::mem::take(&mut self.palette_manager.colors),
        };

        Ok(mesh)
    }
}

struct GlbParser{
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
    palette_manager: PaletteManager    
}

impl GlbParser {
    fn new() -> Self {
        Self {
            vertices: vec![],
            faces: vec![],
            palette_manager: PaletteManager::new(),
        }
    }   

    fn parse_glb(reader: &mut BufReader<File>) -> Result<(gltf::Document, Vec<gltf::buffer::Data>, Vec<gltf::image::Data>), FileParseError> {
        let mut file_bytes = Vec::new();  
        reader.read_to_end(&mut file_bytes)?;  

        Ok(gltf::import_slice(&file_bytes)?)
    } 

    fn parse_mesh(&mut self, primitive: Primitive<'_>, buffers: &Vec<Data>, images: &Vec<gltf::image::Data>) {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));        
    
        let vertex_positions_op = reader.read_positions();
        let uv_values_op = reader.read_tex_coords(0).map(|v| v.into_f32());

        let mut current_image_index = None;

        let pbr = primitive.material().pbr_metallic_roughness();        
        if let Some(texture_info) = pbr.base_color_texture() {
            current_image_index = Some(texture_info.texture().source().index());
        }

        let vertex_offset = self.vertices.len();

        let (vertex_positions, uv_values): (Vec<[f32; 3]>, Vec<[f32; 2]>) = match (vertex_positions_op, uv_values_op) {
            (Some(vertecies), None) => {
                let vertices_vec: Vec<[f32; 3]> = vertecies.collect();
                let target_len = vertices_vec.len();

                (vertices_vec, vec![[0.0, 0.0]; target_len])
            },
            (Some(vertecies), Some(uvs)) => {
                let vertices_vec: Vec<[f32; 3]> = vertecies.collect();
                let mut uvs_vec: Vec<[f32; 2]> = uvs.collect();

                uvs_vec.resize(vertices_vec.len(), [-1.0, -1.0]);

                (vertices_vec, uvs_vec)
            },
            _ => {return;},
        };

        for (vertex_value, uv_value) in vertex_positions.iter().zip(uv_values.iter()){
            let vertex = Vertex { 
                x: vertex_value[0], 
                y: vertex_value[1], 
                z: vertex_value[2], 
                u: uv_value[0], 
                v: uv_value[1], 
            };

            self.vertices.push(vertex);
        }

        if let Some(index_iter) = reader.read_indices() {
            let indices: Vec<u32> = index_iter.into_u32().collect();

            for chunk in indices.chunks_exact(3) {
                let v1 = chunk[0] as usize + vertex_offset;
                let v2 = chunk[1] as usize + vertex_offset;
                let v3 = chunk[2] as usize + vertex_offset;

                let color_id = self.find_color((v1, v2, v3), current_image_index, images);
                let triangle = Face {v1, v2, v3, color_id};

                self.faces.push(triangle);
            }
        }

    }

    fn find_color(&mut self, vertecies: (usize, usize, usize), image_index: Option<usize>, images: &Vec<gltf::image::Data>) -> usize{
        let image_data = match image_index{
            Some(index) => &images[index],
            None => return 1,
        };

        let rgba_image = match GlbParser::convert_gltf_to_rgba(image_data){
            Some(image) => image,
            None => return 1,
        };

        let parsed_vertecies = [self.vertices[vertecies.0].clone(), self.vertices[vertecies.1].clone(), self.vertices[vertecies.2].clone()];
        let mut color_id: Option<usize> = None;

        for vertex in parsed_vertecies{
            if vertex.u < 0.0 || vertex.v < 0.0 {
                continue;
            }

            let u_wrapped = vertex.u.rem_euclid(1.0);
            let v_wrapped = vertex.v.rem_euclid(1.0);   

            let mut color = PaletteManager::get_color_from_position(&rgba_image, (u_wrapped, v_wrapped));
            color.convert_to_real_color();

            self.palette_manager.add_color(color.clone());
            color_id = self.palette_manager.get_index_from_color(color).copied();

            if color_id.is_some() {
                break;
            }
        }

        color_id.unwrap_or(1)
    }

    fn convert_gltf_to_rgba(data: &gltf::image::Data) -> Option<RgbaImage> {
        let width = data.width;
        let height = data.height;
        
        let pixels = data.pixels.clone(); 

        match data.format {
            Format::R8G8B8A8 => {
                RgbaImage::from_raw(width, height, pixels)
            }
            Format::R8G8B8 => {
                let rgb_img = RgbImage::from_raw(width, height, pixels)?;
                
                Some(DynamicImage::ImageRgb8(rgb_img).into_rgba8())
            }
            _ => None
        }
    }

    fn parse_node(&mut self, node: gltf::Node, buffers: &Vec<Data>, images: &Vec<RgbaImage>) {
        todo!()
    }
}

impl FileFormat for GlbParser {
    fn handle_input(&mut self, reader: &mut BufReader<File>, folder: Option<&Path>) -> Result<Mesh, FileParseError> {
        let (document, buffers, images) = GlbParser::parse_glb(reader)?;
        
        let mut rgb_images: Vec<RgbaImage> = Vec::new(); 

        for image in images{
            let rgb_image = match GlbParser::convert_gltf_to_rgba(&image) {
                Some(image) => image,
                None => RgbaImage::new(4, 4),
            };

            rgb_images.push(rgb_image);
        }

        if let Some(scene) = document.default_scene(){
            for node in scene.nodes(){
                self.parse_node(node, &buffers, &rgb_images);
            }
        }

        let mesh = Mesh {
            vertices: std::mem::take(&mut self.vertices),
            faces: std::mem::take(&mut self.faces),
            colors: std::mem::take(&mut self.palette_manager.colors),
        };

        Ok(mesh)
    }
}

fn get_file_format(path: &Path) -> Result<Box<dyn FileFormat>, FileParseError>{
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext|ext.to_lowercase());
    
    match extension.as_deref() {
        Some("obj") => {
            Ok(Box::new(ObjParser::new()))
        },
        Some("glb") => {
            Ok(Box::new(GlbParser::new()))
        },
        _ => {Err(FileParseError::NotSupportedFileFormat(extension))},
    }
}

fn parse_file(filename: &str) -> Result<Mesh, FileParseError> {
    let path = Path::new(filename);
    let folder = path.parent();

    let file = File::open(&path)?;

    let mut reader = BufReader::new(file);

    let mut formatter =  get_file_format(path)?;

    let object = match formatter.handle_input(&mut reader, folder){
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
pub fn file_parse_interface(filename: &str) -> Result<Mesh, FileParseError> {
    parse_file(filename)
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
            Vertex { x: -6.070838, y: 1.759535, z: -26.802847, u: 0.0, v: 0.0},
            Vertex { x: -5.678808, y: 5.600095, z: -26.429026, u: 0.0, v: 0.0},
            Vertex { x: 3.241621, y: 0.874194, z: -27.473124, u: 0.0, v: 0.0 },
            Vertex { x: 3.633651, y: 4.714754, z: -27.099302, u: 0.0, v: 0.0 },
            
            // o alights.000_Plane.049
            Vertex { x: -18.872726, y: 1.170217, z: -5.146016, u: 0.0, v: 0.0 },
            Vertex { x: -18.331558, y: 5.010777, z: -5.169862, u: 0.0, v: 0.0 },
            Vertex { x: -12.906070, y: 0.284877, z: -12.327253, u: 0.0, v: 0.0 },
            Vertex { x: -12.364902, y: 4.125437, z: -12.351101, u: 0.0, v: 0.0 },
            
            // o alights.015_Plane.050
            Vertex { x: 18.468174, y: 7.681436, z: 20.833883, u: 0.0, v: 0.0},
            Vertex { x: 19.072819, y: 11.192630, z: 19.663589, u: 0.0, v: 0.0 },
            Vertex { x: 19.423908, y: 7.429442, z: 20.571625, u: 0.0, v: 0.0},
            Vertex { x: 20.028553, y: 10.940637, z: 19.401331, u: 0.0, v: 0.0 },
            Vertex { x: 15.553186, y: 8.450017, z: 21.633774, u: 0.0, v: 0.0},
            Vertex { x: 16.157831, y: 11.961211, z: 20.463480, u: 0.0, v: 0.0 },
            Vertex { x: 16.508921, y: 8.198023, z: 21.371515, u: 0.0, v: 0.0},
            Vertex { x: 17.113564, y: 11.709217, z: 20.201221, u: 0.0, v: 0.0 },
            Vertex { x: 12.638199, y: 9.218597, z: 22.433664, u: 0.0, v: 0.0},
            Vertex { x: 13.242843, y: 12.729792, z: 21.263371, u: 0.0, v: 0.0 },
            Vertex { x: 13.593933, y: 8.966604, z: 22.171406, u: 0.0, v: 0.0},
            Vertex { x: 14.198576, y: 12.477798, z: 21.001112, u: 0.0, v: 0.0 },
            Vertex { x: 9.723210, y: 9.987179, z: 23.233555, u: 0.0, v: 0.0 },
            Vertex { x: 10.327855, y: 13.498373, z: 22.063261, u: 0.0, v: 0.0 },
            Vertex { x: 10.678944, y: 9.735185, z: 22.971296, u: 0.0, v: 0.0 },
            Vertex { x: 11.283588, y: 13.246380, z: 21.801003, u: 0.0, v: 0.0 },
            Vertex { x: 6.808223, y: 10.755759, z: 24.033445, u: 0.0, v: 0.0 },
            Vertex { x: 7.412868, y: 14.266953, z: 22.863152, u: 0.0, v: 0.0 },
            Vertex { x: 7.763956, y: 10.503765, z: 23.771187, u: 0.0, v: 0.0 },
            Vertex { x: 8.368601, y: 14.014959, z: 22.600893, u: 0.0, v: 0.0 },
            Vertex { x: 3.893235, y: 11.524340, z: 24.833336, u: 0.0, v: 0.0 },
            Vertex { x: 4.497880, y: 15.035534, z: 23.663042, u: 0.0, v: 0.0 },
            Vertex { x: 4.848969, y: 11.272346, z: 24.571075, u: 0.0, v: 0.0 },
            Vertex { x: 5.453613, y: 14.783541, z: 23.400784, u: 0.0, v: 0.0 },
            Vertex { x: 0.978247, y: 12.292921, z: 25.633226, u: 0.0, v: 0.0 },
            Vertex { x: 1.582891, y: 15.804115, z: 24.462933, u: 0.0, v: 0.0 },
            Vertex { x: 1.933981, y: 12.040927, z: 25.370968, u: 0.0, v: 0.0 },
            Vertex { x: 2.538626, y: 15.552121, z: 24.200672, u: 0.0, v: 0.0 },

            // o Plane.047_Plane.042
            Vertex { x: -18.905333, y: 26.318699, z: -10.118521, u: 0.0, v: 0.0 },
            Vertex { x: -30.899866, y: 12.653121, z: -8.870247, u: 0.0, v: 0.0 },
            Vertex { x: -16.865187, y: 26.182327, z: 7.992091, u: 0.0, v: 0.0 },
            Vertex { x: -28.859720, y: 12.516749, z: 9.240364, u: 0.0, v: 0.0 },
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
            Face { v1: 0, v2: 1, v3: 3, color_id: 1},
            Face { v1: 0, v2: 3, v3: 2, color_id: 1},

            // o alights.000_Plane.049
            // f 5//2 6//2 8//2 7//2
            Face { v1: 4, v2: 5, v3: 7, color_id: 1},
            Face { v1: 4, v2: 7, v3: 6, color_id: 1},

            // o alights.015_Plane.050
            // f 9//3 10//3 12//3 11//3
            Face { v1: 8, v2: 9, v3: 11, color_id: 1},
            Face { v1: 8, v2: 11, v3: 10, color_id: 1},

            // f 13//3 14//3 16//3 15//3
            Face { v1: 12, v2: 13, v3: 15, color_id: 1},
            Face { v1: 12, v2: 15, v3: 14, color_id: 1},

            // f 17//3 18//3 20//3 19//3
            Face { v1: 16, v2: 17, v3: 19, color_id: 1},
            Face { v1: 16, v2: 19, v3: 18, color_id: 1},

            // f 21//3 22//3 24//3 23//3
            Face { v1: 20, v2: 21, v3: 23, color_id: 1},
            Face { v1: 20, v2: 23, v3: 22, color_id: 1},

            // f 25//3 26//3 28//3 27//3
            Face { v1: 24, v2: 25, v3: 27, color_id: 1},
            Face { v1: 24, v2: 27, v3: 26, color_id: 1},

            // f 29//3 30//3 32//3 31//3
            Face { v1: 28, v2: 29, v3: 31, color_id: 1},
            Face { v1: 28, v2: 31, v3: 30, color_id: 1},

            // f 33//3 34//3 36//3 35//3
            Face { v1: 32, v2: 33, v3: 35, color_id: 1},
            Face { v1: 32, v2: 35, v3: 34, color_id: 1},

            // o o Plane.047_Plane.042
            // f 37//4 38//4 40//4 39//4
            Face { v1: 36, v2: 37, v3: 39, color_id: 1},
            Face { v1: 36, v2: 39, v3: 38, color_id: 1},
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
                Vertex { x: -6.070838, y: 1.759535, z: -26.802847, u: 0.0, v: 0.0 },
                Vertex { x: -5.678808, y: 5.600095, z: -26.429026, u: 0.0, v: 0.0 },
                Vertex { x: 3.241621, y: 0.874194, z: -27.473124, u: 0.0, v: 0.0 },
                Vertex { x: 3.633651, y: 4.714754, z: -27.099302, u: 0.0, v: 0.0 },
                // From alights.000_plane.049
                Vertex { x: -18.872726, y: 1.170217, z: -5.146016, u: 0.0, v: 0.0 },
                Vertex { x: -18.331558, y: 5.010777, z: -5.169862, u: 0.0, v: 0.0 },
                Vertex { x: -12.906070, y: 0.284877, z: -12.327253, u: 0.0, v: 0.0 },
                Vertex { x: -12.364902, y: 4.125437, z: -12.351101, u: 0.0, v: 0.0 },
                // From alights.015_plane.050
                Vertex { x: 18.468174, y: 7.681436, z: 20.833883, u: 0.0, v: 0.0 },
                Vertex { x: 19.072819, y: 11.192630, z: 19.663589, u: 0.0, v: 0.0 },
                Vertex { x: 19.423908, y: 7.429442, z: 20.571625, u: 0.0, v: 0.0 },
                Vertex { x: 20.028553, y: 10.940637, z: 19.401331, u: 0.0, v: 0.0 },
                Vertex { x: 15.553186, y: 8.450017, z: 21.633774, u: 0.0, v: 0.0 },
                Vertex { x: 16.157831, y: 11.961211, z: 20.463480, u: 0.0, v: 0.0 },
                Vertex { x: 16.508921, y: 8.198023, z: 21.371515, u: 0.0, v: 0.0 },
                Vertex { x: 17.113564, y: 11.709217, z: 20.201221, u: 0.0, v: 0.0 },
                Vertex { x: 12.638199, y: 9.218597, z: 22.433664, u: 0.0, v: 0.0 },
                Vertex { x: 13.242843, y: 12.729792, z: 21.263371, u: 0.0, v: 0.0 },
                Vertex { x: 13.593933, y: 8.966604, z: 22.171406, u: 0.0, v: 0.0 },
                Vertex { x: 14.198576, y: 12.477798, z: 21.001112, u: 0.0, v: 0.0 },
                Vertex { x: 9.723210, y: 9.987179, z: 23.233555, u: 0.0, v: 0.0 },
                Vertex { x: 10.327855, y: 13.498373, z: 22.063261, u: 0.0, v: 0.0 },
                Vertex { x: 10.678944, y: 9.735185, z: 22.971296, u: 0.0, v: 0.0 },
                Vertex { x: 11.283588, y: 13.246380, z: 21.801003, u: 0.0, v: 0.0 },
                Vertex { x: 6.808223, y: 10.755759, z: 24.033445, u: 0.0, v: 0.0 },
                Vertex { x: 7.412868, y: 14.266953, z: 22.863152, u: 0.0, v: 0.0 },
                Vertex { x: 7.763956, y: 10.503765, z: 23.771187, u: 0.0, v: 0.0 },
                Vertex { x: 8.368601, y: 14.014959, z: 22.600893, u: 0.0, v: 0.0 },
                Vertex { x: 3.893235, y: 11.524340, z: 24.833336, u: 0.0, v: 0.0 },
                Vertex { x: 4.497880, y: 15.035534, z: 23.663042, u: 0.0, v: 0.0 },
                Vertex { x: 4.848969, y: 11.272346, z: 24.571075, u: 0.0, v: 0.0 },
                Vertex { x: 5.453613, y: 14.783541, z: 23.400784, u: 0.0, v: 0.0 },
                Vertex { x: 0.978247, y: 12.292921, z: 25.633226, u: 0.0, v: 0.0 },
                Vertex { x: 1.582891, y: 15.804115, z: 24.462933, u: 0.0, v: 0.0 },
                Vertex { x: 1.933981, y: 12.040927, z: 25.370968, u: 0.0, v: 0.0 },
                Vertex { x: 2.538626, y: 15.552121, z: 24.200672, u: 0.0, v: 0.0 },
                // From plane.047_plane.042
                Vertex { x: -18.905333, y: 26.318699, z: -10.118521, u: 0.0, v: 0.0 },
                Vertex { x: -30.899866, y: 12.653121, z: -8.870247, u: 0.0, v: 0.0 },
                Vertex { x: -16.865187, y: 26.182327, z: 7.992091, u: 0.0, v: 0.0 },
                Vertex { x: -28.859720, y: 12.516749, z: 9.240364, u: 0.0, v: 0.0 },
            ],
            faces: vec![
                Face { v1: 0, v2: 1, v3: 3, color_id: 1},
                Face { v1: 0, v2: 3, v3: 2, color_id: 1},
                Face { v1: 4, v2: 5, v3: 7, color_id: 1},
                Face { v1: 4, v2: 7, v3: 6, color_id: 1},
                Face { v1: 8, v2: 9, v3: 11, color_id: 1 },
                Face { v1: 8, v2: 11, v3: 10, color_id: 1},
                Face { v1: 12, v2: 13, v3: 15, color_id: 1},
                Face { v1: 12, v2: 15, v3: 14, color_id: 1},
                Face { v1: 16, v2: 17, v3: 19, color_id: 1},
                Face { v1: 16, v2: 19, v3: 18, color_id: 1},
                Face { v1: 20, v2: 21, v3: 23, color_id: 1},
                Face { v1: 20, v2: 23, v3: 22, color_id: 1},
                Face { v1: 24, v2: 25, v3: 27, color_id: 1},
                Face { v1: 24, v2: 27, v3: 26, color_id: 1},
                Face { v1: 28, v2: 29, v3: 31, color_id: 1},
                Face { v1: 28, v2: 31, v3: 30, color_id: 1},
                Face { v1: 32, v2: 33, v3: 35, color_id: 1},
                Face { v1: 32, v2: 35, v3: 34, color_id: 1},
                Face { v1: 36, v2: 37, v3: 39, color_id: 1},
                Face { v1: 36, v2: 39, v3: 38, color_id: 1},
            ],
            colors: vec![DEFAULT_COLOR, DEFAULT_COLOR],
        };

        assert_eq!(mesh, expected_mesh)
    }

    #[test]
    fn missing_coordinate_test() {
        let filename = "test-resources/file-parsing/bugatti-missing-coordinate-line-6.obj";

        let result: Result<Mesh, FileParseError> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert!(
            matches!(
                actual_err, 
                FileParseError::FailedLineParse(5, inner_error) 
                    if matches!(*inner_error, FileParseError::MissingCoordinate)
            )
        );
    }

    #[test]
    fn invalid_coordinate_test() {
        let filename = "test-resources/file-parsing/bugatti-invalid-coordinate-line-6.obj";

        let result: Result<Mesh, FileParseError> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert!(
            matches!(
                actual_err, 
                FileParseError::FailedLineParse(5, inner_error) 
                    if matches!(*inner_error, FileParseError::InvalidDataType(ref s) if s == "apple")
            )
        );
    }

    #[test]
    fn missing_file_test() {
        let filename = "completed-project-with-no-errors.obj";

        let result: Result<Mesh, FileParseError> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert!(
            matches!(
                actual_err, 
                FileParseError::IoError(err) if err.kind() == ErrorKind::NotFound
            )
        );
    }

    #[test]
    fn not_supported_fileformat_test() {
        let filename = "test-resources/file-parsing/invalid-fileformat.txt";

        let result: Result<Mesh, FileParseError> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert!(
            matches!(
                actual_err, 
                FileParseError::NotSupportedFileFormat(option) if option == Some("txt".to_string())
            )
        );
    }

    #[test]
    fn missing_data_test() {
        let filename = "test-resources/file-parsing/bugatti-missing-data-line-21.obj";

        let result: Result<Mesh, FileParseError> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert!(
            matches!(
                actual_err, 
                FileParseError::FailedLineParse(20, inner_error) 
                    if matches!(*inner_error, FileParseError::MissingData)
            )
        );
    }

    #[test]
    fn invalid_vertex_test() {
        let filename = "test-resources/file-parsing/bugatti-invalid-vertex-line-21.obj";

        let result: Result<Mesh, FileParseError> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();


        assert!(
            matches!(
                actual_err, 
                FileParseError::FailedLineParse(20, inner_error) 
                    if matches!(*inner_error, FileParseError::InvalidDataType(ref s) if s == "apple")
            )
        );
    }

    #[test]
    fn missing_point_test() {
        let filename = "test-resources/file-parsing/bugatti-invalid-index-line-21.obj";

        let result: Result<Mesh, FileParseError> = file_parse_interface(filename);
        let actual_err = result.unwrap_err();

        assert!(
            matches!(
                actual_err, 
                FileParseError::FailedLineParse(20, inner_error) 
                    if matches!(*inner_error, FileParseError::DataOutOfBounds(0))
            )
        );
    }

}