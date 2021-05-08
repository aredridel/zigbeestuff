use crate::ticker_actor::{TickMessage, TickerHandle};
use eyre::Result;
use sonor::Speaker;
use tokio::sync::mpsc;

pub struct SpeakerActor {
    receiver: mpsc::UnboundedReceiver<SpeakerMessage>,
    speaker: Speaker,
    ticker: TickerHandle,
    direction: i16,
}

#[derive(Debug)]
pub enum SpeakerMessage {
    VolumeUp,
    VolumeDown,
    VolumeStop,
    /*   Play,
    Pause,*/
    Next,
    Previous,
    PlayPause,
    Tick,
}

impl SpeakerActor {
    fn new(
        receiver: mpsc::UnboundedReceiver<SpeakerMessage>,
        handle: SpeakerHandle,
        speaker: Speaker,
    ) -> Self {
        SpeakerActor {
            receiver,
            speaker,
            ticker: TickerHandle::new(handle),
            direction: 1,
        }
    }

    async fn move_volume(&mut self, direction: i16) -> Result<()> {
        self.direction = direction;
        self.ticker.send(TickMessage::Tick(5)).await;
        Ok(())
    }

    async fn volume_stop(&mut self) -> Result<()> {
        self.ticker.send(TickMessage::Stop).await;
        Ok(())
    }

    async fn handle_message(&mut self, msg: SpeakerMessage) -> Result<()> {
        match msg {
            SpeakerMessage::VolumeUp => {
                self.move_volume(1).await?;
            }
            SpeakerMessage::VolumeDown => {
                self.move_volume(-1).await?;
            }
            SpeakerMessage::VolumeStop => {
                self.volume_stop().await?;
            }
            /*        SpeakerMessage::Play => Ok(()),
            SpeakerMessage::Pause => Ok(()),*/
            SpeakerMessage::PlayPause => {
                if self.speaker.is_playing().await? {
                    self.speaker.pause().await?;
                } else {
                    self.speaker.play().await?;
                }
            }
            SpeakerMessage::Next => {
                self.speaker.next().await?;
            }
            SpeakerMessage::Previous => {
                self.speaker.previous().await?;
            }
            SpeakerMessage::Tick => {
                self.speaker.set_volume_relative(self.direction).await?;
            }
        }
        Ok(())
    }

    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await.unwrap_or_else(|err| {
                println!("{:?}", err);
            });
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
        let handle = Self { sender };
        let tick_handle = handle.clone();

        tokio::spawn(async move {
            let mut actor = SpeakerActor::new(receiver, tick_handle, speaker);
            actor.run().await;
        });

        handle
    }

    pub async fn send(&self, msg: SpeakerMessage) {
        self.sender.send(msg).unwrap();
    }
}
