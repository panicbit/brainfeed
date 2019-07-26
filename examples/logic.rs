use brainfeed::Context;

fn main() {
    let mut code = String::new();
    let mut ctx = Context::new(&mut code);

    ctx.with_stack_alloc3(|ctx, a, b, res| {
        ctx.cell(a).set_bool(true);
        ctx.cell(b).set_bool(true);
        ctx.copy(a, res);
        ctx.xor_assign(b, res);
    });

    println!("{}", code);
}
