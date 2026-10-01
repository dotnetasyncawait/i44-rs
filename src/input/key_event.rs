use std::sync::mpsc::{self, Sender, Receiver, TryRecvError, RecvTimeoutError};
use std::time::Duration;

pub struct KeyEvent {
	rx: Receiver<()>
}

impl KeyEvent {
	pub(super) fn new() -> (KeyEventNotifier, Self) {
		let (tx, rx) = mpsc::channel();
		(KeyEventNotifier { tx }, Self { rx })
	}
	
	pub fn is_up(&self) -> bool {
		match self.rx.try_recv() {
			Ok(_) => true,
			Err(err) => match err {
				TryRecvError::Empty => false,
				TryRecvError::Disconnected => true,
			}
		}
	}
	
	pub fn wait_timeout(&self, timeout: Duration) -> bool {
		match self.rx.recv_timeout(timeout) {
			Ok(_) => true,
			Err(err) => match err {
				RecvTimeoutError::Timeout => false,
				RecvTimeoutError::Disconnected => true,
			}
		}
	}
	
	pub fn wait(&self) -> bool {
		let _ = self.rx.recv();
		true
	}
}

#[derive(Debug, Clone)]
pub(super) struct KeyEventNotifier {
	tx: Sender<()>
}

impl KeyEventNotifier {
	pub fn notify(self) {
		let _ = self.tx.send(());
	}
}
