use std::collections::HashMap;
use std::fmt::Error;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::vecmath::*;
use crate::octree::*;


/// this is the interface to interact with the file_handler when saving a file.
pub fn save_file_interface(data: HashMap<V3i, Chunk>) -> Result<(), Error>{
    Ok(())
}

fn save_file(filepath: &str) -> Result<(), Box<dyn std::error::Error>>{
    let file = File::create(filepath)?;
    let writer = BufWriter::new(file);

    let data: Vec<u32> = vec![];

    bincode::serialize_into(writer, &data)?;
    Ok(())
}


/// this is the interface to interact with the file_handler when loading a file.
pub fn load_file_interface(file: String) -> Result<(), Error>{
    Ok(())
}

