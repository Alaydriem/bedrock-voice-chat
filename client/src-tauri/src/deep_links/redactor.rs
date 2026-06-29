pub struct UrlRedactor;

impl UrlRedactor {
    // Everything before the first query/fragment delimiter. Safe to log: query
    // and fragment can carry secrets (OAuth `access_token`, auth `code`).
    pub fn for_log(url: &str) -> &str {
        let end = url.find(['?', '#']).unwrap_or(url.len());
        &url[..end]
    }

    // Strips only the OAuth implicit-grant `access_token` from the query and
    // fragment before the URL is persisted to disk, preserving other params the
    // server-login deep link relies on (`code`, `state`). The bearer token must
    // never sit at rest in store.json.
    pub fn for_storage(url: &str) -> String {
        let (base_query, fragment) = match url.split_once('#') {
            Some((bq, f)) => (bq, Some(f)),
            None => (url, None),
        };
        let (base, query) = match base_query.split_once('?') {
            Some((b, q)) => (b, Some(q)),
            None => (base_query, None),
        };

        let mut out = base.to_string();
        if let Some(q) = query {
            let filtered = Self::drop_access_token(q);
            if !filtered.is_empty() {
                out.push('?');
                out.push_str(&filtered);
            }
        }
        if let Some(f) = fragment {
            let filtered = Self::drop_access_token(f);
            if !filtered.is_empty() {
                out.push('#');
                out.push_str(&filtered);
            }
        }
        out
    }

    fn drop_access_token(params: &str) -> String {
        params
            .split('&')
            .filter(|pair| pair.split('=').next().unwrap_or("") != "access_token")
            .collect::<Vec<_>>()
            .join("&")
    }
}
