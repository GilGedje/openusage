//! LiteLLM's device-style SSO login (`/sso/cli/*`): start, send the user to the browser, poll.
//! See docs/litellm-api.md.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct Start {
    pub login_id: String,
    pub poll_secret: String,
    pub user_code: String,
    pub expires_in: u64,
    /// Only present when the proxy sets `allow_cli_sso_verification_uri_complete`.
    pub verification_uri_complete: Option<String>,
}

impl Start {
    /// The page the user opens to sign in.
    pub fn verification_url(&self, base_url: &str) -> String {
        self.verification_uri_complete
            .clone()
            .unwrap_or_else(|| format!("{base_url}/sso/key/generate?source=litellm-cli&key={}", self.login_id))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Team {
    pub team_id: String,
    pub team_alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Poll {
    Pending,
    /// The user belongs to several teams; poll again with one of these team IDs.
    SelectTeam(Vec<Team>),
    Ready { token: String, user_id: String, team_id: Option<String> },
}

#[derive(Deserialize)]
struct PollBody {
    status: String,
    key: Option<String>,
    user_id: Option<String>,
    team_id: Option<String>,
    #[serde(default)]
    requires_team_selection: bool,
    team_details: Option<Vec<Team>>,
    #[serde(default)]
    teams: Vec<String>,
}

pub fn start(client: &Client) -> Result<Start> {
    client.post_json("/sso/cli/start", &serde_json::json!({}))
}

pub fn poll(client: &Client, start: &Start, team_id: Option<&str>) -> Result<Poll> {
    let path = format!("/sso/cli/poll/{}", start.login_id);
    let query: Vec<(&str, &str)> = team_id.map(|t| vec![("team_id", t)]).unwrap_or_default();
    let body: PollBody =
        client.get_with_headers(&path, &query, &[("x-litellm-cli-poll-secret", &start.poll_secret)])?;
    parse_poll(body)
}

fn parse_poll(body: PollBody) -> Result<Poll> {
    if body.status == "pending" {
        return Ok(Poll::Pending);
    }
    if body.status != "ready" {
        return Err(Error::Parse(format!("unknown login status `{}`", body.status)));
    }
    if body.requires_team_selection {
        let teams = body.team_details.unwrap_or_else(|| {
            body.teams.into_iter().map(|team_id| Team { team_id, team_alias: None }).collect()
        });
        return Ok(Poll::SelectTeam(teams));
    }
    match (body.key, body.user_id) {
        (Some(token), Some(user_id)) => Ok(Poll::Ready { token, user_id, team_id: body.team_id }),
        _ => Err(Error::Parse("login finished without a token".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Result<Poll> {
        parse_poll(serde_json::from_str(json).unwrap())
    }

    #[test]
    fn parses_poll_states() {
        assert_eq!(parse(r#"{"status":"pending"}"#).unwrap(), Poll::Pending);
        assert_eq!(
            parse(r#"{"status":"ready","key":"tok","user_id":"u","team_id":null,"teams":[],"team_details":[]}"#)
                .unwrap(),
            Poll::Ready { token: "tok".into(), user_id: "u".into(), team_id: None }
        );
        let teams = parse(
            r#"{"status":"ready","user_id":"u","teams":["a","b"],"requires_team_selection":true,
                "team_details":[{"team_id":"a","team_alias":"Core"},{"team_id":"b","team_alias":null}]}"#,
        )
        .unwrap();
        assert_eq!(
            teams,
            Poll::SelectTeam(vec![
                Team { team_id: "a".into(), team_alias: Some("Core".into()) },
                Team { team_id: "b".into(), team_alias: None },
            ])
        );
        assert!(parse(r#"{"status":"ready"}"#).is_err());
    }
}
