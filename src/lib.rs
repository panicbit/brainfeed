#[macro_use] extern crate lazy_static;

use std::sync::{Arc, Weak};
use std::ops;
use std::cmp;

pub mod ir;
pub mod trans;

#[derive(Debug,Clone,PartialEq,PartialOrd)]
pub struct Ptr(Arc<isize>);

impl Ptr {
    fn new(addr: isize) -> Self {
        Ptr(Arc::new(addr))
    }

    fn as_isize(&self) -> isize {
        *self.0
    }

    fn weak(&self) -> Weak<isize> {
        Arc::downgrade(&self.0)
    }
}

impl<'a> ops::Add for &'a Ptr {
    type Output = Ptr;

    fn add(self, other: Self) -> Ptr {
        let addr = self.as_isize() + other.as_isize();
        Ptr::new(addr)
    }
}

impl<'a> ops::Sub for &'a Ptr {
    type Output = Ptr;

    fn sub(self, other: Self) -> Ptr {
        let addr = self.as_isize() - other.as_isize();
        Ptr::new(addr)
    }
}

impl<'a> cmp::PartialEq<isize> for &'a Ptr {
    fn eq(&self, addr: &isize) -> bool {
        &self.as_isize() == addr
    }
}

impl<'a> cmp::PartialOrd<isize> for &'a Ptr {
    fn partial_cmp(&self, addr: &isize) -> Option<cmp::Ordering> {
        self.as_isize().partial_cmp(addr)
    }
}

pub struct Context<'c> {
    code: &'c mut String,
    addr: isize,
    stack_pointers: Vec<Weak<isize>>,
    known_values: Vec<Option<u8>>,
}

impl<'c> Context<'c> {
    pub fn new(code: &'c mut String) -> Self {
        Self::with_addr(code, 0)
    }

    pub fn with_addr(code: &'c mut String, addr: isize) -> Self {
        Self {
            code,
            addr,
            stack_pointers: Vec::new(),
            known_values: Vec::new(),
        }
    }

    pub fn forget_known_values(&mut self) {
        for known_value in &mut self.known_values {
            *known_value = None;
        }
    }

    pub fn map_known_value<F>(&mut self, ptr: &Ptr, f: F)
    where
        F: FnOnce(u8) -> u8,
    {
        if ptr < 0 {
            return;
        }

        self.known_values
            .get_mut(ptr.as_isize() as usize)
            .and_then(|value| value.as_mut())
            .map(|value| *value = f(*value));
    }

    pub fn assume(&mut self, ptr: &Ptr, value: u8) {
        if ptr < 0 {
            return;
        }

        let addr = ptr.as_isize() as usize;

        while addr >= self.known_values.len() {
            self.known_values.push(None);
        }

        self.known_values[addr] = Some(value);
    }

    pub fn assume_bool(&mut self, ptr: &Ptr, value: bool) {
        debug_assert!(false as u8 == 0);
        debug_assert!(true as u8 == 1);

        self.assume(ptr, value as u8);
    }

    pub fn value(&self, ptr: &Ptr) -> Option<u8> {
        if ptr < 0 {
            return None;
        }

        self.known_values
            .get(ptr.as_isize() as usize)
            .and_then(|value| *value)
    }

    pub fn forget(&mut self, ptr: &Ptr) {
        if ptr < 0 {
            return;
        }

        self.known_values
            .get_mut(ptr.as_isize() as usize)
            .map(|value| *value = None);
    }

    pub fn stack_alloc(&mut self) -> Ptr {
        match self.stack_pointers.iter().position(|ptr| ptr.upgrade().is_none()) {
            Some(addr) => {
                let ptr = Ptr(Arc::new(addr as isize));
                self.stack_pointers[addr] = ptr.weak();
                ptr
            },
            None => {
                let addr = self.stack_pointers.len();
                let ptr = Ptr(Arc::new(addr as isize));
                self.stack_pointers.push(ptr.weak());
                ptr
            }
        }
    }

