use std::collections::HashMap;
use std::fmt::Error;
use rayon::vec;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::vecmath::*;
use crate::octree::*;

#[derive(serde::Serialize)]
struct FormatedChunk<'a>{
    index: V3i,
    data: &'a Vec<u32>,
    min_pos: V3,
    max_pos: V3,
}

/// this is the interface to interact with the file_handler when saving a file.
pub fn save_file_interface(filepath: &str, data: &HashMap<V3i, Chunk>) -> Result<(), Box<dyn std::error::Error>>{
    save_file(filepath, &data)?;

    Ok(())
}

fn save_file(filepath: &str, data: &HashMap<V3i, Chunk>) -> Result<(), Box<dyn std::error::Error>>{
    let file = File::create(filepath)?;
    let writer = BufWriter::new(file);

    let parsed_data = parse_chunks(data);

    bincode::serialize_into(writer, &parsed_data)?;
    Ok(())
}

fn parse_chunks(data: &HashMap<V3i, Chunk>) -> Vec<FormatedChunk>{
    let mut parsed_chunks: Vec<FormatedChunk> = vec![];
    
    for entry in data{
        let formated_chunk = FormatedChunk {
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
pub fn load_file_interface(file: String) -> Result<(), Error>{
    Ok(())
}

