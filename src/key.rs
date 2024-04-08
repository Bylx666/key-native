//! 负责和解释器交互的部分
//! 
//! 此模块定义Key语言中原生模块可能用到的类型

pub use std::collections::HashMap;
use std::fmt::Debug;
use crate::Instance;


/// premain函数接收的函数表
#[repr(C)]
pub struct FuncTable {
  pub intern: fn(&[u8])-> Ident,
  pub err: fn(&str)->!,
  pub find_var: fn(Scope, Ident)-> Option<LitrRef>,
  pub let_var: fn(Scope, Ident, Litr),
  pub const_var: fn(Scope, Ident),
  pub using: fn(Scope, Ident, crate::Class),
  pub call_local: fn(&LocalFunc, Vec<Litr>)-> Litr,
  pub call_at: fn(Scope, *mut Litr, &LocalFunc, Vec<Litr>)-> Litr,
  pub get_self: fn(Scope)-> *mut Litr,
  pub get_parent: fn(Scope)-> Option<Scope>,
  pub outlive_inc: fn(Scope),
  pub outlive_dec: fn(Scope),
  pub symcls: fn()-> crate::Class
}
pub static mut FUNCTABLE:*const FuncTable = std::ptr::null();

/// 将字符串缓存为指针(和Key解释器用一个缓存池)
pub fn intern(s:&[u8])-> Ident {
  unsafe{ ((*FUNCTABLE).intern)(s) }
}

#[no_mangle]
extern fn premain(table: &FuncTable) {
  // 使用kpanic
  std::panic::set_hook(Box::new(|inf|{
    let s = if let Some(s) = inf.payload().downcast_ref::<String>() {s}
    else if let Some(s) = inf.payload().downcast_ref::<&str>() {s}else {"错误"};
    unsafe{((*FUNCTABLE).err)(s)};
  }));

  unsafe {
    FUNCTABLE = table;
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

/// Key语言中的作用域
#[derive(Debug, Clone, Copy)]
pub struct Scope(*mut ());
impl Scope {
  /// 在此作用域找一个变量
  pub fn find_var(self, s:&str)-> Option<LitrRef> {
    unsafe{((*FUNCTABLE).find_var)(self, intern(s.as_bytes()))}
  }
  /// 在此作用域定义一个变量
  pub fn let_var(self, s:&str, v:Litr) {
    unsafe{((*FUNCTABLE).let_var)(self, intern(s.as_bytes()), v)}
  }
  /// 在此作用域锁定一个变量
  pub fn const_var(self, s:&str) {
    unsafe{((*FUNCTABLE).const_var)(self, intern(s.as_bytes()))}
  }
  /// 获取该作用域的self
  pub fn get_self(self)-> *mut Litr {
    unsafe{((*FUNCTABLE).get_self)(self)}
  }
  /// 获取该作用域的父作用域(如果有的话)
  pub fn get_parent(self)-> Option<Scope> {
    unsafe{((*FUNCTABLE).get_parent)(self)}
  }
  /// 为作用域创建一个`use`(`class A = m-:B`的行为)
  /// 
  /// 即使你的类没有被`export_cls`, 也可以被`using`正确使用
  pub fn using(self, name:&str, cls:crate::Class) {
    unsafe{((*FUNCTABLE).using)(self, intern(name.as_bytes()), cls)}
  }
}

/// Key语言中的基本类型
#[derive(Debug, Clone)]
pub enum Litr {
  /// `()`, `uninit`
  Uninit,

  /// `-1`, `-1i`
  Int    (isize),
  /// `1u`
  Uint   (usize),
  /// `1.0`, `1f`
  Float  (f64),
  /// `true`, `false`
  Bool   (bool),

  /// `||{..}`
  Func   (Function), 
  /// `"字符串"`
  Str    (String),
  /// `'buf,可以显示utf8'`
  Buf    (Vec<u8>),
  /// `[1,2,3]`
  List   (Vec<Litr>),
  /// 哈希表`{a:1, b:2}`
  Obj    (HashMap<Ident, Litr>),
  /// 本地声明的类实例, 原生模块不可干涉
  Inst   ([usize;3]),
  /// 原生模块的类实例`m-:A::new()`
  Ninst  (Instance)
}

/// 函数枚举
#[derive(Debug, Clone)]
pub enum Function {
  Native(crate::NativeFn),
  Local(LocalFunc),
  Extern([usize;4])
}
impl Function {
  /// 调用函数, 需要传入`LitrRef`而非`Litr`的参数
  /// 
  /// 如需传入`Litr`参数请判断是否为本地函数, 调用`LocalFunc`的`call``
  /// 
  /// 该函数不会调用extern函数
  pub fn call(&self, args:Vec<LitrRef>, cx:Scope)-> Litr {
    match self {
      Function::Local(f)=> unsafe{
        let args = args.into_iter().map(|n|n.own()).collect();
        ((*FUNCTABLE).call_at)(cx, cx.get_self(), f, args)
      }
      Function::Native(f)=> f(args, cx),
      _=> Litr::Uninit
    }
  }
}

/// Key脚本内定义的本地函数
#[derive(Debug)]
#[repr(C)]
pub struct LocalFunc {
  ptr:*const (),
  scope: Scope,
}

// 因为Key允许开发者实现事件循环一样的效果
// 所以必须保证原生模块的函数持有Key函数时
// 该Key函数不能过期(因此需要实现Clone和Drop)
impl Clone for LocalFunc {
  fn clone(&self) -> Self {
    let scope = self.scope;
    unsafe{((*FUNCTABLE).outlive_inc)(scope)};
    LocalFunc { ptr: self.ptr, scope }
  }
}
impl Drop for LocalFunc {
  fn drop(&mut self) {
    unsafe{((*FUNCTABLE).outlive_dec)(self.scope)}
  }
}

impl LocalFunc {
  /// 调用Key本地函数
  pub fn call(&self, args:Vec<Litr>)-> Litr {
    unsafe{((*FUNCTABLE).call_local)(self, args)}
  }
  /// 在指定作用域调用该函数
  /// 
  /// 比起call多了两个参数: 
  /// 
  /// - `scope`: 要执行在哪个作用域, 可以使用`f.scope`缺省
  /// 
  /// - `kself`: Key脚本中的`self`指向, 可以使用`f.scope.get_self()`缺省
  pub fn call_at(&self, scope:Scope, kself:*mut Litr, args:Vec<Litr>)-> Litr {
    unsafe{((*FUNCTABLE).call_at)(scope, kself, self, args)}
  }
}


/// Key语言的语法标志
pub struct Sym;
impl Sym {
  pub const ITER_END:usize = 1;
  pub fn is_sym(v:&Instance)-> bool {
    v.cls == (unsafe{&*FUNCTABLE}.symcls)()
  }
  pub fn iter_end()-> Litr {
    (unsafe{&*FUNCTABLE}.symcls)().create(Self::ITER_END, 0)
  }
}

/// `Litr`的引用(是有生命周期的)
/// 
/// 注意如果你需要将`Litr`转移到别的线程等需要保证其生命周期的行为时
/// 
/// 调用`own`方法直接获取`Litr`的所有权
pub enum LitrRef {
  Ref(*mut Litr),
  Own(Litr)
}
impl LitrRef {
  /// 消耗CalcRef获取Litr所有权
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

impl Debug for LitrRef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("{:?}", &**self))
  }
}