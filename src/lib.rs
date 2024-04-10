//! Key语言的Native Module库
//! 
//! 参见[Native Module开发](https://docs.subkey.top/native)
//! 
//! 由于已经决定了Rust为唯一Native编程语言，就不需要考虑C的行为了
//! 
//! 数据类型都可以直接使用Rust标准库, 调用约定也可以使用extern "Rust"
#![feature(const_ptr_is_null)]
#![allow(unused)]

use std::{cell::UnsafeCell, collections::HashMap, mem::transmute};

pub mod key;
use key::*;

/// 原生类型实例
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Instance {
  /// 原生类实例的第一个可用值
  pub v: usize,
  /// 原生类实例的第二个可用值
  pub w: usize,
  /// 实例的原生类指针
  pub cls: Class
}

/// drop出错时可以判断你是不是没设置clone函数
fn default_onclone(v:&Instance)-> Instance {
  v.clone()
}

impl Instance {
  /// 以T的格式读取v值, 请保证v是有效指针且可写
  /// 
  /// 虽说不带unsafe, 但还是很容易UB的
  pub fn read<T>(&self)-> &mut T {
    unsafe { &mut*(self.v as *mut T) }
  }
  /// w的read
  pub fn readw<T>(&self)-> &mut T {
    unsafe { &mut*(self.w as *mut T) }
  }

  /// 执行T的析构函数, 并把v覆盖为新值
  /// 
  /// 请务必保证set前后的指针类型相同
  pub fn set<T>(&mut self, v:T) {
    self.dropv::<T>();
    self.v = to_ptr(v);
  }
  /// w的set
  pub fn setw<T>(&mut self, w:T) {
    self.dropw::<T>();
    self.w = to_ptr(w);
  }

  /// 不pub, 所以不管安全性
  fn dealloc<T>(&self, p:*mut T) {
    assert!(!p.is_null(), "无法析构空指针.{}\n  详见https://docs.subkey.top/native/4.class", {
      if unsafe{&**self.cls.p.get()}.onclone == default_onclone 
        {"\n  你或许应该为class定义onclone"}else {""}
    });
    unsafe {
      std::ptr::drop_in_place(p);
      let lo = std::alloc::Layout::new::<T>();
      std::alloc::dealloc(p as _, lo);
    }
  }
  /// 将v作为T的指针, 将T的内存释放
  pub fn dropv<T>(&mut self) {
    self.dealloc(self.v as *mut T);
    self.v = 0;
  }
  /// w版drop
  pub fn dropw<T>(&mut self) {
    self.dealloc(self.w as *mut T);
    self.w = 0;
  }
}


/// Key语言的原生模块的函数
pub type NativeFn = fn(Vec<LitrRef>, Scope)-> Litr;
/// Key语言的原生类型中为模块定义的方法
pub type NativeMethod = fn(&mut Instance, args:Vec<LitrRef>, Scope)-> Litr;

