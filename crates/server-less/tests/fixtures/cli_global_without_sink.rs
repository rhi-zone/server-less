use server_less::cli;

// Declaring `global = [...]` without implementing `CliGlobals` must be a compile
// error: the macro delivers each global flag to the named sink, so the missing
// `impl CliGlobals` surfaces as an unsatisfied trait bound (E0277) instead of a
// silently-inert flag. There is no blanket default impl of `CliGlobals`, so the
// bound can only be satisfied by an explicit impl.
struct MyApp;

#[cli(name = "my-app", global = [verbose])]
impl MyApp {
    /// Do the thing
    fn run(&self) {
        println!("ran");
    }
}

fn main() {}
