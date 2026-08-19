/// Renders an error together with everything underneath it.
///
/// `reqwest::Error` displays as "error sending request for url (...)" and puts the
/// reason — a TLS handshake failure, a refused connection, a DNS miss — in its source.
/// Reporting only the top line turns three distinct failures into one message that
/// names none of them.
pub struct ErrorChain;

impl ErrorChain {
    pub fn of(error: &dyn std::error::Error) -> String {
        let mut rendered = error.to_string();
        let mut source = error.source();

        while let Some(cause) = source {
            let text = cause.to_string();
            // Wrappers frequently restate their source verbatim, and repeating it adds
            // length without adding information.
            if !rendered.contains(&text) {
                rendered.push_str(": ");
                rendered.push_str(&text);
            }
            source = cause.source();
        }

        rendered
    }
}
