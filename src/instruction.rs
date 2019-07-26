mod op_prelude {
    pub use super::Register::*;
    pub use super::Cell::*;
    pub use super::Immediate::*;
    pub use super::Ref::*;
    pub use super::Op::*;
}

pub enum Register {
    R1,
    R2,
}

pub enum Cell {
    Abs(u8),
    Rel(i8),
}

pub enum Immediate {
    U8(u8),
}

pub enum Ref {
    Immediate(Immediate),
    Cell(Cell),
    Register(Register),
}

pub enum Op {
    Mov(Ref, Ref),
}

impl Op {
    fn write(&self) {
        use op_prelude::*;
        match self {
            Mov(Immediate(_), _) => panic!("mov with immediate target"),
            Mov(_, Immediate(_)) => panic!("mov with immediate source"),
            _ => unimplemented!("foo"),
        }
    }
}
