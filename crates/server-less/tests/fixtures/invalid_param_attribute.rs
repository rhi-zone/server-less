use server_less::cli;

#[derive(Clone)]
struct MyService;

// #[param(nonexistent_param_attr)] on a function parameter should error with "unknown attribute".
#[cli]
impl MyService {
    pub fn hello(&self, #[param(nonexistent_param_attr)] name: String) -> String {
        format!("hello {name}")
    }
}

fn main() {}
