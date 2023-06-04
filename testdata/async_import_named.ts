(await import('testdata/export_named.ts')).Interface;

const named = await import('testdata/export_named.ts');

const {Type} = named;
named.Var;

/*import('testdata/export_named.ts').then(mod => mod.Enum);
import('testdata/export_named.ts').then((mod) => mod.Fn);
import('testdata/export_named.ts').then((mod: any) => mod.Const);
*/
