use dotenv::dotenv;
use github_flows::{listen_to_event, EventPayload, GithubLogin::Default};
use slack_flows::send_message_to_channel;
use std::env;

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
        vec!["discussion", "discussion_comment"],
        handler,
    )
    .await;
}

async fn handler(payload: EventPayload) {
    let slack_workspace = env::var("slack_workspace").unwrap_or("secondstate".to_string());
    let slack_channel = env::var("slack_channel").unwrap_or("github-status".to_string());

    if let EventPayload::UnknownEvent(e) = payload {
        let discussion = e.get("discussion").unwrap();

        let name = discussion["user"]["login"].as_str().unwrap();
        let title = discussion["title"].as_str().unwrap();
        send_message_to_channel(&slack_workspace, &slack_channel, title.to_string());

        let html_url = discussion["html_url"].as_str().unwrap();
        let comments_count = match discussion.get("comments") {
            Some(comments) => comments.as_i64().unwrap_or(0i64),
            None => 0i64,
        };

        let mut text = String::new();

        if comments_count > 0 {
            let name = discussion["comment"]["user"]["login"]
                .as_str()
                .unwrap_or("");
            let comment_body = discussion["comment"]["body"].as_str().unwrap_or("");
            text = format!("In discussion {title}\n{name} commented: {comment_body}\n{html_url}");
        } else {
            text = format!("{name} started dicussion {title}\n{html_url}");
        }
        send_message_to_channel(&slack_workspace, &slack_channel, text);
    }
}
