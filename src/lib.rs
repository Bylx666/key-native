
mod key;
use key::*;

static mut SAMPLE:Class = Class::uninit();

fn test(args:Vec<LitrRef>)->Litr {
  println!("示例函数:{:?}", &*args[0]);
  Litr::Uninit
}
fn sample_new(_args:Vec<LitrRef>)-> Litr {
  let inst = unsafe { SAMPLE.create(0, 15) };
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
fn next(v:&mut Instance, _:Vec<LitrRef>)-> Litr {
  if v.v2<v.v1 {
    return Symbol::iter_end();
  }
  let res = Litr::Uint(v.v1);
  v.v1 += 1;
  res
}

pub fn main(module: &mut NativeInterface) {
  unsafe {
    SAMPLE.new("Sample");
    module.export_cls(SAMPLE);
    SAMPLE.static_method("new", sample_new);
    SAMPLE.method("see", sample_see);
    SAMPLE.method("@next", next);
    SAMPLE.getter(sample_getter);
    SAMPLE.setter(sample_setter);
  }
  module.export_fn("test", test);
}
