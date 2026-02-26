use server_less::http;

#[derive(Clone)]
struct MyService;

#[http]
impl MyService {
    pub fn get_user(&self, id: u32) -> String {
        format!("user {}", id)
    }

    pub fn fetch_user(&self, user_id: u32) -> String {
        format!("user {}", user_id)
    }
}

fn main() {}
