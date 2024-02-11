//! 定义了与Key程序交互需要的函数
//! 
//! 由于已经决定了Rust为唯一Native编程语言，就不需要考虑C的行为了
//! 
//! 数据类型和调用约定都可以直接使用Rust标准库

use crate::structs::{
  Executable, Litr
};

pub type NativeFn = fn(Vec<Litr>)->Litr;

/// 导出的函数列表 (函数名, 函数指针)
static mut EXPORTED_FUNCS:Vec<(Box<[u8]>, NativeFn)> = Vec::new();

#[export_name = "GetExportedFuncs"]
extern fn exported_funcs(p:*mut Vec<(Box<[u8]>, NativeFn)>) {
  unsafe{*p = std::mem::take(&mut EXPORTED_FUNCS)}
}

pub fn export_func(sym:&[u8], f:NativeFn) {
  // 标识符不用回收
  unsafe{EXPORTED_FUNCS.push((Box::from(sym), f))}
}
