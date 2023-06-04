require('testdata/export_named.ts').Interface;

const named = require('testdata/export_named.ts');

const {Type, Fn, Const} = named;
named.Var;

const named2 = require('testdata/export_named.ts') as typeof import('testdata/export_named.ts');
named2.Enum;

