use anyhow::anyhow;
use std::env;
mod mcauth;

pub async fn auth() -> Result<(String, String), anyhow::Error> {
    let client_id = match env::var("MCAUTH_CLIENT_ID") {
        Ok(v) => v,
        Err(e) => return Err(anyhow!("Missing client_id environment variable"))
    };

    let mut auth = crate::auth::mcauth::AuthFlow::new(&client_id);
    match auth.request_code().await{
        Ok(code_res) => {
            println!(
                "Open this link in your browser {} and enter the following code: {}\nWaiting authentication...",
                code_res.verification_uri, code_res.user_code
            );
            auth.wait_for_login().await;
        
            println!("Logging in xbox live services...");
            auth.login_in_xbox_live().await;
        
            println!("Logging in minecraft services...");
            match auth.login_in_minecraft().await {
                Ok(minecraft) => {
                    dbg!(&minecraft);
                    return Ok(("a".to_string(), "b".to_string()));
                },
                Err(e) => return Err(anyhow!(e.to_string()))
            }      
        },
        Err(e) => return Err(anyhow!(e.to_string()))
    }    
}