//! Lexer and parser of the template language.

use std::rc::Rc;

use egui::Color32;

use crate::i18n::{self, Lang};

use crate::tr;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    Num(i64),
    Float(f64),
    Str(String),
    P(&'static str),
    Eof,
}

const PUNCTS: &[&str] = &[
    "<<=", ">>=", "==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "++", "--", "+=", "-=", "*=",
    "/=", "|=", "&=", "^=", "%=", "->", "{", "}", "(", ")", "[", "]", ";", ",", ".", "<", ">",
    "=", "+", "-", "*", "/", "%", "!", "~", "&", "|", "^", "?", ":",
];

pub fn lex(src: &str, lang: Lang) -> Result<Vec<(Tok, usize)>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
            continue;
        }
        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let s = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push((Tok::Ident(chars[s..i].iter().collect()), line));
            continue;
        }
        if c.is_ascii_digit() {
            let s = i;
            if c == '0' && i + 1 < chars.len() && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                i += 2;
                let hs = i;
                while i < chars.len() && chars[i].is_ascii_hexdigit() {
                    i += 1;
                }
                let h: String = chars[hs..i].iter().collect();
                let v = u64::from_str_radix(&h, 16)
                    .map_err(|_| tr!(lang, "err.badnum", line = line))?;
                while i < chars.len() && matches!(chars[i], 'u' | 'U' | 'l' | 'L') {
                    i += 1;
                }
                out.push((Tok::Num(v as i64), line));
                continue;
            }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let t: String = chars[s..i].iter().collect();
            while i < chars.len() && matches!(chars[i], 'u' | 'U' | 'l' | 'L' | 'f' | 'F') {
                i += 1;
            }
            if t.contains('.') {
                let f = t.parse().map_err(|_| tr!(lang, "err.badnum", line = line))?;
                out.push((Tok::Float(f), line));
            } else {
                let n = t
                    .parse::<u64>()
                    .map_err(|_| tr!(lang, "err.badnum", line = line))?;
                out.push((Tok::Num(n as i64), line));
            }
            continue;
        }
        if c == '"' || c == '\'' {
            let q = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != q {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    s.push(match chars[i] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '0' => '\0',
                        o => o,
                    });
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            i += 1;
            if q == '\'' {
                out.push((Tok::Num(s.chars().next().map(|c| c as i64).unwrap_or(0)), line));
            } else {
                out.push((Tok::Str(s), line));
            }
            continue;
        }
        let rest: String = chars[i..(i + 3).min(chars.len())].iter().collect();
        match PUNCTS.iter().find(|p| rest.starts_with(**p)) {
            Some(p) => {
                out.push((Tok::P(p), line));
                i += p.len();
            }
            None => return Err(tr!(lang, "err.badchar", line = line, c = c)),
        }
    }
    out.push((Tok::Eof, line));
    Ok(out)
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num(i64),
    Float(f64),
    Str(String),
    Var(String),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Assign(String, &'static str, Box<Expr>),
    PostInc(String, i64),
}

#[derive(Clone, Debug, Default)]
pub struct Attrs {
    pub hex: bool,
    pub comment: Option<String>,
    pub color: Option<Color32>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Decl {
        ty: String,
        name: String,
        array: Option<Expr>,
        attrs: Attrs,
        line: usize,
    },
    Local {
        name: String,
        init: Option<Expr>,
    },
    StructDef {
        name: String,
        body: Rc<Vec<Stmt>>,
    },
    EnumDef {
        name: String,
        base: String,
        items: Vec<(String, Option<Expr>)>,
    },
    Typedef {
        new: String,
        old: String,
    },
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    DoWhile(Box<Stmt>, Expr),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Expr>, Box<Stmt>),
    Block(Vec<Stmt>),
    Expr(Expr),
    Break,
    Continue,
    Return,
    Empty,
}

pub struct Parser {
    toks: Vec<(Tok, usize)>,
    i: usize,
    anon: usize,
    lang: Lang,
}

const TYPE_MODIFIERS: &[&str] = &["unsigned", "signed", "const"];

