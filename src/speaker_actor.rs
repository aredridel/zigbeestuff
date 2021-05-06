use eyre::Result;
use sonor::Speaker;
use std::thread;
use tokio::sync::mpsc;
use tokio::time::Duration;

pub struct SpeakerActor {
    receiver: mpsc::UnboundedReceiver<SpeakerMessage>,
    sender: mpsc::UnboundedSender<SpeakerMessage>,
    speaker: Speaker,
    remaining_steps: i32,
    direction: i16,
}

pub enum SpeakerMessage {
    VolumeUp,
    VolumeDown,
    VolumeStop,
    Play,
    Pause,
    PlayPause,
    Tick,
}

impl SpeakerActor {
    fn new(
        receiver: mpsc::UnboundedReceiver<SpeakerMessage>,
        sender: mpsc::UnboundedSender<SpeakerMessage>,
        speaker: Speaker,
    ) -> Self {
        SpeakerActor {
            receiver,
            sender,
            speaker,
            remaining_steps: 0,
            direction: 1,
        }
    }

    async fn move_volume(&mut self, direction: i16) -> Result<()> {
        let running = self.remaining_steps > 0;
        self.remaining_steps = 5;
        self.direction = direction;
        if !running {
            std::thread::spawn(|| {
                while self.remaining_steps > 0 {
                    self.remaining_steps = self.remaining_steps - 1;
                    thread::sleep(Duration::from_millis(250));
                    //interval.tick().await;
                    //self.speaker.set_volume_relative(self.direction).await?;
                    println!("Volume +{:?}", self.direction);
                }
            });
        }
        Ok(())
    }

    async fn handle_message(&mut self, msg: SpeakerMessage) -> Result<()> {
        match msg {
            SpeakerMessage::VolumeUp => {
                self.move_volume(1).await?;
                Ok(())
            }
            SpeakerMessage::VolumeDown => {
                self.move_volume(-1).await?;
                Ok(())
            }
            SpeakerMessage::VolumeStop => {
                self.remaining_steps = 0;
                Ok(())
            }
            SpeakerMessage::Play => Ok(()),
            SpeakerMessage::Pause => Ok(()),
            SpeakerMessage::PlayPause => Ok(()),
            SpeakerMessage::Tick => Ok(()),
        }
    }

    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            let task = self.handle_message(msg);
            /*
                        tokio::spawn(async move {
            println!("{:?}", err);
                        })
                        */
        }
    }
}

#[derive(Clone)]
pub struct SpeakerHandle {
    sender: mpsc::UnboundedSender<SpeakerMessage>,
}

impl SpeakerHandle {
    pub fn new(speaker: Speaker) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let self_sender = sender.clone();
        tokio::spawn(async move {
            let mut actor = SpeakerActor::new(receiver, self_sender, speaker);
            actor.run().await;
        });

        Self { sender }
    }

    pub async fn send(&self, msg: SpeakerMessage) {
        println!("before");
        let _ = self.sender.send(msg);
        println!("after");
    }
}
