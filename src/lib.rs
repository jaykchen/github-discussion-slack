use chrono::{Duration, Utc};
use dotenv::dotenv;
use github_flows::octocrab::models::GraphQLResponse;
use github_flows::{
    get_octo, listen_to_event,
    octocrab::models::events::payload::{IssuesEventAction, PullRequestEventAction},
    EventPayload,
    GithubLogin::Default,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use slack_flows::send_message_to_channel;
use std::env;
use octocrab_wasi::Octocrab;
use octocrab_wasi::graphql::GraphQLResponse; 

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    dotenv().ok();

    let github_owner = env::var("github_owner").unwrap_or("WasmEdge".to_string());
    let github_repo = env::var("github_repo").unwrap_or("WasmEdge".to_string());

    listen_to_event(
        &Default,
        &github_owner,
        &github_repo,
        vec!["issues", "pull_request"],
        |payload| handler(&github_owner, payload),
    )
    .await;
}

async fn handler(owner: &str, payload: EventPayload) {
    let slack_workspace = env::var("slack_workspace").unwrap_or("secondstate".to_string());
    let slack_channel = env::var("slack_channel").unwrap_or("github-status".to_string());
    let mut is_triggered = false;
    let mut is_valid_event = true;

    match payload {
        EventPayload::IssuesEvent(e) => {
            is_valid_event = e.action != IssuesEventAction::Closed;
            is_triggered = true;
        }

        EventPayload::PullRequestEvent(e) => {
            is_valid_event = e.action != PullRequestEventAction::Closed;
            is_triggered = true;
        }

        _ => (),
    }
    let mut new_discussions = Vec::new();

    if is_valid_event && is_triggered {
        let octocrab = get_octo(&Default);
        let n_days_ago = Utc::now().checked_sub_signed(Duration::days(1)).unwrap();

        let query = r#"
        {
            user(login: "USER_NAME") {
                repositories(first: 100, orderBy: {field: UPDATED_AT, direction: DESC}) {
                    edges {
                        node {
                            name
                            discussions(first: 100, orderBy: {field: CREATED_AT, direction: DESC}) {
                                edges {
                                    node {
                                        id
                                        title
                                        url
                                        comments {
                                            totalCount
                                        }
                                        createdAt
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        "#;

        let query = query.replace("USER_NAME", owner);

        let response: Result<GraphQLResponse, _> = octocrab.graphql(&query).await;
        let response: Result<GraphQLResponse, _> = octocrab.graphql(&query).await;

        match response {
            Ok(response) => {
                for repo_edge in response.user.repositories.edges {
                    for discussion_edge in repo_edge.node.discussions.edges {
                        if discussion_edge.node.comments.totalCount == 0 {
                            let name = repo_edge.node.name;
                            let title = discussion_edge.node.title;
                            let html_url = discussion_edge.node.url;

                            let text =
                                format!("{} started discussion {}\n{}", name, title, html_url);

                            send_message_to_channel(&slack_workspace, &slack_channel, text);
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("Error querying GitHub GraphQL API: {}", error);
            }
        }
    }

    while !new_discussions.is_empty() {
        let discussion = new_discussions.pop().unwrap();

        let name = discussion.name;
        let title = discussion.title;
        let html_url = discussion.html_url;
        let discussion_login = discussion.login;

        let text = format!("{name} started dicussion {title}\n{html_url}");

        send_message_to_channel(&slack_workspace, &slack_channel, text);
    }
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

#[derive(Debug, Deserialize)]
struct Repositories {
    edges: Vec<RepoEdge>,
}

#[derive(Debug, Deserialize)]
struct User {
    repositories: Repositories,
}
