require('./export_named').Interface;

const named = require('./export_named');

const {Type} = named;
named.Var;

const named2 = require('./export_named') as typeof import('./export_named');
named2.Enum;

const Fn = require('./export_named').Fn as typeof import('./export_named').Fn;
const Const = (require('./export_named') as typeof import('./export_named')).Const;
