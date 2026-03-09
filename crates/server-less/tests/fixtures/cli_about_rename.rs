use server_less::cli;

#[derive(Clone)]
struct MyService;

// #[cli(about = "...")] was renamed to description in 0.4.0
#[cli(about = "my tool")]
impl MyService {
    pub fn hello(&self) -> String {
        "hello".into()
    }
}

fn main() {}
