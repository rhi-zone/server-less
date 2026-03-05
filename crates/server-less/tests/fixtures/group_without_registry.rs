use server_less::cli;

#[derive(Clone)]
struct MyService;

#[cli]
impl MyService {
    #[server(group = "admin")]
    pub fn reset(&self) -> String {
        "reset".to_string()
    }
}

fn main() {}
