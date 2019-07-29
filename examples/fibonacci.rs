use brainfeed::Context;

fn main() {
    let nth_fib = 7;
    let mut code = String::new();
    let mut ctx = Context::new(&mut code);

    ctx.with_stack_alloc4(|ctx, current, next, i, tmp| {
        ctx.increment_by(next, 1);
        ctx.increment_by(i, nth_fib);

        ctx.repeat_reverse_destructive(i, |ctx, _| {
            ctx.mov(current, tmp);
            ctx.copy(next, current);
            ctx.add(next, tmp);
        });

        ctx.clear(next);
    });

    println!("{}", code);
}
