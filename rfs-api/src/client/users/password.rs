use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::users::password::CreatePassword;

pub struct UpdatePassword {
    body: CreatePassword
}

impl UpdatePassword {
    pub fn update_password<P>(current: P, updated: P, confirm: P) -> Self
    where
        P: Into<String>
    {
        UpdatePassword {
            body: CreatePassword {
                current: current.into(),
                updated: updated.into(),
                confirm: confirm.into(),
            }
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.validate()?;

        let url = client.info.url.join("/api/user/password").unwrap();
        let res = client.client.post(url)
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
