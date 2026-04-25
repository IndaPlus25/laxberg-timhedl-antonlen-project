use std::collections::HashMap;
use std::fmt::Error;

use crate::vecmath::*;
use crate::octree::*;


/// this is the interface to interact with the file_handler when saving a file.
pub fn save_file_interface(data: HashMap<V3i, Chunk>) -> Result<(), Error>{
    Ok(())
}


/// this is the interface to interact with the file_handler when loading a file.
pub fn load_file_interface(file: String) -> Result<(), Error>{
    Ok(())
}