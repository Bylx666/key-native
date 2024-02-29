
mod key;
use key::*;

static SAMPLE:Class = Class::uninit();

fn test(_args:Vec<LitrRef>, _cx:Scope)->Litr {
  println!("{:?}",&*_cx.find_var("a").unwrap());
  Litr::Uninit
}
fn sample_new(_args:Vec<LitrRef>, _cx:Scope)-> Litr {
  let inst = SAMPLE.create(0, 15);
  inst
}
fn sample_see(v:&mut Instance, _args:Vec<LitrRef>, _cx:Scope)-> Litr {
  println!("{} {}", v.v1, v.v2);
  Litr::Uninit
}
fn sample_getter(_v:&mut Instance, get:Ident)-> Litr {
  println!("dll get: {get}");
  Litr::Uninit
}
fn sample_setter(_v:&mut Instance, set:Ident, to:Litr) {
  println!("dll set: {set}, {to:?}");
}

pub fn main(module: &mut NativeInterface) {
  SAMPLE.new("Sample");
  module.export_cls(SAMPLE.clone());
  SAMPLE.onclone(|v|{
    println!("cloned!");
    v.clone()
  });
  SAMPLE.next(|v|{
    if v.v2<v.v1 {
      return Symbol::iter_end();
    }
    let res = Litr::Uint(v.v1);
    v.v1 += 1;
    res
  });
  SAMPLE.index_get(|_v,i|{
    println!("get:[{:?}]",&*i);
    Litr::Float(2.55)
  });
  SAMPLE.index_set(|_v,i,val|{
    println!("set:[{:?}] = {:?}", &*i, &*val);
  });
  SAMPLE.onclone(|v|{
    println!("clone!");
    v.clone()
  });
  SAMPLE.ondrop(|_|{
    println!("drop!");
  });
  SAMPLE.static_method("new", sample_new);
  SAMPLE.method("see", sample_see);
  SAMPLE.getter(sample_getter);
  SAMPLE.setter(sample_setter);
  module.export_fn("test", test);
}
