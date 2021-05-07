use tokio::time::{interval, Duration, Interval};
use tokio::select;
use tokio::sync::mpsc;
use crate::speaker_actor::{SpeakerHandle, SpeakerMessage};

#[derive(Debug)]
pub enum TickMessage {
    Tick(i32),
    Stop,
}

pub struct TickerActor {
    receiver: mpsc::UnboundedReceiver<TickMessage>,
    speaker: SpeakerHandle, // Fixme refactor to take a channel for Tock messages
    remaining: i32,
    interval: Option<Interval>,
}

impl TickerActor {
    pub fn new(receiver: mpsc::UnboundedReceiver<TickMessage>, speaker: SpeakerHandle) -> Self {
        TickerActor {
            receiver,
            speaker,
            interval: None,
            remaining: 0,
        }
    }

    fn handle_message(&mut self, msg: TickMessage) {
        match msg {
            TickMessage::Tick(times) => {
                self.remaining = std::cmp::max(self.remaining, times);
                if self.interval.is_none() {
                    self.interval = Some(interval(Duration::from_millis(100)));
                }
            },
            TickMessage::Stop => {
                self.interval = None;
                self.remaining = 0;
            }
        }
    }

    async fn run(&mut self) {
        loop {
            if let Some(interval) = &mut self.interval {
                select! {
                    msg = self.receiver.recv() => {
                        if let Some(msg) = msg{
                            self.handle_message(msg);
                        } else {
                            break;
                        }
                    },
                    _ = interval.tick() => {
                        self.remaining = self.remaining - 1;
                        if self.remaining > 0 {
                            self.speaker.send(SpeakerMessage::Tick).await;
                        } else {
                            self.interval = None;
                        }
                    }

                }
            } else {
                if let Some(msg) = self.receiver.recv().await {
                    self.handle_message(msg);
                } else {
                    break;
                }
            }
        }
    }
}

pub struct TickerHandle {
    sender: mpsc::UnboundedSender<TickMessage>,
}

impl TickerHandle {
    pub fn new(speaker: SpeakerHandle) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut actor = TickerActor::new(receiver, speaker);
            actor.run().await;
        });

        Self { sender }
    }

    pub async fn send(&self, msg: TickMessage) {
        self.sender.send(msg).unwrap()
    }
}
