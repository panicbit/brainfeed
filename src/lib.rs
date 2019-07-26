
pub struct Context<'c> {
    code: &'c mut String,
    ptr: isize,
    occupied_stack: Vec<bool>,
}

impl<'c> Context<'c> {
    pub fn new(code: &'c mut String) -> Self {
        Self::with_ptr(code, 0)
    }

    pub fn with_ptr(code: &'c mut String, ptr: isize) -> Self {
        Self {
            code,
            ptr,
            occupied_stack: Vec::new(),
        }
    }

    pub fn stack_alloc(&mut self) -> isize {
        match self.occupied_stack.iter().position(|occupied| !occupied) {
            Some(ptr) => {
                self.occupied_stack[ptr] = true;
                ptr as isize
            },
            None => {
                self.occupied_stack.push(true);
                let ptr = self.occupied_stack.len() - 1;
                ptr as isize
            }
        }
    }

    pub fn stack_free(&mut self, ptr: isize) {
        assert!(ptr >= 0);

        let ptr = ptr as usize;

        assert!(ptr < self.occupied_stack.len());
        assert!(self.occupied_stack[ptr]);

        self.occupied_stack[ptr] = false;
    }

    pub fn with_stack_alloc<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, isize)
    {
        let ptr = self.stack_alloc();
        f(self, ptr);
        self.stack_free(ptr);
    }

    pub fn with_stack_alloc2<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, isize, isize)
    {
        self.with_stack_alloc(|ctx, ptr1|{
            ctx.with_stack_alloc(|ctx, ptr2| {
                f(ctx, ptr1, ptr2);
            })
        })
    }

    pub fn with_stack_alloc3<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, isize, isize, isize)
    {
        self.with_stack_alloc2(|ctx, ptr1, ptr2|{
            ctx.with_stack_alloc(|ctx, ptr3| {
                f(ctx, ptr1, ptr2, ptr3);
            })
        })
    }

    pub fn with_stack_alloc4<F> (&mut self, f: F)
    where
        F: FnOnce(&mut Context, isize, isize, isize, isize)
    {
        self.with_stack_alloc3(|ctx, ptr1, ptr2, ptr3|{
            ctx.with_stack_alloc(|ctx, ptr4| {
                f(ctx, ptr1, ptr2, ptr3, ptr4);
            })
        })
    }

    pub fn cell(&mut self, ptr: isize) -> CellContext<'_, 'c> {
        CellContext::new(self, ptr)
    }

    fn seek(&mut self, ptr: isize) {
        let offset = ptr - self.ptr;
        let direction = if offset.is_positive() { ">" } else { "<" };
        let offset = offset.abs() as usize;

        self.emit(&direction.repeat(offset));
        self.ptr = ptr;
    }

    pub fn clear(&mut self, ptr: isize) {
        self.cell(ptr).clear();
    }

    pub fn increment(&mut self, ptr: isize) {
        self.cell(ptr).increment();
    }

    pub fn decrement(&mut self, ptr: isize) {
        self.cell(ptr).decrement();
    }

    pub fn iff<F>(&mut self, cond: isize, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.repeat_reverse(cond, |ctx, _| f(ctx));
    }

    pub fn if_not<F>(&mut self, cond: isize, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.with_stack_alloc(|ctx, not_cond| {
            ctx.copy(cond, not_cond);
            ctx.not(not_cond);
            ctx.iff_destructive(not_cond, f);
        })
    }

    pub fn if_not_destructive<F>(&mut self, cond: isize, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.not(cond);
        self.iff_destructive(cond, f);
    }

    pub fn iff_destructive<F>(&mut self, cond: isize, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.repeat_reverse_destructive(cond, |ctx, _| f(ctx));
    }

    pub fn if_else<F, G>(&mut self, cond: isize, f: F, g: G)
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

    pub fn while_not_null<F>(&mut self, ptr: isize, f: F)
    where
        F: FnOnce(&mut Context),
    {
        self.seek(ptr);
        self.emit("[");
        f(self);
        self.seek(ptr);
        self.emit("]");
    }

    /// Runs the code emitted by `f` `*ptr` many times.
    /// Sideffect: *ptr = 0
    pub fn repeat_reverse_destructive<F> (&mut self, counter: isize, f: F)
    where
        F: FnOnce(&mut Context, isize)
    {
        self.while_not_null(counter, |ctx| {
            f(ctx, counter);
            ctx.decrement(counter);
        })
    }

    /// Runs the code emitted by `f` `*ptr` many times.
    pub fn repeat_reverse<F> (&mut self, ptr: isize, f: F)
    where
        F: FnOnce(&mut Context, isize)
    {
        self.with_stack_alloc(|ctx, counter| {
            ctx.copy(ptr, counter);
            ctx.repeat_reverse_destructive(counter, f);
        })
    }

    pub fn add(&mut self, a: isize, b: isize, target: isize) {
        self.copy(b, target);
        self.add_assign(a, target);
    }

    pub fn add_assign(&mut self, source: isize, target: isize) {
        assert_ne!(source, target);

        self.repeat_reverse(source, |ctx, _| {
            ctx.increment(target);
        })
    }

    pub fn multiply(&mut self, a: isize, b: isize, target: isize) {
        self.copy(b, target);
        self.multiply_assign(a, target);
    }

    pub fn multiply_assign(&mut self, source: isize, target: isize) {
        assert_ne!(source, target);

        self.with_stack_alloc(|ctx, tmp| {
            ctx.mov(target, tmp);
            ctx.repeat_reverse_destructive(tmp, |ctx, _| {
                ctx.add_assign(source, target);
            })
        })
    }

    pub fn mov(&mut self, source: isize, target: isize) {
        if source == target {
            return;
        }

        self.clear(target);

        self.while_not_null(source, |ctx| {
            ctx.increment(target);
            ctx.decrement(source);
        })
    }

    pub fn is_zero_destructive(&mut self, value: isize) {
        self.with_stack_alloc(|ctx, is_zero| {
            ctx.cell(is_zero).set_bool(true);

            ctx.while_not_null(value, |ctx| {
                ctx.cell(is_zero).assume_bool(true).set_bool(false);
                ctx.cell(value).set_bool(false);
            });

            ctx.iff_destructive(is_zero, |ctx| {
                ctx.cell(value).assume_bool(false).set_bool(true);
            })
        })
    }

    pub fn is_zero(&mut self, source: isize, target: isize) {
        self.copy(source, target);
        self.is_zero_destructive(target);
    }

    pub fn greater_zero(&mut self, source: isize, target: isize) {
        self.copy(source, target);
        self.greater_zero_destructive(target);
    }

    pub fn greater_zero_destructive(&mut self, value: isize) {
        self.is_zero_destructive(value);
        self.not(value);
    }

    pub fn equals_assign(&mut self, source: isize, target: isize) {
        self.with_stack_alloc(|ctx, tmp| {
            ctx.copy(source, tmp);
            
            ctx.repeat_reverse_destructive(tmp, |ctx, _| {
                ctx.decrement(target);
            });

            ctx.is_zero_destructive(target);
        })
    }

    pub fn not_equals_assign(&mut self, source: isize, target: isize) {
        self.equals_assign(source, target);
        self.not(target);
    }

    pub fn copy(&mut self, source: isize, target: isize) {
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

    pub fn not(&mut self, cond: isize) {
        self.with_stack_alloc(|ctx, is_false| {
            ctx.cell(is_false).set(1);

            ctx.repeat_reverse_destructive(cond, |ctx, _| {
                ctx.decrement(is_false);
            });

            ctx.repeat_reverse_destructive(is_false, |ctx, _| {
                ctx.increment(cond);
            });
        })
    }

    pub fn and_assign(&mut self, source: isize, target: isize) {
        self.with_stack_alloc(|ctx, tmp| {
            ctx.mov(target, tmp);

            ctx.iff(source, |ctx| {
                ctx.iff_destructive(tmp, |ctx| {
                    ctx.cell(target).increment_by(1);
                })
            })
        });
    }

    pub fn and(&mut self, a: isize, b: isize, target: isize) {
        self.copy(b, target);
        self.and_assign(a, target);
    }

    pub fn or_assign(&mut self, source: isize, target: isize) {
        self.with_stack_alloc(|ctx, tmp| {
            ctx.mov(target, tmp);

            ctx.iff(source, |ctx| {
                ctx.cell(target).assume_bool(false).set_bool(true);
            });

            ctx.iff_destructive(tmp, |ctx| {
                ctx.cell(target).set_bool(true);
            })
        });
    }

    pub fn or(&mut self, a: isize, b: isize, target: isize) {
        self.copy(b, target);
        self.or_assign(a, target);
    }

    pub fn xor_assign(&mut self, source: isize, target: isize) {
        self.equals_assign(source, target);
    }

    pub fn xor(&mut self, a: isize, b: isize, target: isize) {
        self.copy(b, target);
        self.xor_assign(a, target);
    }

    pub fn emit(&mut self, code: &str) {
        self.code.push_str(code);
    }

    pub fn ptr(&self) -> isize {
        self.ptr
    }
}

