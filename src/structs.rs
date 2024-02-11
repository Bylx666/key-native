//! 定义了模块会用到的基本类型

type Scope = ();

/// 变量或字面量
#[derive(Debug, Clone)]
pub enum Litr {
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Box<Executable>), // extern和Func(){} 都属于Func直接表达式
  Str    (Box<String>),
  Buffer (Box<Vec<u8>>),
  List   (Box<Vec<Litr>>),
  // Struct   {targ:Ident, cont:HashMap<Ident, Exprp>},    // 直接构建结构体
}


#[derive(Debug, Clone)]
pub enum Executable {
  Local(Box<LocalFunc>),             // 脚本内的定义
  Extern(Box<ExternFunc>),           // 脚本使用extern获取的函数
  Native(fn(Vec<Litr>)-> Litr)      // runtime提供的函数 
}

#[derive(Debug, Clone)]
pub struct LocalFunc;

#[derive(Debug, Clone)]
pub struct ExternFunc;
