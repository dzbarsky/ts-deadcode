use clap::Parser;
use parcel_resolver::{OsFileSystem, Resolver};
use std::collections::HashMap;
use std::env::set_current_dir;
use std::fs::{self, read_to_string, DirEntry};
use std::io;
use std::path::{Path, PathBuf};

use ts_deadcode::{Analyzer, ModuleResults};

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

#[derive(Parser)]
struct Cli {
    repo_root: std::path::PathBuf,

    #[clap(long, action)]
    ignore_unused_type_exports: bool,

    #[clap(long, action)]
    allow_unused_export_if_used_in_self_module: bool,

    #[clap(short = 'i', long)]
    ignore: Vec<String>,

    #[clap(long, action)]
    ignore_tests: bool,
}

fn main() {
    let args = Cli::parse();

    set_current_dir(&args.repo_root).expect("should set current dir");

    // Find internal packages to build resolver map.
    let mut resolvers = HashMap::new();
    visit_dirs(&args.repo_root, &mut |entry: &DirEntry| {
        if entry.file_name() != "package.json" {
            return;
        }

        let project = PathBuf::from(entry.path().parent().unwrap());
        let resolver = Resolver::parcel(
            project.clone().into(),
            parcel_resolver::CacheCow::Owned(parcel_resolver::Cache::new(OsFileSystem)),
        );
        resolvers.insert(project, resolver);
    })
    .expect("Failed to build resolver map");

    let mut analyzer = Analyzer::new();

    // Specify the directory containing the files to be parsed
    let dir_path = Path::new(&args.repo_root);

    visit_dirs(dir_path, &mut |entry: &DirEntry| {
        let file_path = entry.path();

        for item in &args.ignore {
            if file_path.iter().any(|c| item == c.to_str().unwrap()) {
                return;
            }
        }

        let filename = file_path.to_str().unwrap();
        if filename.ends_with(".d.ts") {
            return;
        }

        if args.ignore_tests && filename.ends_with(".test.tsx") {
            return;
        }

        let ext = file_path.extension().unwrap_or_default();
        if ext == "ts"
            || ext == "tsx"
            || ext == "js"
            || ext == "jsx"
            || ext == "mjs"
            || ext == "cjs"
        {
            // Find the resolver for the nearest enclosing project
            let mut package_path = file_path.clone();
            while let Some(package_path_parent) = package_path.parent() {
                if let Some(resolver) = resolvers.get(package_path_parent) {
                    analyzer.add_file(resolver, &file_path);
                    return;
                }
                package_path = package_path_parent.into();
            }
            println!("no resolver for {:?}", file_path);
        }
    })
    .expect("should not fail");

    let mut count = 0;

    let results = analyzer.finalize();
    let mut files: Vec<(&PathBuf, &ModuleResults)> = results.iter().collect();
    files.sort_by_key(|(k, _)| *k);
    for (file, module_results) in files {
        let mut export_providers = vec![&module_results.unused_exports];
        if !args.ignore_unused_type_exports {
            export_providers.push(&module_results.unused_type_exports);
        }

        for unused_exports in export_providers {
            let c = unused_exports.len();
            if c == 0 {
                continue;
            }

            let contents = read_to_string(file).expect("should read file");
            for export in unused_exports {
                let export = export.to_string();
                let first_usage = contents.find(&export).unwrap();
                match contents[first_usage + 1..].find(&export) {
                    None => {
                        println!("{:?}: {:?}", file, export);
                        count += 1;
                    }
                    _ => {
                        if !args.allow_unused_export_if_used_in_self_module {
                            println!("{:?}: {:?} [USED IN FILE]", file, export);
                        }
                    }
                }
            }
        }
    }
    println!("TOTAL RESULTS: {}", count);
}
