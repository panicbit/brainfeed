use crate::ir::*;
use crate::{Context, Ptr};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
pub type Error = Box<std::error::Error>;

pub fn trans(ir: &IR) -> Result<String> {
    let mut code = String::new();
    let mut context = Context::new(&mut code);
    Trans::new(&mut context).run(ir)?;

    Ok(code)
}

struct Trans<'ctx> {
    context: &'ctx mut Context<'ctx>,
}

impl<'ctx> Trans<'ctx> {
    fn new(context: &'ctx mut Context<'ctx>) -> Self {
        Self { context }
    }

    fn run(mut self, ir: &IR) -> Result {
        Scope::root(&mut self, |trans, scope| {
            for stmt in &ir.stmts {
                trans.trans_stmt(scope, stmt)?;
            }
            Ok(())
        })
    }

    fn trans_stmt(&mut self, scope: &mut Scope, stmt: &Statement) -> Result {
        Ok(match stmt {
            Statement::Decl(Decl { name, value }) => {
                let ptr = &scope.alloc_var(self.context, name.clone());

                if let Some(value) = value {
                    self.trans_expr(scope, value, ptr)?;
                }
            }
            Statement::Assign(Assign { name, value }) => {
                let ptr = &scope.resolve_var(name)?;

                self.trans_expr(scope, value, ptr)?;
            }
            Statement::While(While { cond, body }) => self.trans_stmt_while(scope, cond, body)?,
            Statement::If(if_) => self.trans_stmt_if(scope, if_)?,
        })
    }

    fn trans_stmt_while(&mut self, scope: &mut Scope, cond: &Expr, body: &[Statement]) -> Result {
        let tmp = &self.context.stack_alloc();
        
        self.trans_expr(scope, cond, tmp)?;
        self.context.seek(tmp);
        self.context.emit("[");
        self.context.forget_known_values();

        scope.new(self, |this, scope| {
            for stmt in body {
                this.trans_stmt(scope, stmt)?;
            }
            Ok(())
        })?;

        self.trans_expr(scope, cond, tmp)?;
        self.context.seek(tmp);
        self.context.emit("]");

        Ok(())
    }

    fn trans_stmt_if(&mut self, scope: &mut Scope, If { cond, body }: &If) -> Result {        
        let tmp = &self.context.stack_alloc();
        
        self.trans_expr(scope, cond, tmp)?;
        self.context.seek(tmp);
        self.context.emit("[");
        self.context.forget_known_values();

        scope.new(self, |this, scope| {
            for stmt in body {
                this.trans_stmt(scope, stmt)?;
            }
            Ok(())
        })?;

        self.context.decrement(tmp);
        self.context.seek(tmp);
        self.context.emit("]");

        Ok(())
    }

    fn trans_expr(&mut self, scope: &mut Scope, expr: &Expr, target: &Ptr) -> Result {
        use Expr::*;
        Ok(match expr {
            Const(value) => {
                self.context.set(target, *value);
            }
            Var(name) => {
                let ptr = &scope.resolve_var(name)?;

                self.context.copy(ptr, target);
            }
            Add(a, b) => {
                let a_tmp = &self.context.stack_alloc();
                let b_tmp = &self.context.stack_alloc();

                self.trans_expr(scope, a, a_tmp)?;
                self.trans_expr(scope, b, b_tmp)?;

                self.context.add(a_tmp, b_tmp);
                self.context.mov(a_tmp, target);
            }
            Sub(a, b) => {
                let a_tmp = &self.context.stack_alloc();
                let b_tmp = &self.context.stack_alloc();

                self.trans_expr(scope, a, a_tmp)?;
                self.trans_expr(scope, b, b_tmp)?;

                self.context.sub(a_tmp, b_tmp);
                self.context.mov(a_tmp, target);
            }
            Gt(a, b) => {
                let a_tmp = &self.context.stack_alloc();
                let b_tmp = &self.context.stack_alloc();

                self.trans_expr(scope, a, a_tmp)?;
                self.trans_expr(scope, b, b_tmp)?;

                self.context.greater_than(a_tmp, b_tmp, target);
            }
        })
    }
}

struct Scope<'a> {
    variables: Vec<(Ident, Ptr)>,
    outer: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    fn root<F, R>(trans: &mut Trans, f: F) -> Result<R>
    where
        F: FnOnce(&mut Trans, &mut Self) -> Result<R>,
    {
        let mut root = Scope {
            variables: Vec::new(),
            outer: None,
        };

        f(trans, &mut root)
    }

    fn new<F, R>(&'a self, trans: &mut Trans, f: F) -> Result<R>
    where
        F: FnOnce(&mut Trans, &mut Self) -> Result<R>,
    {
        let mut inner = Scope {
            variables: Vec::new(),
            outer: Some(self),
        };

        f(trans, &mut inner)
    }

    fn resolve_var(&self, ident: &Ident) -> Result<Ptr> {
        self.variables.iter()
            .rev()
            .find(|(other_ident, _)| other_ident == ident)
            .map(|(_, ptr)| ptr.clone())
            .or_else(||
                self.outer.and_then(|outer|
                    outer.resolve_var(ident).ok()
                )
            )
            .ok_or(format!("Variable '{}' does not exist in the current scope", &**ident).into())
    }

    fn alloc_var(&mut self, context: &mut Context, ident: Ident) -> Ptr {
        let ptr = context.stack_alloc();

        self.variables.push((ident, ptr.clone()));

        ptr
    }
}
