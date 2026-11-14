//! Runs a binary template over a file and builds the result tree.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use egui::Color32;

use crate::i18n::{self, Lang};
use crate::tparse::{parse, Attrs, Expr, Stmt};

use crate::tr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prim {
    I8,
    U8,
    Char,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl Prim {
    pub fn size(self) -> usize {
        match self {
            Prim::I8 | Prim::U8 | Prim::Char => 1,
            Prim::I16 | Prim::U16 => 2,
            Prim::I32 | Prim::U32 | Prim::F32 => 4,
            Prim::I64 | Prim::U64 | Prim::F64 => 8,
        }
    }
}

fn prim_of(name: &str) -> Option<Prim> {
    Some(match name {
        "char" | "CHAR" => Prim::Char,
        "byte" | "int8" => Prim::I8,
        "uchar" | "ubyte" | "uint8" | "BYTE" | "UCHAR" | "bool" => Prim::U8,
        "short" | "int16" | "SHORT" => Prim::I16,
        "ushort" | "uint16" | "WORD" | "USHORT" | "wchar_t" => Prim::U16,
        "int" | "int32" | "long" | "INT" | "LONG" => Prim::I32,
        "uint" | "uint32" | "ulong" | "DWORD" | "UINT" | "ULONG" | "time_t" => Prim::U32,
        "int64" | "__int64" | "quad" | "INT64" => Prim::I64,
        "uint64" | "__uint64" | "uquad" | "QWORD" | "UINT64" | "FILETIME" => Prim::U64,
        "float" | "FLOAT" => Prim::F32,
        "double" | "DOUBLE" => Prim::F64,
        _ => return None,
    })
}

pub fn is_primitive(name: &str) -> bool {
    prim_of(name).is_some() || name == "string"
}

#[derive(Clone, Debug)]
pub enum Val {
    Int(i64),
    Float(f64),
    Str(String),
}

impl Val {
    fn as_i(&self) -> i64 {
        match self {
            Val::Int(i) => *i,
            Val::Float(f) => *f as i64,
            Val::Str(s) => s.parse().unwrap_or(0),
        }
    }
    fn as_f(&self) -> f64 {
        match self {
            Val::Int(i) => *i as f64,
            Val::Float(f) => *f,
            Val::Str(s) => s.parse().unwrap_or(0.0),
        }
    }
    fn truthy(&self) -> bool {
        match self {
            Val::Int(i) => *i != 0,
            Val::Float(f) => *f != 0.0,
            Val::Str(s) => !s.is_empty(),
        }
    }
    fn text(&self) -> String {
        match self {
            Val::Int(i) => i.to_string(),
            Val::Float(f) => f.to_string(),
            Val::Str(s) => s.clone(),
        }
    }
}

/// One entry of the template result tree.
#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub type_name: String,
    pub offset: usize,
    pub size: usize,
    pub value: Option<Val>,
    pub display: String,
    pub comment: Option<String>,
    pub children: Vec<Node>,
    /// For primitive arrays: element type, so huge arrays need no child nodes.
    pub elem: Option<Prim>,
    pub count: usize,
    pub color: Option<Color32>,
    pub is_struct: bool,
    /// Single primitive value: its type and byte order, so it can be edited.
    pub prim: Option<Prim>,
    pub le: bool,
}

impl Node {
    fn new(name: &str, type_name: String, offset: usize) -> Self {
        Node {
            name: name.to_string(),
            type_name,
            offset,
            size: 0,
            value: None,
            display: String::new(),
            comment: None,
            children: Vec::new(),
            elem: None,
            count: 0,
            color: None,
            is_struct: false,
            prim: None,
            le: true,
        }
    }
}

pub struct TemplateResult {
    pub name: String,
    pub source: String,
    /// Document revision the result was made from.
    pub revision: u64,
    pub nodes: Vec<Node>,
    pub log: Vec<String>,
    pub error: Option<String>,
    pub expanded: HashSet<String>,
}

enum Flow {
    Normal,
    Break,
    Continue,
    Return,
}

#[derive(Clone)]
enum TypeDef {
    Struct(Rc<Vec<Stmt>>),
    Enum(String, Vec<(String, i64)>),
}

struct Interp<'a> {
    data: &'a [u8],
    pos: usize,
    le: bool,
    lang: Lang,
    types: HashMap<String, TypeDef>,
    aliases: HashMap<String, String>,
    consts: HashMap<String, i64>,
    /// Stack of structs being built; the bottom one is the file itself.
    frames: Vec<Vec<Node>>,
    locals: Vec<HashMap<String, Val>>,
    log: Vec<String>,
    steps: u64,
    nodes: usize,
}

