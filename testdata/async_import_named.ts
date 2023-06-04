(await import('testdata/export_named.ts')).Interface;

const named = await import('testdata/export_named.ts');

const {Type} = named;
named.Var;

import('testdata/export_named.ts').then(mod => {
    mod.Enum;
    const {Fn} = mod;
});
import('testdata/export_named.ts').then((mod: any) => mod.Const);

import('testdata/side_effects.ts').then(() => {});
