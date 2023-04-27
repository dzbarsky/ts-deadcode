
use std::fs;
use std::path::Path;

use ts_deadcode::{Analyzer};

fn main() {
    let mut analyzer = Analyzer::new();

    // Specify the directory containing the files to be parsed
    let dir_path = Path::new("testdata");

    // Loop over each file in the directory
    for entry in fs::read_dir(dir_path).unwrap() {
        let entry = entry.unwrap();
        let file_path = entry.path();

        // Check that the entry is a file and has a ".ts" extension
        if entry.file_type().unwrap().is_file() && file_path.extension().unwrap() == "ts" {
            // Parse the file into an AST
            analyzer.add_file(&file_path);
        }
    }

    // Print the recorded symbols
    /*println!("Imports:");
    for (src, symbols) in visitor.imports {
        println!("  {}:", src);
        for symbol in symbols {
            println!("    {}", symbol.sym);
        }
    }
    println!("Exports:");
    for (src, symbols) in visitor.exports {
        println!("  {}:", src);
        for symbol in symbols {
            println!("    {}", symbol.sym);
        }
    }*/
}
