import * as mod from './export_foo';
mod.foo;

import('./export_bar').then(mod => {
  const {bar, baz} = mod;
});

mod.bar;
