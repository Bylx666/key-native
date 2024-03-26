#![allow(unused)]
mod key;
use std::{alloc::{dealloc, Layout}, mem::transmute, ptr::drop_in_place};

use key::*;

static SAMPLE:Class = Class::uninit();

fn sample_getter(_v:&Instance, get:Ident)-> Litr {
  println!("dll get: {get}");
  Litr::Uninit
}
fn sample_setter(_v:&mut Instance, set:Ident, to:Litr) {
  println!("dll set: {set}, {to:?}");
}

pub fn main(module: &mut NativeInterface) {
  SAMPLE.new("A");

  module.export_cls(SAMPLE.clone());
  SAMPLE.onclone(|v|{
    println!("cloned!");
    v.clone()
  });
  SAMPLE.next(|v|{
    if v.w<v.v {
      return Symbol::iter_end();
    }
    let res = Litr::Uint(v.v);
    v.v += 1;
    res
  });
  SAMPLE.index_get(|v,i|{
    let n:&mut Vec<u8> = unsafe {transmute(v.v)};
    let i = match &*i {
      Litr::Int(n)=> *n as usize,
      _=> 0
    };
    Litr::Uint(n[i] as usize)
  });
  SAMPLE.index_set(|_v,i,val|{
    println!("set:[{:?}] = {:?}", &*i, &*val);
  });
  SAMPLE.onclone(|v|{
    println!("clone!");
    v.clone()
  });
  SAMPLE.ondrop(|v|{
  });
  SAMPLE.static_method("new", |_,_|{
    let n = Box::into_raw(Box::new(vec![1,3,5,7u8]));
    unsafe {
      struct N (Vec<u8>);
      impl Drop for N {
        fn drop(&mut self) {
          println!("drop")
        }
      }
      let boxe = Box::into_raw(Box::new(N(vec![1,5,4,3])));
      drop_in_place(boxe);
      dealloc(boxe as _, Layout::new::<Vec<u8>>());
      println!("{boxe:?} {:?}", (&*boxe).0);
    };
    let inst = SAMPLE.create(n as usize, 0);
    inst
  });
  SAMPLE.method("see", |v,_,_|{
    let n:&mut Vec<u8> = unsafe {transmute(v.v)};
    Litr::Buf(n.clone())
  });
  SAMPLE.getter(sample_getter);
  SAMPLE.setter(sample_setter);
}
