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
    scopes: Vec<Scope>,
}

impl<'ctx> Trans<'ctx> {
    fn new(context: &'ctx mut Context<'ctx>) -> Self {
        Self {
            context,
            scopes: Vec::new(),
        }
    }

    fn run(mut self, ir: &IR) -> Result {
        self.push_scope();

        for stmt in &ir.stmts {
            self.trans_stmt(stmt)?;
        }

        self.pop_scope();
        Ok(())
    }

    fn trans_stmt(&mut self, stmt: &Statement) -> Result {
        Ok(match stmt {
            Statement::Decl(Decl { name, value }) => {
                let ptr = self.context.stack_alloc();
                self.decl_var(name.clone(), &ptr);

                if let Some(value) = value {
                    let value = self.trans_expr(value)?;
                    self.context.copy(&value, &ptr);
                }
            }
            Statement::Assign(Assign { name, value }) => {
                let value = self.trans_expr(value)?;
                let ptr = self.resolve_var(name)?;
                self.context.copy(&value, &ptr);
            }
            Statement::AddAssign(AddAssign { name, value }) => {
                let value = self.trans_expr(value)?;
                let ptr = self.resolve_var(name)?;
                self.context.add(&ptr, &value);
            }
            Statement::While(While { cond, body }) => self.trans_stmt_while(cond, body)?,
            Statement::If(if_) => self.trans_stmt_if(if_)?,
        })
    }

    fn trans_stmt_while(&mut self, cond: &Expr, body: &[Statement]) -> Result {
        let tmp = self.trans_expr(cond)?;
        self.context.seek(&tmp);
        self.context.emit("[");
        self.context.forget_known_values();

        self.push_scope();

        for stmt in body {
            self.trans_stmt(stmt)?;
        }

        self.pop_scope();

        drop(tmp);
        let tmp = self.trans_expr(cond)?;
        self.context.seek(&tmp);
        self.context.emit("]");

        Ok(())
    }

    fn trans_stmt_if(&mut self, If { cond, body }: &If) -> Result {
        let cond = &self.trans_expr(cond)?;
        let tmp = &self.context.stack_alloc();
        self.context.copy(cond, tmp);

        self.context.seek(tmp);
        self.context.emit("[");
        self.context.forget_known_values();

        self.push_scope();

        for stmt in body {
            self.trans_stmt(stmt)?;
        }

        self.pop_scope();

        self.context.decrement(tmp);
        self.context.seek(tmp);
        self.context.emit("]");

        Ok(())
    }

    fn trans_expr(&mut self, expr: &Expr) -> Result<Ptr> {
        use Expr::*;
        Ok(match expr {
            Const(value) => {
                let ptr = self.context.stack_alloc();
                self.context.set(&ptr, *value);
                ptr
            }
            Var(name) => {
                let var = self.resolve_var(name)?;
                let ret = self.context.stack_alloc();
                self.context.copy(&var, &ret);
                ret
            },
            Add(a, b) => {
                let a = self.trans_expr(a)?;
                let b = self.trans_expr(b)?;
                self.context.add(&a, &b);
                a
            }
            Sub(a, b) => {
                let a = self.trans_expr(a)?;
                let b = self.trans_expr(b)?;
                self.context.sub(&a, &b);
                a
            }
            Gt(a, b) => {
                let a = &self.trans_expr(a)?;
                let b = &self.trans_expr(b)?;
                let res = self.context.stack_alloc();

                self.context.greater_than(a, b, &res);

                res
            }
        })
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop_scope(&mut self) {
        println!("scopes: {:#?}", self.scopes);
        self.scopes.pop();
    }

    fn decl_var(&mut self, name: Ident, ptr: &Ptr) {
        self.scopes.last_mut().unwrap().decl_var(name, ptr);
    }

    fn find_var(&self, name: &Ident) -> Result<&Var> {
        self.scopes.iter()
            .rev()
            .flat_map(|scope| scope.find_var(name))
            .next()
            .ok_or_else(|| format!("Variable '{}' is not in scope", &**name).into())
    }

    fn resolve_var(&self, name: &Ident) -> Result<Ptr> {
        Ok(self.find_var(name)?.ptr.clone())
    }
}

#[derive(Debug)]
struct Scope {
    variables: Vec<Var>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: Vec::new(),
        }
    }

    fn decl_var(&mut self, name: Ident, ptr: &Ptr) {
        self.variables.push(Var {
            name,
            ptr: ptr.clone(),
        });
    }

    fn find_var(&self, name: &Ident) -> Option<&Var> {
        self.variables.iter()
            .rev()
            .find(|var| var.name == *name)
    }
}

#[derive(Debug)]
struct Var {
    name: Ident,
    ptr: Ptr,
}
