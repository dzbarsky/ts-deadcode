use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use swc_ecma_loader::{
    resolvers::{/*lru::CachingResolver, */ node::NodeModulesResolver, tsc::TsConfigResolver},
    TargetEnv,
};
use tsconfig::TsConfig;

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

fn resolve(repo_root: &str, to: &[String]) -> Vec<String> {
    to.iter()
        .map(|s| {
            let mut resolved = repo_root.to_owned();
            resolved.push_str(&s[1..]);
            if &s[s.len() - 1..] != "*" {
                resolved.push_str(".tsx");
            }
            resolved
        })
        .collect()
}

fn main() {
    let tsconfig_path = std::env::args().nth(1).unwrap();
    let repo_root = std::env::args().nth(2).unwrap();

    let tsconfig = TsConfig::parse_file(&tsconfig_path).unwrap();
    let mut resolved_paths = vec![];
    if let Some(compiler_options) = tsconfig.compiler_options {
        if let Some(paths) = compiler_options.paths {
            resolved_paths = paths
                .into_iter()
                .map(|(from, to)| (from, resolve(&repo_root, &to)))
                .collect();
        }
    }

    let resolver = {
        let r = TsConfigResolver::new(
            NodeModulesResolver::new(TargetEnv::Node, Default::default(), false),
            ".".into(),
            resolved_paths,
        );
        //let r = CachingResolver::new(40, r);

        //let r = NodeImportResolver::new(r);
        Box::new(r)
    };

    let mut analyzer = Analyzer::new(resolver);

    // Specify the directory containing the files to be parsed
    let dir_path = Path::new(&repo_root);

    visit_dirs(dir_path, &mut |entry: &DirEntry| {
        let file_path = entry.path();

        //println!("entry {:?}", entry);
        if !file_path.to_str().unwrap().ends_with(".d.ts") {
            let ext = file_path.extension().unwrap_or_default();
            if ext == "ts" || ext == "tsx" {
                // Parse the file into an AST
                // println!("analyzing file {:?}", file_path);
                analyzer.add_file(&file_path);
            }
        }
    }).expect("should not fail");

    let mut count = 0;

    for (file, results) in analyzer.finalize() {
        let c = results.unused_symbols.len();
        println!("{:?}: {:?}, {:?}", file, c, results.unused_symbols);
        count += c;
    }
    println!("TOTAL RESULTS: {}", count);
}
