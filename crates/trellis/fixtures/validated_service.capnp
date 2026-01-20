@0xabcd1234abcd1234;

interface ValidatedCapnpService {
  # Get greeting
  getGreeting @0 (GetGreetingParams) -> (GetGreetingResult);
  # Create item
  createItem @1 (CreateItemParams) -> (CreateItemResult);
}

struct GetGreetingParams {
}

struct GetGreetingResult {
  value @0 :Text;
}

struct CreateItemParams {
  name @0 :Text;
}

struct CreateItemResult {
  value @0 :Text;
}
