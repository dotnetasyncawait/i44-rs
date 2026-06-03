use std::sync::mpsc::{self, Sender, Receiver, TryRecvError, RecvTimeoutError};
use std::time::Duration;

pub struct KeyEvent {
	rx: Receiver<()>
}

impl KeyEvent {
	pub(super) fn new() -> (Self, KeyEventNotifier) {
		let (tx, rx) = mpsc::channel();
		(Self { rx }, KeyEventNotifier { tx })
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
}

#[derive(Debug)]
pub(super) struct KeyEventNotifier {
	tx: Sender<()>
}

impl KeyEventNotifier {
	pub fn notify(self) {
		let _ = self.tx.send(());
	}
}

impl Clone for KeyEventNotifier {
	fn clone(&self) -> Self {
		Self { tx: self.tx.clone() }
	}
}