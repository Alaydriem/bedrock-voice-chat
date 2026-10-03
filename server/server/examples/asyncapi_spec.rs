use bvc_server_lib::http::asyncapi::AsyncApiSpec;

fn main() {
    let spec = AsyncApiSpec::generate();
    let json = serde_json::to_string_pretty(&spec).expect("Failed to serialize AsyncAPI spec");
    println!("{}", json);
}
