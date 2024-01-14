use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::auth::password::CreatePassword;

pub struct UpdatePassword {
    body: CreatePassword
}

impl UpdatePassword {
    pub fn update_password<P>(updated: P, confirm: P) -> Self
    where
        P: Into<String>
    {
        UpdatePassword {
            body: CreatePassword {
                current: None,
                updated: updated.into(),
                confirm: confirm.into(),
            }
        }
    }

    pub fn current<P>(&mut self, current: P) -> &mut Self
    where
        P: Into<String>
    {
        self.body.current = Some(current.into());

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.validate()?;

        let url = client.info.url.join("/auth/password").unwrap();
        let res = client.client.post(url)
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
