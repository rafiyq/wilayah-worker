use worker::*;

pub fn check_auth(req: &Request, env: &Env) -> Result<()> {
    let token = env.secret("ADMIN_TOKEN")?.to_string();
    let auth = req.headers().get("Authorization")?.unwrap_or_default();
    if auth == format!("Bearer {token}") {
        Ok(())
    } else {
        Err(Error::from("Unauthorized"))
    }
}
