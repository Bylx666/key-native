//! 由于已经决定了Rust为唯一Native编程语言，就不需要考虑C的行为了
//! 
//! 数据类型和调用约定都可以直接使用Rust标准库
type Scope = ();
#[derive(Debug, Clone, Copy)]
pub struct Interned {
  pub p: &'static Box<[u8]>
}
impl std::ops::Deref for Interned {
  type Target = [u8];
  fn deref(&self) -> &Self::Target {
    &*self.p
  }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub enum Litr {
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Box<Function>), 
  Str    (Box<String>),
  Buffer (Box<Vec<u8>>),
  List   (Box<Vec<Litr>>),
  Obj,
  Inst   (Box<()>),
  Ninst  (Box<Instance>)
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


pub type NativeFn = fn(Vec<Litr>)-> Litr;
pub type NativeMethod = fn(v: &mut Instance, args:Vec<Litr>)-> Litr;
pub type Getter = fn(get:Interned)-> Litr;
pub type Setter = fn(set:Interned, to:Litr);
pub type IndexGetter = fn(get:usize)-> Litr;
pub type IndexSetter = fn(set:usize, to:Litr);

/// Getter占位符，什么都不做
fn getter(_get:Interned)-> Litr {Litr::Uninit}
/// Setter占位符
fn setter(_set:Interned, _to:Litr) {}
/// index gettet占位符
fn igetter(_get:usize)-> Litr {Litr::Uninit}
/// index setter占位符
fn isetter(_set:usize, _to:Litr) {}
/// onclone ondrop的占位符
fn method(_v:&mut Instance, _args:Vec<Litr>)-> Litr {Litr::Uninit}


/// INTERN占位符，不应当被可及(needs to be unreachable)
fn _intern(s:&[u8])-> Interned {unsafe{std::mem::transmute(1usize)}}
/// intern函数本体。将其pub是未定义行为。
static mut INTERN:fn(&[u8])-> Interned = _intern;
/// 将字符串缓存为指针
pub fn intern(s:&[u8])-> Interned {
  unsafe{ INTERN(s) }
}

/// err占位符，不应被可及
fn _err(s:&str)->! {panic!()}
static mut ERR:fn(&str)->! = _err;
pub fn err(s:&str)->! {
  unsafe {ERR(s)}
}

/// 原生类型定义
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ClassInner {
  name: Interned,
  getter: Getter,
  setter: Setter,
  igetter: IndexGetter,
  isetter: IndexSetter,
  onclone: NativeMethod,
  ondrop: NativeMethod,
  statics: Vec<(Interned, NativeFn)>,
  methods: Vec<(Interned, NativeMethod)>
}

/// 原生类型指针
/// 
/// 使用时会提示static mut需要unsafe块，可以包个大unsafe块专门写Class内容
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Class {
  p: *mut ClassInner
}
impl Class {
  /// 为static创建一个空指针
  /// 
  /// 要在此后调用其new方法才能访问
  pub const fn uninit()-> Self {
    Class { p: std::ptr::null_mut() }
  }
  /// 为Class内部创建一个新类
  /// 
  /// 重复调用会引起一个ClassInner的内存泄漏
  pub fn new(&mut self, name:&[u8]) {
    let v = ClassInner { 
      name:intern(name), 
      getter, setter, igetter, isetter, 
      onclone: method, ondrop: method, 
      statics: Vec::new(), methods: Vec::new() 
    };
    self.p = Box::into_raw(Box::new(v))
  }
  /// 为此类创建一个实例
  /// 
  /// v是两个指针长度的内容，可以传任何东西然后as或者transmute
  pub fn create(&self, v1:usize, v2:usize)-> Box<Instance> {
    Box::new(Instance { cls: self.clone(), v1, v2 })
  }
  /// 设置getter, 用来处理.运算符
  pub fn getter(&self, f:Getter) {
    unsafe{(*self.p).getter = f;}
  }
  /// 设置setter, 用来处理a.b = c的写法
  pub fn setter(&self, f:Setter) {
    unsafe{(*self.p).setter = f;}
  }
  /// 设置index getter, 返回a[i]的值
  pub fn igetter(&self, f:IndexGetter) {
    unsafe{(*self.p).igetter = f;}
  }
  /// 设置index setter, 处理a[i] = b
  pub fn isetter(&self, f:IndexSetter) {
    unsafe{(*self.p).isetter = f;}
  }
  /// 添加一个方法
  pub fn method(&self, name:&[u8], f:NativeMethod) {
    unsafe{(*self.p).methods.push((intern(name), f));}
  }
  /// 添加一个静态方法
  pub fn static_method(&self, name:&[u8], f:NativeFn) {
     unsafe{(*self.p).statics.push((intern(name), f));}
  }
}

/// Ks解释器对Native做出的接口
#[repr(C)]
pub struct NativeInterface {
  intern: fn(&[u8])-> Interned,
  err: fn(&str)->!,
  funcs: *mut Vec<(Interned, NativeFn)>,
  classes: *mut Vec<Class>
}

impl NativeInterface {
  /// 导出函数
  pub fn export_fn(&mut self, name:&[u8], f:NativeFn) {
    unsafe{&mut *self.funcs}.push((intern(name), f))
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
  }
  crate::main(module);
}
