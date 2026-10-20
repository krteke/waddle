use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Child, Command},
    time::{Duration, Instant},
};

pub struct Wing {
    process: Child,
    pub address: SocketAddr,
    _directory: tempfile::TempDir,
}

impl Wing {
    pub fn start() -> Self {
        let binary =
            std::env::var("DAE_WING_BIN").expect("Set DAE_WING_BIN to a dae-wing executable");
        let directory = tempfile::tempdir().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let log = std::fs::File::create(directory.path().join("dae-wing.log")).unwrap();
        let process = Command::new(binary)
            .args([
                "run",
                "--api-only",
                "--listen",
                &address.to_string(),
                "--config",
            ])
            .arg(directory.path())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap();
        let mut wing = Self {
            process,
            address,
            _directory: directory,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while TcpStream::connect(address).is_err() {
            assert!(
                wing.process.try_wait().unwrap().is_none(),
                "dae-wing exited during startup"
            );
            assert!(Instant::now() < deadline, "dae-wing did not start");
            std::thread::sleep(Duration::from_millis(20));
        }
        wing
    }
}

impl Drop for Wing {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
