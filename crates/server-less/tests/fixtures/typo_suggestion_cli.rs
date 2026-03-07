use server_less::cli;

#[derive(Clone)]
struct MyService;

// #[cli(naem = "...")] is a typo for name — should suggest "name"
#[cli(naem = "my-tool")]
impl MyService {
    pub fn hello(&self) -> String {
        "hello".into()
    }
}

fn main() {}
