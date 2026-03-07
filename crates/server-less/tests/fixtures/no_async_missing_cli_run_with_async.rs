use server_less::cli;

#[derive(Clone)]
struct MySvc;

#[cli(name = "my-svc", no_async)]
impl MySvc {
    pub fn run(&self) -> String {
        "ran".to_string()
    }
}

async fn main_async() {
    let svc = MySvc;
    // cli_run_with_async should not exist on a no_async service
    svc.cli_run_with_async(["my-svc", "run"]).await.unwrap();
}

fn main() {}
