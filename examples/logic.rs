use brainfeed::Context;

fn main() {
    let mut code = String::new();
    let mut ctx = Context::new(&mut code);

    ctx.with_stack_alloc3(|ctx, a, b, res| {
        ctx.set_bool(a, true);
        ctx.set_bool(b, true);
        ctx.copy(a, res);
        ctx.xor_assign(b, res);
    });

    println!("{}", code);
}
