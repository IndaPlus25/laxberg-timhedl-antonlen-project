use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use flate2::bufread::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

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

#[derive(Serialize)]
struct FormatedFileRef <'a>{
    color: &'a Vec<[f32; 4]>,
    parsed_data: &'a Vec<FormatedChunkRef<'a>>,
}

#[derive(Deserialize)]
struct FormatedChunk {
    index: V3i,
    data: Vec<u32>,
    min_pos: V3,
    max_pos: V3,
}

#[derive(Deserialize)]
struct FormatedFile{
    color: Vec<[f32; 4]>,
    parsed_data: Vec<FormatedChunk>,
}

/// this is the interface to interact with the file_handler when saving a file.
pub fn save_file_interface(filepath: &str, data: &HashMap<V3i, Chunk>, colors: &Vec<[f32; 4]>) -> Result<(), SaveAndLoadError>{
    let path = Path::new(filepath);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext|ext.to_lowercase());
    
    match extension.as_deref() {
        Some("bin") => {},
        _ => {return Err(SaveAndLoadError::NotSupportedFileFormat(extension));},
    }
    
    save_file(path, &data, colors)
}

fn save_file(filepath: &Path, data: &HashMap<V3i, Chunk>, colors: &Vec<[f32; 4]>) -> Result<(), SaveAndLoadError>{
    let file = File::create(filepath)?;
    let writer = BufWriter::new(file);

    let compressor = GzEncoder::new(writer, Compression::default());

    let parsed_data = parse_chunks(data);
    let formated_file = FormatedFileRef {color: &colors, parsed_data: &parsed_data};

    bincode::serialize_into(compressor, &formated_file)?;
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
pub fn load_file_interface(filepath: &str) -> Result<(HashMap<V3i, Chunk>, Vec<[f32; 4]>), SaveAndLoadError>{
    let path = Path::new(filepath);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext|ext.to_lowercase());
    
    match extension.as_deref() {
        Some("bin") => {},
        _ => {return Err(SaveAndLoadError::NotSupportedFileFormat(extension));},
    }
    
    let data = load_file(path)?;

    Ok(data)
}

fn load_file(filepath: &Path) -> Result<(HashMap<V3i, Chunk>, Vec<[f32; 4]>), SaveAndLoadError> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);

    let decompressor = GzDecoder::new(reader);

    let loaded_file: FormatedFile = bincode::deserialize_from(decompressor)?;
    let mut world_map: HashMap<V3i, Chunk> = HashMap::new();

    let loaded_data = loaded_file.parsed_data;
    let loaded_color = loaded_file.color;

    for entry in loaded_data {
        let chunk = Chunk { 
            data: entry.data, 
            min_pos: entry.min_pos, 
            max_pos: entry.max_pos ,
        };

        world_map.insert(entry.index, chunk);
    }

    Ok((world_map, loaded_color))
}

/*
#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::*;

    #[test]
    fn same_input_and_output_test(){
        let mut original_data = HashMap::new();
        let pos = V3i { x: 0, y: 0, z: 0 };
        let chunk = Chunk {
            data: vec![0xFFFFFFFF, 0x00000000],
            min_pos: V3 { x: 0.0, y: 0.0, z: 0.0 },
            max_pos: V3 { x: 10.0, y: 10.0, z: 10.0 },
        };
        original_data.insert(pos, chunk);

        let filepath = "test_file1.bin";

        save_file_interface(filepath, &original_data).expect("Failed to save data");

        let loaded_data = load_file_interface(filepath).expect("Failed to load data");

        let orig_chunk = original_data.get(&pos).unwrap();
        let load_chunk = loaded_data.get(&pos).unwrap();

        assert_eq!(original_data.len(), loaded_data.len());
        assert_eq!(
            (orig_chunk.min_pos.x, orig_chunk.min_pos.y, orig_chunk.min_pos.z),
            (load_chunk.min_pos.x, load_chunk.min_pos.y, load_chunk.min_pos.z),
            "min_pos did not match!"
        );
        assert_eq!(
            (orig_chunk.max_pos.x, orig_chunk.max_pos.y, orig_chunk.max_pos.z),
            (load_chunk.max_pos.x, load_chunk.max_pos.y, load_chunk.max_pos.z),
            "max_pos did not match!"
        );
        assert_eq!(orig_chunk.data, load_chunk.data);

        std::fs::remove_file(filepath).unwrap();
    }

    #[test]
    fn reject_invalid_file_type_save_test(){
        let original_data: HashMap<V3i, Chunk> = HashMap::new();
  
        let filepath = "test_file2.txt";

        let path = Path::new(filepath);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext|ext.to_lowercase());

        let save_result = save_file_interface(filepath, &original_data);
        let actual_err = save_result.unwrap_err();

        assert!(
            matches!(
                actual_err, 
                SaveAndLoadError::NotSupportedFileFormat(ext) if ext == extension
            ),
            "Expected NotSupportedFileFormat with 'txt', got something else!"
        );
    }

    #[test]
    fn reject_invalid_file_type_load_test(){  
        let filepath = "test_file3.txt";

        let path = Path::new(filepath);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext|ext.to_lowercase());

        let save_result = load_file_interface(filepath);
        let actual_err = match save_result {
            Ok(_) => panic!("Should not have loaded data"),
            Err(error) => error,
        };

        assert!(
            matches!(
                actual_err, 
                SaveAndLoadError::NotSupportedFileFormat(ext) if ext == extension
            ),
            "Expected NotSupportedFileFormat with 'txt', got something else!"
        );
    }

    #[test]
    fn handle_invalid_file_content_test(){
        let filepath = "test_file4.bin";

        let file = File::create(filepath).expect("Could not create file");
        let writer = BufWriter::new(file);
        let compressor = GzEncoder::new(writer, Compression::default());

        let data = vec![(218792, 289122), (32312, 3243242), (3543523, 7643623)];
        bincode::serialize_into(compressor, &data).expect("Could not serialize");

        let load_result = load_file_interface(filepath);
        let actual_err = match load_result {
            Ok(_) => panic!("Should not have loaded data"),
            Err(error) => error,
        };

        assert!(
            matches!(
                actual_err, 
                SaveAndLoadError::BincodeError(err) if matches!(*err, bincode::ErrorKind::Io(_))
            )
        );

        std::fs::remove_file(filepath).unwrap();
    }

    #[test]
    fn handle_invalid_file_savepath_test(){
        let original_data: HashMap<V3i, Chunk> = HashMap::new();
  
        let filepath = "magic-folder/test_file5.bin";

        let save_result = save_file_interface(filepath, &original_data);
        let actual_err = save_result.unwrap_err();

        assert!(
            matches!(
                actual_err, 
                SaveAndLoadError::IoError(err) if err.kind() == ErrorKind::NotFound
            )
        );
    }
}
*/