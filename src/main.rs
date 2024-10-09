extern crate core;

use std::error;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Sender;

#[derive(Debug,Clone)]
struct Message {
    id: u16,
    data: String
}

async fn connection_handler(stream: TcpStream, tx: &Sender<Arc<Message>>) -> Result<(), Box<dyn error::Error>> {
    let local_port = stream.peer_addr()?.port();
    let (mut read, mut write) = stream.into_split();
    
    let tx = tx.clone();
    let mut rx = tx.subscribe();
    
    write.write_all(format!("LOGIN:{}\n", local_port).as_bytes()).await.unwrap();
    
    // read task
    tokio::spawn(async move {
        loop {
            let reader = tokio::io::BufReader::new(&mut read);
            let line = reader.lines().next_line().await;
            
            match line { 
                Err(_) => return,
                Ok(None) => return,
                Ok(Some(s)) => {
                    println!("{}", format!("message {} {}", local_port, s));
                    tx.send(Arc::new(Message{id: local_port, data: s})).unwrap()
                },
            };
        }
    });
    
    // write task
    tokio::spawn(async move {
        loop {
            let msg = rx.recv().await.unwrap();

            let msg_str = if msg.id == local_port {
                "ACK:MESSAGE\n".to_string()
            } else {
                format!("MESSAGE:{} {}\n", msg.id, msg.data)
            };

            if let Err(_) = write.write_all(msg_str.as_bytes()).await {
                return;
            }
        }
    });
    
    Ok(())
}

async fn accept_connection(listener: &TcpListener, tx: &Sender<Arc<Message>>) -> Result<(), Box<dyn error::Error>> {
    let (accept, addr) = listener.accept().await.or_else(|e| {
        eprintln!("accept error: {:?}", e);
        Err(e)
    })?;
    
    println!("connected {} {}", addr.ip(), addr.port());

    if let Err(e) = connection_handler(accept, tx).await {
        eprintln!("connection error: {:?}", e);
        Err(e)
    } else { Ok(()) }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8888").await.unwrap();
    println!("listening on port {}", listener.local_addr()?.port());
    
    let (tx, _) = tokio::sync::broadcast::channel::<Arc<Message>>(16);
    
    loop {
        accept_connection(&listener, &tx).await?;
    }
}
