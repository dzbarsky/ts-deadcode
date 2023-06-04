use std::collections::{HashMap, HashSet};

use std::path::Path;

use swc_atoms::JsWord;
use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    SourceMap,
};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};
use swc_ecma_visit::Visit;
use swc_ecma_visit::VisitWith;

#[derive(Debug)]
pub struct ImportUsage {
    // Filename -> symbols
    imports: HashMap<JsWord, HashSet<JsWord>>,
    // Filenames that have default imports used;
    //default_imports: HashSet<JsWord>,
}

impl ImportUsage {
    fn new() -> Self {
        Self {
            imports: HashMap::new(),
            //default_imports: HashSet::new(),
        }
    }

    fn record_import(&mut self, path: JsWord, symbol: JsWord) {
        self.imports.entry(path).or_default().insert(symbol);
    }

    /*fn record_default_import(&mut self, path: JsWord) {
        self.default_imports.insert(path);
    }*/
}

fn export_name_atom(export: &ModuleExportName) -> JsWord {
    match export {
        ModuleExportName::Ident(ident) => ident.sym.clone(),
        ModuleExportName::Str(str) => str.value.clone(),
    }
}

pub struct FileAnalyzer<'a> {
    filename: String,
    // exported_name -> original_name
    exports: HashMap<JsWord, JsWord>,
    //has_default_export: bool,
    import_usage: &'a mut ImportUsage,
    // local name -> file
    namespace_imports: HashMap<JsWord, JsWord>,
}

impl<'a> FileAnalyzer<'a> {
    fn new(filename: String, import_usage: &'a mut ImportUsage) -> Self {
        Self {
            filename,
            //has_default_export: false,
            exports: HashMap::new(),
            namespace_imports: HashMap::new(),
            import_usage,
        }
    }

    //fn record_default_export(&mut self) {
    //    self.has_default_export = true;
    //}

    fn record_export(&mut self, exported_name: &JsWord, original_name: &JsWord) {
        self.exports
            .insert(exported_name.clone(), original_name.clone());
    }

    // When an import object is destructured, this marks all the object keys as imported.
    fn record_destructured_import(&mut self, file: JsWord, object: &ObjectPat) {
        for prop in &object.props {
            match prop {
                ObjectPatProp::Assign(assign) => self
                    .import_usage
                    .record_import(file.clone(), assign.key.sym.clone()),
                ObjectPatProp::KeyValue(kv) => match &kv.key {
                    PropName::Ident(ident) => {
                        self.import_usage
                            .record_import(file.clone(), ident.sym.clone());
                    }
                    _ => {
                        panic!("unhandle object prop: {:?}", prop);
                    }
                },
                _ => panic!("unhandle object prop: {:?}", prop),
            }
        }
    }

    // Record usage of:
    //   require('testdata/export_named.ts').Interface
    fn handle_potential_require_call_member_expr(
        &mut self,
        call: &CallExpr,
        member_expr: &MemberExpr,
    ) {
        if let Some(filename) = extract_require_call(call) {
            match &member_expr.prop {
                MemberProp::Ident(ident) => {
                    self.import_usage.record_import(filename, ident.sym.clone())
                }
                _ => panic!("unhandled"),
            }
        }
    }

    // Record usage of:
    //   (await import('testdata/export_named.ts')).Interface;
    fn handle_potential_import_call_member_expr(
        &mut self,
        call: &CallExpr,
        member_expr: &MemberExpr,
    ) {
        if let Some(filename) = extract_import_call(call) {
            match &member_expr.prop {
                MemberProp::Ident(ident) => {
                    self.import_usage.record_import(filename, ident.sym.clone())
                }
                _ => panic!("unhandled"),
            }
        }
    }
}

fn extract_require_call(call: &CallExpr) -> Option<JsWord> {
    match &call.callee {
        Callee::Super(_) => {}
        Callee::Import(_import) => {}
        // Handle `require('filename')`
        Callee::Expr(expr) => {
            if let Expr::Ident(ref ident) = **expr {
                if ident.sym == *"require" {
                    match *call.args[0].expr {
                        Expr::Lit(Lit::Str(ref file)) => return Some(file.value.clone()),
                        _ => println!("WARNING: unhandled non-literal require"),
                    }
                }
            }
        }
    }
    None
}

