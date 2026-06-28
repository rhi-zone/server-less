use server_less::{cli, CliGlobals};

struct MyApp;

// The sink is present, so `Self: CliGlobals` is satisfied — the only defect is that
// `list` declares a parameter named like the declared `verbose` global flag.
impl CliGlobals for MyApp {
    fn set_global_flag(&self, _name: &str, _value: bool) {}
}

// A declared `global = [...]` flag is delivered solely through the `CliGlobals` sink.
// A method parameter that shares its flag name no longer gets auto-filled from the
// global (the legacy path is gone); it would collide with the root `.global(true)` flag
// at clap-build time. The collision guard turns that into a spanned compile error
// instead of silently looking-like-it-receives-the-global.
#[cli(name = "my-app", global = [verbose])]
impl MyApp {
    fn list(&self, verbose: bool) {
        let _ = verbose;
    }
}

fn main() {}
