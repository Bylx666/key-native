
mod key;
use key::*;

static mut FLOAT_ARRAY:Class = Class::uninit();

fn test(args:Vec<LitrRef>)->Litr {
  println!("示例函数:{:?}", &*args[0]);
  Litr::Uninit
}
fn sample_new(_args:Vec<LitrRef>)-> Litr {
  let inst = unsafe { FLOAT_ARRAY.create(24, 56) };
  inst
}
fn sample_see(v:&mut Instance, _args:Vec<LitrRef>)-> Litr {
  println!("{} {}", v.v1, v.v2);
  Litr::Uninit
}
fn sample_getter(_v:&mut Instance, get:Interned)-> Litr {
  println!("dll get: {get}");
  Litr::Uninit
}
fn sample_setter(_v:&mut Instance, set:Interned, to:Litr) {
  println!("dll set: {set}, {to:?}");
}

pub fn main(module: &mut NativeInterface) {
  unsafe {
    FLOAT_ARRAY.new(b"Sample");
    module.export_cls(FLOAT_ARRAY);
    FLOAT_ARRAY.static_method(b"new", sample_new);
    FLOAT_ARRAY.method(b"see", sample_see);
    FLOAT_ARRAY.getter(sample_getter);
    FLOAT_ARRAY.setter(sample_setter);
  }
  module.export_fn(b"test", test);
}