pub fn parse(src: &str, lang: Lang) -> Result<Vec<Stmt>, String> {
    let toks = lex(src, lang)?;
    Parser { toks, i: 0, anon: 0, lang }.program()
}

impl Parser {
    fn lang(&self) -> Lang {
        self.lang
    }
    fn peek(&self) -> &Tok {
        &self.toks[self.i].0
    }
    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.i + n).min(self.toks.len() - 1)].0
    }
    fn line(&self) -> usize {
        self.toks[self.i].1
    }
    fn next(&mut self) -> Tok {
        let t = self.toks[self.i].0.clone();
        if self.i < self.toks.len() - 1 {
            self.i += 1;
        }
        t
    }
    fn is_p(&self, p: &str) -> bool {
        matches!(self.peek(), Tok::P(x) if *x == p)
    }
    fn is_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Tok::Ident(x) if x == s)
    }
    fn eat_p(&mut self, p: &str) -> bool {
        if self.is_p(p) {
            self.next();
            true
        } else {
            false
        }
    }
    fn expect_p(&mut self, p: &str) -> Result<(), String> {
        if self.eat_p(p) {
            Ok(())
        } else {
            Err(tr!(
                self.lang(),
                "err.expected",
                line = self.line(),
                p = p,
                t = tok_text(self.peek(), self.lang())
            ))
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        let line = self.line();
        match self.next() {
            Tok::Ident(s) => Ok(s),
            t => Err(tr!(
                self.lang(),
                "err.expected_name",
                line = line,
                t = tok_text(&t, self.lang())
            )),
        }
    }

    fn program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut v = Vec::new();
        while *self.peek() != Tok::Eof {
            v.push(self.stmt()?);
        }
        Ok(v)
    }

    fn block_body(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect_p("{")?;
        let mut v = Vec::new();
        while !self.is_p("}") {
            if *self.peek() == Tok::Eof {
                return Err(tr!(self.lang(), "err.unexpected_end"));
            }
            v.push(self.stmt()?);
        }
        self.expect_p("}")?;
        Ok(v)
    }

    fn attrs(&mut self) -> Result<Attrs, String> {
        let mut a = Attrs::default();
        if !self.eat_p("<") {
            return Ok(a);
        }
        loop {
            if self.eat_p(">") {
                break;
            }
            let key = self.ident()?;
            if self.eat_p("=") {
                let val = match self.next() {
                    Tok::Ident(s) => s,
                    Tok::Str(s) => s,
                    Tok::Num(n) => n.to_string(),
                    t => tok_text(&t, self.lang()),
                };
                match key.as_str() {
                    "format" => a.hex = val == "hex",
                    "comment" => a.comment = Some(val),
                    "bgcolor" => a.color = named_color(&val),
                    _ => {}
                }
            }
            self.eat_p(",");
            if *self.peek() == Tok::Eof {
                return Err(tr!(self.lang(), "err.gt_missing"));
            }
        }
        Ok(a)
    }

    fn type_name(&mut self) -> Result<String, String> {
        let mut unsigned = false;
        while let Tok::Ident(s) = self.peek() {
            if TYPE_MODIFIERS.contains(&s.as_str()) {
                unsigned |= s == "unsigned";
                self.next();
            } else {
                break;
            }
        }
        if self.is_ident("struct") || self.is_ident("enum") || self.is_ident("union") {
            self.next();
        }
        let base = self.ident()?;
        if unsigned {
            return Ok(match base.as_str() {
                "char" => "uchar".into(),
                "short" => "ushort".into(),
                "int" | "long" => "uint".into(),
                "int64" => "uint64".into(),
                o => format!("u{o}"),
            });
        }
        Ok(base)
    }

    fn decl_after_type(&mut self, ty: String) -> Result<Stmt, String> {
        let mut decls = Vec::new();
        loop {
            let line = self.line();
            let name = self.ident()?;
            let array = if self.eat_p("[") {
                if self.eat_p("]") {
                    None
                } else {
                    let e = self.expr()?;
                    self.expect_p("]")?;
                    Some(e)
                }
            } else {
                None
            };
            let attrs = self.attrs()?;
            decls.push(Stmt::Decl {
                ty: ty.clone(),
                name,
                array,
                attrs,
                line,
            });
            if !self.eat_p(",") {
                break;
            }
        }
        self.expect_p(";")?;
        Ok(if decls.len() == 1 {
            decls.pop().unwrap()
        } else {
            Stmt::Block(decls)
        })
    }

    fn enum_body(&mut self) -> Result<Vec<(String, Option<Expr>)>, String> {
        self.expect_p("{")?;
        let mut items = Vec::new();
        while !self.is_p("}") {
            let n = self.ident()?;
            let v = if self.eat_p("=") { Some(self.expr()?) } else { None };
            items.push((n, v));
            if !self.eat_p(",") {
                break;
            }
        }
        self.expect_p("}")?;
        Ok(items)
    }

    fn enum_base(&mut self) -> Result<String, String> {
        if self.eat_p("<") {
            let t = self.type_name()?;
            self.expect_p(">")?;
            Ok(t)
        } else {
            Ok("int".into())
        }
    }

    fn paren_expr(&mut self) -> Result<Expr, String> {
        self.expect_p("(")?;
        let c = self.expr()?;
        self.expect_p(")")?;
        Ok(c)
    }

    fn stmt(&mut self) -> Result<Stmt, String> {
        if self.eat_p(";") {
            return Ok(Stmt::Empty);
        }
        if self.is_p("{") {
            return Ok(Stmt::Block(self.block_body()?));
        }
        let word = match self.peek() {
            Tok::Ident(s) => s.clone(),
            _ => {
                let e = self.expr()?;
                self.expect_p(";")?;
                return Ok(Stmt::Expr(e));
            }
        };
        match word.as_str() {
            "if" => {
                self.next();
                let c = self.paren_expr()?;
                let t = self.stmt()?;
                let e = if self.is_ident("else") {
                    self.next();
                    Some(Box::new(self.stmt()?))
                } else {
                    None
                };
                Ok(Stmt::If(c, Box::new(t), e))
            }
            "while" => {
                self.next();
                let c = self.paren_expr()?;
                Ok(Stmt::While(c, Box::new(self.stmt()?)))
            }
            "do" => {
                self.next();
                let b = self.stmt()?;
                if !self.is_ident("while") {
                    return Err(tr!(self.lang(), "err.dowhile", line = self.line()));
                }
                self.next();
                let c = self.paren_expr()?;
                self.expect_p(";")?;
                Ok(Stmt::DoWhile(Box::new(b), c))
            }
            "for" => {
                self.next();
                self.expect_p("(")?;
                let init = if self.eat_p(";") {
                    None
                } else {
                    Some(Box::new(self.stmt()?))
                };
                let cond = if self.is_p(";") { None } else { Some(self.expr()?) };
                self.expect_p(";")?;
                let step = if self.is_p(")") { None } else { Some(self.expr()?) };
                self.expect_p(")")?;
                Ok(Stmt::For(init, cond, step, Box::new(self.stmt()?)))
            }
            "break" | "continue" => {
                self.next();
                self.expect_p(";")?;
                Ok(if word == "break" { Stmt::Break } else { Stmt::Continue })
            }
            "return" => {
                self.next();
                while !self.is_p(";") && *self.peek() != Tok::Eof {
                    self.next();
                }
                self.expect_p(";")?;
                Ok(Stmt::Return)
            }
            "local" => {
                self.next();
                let _ty = self.type_name()?;
                let mut v = Vec::new();
                loop {
                    let name = self.ident()?;
                    if self.eat_p("[") {
                        while !self.eat_p("]") {
                            if *self.peek() == Tok::Eof {
                                return Err(tr!(self.lang(), "err.bracket_missing"));
                            }
                            self.next();
                        }
                    }
                    let init = if self.eat_p("=") { Some(self.expr()?) } else { None };
                    v.push(Stmt::Local { name, init });
                    if !self.eat_p(",") {
                        break;
                    }
                }
                self.expect_p(";")?;
                Ok(if v.len() == 1 { v.pop().unwrap() } else { Stmt::Block(v) })
            }
            "typedef" => {
                self.next();
                if self.is_ident("struct") || self.is_ident("union") {
                    self.next();
                    if let Tok::Ident(_) = self.peek() {
                        self.next(); // tag name
                    }
                    let body = self.block_body()?;
                    let name = self.ident()?;
                    let _ = self.attrs()?;
                    self.expect_p(";")?;
                    return Ok(Stmt::StructDef {
                        name,
                        body: Rc::new(body),
                    });
                }
                if self.is_ident("enum") {
                    self.next();
                    let base = self.enum_base()?;
                    if let Tok::Ident(_) = self.peek() {
                        self.next();
                    }
                    let items = self.enum_body()?;
                    let name = self.ident()?;
                    let _ = self.attrs()?;
                    self.expect_p(";")?;
                    return Ok(Stmt::EnumDef { name, base, items });
                }
                let old = self.type_name()?;
                let new = self.ident()?;
                let _ = self.attrs()?;
                self.expect_p(";")?;
                Ok(Stmt::Typedef { new, old })
            }
            "struct" | "union"
                if matches!(self.peek_at(1), Tok::P("{")) || matches!(self.peek_at(2), Tok::P("{")) =>
            {
                self.next();
                let name = if let Tok::Ident(n) = self.peek().clone() {
                    self.next();
                    n
                } else {
                    self.anon += 1;
                    format!("struct_{}", self.anon)
                };
                let body = self.block_body()?;
                let def = Stmt::StructDef {
                    name: name.clone(),
                    body: Rc::new(body),
                };
                if self.eat_p(";") {
                    return Ok(def);
                }
                let d = self.decl_after_type(name)?;
                Ok(Stmt::Block(vec![def, d]))
            }
            "enum"
                if matches!(self.peek_at(1), Tok::P("<"))
                    || matches!(self.peek_at(1), Tok::P("{"))
                    || matches!(self.peek_at(2), Tok::P("{")) =>
            {
                self.next();
                let base = self.enum_base()?;
                let name = if let Tok::Ident(n) = self.peek().clone() {
                    self.next();
                    n
                } else {
                    self.anon += 1;
                    format!("enum_{}", self.anon)
                };
                let items = self.enum_body()?;
                let def = Stmt::EnumDef {
                    name: name.clone(),
                    base,
                    items,
                };
                if self.eat_p(";") {
                    return Ok(def);
                }
                let d = self.decl_after_type(name)?;
                Ok(Stmt::Block(vec![def, d]))
            }
            _ => {
                let is_decl = TYPE_MODIFIERS.contains(&word.as_str())
                    || word == "struct"
                    || word == "enum"
                    || matches!(self.peek_at(1), Tok::Ident(_));
                if is_decl {
                    let ty = self.type_name()?;
                    return self.decl_after_type(ty);
                }
                let e = self.expr()?;
                self.expect_p(";")?;
                Ok(Stmt::Expr(e))
            }
        }
    }

    pub fn expr(&mut self) -> Result<Expr, String> {
        if let (Tok::Ident(name), Tok::P(op)) = (self.peek().clone(), self.peek_at(1).clone()) {
            if matches!(
                op,
                "=" | "+=" | "-=" | "*=" | "/=" | "|=" | "&=" | "^=" | "%=" | "<<=" | ">>="
            ) {
                self.next();
                self.next();
                let rhs = self.expr()?;
                return Ok(Expr::Assign(name, op, Box::new(rhs)));
            }
        }
        self.ternary()
    }

    fn ternary(&mut self) -> Result<Expr, String> {
        let c = self.binary(0)?;
        if self.eat_p("?") {
            let a = self.expr()?;
            self.expect_p(":")?;
            let b = self.ternary()?;
            return Ok(Expr::Ternary(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn binary(&mut self, level: usize) -> Result<Expr, String> {
        const LEVELS: &[&[&str]] = &[
            &["||"],
            &["&&"],
            &["|"],
            &["^"],
            &["&"],
            &["==", "!="],
            &["<", ">", "<=", ">="],
            &["<<", ">>"],
            &["+", "-"],
            &["*", "/", "%"],
        ];
        if level >= LEVELS.len() {
            return self.unary();
        }
        let mut lhs = self.binary(level + 1)?;
        loop {
            let op = match self.peek() {
                Tok::P(p) if LEVELS[level].contains(p) => *p,
                _ => break,
            };
            self.next();
            let rhs = self.binary(level + 1)?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        for op in ["!", "-", "~", "+"] {
            if self.eat_p(op) {
                let e = self.unary()?;
                return Ok(Expr::Unary(op, Box::new(e)));
            }
        }
        // Casts like (uint)x are skipped.
        if self.is_p("(") {
            if let (Tok::Ident(t), Tok::P(")")) = (self.peek_at(1).clone(), self.peek_at(2).clone()) {
                if crate::template::is_primitive(&t) {
                    self.next();
                    self.next();
                    self.next();
                    return self.unary();
                }
            }
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.primary()?;
        loop {
            if self.eat_p(".") || self.eat_p("->") {
                let m = self.ident()?;
                e = Expr::Member(Box::new(e), m);
            } else if self.eat_p("[") {
                let i = self.expr()?;
                self.expect_p("]")?;
                e = Expr::Index(Box::new(e), Box::new(i));
            } else if self.is_p("++") || self.is_p("--") {
                let d = if self.is_p("++") { 1 } else { -1 };
                self.next();
                if let Expr::Var(n) = e {
                    e = Expr::PostInc(n, d);
                } else {
                    return Err(tr!(self.lang(), "err.incr", line = self.line()));
                }
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let line = self.line();
        match self.next() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Float(f) => Ok(Expr::Float(f)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::P("(") => {
                let e = self.expr()?;
                self.expect_p(")")?;
                Ok(e)
            }
            Tok::P(op @ ("++" | "--")) => {
                let n = self.ident()?;
                let o = if op == "++" { "+=" } else { "-=" };
                Ok(Expr::Assign(n, o, Box::new(Expr::Num(1))))
            }
            Tok::Ident(name) => {
                if self.eat_p("(") {
                    let mut args = Vec::new();
                    while !self.is_p(")") {
                        args.push(self.expr()?);
                        if !self.eat_p(",") {
                            break;
                        }
                    }
                    self.expect_p(")")?;
                    return Ok(Expr::Call(name, args));
                }
                match name.as_str() {
                    "true" => Ok(Expr::Num(1)),
                    "false" => Ok(Expr::Num(0)),
                    _ => Ok(Expr::Var(name)),
                }
            }
            t => Err(tr!(
                self.lang(),
                "err.unexpected",
                line = line,
                t = tok_text(&t, self.lang())
            )),
        }
    }
}

fn tok_text(t: &Tok, lang: Lang) -> String {
    match t {
        Tok::Ident(s) => format!("'{s}'"),
        Tok::Num(n) => n.to_string(),
        Tok::Float(f) => f.to_string(),
        Tok::Str(s) => format!("\"{s}\""),
        Tok::P(p) => format!("'{p}'"),
        Tok::Eof => i18n::get(lang, "err.eof.tok").into(),
    }
}

fn named_color(s: &str) -> Option<Color32> {
    Some(match s {
        "cRed" => Color32::from_rgb(255, 180, 180),
        "cGreen" => Color32::from_rgb(180, 235, 180),
        "cBlue" => Color32::from_rgb(180, 205, 255),
        "cYellow" => Color32::from_rgb(255, 240, 160),
        "cAqua" => Color32::from_rgb(170, 235, 235),
        "cPurple" => Color32::from_rgb(225, 190, 245),
        "cGray" | "cLtGray" => Color32::from_rgb(218, 218, 218),
        _ => return None,
    })
}
