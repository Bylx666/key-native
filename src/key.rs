//! 由于已经决定了Rust为唯一Native编程语言，就不需要考虑C的行为了
//! 
//! 数据类型和调用约定都可以直接使用Rust标准库
#![allow(unused)]

use std::{cell::UnsafeCell, collections::HashMap};

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
impl std::fmt::Display for Ident{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.str())
  }
}

static mut FIND_VAR:fn(Scope, Ident)->Option<LitrRef> = |_,_|None;
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
  Buffer (Vec<u8>),
  List   (Vec<Litr>),
  Obj    (HashMap<Ident, Litr>),
  Inst   (()),
  Ninst  (Instance),
  Sym    (Symbol)
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

/// 原生类型实例
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Instance {
  pub v1:usize,
  pub v2:usize,
  cls: Class
}
impl Instance {
  pub fn set(&mut self, v1:usize, v2:usize) {
    self.v1 = v1;
    self.v2 = v2;
  }
}

/// 函数枚举
#[derive(Debug, Clone)]
pub enum Function {
  Local(Box<()>),
  Extern(Box<()>),
  Native(fn(Vec<Litr>)-> Litr)
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

/// 原生类型实例版的LitrRef
#[derive(Debug, Clone)]
pub enum InstanceRef {
  Ref(*mut Instance),
  Own(Instance)
}
impl std::ops::Deref for InstanceRef {
  type Target = Instance;
  fn deref(&self) -> &Self::Target {
    match self {
      InstanceRef::Ref(p)=> unsafe{&**p},
      InstanceRef::Own(b)=> b
    }
  }
}
impl std::ops::DerefMut for InstanceRef {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      InstanceRef::Ref(p)=> unsafe{&mut **p},
      InstanceRef::Own(b)=> b
    }
  }
}


pub type NativeFn = fn(Vec<LitrRef>, Scope)-> Litr;
pub type NativeMethod = fn(InstanceRef, args:Vec<LitrRef>, Scope)-> Litr;
pub type Getter = fn(InstanceRef, get:Ident)-> Litr;
pub type Setter = fn(InstanceRef, set:Ident, to:Litr);

/// intern函数本体。将其pub是未定义行为。
static mut INTERN:fn(&[u8])-> Ident = |_|unsafe{std::mem::transmute(1usize)};
/// 将字符串缓存为指针
pub fn intern(s:&[u8])-> Ident {
  unsafe{ INTERN(s) }
}

static mut ERR:fn(&str)->! = |s|panic!("{}",s);
pub fn err(s:&str)->! {
  unsafe {ERR(s)}
}

/// 原生类型定义
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ClassInner {
  name: Ident,
  statics: Vec<(Ident, NativeFn)>,
  methods: Vec<(Ident, NativeMethod)>,
  getter: Getter,
  setter: Setter,
  index_get: fn(&mut Instance, LitrRef)-> Litr,
  index_set: fn(&mut Instance, LitrRef, LitrRef),
  next: fn(&mut Instance)-> Litr,
  onclone: fn(&mut Instance)-> Instance,
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
      onclone:|v|v.clone(), 
      ondrop:|_|()
    };
    unsafe{*self.p.get() = Box::into_raw(Box::new(v))}
  }
  /// 为此类创建一个实例
  /// 
  /// v是两个指针长度的内容，可以传任何东西然后as或者transmute
  pub fn create(&self, v1:usize, v2:usize)-> Litr {
    Litr::Ninst(Instance { cls: self.clone(), v1, v2 })
  }
  impl_class_setter!{
    "设置getter, 用来处理`.`运算符"
    getter(Getter);
    "设置setter, 用来处理a.b = c的写法"
    setter(Setter);
    "设置index getter, 返回a[i]的值"
    index_get(fn(&mut Instance, LitrRef)-> Litr);
    "设置index setter, 处理a[i] = b"
    index_set(fn(&mut Instance, LitrRef, LitrRef));
    "设置迭代器, 处理for n:instance {}"
    next(fn(&mut Instance)-> Litr);
    "自定义复制行为(往往是赋值和传参)"
    onclone(fn(&mut Instance)-> Instance);
    "自定义垃圾回收回收行为(只需要写额外工作,不需要drop此指针)"
    ondrop(fn(&mut Instance));
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

/// Ks解释器对Native做出的接口
/// 
/// 在main外调用内部函数是UB
#[repr(C)]
pub struct NativeInterface {
  intern: fn(&[u8])-> Ident,
  err: fn(&str)->!,
  find_var: fn(Scope, Ident)-> Option<LitrRef>,
  funcs: *mut Vec<(Ident, NativeFn)>,
  classes: *mut Vec<Class>
}

impl NativeInterface {
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

#[export_name = "keymain"]
extern fn _main(module: &mut NativeInterface) {
  unsafe {
    INTERN = module.intern;
    ERR = module.err;
    FIND_VAR = module.find_var;
  }
  crate::main(module);
}
