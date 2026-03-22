use bvc_server_lib::http::openapi::OpenApiSpec;

fn main() {
    let spec = OpenApiSpec::generate();
    let json = serde_json::to_string_pretty(&spec).expect("Failed to serialize OpenAPI spec");
    println!("{}", json);
}
