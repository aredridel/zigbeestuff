use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::Deserialize;
use std::time::Duration;
use eyre::Result;
use std::fs;

#[tokio::main]
pub async fn main() -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string("config.toml")?)?;
    let mut mqttoptions = MqttOptions::new("rumqtt-async", "homeassistant.local", 1883);
    mqttoptions.set_keep_alive(5);
    mqttoptions.set_clean_session(true);
    mqttoptions.set_credentials(config.username, config.password);
    mqttoptions.set_connection_timeout(30);
    mqttoptions.set_max_packet_size(65535, 65535);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    client
        .subscribe("homeassistant/+/+/+/config", QoS::AtMostOnce)
        .await?;

    for bind in config.bind {
        do_bind(&bind, &client).await?;
    }

    /*
    task::spawn(async move {
        for i in 0..10 {
            client
                .publish("hello/rumqtt", QoS::AtLeastOnce, false, vec![i; i as usize])
                .await
                .unwrap();
            time::sleep(Duration::from_millis(100)).await;
        }
    });
    */

    loop {
        match eventloop.poll().await {
            Ok(notification) => {
                println!("Received = {:?}", notification);
                if let Event::Incoming(Incoming::Publish(packet)) = notification {
                    println!("topic = {:?}", packet.topic);
                    let v: serde_json::Result<HAConfig> = serde_json::from_slice(&packet.payload);
                    match v {
                        Ok(payload) => println!("payload = {:?}", payload),
                        Err(_) => println!("payload = {:?}", packet.payload),
                    }
                }
            }
            Err(e) => {
                println!("{:?}", e);
            }
        };
    }
}

async fn do_bind(bind: &BindConfig, client: &AsyncClient) -> Result<()> {
    client
        .subscribe(&bind.action_topic, QoS::AtMostOnce)
        .await?;
    let speaker = sonor::find(&bind.speaker, Duration::from_secs(2))
        .await?
        .expect("room exists");

    println!("The volume is currently at {}", speaker.volume().await?);

    match speaker.track().await? {
        Some(track_info) => println!("- Currently playing '{}", track_info.track()),
        None => println!("- No track currently playing"),
    }

    Ok(())
}

#[derive(Deserialize, Debug)]
struct HAConfig {
    availability: Vec<TopicObj>,
    command_topic: String,
    device: Device,
    json_attributes_topic: String,
    name: String,
    /*
    position_template: String,
    position_topic: String,
    set_position_template: String,
    set_position_topic: String,
    */
    unique_id: String,
}

#[derive(Deserialize, Debug)]
struct Device {
    identifiers: Vec<String>,
    manufacturer: String,
    model: String,
    name: String,
    sw_version: String,
}

#[derive(Deserialize, Debug)]
struct TopicObj {
    topic: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    username: String,
    password: String,
    bind: Vec<BindConfig>,
}
#[derive(Deserialize, Debug)]
struct BindConfig {
    action_topic: String,
    speaker: String,
}