fn extract_import_call(call: &CallExpr) -> Option<JsWord> {
    match &call.callee {
        Callee::Super(_) => {}
        Callee::Import(_import) => match *call.args[0].expr {
            Expr::Lit(Lit::Str(ref file)) => return Some(file.value.clone()),
            _ => println!("WARNING: unhandled non-literal require"),
        },
        Callee::Expr(_expr) => {}
    }
    None
}

impl<'a> Visit for FileAnalyzer<'a> {
    fn visit_module_decl(&mut self, decl: &ModuleDecl) {
        match decl {
            ModuleDecl::Import(import_decl) => {
                for specifier in &import_decl.specifiers {
                    match specifier {
                        ImportSpecifier::Named(named_specifier) => {
                            let atom = match &named_specifier.imported {
                                Some(imported) => export_name_atom(imported),
                                None => named_specifier.local.sym.clone(),
                            };

                            //println!("named import from {:?}: {:?}", import_decl.src, atom);
                            self.import_usage
                                .record_import(import_decl.src.value.clone(), atom);
                            /*self.record_import(
                                named_specifier
                                    .imported
                                    .unwrap_or(named_specifier.local)
                                    .clone(),
                            );*/
                        }
                        ImportSpecifier::Default(_default_specifier) => {
                            //println!("USING {:?}", import_decl.src.value.clone());
                            self.import_usage
                                .record_import(import_decl.src.value.clone(), "default".into())
                        }
                        ImportSpecifier::Namespace(namespace_specifier) => {
                            self.namespace_imports.insert(
                                namespace_specifier.local.sym.clone(),
                                import_decl.src.value.clone(),
                            );
                        }
                    }
                }
            }
            ModuleDecl::ExportDecl(export_decl) => match &export_decl.decl {
                Decl::Class(class) => {
                    let atom = &class.ident.sym;
                    self.record_export(atom, atom);
                    //println!("{}: class decl {:?}", self.filename, atom.clone());
                }
                Decl::Fn(func) => {
                    let atom = &func.ident.sym;
                    self.record_export(atom, atom);
                    //println!("{}: func decl {:?}", self.filename, atom.clone());
                }
                Decl::Var(var) => {
                    for decl in &var.decls {
                        match &decl.name {
                            Pat::Ident(ident) => {
                                let atom = &ident.id.sym;
                                self.record_export(atom, atom);
                                //println!("{}: var decl {:?}", self.filename, atom);
                            }
                            /*
                             * Handle
                             * export const [Const, Var] = ["1", "2"];
                             */
                            Pat::Array(array) => {
                                for elem in &array.elems {
                                    match elem {
                                        Some(Pat::Ident(ident)) => {
                                            let atom = &ident.id.sym;
                                            self.record_export(atom, atom);
                                            //println!("{}: var decl {:?}", self.filename, atom);
                                        }
                                        _ => panic!("unknown array export pat: {:?}", elem),
                                    }
                                }
                            }
                            _ => panic!("unknown decl.name: {:?}", decl.name),
                        }
                    }
                }
                Decl::TsInterface(interface) => {
                    let atom = &interface.id.sym;
                    self.record_export(atom, atom);
                    //println!("{}: interface decl {:?}", self.filename, atom);
                }
                Decl::TsTypeAlias(alias) => {
                    let atom = &alias.id.sym;
                    self.record_export(atom, atom);
                    //println!("{}: type decl {:?}", self.filename, atom);
                }
                Decl::TsEnum(ts_enum) => {
                    let atom = &ts_enum.id.sym;
                    self.record_export(atom, atom);
                    //println!("{}: enum decl {:?}", self.filename, atom);
                }
                _ => {
                    println!("WARNING: unhandled export namespace {}", self.filename);
                }
            },
            ModuleDecl::ExportNamed(named_export) => {
                for specifier in &named_export.specifiers {
                    match specifier {
                        ExportSpecifier::Named(named_specifier) => {
                            let orig_atom = export_name_atom(&named_specifier.orig);
                            let exported_atom = match &named_specifier.exported {
                                Some(exported) => export_name_atom(exported),
                                None => orig_atom.clone(),
                            };
                            self.record_export(&exported_atom, &orig_atom);
                            //println!("{}: {:?} as {:?}", self.filename, orig_atom, exported_atom);
                            //self.record_export(named_specifier.orig.sym.clone());
                        }
                        ExportSpecifier::Namespace(namespace_specifier) => {
                            let atom = export_name_atom(&namespace_specifier.name);
                            self.record_export(&atom, &atom);
                            //println!("{}: namespace {:?}", self.filename, atom);
                            //self.record_export(namespace_specifier.name.sym.clone());
                        }
                        ExportSpecifier::Default(_default_specifier) => {
                            self.record_export(&"default".into(), &"default".into());
                            //self.record_default_export();
                            //println!("{}: default {:?}", self.filename, default_specifier);
                            //self.record_export(default_specifier.exported.sym.clone());
                        }
                    }
                }
            }
            ModuleDecl::ExportAll(_export_all) => {}
            ModuleDecl::ExportDefaultDecl(_export_default_decl) => {
                self.record_export(&"default".into(), &"default".into())
            }
            ModuleDecl::ExportDefaultExpr(_export_default_decl) => {
                self.record_export(&"default".into(), &"default".into())
            }
            _ => {
                println!("WARNING: unhandled ModuleDecl {:?}", decl);
            }
        }
        decl.visit_children_with(self);
    }

