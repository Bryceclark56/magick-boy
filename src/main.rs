// Created by Bryce Clark (bryce@lowquality.dev)
// Licensed under the MIT license
// (Copy my code if you dare)
// 
// Spaghetti code is the only true path forward

use std::{env, error::Error};

use futures::stream::StreamExt;

use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, Cluster, cluster::{ShardScheme}};
use twilight_http::Client as HttpClient;
use twilight_model::gateway::{Intents, payload::MessageCreate};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let token = env::var("DISCORD_TOKEN")?;

    // This is the default scheme. It will automatically create as many
    // shards as is suggested by Discord.
    let scheme = ShardScheme::Auto;

    // Use intents to only receive guild message events.
    let (cluster, mut events) = Cluster::builder(token.to_owned(), Intents::GUILD_MESSAGES)
        .shard_scheme(scheme)
        .build()
        .await?;

    // Start up the cluster.
    let cluster_spawn = cluster.clone();

    // Start all shards in the cluster in the background.
    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    // HTTP is separate from the gateway, so create a new client.
    let http = HttpClient::new(token);

    // Since we only care about new messages, make the cache only
    // cache new messages.
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    // Process each event as they come in
    while let Some((shard_id, event)) = events.next().await {
        // Update the cache with the event.
        cache.update(&event);

        tokio::spawn(handle_event(shard_id, event, http.clone()));
    }

    Ok(())
}

static CMD_PREFIX: &'static str = "&";

async fn handle_event(
    shard_id: u64,
    event: Event,
    http: HttpClient,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) => {
            if msg.content.starts_with(CMD_PREFIX) { handle_command(shard_id, msg, http).await? }
        }
        Event::ShardConnected(_) => {
            println!("Connected on shard {}", shard_id);
        }
        // Other events here...
        _ => {}
    }

    Ok(())
}

async fn handle_command(shard_id: u64, msg: Box<MessageCreate>, http: HttpClient) -> Result<(), Box<dyn Error + Send + Sync>> {

    let parsed = parse_command(&msg.content);

    match parsed.root.as_str() {
        "ping" => {
            http.create_message(msg.channel_id)
            .content("Pong!~")?
            .exec()
            .await?;
        }
        &_ => {}
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
struct UserCommand {
    root: String,
    arguments: Vec<String>
}

fn parse_command(content: &String) -> UserCommand {
    // Grab the tokens without the prefix
    let split: Vec<String> = 
        content[CMD_PREFIX.len()..]
            .split(' ')
            .map(String::from)
            .collect();

    UserCommand {
        root: split[0].clone(),
        arguments: split[1..].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use crate::{UserCommand, parse_command};

    #[test]
    fn test_command_parser() {
        let test_string = String::from("&test 1 2 what the fuck");

        let expected = UserCommand {
            root: String::from("test"),
            arguments: vec!["1", "2", "what", "the", "fuck"]
                            .iter()
                            .map(|s| s.to_owned().to_owned())
                            .collect()
        };

        assert_eq!(parse_command(&test_string), expected);
    }
}