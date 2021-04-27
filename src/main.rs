use eyre::Result;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::Deserialize;
use sonor::Speaker;
use std::collections::HashMap;
use std::fs;
use std::str;
use std::time::Duration;

#[tokio::main]
pub async fn main() -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string("config.toml")?)?;
    let mut mqttoptions = MqttOptions::new("rumqtt-async", "homeassistant.local", 1883);
    mqttoptions.set_keep_alive(5);
    mqttoptions.set_clean_session(false);
    mqttoptions.set_credentials(config.username.clone(), config.password.clone());
    mqttoptions.set_connection_timeout(30);
    mqttoptions.set_max_packet_size(65535, 65535);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    /* client
    .subscribe("homeassistant/+/+/+/config", QoS::AtMostOnce)
    .await?;*/

    let mut app: App = App {
        bindings: HashMap::new(),
        client: client,
    };

    app.connect_bindings(config).await?;

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
                    app.handle_payload(&packet.topic, str::from_utf8(&packet.payload).unwrap())
                        .await?;
                }
            }
            Err(e) => {
                println!("{:?}", e);
            }
        };
    }
}

#[derive(Debug)]
struct App {
    bindings: HashMap<String, Vec<Binding>>,
    client: AsyncClient,
}

impl App {
    async fn connect_bindings(&mut self, config: Config) -> Result<()> {
        for bind in config.bind {
            self.do_bind(bind).await?;
        }
        Ok(())
    }

    async fn handle_payload(&mut self, topic: &str, payload: &str) -> Result<()> {
        println!("payload = {:?}, topic = {}", payload, topic);
        if let Some(bindings) = self.bindings.get(topic) {
            for binding in bindings {
                if payload == "brightness_move_up" {
                    binding.speaker.set_volume_relative(1).await?;
                } else if payload == "brightness_move_down" {
                    binding.speaker.set_volume_relative(-1).await?;
                } else if payload == "brightness_stop" {
                }
            }
        }
        Ok(())
    }

    async fn do_bind(&mut self, bind: BindConfig) -> Result<()> {
        println!("topic = {}", bind.action_topic);

        self.client
            .subscribe(&bind.action_topic, QoS::AtMostOnce)
            .await?;

        let speaker = sonor::find(&bind.speaker, Duration::from_secs(2))
            .await?
            .expect("room exists");

        println!("The volume is currently at {}", speaker.volume().await?);

        match speaker.track().await? {
            Some(track_info) => println!("Currently playing '{}", track_info.track()),
            None => println!("No track currently playing"),
        }

        let binding = Binding {
            speaker: speaker,
            action_topic: bind.action_topic.clone(),
        };
        if let Some(vec) = self.bindings.get_mut(&bind.action_topic) {
            vec.push(binding);
        } else {
            self.bindings
                .insert(binding.action_topic.clone(), vec![binding]);
        }

        Ok(())
    }
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

#[derive(Debug)]
struct Binding {
    action_topic: String,
    speaker: Speaker,
}
