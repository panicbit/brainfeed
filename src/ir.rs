use pest::Parser as _;
use pest::prec_climber::{PrecClimber, Assoc, Operator};
use pest_derive::*;
use std::ops::Deref;

#[derive(Parser)]
#[grammar = "ir.pest"]
struct Parser;

pub type Result<T, E = Error> = std::result::Result<T, E>;
pub type Error = Box<std::error::Error>;
pub type Pair<'a, R = Rule> = pest::iterators::Pair<'a, R>;
pub type Pairs<'a, R = Rule> = pest::iterators::Pairs<'a, R>;

#[derive(Debug)]
pub struct IR {
    code: Vec<Statement>,
}

impl IR {
    pub fn parse_str(code: &str) -> Result<Self> {
        let pairs = Parser::parse(Rule::ir, code)?.into_iter();

        Ok(Self {
            code: pairs
                .filter(|pair| pair.as_rule() != Rule::EOI)
                .map(Statement::parse)
                .collect::<Result<_>>()?,
        })
    }
}

#[derive(Debug)]
pub enum Statement {
    Decl(Decl),
    Assign(Assign),
    While(While),
    If(If),
}

impl Statement {
    fn parse(pair: Pair) -> Result<Self> {
        ensure_rule(&pair, Rule::stmt)?;

        let pair = pair.into_inner().next().unwrap() ;

        Ok(match pair.as_rule() {
            Rule::stmt_decl => Statement::Decl(Decl::parse(pair)?),
            Rule::stmt_assign => Statement::Assign(Assign::parse(pair)?),
            Rule::stmt_while => Statement::While(While::parse(pair)?),
            Rule::stmt_if => Statement::If(If::parse(pair)?),
            rule => Err(format!("BUG: unhandled stmt rule: {:?}", rule))?,
        })
    }
}

#[derive(Debug)]
pub struct Decl {
    name: Ident,
    value: Option<Expr>,
}

impl Decl {
    fn parse(pair: Pair) -> Result<Self> {
        ensure_rule(&pair, Rule::stmt_decl)?;

        let mut pairs = pair.into_inner();

        Ok(Self {
            name: Ident::parse(pairs.next().unwrap())?,
            value: pairs.next().map(Expr::parse).transpose()?,
        })
    }
}

#[derive(Debug)]
pub struct Assign {
    name: Ident,
    value: Expr,
}

impl Assign {
    fn parse(pair: Pair) -> Result<Self> {
        ensure_rule(&pair, Rule::stmt_assign)?;

        let mut pairs = pair.into_inner();

        Ok(Self {
            name: Ident::parse(pairs.next().unwrap())?,
            value: Expr::parse(pairs.next().unwrap())?,
        })
    }
}

#[derive(Debug)]
pub struct While {
    cond: Expr,
    body: Vec<Statement>,
}

impl While {
    fn parse(pair: Pair) -> Result<Self> {
        ensure_rule(&pair, Rule::stmt_while)?;

        let mut pairs = pair.into_inner();

        Ok(Self {
            cond: Expr::parse(pairs.next().unwrap())?,
            body: pairs.map(Statement::parse).collect::<Result<_>>()?,
        })
    }
}

#[derive(Debug)]
pub struct If {
    cond: Expr,
    body: Vec<Statement>,
}

impl If {
    fn parse(pair: Pair) -> Result<Self> {
        ensure_rule(&pair, Rule::stmt_if)?;

        let mut pairs = pair.into_inner();

        Ok(Self {
            cond: Expr::parse(pairs.next().unwrap())?,
            body: pairs.map(Statement::parse).collect::<Result<_>>()?,
        })
    }
}

lazy_static! {
    static ref EXPR_CLIMBER: PrecClimber<Rule> = {
        use Rule::*;
        use Assoc::*;

        PrecClimber::new(vec![
            Operator::new(op_gt, Left),
            Operator::new(op_add, Left) | Operator::new(op_sub, Left),
        ])
    };
}

#[derive(Debug)]
pub enum Expr {
    Const(u8),
    Var(Ident),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
}

impl Expr {
    fn parse(pair: Pair) -> Result<Self> {
        ensure_rule(&pair, Rule::expr)?;

        EXPR_CLIMBER.climb(
            pair.into_inner(),
            Self::parse_term,
            Self::parse_op,
        )
    }

    fn parse_term(pair: Pair) -> Result<Self> {
        let rule = pair.as_rule();
        let mut pairs = pair.into_inner();

        Ok(match rule {
            Rule::expr_const => Expr::Const(pairs.as_str().parse()?),
            Rule::expr_var => Expr::Var(Ident::parse(pairs.next().unwrap())?),
            rule => Err(format!("BUG: Unhandled term rule: {:?}", rule))?,
        })
    }

    fn parse_op(lhs: Result<Expr>, op: Pair, rhs: Result<Expr>) -> Result<Self> {
        ensure_rule(&op, Rule::op)?;

        let lhs = Box::new(lhs?);
        let rhs = Box::new(rhs?);

        Ok(match op.as_rule() {
            Rule::op_add => Expr::Add(lhs, rhs),
            Rule::op_sub => Expr::Sub(lhs, rhs),
            Rule::op_gt => Expr::Gt(lhs, rhs),
            rule => Err(format!("BUG: Unhandled op rule: {:?}", rule))?,
        })
    }
}

#[derive(Debug)]
pub struct Ident(String);

impl Ident {
    fn parse(pair: Pair) -> Result<Self> {
        ensure_rule(&pair, Rule::ident)?;

        Ok(Ident(pair.as_str().into()))
    }
}

impl Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn ensure_rule(pair: &Pair, rule: Rule) -> Result<()> {
    if pair.as_rule() != rule {
        Err(format!("BUG: Expected {:?}, found {:?}", rule, pair.as_rule()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ir() {
        IR::parse_str("
            while x {
                let y = 1 + 2
            }
        ").unwrap();
    }
}
