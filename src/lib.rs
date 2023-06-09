use chrono::{DateTime, Duration,  Utc};
use dotenv::dotenv;
use http_req::{
    request::{Method, Request},
    uri::Uri,
};
use schedule_flows::schedule_cron_job;
use serde::Deserialize;
use serde_json;
use slack_flows::send_message_to_channel;
use std::env;

#[no_mangle]
pub fn run() {
    dotenv().ok();
    //time_to_invoke is a string of 3 numbers separated by spaces, representing minute, hour, and day
    //* is the spaceholder for non-specified numbers
    let mut time_to_invoke = env::var("time_to_invoke").unwrap_or("35 12 *".to_string());
    time_to_invoke.push_str(" * *");

    schedule_cron_job(time_to_invoke, String::from("cron_job_evoked"), |payload| {
        handler(payload);
    });
}

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
async fn handler(_payload: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("github_token").unwrap_or("some_random_digits".to_string());
    let owner = env::var("owner").unwrap_or("alabulei1".to_string());

    let slack_workspace = env::var("slack_workspace").unwrap_or("secondstate".to_string());
    let slack_channel = env::var("slack_channel").unwrap_or("github-status".to_string());

    let n_days = env::var("n_days").unwrap_or("1".to_string());
    let n_days_ago = Utc::now()
        .checked_sub_signed(Duration::days(n_days.parse::<i64>().unwrap_or(1)))
        .unwrap_or(Utc::now());

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

    let gql_api_url = Uri::try_from("https://api.github.com/graphql").unwrap();

    let _ = Request::new(&gql_api_url)
        .method(Method::POST)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("User-Agent", "Flows Network Connector")
        .header("Content-Length", &query.to_string().len())
        .body(&query.to_string().into_bytes())
        .send(&mut buffer)?;

    let response: Response = serde_json::from_slice(&buffer)?;

    let repo_edges = response.data.user.repositories;
    for repo_edge in repo_edges.edges {
        let node = &repo_edge.node;
        for discussion_edge in &node.discussions.edges {
            let discussion_node = &discussion_edge.node;
            let comments = &discussion_node.comments;
            let mut in_date_range = false;
            match DateTime::parse_from_rfc3339(&discussion_node.createdAt) {
                Ok(dt) => {
                    let dt_utc = dt.with_timezone(&Utc);
                    let dt_utc_no_frac = dt_utc.date_naive();
                    let n_days_ago_no_frac = n_days_ago.date_naive();
                    in_date_range = dt_utc_no_frac >= n_days_ago_no_frac;
                }
                Err(_e) => continue,
            };
            if in_date_range && comments.totalCount == 0 {
                let title = &discussion_node.title;
                let html_url = &discussion_node.url;

                let text = format!("New discussion: {title}\n{html_url}");
                send_message_to_channel(&slack_workspace, &slack_channel, text);
            }
        }
    }
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
