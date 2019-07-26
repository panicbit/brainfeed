
const MAX_STEPS: usize = 1_000_000;
const MEM_SIZE: usize = 30_000;

pub struct VM {
    mem: [u8; MEM_SIZE],
    loop_stack: Vec<usize>,
    ip: usize,
    dp: usize,
    op_count: usize,
}

impl VM {
    pub fn new() -> Self {
        Self {
            mem: [0; MEM_SIZE],
            loop_stack: Vec::new(),
            ip: 0,
            dp: 0,
            op_count: 0,
        }
    }
    pub fn run<C: AsRef<[u8]>>(&mut self, code: C) {
        let code = code.as_ref();
        self.ip = 0;
        self.op_count = 0;

        while self.ip < code.len() {
            match code[self.ip] {
                b'<' => self.left(),
                b'>' => self.right(),
                b'+' => self.increment(),
                b'-' => self.decrement(),
                b'[' => self.loop_start(code),
                b']' => self.loop_end(),
                b'.' => unimplemented!("op: ."),
                b',' => unimplemented!("op: ,"),
                _ => {}
            }

            self.op_count += 1;
            assert!(self.op_count <= MAX_STEPS);
        }
    }

    pub fn mem(&self) -> &[u8; MEM_SIZE] {
        &self.mem
    }

    pub fn mem_mut(&mut self) -> &mut [u8; MEM_SIZE] {
        &mut self.mem
    }

    fn left(&mut self) {
        self.dp += MEM_SIZE;
        self.dp -= 1;
        self.dp %= MEM_SIZE;
        self.ip += 1;
    }

    fn right(&mut self) {
        self.dp += 1;
        self.dp %= MEM_SIZE;
        self.ip += 1;
    }

    fn increment(&mut self) {
        let cell = self.mem[self.dp];
        self.mem[self.dp] = cell.wrapping_add(1);
        self.ip += 1;
    }

    fn decrement(&mut self) {
        let cell = self.mem[self.dp];
        self.mem[self.dp] = cell.wrapping_sub(1);
        self.ip += 1;
    }

    fn loop_start(&mut self, code: &[u8]) {
        let cell = self.mem[self.dp];

        if cell != 0 {
            self.loop_stack.push(self.ip);
            self.ip += 1;
            return;
        }

        let mut unclosed_brackets = 1;
        while unclosed_brackets > 0 {
            self.ip += 1;
            match code[self.ip] {
                b'[' => unclosed_brackets += 1,
                b']' => unclosed_brackets -= 1,
                _ => {},
            }
        }

        self.ip += 1;
    }

    fn loop_end(&mut self) {
        self.ip = self.loop_stack.pop().expect("unmatched ']'");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left() {
        let mut vm = VM::new();

        vm.run("<");
        assert_eq!(vm.dp, MEM_SIZE-1);

        vm.run("<");
        assert_eq!(vm.dp, MEM_SIZE-2);
    }

    #[test]
    fn right() {
        let mut vm = VM::new();

        vm.run(">");
        assert_eq!(vm.dp, 1);

        for _ in 0..MEM_SIZE {
            vm.run(">");
        }
        assert_eq!(vm.dp, 1);
    }

    #[test]
    fn increment() {
        let mut vm = VM::new();

        vm.run("+>++>+++");
        assert_eq!(vm.mem()[..3], [1, 2, 3]);
    }

    #[test]
    fn decrement() {
        let mut vm = VM::new();

        vm.run("->-->---");
        assert_eq!(vm.mem()[..3], [255, 254, 253]);
    }

    #[test]
    fn loops() {
        let mut vm = VM::new();

        vm.run(">++++++[<+++++++>-]");
        assert_eq!(vm.mem()[..2], [42, 0]);
    }

    #[test]
    fn nested_loops() {
        let mut vm = VM::new();
        vm.run("[[[]]]");
    }

    #[test]
    #[should_panic(expected = "unmatched ']'")]
    fn unbalanced_loops() {
        VM::new().run("]");
    }
}
