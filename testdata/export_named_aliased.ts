// https://rustdoc.swc.rs/swc_ecma_ast/enum.Decl.html#
class Class {};
function Fn() {};
const Const = "1";
var Var = "2";
interface Interface {};
type Type = {};
enum Enum {};

export {
  Class as AliasedClass,
  Fn as AliasedFn,
  Const as AliasedConst,
  Var as AliasedVar,
  Interface as AliasedInterface,
  Type as AliasedType,
  Enum as AliasedEnum,
}