const MAX_STEPS: u64 = 5_000_000;
const MAX_NODES: usize = 300_000;
const CHILD_LIMIT: usize = 4096;
const EXIT: &str = "__exit__";

impl<'a> Interp<'a> {
    fn resolve_alias(&self, t: &str) -> String {
        let mut t = t.to_string();
        for _ in 0..32 {
            match self.aliases.get(&t) {
                Some(x) => t = x.clone(),
                None => break,
            }
        }
        t
    }

    fn tick(&mut self) -> Result<(), String> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(tr!(self.lang, "err.toomanysteps"));
        }
        Ok(())
    }

    fn read_prim(&self, p: Prim, at: usize) -> Result<Val, String> {
        let n = p.size();
        if at + n > self.data.len() {
            return Err(tr!(self.lang, "err.eof_read", at = format!("{at:X}")));
        }
        let mut buf = [0u8; 8];
        buf[..n].copy_from_slice(&self.data[at..at + n]);
        if !self.le {
            buf[..n].reverse();
        }
        let u = u64::from_le_bytes(buf);
        Ok(match p {
            Prim::I8 | Prim::Char => Val::Int(u as u8 as i8 as i64),
            Prim::U8 => Val::Int(u as u8 as i64),
            Prim::I16 => Val::Int(u as u16 as i16 as i64),
            Prim::U16 => Val::Int(u as u16 as i64),
            Prim::I32 => Val::Int(u as u32 as i32 as i64),
            Prim::U32 => Val::Int(u as u32 as i64),
            Prim::I64 | Prim::U64 => Val::Int(u as i64),
            Prim::F32 => Val::Float(f32::from_bits(u as u32) as f64),
            Prim::F64 => Val::Float(f64::from_bits(u)),
        })
    }

    fn lookup_node(&self, name: &str) -> Option<&Node> {
        self.frames.iter().rev().find_map(|f| {
            f.iter()
                .rev()
                .find(|n| n.name == name || base_name(&n.name) == name)
        })
    }

    /// Evaluate an expression that refers to a field, giving the node.
    fn eval_path(&mut self, e: &Expr) -> Result<Option<Node>, String> {
        match e {
            Expr::Var(n) => Ok(self.lookup_node(n).cloned()),
            Expr::Member(b, m) => {
                let base = self.eval_path(b)?;
                Ok(base.and_then(|b| {
                    b.children
                        .iter()
                        .rev()
                        .find(|c| c.name == *m || base_name(&c.name) == m)
                        .cloned()
                }))
            }
            Expr::Index(b, i) => {
                let idx = self.eval(i)?.as_i();
                if idx < 0 {
                    return Ok(None);
                }
                let idx = idx as usize;
                // Repeated declarations (chunk[0], chunk[1], ...) live as siblings.
                if let Expr::Var(n) = &**b {
                    let key = format!("{n}[{idx}]");
                    for f in self.frames.iter().rev() {
                        if let Some(found) = f.iter().find(|x| x.name == key) {
                            return Ok(Some(found.clone()));
                        }
                    }
                }
                let Some(base) = self.eval_path(b)? else {
                    return Ok(None);
                };
                if let Some(p) = base.elem {
                    if idx >= base.count {
                        return Err(tr!(self.lang, "err.indexout", idx = idx, name = base.name));
                    }
                    let off = base.offset + idx * p.size();
                    let v = self.read_prim(p, off)?;
                    let mut n = Node::new(&format!("{}[{idx}]", base.name), base.type_name.clone(), off);
                    n.size = p.size();
                    n.display = v.text();
                    n.value = Some(v);
                    return Ok(Some(n));
                }
                Ok(base.children.get(idx).cloned())
            }
            _ => Ok(None),
        }
    }

    fn get_local(&self, n: &str) -> Option<Val> {
        self.locals.iter().rev().find_map(|l| l.get(n).cloned())
    }

    fn set_local(&mut self, n: &str, v: Val) -> Result<(), String> {
        for l in self.locals.iter_mut().rev() {
            if let Some(slot) = l.get_mut(n) {
                *slot = v;
                return Ok(());
            }
        }
        Err(tr!(self.lang, "err.unknownvar", n = n))
    }

    fn eval(&mut self, e: &Expr) -> Result<Val, String> {
        self.tick()?;
        Ok(match e {
            Expr::Num(n) => Val::Int(*n),
            Expr::Float(f) => Val::Float(*f),
            Expr::Str(s) => Val::Str(s.clone()),
            Expr::Var(n) => {
                if let Some(v) = self.get_local(n) {
                    v
                } else if let Some(node) = self.lookup_node(n) {
                    node.value.clone().unwrap_or(Val::Int(0))
                } else if let Some(c) = self.consts.get(n) {
                    Val::Int(*c)
                } else {
                    return Err(tr!(self.lang, "err.unknownname", n = n));
                }
            }
            Expr::Member(..) | Expr::Index(..) => match self.eval_path(e)? {
                Some(n) => n.value.unwrap_or(Val::Int(0)),
                None => return Err(tr!(self.lang, "err.fieldnotfound")),
            },
            Expr::Call(name, args) => self.call(name, args)?,
            Expr::Unary(op, a) => {
                let v = self.eval(a)?;
                match *op {
                    "!" => Val::Int(!v.truthy() as i64),
                    "-" => match v {
                        Val::Float(f) => Val::Float(-f),
                        o => Val::Int(o.as_i().wrapping_neg()),
                    },
                    "~" => Val::Int(!v.as_i()),
                    _ => v,
                }
            }
            Expr::Binary(op, a, b) => {
                if *op == "&&" {
                    if !self.eval(a)?.truthy() {
                        return Ok(Val::Int(0));
                    }
                    return Ok(Val::Int(self.eval(b)?.truthy() as i64));
                }
                if *op == "||" {
                    if self.eval(a)?.truthy() {
                        return Ok(Val::Int(1));
                    }
                    return Ok(Val::Int(self.eval(b)?.truthy() as i64));
                }
                let l = self.eval(a)?;
                let r = self.eval(b)?;
                binop(op, l, r, self.lang)?
            }
            Expr::Ternary(c, a, b) => {
                if self.eval(c)?.truthy() {
                    self.eval(a)?
                } else {
                    self.eval(b)?
                }
            }
            Expr::Assign(n, op, rhs) => {
                let r = self.eval(rhs)?;
                let v = if *op == "=" {
                    r
                } else {
                    let cur = self
                        .get_local(n)
                        .ok_or(tr!(self.lang, "err.unknownvar2", n = n))?;
                    binop(&op[..op.len() - 1], cur, r, self.lang)?
                };
                self.set_local(n, v.clone())?;
                v
            }
            Expr::PostInc(n, d) => {
                let cur = self
                    .get_local(n)
                    .ok_or(tr!(self.lang, "err.unknownvar2", n = n))?;
                self.set_local(n, Val::Int(cur.as_i() + d))?;
                cur
            }
        })
    }

    fn arg_i(&mut self, args: &[Expr], i: usize, def: i64) -> Result<i64, String> {
        match args.get(i) {
            Some(e) => Ok(self.eval(e)?.as_i()),
            None => Ok(def),
        }
    }

    fn read_at(&mut self, args: &[Expr], p: Prim) -> Result<Val, String> {
        let at = self.arg_i(args, 0, self.pos as i64)?;
        self.read_prim(p, at.max(0) as usize)
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> Result<Val, String> {
        Ok(match name {
            "LittleEndian" => {
                self.le = true;
                Val::Int(0)
            }
            "BigEndian" => {
                self.le = false;
                Val::Int(0)
            }
            "IsLittleEndian" => Val::Int(self.le as i64),
            "IsBigEndian" => Val::Int(!self.le as i64),
            "FTell" => Val::Int(self.pos as i64),
            "FileSize" => Val::Int(self.data.len() as i64),
            "FEof" => Val::Int((self.pos >= self.data.len()) as i64),
            "FSeek" => {
                let p = self.arg_i(args, 0, 0)?;
                if p < 0 || p as usize > self.data.len() {
                    return Ok(Val::Int(-1));
                }
                self.pos = p as usize;
                Val::Int(0)
            }
            "FSkip" => {
                let p = self.arg_i(args, 0, 0)?;
                self.pos = (self.pos as i64 + p).clamp(0, self.data.len() as i64) as usize;
                Val::Int(0)
            }
            "ReadByte" | "ReadChar" => self.read_at(args, Prim::I8)?,
            "ReadUByte" | "ReadUChar" => self.read_at(args, Prim::U8)?,
            "ReadShort" => self.read_at(args, Prim::I16)?,
            "ReadUShort" => self.read_at(args, Prim::U16)?,
            "ReadInt" => self.read_at(args, Prim::I32)?,
            "ReadUInt" => self.read_at(args, Prim::U32)?,
            "ReadInt64" | "ReadQuad" => self.read_at(args, Prim::I64)?,
            "ReadUInt64" | "ReadUQuad" => self.read_at(args, Prim::U64)?,
            "ReadFloat" => self.read_at(args, Prim::F32)?,
            "ReadDouble" => self.read_at(args, Prim::F64)?,
            "ReadString" => {
                let at = self.arg_i(args, 0, self.pos as i64)?.max(0) as usize;
                let max = self.arg_i(args, 1, -1)?;
                let mut s = String::new();
                let mut i = at;
                while i < self.data.len() && self.data[i] != 0 {
                    if max >= 0 && (i - at) as i64 >= max {
                        break;
                    }
                    s.push(self.data[i] as char);
                    i += 1;
                }
                Val::Str(s)
            }
            "Printf" | "Warning" => {
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.eval(a)?);
                }
                let s = format_printf(&vals);
                self.log.push(if name == "Warning" {
                    tr!(self.lang, "err.warn", s = s)
                } else {
                    s
                });
                Val::Int(0)
            }
            "Strlen" => {
                let v = match args.first() {
                    Some(a) => self.eval(a)?,
                    None => Val::Str(String::new()),
                };
                Val::Int(v.text().len() as i64)
            }
            "Exit" => return Err(EXIT.into()),
            "sizeof" => {
                let a = args.first().ok_or(tr!(self.lang, "err.fnarg.sizeof"))?;
                if let Expr::Var(t) = a {
                    let t = self.resolve_alias(t);
                    if let Some(p) = prim_of(&t) {
                        return Ok(Val::Int(p.size() as i64));
                    }
                }
                match self.eval_path(a)? {
                    Some(n) => Val::Int(n.size as i64),
                    None => return Err(tr!(self.lang, "err.unknown.sizeof")),
                }
            }
            "startof" => {
                let a = args.first().ok_or(tr!(self.lang, "err.fnarg.startof"))?;
                match self.eval_path(a)? {
                    Some(n) => Val::Int(n.offset as i64),
                    None => return Err(tr!(self.lang, "err.unknown.startof")),
                }
            }
            "exists" => {
                let a = args.first().ok_or(tr!(self.lang, "err.fnarg.exists"))?;
                Val::Int(self.eval_path(a).ok().flatten().is_some() as i64)
            }
            "Min" | "Max" => {
                let a = self.arg_i(args, 0, 0)?;
                let b = self.arg_i(args, 1, 0)?;
                Val::Int(if name == "Min" { a.min(b) } else { a.max(b) })
            }
            "RequiresVersion" | "DisplayFormatHex" | "DisplayFormatDecimal" | "SetBackColor"
            | "SetForeColor" => Val::Int(0),
            _ => return Err(tr!(self.lang, "err.unknownfunc", name = name)),
        })
    }

    fn push_node(&mut self, mut node: Node) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(tr!(self.lang, "err.toomanyfields"));
        }
        let frame = self.frames.last_mut().unwrap();
        let base = node.name.clone();
        let same = frame.iter().filter(|n| base_name(&n.name) == base).count();
        if same > 0 {
            if same == 1 {
                if let Some(first) = frame.iter_mut().find(|n| n.name == base) {
                    first.name = format!("{base}[0]");
                }
            }
            node.name = format!("{base}[{same}]");
        }
        frame.push(node);
        Ok(())
    }

    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Flow, String> {
        for s in stmts {
            match self.exec(s)? {
                Flow::Normal => {}
                f => return Ok(f),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec(&mut self, s: &Stmt) -> Result<Flow, String> {
        self.tick()?;
        match s {
            Stmt::Empty => {}
            Stmt::Break => return Ok(Flow::Break),
            Stmt::Continue => return Ok(Flow::Continue),
            Stmt::Return => return Ok(Flow::Return),
            Stmt::Block(b) => return self.exec_block(b),
            Stmt::Expr(e) => {
                self.eval(e)?;
            }
            Stmt::Local { name, init } => {
                let v = match init {
                    Some(e) => self.eval(e)?,
                    None => Val::Int(0),
                };
                self.locals.last_mut().unwrap().insert(name.clone(), v);
            }
            Stmt::StructDef { name, body } => {
                self.types.insert(name.clone(), TypeDef::Struct(body.clone()));
            }
            Stmt::EnumDef { name, base, items } => {
                let mut next = 0i64;
                let mut list = Vec::new();
                for (n, e) in items {
                    if let Some(e) = e {
                        next = self.eval(e)?.as_i();
                    }
                    self.consts.insert(n.clone(), next);
                    list.push((n.clone(), next));
                    next += 1;
                }
                self.types
                    .insert(name.clone(), TypeDef::Enum(base.clone(), list));
            }
            Stmt::Typedef { new, old } => {
                self.aliases.insert(new.clone(), old.clone());
            }
            Stmt::If(c, t, e) => {
                if self.eval(c)?.truthy() {
                    return self.exec(t);
                } else if let Some(e) = e {
                    return self.exec(e);
                }
            }
            Stmt::While(c, b) => {
                while self.eval(c)?.truthy() {
                    match self.exec(b)? {
                        Flow::Break => break,
                        Flow::Return => return Ok(Flow::Return),
                        _ => {}
                    }
                }
            }
            Stmt::DoWhile(b, c) => loop {
                match self.exec(b)? {
                    Flow::Break => break,
                    Flow::Return => return Ok(Flow::Return),
                    _ => {}
                }
                if !self.eval(c)?.truthy() {
                    break;
                }
            },
            Stmt::For(init, cond, step, body) => {
                if let Some(i) = init {
                    self.exec(i)?;
                }
                loop {
                    if let Some(c) = cond {
                        if !self.eval(c)?.truthy() {
                            break;
                        }
                    }
                    match self.exec(body)? {
                        Flow::Break => break,
                        Flow::Return => return Ok(Flow::Return),
                        _ => {}
                    }
                    if let Some(st) = step {
                        self.eval(st)?;
                    }
                }
            }
            Stmt::Decl {
                ty,
                name,
                array,
                attrs,
                line,
            } => {
                let count = match array {
                    Some(e) => Some(
                        self.eval(e)
                            .map_err(|m| tr!(self.lang, "err.line", line = line, m = m))?
                            .as_i(),
                    ),
                    None => None,
                };
                if matches!(count, Some(c) if c < 0) {
                    return Err(tr!(self.lang, "err.negsize", line = line, name = name));
                }
                let node = self
                    .declare(ty, name, count.map(|c| c as usize), attrs)
                    .map_err(|m| {
                        // Errors from a nested field already carry their line number.
                        let has_line = m.starts_with(&i18n::get(self.lang, "err.line.word"))
                            || m.starts_with(&i18n::get(self.lang, "err.abort.word"));
                        if has_line || m == EXIT {
                            m
                        } else {
                            tr!(self.lang, "err.line", line = line, m = m)
                        }
                    })?;
                self.push_node(node)?;
            }
        }
        Ok(Flow::Normal)
    }

    fn declare(&mut self, ty: &str, name: &str, count: Option<usize>, attrs: &Attrs) -> Result<Node, String> {
        let rt = self.resolve_alias(ty);
        let start = self.pos;
        let mut base = Node::new(name, ty.to_string(), start);
        base.comment = attrs.comment.clone();
        base.color = attrs.color;
        if rt == "string" {
            if start >= self.data.len() {
                return Err(tr!(self.lang, "err.eof"));
            }
            let mut s = String::new();
            let mut i = self.pos;
            while i < self.data.len() && self.data[i] != 0 {
                s.push(self.data[i] as char);
                i += 1;
            }
            let end = (i + 1).min(self.data.len());
            self.pos = end;
            base.type_name = "string".into();
            base.size = end - start;
            base.display = format!("\"{}\"", escape(&s));
            base.value = Some(Val::Str(s));
            return Ok(base);
        }
        if let Some(p) = prim_of(&rt) {
            return self.declare_prim(p, ty, name, count, attrs, base);
        }
        if let Some(TypeDef::Enum(ebase, items)) = self.types.get(&rt).cloned() {
            let p = prim_of(&self.resolve_alias(&ebase)).unwrap_or(Prim::I32);
            base.type_name = format!("enum {ty}");
            let mut n = self.declare_prim(p, ty, name, count, attrs, base)?;
            if count.is_none() {
                if let Some(v) = &n.value {
                    let v = v.as_i();
                    n.display = match items.iter().find(|(_, x)| *x == v) {
                        Some((l, _)) => format!("{l} (0x{v:X})"),
                        None => format!("0x{v:X}"),
                    };
                }
            }
            return Ok(n);
        }
        if let Some(TypeDef::Struct(body)) = self.types.get(&rt).cloned() {
            base.type_name = format!("struct {ty}");
            base.is_struct = true;
            match count {
                None => {
                    base.children = self.run_struct(&body)?;
                }
                Some(c) => {
                    base.type_name = format!("struct {ty}[{c}]");
                    for i in 0..c {
                        if self.pos >= self.data.len() {
                            return Err(tr!(self.lang, "err.eof_in", name = name, i = i));
                        }
                        let es = self.pos;
                        let children = self.run_struct(&body)?;
                        self.nodes += 1;
                        if self.nodes > MAX_NODES {
                            return Err(tr!(self.lang, "err.toomanyfields"));
                        }
                        let mut el = Node::new(&format!("{name}[{i}]"), format!("struct {ty}"), es);
                        el.size = self.pos - es;
                        el.children = children;
                        el.is_struct = true;
                        base.children.push(el);
                    }
                    base.count = c;
                }
            }
            base.size = self.pos - start;
            return Ok(base);
        }
        Err(tr!(self.lang, "err.unknowntype", ty = ty))
    }

    fn run_struct(&mut self, body: &Rc<Vec<Stmt>>) -> Result<Vec<Node>, String> {
        self.frames.push(Vec::new());
        self.locals.push(HashMap::new());
        let r = self.exec_block(body);
        self.locals.pop();
        let children = self.frames.pop().unwrap();
        r.map(|_| children)
    }

    fn declare_prim(&mut self, p: Prim, ty: &str, name: &str, count: Option<usize>, attrs: &Attrs, mut n: Node) -> Result<Node, String> {
        let sz = p.size();
        match count {
            None => {
                let v = self.read_prim(p, self.pos)?;
                self.pos += sz;
                n.size = sz;
                n.prim = Some(p);
                n.le = self.le;
                n.display = fmt_val(&v, p, attrs.hex);
                n.value = Some(v);
            }
            Some(c) => {
                let total = c.checked_mul(sz).ok_or(tr!(self.lang, "err.fieldbig"))?;
                if self.pos + total > self.data.len() {
                    return Err(tr!(
                        self.lang,
                        "err.eof_need",
                        name = name,
                        total = total,
                        pos = format!("{:X}", self.pos)
                    ));
                }
                n.type_name = format!("{ty}[{c}]");
                n.size = total;
                n.elem = Some(p);
                n.count = c;
                let bytes = &self.data[self.pos..self.pos + total];
                if matches!(p, Prim::Char) {
                    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
                    let s: String = bytes[..end].iter().map(|b| *b as char).collect();
                    n.display = format!("\"{}\"", escape(&s));
                    n.value = Some(Val::Str(s));
                } else if c <= 16 && sz == 1 {
                    n.display = bytes
                        .iter()
                        .map(|b| format!("{b:02X}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                } else {
                    n.display = tr!(self.lang, "err.elems", n = c);
                }
                if c <= CHILD_LIMIT && c > 0 && !matches!(p, Prim::Char) {
                    for i in 0..c {
                        let off = self.pos + i * sz;
                        let v = self.read_prim(p, off)?;
                        let mut ch = Node::new(&format!("{name}[{i}]"), ty.to_string(), off);
                        ch.size = sz;
                        ch.prim = Some(p);
                        ch.le = self.le;
                        ch.display = fmt_val(&v, p, attrs.hex);
                        ch.value = Some(v);
                        n.children.push(ch);
                    }
                    self.nodes += c;
                }
                self.pos += total;
            }
        }
        Ok(n)
    }
}

fn base_name(n: &str) -> &str {
    match n.find('[') {
        Some(i) => &n[..i],
        None => n,
    }
}

fn escape(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { '.' } else { c }).collect()
}

fn fmt_val(v: &Val, p: Prim, hex: bool) -> String {
    match (v, p) {
        (Val::Int(i), Prim::Char) => {
            let c = *i as u8;
            if (0x20..0x7f).contains(&c) {
                format!("{i} '{}'", c as char)
            } else {
                i.to_string()
            }
        }
        (Val::Int(i), _) if hex => {
            let bits = p.size() * 8;
            let mask = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
            format!("0x{:0w$X}", (*i as u64) & mask, w = p.size() * 2)
        }
        (Val::Int(i), _) => i.to_string(),
        (Val::Float(f), Prim::F32) => crate::tools::fmt_float(*f, 7),
        (Val::Float(f), _) => crate::tools::fmt_float(*f, 15),
        (Val::Str(s), _) => format!("\"{s}\""),
    }
}

fn binop(op: &str, l: Val, r: Val, lang: Lang) -> Result<Val, String> {
    if let (Val::Str(a), Val::Str(b)) = (&l, &r) {
        return Ok(match op {
            "==" => Val::Int((a == b) as i64),
            "!=" => Val::Int((a != b) as i64),
            "+" => Val::Str(format!("{a}{b}")),
            "<" => Val::Int((a < b) as i64),
            ">" => Val::Int((a > b) as i64),
            _ => return Err(tr!(lang, "err.optext", op = op)),
        });
    }
    if matches!(l, Val::Str(_)) || matches!(r, Val::Str(_)) {
        if op == "+" {
            return Ok(Val::Str(format!("{}{}", l.text(), r.text())));
        }
        if op == "==" || op == "!=" {
            let eq = l.text() == r.text();
            return Ok(Val::Int(((op == "==") == eq) as i64));
        }
    }
    if matches!(l, Val::Float(_)) || matches!(r, Val::Float(_)) {
        let (a, b) = (l.as_f(), r.as_f());
        return Ok(match op {
            "+" => Val::Float(a + b),
            "-" => Val::Float(a - b),
            "*" => Val::Float(a * b),
            "/" => Val::Float(a / b),
            "<" => Val::Int((a < b) as i64),
            ">" => Val::Int((a > b) as i64),
            "<=" => Val::Int((a <= b) as i64),
            ">=" => Val::Int((a >= b) as i64),
            "==" => Val::Int((a == b) as i64),
            "!=" => Val::Int((a != b) as i64),
            _ => binop(op, Val::Int(a as i64), Val::Int(b as i64), lang)?,
        });
    }
    let (a, b) = (l.as_i(), r.as_i());
    Ok(Val::Int(match op {
        "+" => a.wrapping_add(b),
        "-" => a.wrapping_sub(b),
        "*" => a.wrapping_mul(b),
        "/" | "%" if b == 0 => return Err(tr!(lang, "err.div0")),
        "/" => a.wrapping_div(b),
        "%" => a.wrapping_rem(b),
        "<<" => a.wrapping_shl(b as u32),
        ">>" => a.wrapping_shr(b as u32),
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        "<" => (a < b) as i64,
        ">" => (a > b) as i64,
        "<=" => (a <= b) as i64,
        ">=" => (a >= b) as i64,
        "==" => (a == b) as i64,
        "!=" => (a != b) as i64,
        _ => return Err(tr!(lang, "err.unknownop", op = op)),
    }))
}

fn format_printf(vals: &[Val]) -> String {
    let Some(Val::Str(fmt)) = vals.first() else {
        return vals.iter().map(|v| v.text()).collect::<Vec<_>>().join(" ");
    };
    let mut out = String::new();
    let mut args = vals[1..].iter();
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut spec = String::new();
        while let Some(&n) = chars.peek() {
            spec.push(n);
            chars.next();
            if (n.is_ascii_alphabetic() && n != 'l') || n == '%' {
                break;
            }
        }
        let conv = spec.chars().last().unwrap_or('%');
        let digits: String = spec.chars().filter(|c| c.is_ascii_digit()).collect();
        let zero = digits.starts_with('0');
        let width: usize = digits.parse().unwrap_or(0);
        let s = match conv {
            '%' => "%".to_string(),
            'd' | 'i' | 'u' => args.next().map(|v| v.as_i().to_string()).unwrap_or_default(),
            'x' => args.next().map(|v| format!("{:x}", v.as_i())).unwrap_or_default(),
            'X' => args.next().map(|v| format!("{:X}", v.as_i())).unwrap_or_default(),
            'c' => args
                .next()
                .map(|v| (v.as_i() as u8 as char).to_string())
                .unwrap_or_default(),
            'f' | 'g' | 'e' => args.next().map(|v| format!("{}", v.as_f())).unwrap_or_default(),
            _ => args.next().map(|v| v.text()).unwrap_or_default(),
        };
        if s.len() < width {
            let pad = if zero { '0' } else { ' ' };
            out.extend(std::iter::repeat_n(pad, width - s.len()));
        }
        out.push_str(&s);
    }
    out
}

/// Pastel colours for template fields.
pub const PALETTE: [Color32; 6] = [
    Color32::from_rgb(200, 225, 255),
    Color32::from_rgb(206, 240, 200),
    Color32::from_rgb(255, 224, 190),
    Color32::from_rgb(232, 212, 250),
    Color32::from_rgb(255, 244, 178),
    Color32::from_rgb(190, 238, 232),
];

fn collect_highlights(nodes: &[Node], out: &mut Vec<(usize, usize, Color32)>, idx: &mut usize, inherited: Option<Color32>) {
    for n in nodes {
        let col = n.color.or(inherited);
        if n.children.is_empty() || n.elem.is_some() {
            if n.size > 0 {
                let c = col.unwrap_or(PALETTE[*idx % PALETTE.len()]);
                out.push((n.offset, n.offset + n.size, c));
                *idx += 1;
            }
        } else {
            collect_highlights(&n.children, out, idx, col);
        }
    }
}

/// Run a template over `data`.
pub fn run(name: &str, src: &str, data: &[u8], lang: Lang) -> TemplateResult {
    let mut res = TemplateResult {
        name: name.to_string(),
        source: src.to_string(),
        revision: 0,
        nodes: Vec::new(),
        log: Vec::new(),
        error: None,
        expanded: HashSet::new(),
    };
    let prog = match parse(src, lang) {
        Ok(p) => p,
        Err(e) => {
            res.error = Some(tr!(lang, "err.syntax", e = e));
            return res;
        }
    };
    let mut it = Interp {
        data,
        pos: 0,
        le: true,
        lang,
        types: HashMap::new(),
        aliases: HashMap::new(),
        consts: HashMap::new(),
        frames: vec![Vec::new()],
        locals: vec![HashMap::new()],
        log: Vec::new(),
        steps: 0,
        nodes: 0,
    };
    if let Err(e) = it.exec_block(&prog) {
        if e != EXIT {
            res.error = Some(e);
        }
    }
    it.frames.truncate(1);
    res.nodes = std::mem::take(&mut it.frames[0]);
    res.log = it.log;
    for n in &res.nodes {
        if n.is_struct && n.children.len() <= 64 {
            res.expanded.insert(format!("/{}", n.name));
        }
    }
    res
}

impl TemplateResult {
    pub fn highlights(&self) -> Vec<(usize, usize, Color32)> {
        let mut out = Vec::new();
        let mut idx = 0;
        collect_highlights(&self.nodes, &mut out, &mut idx, None);
        out.sort_by_key(|h| h.0);
        let mut clean: Vec<(usize, usize, Color32)> = Vec::with_capacity(out.len());
        for h in out {
            if let Some(last) = clean.last() {
                if h.0 < last.1 {
                    if h.1 > last.1 {
                        let s = last.1;
                        clean.push((s, h.1, h.2));
                    }
                    continue;
                }
            }
            clean.push(h);
        }
        clean
    }

    /// Ids (paths) of the nested fields that cover `pos`, outermost first.
    pub fn path_at(&self, pos: usize) -> Vec<String> {
        fn walk(nodes: &[Node], pos: usize, prefix: &str, path: &mut Vec<String>) {
            for n in nodes {
                if n.size > 0 && pos >= n.offset && pos < n.offset + n.size {
                    let id = format!("{prefix}/{}", n.name);
                    path.push(id.clone());
                    walk(&n.children, pos, &id, path);
                    return;
                }
            }
        }
        let mut p = Vec::new();
        walk(&self.nodes, pos, "", &mut p);
        p
    }
}

// ---------------------------------------------------------------- built-in templates

pub struct BuiltIn {
    pub name: &'static str,
    pub exts: &'static [&'static str],
    pub magic: &'static [u8],
    pub source: &'static str,
}

pub const BUILTINS: &[BuiltIn] = &[
    BuiltIn {
        name: "tpl.bmp",
        exts: &["bmp", "dib"],
        magic: b"BM",
        source: include_str!("../templates/bmp.tpl"),
    },
    BuiltIn {
        name: "tpl.png",
        exts: &["png"],
        magic: b"\x89PNG",
        source: include_str!("../templates/png.tpl"),
    },
    BuiltIn {
        name: "tpl.gif",
        exts: &["gif"],
        magic: b"GIF8",
        source: include_str!("../templates/gif.tpl"),
    },
    BuiltIn {
        name: "tpl.wav",
        exts: &["wav"],
        magic: b"RIFF",
        source: include_str!("../templates/wav.tpl"),
    },
    BuiltIn {
        name: "tpl.zip",
        exts: &["zip", "jar", "docx", "xlsx", "pptx", "apk"],
        magic: b"PK\x03\x04",
        source: include_str!("../templates/zip.tpl"),
    },
    BuiltIn {
        name: "tpl.exe",
        exts: &["exe", "dll", "sys"],
        magic: b"MZ",
        source: include_str!("../templates/exe.tpl"),
    },
];

/// Pick a built-in template for a file by its extension and first bytes.
pub fn detect(name: &str, data: &[u8]) -> Option<&'static BuiltIn> {
    let ext = std::path::Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    BUILTINS
        .iter()
        .find(|b| b.exts.contains(&ext.as_str()) && data.starts_with(b.magic))
        .or_else(|| BUILTINS.iter().find(|b| data.starts_with(b.magic)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmp_sample_parses() {
        let data = crate::sample::sample_bmp();
        let r = run("bmp", BUILTINS[0].source, &data, Lang::En);
        assert!(r.error.is_none(), "{:?}", r.error);
        assert!(r.nodes.len() >= 3);
    }

    #[test]
    fn png_like() {
        let mut d = b"\x89PNG\r\n\x1a\n".to_vec();
        d.extend_from_slice(&[0, 0, 0, 13]);
        d.extend_from_slice(b"IHDR");
        d.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]);
        d.extend_from_slice(&[1, 2, 3, 4]);
        d.extend_from_slice(&[0, 0, 0, 0]);
        d.extend_from_slice(b"IEND");
        d.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);
        let r = run("png", BUILTINS[1].source, &d, Lang::En);
        assert!(r.error.is_none(), "{:?}", r.error);
        assert_eq!(r.nodes.len(), 3);
        assert_eq!(r.nodes[1].name, "chunk[0]");
    }

    #[test]
    fn locals_and_loops() {
        let src = "local int i; local int s = 0; for (i = 0; i < 4; i++) { s += i; } uchar a[s];";
        let r = run("t", src, &[0u8; 10], Lang::En);
        assert!(r.error.is_none(), "{:?}", r.error);
        assert_eq!(r.nodes[0].size, 6);
    }
}
