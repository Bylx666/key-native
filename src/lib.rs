#![allow(unused)]
mod structs;
mod key;

use structs::Litr;
use key::{
  export_func,
};

fn test(args:Vec<Litr>)->Litr {
  println!("哎呀{args:?}");
  Litr::Uninit
}

#[export_name = "keymain"]
extern fn main() {
  export_func(b"test", test);
}