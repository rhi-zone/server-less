use server_less::cli;

#[derive(Clone)]
struct MyService;

// #[cli(nonexistent_attr = "val")] should error with "unknown argument"
#[cli(nonexistent_attr = "val")]
impl MyService {
    pub fn hello(&self) -> String {
        "hello".into()
    }
}

fn main() {}