/// 原生类型定义
#[derive(Debug, Clone)]
#[repr(C)]
struct ClassInner {
  statics: Vec<(Ident, NativeFn)>,
  methods: Vec<(Ident, NativeMethod)>,
  name: Ident,
  getter: fn(&Instance, get:Ident)-> Litr,
  setter: fn(&mut Instance, set:Ident, to:Litr),
  index_get: fn(&Instance, LitrRef)-> Litr,
  index_set: fn(&mut Instance, LitrRef, Litr),
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
      self.assert();
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
  #[inline]
  /// 判断是否new过了
  const fn assert(&self) {
    unsafe{
      assert!(!(
        *((&self.p) as *const UnsafeCell<*mut ClassInner> as *const *mut ClassInner)
      ).is_null(), "请先为Class调用new方法"
    )}
  }
  /// 为Class内部创建一个新类
  /// 
  /// 重复调用会引起一个ClassInner的内存泄漏
  pub fn new(&self, name:&str) {
    let v = ClassInner { 
      getter:|_,_|Litr::Uninit, 
      setter:|_,_,_|(), 
      statics: Vec::new(), 
      methods: Vec::new() ,
      index_get:|_,_|Litr::Uninit, 
      index_set:|_,_,_|(),
      next:|_|key::Sym::iter_end(), 
      to_str: |v|format!("{} {{ Native }}", unsafe{&**v.cls.p.get()}.name),
      onclone:default_onclone, 
      ondrop:|_|(),
      name:intern(name.as_bytes())
    };
    let p = Box::into_raw(Box::new(v));
    unsafe{*self.p.get() = p}
  }
  /// 为此类创建一个实例
  /// 
  /// v是两个指针长度的内容，可以传任何东西然后as或者transmute
  pub fn create_raw(&self, v:usize, w:usize)-> Instance {
    self.assert();
    Instance { cls: self.clone(), v, w }
  }
  /// 为此类创建一个实例并包装为Litr
  /// 
  /// v是两个指针长度的内容，可以传任何东西然后as或者transmute
  pub fn create(&self, v:usize, w:usize)-> Litr {
    Litr::Ninst(self.create_raw(v, w))
  }
  impl_class_setter!{
    "设置getter, 用来处理`.`运算符"
    getter(fn(&Instance, get:Ident)-> Litr);
    "设置setter, 用来处理a.b = c的写法"
    setter(fn(&mut Instance, set:Ident, to:Litr));
    "设置index getter, 返回a[i]的值"
    index_get(fn(&Instance, LitrRef)-> Litr);
    "设置index setter, 处理a[i] = b"
    index_set(fn(&mut Instance, LitrRef, Litr));
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
    self.assert();
    unsafe{(**self.p.get()).methods.push((intern(name.as_bytes()), f));}
  }
  /// 添加一个静态方法
  pub fn static_method(&self, name:&str, f:NativeFn) {
    self.assert();
    unsafe{(**self.p.get()).statics.push((intern(name.as_bytes()), f));}
  }
}

impl PartialEq for Class {
  fn eq(&self, other: &Self) -> bool {
    unsafe{*self.p.get() == *other.p.get()}
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
    cls.assert();
    unsafe{&mut *self.classes}.push(cls);
  }
}

#[macro_export]
macro_rules! get_arg {
  ($args:ident[$i:literal])=> {
    match $args.get($i) {
      Some(v)=> &**v,
      _=> panic!("至少需要{}个参数", $i+1)
    }
  };
  ($args:ident[$i:literal]:$t:ident)=> {
    match $args.get($i) {
      Some(v)=> match &**v {
        Litr::$t(v)=> v,
        _=> panic!("第{}个参数必须是{}", $i+1, stringify!($t))
      },
      _=> panic!("至少需要{}个参数", $i+1)
    }
  };
  ($args:ident[$i:literal]:$t:ident?$def:expr)=> {
    $args.get($i).map_or($def, |val|{
      match &**val {
        Litr::$t(n)=> *n as u64,
        _=> $def
      }
    });
  }
}

/// 将一个非指针的值转为方便实例储存的指针
#[inline]
pub fn to_ptr<T>(v:T)-> usize {
  Box::into_raw(Box::new(v)) as usize
}

/// 增加该作用域的引用计数
/// 
/// 警告: 你应当在作用域用完时, 
/// 为此调用对应的`outlive_dec`,
/// 否则会导致该作用域以上所有作用域无法回收
pub fn outlive_inc(s:Scope) {
  unsafe{((*FUNCTABLE).outlive_inc)(s)};
}
/// 减少该作用域的引用计数
pub fn outlive_dec(s:Scope) {
  unsafe{((*FUNCTABLE).outlive_dec)(s)};
}

/// 阻塞主线程的计数+1
pub fn wait_inc() {
  unsafe{((*FUNCTABLE).wait_inc)()};
}
/// 阻塞主线程的计数-1
pub fn wait_dec() {
  unsafe{((*FUNCTABLE).wait_dec)()};
}

pub mod prelude {
  pub use crate::key::{Litr, LitrRef, Scope, Ident};
  pub use crate::{NativeModule, Class, get_arg, Instance, to_ptr};
}
