import * as mod from 'testdata/export_foo.ts';
mod.foo;

import('testdata/export_bar.ts').then(mod => {
  const {bar, baz} = mod;
});

mod.bar;
