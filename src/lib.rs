#![allow(unused)]
mod key;
use key::*;

static mut OKK:Class = Class::uninit();

fn test(args:Vec<Litr>)->Litr {
  println!("呃呃{args:?}");
  Litr::Uninit
}
fn new(_args:Vec<Litr>)-> Litr {
  Litr::Float(2.024)
}

pub fn main(module: &mut NativeInterface) {unsafe{
  OKK.new(b"Okk");
  module.export_cls(OKK);
  OKK.static_method(b"new", new);
  module.export_fn(b"test", test);
}}
