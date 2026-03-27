use crate::file_parser::file_parse_interface;

mod file_parser;

fn main() {
    if let Some(mesh) = file_parse_interface("bugatti.obj"){        
        for object in mesh{
            println!("{}", object.name)
        }
    } else {
        println!("Fail ;c")
    };
}
