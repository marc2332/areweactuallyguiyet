use benchmark_common::{Command, Event};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
};

pub struct Connection {
    stream: TcpStream,
    input: Vec<u8>,
    pub events: Vec<Event>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        stream.set_nodelay(true)?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            input: Vec::new(),
            events: Vec::new(),
        })
    }

    pub fn send(&mut self, command: &Command) -> io::Result<()> {
        self.stream.set_nonblocking(false)?;
        self.stream
            .set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
        let mut data = serde_json::to_vec(command)?;
        data.push(b'\n');
        let result = self.stream.write_all(&data);
        self.stream.set_nonblocking(true)?;
        result
    }

    pub fn receive(&mut self) -> io::Result<Vec<Event>> {
        let mut buffer = [0; 4096];
        loop {
            match self.stream.read(&mut buffer) {
                Ok(0) if self.input.is_empty() => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Adapter disconnected",
                    ));
                }
                Ok(0) => break,
                Ok(length) => self.input.extend_from_slice(&buffer[..length]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
            if self.input.len() > 1_048_576 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Adapter event buffer exceeded 1 MiB",
                ));
            }
        }
        let mut events = Vec::new();
        while let Some(newline) = self.input.iter().position(|byte| *byte == b'\n') {
            let line: Vec<_> = self.input.drain(..=newline).collect();
            events.push(serde_json::from_slice::<Event>(&line)?);
        }
        self.events.extend(events.iter().cloned());
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::Connection;
    use benchmark_common::{Command, Event};
    use std::{
        io::{BufRead, BufReader, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    #[test]
    fn dedicated_connection_exchanges_commands_and_events() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let worker = thread::spawn(move || {
            let mut reader = BufReader::new(server);
            let mut command = String::new();
            reader.read_line(&mut command).unwrap();
            assert!(matches!(
                serde_json::from_str::<Command>(&command).unwrap(),
                Command::Prepare
            ));
            reader.get_mut().write_all(b"{\"event\":\"rea").unwrap();
            reader
                .get_mut()
                .write_all(b"dy\",\"boundary\":\"ui-built\"}\n{\"event\":\"started\"}\n")
                .unwrap();
        });
        let mut connection = Connection::new(client).unwrap();
        connection.send(&Command::Prepare).unwrap();
        worker.join().unwrap();
        let events = connection.receive().unwrap();
        assert!(matches!(&events[0], Event::Ready { boundary } if boundary == "ui-built"));
        assert!(matches!(events[1], Event::Started));
        assert_eq!(connection.events.len(), 2);
        assert!(connection.receive().is_err());
    }
}
