//! Bounded control queue. Only consecutive absolute pointer positions may be
//! replaced; keys, buttons, focus and ownership messages are ordering barriers.
use crate::model::{ControlQueueDiagnostics, InputEvent, Message};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
    time::Instant,
};
use tokio::sync::Notify;

struct Pending {
    epoch: u64,
    enqueued: Instant,
    message: Message,
}
pub(crate) struct Outbound {
    pending: Mutex<VecDeque<Pending>>,
    capacity: usize,
    ready: Notify,
    peak_pending: AtomicUsize,
    coalesced_moves: AtomicU64,
    full_rejections: AtomicU64,
    longest_wait_us: AtomicU64,
}
impl Outbound {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
            capacity,
            ready: Notify::new(),
            peak_pending: AtomicUsize::new(0),
            coalesced_moves: AtomicU64::new(0),
            full_rejections: AtomicU64::new(0),
            longest_wait_us: AtomicU64::new(0),
        }
    }
    pub(crate) fn try_send(&self, item: (u64, Message)) -> bool {
        let item = Pending {
            epoch: item.0,
            enqueued: Instant::now(),
            message: item.1,
        };
        let mut pending = self.pending.lock().unwrap();
        if matches!(item.message, Message::Input(InputEvent::Move { .. })) {
            if let Some(last) = pending.back_mut() {
                if last.epoch == item.epoch
                    && matches!(last.message, Message::Input(InputEvent::Move { .. }))
                {
                    *last = item;
                    self.coalesced_moves.fetch_add(1, Ordering::Relaxed);
                    return true;
                }
            }
        }
        if pending.len() >= self.capacity {
            self.full_rejections.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        pending.push_back(item);
        self.peak_pending
            .fetch_max(pending.len(), Ordering::Relaxed);
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
                return self.dequeued(item);
            }
            notified.await;
        }
    }
    fn dequeued(&self, item: Pending) -> (u64, Message) {
        self.longest_wait_us.fetch_max(
            item.enqueued.elapsed().as_micros().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
        (item.epoch, item.message)
    }
    pub(crate) fn diagnostics(&self) -> ControlQueueDiagnostics {
        ControlQueueDiagnostics {
            pending: self.pending.lock().unwrap().len(),
            peak_pending: self.peak_pending.load(Ordering::Relaxed),
            coalesced_moves: self.coalesced_moves.load(Ordering::Relaxed),
            full_rejections: self.full_rejections.load(Ordering::Relaxed),
            longest_wait_us: self.longest_wait_us.load(Ordering::Relaxed),
        }
    }
    #[cfg(test)]
    pub(crate) fn try_recv(&self) -> Option<(u64, Message)> {
        self.pending
            .lock()
            .unwrap()
            .pop_front()
            .map(|item| self.dequeued(item))
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
    async fn pressure_metrics_count_coalescing_and_keep_scroll_as_a_barrier() {
        let queue = Outbound::new(3);
        assert!(queue.try_send((0, movement(1.))));
        assert!(queue.try_send((0, movement(2.))));
        assert!(queue.try_send((0, Message::Input(InputEvent::Scroll { x: 0., y: 0.125 }))));
        assert!(queue.try_send((0, movement(3.))));
        assert!(!queue.try_send((0, key(true))));
        let stats = queue.diagnostics();
        assert_eq!(stats.pending, 3);
        assert_eq!(stats.peak_pending, 3);
        assert_eq!(stats.coalesced_moves, 1);
        assert_eq!(stats.full_rejections, 1);
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        assert!(matches!(
            queue.recv().await.1,
            Message::Input(InputEvent::Move { x: 2., .. })
        ));
        assert!(matches!(
            queue.recv().await.1,
            Message::Input(InputEvent::Scroll { y: 0.125, .. })
        ));
        assert!(matches!(
            queue.recv().await.1,
            Message::Input(InputEvent::Move { x: 3., .. })
        ));
        let stats = queue.diagnostics();
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.peak_pending, 3);
        assert!(stats.longest_wait_us >= 1000);
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
