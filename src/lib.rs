use chrono::{DateTime, Duration, Utc};
use dotenv::dotenv;
// use github_flows::{
//     listen_to_event,
//     octocrab::models::events::payload::{IssuesEventAction, PullRequestEventAction},
//     EventPayload,
//     GithubLogin::Default,
// };
use http_req::{
    request::{Method, Request},
    uri::Uri,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use slack_flows::{listen_to_channel, send_message_to_channel, SlackMessage};
use std::env;

// #[no_mangle]
// #[tokio::main(flavor = "current_thread")]
// pub async fn run() {
//     dotenv().ok();

//     let github_owner = env::var("github_owner").unwrap_or("WasmEdge".to_string());
//     let github_repo = env::var("github_repo").unwrap_or("WasmEdge".to_string());

//     listen_to_event(
//         &Default,
//         &github_owner,
//         &github_repo,
//         vec!["issues", "pull_request"],
//         |payload| handler(&github_owner, payload),
//     )
//     .await;
// }

// async fn handler(owner: &str, payload: EventPayload) {
#[no_mangle]
pub fn run() {
    dotenv().ok();

    let slack_workspace = env::var("slack_workspace").unwrap_or("secondstate".to_string());
    let slack_channel = env::var("slack_channel").unwrap_or("github-status".to_string());

    listen_to_channel(&slack_workspace, &slack_channel, |sm| {
        handler(&slack_workspace, &slack_channel, sm);
    });
}

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
async fn handler(
    worksapce: &str,
    channel: &str,
    sm: SlackMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    let trigger_word = env::var("trigger_word").unwrap_or("diss".to_string());
    let token = env::var("github_token").unwrap_or("secondstate".to_string());
    let owner = env::var("owner").unwrap_or("alabulei1".to_string());
    // let slack_workspace = env::var("slack_workspace").unwrap_or("secondstate".to_string());
    // let slack_channel = env::var("slack_channel").unwrap_or("github-status".to_string());
    let slack_workspace = worksapce;
    let slack_channel = channel;
    // let mut is_triggered = false;
    // let mut is_valid_event = true;

    // match payload {
    //     EventPayload::IssuesEvent(e) => {
    //         is_valid_event = e.action != IssuesEventAction::Closed;
    //         is_triggered = true;
    //     }

    //     EventPayload::PullRequestEvent(e) => {
    //         is_valid_event = e.action != PullRequestEventAction::Closed;
    //         is_triggered = true;
    //     }

    //     _ => (),
    // }

    // if is_valid_event && is_triggered {
    let n_days_ago = Utc::now().checked_sub_signed(Duration::days(30)).unwrap();

    let query = serde_json::json!({
        "query": format!(
            "query($login: String!) {{
                user(login: $login) {{
                    repositories(first: 100, orderBy: {{field: UPDATED_AT, direction: DESC}}) {{
                        edges {{
                            node {{
                                name,
                                discussions(first: 100, orderBy: {{field: UPDATED_AT, direction: DESC}}) {{
                                    edges {{
                                        node {{
                                            id,
                                            title,
                                            url,
                                            comments {{
                                                totalCount
                                            }},
                                            createdAt
                                        }}
                                    }}
                                }}
                            }}
                        }}
                    }}
                }}
            }}"),
        "variables": {
            "login": owner
        }
    });

    let mut buffer = Vec::new();

    let raw_url = "https://api.github.com/graphql";
    let gql_api_url = Uri::try_from(raw_url).unwrap();

    let bearer_token = format!("Bearer {}", token);
    let _response = Request::new(&gql_api_url)
        .method(Method::POST)
        .header("Authorization", &bearer_token)
        .header("Content-Type", "application/json")
        .header("User-Agent", "Flows Network Connector")
        .header("Content-Length", &query.to_string().len())
        .body(&query.to_string().into_bytes())
        .send(&mut buffer)?;

    let response_str = String::from_utf8_lossy(&buffer).to_string();
    send_message_to_channel(&slack_workspace, &slack_channel, response_str);

    let response: Response = serde_json::from_slice(&buffer)?;

    let repo_edges = response.data.user.repositories;
    for repo_edge in repo_edges.edges {
        let node = &repo_edge.node;
        for discussion_edge in &node.discussions.edges {
            let discussion_node = &discussion_edge.node;
            let comments = &discussion_node.comments;
            let mut in_time_range = false;
            match DateTime::parse_from_rfc3339(&discussion_node.createdAt) {
                Ok(dt) => in_time_range = dt > n_days_ago,
                Err(_e) => continue,
            };
            if in_time_range && comments.totalCount == 0 {
                let name = &node.name;
                let title = &discussion_node.title;
                let html_url = &discussion_node.url;

                let text = format!("{} started discussion {}\n{}", name, title, html_url);
                send_message_to_channel(&slack_workspace, &slack_channel, text);
            }
        }
    }

    // }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct Response {
    data: Data,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    user: User,
}
#[derive(Debug, Deserialize)]
struct User {
    repositories: Repositories,
}

#[derive(Debug, Deserialize)]
struct Repositories {
    edges: Vec<RepoEdge>,
}

#[derive(Debug, Deserialize)]
struct Comment {
    totalCount: usize,
}

#[derive(Debug, Deserialize)]
struct DiscussionNode {
    id: String,
    title: String,
    url: String,
    comments: Comment,
    createdAt: String,
}

#[derive(Debug, Deserialize)]
struct DiscussionEdge {
    node: DiscussionNode,
}

#[derive(Debug, Deserialize)]
struct Discussions {
    edges: Vec<DiscussionEdge>,
}

#[derive(Debug, Deserialize)]
struct RepoNode {
    name: String,
    discussions: Discussions,
}

#[derive(Debug, Deserialize)]
struct RepoEdge {
    node: RepoNode,
}