    fn visit_var_declarator(&mut self, var: &VarDeclarator) {
        if let Some(ref init) = var.init {
            let filename = match **init {
                Expr::Ident(ref ident) => {
                    /*
                    Handle the following:

                    import * as utils from 'testdata/export_named.ts';
                    const {Class, Fn} = utils;
                    */

                    match self.namespace_imports.get(&ident.sym) {
                        Some(file) => match &var.name {
                            Pat::Object(object) => {
                                self.record_destructured_import(file.clone(), object);
                                None
                            }
                            _ => panic!("unhandled var name"),
                        },
                        None => None,
                    }
                }
                // const named = require('testdata/export_named.ts');
                Expr::Call(ref call) => extract_require_call(call),
                // const named = require('testdata/export_named.ts') as typeof import('testdata/export_named.ts');
                Expr::TsAs(ref as_expr) => match *as_expr.expr {
                    Expr::Call(ref call) => extract_require_call(call),
                    _ => None,
                },
                // const named = await import('testdata/export_named.ts');
                Expr::Await(ref await_expr) => match *await_expr.arg {
                    Expr::Call(ref call) => extract_import_call(call),
                    _ => None,
                },
                _ => None,
            };

            if let Some(filename) = filename {
                match &var.name {
                    // const named = require('testdata/export_named.ts');
                    Pat::Ident(binding) => {
                        self.namespace_imports
                            .insert(binding.id.sym.clone(), filename);
                    }
                    // const {Enum, Fn} = require('testdata/export_named.ts');
                    Pat::Object(object) => self.record_destructured_import(filename, object),
                    _ => todo!("fuck"),
                }
            }

            var.visit_children_with(self);

            // TODO(zbarsky): pop bindings we added!
        } else {
            var.visit_children_with(self);
        }
    }

