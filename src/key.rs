//! 负责和解释器交互的部分
//! 
//! 此模块定义Key语言中原生模块可能用到的类型

use std::collections::HashMap;
use crate::Instance;

static mut INTERN:fn(&[u8])-> Ident = |_|unsafe{std::mem::transmute(1usize)};
/// 将字符串缓存为指针(和Key解释器用一个缓存池)
pub fn intern(s:&[u8])-> Ident {
  unsafe{ INTERN(s) }
}

pub static mut _KEY_LANG_PANIC:fn(&str)-> ! = |s|panic!("{}",s);
/// 用Key解释器的报错
#[macro_export]
macro_rules! kpanic {($($arg:tt)*)=> {
  unsafe{::key_native::key::_KEY_LANG_PANIC(&format!($($arg)*))}
}}

/// premain函数接收的函数表
#[repr(C)]
struct PreMain {
  intern: fn(&[u8])-> Ident,
  err: fn(&str)->!,
  find_var: fn(Scope, Ident)-> Option<LitrRef>,
}

#[no_mangle]
extern fn premain(module: &PreMain) {
  unsafe {
    INTERN = module.intern;
    _KEY_LANG_PANIC = module.err;
    FIND_VAR = module.find_var;
  }
}

/// 一个合法的标识符, 可以理解为字符串的指针
#[derive(Debug, Clone, Copy)]
pub struct Ident {
  pub p: &'static Box<[u8]>
}
impl Ident {
  /// 将ident作为字符串
  pub fn str(&self)-> String {
    String::from_utf8_lossy(&self.p).into_owned()
  }
  /// 获得ident的slice
  pub fn slice(&self)-> &[u8] {
    self
  }
}
impl std::ops::Deref for Ident {
  type Target = [u8];
  fn deref(&self) -> &Self::Target {
    &*self.p
  }
}
impl std::fmt::Display for Ident {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.str())
  }
}

pub(super) static mut FIND_VAR:fn(Scope, Ident)-> Option<LitrRef> = |_,_|None;

#[derive(Debug, Clone, Copy)]
pub struct Scope(*mut ());
impl Scope {
  pub fn find_var(self, s:&str)-> Option<LitrRef> {
    unsafe{FIND_VAR(self, intern(s.as_bytes()))}
  }
}

#[derive(Debug, Clone)]
pub enum Litr {
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Function), 
  Str    (String),
  Buf    (Vec<u8>),
  List   (Vec<Litr>),
  Obj    (HashMap<Ident, Litr>),
  Inst   (()),
  Ninst  (Instance),
  Sym    (Symbol)
}

/// 函数枚举
#[derive(Debug, Clone)]
pub enum Function {
  Local(Box<()>),
  Extern(Box<()>),
  Native(fn(Vec<Litr>)-> Litr)
}

#[derive(Debug, Clone)]
pub enum Symbol {
  IterEnd,
  Reserved
}
impl Symbol {
  pub fn iter_end()-> Litr {
    Litr::Sym(Symbol::IterEnd)
  }
}

/// 可能是引用的Litr
pub enum LitrRef {
  Ref(*mut Litr),
  Own(Litr)
}
impl LitrRef {
  /// 消耗CalcRef返回内部值
  pub fn own(self)-> Litr {
    match self {
      LitrRef::Ref(p)=> unsafe {(*p).clone()}
      LitrRef::Own(v)=> v
    }
  }
}
impl std::ops::Deref for LitrRef {
  type Target = Litr;
  fn deref(&self) -> &Self::Target {
    match self {
      LitrRef::Ref(p)=> unsafe{&**p},
      LitrRef::Own(b)=> b
    }
  }
}
impl std::ops::DerefMut for LitrRef {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      LitrRef::Ref(p)=> unsafe{&mut **p},
      LitrRef::Own(b)=> b
    }
  }
}
