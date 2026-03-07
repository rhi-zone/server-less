use server_less::cli;

#[derive(Clone)]
struct MyService;

// #[param(hlep)] is a typo for help — should suggest "help"
#[cli]
impl MyService {
    pub fn hello(&self, #[param(hlep = "A greeting")] name: String) -> String {
        format!("hello {name}")
    }
}

fn main() {}
