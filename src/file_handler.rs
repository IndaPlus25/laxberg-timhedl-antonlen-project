use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::vecmath::*;
use crate::octree::*;
use crate::error::*;

#[derive(Serialize)]
struct FormatedChunkRef<'a> {
    index: V3i,
    data: &'a Vec<u32>,
    min_pos: V3,
    max_pos: V3,
}

#[derive(Deserialize)]
struct FormatedChunk {
    index: V3i,
    data: Vec<u32>,
    min_pos: V3,
    max_pos: V3,
}

/// this is the interface to interact with the file_handler when saving a file.
pub fn save_file_interface(filepath: &str, data: &HashMap<V3i, Chunk>) -> Result<(), SaveAndLoadError>{
    let path = Path::new(filepath);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext|ext.to_lowercase());
    
    match extension.as_deref() {
        Some("bin") => {},
        _ => {SaveAndLoadError::NotSupportedFileFormat(extension);},
    }
    
    save_file(path, &data)
}

fn save_file(filepath: &Path, data: &HashMap<V3i, Chunk>) -> Result<(), SaveAndLoadError>{
    let file = File::create(filepath)?;
    let writer = BufWriter::new(file);

    let parsed_data = parse_chunks(data);

    bincode::serialize_into(writer, &parsed_data)?;
    Ok(())
}

fn parse_chunks(data: &'_ HashMap<V3i, Chunk>) -> Vec<FormatedChunkRef<'_>>{
    let mut parsed_chunks: Vec<FormatedChunkRef> = vec![];
    
    for entry in data{
        let formated_chunk = FormatedChunkRef {
            index: *entry.0,
            data: &entry.1.data,
            min_pos: entry.1.min_pos,
            max_pos: entry.1.max_pos,
        };

        parsed_chunks.push(formated_chunk);
    }

    parsed_chunks
}


/// this is the interface to interact with the file_handler when loading a file.
pub fn load_file_interface(filepath: &str) -> Result<HashMap<V3i, Chunk>, SaveAndLoadError>{
    let path = Path::new(filepath);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext|ext.to_lowercase());
    
    match extension.as_deref() {
        Some("bin") => {},
        _ => {SaveAndLoadError::NotSupportedFileFormat(extension);},
    }
    
    let data = load_file(path)?;

    Ok(data)
}

fn load_file(filepath: &Path) -> Result<HashMap<V3i, Chunk>, SaveAndLoadError> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);

    let loaded_data: Vec<FormatedChunk> = bincode::deserialize_from(reader)?;
    let mut world_map: HashMap<V3i, Chunk> = HashMap::new();

    for entry in loaded_data {
        let chunk = Chunk { 
            data: entry.data, 
            min_pos: entry.min_pos, 
            max_pos: entry.max_pos ,
        };

        world_map.insert(entry.index, chunk);
    }

    Ok(world_map)
}

