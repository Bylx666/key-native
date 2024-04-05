//! Key语言的Native Module库
//! 
//! 参见[Native Module开发](https://docs.subkey.top/native)
//! 
//! 由于已经决定了Rust为唯一Native编程语言，就不需要考虑C的行为了
//! 
//! 数据类型都可以直接使用Rust标准库, 调用约定也可以使用extern "Rust"
#![allow(unused)]

use std::{cell::UnsafeCell, collections::HashMap, mem::transmute};

pub mod key;
use key::*;

/// 原生类型实例
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Instance {
  pub v: usize,
  pub w: usize,
  pub cls: Class
}


/// Key语言的原生模块的函数
pub type NativeFn = fn(Vec<LitrRef>, Scope)-> Litr;
/// Key语言的原生类型中为模块定义的方法
pub type NativeMethod = fn(&mut Instance, args:Vec<LitrRef>, Scope)-> Litr;

/// 原生类型定义
#[repr(C)]
#[derive(Debug, Clone)]
struct ClassInner {
  name: Ident,
  statics: Vec<(Ident, NativeFn)>,
  methods: Vec<(Ident, NativeMethod)>,
  getter: fn(&Instance, get:Ident)-> Litr,
  setter: fn(&mut Instance, set:Ident, to:Litr),
  index_get: fn(&Instance, LitrRef)-> Litr,
  index_set: fn(&mut Instance, LitrRef, LitrRef),
  next: fn(&mut Instance)-> Litr,
  to_str: fn(&Instance)-> String,
  onclone: fn(&Instance)-> Instance,
  ondrop: fn(&mut Instance)
}

/// 原生类型指针
#[repr(transparent)]
#[derive(Debug)]
pub struct Class {
  p: UnsafeCell<*mut ClassInner>
}
impl Clone for Class {
  fn clone(&self) -> Self {
    Class {p: UnsafeCell::new(unsafe{*self.p.get()})}
  }
}

unsafe impl Sync for Class {}

macro_rules! impl_class_setter {($($doc:literal $f:ident($t:ty);)*) => {
  $(
    #[doc = $doc]
    pub fn $f(&self, f:$t) {
      unsafe{(**self.p.get()).$f = f;}
    }
  )*
}}
impl Class {
  /// 为static创建一个空指针
  /// 
  /// 要在此后调用其new方法才能访问
  pub const fn uninit()-> Self {
    Class { p: UnsafeCell::new(std::ptr::null_mut()) }
  }
  /// 为Class内部创建一个新类
  /// 
  /// 重复调用会引起一个ClassInner的内存泄漏
  pub fn new(&self, name:&str) {
    let v = ClassInner { 
      name:intern(name.as_bytes()), 
      getter:|_,_|Litr::Uninit, 
      setter:|_,_,_|(), 
      statics: Vec::new(), 
      methods: Vec::new() ,
      index_get:|_,_|Litr::Uninit, 
      index_set:|_,_,_|(),
      next:|_|Symbol::iter_end(), 
      to_str: |v|format!("{} {{ Native }}", unsafe{&**v.cls.p.get()}.name),
      onclone:|v|v.clone(), 
      ondrop:|_|()
    };
    unsafe{*self.p.get() = Box::into_raw(Box::new(v))}
  }
  /// 为此类创建一个实例
  /// 
  /// v是两个指针长度的内容，可以传任何东西然后as或者transmute
  pub fn create(&self, v:usize, w:usize)-> Litr {
    Litr::Ninst(Instance { cls: self.clone(), v, w })
  }
  impl_class_setter!{
    "设置getter, 用来处理`.`运算符"
    getter(fn(&Instance, get:Ident)-> Litr);
    "设置setter, 用来处理a.b = c的写法"
    setter(fn(&mut Instance, set:Ident, to:Litr));
    "设置index getter, 返回a[i]的值"
    index_get(fn(&Instance, LitrRef)-> Litr);
    "设置index setter, 处理a[i] = b"
    index_set(fn(&mut Instance, LitrRef, LitrRef));
    "设置迭代器, 处理for n:instance {}"
    next(fn(&mut Instance)-> Litr);
    "自定义复制行为(往往是赋值和传参)"
    onclone(fn(&Instance)-> Instance);
    "自定义垃圾回收回收行为(只需要写额外工作,不需要drop此指针)"
    ondrop(fn(&mut Instance));
    "自定义Str::from得到的字符串"
    to_str(fn(&Instance)-> String);
  }
  /// 添加一个方法
  pub fn method(&self, name:&str, f:NativeMethod) {
    unsafe{(**self.p.get()).methods.push((intern(name.as_bytes()), f));}
  }
  /// 添加一个静态方法
  pub fn static_method(&self, name:&str, f:NativeFn) {
     unsafe{(**self.p.get()).statics.push((intern(name.as_bytes()), f));}
  }
}


/// 传进main函数的可写空模块
#[repr(C)]
pub struct NativeModule {
  funcs: *mut Vec<(Ident, NativeFn)>,
  classes: *mut Vec<Class>
}
impl NativeModule {
  /// 导出函数
  pub fn export_fn(&mut self, name:&str, f:NativeFn) {
    unsafe{&mut *self.funcs}.push((intern(name.as_bytes()), f))
  }
  /// 导出一个类
  /// 
  /// 可以提前调用此函数，之后再追加方法
  pub fn export_cls(&mut self, cls:Class) {
    unsafe{&mut *self.classes}.push(cls);
  }
}

/// 将全局panic用kpanic代理
pub fn use_kpanic() {
  std::panic::set_hook(Box::new(|inf|{
    let s = if let Some(s) = inf.payload().downcast_ref::<String>() {s}
    else if let Some(s) = inf.payload().downcast_ref::<&str>() {s}else {"错误"};
    unsafe{key::_KEY_LANG_PANIC(s)};
  }));
}

pub mod prelude {
  pub use crate::key::{Litr, LitrRef, Scope};
  pub use crate::{NativeModule, Class, kpanic};
}