pub struct CellContext<'ctx, 'c> {
    context: &'ctx mut Context<'c>,
    ptr: isize,
    current_value: Option<u8>,
}

impl<'ctx, 'c> CellContext<'ctx, 'c> {
    pub fn new(context: &'ctx mut Context<'c>, ptr: isize) -> Self {
        Self {
            context,
            ptr,
            current_value: None,
        }
    }

    pub fn assume(&mut self, value: u8) -> &mut Self {
        self.current_value = Some(value);
        self
    }

    pub fn assume_bool(&mut self, value: bool) -> &mut Self {
        debug_assert!(false as u8 == 0);
        debug_assert!(true as u8 == 1);

        self.current_value = Some(value as u8);
        self
    }

    fn seek(&mut self) {
        self.context.seek(self.ptr);
    }

    fn emit(&mut self, code: &str) -> &mut Self {
        self.context.emit(code);
        self
    }

    fn map_current_value<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(u8) -> u8,
    {
        self.current_value = self.current_value.map(f);
        self
    }

    pub fn clear(&mut self) -> &mut Self {
        if self.current_value == Some(0) {
            return self;
        }

        self.seek();
        self.emit("[-]");
        self.assume(0)
    }

    pub fn set(&mut self, value: u8) -> &mut Self {
        if self.current_value == Some(value) {
            return self;
        }

        self.seek();
        self.clear();
        self.increment_by(value)
    }