    pub fn with_stack_alloc<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, &Ptr)
    {
        let ptr = self.stack_alloc();
        f(self, &ptr);
    }

    pub fn with_stack_alloc2<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, &Ptr, &Ptr)
    {
        self.with_stack_alloc(|ctx, ptr1|{
            ctx.with_stack_alloc(|ctx, ptr2| {
                f(ctx, ptr1, ptr2);
            })
        })
    }

    pub fn with_stack_alloc3<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, &Ptr, &Ptr, &Ptr)
    {
        self.with_stack_alloc2(|ctx, ptr1, ptr2|{
            ctx.with_stack_alloc(|ctx, ptr3| {
                f(ctx, ptr1, ptr2, ptr3);
            })
        })
    }

    pub fn with_stack_alloc4<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, &Ptr, &Ptr, &Ptr, &Ptr)
    {
        self.with_stack_alloc3(|ctx, ptr1, ptr2, ptr3|{
            ctx.with_stack_alloc(|ctx, ptr4| {
                f(ctx, ptr1, ptr2, ptr3, ptr4);
            })
        })
    }

    pub fn with_stack_alloc5<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, &Ptr, &Ptr, &Ptr, &Ptr, &Ptr)
    {
        self.with_stack_alloc4(|ctx, ptr1, ptr2, ptr3, ptr4|{
            ctx.with_stack_alloc(|ctx, ptr5| {
                f(ctx, ptr1, ptr2, ptr3, ptr4, ptr5);
            })
        })
    }

    fn seek(&mut self, ptr: &Ptr) {
        let offset = ptr.as_isize() - self.addr;
        let direction = if offset.is_positive() { ">" } else { "<" };
        let offset = offset.abs() as usize;

        self.emit(&direction.repeat(offset));
        self.addr = ptr.as_isize();
    }

    pub fn clear(&mut self, ptr: &Ptr) {
        if self.value(ptr) == Some(0) {
            return;
        }

        self.seek(ptr);
        self.emit("[-]");
        self.assume(ptr, 0);
    }

    pub fn set(&mut self, ptr: &Ptr, value: u8) {
        if self.value(ptr) == Some(value) {
            return;
        }

        self.seek(ptr);
        self.clear(ptr);
        self.increment_by(ptr, value);
    }

    pub fn set_bool(&mut self, ptr: &Ptr, value: bool) {
        debug_assert!(false as u8 == 0);
        debug_assert!(true as u8 == 1);

        match self.value(ptr) {
            Some(0) => self.increment(ptr),
            Some(1) => self.decrement(ptr),
            _ => self.set(ptr, value as u8),
        }
    }

    pub fn print(&mut self, ptr: &Ptr) {
        self.seek(ptr);
        self.emit(".");
    }

    pub fn read(&mut self, ptr: &Ptr) {
        self.seek(ptr);
        self.forget(ptr);
        self.emit(",");
    }

    pub fn increment(&mut self, ptr: &Ptr) {
        self.seek(ptr);
        self.emit("+");
        self.map_known_value(ptr, |v| v + 1)
    }

    pub fn increment_by(&mut self, ptr: &Ptr, amount: u8) {
        self.seek(ptr);
        self.emit(&"+".repeat(amount as usize));
        self.map_known_value(ptr, |v| v + amount)
    }

    pub fn decrement(&mut self, ptr: &Ptr) {
        self.seek(ptr);
        self.emit("-");
        self.map_known_value(ptr, |v| v - 1)
    }

    pub fn decrement_by(&mut self, ptr: &Ptr, amount: u8) {
        self.seek(ptr);
        self.emit(&"-".repeat(amount as usize));
        self.map_known_value(ptr, |v| v - amount)
    }

    pub fn iff<F>(&mut self, cond: &Ptr, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.repeat_reverse(cond, |ctx, _| f(ctx));
    }

    pub fn if_not<F>(&mut self, cond: &Ptr, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.with_stack_alloc(|ctx, not_cond| {
            ctx.copy(cond, not_cond);
            ctx.not(not_cond);
            ctx.iff_destructive(not_cond, f);
        })
    }

    pub fn if_not_destructive<F>(&mut self, cond: &Ptr, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.not(cond);
        self.iff_destructive(cond, f);
    }

    pub fn iff_destructive<F>(&mut self, cond: &Ptr, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.repeat_reverse_destructive(cond, |ctx, _| f(ctx));
    }

    pub fn if_else<F, G>(&mut self, cond: &Ptr, f: F, g: G)
    where
        F: FnOnce(&mut Context),
        G: FnOnce(&mut Context),
    {
        self.with_stack_alloc(|ctx, tmp_cond| {
            ctx.copy(cond, tmp_cond);
            ctx.iff(cond, f);
            ctx.if_not_destructive(tmp_cond, g);
        });
    }

    pub fn while_not_zero<F>(&mut self, ptr: &Ptr, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.seek(ptr);
        self.emit("[");
        self.forget_known_values();
        f(self);
        self.seek(ptr);
        self.emit("]");
    }

    pub fn while_true<F>(&mut self, cond: &Ptr, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.while_not_zero(cond, f);
    }


    /// Runs the code emitted by `f` `*ptr` many times.
    /// Sideffect: *ptr = 0
    pub fn repeat_reverse_destructive<F> (&mut self, counter: &Ptr, f: F)
    where
        F: FnOnce(&mut Context, &Ptr)
    {
        self.while_not_zero(counter, |ctx| {
            f(ctx, counter);
            ctx.decrement(counter);
        })
    }

    /// Runs the code emitted by `f` `*ptr` many times.
    pub fn repeat_reverse<F> (&mut self, ptr: &Ptr, f: F)
    where
        F: FnOnce(&mut Context, &Ptr)
    {
        self.with_stack_alloc(|ctx, counter| {
            ctx.copy(ptr, counter);
            ctx.repeat_reverse_destructive(counter, f);
        })
    }

    /// target = target + source; source = 0;
    pub fn add(&mut self, target: &Ptr, source: &Ptr) {
        assert_ne!(source, target);

        self.repeat_reverse_destructive(source, |ctx, _| {
            ctx.increment(target);
        });
    }

    /// target = target - source; source = 0;
    pub fn sub(&mut self, target: &Ptr, source: &Ptr) {
        assert_ne!(source, target);

        self.repeat_reverse_destructive(source, |ctx, _| {
            ctx.decrement(target);
        });
    }

    /// target = target * source;
    pub fn mul(&mut self, target: &Ptr, source: &Ptr) {
        assert_ne!(source, target);

        self.with_stack_alloc2(|ctx, product, tmp| {
            ctx.clear(product);

            ctx.repeat_reverse_destructive(target, |ctx, _| {
                ctx.copy(source, tmp);
                ctx.add(product, tmp);
            });

            ctx.mov(product, target);
        })
    }

    pub fn mov(&mut self, source: &Ptr, target: &Ptr) {
        if source == target {
            return;
        }

        self.clear(target);

        self.while_not_zero(source, |ctx| {
            ctx.increment(target);
            ctx.decrement(source);
        })
    }

    pub fn is_zero_destructive(&mut self, value: &Ptr) {
        self.with_stack_alloc(|ctx, is_zero| {
            ctx.set_bool(is_zero, true);

            ctx.while_not_zero(value, |ctx| {
                ctx.assume_bool(is_zero, true);
                ctx.set_bool(is_zero, false);
                ctx.set_bool(value, false);
            });

            ctx.iff_destructive(is_zero, |ctx| {
                ctx.assume_bool(value, false);
                ctx.set_bool(value, true);
            })
        })
    }

    pub fn is_zero(&mut self, source: &Ptr, target: &Ptr) {
        self.copy(source, target);
        self.is_zero_destructive(target);
    }

    pub fn is_not_zero_destructive(&mut self, value: &Ptr) {
        self.is_zero_destructive(value);
        self.not(value);
    }

    pub fn is_not_zero(&mut self, source: &Ptr, target: &Ptr) {
        self.is_zero(source, target);
        self.not(target);
    }

    pub fn equals_assign(&mut self, source: &Ptr, target: &Ptr) {
        self.with_stack_alloc(|ctx, tmp| {
            ctx.copy(source, tmp);
            
            ctx.repeat_reverse_destructive(tmp, |ctx, _| {
                ctx.decrement(target);
            });

            ctx.is_zero_destructive(target);
        })
    }

    pub fn equals(&mut self, a: &Ptr, b: &Ptr, target: &Ptr) {
        self.copy(b, target);
        self.equals_assign(a, target);
    }

    pub fn greater_than_assign(&mut self, source: &Ptr, target: &Ptr) {
        if let (Some(source_val), Some(target_val)) = (self.value(source), self.value(target)) {
            self.set_bool(target, source_val > target_val);
            return;
        }

        self.with_stack_alloc4(|ctx, tmp, tmp_is_zero, target_is_zero, neither_is_zero| {
            ctx.copy(source, tmp);

            ctx.is_zero(tmp, tmp_is_zero);
            ctx.is_zero(target, target_is_zero);
            ctx.nor(tmp_is_zero, target_is_zero, neither_is_zero);

            ctx.while_true(neither_is_zero, |ctx| {
                ctx.decrement(tmp);
                ctx.decrement(target);

                ctx.is_zero(tmp, tmp_is_zero);
                ctx.is_zero(target, target_is_zero);
                ctx.nor(tmp_is_zero, target_is_zero, neither_is_zero);
            });

            ctx.and_not(target_is_zero, tmp_is_zero, target);
        })
    }

    pub fn greater_than(&mut self, a: &Ptr, b: &Ptr, target: &Ptr) {
        if let (Some(a), Some(b)) = (self.value(a), self.value(b)) {
            self.set_bool(target, a > b);
            return;
        }
        self.copy(b, target);
        self.greater_than_assign(a, target);
    }

    pub fn not_equals_assign(&mut self, source: &Ptr, target: &Ptr) {
        self.equals_assign(source, target);
        self.not(target);
    }

    pub fn copy(&mut self, source: &Ptr, target: &Ptr) {
        if source == target {
            return;
        }

        self.with_stack_alloc(|ctx, tmp| {
            ctx.clear(target);
            ctx.mov(source, tmp);
            ctx.repeat_reverse_destructive(tmp, |ctx, _| {
                ctx.increment(source);
                ctx.increment(target);
            });
        })
    }

    pub fn not(&mut self, cond: &Ptr) {
        self.with_stack_alloc(|ctx, is_false| {
            ctx.set(is_false, 1);

            ctx.repeat_reverse_destructive(cond, |ctx, _| {
                ctx.decrement(is_false);
            });

            ctx.repeat_reverse_destructive(is_false, |ctx, _| {
                ctx.increment(cond);
            });
        })
    }

    pub fn and_assign(&mut self, source: &Ptr, target: &Ptr) {
        self.with_stack_alloc(|ctx, tmp| {
            ctx.mov(target, tmp);

            ctx.iff(source, |ctx| {
                ctx.iff_destructive(tmp, |ctx| {
                    ctx.increment_by(target, 1);
                })
            })
        });
    }

    pub fn and(&mut self, a: &Ptr, b: &Ptr, target: &Ptr) {
        assert_ne!(a, target);
        assert_ne!(b, target);
        self.copy(b, target);
        self.and_assign(a, target);
    }

    pub fn and_not(&mut self, a: &Ptr, b: &Ptr, target: &Ptr) {
        self.copy(b, target);
        self.not(target);
        self.and_assign(a, target);
    }

    pub fn or_assign(&mut self, source: &Ptr, target: &Ptr) {
        self.with_stack_alloc(|ctx, tmp| {
            ctx.mov(target, tmp);

            ctx.iff(source, |ctx| {
                ctx.assume_bool(target, false);
                ctx.set_bool(target, true);
            });

            ctx.iff_destructive(tmp, |ctx| {
                ctx.set_bool(target, true);
            })
        });
    }

    pub fn or(&mut self, a: &Ptr, b: &Ptr, target: &Ptr) {
        assert_ne!(a, target);
        assert_ne!(b, target);
        self.copy(b, target);
        self.or_assign(a, target);
    }

    pub fn nor_assign(&mut self, source: &Ptr, target: &Ptr) {
        self.or_assign(source, target);
        self.not(target);
    }

    pub fn nor(&mut self, a: &Ptr, b: &Ptr, target: &Ptr) {
        assert_ne!(a, target);
        assert_ne!(b, target);
        self.copy(b, target);
        self.nor_assign(a, target);
    }

    pub fn xor_assign(&mut self, source: &Ptr, target: &Ptr) {
        self.equals_assign(source, target);
    }

    pub fn xor(&mut self, a: &Ptr, b: &Ptr, target: &Ptr) {
        assert_ne!(a, target);
        assert_ne!(b, target);
        self.copy(b, target);
        self.xor_assign(a, target);
    }

    pub fn emit(&mut self, code: &str) {
        self.code.push_str(code);
    }

    pub fn addr(&self) -> isize {
        self.addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minibf::VM;

    #[test]
    fn seek() {
        let code = gen(|ctx| {
            ctx.seek(&Ptr::new(3));
            ctx.emit("a");
            ctx.seek(&Ptr::new(1));
            ctx.emit("b");
            ctx.seek(&Ptr::new(5));
        });

        assert_eq!(code, ">>>a<<b>>>>");
    }

    #[test]
    fn while_not_zero() {
        let code = gen(|ctx| {
            let a = &ctx.stack_alloc();
            let i = &ctx.stack_alloc();

            ctx.set(a, 2);
            ctx.set(i, 3);
            ctx.while_not_zero(i, |ctx| {
                ctx.increment(a);
            });
        });

        assert_eq!(code, "[-]++>[-]+++[<+>]");
    }

    #[test]
    fn repeat_reverse_destructive() {
        let code = gen(|ctx| {

            let a = &ctx.stack_alloc();
            let i = &ctx.stack_alloc();

            ctx.set(a, 2);
            ctx.set(i, 3);

            ctx.repeat_reverse_destructive(i, |ctx, _| {
                ctx.increment(a);
            });
        });

        assert_eq!(code, "[-]++>[-]+++[<+>-]");
    }

    #[test]
    fn repeat_reverse() {
        let code = gen(|ctx| {
            let a = &ctx.stack_alloc();
            let i = &ctx.stack_alloc();

            ctx.set(a, 2);
            ctx.set(i, 3);

            ctx.repeat_reverse(i, |ctx, _| {
                ctx.increment(a);
            });
        });

        assert_eq!(code, "[-]++>[-]+++>[-]>[-]<<[>>+<<-]>>[<<+>+>-]<[<<+>>-]");
    }

    #[test]
    fn set() {
        let code = gen(|ctx| {
            ctx.set(&Ptr::new(3), 13);
        });

        assert_eq!(code, ">>>[-]+++++++++++++");
    }

    #[test]
    fn not() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc2(|ctx, a, b| {
                ctx.set_bool(a, false);
                ctx.set_bool(b, true);
                ctx.not(a);
                ctx.not(b);
            })
        });

        assert_eq!(mem[..2], [1, 0]);
    }

    #[test]
    fn or() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc2(|ctx, false_, true_| {
                ctx.with_stack_alloc4(|ctx, a, b, c, d| {
                    ctx.set_bool(false_, false);
                    ctx.set_bool(true_, true);
                    ctx.or(false_, false_, a);
                    ctx.or(false_,  true_, b);
                    ctx.or( true_, false_, c);
                    ctx.or( true_,  true_, d);
                })
            })
        });

        assert_eq!(mem[..6], [0, 1, 0, 1, 1, 1]);
    }

    #[test]
    fn xor() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc2(|ctx, false_, true_| {
                ctx.with_stack_alloc4(|ctx, a, b, c, d| {
                    ctx.set_bool(false_, false);
                    ctx.set_bool(true_, true);
                    ctx.xor(false_, false_, a);
                    ctx.xor(false_,  true_, b);
                    ctx.xor( true_, false_, c);
                    ctx.xor( true_,  true_, d);
                })
            })
        });

        assert_eq!(mem[..6], [0, 1, 1, 0, 0, 1]);
    }

    #[test]
    fn and() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc2(|ctx, false_, true_| {
                ctx.with_stack_alloc4(|ctx, a, b, c, d| {
                    ctx.set_bool(false_, false);
                    ctx.set_bool(true_, true);
                    ctx.and(false_, false_, a);
                    ctx.and(false_,  true_, b);
                    ctx.and( true_, false_, c);
                    ctx.and( true_,  true_, d);
                })
            })
        });

        assert_eq!(mem[..6], [0, 1, 0, 0, 0, 1]);
    }

    #[test]
    fn add() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc4(|ctx, a, b, c, d| {
                ctx.set(a, 6);
                ctx.set(b, 7);
                ctx.set(c, 8);
                ctx.set(d, 9);
                ctx.add(a, b);
                ctx.add(d, c);
            })
        });

        assert_eq!(mem[..4], [13, 0, 0, 17]);
    }

    #[test]
    fn mul() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc4(|ctx, a, b, c, d| {
                ctx.set(a, 6);
                ctx.set(b, 7);
                ctx.set(c, 8);
                ctx.set(d, 9);
                ctx.mul(a, b);
                ctx.mul(d, c);
            })
        });

        assert_eq!(mem[..4], [42, 7, 8, 72]);
    }


    #[test]
    fn sub() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc4(|ctx, a, b, c, d| {
                ctx.set(a, 9);
                ctx.set(b, 8);
                ctx.set(c, 6);
                ctx.set(d, 7);
                ctx.sub(a, b);
                ctx.sub(d, c);
            })
        });

        assert_eq!(mem[..4], [1, 0, 0, 1]);
    }

    #[test]
    fn greater_than() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc5(|ctx, a, b, r1, r2, r3| {
                ctx.set(a, 6);
                ctx.set(b, 10);
                ctx.greater_than(a, b, r1);
                ctx.greater_than(b, a, r2);
                ctx.greater_than(a, a, r3);
            })
        });

        assert_eq!(mem[..5], [6, 10, 0, 1, 0]);
    }

    #[test]
    fn clear() {
        let code = gen(|ctx| {
            ctx.clear(&Ptr::new(3));
        });

        assert_eq!(code, ">>>[-]");
    }

    fn gen<F>(f: F) -> String
    where
        F: FnOnce(&mut Context),
    {
        let mut code = String::new();
        let mut ctx = Context::new(&mut code);
        f(&mut ctx);

        code
    }

    fn run<F>(f: F) -> Vec<u8>
    where
        F: FnOnce(&mut Context),
    {
        let code = gen(f);
        let mut vm = VM::new();

        println!("code: {}", code);

        vm.run(&code);
        vm.mem().to_vec()
    }
}