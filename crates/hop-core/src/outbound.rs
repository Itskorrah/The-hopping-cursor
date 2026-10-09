//! Bounded control queue. Only consecutive absolute pointer positions may be
//! replaced; keys, buttons, focus and ownership messages are ordering barriers.
use crate::model::{InputEvent, Message};
use std::{collections::VecDeque, sync::Mutex};
use tokio::sync::Notify;

pub(crate) struct Outbound {
    pending: Mutex<VecDeque<(u64, Message)>>,
    capacity: usize,
    ready: Notify,
}
impl Outbound {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
            capacity,
            ready: Notify::new(),
        }
    }
    pub(crate) fn try_send(&self, item: (u64, Message)) -> bool {
        let mut pending = self.pending.lock().unwrap();
        if matches!(item.1, Message::Input(InputEvent::Move { .. })) {
            if let Some(last) = pending.back_mut() {
                if last.0 == item.0 && matches!(last.1, Message::Input(InputEvent::Move { .. })) {
                    *last = item;
                    return true;
                }
            }
        }
        if pending.len() >= self.capacity {
            return false;
        }
        pending.push_back(item);
        drop(pending);
        self.ready.notify_one();
        true
    }
    // One receiver per connection; Notify retains a permit if it races the pop.
    // The service's stop/priority channels govern connection closure.
    pub(crate) async fn recv(&self) -> (u64, Message) {
        loop {
            let notified = self.ready.notified();
            if let Some(item) = self.pending.lock().unwrap().pop_front() {
                return item;
            }
            notified.await;
        }
    }
    #[cfg(test)]
    pub(crate) fn try_recv(&self) -> Option<(u64, Message)> {
        self.pending.lock().unwrap().pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn movement(x: f64) -> Message {
        Message::Input(InputEvent::Move { x, y: 10. })
    }
    fn key(down: bool) -> Message {
        Message::Input(InputEvent::Key { code: 8, down })
    }
    #[tokio::test]
    async fn high_rate_motion_keeps_latest_position_and_discrete_order() {
        let queue = Outbound::new(5);
        for x in 0..8000 {
            assert!(queue.try_send((0, movement(x as f64))));
        }
        assert!(queue.try_send((0, key(true))));
        for x in 8000..16000 {
            assert!(queue.try_send((0, movement(x as f64))));
        }
        assert!(queue.try_send((0, key(false))));
        let messages = [
            queue.recv().await.1,
            queue.recv().await.1,
            queue.recv().await.1,
            queue.recv().await.1,
        ];
        assert!(matches!(
            messages[0],
            Message::Input(InputEvent::Move { x: 7999., .. })
        ));
        assert!(matches!(
            messages[1],
            Message::Input(InputEvent::Key { down: true, .. })
        ));
        assert!(matches!(
            messages[2],
            Message::Input(InputEvent::Move { x: 15999., .. })
        ));
        assert!(matches!(
            messages[3],
            Message::Input(InputEvent::Key { down: false, .. })
        ));
    }
    #[tokio::test]
    async fn epochs_and_buttons_are_barriers_and_pressure_stays_bounded() {
        let queue = Outbound::new(3);
        assert!(queue.try_send((0, movement(1.))));
        assert!(queue.try_send((1, movement(2.))));
        assert!(queue.try_send((
            1,
            Message::Input(InputEvent::Button {
                button: 0,
                down: true
            })
        )));
        assert!(!queue.try_send((1, movement(3.))));
        assert_eq!(queue.recv().await.0, 0);
        assert_eq!(queue.recv().await.0, 1);
        assert!(matches!(
            queue.recv().await.1,
            Message::Input(InputEvent::Button { .. })
        ));
    }
    #[tokio::test]
    async fn waiting_receiver_wakes_for_later_input() {
        let queue = std::sync::Arc::new(Outbound::new(1));
        let reader = queue.clone();
        let receive = tokio::spawn(async move { reader.recv().await });
        tokio::task::yield_now().await;
        assert!(queue.try_send((7, movement(42.))));
        let item = tokio::time::timeout(std::time::Duration::from_secs(1), receive)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(item.0, 7);
    }
}
