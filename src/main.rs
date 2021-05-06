mod speaker_actor;

use eyre::Result;
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS};
use serde::Deserialize;
use std::collections::HashMap;
use std::{fs, str};
use tokio::time::Duration;

use crate::speaker_actor::{SpeakerHandle, SpeakerMessage};

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut app: App = App::new().await?;
    app.run().await?;
    Ok(())
}

struct App {
    bindings: HashMap<String, Vec<Binding>>,
    client: AsyncClient,
    eventloop: EventLoop,
}

impl App {
    async fn new() -> Result<App> {
        let config: Config = toml::from_str(&fs::read_to_string("config.toml")?)?;
        let mut mqttoptions =
            MqttOptions::new("rumqtt-async", &config.mqtt.hostname, config.mqtt.port);
        mqttoptions.set_keep_alive(5);
        mqttoptions.set_clean_session(false);
        mqttoptions.set_credentials(&config.mqtt.username, &config.mqtt.password);
        mqttoptions.set_connection_timeout(30);
        mqttoptions.set_max_packet_size(65535, 65535);

        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
        /* client
        .subscribe("homeassistant/+/+/+/config", QoS::AtMostOnce)
        .await?;*/

        let mut app = App {
            bindings: HashMap::new(),
            client: client,
            eventloop: eventloop,
        };
        app.connect_bindings(config).await?;
        return Ok(app);
    }

    async fn run(&mut self) -> Result<()> {
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
            match self.eventloop.poll().await {
                Ok(notification) => {
                    println!("Received = {:?}", notification);
                    if let Event::Incoming(Incoming::Publish(packet)) = notification {
                        println!("topic = {:?}", packet.topic);
                        self.handle_payload(
                            &packet.topic,
                            str::from_utf8(&packet.payload).unwrap(),
                        )
                        .await?;
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            };
        }
    }

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
                    binding.speaker.send(SpeakerMessage::VolumeUp).await;
                } else if payload == "brightness_move_down" {
                    binding.speaker.send(SpeakerMessage::VolumeDown).await;
                } else if payload == "brightness_stop" {
                    binding.speaker.send(SpeakerMessage::VolumeStop).await;
                } else if payload == "toggle" {
                    binding.speaker.send(SpeakerMessage::PlayPause).await;
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
            speaker: SpeakerHandle::new(speaker),
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
    mqtt: MqttConfig,
    bind: Vec<BindConfig>,
}

#[derive(Deserialize, Debug)]
struct MqttConfig {
    username: String,
    password: String,
    hostname: String,
    port: u16,
}

#[derive(Deserialize, Debug)]
struct BindConfig {
    action_topic: String,
    speaker: String,
}

struct Binding {
    action_topic: String,
    speaker: SpeakerHandle,
}