    fn visit_member_expr(&mut self, member_expr: &MemberExpr) {
        if !self.namespace_imports.is_empty() {
            if let Expr::Ident(Ident { ref sym, .. }) = *member_expr.obj {
                /*
                Handle the following:

                import * as utils from 'testdata/export_named.ts';
                utils.Const;
                */
                if let Some(file) = self.namespace_imports.get(sym) {
                    match &member_expr.prop {
                        MemberProp::Ident(ident) => self
                            .import_usage
                            .record_import(file.clone(), ident.sym.clone()),
                        _ => println!("WARNING: unhandled MemberExpr: {:?}", member_expr),
                    }
                }
            }
        }

        match *member_expr.obj {
            // require('testdata/export_named.ts').Interface
            // require('testdata/export_named.ts').default
            Expr::Call(ref call) => {
                //println!("MEMBER EXPR: {:?}", member_expr);
                self.handle_potential_require_call_member_expr(call, member_expr)
            }
            // TODO(zbarsky): would be nice to have box_deref_patterns
            Expr::Paren(ref paren_expr) => {
                match *paren_expr.expr {
                    // (require('testdata/export_named.ts') as import('testdata/export_named.ts')).Interface
                    Expr::TsAs(ref as_expr) => match *as_expr.expr {
                        Expr::Call(ref call) => {
                            self.handle_potential_require_call_member_expr(call, member_expr)
                        }
                        _ => {}
                    },
                    // (await import('testdata/export_named.ts')).Interface;
                    Expr::Await(ref await_expr) => match *await_expr.arg {
                        Expr::Call(ref call) => {
                            self.handle_potential_import_call_member_expr(call, member_expr)
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {}
        }

        member_expr.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, call_expr: &CallExpr) {
        let mut sym = None;
        let mut filename = None;
        // import('testdata/export_named.ts').then(mod => mod.Enum);
        if let Callee::Expr(ref callee_expr) = call_expr.callee {
            if let Expr::Member(ref member_expr) = **callee_expr {
                if let MemberProp::Ident(ident_expr) = &member_expr.prop {
                    if ident_expr.sym == *"then" {
                        if let Expr::Call(ref call) = *member_expr.obj {
                            println!("call: {:?}", call_expr);
                            if let Some(ref arg) = call_expr.args.get(0) {
                                if let Expr::Arrow(ref arrow_expr) = *arg.expr {
                                    if let Some(Pat::Ident(ref ident)) = arrow_expr.params.get(0) {
                                        sym = Some(ident.id.sym.clone());
                                        filename = extract_import_call(call);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        match (sym, filename) {
            (Some(sym), Some(filename)) => {
                let prev_binding = self.namespace_imports.insert(sym.clone(), filename);
                call_expr.visit_children_with(self);
                if let Some(prev_binding) = prev_binding {
                    self.namespace_imports.insert(sym, prev_binding);
                }
            }
            _ => call_expr.visit_children_with(self),
        }
    }

    /*fn visit_module_item(&mut self, n: &ModuleItem) {
        println!("item {:?}", n);
    }*/
}

pub struct ModuleExports {
    // exported_name -> original_name
    exports: HashMap<JsWord, JsWord>,
    has_default_export: bool,
}

#[derive(Debug, PartialEq)]
pub struct ModuleResults {
    unused_default_export: bool,
    unused_symbols: HashSet<JsWord>,
}
pub type AnalysisResults = HashMap<String, ModuleResults>;

pub struct Analyzer {
    import_usage: ImportUsage,

    cm: Lrc<SourceMap>,
    handler: Handler,

    exports: HashMap<String, ModuleExports>,
}

impl Analyzer {
    pub fn new() -> Self {
        let cm: Lrc<SourceMap> = Default::default();
        Self {
            import_usage: ImportUsage::new(),
            handler: Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone())),
            exports: HashMap::new(),
            cm,
        }
    }

    pub fn add_file(&mut self, file_path: &Path) {
        // Parse the file into an AST
        let fm = self.cm.load_file(file_path).expect("failed to load file");

        // Create a visitor to traverse the ASTs and record imported and exported symbols
        let mut visitor = FileAnalyzer::new(
            file_path.to_str().unwrap().to_owned(),
            &mut self.import_usage,
        );

        let lexer = Lexer::new(
            // We want to parse ecmascript
            Syntax::Typescript(TsConfig {
                tsx: true,
                decorators: true,
                ..Default::default()
            }),
            // EsVersion defaults to es5
            Default::default(),
            StringInput::from(&*fm),
            None,
        );

        let mut parser = Parser::new_from(lexer);

        for e in parser.take_errors() {
            e.into_diagnostic(&self.handler).emit();
        }

        let module = parser
            .parse_module()
            .map_err(|e| {
                // Unrecoverable fatal error occurred
                e.into_diagnostic(&self.handler).emit()
            })
            .expect("failed to parser module");

        // Traverse the AST and record imported and exported symbols
        module.visit_with(&mut visitor);

        self.exports.insert(
            visitor.filename,
            ModuleExports {
                exports: visitor.exports,
                has_default_export: false,
                //has_default_export: visitor.has_default_export,
            },
        );
    }

    pub fn finalize(self) -> AnalysisResults {
        let mut results = AnalysisResults::new();
        for (file, exports) in self.exports {
            let mut module_results = ModuleResults {
                unused_default_export: false,
                unused_symbols: HashSet::new(),
            };
            println!("State: {:?}", self.import_usage);
            let file_atom = &file.clone().into();
            /*if exports.has_default_export {
                module_results.unused_default_export = self.import_usage.default_imports.contains(file_atom);
            }*/
            let imports = self.import_usage.imports.get(file_atom);
            for (exported_name, original_name) in exports.exports {
                //if !imports.is_some_and(|v| v.contains(&exported_name)) {
                let used = match imports {
                    Some(v) => v.contains(&exported_name),
                    None => false,
                };
                if !used {
                    module_results.unused_symbols.insert(original_name);
                }
            }
            if module_results.unused_default_export || !module_results.unused_symbols.is_empty() {
                results.insert(file, module_results);
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    fn analyze(filepaths: Vec<&str>) -> AnalysisResults {
        let mut analyzer = Analyzer::new();
        for path in filepaths {
            analyzer.add_file(Path::new(path));
        }
        analyzer.finalize()
    }

    #[test]
    fn named_exports() {
        let results = analyze(vec!["testdata/export_named.ts"]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_named.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from([
                        "Class".into(),
                        "Enum".into(),
                        "Fn".into(),
                        "Var".into(),
                        "Interface".into(),
                        "Const".into(),
                        "Type".into(),
                    ])
                }
            )])
        );
    }

    #[test]
    fn named_exports_inline() {
        let results = analyze(vec!["testdata/export_decl.ts"]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_decl.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from([
                        "Class".into(),
                        "Enum".into(),
                        "Fn".into(),
                        "Var".into(),
                        "Interface".into(),
                        "Const".into(),
                        "Type".into(),
                    ])
                }
            )])
        );
    }

    #[test]
    fn named_exports_imported_partially() {
        let results = analyze(vec![
            "testdata/export_named.ts",
            "testdata/import_named_partial_no_class.ts",
        ]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_named.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from(["Class".into(),])
                }
            )])
        );
    }

    #[test]
    fn named_exports_imported_fully() {
        let results = analyze(vec![
            "testdata/export_named.ts",
            "testdata/import_named_partial_no_class.ts",
            "testdata/import_named_partial_only_class.ts",
        ]);
        assert_eq!(results, HashMap::new());
    }

    #[test]
    fn aliased_named_exports() {
        let results = analyze(vec!["testdata/export_named_aliased.ts"]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_named_aliased.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from([
                        "Class".into(),
                        "Enum".into(),
                        "Fn".into(),
                        "Var".into(),
                        "Interface".into(),
                        "Const".into(),
                        "Type".into(),
                    ])
                }
            )])
        );
    }

    #[test]
    fn aliased_named_exports_imported_partially() {
        let results = analyze(vec![
            "testdata/export_named_aliased.ts",
            "testdata/import_named_aliased_no_enum.ts",
        ]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_named_aliased.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from(["Enum".into(),])
                }
            )])
        );
    }

    #[test]
    fn aliased_named_exports_imported_fully() {
        let results = analyze(vec![
            "testdata/export_named_aliased.ts",
            "testdata/import_named_aliased_no_enum.ts",
            "testdata/import_named_aliased_only_enum.ts",
        ]);
        assert_eq!(results, HashMap::new());
    }

    #[test]
    fn namespace_imported_partially() {
        let results = analyze(vec![
            "testdata/export_named.ts",
            "testdata/import_namespace_partial.ts",
        ]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_named.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from(["Enum".into(),])
                }
            )])
        );
    }

    #[test]
    fn require_named() {
        let results = analyze(vec![
            "testdata/export_named.ts",
            "testdata/require_named.ts",
        ]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_named.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from(["Class".into(),])
                }
            )])
        );
    }

    #[test]
    fn async_import_named() {
        let results = analyze(vec![
            "testdata/export_named.ts",
            "testdata/async_import_named.ts",
        ]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_named.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from(["Class".into(),])
                }
            )])
        );
    }

    #[test]
    fn import_defaults() {
        let results = analyze(vec![
            "testdata/export_default_class.ts",
            "testdata/export_default_interface.ts",
            "testdata/export_default_function.ts",
            "testdata/export_default_object.ts",
            "testdata/import_defaults.ts",
        ]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_default_interface.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from(["default".into(),])
                }
            )])
        );
    }

    #[test]
    fn import_module_obj_name_collisions() {
        let results = analyze(vec![
            "testdata/export_foo.ts",
            "testdata/export_bar.ts",
            "testdata/import_foo_bar.ts",
        ]);
        assert_eq!(
            results,
            HashMap::from([(
                "testdata/export_foo.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from(["baz".into(),])
                }
            ),(
                "testdata/export_bar.ts".into(),
                ModuleResults {
                    unused_default_export: false,
                    unused_symbols: HashSet::from(["foo".into(),])
                }
            )])
        );
    }

    #[test]
    fn acid_test() {
        let results = analyze(vec![
            "testdata/acid.ts",
        ]);
        assert_eq!(
            results,
            HashMap::from([]),
        );
    }
}
