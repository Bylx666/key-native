#![allow(unused)]
mod key;
use key::*;

static mut SAMPLE:Class = Class::uninit();

fn test(args:Vec<Litr>)->Litr {
  println!("示例函数{args:?}");
  Litr::Uninit
}
fn sample_new(args:Vec<Litr>)-> Litr {
  let inst = unsafe { SAMPLE.create(24, 56) };
  Litr::Ninst(inst)
}
fn sample_see(v:&mut Instance, args:Vec<Litr>)-> Litr {
  println!("{v:?}");
  Litr::Float(222.22)
}

pub fn main(module: &mut NativeInterface) {
  unsafe {
    SAMPLE.new(b"Sample");
    module.export_cls(SAMPLE);
    SAMPLE.static_method(b"new", sample_new);
    SAMPLE.method(b"see", sample_see);
  }
  module.export_fn(b"test", test);
}
