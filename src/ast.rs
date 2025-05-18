//! AST 定義 + Display 実装

use std::fmt::{self, Display};

/// プレースホルダー ID （`PH0`, `PH1`, ...）
pub type Placeholder = String;

/* ---------- Expressions ---------- */
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(i64),
    Str(String),
    Var(String),
    New(String),   // new <Class>()
    This,
    Lhs(Box<Lhs>),
    Hole(Placeholder),
    Literal(String), // 新しいバリアント
    MethodCall(Box<Lhs>, String, Vec<Expr>), // obj.method(args)
}

/* ---------- L-value ---------- */
#[derive(Debug, Clone, PartialEq)]
pub enum Lhs {
    Var(String),
    ObjAccess(Box<Lhs>, String), // obj.prop ...
    Hole(Placeholder),
    This, // New variant for 'this' keyword
}

/* ---------- Statements ---------- */
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl { name: String, expr: Expr },
    Assign  { lhs: Lhs,     expr: Expr },
    Expr(Expr), 
    Hole(Placeholder), // Added
}

/* ---------- Program ---------- */
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

/* ---------- Display impls (pretty-printer) ---------- */
impl Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Expr::*;
        match self {
            Num(n)        => write!(f, "{n}"),
            Str(s)        => write!(f, "{s:?}"), // Strings are quoted
            Var(v)        => write!(f, "{v}"),
            New(cls)      => write!(f, "new {cls}()"),
            This          => write!(f, "this"),
            Lhs(lhs)      => write!(f, "{lhs}"),
            Hole(ph)      => write!(f, "{ph}"),
            Literal(lit)  => write!(f, "{lit}"), // Literals (e.g. from JSON) might not need quotes here if they are already strings
            MethodCall(obj, method, args) => {
                let args_str = args.iter().map(|a| format!("{}", a)).collect::<Vec<String>>().join(", ");
                write!(f, "{}.{}({})", obj, method, args_str)
            },
        }
    }
}

impl Display for Lhs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Lhs::*;
        match self {
            Var(v)             => write!(f, "{v}"),
            ObjAccess(obj, p)  => write!(f, "{obj}.{p}"),
            Hole(ph)           => write!(f, "{ph}"),
            This               => write!(f, "this"),
        }
    }
}

impl Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Stmt::*;
        match self {
            VarDecl { name, expr } => write!(f, "var {} = {};", name, expr),
            Assign { lhs, expr }   => write!(f, "{} = {};", lhs, expr),
            Expr(expr)             => write!(f, "{};", expr), // Assuming expressions as statements end with a semicolon
            Hole(ph)               => write!(f, "{};", ph), // Placeholder statement
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stmt in &self.stmts {
            writeln!(f, "{}", stmt)?;
        }
        Ok(())
    }
}
