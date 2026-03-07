use server_less::cli;

#[derive(Clone)]
struct MySvc;

#[cli(name = "my-svc", no_sync)]
impl MySvc {
    pub async fn run(&self) -> String {
        "ran".to_string()
    }
}

fn main() {
    let svc = MySvc;
    // cli_run_with should not exist on a no_sync service
    svc.cli_run_with(["my-svc", "run"]).unwrap();
}
