use server_less::cli;

struct MyCli;

// Passing a number where a string is expected should produce a clear type error.
#[cli(name = 42)]
impl MyCli {
    pub fn hello(&self) -> String {
        "hello".into()
    }
}

fn main() {}