    pub fn set_bool(&mut self, value: bool) -> &mut Self {
        debug_assert!(false as u8 == 0);
        debug_assert!(true as u8 == 1);

        match self.current_value {
            Some(0) => self.increment(),
            Some(1) => self.decrement(),
            _ => self.set(value as u8),
        }
    }

    pub fn increment(&mut self) -> &mut Self {
        self.seek();
        self.emit("+");
        self.map_current_value(|v| v + 1)
    }

    pub fn increment_by(&mut self, amount: u8) -> &mut Self {
        self.seek();
        self.context.emit(&"+".repeat(amount as usize));
        self.map_current_value(|v| v + amount)
    }

    pub fn decrement(&mut self) -> &mut Self {
        self.seek();
        self.emit("-");
        self.map_current_value(|v| v - 1)
    }

    pub fn decrement_by(&mut self, amount: u8) -> &mut Self {
        self.seek();
        self.context.emit(&"-".repeat(amount as usize));
        self.map_current_value(|v| v - amount)
    }

    pub fn print(&mut self) -> &mut Self {
        self.seek();
        self.emit(".")
    }

    pub fn read(&mut self) -> &mut Self {
        self.seek();
        self.current_value = None;
        self.emit(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minibf::VM;

    #[test]
    fn seek() {
        let code = gen(|ctx| {
            ctx.seek(3);
            ctx.emit("a");
            ctx.seek(1);
            ctx.emit("b");
            ctx.seek(5);
        });

        assert_eq!(code, ">>>a<<b>>>>");
    }

    #[test]
    fn while_not_null() {
        let code = gen(|ctx| {

            let a = ctx.stack_alloc();
            let i = ctx.stack_alloc();

            ctx.cell(a).set(2);
            ctx.cell(i).set(3);
            ctx.while_not_null(i, |ctx| {
                ctx.increment(a);
            });
        });

        assert_eq!(code, "[-]++>[-]+++[<+>]");
    }

    #[test]
    fn repeat_reverse_destructive() {
        let code = gen(|ctx| {

            let a = ctx.stack_alloc();
            let i = ctx.stack_alloc();

            ctx.cell(a).set(2);
            ctx.cell(i).set(3);

            ctx.repeat_reverse_destructive(i, |ctx, _| {
                ctx.increment(a);
            });
        });

        assert_eq!(code, "[-]++>[-]+++[<+>-]");
    }

    #[test]
    fn repeat_reverse() {
        let code = gen(|ctx| {
            let a = ctx.stack_alloc();
            let i = ctx.stack_alloc();

            ctx.cell(a).set(2);
            ctx.cell(i).set(3);

            ctx.repeat_reverse(i, |ctx, _| {
                ctx.increment(a);
            });
        });

        assert_eq!(code, "[-]++>[-]+++>[-]>[-]<<[>>+<<-]>>[<<+>+>-]<[<<+>>-]");
    }

    #[test]
    fn set() {
        let code = gen(|ctx| {
            ctx.cell(3).set(13);
        });

        assert_eq!(code, ">>>[-]+++++++++++++");
    }

    #[test]
    fn not() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc2(|ctx, a, b| {
                ctx.cell(a).set_bool(false);
                ctx.cell(b).set_bool(true);
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
                    ctx.cell(false_).set_bool(false);
                    ctx.cell(true_).set_bool(true);
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
                    ctx.cell(false_).set_bool(false);
                    ctx.cell(true_).set_bool(true);
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
                    ctx.cell(false_).set_bool(false);
                    ctx.cell(true_).set_bool(true);
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
    fn multiply() {
        let mem = run(|ctx| {
            ctx.with_stack_alloc4(|ctx, a, b, r1, r2| {
                ctx.cell(a).set(6);
                ctx.cell(b).set(7);
                ctx.multiply(a, b, r1);
                ctx.multiply(a, b, r2);
            })
        });

        assert_eq!(mem[..4], [6, 7, 42, 42]);
    }

    #[test]
    fn clear() {
        let code = gen(|ctx| {
            ctx.cell(3).clear();
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