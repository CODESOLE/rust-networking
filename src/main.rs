use basic_pathfinding::{
    coord::Coord,
    grid::{Grid, GridType},
    pathfinding::*,
};
use mio::{
    event::Event,
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Registry, Token,
};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    io::{self, Read, Write},
    str::from_utf8,
    str::FromStr,
};

mod parser;
use crate::parser::{parse_ascii_to_binary, parse_binary_to_ascii};

// #..#######
// #..#..#..#
// #..#..#..#
// #..#.....#
// #..#.....#
// #..####..#
// #........#
// ##########
const MAP: &str = "#..#######
#..#..#..#
#..#..#..#
#..#.....#
#..#.....#
#..####..#
#........#
##########";

#[derive(Clone, Copy)]
pub struct Car {
    pos: (i32, i32),
    target: (i32, i32),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseCarErr;

impl FromStr for Car {
    type Err = ParseCarErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (x, y) = s
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.split_once(':'))
            .ok_or(ParseCarErr)?;

        let (x1, y1) = x
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.split_once(','))
            .ok_or(ParseCarErr)?;

        let (x2, y2) = y
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.split_once(','))
            .ok_or(ParseCarErr)?;

        let x1_fromstr = x1.parse::<i32>().map_err(|_| ParseCarErr)?;
        let y1_fromstr = y1.parse::<i32>().map_err(|_| ParseCarErr)?;
        let x2_fromstr = x2.parse::<i32>().map_err(|_| ParseCarErr)?;
        let y2_fromstr = y2.parse::<i32>().map_err(|_| ParseCarErr)?;

        Ok(Car {
            pos: (x1_fromstr, y1_fromstr),
            target: (x2_fromstr, y2_fromstr),
        })
    }
}

impl Display for Car {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(({},{}):({},{}))",
            self.pos.0, self.pos.1, self.target.0, self.target.1
        )
    }
}

fn main() -> io::Result<()> {
    const SERVER: Token = Token(0);
    let mut c = Car {
        pos: (0, 0),
        target: (0, 0),
    };

    println!("{}", parse_binary_to_ascii(parse_ascii_to_binary(MAP.to_string())));

    let grid = Grid {
        tiles: parse_ascii_to_binary(MAP.to_string()),
        walkable_tiles: vec![1],
        grid_type: GridType::Cardinal,
        ..Default::default()
    };

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut server = TcpListener::bind("127.0.0.1:9123".parse().unwrap())?;

    poll.registry()
        .register(&mut server, SERVER, Interest::READABLE | Interest::WRITABLE)?;

    let mut connections = HashMap::new();
    let mut unique_token = Token(SERVER.0 + 1);

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    let (mut connection, address) = match server.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    };

                    println!("Accepted connection from: {}", address);

                    let token = Token(unique_token.0);
                    unique_token.0 += 1;

                    poll.registry()
                        .register(&mut connection, token, Interest::WRITABLE)?;

                    connections.insert(token, connection);
                },
                token => {
                    let done = if let Some(connection) = connections.get_mut(&token) {
                        handle_connection_event(&mut c, &grid, poll.registry(), connection, event)?
                    } else {
                        false
                    };
                    if done {
                        if let Some(mut connection) = connections.remove(&token) {
                            poll.registry().deregister(&mut connection)?;
                        }
                    }
                }
            }
        }
    }
}

fn handle_connection_event(
    c: &mut Car,
    grid: &Grid,
    registry: &Registry,
    connection: &mut TcpStream,
    event: &Event,
) -> io::Result<bool> {
    if event.is_writable() {
        println!("writable: {}", c);
        match connection.write(c.to_string().as_bytes()) {
            Ok(n) if n < c.to_string().as_bytes().len() => {
                return Err(io::ErrorKind::WriteZero.into())
            }
            Ok(_) => {
                registry.reregister(connection, event.token(), Interest::WRITABLE)?
            }
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {}
            Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                return handle_connection_event(c, &grid, registry, connection, event)
            }
            Err(err) => return Err(err),
        }
    }

    if event.is_readable() {
        let mut connection_closed = false;
        let mut received_data = vec![0; 13];
        let bytes_read = 0;
        println!("readable: ");
        loop {
            match connection.read(&mut received_data[bytes_read..]) {
                Ok(0) => {
                    connection_closed = true;
                    break;
                }
                Ok(_) => {}
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }

        if bytes_read != 0 {
            let received_data = &received_data[..bytes_read];
            let temp_c = from_utf8(received_data).unwrap().parse::<Car>().unwrap();
            let coor_start = Coord::new(temp_c.pos.0, temp_c.pos.1);
            let coor_end = Coord::new(temp_c.target.0, temp_c.target.1);
            if let Some(path) = find_path(&grid, coor_start, coor_end, Default::default()) {
                c.pos.0 = path[1].x;
                c.pos.1 = path[1].y;
            }
        }
        registry.reregister(connection, event.token(), Interest::READABLE)?;
        if connection_closed {
            println!("Connection closed");
            return Ok(true);
        }
    }

    Ok(false)
}