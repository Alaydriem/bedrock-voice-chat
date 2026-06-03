#[derive(Default)]
pub(crate) struct MessageVisitor {
    pub(crate) message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let formatted = format!("{:?}", value);
            self.message = if formatted.starts_with('"') && formatted.ends_with('"') && formatted.len() >= 2 {
                formatted[1..formatted.len() - 1].to_string()
            } else {
                formatted
            };
        }
    }
}
