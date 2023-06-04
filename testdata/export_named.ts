// https://rustdoc.swc.rs/swc_ecma_ast/enum.Decl.html#
class Class {};
function Fn() {};
var Var = "2";
interface Interface {};
type Type = {};
enum Enum {};

export {
  Class,
  Fn,
  Var,
  Interface,
  Type,
  Enum,
}

export const {Const} = {Const: "1"};