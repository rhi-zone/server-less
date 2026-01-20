namespace rs validated

service ValidatedThriftService {
  // Get greeting
  string get_greeting(GetGreetingArgs args) = 1;
  // Create item
  string create_item(CreateItemArgs args) = 2;
}

struct GetGreetingArgs {
}

struct CreateItemArgs {
  1: string name;
}
