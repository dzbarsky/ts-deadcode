use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use swc_ecma_loader::resolvers::node::NodeModulesResolver;
use swc_ecma_loader::TargetEnv;

use ts_deadcode::Analyzer;

fn visit_dirs(dir: &Path, cb: &mut dyn for<'a> FnMut(&'a DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && entry.file_name() != "node_modules" {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}

fn main() {
    let mut analyzer = Analyzer::new(Box::new(NodeModulesResolver::new(
        TargetEnv::Node,
        Default::default(),
        false,
    )));

    // Specify the directory containing the files to be parsed
    let dir = std::env::args().nth(1).unwrap();
    let dir_path = Path::new(&dir);

    visit_dirs(dir_path, &mut |entry: &DirEntry| {
        let file_path = entry.path();

        //println!("entry {:?}", entry);
        let ext = file_path.extension().unwrap_or_default();
        if ext == "ts" || ext == "tsx" {
            // Parse the file into an AST
            println!("{:?}", file_path);
            analyzer.add_file(&file_path);
        }
    });

    let mut count = 0;

    for (file, results) in analyzer.finalize() {
        let c = results.unused_symbols.len();
        println!("{:?}: {:?}", file, c);
        count += c;
    }
    println!("TOTAL RESULTS: {}", count);
}
