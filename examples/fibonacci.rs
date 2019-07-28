use brainfeed::Context;

fn main() {
    let nth_fib = 7;
    let mut code = String::new();
    let mut ctx = Context::new(&mut code);

    ctx.with_stack_alloc4(|ctx, current, next, i, tmp| {
        ctx.cell(next).increment_by(1);
        ctx.cell(i).increment_by(nth_fib);

        ctx.repeat_reverse_destructive(i, |ctx, _| {
            ctx.mov(current, tmp);
            ctx.copy(next, current);
            ctx.add(next, tmp);
        });

        ctx.clear(next);
    });

    println!("{}", code);
}
