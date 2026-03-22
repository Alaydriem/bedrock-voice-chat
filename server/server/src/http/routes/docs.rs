use rocket::response::content::RawHtml;
use rocket_okapi::openapi;

#[openapi(skip)]
#[get("/")]
pub fn scalar_ui() -> RawHtml<&'static str> {
    RawHtml(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Bedrock Voice Chat API Docs</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
    <script id="api-reference" data-url="/openapi.json"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#,
    )
}
