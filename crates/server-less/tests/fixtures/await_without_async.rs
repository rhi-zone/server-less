use server_less::cli;

struct App;

async fn something() -> String {
    "hello".into()
}

#[cli]
impl App {
    // Sync method that awaits — should be a server-less framed error, not E0728.
    fn f(&self) {
        let _ = something().await;
    }
}

fn main() {}
