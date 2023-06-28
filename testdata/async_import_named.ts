(await import('./export_named')).Interface;

const named = await import('./export_named');

const {Type} = named;
named.Var;

import('./export_named').then(mod => {
    mod.Enum;
    const {Fn} = mod;
});
import('./export_named').then((mod: any) => mod.Const);
