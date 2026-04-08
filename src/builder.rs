use crate::octree::*;
use crate::vecmath::*;

use std::collections::HashMap;

/*
pub fn to_flat(data: &[&[&[u32]]]) -> Vec<u32> {
    let width = data.len();
    let height = if width > 0 { data[0].len() } else { 0 };
    let depth = if height > 0 { data[0][0].len() } else { 0 };

    let total_size = width * height * depth;

    let mut flat_data = vec![0; total_size];

    for x in 0..width {
        for y in 0..height {
            for z in 0..depth {
                let block = data[x][y][z];
                
                if block != 0 {
                    let index = (x) + (y * depth) + (z * depth * height);
                    
                    flat_data[index] = block;
                }
            }
        }
    }

    flat_data
}
*/

pub fn to_chunks(data: &[&[&[u32]]]) -> HashMap<V3i, Chunk> {

    //turn chunks into flat data
    //
    //check from position 0 to 32 in x,y,z to build the first chunks
    //
    //send that data to a chunk builder function
    //
    //loop through all the chunks and add their V3i identifiers, 
    //
    //this should produce the hashmap which we then return.


}

//takes a 32768 long u32 array and loops through it. 
pub fn build_chunk(data: &[u32]) -> Chunk {
    //here is my though on how to implement this function:

    //go downwards in the chunk, start from the bottom, first we check the lowest cube, is there
    //data then set that data.
    //
    //if we continue and build the first litte 8 bit of information and it is all the same data,
    //then combine it and say it is all the same, now do this to the neighboring 8 bit of
    //information.
    //
    //we continue this upwards in the structure and combine if all 8 neighbors are identical and if
    //not we set the data to my svo standard 32 bit identifier. 

}
